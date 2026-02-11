use chrono::NaiveDate;
use rusqlite::{Connection, params};

use crate::errors::{AppError, AppResult};
use crate::models::location::Location;

use super::events::map_row;

/// Ricalcola i valori "pair" per tutti gli eventi di una data.
pub fn recalc_pairs_for_date(conn: &Connection, date: &NaiveDate) -> AppResult<()> {
    let date_str = date.format("%Y-%m-%d").to_string();

    let mut stmt = conn.prepare(
        "SELECT * FROM events
         WHERE date = ?1
         ORDER BY time ASC",
    )?;
    let rows = stmt.query_map([date_str.clone()], map_row)?;

    let mut events = Vec::new();
    for r in rows {
        events.push(r?);
    }

    if events.is_empty() {
        return Ok(());
    }

    // ✅ Day-marker handling (Holiday OR NationalHoliday)
    let has_marker = events
        .iter()
        .any(|e| e.location == Location::Holiday || e.location == Location::NationalHoliday);

    if has_marker {
        if events.len() > 1 {
            return Err(AppError::InvalidTime(format!(
                "Invalid sequence on {}: Holiday/NationalHoliday cannot coexist with IN/OUT events.",
                date_str
            )));
        }

        conn.execute(
            "UPDATE events SET pair = 0 WHERE date = ?1",
            params![date_str],
        )?;
        return Ok(());
    }

    let mut current_pair = 1;
    let mut open_in: Option<i32> = None;

    for ev in &events {
        if ev.kind.is_in() {
            if open_in.is_some() {
                return Err(AppError::InvalidTime(format!(
                    "Invalid sequence on {}: Found IN at {} but previous pair {} has no OUT.",
                    date_str, ev.time, current_pair
                )));
            }

            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                params![current_pair, ev.id],
            )?;

            open_in = Some(ev.id);
        } else if ev.kind.is_out() {
            if open_in.is_none() {
                return Err(AppError::InvalidTime(format!(
                    "Invalid sequence on {}: Found OUT at {} without matching IN.",
                    date_str, ev.time
                )));
            }

            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                params![current_pair, ev.id],
            )?;

            open_in = None;
            current_pair += 1;
        }
    }

    Ok(())
}

pub fn recalc_all_pairs(conn: &mut Connection) -> AppResult<()> {
    let dates: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT date FROM events ORDER BY date ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut v = Vec::new();
        for r in rows {
            v.push(r?);
        }
        v
    };

    for d in dates {
        let date = NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|_| AppError::InvalidDate(d.clone()))?;

        recalc_pairs_for_date(conn, &date)?;
    }

    Ok(())
}
