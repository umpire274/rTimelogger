use chrono::NaiveDate;
use rusqlite::{Connection, params};

use crate::errors::AppResult;

/// True se esiste già un marker Holiday/NationalHoliday per quel giorno.
/// Marker = evento IN alle 00:00 con position in ('H','N').
pub fn day_marker_exists(conn: &Connection, date: &NaiveDate) -> AppResult<bool> {
    let date_str = date.format("%Y-%m-%d").to_string();

    let count: i64 = conn.query_row(
        r#"
        SELECT COUNT(1)
        FROM events
        WHERE date = ?1
          AND time = '00:00'
          AND kind = 'in'
          AND position IN ('H','N')
        "#,
        params![date_str],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// True se la data contiene eventi lavorativi (non marker H/N).
pub fn date_has_work_events(conn: &Connection, date: &NaiveDate) -> AppResult<bool> {
    let date_str = date.format("%Y-%m-%d").to_string();

    let count: i64 = conn.query_row(
        r#"
        SELECT COUNT(1)
        FROM events
        WHERE date = ?1
          AND position NOT IN ('H','N')
        "#,
        params![date_str],
        |row| row.get(0),
    )?;

    Ok(count > 0)
}

/// Cancella tutti gli eventi di una data (solo con --replace).
pub fn delete_events_for_date(conn: &Connection, date: &NaiveDate) -> AppResult<usize> {
    let date_str = date.format("%Y-%m-%d").to_string();
    Ok(conn.execute("DELETE FROM events WHERE date = ?1", params![date_str])?)
}
