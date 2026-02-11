use crate::config::Config;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::{
    insert_event, load_events_by_date, load_pair_by_index, recalc_pairs_for_date,
};
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
        from: Option<NaiveDate>,
        to: Option<NaiveDate>,
        pos: Option<String>,
    ) -> AppResult<()> {
        // ------------------------------------------------
        // Resolve final position (only if --pos is provided)
        // ------------------------------------------------
        let pos_final = match &pos {
            Some(code) => Location::from_code(code).ok_or_else(|| {
                AppError::InvalidPosition(format!(
                    "Invalid location code '{}'. Use a valid code such as 'O', 'R', 'H', 'N', 'C', 'M', 'S'.",
                    code
                ))
            })?,
            None => position,
        };

        // ------------------------------------------------
        // Sanity: range args only allowed for SickLeave
        // ------------------------------------------------
        let range = match (from, to) {
            (Some(f), Some(t)) => {
                if pos_final != Location::SickLeave {
                    return Err(AppError::InvalidArgs(
                        "--from/--to can only be used with --pos Malattia".into(),
                    ));
                }
                if f > t {
                    // se hai questa variante tipizzata, usa quella; altrimenti InvalidArgs
                    return Err(AppError::InvalidDateRange { from: f, to: t });
                }
                Some((f, t))
            }
            (None, None) => None,
            _ => {
                return Err(AppError::InvalidArgs(
                    "Both --from and --to must be provided together.".into(),
                ));
            }
        };

        // ------------------------------------------------
        // 1️⃣ EDIT MODE
        // ------------------------------------------------
        if edit_mode {
            if range.is_some() {
                return Err(AppError::InvalidArgs(
                    "--from/--to cannot be used with --edit.".into(),
                ));
            }

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
                    extras_cli(Some(0), false),
                );
            }

            // LUNCH (applies to OUT)
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

            recalc_pairs_for_date(&pool.conn, &date)?;

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
        let wg = work_gap.unwrap_or(false);

        // --work-gap valid only with OUT present
        if wg && end.is_none() {
            return Err(AppError::InvalidArgs(
                "--work-gap can only be used when adding an OUT event.".into(),
            ));
        }

        // ------------------------------------------------
        // ✅ CASE: SickLeave marker day (like Holiday)
        // ------------------------------------------------
        if pos_final == Location::SickLeave {
            // Marker day: do not accept time/lunch/work-gap args
            if start.is_some() || end.is_some() || lunch.is_some() || work_gap.is_some() {
                return Err(AppError::InvalidArgs(
                    "For Sick Leave do not specify --in, --out, --lunch or --work-gap.".into(),
                ));
            }

            // Range: if omitted -> single-day (date,date)
            let (from_date, to_date) = match (from, to) {
                (Some(f), Some(t)) => {
                    if f > t {
                        return Err(AppError::InvalidDateRange { from: f, to: t });
                    }
                    (f, t)
                }
                (None, None) => (date, date),
                _ => {
                    return Err(AppError::InvalidArgs(
                        "Both --from and --to must be provided together.".into(),
                    ));
                }
            };

            // Sentinel time (00:00) like holiday
            let marker_time = NaiveTime::from_hms_opt(0, 0, 0)
                .ok_or_else(|| AppError::Other("Invalid Sick Leave time sentinel.".into()))?;

            let tx = pool.conn.transaction()?;

            let mut day = from_date;
            while day <= to_date {
                // Check existing events on that day (fail fast)
                let day_str = day.to_string();
                let exists: i64 = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM events WHERE date = ?1 LIMIT 1)",
                    rusqlite::params![day_str],
                    |r| r.get(0),
                )?;
                if exists == 1 {
                    return Err(AppError::InvalidArgs(format!(
                        "Cannot set Sick Leave on {}: the date already has events.",
                        day
                    )));
                }

                let ev = build_event_cli(
                    day,
                    marker_time,
                    EventType::In,
                    Location::SickLeave,
                    extras_cli(Some(0), false),
                );

                insert_event(&tx, &ev)?;
                recalc_pairs_for_date(&tx, &day)?;

                day = day
                    .succ_opt()
                    .ok_or_else(|| AppError::Other("Invalid date increment.".into()))?;
            }

            tx.commit()?;

            if from_date == to_date {
                success(format!("Added SICK LEAVE on {}.", from_date));
            } else {
                success(format!(
                    "Added SICK LEAVE from {} to {} ({} days).",
                    from_date,
                    to_date,
                    (to_date - from_date).num_days() + 1
                ));
            }

            return Ok(());
        }

        // ------------------------------------------------
        // Events for the single day (normal flow)
        // ------------------------------------------------
        let date_str = date.to_string();
        let events_today = load_events_by_date(pool, &date)?;
        let has_events = !events_today.is_empty();

        // ------------------------------------------------
        // ✅ CASE: Holiday / NationalHoliday marker day
        // ------------------------------------------------
        if pos_final == Location::Holiday || pos_final == Location::NationalHoliday {
            // Marker day: do not accept time/lunch/work-gap args
            if start.is_some()
                || end.is_some()
                || lunch.is_some()
                || work_gap.is_some()
                || range.is_some()
            {
                return Err(AppError::InvalidArgs(
                    "For holiday days do not specify --start, --end, --lunch, --work-gap, --from or --to.".into(),
                ));
            }

            if has_events {
                return Err(AppError::InvalidArgs(
                    "Cannot set a holiday marker on a date that already has events.".into(),
                ));
            }

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
            recalc_pairs_for_date(&pool.conn, &date)?;

            success(match pos_final {
                Location::Holiday => format!("Added HOLIDAY on {}.", date_str),
                Location::NationalHoliday => format!("Added NATIONAL HOLIDAY on {}.", date_str),
                _ => unreachable!(),
            });
            return Ok(());
        }

        // CASE A: only lunch update
        if start.is_none() && end.is_none() && lunch.is_some() {
            if range.is_some() {
                return Err(AppError::InvalidArgs(
                    "--from/--to are not valid for lunch-only updates.".into(),
                ));
            }
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
                "Nothing to do: specify at least --start, --end or --lunch.".into(),
            ));
        }

        // CASE C: IN only
        if let Some(start_time) = start
            && end.is_none()
        {
            if range.is_some() {
                return Err(AppError::InvalidArgs(
                    "--from/--to require --pos Malattia.".into(),
                ));
            }

            let ev_in = build_event_cli(
                date,
                start_time,
                EventType::In,
                pos_final,
                extras_cli(lunch, false),
            );

            insert_event(&pool.conn, &ev_in)?;
            recalc_pairs_for_date(&pool.conn, &date)?;

            let events_after = load_events_by_date(pool, &date)?;
            let summary = Core::build_daily_summary(&events_after, cfg);

            let tgt_time = start_time + chrono::Duration::minutes(summary.expected);

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
            if range.is_some() {
                return Err(AppError::InvalidArgs(
                    "--from/--to require --pos Malattia.".into(),
                ));
            }

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

            let mut ev_out = build_event_cli(
                date,
                end_time,
                EventType::Out,
                out_position,
                extras_cli(lunch, false),
            );

            if let Some(wg_explicit) = work_gap {
                ev_out.work_gap = wg_explicit;
            }

            insert_event(&pool.conn, &ev_out)?;
            recalc_pairs_for_date(&pool.conn, &date)?;

            success(format!(
                "Added OUT on {} ({} → {}).",
                date_str, last_in.time, end_time
            ));
            return Ok(());
        }

        // CASE E: full pair
        if let (Some(start_time), Some(end_time)) = (start, end) {
            if range.is_some() {
                return Err(AppError::InvalidArgs(
                    "--from/--to require --pos Malattia.".into(),
                ));
            }

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

            let mut ev_out = build_event_cli(
                date,
                end_time,
                EventType::Out,
                pos_final,
                extras_cli(lunch, false),
            );

            if let Some(wg_explicit) = work_gap {
                ev_out.work_gap = wg_explicit;
            }

            insert_event(&pool.conn, &ev_in)?;
            insert_event(&pool.conn, &ev_out)?;
            recalc_pairs_for_date(&pool.conn, &date)?;

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
