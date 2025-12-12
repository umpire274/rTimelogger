use crate::db::pool::DbPool;
use crate::db::queries::{insert_event, load_events_by_date, load_pair_by_index};
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;
use crate::ui::messages::success;
use chrono::{NaiveDate, NaiveTime};
use rusqlite::params;

/// High-level business logic for the `add` command.
pub struct AddLogic;

fn upsert_event(conn: &rusqlite::Connection, ev: &Event) -> AppResult<()> {
    if ev.id == 0 {
        insert_event(conn, ev)?;
    } else {
        crate::db::queries::update_event(conn, ev)?;
    }
    Ok(())
}

impl AddLogic {
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        pool: &mut DbPool,
        date: NaiveDate,
        position: Location,
        start: Option<NaiveTime>,
        lunch: Option<i32>,
        work_gap: Option<bool>,
        end: Option<NaiveTime>,
        edit_mode: bool,
        edit_pair: Option<usize>,
        pos: Option<String>,
    ) -> AppResult<()> {
        // ------------------------------------------------
        // Resolve final position (only if --pos is provided)
        // ------------------------------------------------
        let pos_final = match &pos {
            Some(code) => Location::from_code(code).ok_or_else(|| {
                AppError::InvalidPosition(format!(
                    "Invalid location code '{}'. Use a valid code such as 'office', 'remote', 'customer', ...",
                    code
                ))
            })?,
            None => position,
        };

        // ------------------------------------------------
        // 1️⃣ EDIT MODE
        // ------------------------------------------------
        if edit_mode {
            let pair_num = edit_pair
                .ok_or_else(|| AppError::InvalidTime("Missing --pair when using --edit.".into()))?;

            let (mut ev_in, mut ev_out) = load_pair_by_index(&pool.conn, &date, pair_num)?;

            // POSITION
            if let Some(ref mut e) = ev_in
                && pos.is_some()
            {
                e.location = pos_final;
            }
            if let Some(ref mut e) = ev_out
                && pos.is_some()
            {
                e.location = pos_final;
            }

            // IN time
            if let Some(start_time) = start {
                if let Some(ref mut e) = ev_in {
                    e.time = start_time;
                } else {
                    ev_in = Some(Event::new(
                        0,
                        date,
                        start_time,
                        EventType::In,
                        pos_final,
                        lunch,
                        false,
                    ));
                }
            }

            // OUT time
            if let Some(end_time) = end {
                if let Some(ref mut e) = ev_out {
                    e.time = end_time;
                } else {
                    ev_out = Some(Event::new(
                        0,
                        date,
                        end_time,
                        EventType::Out,
                        pos_final,
                        Some(0),
                        false,
                    ));
                }
            }

            // LUNCH
            if let Some(lunch_val) = lunch
                && let Some(ref mut e) = ev_out
            {
                e.lunch = Some(lunch_val);
            }

            // WORK GAP (solo se esplicitamente richiesto)
            if let Some(wg) = work_gap {
                if let Some(ref mut e) = ev_out {
                    e.work_gap = wg;
                } else {
                    return Err(AppError::InvalidTime(
                        "Cannot modify work_gap: pair has no OUT event.".into(),
                    ));
                }
            }

            // Save
            if let Some(ref e) = ev_in {
                upsert_event(&pool.conn, e)?;
            }
            if let Some(ref e) = ev_out {
                upsert_event(&pool.conn, e)?;
            }

            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            let (icon, msg) = if Some(work_gap) == Some(Option::from(true)) {
                ("🔗", "Work gap enabled")
            } else if Some(work_gap) == Some(Option::from(false)) {
                ("✂️", "Work gap removed")
            } else {
                ("✏️", "Pair updated")
            };

            success(format!("{} {} for pair {}.", icon, msg, pair_num));

            return Ok(());
        }

        // ------------------------------------------------
        // 2️⃣ INSERT MODE
        // ------------------------------------------------

        let lunch_val = lunch.unwrap_or(0);
        let date_str = date.to_string();
        let wg = work_gap.unwrap_or(false); // 🔑 risolvo QUI

        let events_today = load_events_by_date(pool, &date)?;
        let has_events = !events_today.is_empty();

        // --work-gap valido solo con OUT
        if wg && end.is_none() {
            return Err(AppError::InvalidTime(
                "--work-gap can only be used when adding an OUT event.".into(),
            ));
        }

        // CASE A: only lunch update
        if start.is_none() && end.is_none() && lunch.is_some() {
            if !has_events {
                return Err(AppError::InvalidTime(
                    "Cannot set lunch on a date with no events.".into(),
                ));
            }

            pool.conn.execute(
                r#"
                UPDATE events
                SET lunch_break = ?1
                WHERE id = (
                    SELECT id FROM events
                    WHERE date = ?2
                    ORDER BY time DESC
                    LIMIT 1
                )
                "#,
                params![lunch_val, &date_str],
            )?;

            success(format!(
                "Lunch updated to {} minutes for {}.",
                lunch_val, date_str
            ));
            return Ok(());
        }

        // CASE B: nothing to do
        if start.is_none() && end.is_none() {
            return Err(AppError::InvalidTime(
                "Nothing to do: specify at least --in, --out or --lunch.".into(),
            ));
        }

        // CASE C: IN only
        if let Some(start_time) = start
            && end.is_none()
        {
            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                pos_final,
                Some(lunch_val),
                false,
            );

            insert_event(&pool.conn, &ev_in)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            success(format!("Added IN at {} on {}.", start_time, date_str));
            return Ok(());
        }

        // CASE D: OUT only
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
                return Err(AppError::InvalidTime(
                    "OUT must be later than the previous IN.".into(),
                ));
            }

            let out_position = if pos.is_some() {
                pos_final
            } else {
                last_in.location
            };

            let ev_out = Event::new(
                0,
                date,
                end_time,
                EventType::Out,
                out_position,
                Some(lunch_val),
                wg,
            );

            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            success(format!(
                "Added OUT on {} ({} → {}).",
                date_str, last_in.time, end_time
            ));
            return Ok(());
        }

        // CASE E: full pair
        if let (Some(start_time), Some(end_time)) = (start, end) {
            if end_time <= start_time {
                return Err(AppError::InvalidTime("END must be later than IN.".into()));
            }

            let in_position = pos_final;
            let ev_in = Event::new(
                0,
                date,
                start_time,
                EventType::In,
                in_position,
                Some(lunch_val),
                false,
            );

            let ev_out = Event::new(0, date, end_time, EventType::Out, in_position, Some(0), wg);

            insert_event(&pool.conn, &ev_in)?;
            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            success(format!(
                "Added IN/OUT pair on {}: {} → {}.",
                date_str, start_time, end_time
            ));
            return Ok(());
        }

        Err(AppError::InvalidTime(
            "Unhandled combination of parameters.".into(),
        ))
    }
}
