use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;
use chrono::{NaiveDate, NaiveTime};
use rusqlite::params;
use rusqlite::{Connection, Result, Row};

pub fn load_events_by_date(pool: &mut DbPool, date: &NaiveDate) -> AppResult<Vec<Event>> {
    let mut stmt = pool.conn.prepare(
        "SELECT * FROM events
         WHERE date = ?1
         ORDER BY time ASC",
    )?;

    let date_str = date.format("%Y-%m-%d").to_string();

    let rows = stmt.query_map([date_str], map_row)?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

pub fn map_row(row: &Row) -> Result<Event> {
    let date_str: String = row.get("date")?;
    let time_str: String = row.get("time")?;

    let date = NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::InvalidDate(date_str.clone())),
        )
    })?;

    let time = NaiveTime::parse_from_str(&time_str, "%H:%M").map_err(|_| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::InvalidTime(time_str.clone())),
        )
    })?;

    let kind_str: String = row.get("kind")?;
    let kind = EventType::from_db_str(&kind_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::InvalidTime(format!("Invalid kind: {}", kind_str))),
        )
    })?;

    let loc_str: String = row.get("position")?;
    let location = Location::from_db_str(&loc_str).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Text,
            Box::new(AppError::InvalidPosition(format!(
                "Invalid location: {}",
                loc_str
            ))),
        )
    })?;

    Ok(Event {
        id: row.get("id")?,
        date,
        time,
        kind,
        location,
        lunch: row.get("lunch_break")?,
        work_gap: row.get::<_, i32>("work_gap")? == 1,
        pair: row.get("pair")?,
        source: row.get("source")?,
        meta: row.get("meta")?,
        created_at: row.get("created_at")?,
    })
}

pub fn insert_event(conn: &Connection, ev: &Event) -> AppResult<()> {
    conn.execute(
        "INSERT INTO events (date, time, kind, position, lunch_break, work_gap, pair, source, meta, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            ev.date.format("%Y-%m-%d").to_string(),
            ev.time.format("%H:%M").to_string(),
            ev.kind.to_db_str(),
            ev.location.to_db_str(),
            ev.lunch.unwrap_or(0),
            if ev.work_gap { 1 } else { 0 },
            ev.pair,
            ev.source,
            ev.meta,
            ev.created_at,
        ],
    )?;
    Ok(())
}

pub fn load_log(pool: &mut DbPool) -> Result<Vec<(String, String)>> {
    let mut stmt = pool
        .conn
        .prepare("SELECT timestamp, message FROM log ORDER BY timestamp DESC")?;

    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }

    Ok(out)
}

pub fn delete_event(pool: &mut DbPool, id: i32) -> Result<()> {
    pool.conn.execute("DELETE FROM events WHERE id = ?", [id])?;
    Ok(())
}

/// Carica la "pair logica" N-esima per una certa data.
///
/// ATTENZIONE: non usa il campo `pair` del DB.
/// Le coppie vengono ricostruite in memoria in base all'ordine temporale:
/// - ogni IN apre una nuova coppia
/// - un OUT si aggancia all'ultima coppia senza OUT; se non esiste, crea una coppia "solo OUT"
pub fn load_pair_by_index(
    conn: &Connection,
    date: &NaiveDate,
    pair_index: usize, // 1-based nel CLI, qui lo convertiamo in 0-based
) -> AppResult<(Option<Event>, Option<Event>)> {
    // 1️⃣ Prendo tutti gli eventi del giorno, ordinati per ora
    let mut stmt = conn.prepare("SELECT * FROM events WHERE date = ?1 ORDER BY time ASC")?;

    let rows = stmt.query_map([date.to_string()], map_row)?;

    let mut events: Vec<Event> = Vec::new();
    for r in rows {
        events.push(r?);
    }

    if events.is_empty() {
        return Err(AppError::InvalidPair(pair_index)); // nessun evento in quel giorno
    }

    // 2️⃣ Ricostruisco la sequenza di pair logiche
    let mut pairs: Vec<(Option<Event>, Option<Event>)> = Vec::new();

    for ev in events.into_iter() {
        match ev.kind {
            EventType::In => {
                // ogni IN apre una nuova pair
                pairs.push((Some(ev), None));
            }
            EventType::Out => {
                // cerco l'ultima pair senza OUT
                if let Some(last) = pairs.last_mut()
                    && last.1.is_none()
                {
                    last.1 = Some(ev);
                    continue;
                }
                // se non c'era una pair aperta, creo una pair "solo OUT"
                pairs.push((None, Some(ev)));
            }
        }
    }

    if pair_index == 0 {
        return Err(AppError::InvalidPair(0));
    }

    let idx = pair_index - 1;
    if idx >= pairs.len() {
        return Err(AppError::InvalidPair(pair_index));
    }

    Ok(pairs[idx].clone())
}
/// Update an event (all fields except id)
pub fn update_event(conn: &Connection, ev: &Event) -> AppResult<()> {
    conn.execute(
        "UPDATE events
         SET date = ?1, time = ?2, kind = ?3,
             position = ?4, lunch_break = ?5,
             work_gap = ?6, pair = ?7,
             source = ?8, meta = ?9, created_at = ?10
         WHERE id = ?11",
        rusqlite::params![
            ev.date.to_string(),
            ev.time.format("%H:%M").to_string(),
            ev.kind.to_db_str(),
            ev.location.to_db_str(),
            ev.lunch.unwrap_or(0),
            if ev.work_gap { 1 } else { 0 },
            ev.pair,
            ev.source,
            ev.meta,
            ev.created_at,
            ev.id,
        ],
    )?;
    Ok(())
}

/// Ricalcola i valori "pair" per tutti gli eventi di una data.
/// Usa la logica:
/// - ogni IN apre una nuova coppia (pair N)
/// - l'OUT successivo (se esiste) prende lo stesso N
/// - se esiste un OUT senza IN, gli viene assegnata una nuova coppia
pub fn recalc_pairs_for_date(conn: &mut Connection, date: &NaiveDate) -> AppResult<()> {
    let date_str = date.format("%Y-%m-%d").to_string();

    // 1) Load events ordered by time
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
        return Ok(()); // nothing to do
    }

    // 2) Recalculate pairs with **strict validation**
    let mut current_pair = 1;
    let mut open_in: Option<i32> = None; // stores ID of last IN without OUT

    for ev in &events {
        if ev.kind.is_in() {
            //
            // CASE: IN event
            //
            if open_in.is_some() {
                // Found IN while previous IN has no OUT → INVALID
                return Err(AppError::InvalidTime(format!(
                    "Invalid sequence on {}: Found IN at {} but previous pair {} has no OUT.",
                    date_str, ev.time, current_pair
                )));
            }

            // Assign IN to the current pair
            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                params![current_pair, ev.id],
            )?;

            open_in = Some(ev.id);
        } else if ev.kind.is_out() {
            //
            // CASE: OUT event
            //
            if open_in.is_none() {
                // Found OUT without IN → INVALID
                return Err(AppError::InvalidTime(format!(
                    "Invalid sequence on {}: Found OUT at {} without matching IN.",
                    date_str, ev.time
                )));
            }

            // Assign OUT to the same pair
            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                params![current_pair, ev.id],
            )?;

            // Close the pair and increment
            open_in = None;
            current_pair += 1;
        }
    }

    // If after processing all events an IN is left open → it's allowed
    // because user may not have entered OUT yet (ongoing workday)
    // BUT the next IN will be rejected until OUT is inserted.

    Ok(())
}

pub fn recalc_all_pairs(conn: &mut Connection) -> AppResult<()> {
    // --- 1) Carico tutte le date in memoria
    let dates: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT date FROM events ORDER BY date ASC")?;

        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        let mut v = Vec::new();
        for r in rows {
            v.push(r?);
        }
        v
    }; // <-- stmt DROPPATO QUI

    // --- 2) Ora posso usare conn come mutabile in sicurezza
    for d in dates {
        let date = NaiveDate::parse_from_str(&d, "%Y-%m-%d")
            .map_err(|_| AppError::InvalidDate(d.clone()))?;

        recalc_pairs_for_date(conn, &date)?;
    }

    Ok(())
}
