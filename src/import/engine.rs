use std::collections::BTreeMap;

use chrono::{NaiveDate, NaiveTime};

use crate::config::Config;
use crate::db::pool::DbPool;
use crate::db::queries;
use crate::db::queries::import as qimp;
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;

use super::parser_csv::parse_csv_days;
use super::parser_json::parse_json_days;
use super::types::{ImportDay, ImportInputFormat, ImportReport};

pub fn import_days_from_str(
    cfg: &Config,
    content: &str,
    format: ImportInputFormat,
    dry_run: bool,
    replace: bool,
    source: &str,
) -> AppResult<ImportReport> {
    let parsed = match format {
        ImportInputFormat::Json => parse_json_days(content),
        ImportInputFormat::Csv => parse_csv_days(content),
    };

    let mut rep = ImportReport::default();
    rep.total = parsed.len(); // ✅ totale righe lette dal file

    let mut dedup: BTreeMap<NaiveDate, ImportDay> = BTreeMap::new();

    for row in parsed {
        match row {
            Ok(day) => {
                if day.position != Location::Holiday && day.position != Location::NationalHoliday {
                    rep.invalid += 1;
                    continue;
                }
                dedup.insert(day.date, day);
            }
            Err(_) => rep.invalid += 1,
        }
    }

    let mut pool = DbPool::new(&cfg.database)?;

    if dry_run {
        for (_, day) in dedup {
            evaluate_one(&pool, &day, replace, &mut rep)?;
        }
        return Ok(rep);
    }

    let tx = pool.conn.transaction()?;

    for (_, day) in dedup {
        apply_one(&tx, &day, replace, source, &mut rep)?;
    }

    tx.commit()?;
    Ok(rep)
}

fn evaluate_one(
    pool: &DbPool,
    day: &ImportDay,
    replace: bool,
    rep: &mut ImportReport,
) -> AppResult<()> {
    if qimp::day_marker_exists(&pool.conn, &day.date)? {
        rep.skipped_existing += 1;
        return Ok(());
    }

    let has_work = qimp::date_has_work_events(&pool.conn, &day.date)?;
    if has_work && !replace {
        rep.conflicts += 1;
        return Ok(());
    }

    rep.imported += 1;
    Ok(())
}

fn apply_one(
    conn: &rusqlite::Connection, // tx deref -> Connection
    day: &ImportDay,
    replace: bool,
    source: &str,
    rep: &mut ImportReport,
) -> AppResult<()> {
    if qimp::day_marker_exists(conn, &day.date)? {
        rep.skipped_existing += 1;
        return Ok(());
    }

    let has_work = qimp::date_has_work_events(conn, &day.date)?;
    if has_work && !replace {
        rep.conflicts += 1;
        return Ok(());
    }

    if has_work && replace {
        qimp::delete_events_for_date(conn, &day.date)?;
    }

    // Insert marker at 00:00 as IN with location H/N
    let t0 = NaiveTime::from_hms_opt(0, 0, 0)
        .ok_or_else(|| AppError::InvalidArgs("Invalid midnight time".into()))?;

    let ev = Event::new(
        0,
        day.date,
        t0,
        EventType::In,
        day.position,
        Some(0),
        false,
        Some(source),
        day.meta.clone(),
    );

    queries::insert_event(conn, &ev)?;

    // For markers, pair can stay 0 (non-working day marker).
    rep.imported += 1;
    Ok(())
}
