use crate::config::Config;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::{insert_event, load_events_by_date, load_pair_by_index};
use crate::errors::{AppError, AppResult};
use crate::models::event::{Event, EventExtras};
use crate::models::event_type::EventType;
use crate::models::location::Location;
use crate::ui::messages::success;
use chrono::{NaiveDate, NaiveTime, Timelike};
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

fn build_event_cli(
    date: NaiveDate,
    time: NaiveTime,
    kind: EventType,
    location: Location,
    event_extras: EventExtras,
) -> Event {
    Event::new(0, date, time, kind, location, event_extras)
}

fn extras_cli(lunch: Option<i32>, work_gap: bool) -> EventExtras {
    EventExtras {
        lunch,
        work_gap,
        source: Some("cli".to_string()),
        meta: None,
        ..Default::default()
    }
}

fn upsert_event_time(
    slot: &mut Option<Event>,
    date: NaiveDate,
    time: NaiveTime,
    kind: EventType,
    location: Location,
    extras: EventExtras,
) {
    let e = slot.get_or_insert_with(|| build_event_cli(date, time, kind, location, extras));
    e.time = time;
}

impl AddLogic {
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        cfg: &Config,
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
                    "Invalid location code '{}'. Use a valid code such as 'O', 'R', 'H', 'N', 'C', 'M'.",
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
                .ok_or_else(|| AppError::InvalidArgs("Missing --pair when using --edit.".into()))?;

            let (mut ev_in, mut ev_out) = load_pair_by_index(&pool.conn, &date, pair_num)?;

            // POSITION (apply only if --pos explicitly provided)
            if pos.is_some() {
                if let Some(ref mut e) = ev_in {
                    e.location = pos_final;
                }
                if let Some(ref mut e) = ev_out {
                    e.location = pos_final;
                }
            }

            // IN time
            // IN time
            if let Some(start_time) = start {
                upsert_event_time(
                    &mut ev_in,
                    date,
                    start_time,
                    EventType::In,
                    pos_final,
                    extras_cli(lunch, false),
                );
            }

            // OUT time
            if let Some(end_time) = end {
                upsert_event_time(
                    &mut ev_out,
                    date,
                    end_time,
                    EventType::Out,
                    pos_final,
                    extras_cli(Some(0), false), // se vuoi preservare la tua semantica OUT default
                );
            }

            // LUNCH (applies to OUT; coherent with your current model)
            if let Some(lunch_val) = lunch
                && let Some(ref mut e) = ev_out
            {
                e.lunch = Some(lunch_val);
            }

            // WORK GAP (only if explicitly requested; requires OUT)
            if let Some(wg) = work_gap {
                if let Some(ref mut e) = ev_out {
                    e.work_gap = wg;
                } else {
                    return Err(AppError::InvalidArgs(
                        "Cannot modify --work-gap: pair has no OUT event.".into(),
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

            let (icon, msg) = match work_gap {
                Some(true) => ("🔗", "Work gap enabled"),
                Some(false) => ("✂️", "Work gap removed"),
                None => ("✏️", "Pair updated"),
            };

            success(format!("{} {} for pair {}.", icon, msg, pair_num));
            return Ok(());
        }

        // ------------------------------------------------
        // 2️⃣ INSERT MODE
        // ------------------------------------------------

        let lunch_val = lunch.unwrap_or(0);
        let date_str = date.to_string();
        let wg = work_gap.unwrap_or(false);

        let events_today = load_events_by_date(pool, &date)?;
        let has_events = !events_today.is_empty();

        // --work-gap valid only with OUT
        if wg && end.is_none() {
            return Err(AppError::InvalidArgs(
                "--work-gap can only be used when adding an OUT event.".into(),
            ));
        }

        // ------------------------------------------------
        // ✅ CASE: Holiday / NationalHoliday marker day
        // ------------------------------------------------
        if pos_final == Location::Holiday || pos_final == Location::NationalHoliday {
            // Marker day: do not accept time/lunch/work-gap args
            if start.is_some() || end.is_some() || lunch.is_some() || work_gap.is_some() {
                return Err(AppError::InvalidArgs(
                    "For holiday days do not specify --in, --out, --lunch or --work-gap.".into(),
                ));
            }

            // If there are already events, it's inconsistent to mark holiday
            if has_events {
                return Err(AppError::InvalidArgs(
                    "Cannot set a holiday marker on a date that already has events.".into(),
                ));
            }

            // Sentinel event at 00:00
            let holiday_time = NaiveTime::from_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::Other("Invalid holiday time sentinel.".into()))?;

            let ev_holiday = build_event_cli(
                date,
                holiday_time,
                EventType::In,
                pos_final,
                extras_cli(lunch, false),
            );

            insert_event(&pool.conn, &ev_holiday)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            success(match pos_final {
                Location::Holiday => format!("Added HOLIDAY on {}.", date_str),
                Location::NationalHoliday => format!("Added NATIONAL HOLIDAY on {}.", date_str),
                _ => unreachable!(),
            });
            return Ok(());
        }

        // CASE A: only lunch update
        if start.is_none() && end.is_none() && lunch.is_some() {
            if !has_events {
                return Err(AppError::InvalidArgs(
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
            return Err(AppError::InvalidArgs(
                "Nothing to do: specify at least --in, --out or --lunch.".into(),
            ));
        }

        // CASE C: IN only
        if let Some(start_time) = start
            && end.is_none()
        {
            let ev_in = build_event_cli(
                date,
                start_time,
                EventType::In,
                pos_final,
                extras_cli(lunch, false),
            );

            insert_event(&pool.conn, &ev_in)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            // --- Compute TGT (same logic as list: uses summary.expected) ---
            let events_after = load_events_by_date(pool, &date)?;
            let summary = Core::build_daily_summary(&events_after, cfg);

            let tgt_time = start_time + chrono::Duration::minutes(summary.expected);

            // Usa helper esistente: minuti da mezzanotte -> "HH:MM"
            let tgt_mins = (tgt_time.hour() as i64) * 60 + (tgt_time.minute() as i64);
            let tgt_str = crate::utils::time::format_minutes(tgt_mins);

            success(format!(
                "Added IN at {} on {}. TGT => {}",
                start_time, date_str, tgt_str
            ));
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
                    AppError::InvalidArgs("Cannot add OUT without a previous IN.".into())
                })?;

            if end_time <= last_in.time {
                return Err(AppError::InvalidArgs(
                    "OUT must be later than the previous IN.".into(),
                ));
            }

            // If --pos provided, use it; otherwise inherit last IN location
            let out_position = if pos.is_some() {
                pos_final
            } else {
                last_in.location
            };

            let ev_out = build_event_cli(
                date,
                end_time,
                EventType::Out,
                out_position,
                extras_cli(lunch, false),
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
                return Err(AppError::InvalidArgs("END must be later than IN.".into()));
            }

            let ev_in = build_event_cli(
                date,
                start_time,
                EventType::In,
                pos_final,
                extras_cli(lunch, false),
            );

            let ev_out = build_event_cli(
                date,
                end_time,
                EventType::Out,
                pos_final,
                extras_cli(lunch, false),
            );

            insert_event(&pool.conn, &ev_in)?;
            insert_event(&pool.conn, &ev_out)?;
            crate::db::queries::recalc_pairs_for_date(&mut pool.conn, &date)?;

            success(format!(
                "Added IN/OUT pair on {}: {} → {}.",
                date_str, start_time, end_time
            ));
            return Ok(());
        }

        Err(AppError::InvalidArgs(
            "Unhandled combination of parameters.".into(),
        ))
    }
}
