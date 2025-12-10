use crate::db::pool::DbPool;
use crate::db::queries::map_row;
use crate::errors::AppResult;
use crate::models::event::Event;
use chrono::NaiveDate;
use rusqlite::{Row, params};

fn map_event(row: &Row) -> rusqlite::Result<Event> {
    map_row(row) // <-- QUI richiami la tua funzione originale
}

/// Rebuild `pair` for a single date.
pub fn rebuild_pairs_for_date(pool: &mut DbPool, date: &NaiveDate) -> AppResult<()> {
    let date_str = date.format("%Y-%m-%d").to_string();

    let mut stmt = pool.conn.prepare(
        r#"
        SELECT id, date, time, kind, position, lunch_break, source, meta, created_at, pair
        FROM events
        WHERE date = ?
        ORDER BY time ASC
        "#,
    )?;

    let events: Vec<Event> = stmt
        .query_map([&date_str], map_event)?
        .filter_map(|r| r.ok())
        .collect();

    if events.is_empty() {
        return Ok(());
    }

    let mut pair_id = 1;
    let mut last_was_in = false;

    for ev in events {
        match ev.kind {
            crate::models::event_type::EventType::In => {
                pool.conn.execute(
                    "UPDATE events SET pair = ? WHERE id = ?",
                    params![pair_id, ev.id],
                )?;
                last_was_in = true;
            }

            crate::models::event_type::EventType::Out => {
                if last_was_in {
                    pool.conn.execute(
                        "UPDATE events SET pair = ? WHERE id = ?",
                        params![pair_id, ev.id],
                    )?;
                } else {
                    pair_id += 1;
                    pool.conn.execute(
                        "UPDATE events SET pair = ? WHERE id = ?",
                        params![pair_id, ev.id],
                    )?;
                }
                pair_id += 1;
                last_was_in = false;
            }
        }
    }

    Ok(())
}

/// Rebuild pairs for all dates.
pub fn rebuild_all_pairs(pool: &mut DbPool) -> AppResult<()> {
    // 1️⃣ First collect all dates WITHOUT borrowing pool.conn for the whole duration
    let dates: Vec<String> = {
        let mut stmt = pool
            .conn
            .prepare("SELECT DISTINCT date FROM events ORDER BY date ASC")?;

        stmt.query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect()
    };

    // 2️⃣ Only now iterate and process dates mutably
    for d in dates {
        if let Ok(date) = NaiveDate::parse_from_str(&d, "%Y-%m-%d") {
            rebuild_pairs_for_date(pool, &date)?;
        }
    }

    println!("✅ Rebuilt pair IDs for all dates.");
    Ok(())
}
