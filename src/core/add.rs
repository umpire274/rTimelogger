use crate::db::pool::DbPool;
use crate::db::queries::{insert_event, load_events_by_date, load_pair_by_index};
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;
use chrono::{NaiveDate, NaiveTime};
use rusqlite::params;

/// High-level business logic for the `add` command.
pub struct AddLogic;

impl AddLogic {
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        pool: &mut DbPool,
        date: NaiveDate,
        position: Location,
        start: Option<NaiveTime>,
        lunch: Option<i32>,
        end: Option<NaiveTime>,
        edit_mode: bool,
        edit_pair: Option<usize>,
        _pos: Option<String>, // used only for audit logging (optional)
    ) -> AppResult<()> {
        // ------------------------------------------------
        // 1️⃣ EDIT MODE
        // ------------------------------------------------
        if edit_mode {
            let pair_num = edit_pair
                .ok_or_else(|| AppError::InvalidTime("Missing --pair when using --edit".into()))?;

            // Load IN/OUT pair
            let (mut ev_in, mut ev_out) = load_pair_by_index(&pool.conn, &date, pair_num)?;

            // Apply ONLY explicitly passed values

            // POSITION
            if let Some(ref mut e) = ev_in {
                e.location = position;
            }
            if let Some(ref mut e) = ev_out {
                e.location = position;
            }

            // IN
            if let Some(start_time) = start {
                if let Some(ref mut e) = ev_in {
                    e.time = start_time;
                } else {
                    // There was no IN → create new one
                    ev_in = Some(Event::new(
                        0,
                        date,
                        start_time,
                        EventType::In,
                        position,
                        lunch,
                        false,
                    ));
                }
            }

            // OUT
            if let Some(end_time) = end {
                if let Some(ref mut e) = ev_out {
                    e.time = end_time;
                } else {
                    // No OUT → create
                    ev_out = Some(Event::new(
                        0,
                        date,
                        end_time,
                        EventType::Out,
                        position,
                        Some(0),
                        false,
                    ));
                }
            }

            // LUNCH
            if let Some(lunch_val) = lunch {
                if let Some(ref mut e) = ev_out {
                    e.lunch = Some(lunch_val);
                } else if let Some(ref mut e) = ev_in {
                    e.lunch = Some(lunch_val);
                }
            }

            // Save changes
            if let Some(ref e) = ev_in {
                if e.id == 0 {
                    insert_event(&pool.conn, e)?;
                } else {
                    crate::db::queries::update_event(&pool.conn, e)?;
                }
            }

            if let Some(ref e) = ev_out {
                if e.id == 0 {
                    insert_event(&pool.conn, e)?;
                } else {
                    crate::db::queries::update_event(&pool.conn, e)?;
                }
            }

            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!("Updated pair {}", pair_num);
            return Ok(());
        }

        // ------------------------------------------------
        // 2️⃣ INSERT MODE (no edit)
        // ------------------------------------------------

        let lunch_val = lunch.unwrap_or(0);
        let date_str = date.to_string();

        // Load events of the day (ascending)
        let events_today = load_events_by_date(pool, &date)?;
        let has_events = !events_today.is_empty();

        // CASE A: Only lunch
        if start.is_none() && end.is_none() && lunch.is_some() {
            if !has_events {
                return Err(AppError::InvalidTime(
                    "Cannot set lunch on a date with no events.".into(),
                ));
            }

            let updated = pool.conn.execute(
                r#"
                UPDATE events
                SET lunch_break = ?1
                WHERE id = (
                    SELECT id
                    FROM events
                    WHERE date = ?2
                    ORDER BY time DESC
                    LIMIT 1
                )
                "#,
                params![lunch_val, &date_str],
            )?;

            if updated == 0 {
                return Err(AppError::InvalidTime(
                    "Unable to update lunch: no event found.".into(),
                ));
            }

            println!(
                "Lunch updated to {} minutes for the last event of {}",
                lunch_val, date_str
            );
            return Ok(());
        }

        // CASE B: No meaningful input
        if start.is_none() && end.is_none() {
            return Err(AppError::InvalidTime(
                "Nothing to do: specify at least --in, --out or --lunch.".into(),
            ));
        }

        // CASE C: start only → new IN
        if let Some(start_time) = start
            && end.is_none()
        {
            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                position,
                Some(lunch_val),
                false,
            );

            insert_event(&pool.conn, &ev_in)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Added IN: {} {} @ {} (lunch {} min)",
                date_str,
                position.code(),
                start_time,
                lunch_val
            );
            return Ok(());
        }

        // CASE D: end only → close last IN
        if start.is_none()
            && let Some(end_time) = end
        {
            let last_in = events_today
                .iter()
                .rev()
                .find(|ev| ev.kind == EventType::In)
                .cloned()
                .ok_or_else(|| {
                    AppError::InvalidTime("Cannot add OUT without a previous IN.".into())
                })?;

            if end_time <= last_in.time {
                return Err(AppError::InvalidTime("OUT must be later than IN.".into()));
            }

            let ev_out = Event::new(
                0,
                date,
                end_time,
                EventType::Out,
                position,
                Some(lunch_val),
                false,
            );

            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Added OUT: {} {} -> {} (lunch {} min)",
                date_str, last_in.time, end_time, lunch_val
            );
            return Ok(());
        }

        // CASE E: start + end → full pair
        if let (Some(start_time), Some(end_time)) = (start, end) {
            if end_time <= start_time {
                return Err(AppError::InvalidTime("END must be later than IN.".into()));
            }

            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                position,
                Some(lunch_val),
                false,
            );
            let ev_out = Event::new(0, date, end_time, EventType::Out, position, Some(0), false);

            insert_event(&pool.conn, &ev_in)?;
            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            println!(
                "Added IN/OUT pair for {}: {} → {} (lunch {} min)",
                date_str, start_time, end_time, lunch_val
            );
            return Ok(());
        }

        // ------------------------------------------------
        // Fallback (should never happen)
        // ------------------------------------------------
        Err(AppError::InvalidTime(
            "Unhandled combination of parameters (internal bug).".into(),
        ))
    }
}
