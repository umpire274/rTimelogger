use chrono::{NaiveTime, Utc};
use rusqlite::{Connection, OptionalExtension, Result, ToSql, params};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
mod migrate;
pub use migrate::run_pending_migrations;

/// Represents a work session entry
#[derive(Debug, Clone, Serialize)]
pub struct WorkSession {
    pub id: i32,
    pub date: String,
    pub position: String, // O,R,H,C,M
    pub start: String,
    pub lunch: i32,
    pub end: String,
    pub work_duration: Option<i32>, // minuti netti: (end-start)-lunch
}

/// Represents a single punch event (in/out)
#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i32,
    pub date: String,
    pub time: String,     // HH:MM
    pub kind: String,     // "in" or "out"
    pub position: String, // O,R,H,C,M
    pub lunch_break: i32, // minutes, typically set on out
    pub pair: i32,
    pub source: String,
    pub meta: String,
    pub created_at: String, // ISO timestamp
}

fn hhmm_to_minutes(s: &str) -> Option<i32> {
    let mut it = s.split(':');
    let h = it.next()?.parse::<i32>().ok()?;
    let m = it.next()?.parse::<i32>().ok()?;
    Some(h * 60 + m)
}

fn calculate_work_duration(start: &str, end: &str, lunch: i32) -> Option<i32> {
    let sm = hhmm_to_minutes(start)?;
    let em = hhmm_to_minutes(end)?;
    if em >= sm {
        Some(((em - sm) - lunch).max(0))
    } else {
        // opzionale: gestisci overnight
        Some((((em + 24 * 60) - sm) - lunch).max(0))
    }
}

pub fn row_to_worksession(row: &rusqlite::Row) -> Result<WorkSession> {
    let start: Option<String> = row.get("start_time")?;
    let end: Option<String> = row.get("end_time")?;
    let lunch: i32 = row.get::<_, Option<i32>>("lunch_break")?.unwrap_or(0);
    // Avoid cloning the strings: use as_deref to obtain &str (empty string if None)
    let work_duration = calculate_work_duration(
        start.as_deref().unwrap_or(""),
        end.as_deref().unwrap_or(""),
        lunch,
    );

    Ok(WorkSession {
        id: row.get("id")?,
        date: row.get("date")?,
        position: row.get("position")?,
        start: start.unwrap_or_default(),
        lunch,
        end: end.unwrap_or_default(),
        work_duration,
    })
}

pub(crate) fn row_to_event(row: &rusqlite::Row) -> Result<Event> {
    Ok(Event {
        id: row.get("id")?,
        date: row.get("date")?,
        time: row.get("time")?,
        kind: row.get("kind")?,
        position: row.get("position")?,
        lunch_break: row.get("lunch_break")?,
        pair: row.get("pair")?,
        source: row.get("source")?,
        meta: row.get("meta")?,
        created_at: row.get("created_at")?,
    })
}

// Generic helper to build a query with optional filters for period and position
fn build_filtered_query(
    base_query: &str,
    period: Option<&str>,
    pos: Option<&str>,
) -> Result<(String, Vec<String>)> {
    let mut query = base_query.to_string();
    let mut conditions = Vec::new();
    let mut params: Vec<String> = Vec::new();

    if let Some(p) = period {
        // Nuova sintassi: range con ":" → start:end
        if let Some((start_raw, end_raw)) = p.split_once(':') {
            let start = start_raw.trim();
            let end = end_raw.trim();

            if start.is_empty() || end.is_empty() || start.len() != end.len() {
                return Err(rusqlite::Error::InvalidQuery);
            }

            match start.len() {
                4 => {
                    // Range di anni: es. "2024:2025"
                    conditions.push("strftime('%Y', date) >= ?".to_string());
                    conditions.push("strftime('%Y', date) <= ?".to_string());
                    params.push(start.to_string());
                    params.push(end.to_string());
                }
                7 => {
                    // Range di mesi: es. "2025-01:2025-03"
                    conditions.push("strftime('%Y-%m', date) >= ?".to_string());
                    conditions.push("strftime('%Y-%m', date) <= ?".to_string());
                    params.push(start.to_string());
                    params.push(end.to_string());
                }
                10 => {
                    // Range di giorni: es. "2025-06-01:2025-06-30"
                    conditions.push("date >= ?".to_string());
                    conditions.push("date <= ?".to_string());
                    params.push(start.to_string());
                    params.push(end.to_string());
                }
                _ => return Err(rusqlite::Error::InvalidQuery),
            }
        } else if p.len() == 4 {
            // Solo anno: "2025"
            conditions.push("strftime('%Y', date) = ?".to_string());
            params.push(p.to_string());
        } else if p.len() == 7 {
            // Solo mese: "2025-06"
            conditions.push("strftime('%Y-%m', date) = ?".to_string());
            params.push(p.to_string());
        } else if p.len() == 10 {
            // Giorno singolo: "2025-06-01"
            conditions.push("date = ?".to_string());
            params.push(p.to_string());
        } else {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    if let Some(pos_filter) = pos {
        conditions.push("position = ?".to_string());
        params.push(pos_filter.to_uppercase());
    }

    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    Ok((query, params))
}

/// Initialize the database schema.
/// Ensures table `work_sessions` exists and adds missing `position` column if required.
pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS work_sessions (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            date         TEXT NOT NULL,          -- YYYY-MM-DD
            position     TEXT NOT NULL DEFAULT 'O' CHECK (position IN ('O','R','H','C','M')),
            start_time   TEXT NOT NULL DEFAULT '',
            lunch_break  INTEGER NOT NULL DEFAULT 0,
            end_time     TEXT NOT NULL DEFAULT '',
            work_duration INTEGER DEFAULT 0  -- minuti netti: (end-start)-lunch
        );

        CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            operation TEXT NOT NULL,
            target TEXT DEFAULT '',
            message TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,          -- YYYY-MM-DD
            time TEXT NOT NULL,          -- HH:MM
            kind TEXT NOT NULL CHECK (kind IN ('in','out')),
            position TEXT NOT NULL CHECK (position IN ('O','R','H','C','M')),
            lunch_break INTEGER NOT NULL DEFAULT 0, -- minutes, typically set on out
            pair INTEGER DEFAULT 0,
            source TEXT NOT NULL,
            meta TEXT,
            created_at TEXT NOT NULL     -- ISO 8601 timestamp
        );
        ",
    )?;
    run_pending_migrations(conn)?;
    Ok(())
}

/// Aggregate the day's position using events: if no events, returns None.
/// If all event positions are the same, returns that position ("O","R","H","C").
/// If multiple distinct positions exist, returns "M" for Mixed.
pub fn aggregate_position_from_events(conn: &Connection, date: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare_cached("SELECT DISTINCT position FROM events WHERE date = ?1")?;
    let rows = stmt.query_map([date], |row| row.get::<_, String>(0))?;
    use std::collections::HashSet;
    let mut set: HashSet<String> = HashSet::new();
    for r in rows {
        set.insert(r?);
        if set.len() > 1 {
            return Ok(Some("M".to_string()));
        }
    }
    // zero or one distinct positions
    if let Some(pos) = set.into_iter().next() {
        Ok(Some(pos))
    } else {
        Ok(None)
    }
}

/// Insert a new work session
pub fn add_session(
    conn: &Connection,
    date: &str,
    position: &str,
    start: &str,
    lunch: u32,
    end: &str,
) -> Result<()> {
    conn.execute(
        "INSERT INTO work_sessions (date, position, start_time, lunch_break, end_time)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![date, position, start, lunch, end],
    )?;
    Ok(())
}

pub fn delete_session(conn: &Connection, id: i32) -> Result<usize> {
    conn.execute("DELETE FROM work_sessions WHERE id = ?", [id])
}

/// Delete all work_sessions for a given date. Returns number of rows deleted.
pub fn delete_sessions_by_date(conn: &Connection, date: &str) -> Result<usize> {
    conn.execute("DELETE FROM work_sessions WHERE date = ?1", params![date])
}

/// Delete all events for a given date. Returns number of rows deleted.
pub fn delete_events_by_date(conn: &Connection, date: &str) -> Result<usize> {
    conn.execute("DELETE FROM events WHERE date = ?1", params![date])
}

/// Delete events by a list of ids. Returns number of rows deleted.
pub fn delete_events_by_ids(conn: &Connection, ids: &[i32]) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }
    // Build a query with the appropriate number of placeholders
    let mut sql = String::from("DELETE FROM events WHERE id IN (");
    sql.push_str(&vec!["?"; ids.len()].join(","));
    sql.push(')');
    let params_vec: Vec<&dyn ToSql> = ids.iter().map(|i| i as &dyn ToSql).collect();
    let mut stmt = conn.prepare_cached(&sql)?;
    stmt.execute(rusqlite::params_from_iter(params_vec))?;
    Ok(ids.len())
}

/// Return all saved work sessions, optionally filtered by year or year-month.
pub fn list_sessions(
    conn: &Connection,
    period: Option<&str>,
    pos: Option<&str>,
) -> Result<Vec<WorkSession>> {
    // NEW: support for --period all
    if let Some("all") = period {
        let mut sql =
            "SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions"
                .to_string();

        let mut owned_params: Vec<String> = Vec::new();
        let mut param_refs: Vec<&dyn ToSql> = Vec::new();

        apply_position_filter(&mut sql, pos, &mut owned_params, &mut param_refs);

        sql.push_str(" ORDER BY date ASC");

        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), row_to_worksession)?;
        return rows.collect::<Result<Vec<_>, _>>();
    }

    // DEFAULT PATH (filtered)
    let base_query =
        "SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions";
    let (mut query, params) = build_filtered_query(base_query, period, pos)?;

    query.push_str(" ORDER BY date ASC");

    let mut stmt = conn.prepare_cached(&query)?;
    let params_refs: Vec<&dyn ToSql> = params.iter().map(|s| s as &dyn ToSql).collect();
    let rows = stmt.query_map(params_refs.as_slice(), row_to_worksession)?;

    rows.collect::<Result<Vec<_>, _>>()
}

/// Generic upsert helper for a single field in `work_sessions` table.
fn upsert_field<T: ToSql>(
    conn: &Connection,
    date: &str,
    field: &str,
    value: T,
    default_pos: &str,
) -> Result<()> {
    let update_sql = format!("UPDATE work_sessions SET {} = ?1 WHERE date = ?2", field);
    let mut stmt = conn.prepare_cached(&update_sql)?;
    let rows = stmt.execute(params![&value, date])?;

    if rows == 0 {
        let insert_sql = format!(
            "INSERT INTO work_sessions (date, position, {}) VALUES (?1, ?2, ?3)",
            field
        );
        let mut ins = conn.prepare_cached(&insert_sql)?;
        ins.execute(params![date, default_pos, &value])?;
    }
    Ok(())
}

/// Insert or update the position (A=office, R=remote) for a given date.
pub fn upsert_position(conn: &Connection, date: &str, pos: &str) -> Result<()> {
    upsert_field(conn, date, "position", pos, pos)
}

/// Insert or update the start time (HH:MM) for a given date.
pub fn upsert_start(conn: &Connection, date: &str, start: &str) -> Result<()> {
    // Custom logic: only update if start_time is empty
    let mut stmt = conn.prepare_cached(
        "UPDATE work_sessions SET start_time = ?1 WHERE date = ?2 AND (start_time = '' OR start_time IS NULL)",
    )?;
    if stmt.execute(params![start, date])? == 0 {
        let exists = conn
            .query_row(
                "SELECT 1 FROM work_sessions WHERE date = ?1",
                [date],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            upsert_field(conn, date, "start_time", start, "O")?;
        }
    }
    Ok(())
}

/// Insert or update the lunch break (minutes) for a given date.
pub fn upsert_lunch(conn: &Connection, date: &str, lunch: i32) -> Result<()> {
    upsert_field(conn, date, "lunch_break", lunch, "O")
}

/// Insert or update the end time (HH:MM) for a given date.
pub fn upsert_end(conn: &Connection, date: &str, end: &str) -> Result<()> {
    upsert_field(conn, date, "end_time", end, "O")
}

pub fn ttlog(conn: &Connection, operation: &str, target: &str, message: &str) -> Result<()> {
    let now = Utc::now().to_rfc3339(); // ISO 8601
    let mut stmt = conn.prepare_cached(
        "INSERT INTO log (date, operation, target, message) VALUES (?1, ?2, ?3, ?4)",
    )?;
    stmt.execute(params![&now, operation, target, message])?;
    Ok(())
}

/// Retrieve a single work session by id
pub fn get_session(conn: &Connection, id: i32) -> Result<Option<WorkSession>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions WHERE id = ?1",
    )?;

    match stmt.query_row([id], row_to_worksession) {
        Ok(ws) => Ok(Some(ws)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Retrieve work sessions for a specific date
pub fn list_sessions_by_date(conn: &Connection, date: &str) -> Result<Vec<WorkSession>> {
    let query = "SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions WHERE date = ?1 ORDER BY date ASC";
    let mut stmt = conn.prepare_cached(query)?;
    let rows = stmt.query_map([date], row_to_worksession)?;

    rows.collect::<Result<Vec<_>, _>>()
}

/// List events for a specific date (ordered by time asc)
pub fn list_events_by_date(conn: &Connection, date: &str) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at \
        FROM events \
        WHERE date = ?1 \
        ORDER BY time ASC",
    )?;
    let rows = stmt.query_map([date], row_to_event)?;

    rows.collect::<Result<Vec<_>, _>>()
}

/// List all events in the database ordered by date and time
pub fn list_events(conn: &Connection) -> Result<Vec<Event>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at \
        FROM events \
        ORDER BY date ASC, time ASC",
    )?;
    let rows = stmt.query_map([], row_to_event)?;

    rows.collect::<Result<Vec<_>, _>>()
}

/// Append a position filter to SQL query while safely managing lifetimes.
/// Stores the uppercase copy into owned_params so references remain valid.
fn apply_position_filter<'a>(
    sql: &mut String,
    pos: Option<&str>,
    owned_params: &'a mut Vec<String>,
    param_refs: &mut Vec<&'a dyn ToSql>,
) {
    if let Some(p) = pos {
        let upper = p.to_uppercase();
        sql.push_str(" WHERE position = ?1");
        owned_params.push(upper);
        param_refs.push(&owned_params[owned_params.len() - 1]);
    }
}

/// List events filtered by optional period (YYYY or YYYY-MM) and position
pub fn list_events_filtered(
    conn: &Connection,
    period: Option<&str>,
    pos: Option<&str>,
) -> Result<Vec<Event>> {
    // NEW: support for --period all
    if let Some("all") = period {
        let mut sql = "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at FROM events"
            .to_string();

        let mut owned_params: Vec<String> = Vec::new();
        let mut param_refs: Vec<&dyn ToSql> = Vec::new();

        apply_position_filter(&mut sql, pos, &mut owned_params, &mut param_refs);

        sql.push_str(" ORDER BY date ASC, time ASC");

        let mut stmt = conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(param_refs.as_slice(), row_to_event)?;
        return rows.collect::<Result<Vec<_>, _>>();
    }

    // DEFAULT PATH (filtered)
    let base_query = "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at FROM events";
    let (mut query, params) = build_filtered_query(base_query, period, pos)?;

    query.push_str(" ORDER BY date ASC, time ASC");

    let mut stmt = conn.prepare_cached(&query)?;
    let param_refs: Vec<&dyn ToSql> = params.iter().map(|s| s as &dyn ToSql).collect();
    let rows = stmt.query_map(param_refs.as_slice(), row_to_event)?;

    rows.collect::<Result<Vec<_>, _>>()
}

/// Find last out event before a given time on the same date
pub fn last_out_before(conn: &Connection, date: &str, time: &str) -> Result<Option<Event>> {
    let mut stmt = conn.prepare_cached(
        "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at \
        FROM events \
        WHERE date = ?1 AND kind = 'out' AND time < ?2 \
        ORDER BY time DESC \
        LIMIT 1",
    )?;
    match stmt.query_row([date, time], row_to_event) {
        Ok(ev) => Ok(Some(ev)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Update lunch_break for a specific event (typically an 'out')
pub fn set_event_lunch(conn: &Connection, event_id: i32, lunch: i32) -> Result<()> {
    conn.execute(
        "UPDATE events SET lunch_break = ?1 WHERE id = ?2",
        params![lunch, event_id],
    )?;
    Ok(())
}

/// Update time for a specific event
pub fn set_event_time(conn: &Connection, event_id: i32, new_time: &str) -> Result<()> {
    conn.execute(
        "UPDATE events SET time = ?1 WHERE id = ?2",
        params![new_time, event_id],
    )?;
    Ok(())
}

/// Update position for a specific event
pub fn set_event_position(conn: &Connection, event_id: i32, new_pos: &str) -> Result<()> {
    conn.execute(
        "UPDATE events SET position = ?1 WHERE id = ?2",
        params![new_pos, event_id],
    )?;
    Ok(())
}

// Helper used by force_set_* to update or insert a legacy work_sessions row when forcing a single field.
fn force_set_field<T: ToSql>(
    conn: &Connection,
    date: &str,
    field: &str,
    value: T,
    default_pos: &str,
) -> Result<()> {
    let sql = format!("UPDATE work_sessions SET {} = ?1 WHERE date = ?2", field);
    let changed = conn.execute(&sql, params![&value, date])?;
    if changed == 0 {
        // Insert full row with only this field populated (others default)
        let insert_sql = match field {
            "position" => "INSERT INTO work_sessions (date, position) VALUES (?1, ?2)",
            "start_time" => {
                "INSERT INTO work_sessions (date, position, start_time) VALUES (?1, ?2, ?3)"
            }
            "end_time" => {
                "INSERT INTO work_sessions (date, position, end_time) VALUES (?1, ?2, ?3)"
            }
            "lunch_break" => {
                "INSERT INTO work_sessions (date, position, lunch_break) VALUES (?1, ?2, ?3)"
            }
            _ => "INSERT INTO work_sessions (date, position) VALUES (?1, ?2)",
        };
        match field {
            "position" => {
                conn.execute(insert_sql, params![date, &value])?;
            }
            "start_time" | "end_time" | "lunch_break" => {
                conn.execute(insert_sql, params![date, default_pos, &value])?;
            }
            _ => {
                conn.execute(
                    "INSERT INTO work_sessions (date, position) VALUES (?1, ?2)",
                    params![date, default_pos],
                )?;
            }
        }
    }
    Ok(())
}

pub fn force_set_position(conn: &Connection, date: &str, pos: &str) -> Result<()> {
    force_set_field(conn, date, "position", pos, pos)
}

pub fn force_set_start(conn: &Connection, date: &str, start: &str) -> Result<()> {
    force_set_field(conn, date, "start_time", start, "O")
}

pub fn force_set_end(conn: &Connection, date: &str, end: &str) -> Result<()> {
    force_set_field(conn, date, "end_time", end, "O")
}

pub fn force_set_lunch(conn: &Connection, date: &str, lunch: i32) -> Result<()> {
    force_set_field(conn, date, "lunch_break", lunch, "O")
}

pub fn count_events_by_date(conn: &Connection, date: &str) -> Result<i64> {
    let mut stmt = conn.prepare_cached("SELECT COUNT(*) FROM events WHERE date = ?1")?;
    let n: i64 = stmt.query_row([date], |r| r.get(0))?;
    Ok(n)
}

pub fn count_sessions_by_date(conn: &Connection, date: &str) -> Result<i64> {
    let mut stmt = conn.prepare_cached("SELECT COUNT(*) FROM work_sessions WHERE date = ?1")?;
    let n: i64 = stmt.query_row([date], |r| r.get(0))?;
    Ok(n)
}

// Struct per passare argomenti alla funzione add_event
pub struct AddEventArgs<'a> {
    pub date: &'a str,
    pub time: &'a str,
    pub kind: &'a str,
    pub position: Option<&'a str>,
    pub source: &'a str,
    pub meta: Option<&'a str>,
}

/// Insert an event and run auto-lunch logic if kind == 'in'.
/// This function uses a transaction to ensure atomicity. It also performs dual-write
/// to legacy `work_sessions` via existing upsert_* helpers to keep backwards compatibility.
#[allow(clippy::too_many_arguments)]
pub fn add_event(
    conn: &mut Connection,
    args: &AddEventArgs,
    config: &crate::config::Config,
) -> Result<i64> {
    let tx = conn.transaction()?;

    // Determine position_to_use:
    // - if user provided position (Some) -> use it
    // - else if kind == 'out' -> try to inherit from last 'in' on the same date
    // - otherwise fallback to config.default_position
    let mut position_to_use = if let Some(p) = args.position {
        p.to_string()
    } else {
        config.default_position.clone()
    };
    if args.position.is_none() && args.kind == "out" {
        let mut stmt = tx.prepare_cached(
            "SELECT position FROM events WHERE date = ?1 AND kind = 'in' AND time <= ?2 ORDER BY time DESC LIMIT 1",
        )?;
        if let Some(found_pos) = stmt
            .query_row([args.date, args.time], |row| row.get::<_, String>(0))
            .optional()?
        {
            position_to_use = found_pos;
        }
    }

    tx.execute(
        "INSERT INTO events (date, time, kind, position, lunch_break, source, meta, created_at) VALUES (?1, ?2, ?3, ?4, 0, ?5, ?6, ?7)",
        params![args.date, args.time, args.kind, position_to_use, args.source, args.meta.unwrap_or(""), Utc::now().to_rfc3339()],
    )?;

    let event_id = tx.last_insert_rowid();

    // Dual-write to legacy table to ease rollout
    if args.kind == "in" {
        // store start in legacy place
        let _ = upsert_start(&tx, args.date, args.time);
    } else if args.kind == "out" {
        let _ = upsert_end(&tx, args.date, args.time);
    }

    // If this is an 'in' event, attempt to populate lunch on the previous 'out' (auto-lunch)
    if args.kind == "in"
        && let Some(prev_out) = last_out_before(&tx, args.date, args.time)?
    {
        // Exclude holiday positions
        if prev_out.position != "H" && position_to_use != "H" {
            // Ensure previous out has no lunch yet
            if prev_out.lunch_break == 0 {
                // Parse times
                if let (Ok(prev_time), Ok(new_time)) = (
                    NaiveTime::parse_from_str(&prev_out.time, "%H:%M"),
                    NaiveTime::parse_from_str(args.time, "%H:%M"),
                ) {
                    let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
                    let latest = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
                    if prev_time >= noon && new_time <= latest && new_time > prev_time {
                        let delta = (new_time - prev_time).num_minutes() as i32;
                        let mut lunch_val = delta;
                        if lunch_val < config.min_duration_lunch_break {
                            lunch_val = config.min_duration_lunch_break;
                        }
                        if lunch_val > config.max_duration_lunch_break {
                            lunch_val = config.max_duration_lunch_break;
                        }
                        if lunch_val > 0 {
                            tx.execute(
                                "UPDATE events SET lunch_break = ?1 WHERE id = ?2",
                                params![lunch_val, prev_out.id],
                            )?;
                            // also update legacy work_sessions lunch for compatibility
                            let _ = upsert_lunch(&tx, args.date, lunch_val);
                            // write an audit log entry inside the same transaction
                            let msg = format!(
                                "auto_lunch {} min for out_event {} (date={})",
                                lunch_val, prev_out.id, args.date
                            );
                            tx.execute(
                                "INSERT INTO log (date, operation, message) VALUES (?1, ?2, ?3)",
                                params![Utc::now().to_rfc3339(), "auto_lunch", msg],
                            )?;
                        }
                    }
                }
            }
        }
    }

    tx.commit()?;
    Ok(event_id)
}

/// Reconstruct work sessions from events for a given date.
/// Produces one WorkSession per matched in/out pair (or partial if unmatched).
pub fn reconstruct_sessions_from_events(conn: &Connection, date: &str) -> Result<Vec<WorkSession>> {
    let events = list_events_by_date(conn, date)?;
    let mut sessions: Vec<WorkSession> = Vec::new();

    let mut pending_in: Option<Event> = None;

    for e in events {
        if e.kind == "in" {
            // treat latest 'in' as the pending entry
            pending_in = Some(e);
        } else if e.kind == "out" {
            if let Some(in_ev) = pending_in.take() {
                // avoid cloning Event strings; use references
                let work_duration =
                    calculate_work_duration(in_ev.time.as_str(), e.time.as_str(), e.lunch_break);
                // matched pair
                let ws = WorkSession {
                    id: e.id, // use out event id as session id
                    date: date.to_string(),
                    position: in_ev.position.clone(),
                    start: in_ev.time.clone(),
                    lunch: e.lunch_break,
                    end: e.time.clone(),
                    work_duration,
                };
                sessions.push(ws);
            } else {
                let work_duration = calculate_work_duration("", e.time.as_str(), e.lunch_break);
                // out without in -> partial session
                let ws = WorkSession {
                    id: e.id,
                    date: date.to_string(),
                    position: e.position.clone(),
                    start: "".to_string(),
                    lunch: e.lunch_break,
                    end: e.time.clone(),
                    work_duration,
                };
                sessions.push(ws);
            }
        }
    }

    // any remaining pending_in -> incomplete session
    if let Some(in_ev) = pending_in {
        let work_duration = calculate_work_duration(in_ev.time.as_str(), "", 0);
        let ws = WorkSession {
            id: in_ev.id,
            date: date.to_string(),
            position: in_ev.position.clone(),
            start: in_ev.time.clone(),
            lunch: 0,
            end: "".to_string(),
            work_duration,
        };
        sessions.push(ws);
    }

    Ok(sessions)
}

/// Delete events by ids and recompute/update work_sessions for the given date atomically.
pub fn delete_events_by_ids_and_recompute_sessions(
    conn: &mut Connection,
    ids: &[i32],
    date: &str,
) -> Result<usize> {
    if ids.is_empty() {
        return Ok(0);
    }

    let tx = conn.transaction()?;

    // Execute delete inside a narrow scope so statement is dropped early
    let deleted = {
        // Build delete SQL
        let mut sql = String::from("DELETE FROM events WHERE id IN (");
        sql.push_str(&vec!["?"; ids.len()].join(","));
        sql.push(')');
        let params_vec: Vec<&dyn ToSql> = ids.iter().map(|i| i as &dyn ToSql).collect();
        let mut del_stmt = tx.prepare(&sql)?;

        del_stmt.execute(rusqlite::params_from_iter(params_vec))?
    };

    // Query remaining events for the date inside the same transaction; keep statement scoped
    let mut remaining: Vec<Event> = Vec::new();
    {
        let mut sel = tx.prepare(
            "SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at \
            FROM events \
            WHERE date = ?1 \
            ORDER BY time ASC",
        )?;
        let remaining_rows = sel.query_map([date], row_to_event)?;
        for r in remaining_rows {
            remaining.push(r?);
        }
        // sel and remaining_rows dropped here
    }

    if remaining.is_empty() {
        // delete work_sessions row(s) for date
        tx.execute("DELETE FROM work_sessions WHERE date = ?1", params![date])?;
    } else {
        // end_time = max time among remaining events
        if let Some(max_time) = remaining.iter().map(|e| e.time.clone()).max() {
            // Update or insert end_time via existing helper using the transaction
            // We'll use direct SQL to update within the tx (force_set_end uses Connection methods)
            let changed = tx.execute(
                "UPDATE work_sessions SET end_time = ?1 WHERE date = ?2",
                params![&max_time, date],
            )?;
            if changed == 0 {
                tx.execute(
                    "INSERT INTO work_sessions (date, position, end_time) VALUES (?1, ?2, ?3)",
                    params![date, "O", &max_time],
                )?;
            }
        }

        // start_time: choose earliest 'in' if any, otherwise earliest event time
        let min_time_opt = remaining
            .iter()
            .filter(|e| e.kind == "in")
            .map(|e| e.time.clone())
            .min()
            .or_else(|| remaining.iter().map(|e| e.time.clone()).min());
        if let Some(min_time) = min_time_opt {
            let changed = tx.execute(
                "UPDATE work_sessions SET start_time = ?1 WHERE date = ?2",
                params![&min_time, date],
            )?;
            if changed == 0 {
                tx.execute(
                    "INSERT INTO work_sessions (date, position, start_time) VALUES (?1, ?2, ?3)",
                    params![date, "O", &min_time],
                )?;
            }
        }

        // lunch_break: get the latest 'out' event (max time among kind='out') and use its lunch_break
        let last_out = remaining
            .iter()
            .filter(|e| e.kind == "out")
            .max_by(|a, b| a.time.cmp(&b.time));
        if let Some(out_ev) = last_out {
            let lunch_val = out_ev.lunch_break;
            let changed = tx.execute(
                "UPDATE work_sessions SET lunch_break = ?1 WHERE date = ?2",
                params![lunch_val, date],
            )?;
            if changed == 0 {
                tx.execute(
                    "INSERT INTO work_sessions (date, position, lunch_break) VALUES (?1, ?2, ?3)",
                    params![date, "O", lunch_val],
                )?;
            }
        }

        // position: if all remaining events share same position, set it; otherwise leave as-is
        // Use a single SQL query to determine if there's exactly one distinct position left.
        // This pushes the distinct/count work to SQLite and avoids materializing positions in Rust.
        let mut pos_stmt = tx.prepare(
            "SELECT COUNT(DISTINCT position) as cnt, MIN(position) as pos FROM events WHERE date = ?1",
        )?;
        let (cnt, pos_opt): (i64, Option<String>) =
            pos_stmt.query_row([date], |r| Ok((r.get(0)?, r.get(1)?)))?;
        if cnt == 1
            && let Some(pos) = pos_opt
        {
            let changed = tx.execute(
                "UPDATE work_sessions SET position = ?1 WHERE date = ?2",
                params![pos, date],
            )?;
            if changed == 0 {
                tx.execute(
                    "INSERT INTO work_sessions (date, position) VALUES (?1, ?2)",
                    params![date, pos],
                )?;
            }
        }
    }

    tx.commit()?;
    Ok(deleted)
}

#[derive(Debug)]
struct EventRow {
    time: String,
    kind: String,     // "in" | "out"
    position: String, // "O", "R", "H", "C", "M"
    lunch_break: i64, // minuti
    pair: i64,
}

/// Ricostruisce completamente la tabella `work_sessions` a partire da `events`.
/// Pensata per la versione 0.7.x (vecchia architettura).
pub fn rebuild_work_sessions(conn: &Connection) -> Result<u32> {
    // BEGIN
    conn.execute("BEGIN IMMEDIATE TRANSACTION", [])?;

    // Backup semplice
    conn.execute("DROP TABLE IF EXISTS work_sessions_backup", [])?;
    conn.execute(
        "CREATE TABLE work_sessions_backup AS SELECT * FROM work_sessions",
        [],
    )?;

    // Svuota la tabella
    conn.execute("DELETE FROM work_sessions", [])?;

    // Per contare quante righe inseriamo
    let mut inserted_rows: u32 = 0;

    // Prendi tutte le date
    let mut dates_stmt = conn.prepare("SELECT DISTINCT date FROM events ORDER BY date")?;
    let dates_iter = dates_stmt.query_map([], |row| row.get::<_, String>(0))?;

    for date_res in dates_iter {
        let date: String = date_res?;

        // Prendi gli eventi del giorno
        let mut ev_stmt = conn.prepare(
            r#"
            SELECT time, kind, position, lunch_break, pair
            FROM events
            WHERE date = ?
            ORDER BY pair, time
            "#,
        )?;

        let ev_iter = ev_stmt.query_map(params![&date], |row| {
            Ok(EventRow {
                time: row.get(0)?,
                kind: row.get(1)?,
                position: row.get(2)?,
                lunch_break: row.get(3)?,
                pair: row.get(4)?,
            })
        })?;

        let mut events: Vec<EventRow> = Vec::new();
        for ev in ev_iter {
            events.push(ev?);
        }

        if events.is_empty() {
            continue;
        }

        // ---- calcolo position ----
        let mut positions: HashSet<String> = HashSet::new();
        let mut total_lunch_break: i64 = 0;

        for ev in &events {
            positions.insert(ev.position.clone());
            total_lunch_break += ev.lunch_break;
        }

        let position = if positions.len() == 1 {
            positions.into_iter().next().unwrap()
        } else {
            "M".to_string()
        };

        // ---- raggruppo per pair ----
        #[derive(Default)]
        struct PairTimes {
            in_time: Option<NaiveTime>,
            out_time: Option<NaiveTime>,
        }

        let mut pairs: HashMap<i64, PairTimes> = HashMap::new();

        for ev in &events {
            let t = match parse_time(&ev.time) {
                Some(t) => t,
                None => {
                    eprintln!(
                        "Invalid time '{}' on date {}, skipping event",
                        ev.time, date
                    );
                    continue;
                }
            };

            let entry = pairs.entry(ev.pair).or_default();

            match ev.kind.as_str() {
                "in" => {
                    entry.in_time = match entry.in_time {
                        Some(existing) => Some(existing.min(t)),
                        None => Some(t),
                    };
                }
                "out" => {
                    entry.out_time = match entry.out_time {
                        Some(existing) => Some(existing.max(t)),
                        None => Some(t),
                    };
                }
                _ => {
                    eprintln!("Unknown event kind '{}'", ev.kind);
                }
            }
        }

        // ---- calcolo della giornata ----
        let mut earliest_in: Option<NaiveTime> = None;
        let mut latest_out: Option<NaiveTime> = None;
        let mut total_work_minutes: i64 = 0;

        for pt in pairs.values() {
            match (pt.in_time, pt.out_time) {
                (Some(t_in), Some(t_out)) => {
                    earliest_in = Some(match earliest_in {
                        Some(existing) => existing.min(t_in),
                        None => t_in,
                    });

                    latest_out = Some(match latest_out {
                        Some(existing) => existing.max(t_out),
                        None => t_out,
                    });

                    let diff = t_out - t_in;
                    let minutes = diff.num_minutes();
                    if minutes > 0 {
                        total_work_minutes += minutes;
                    }
                }
                (Some(t_in), None) => {
                    earliest_in = Some(match earliest_in {
                        Some(existing) => existing.min(t_in),
                        None => t_in,
                    });
                }
                _ => {}
            }
        }

        let earliest_in = match earliest_in {
            Some(t) => t,
            None => {
                eprintln!("No IN event found for {}, skipping day", date);
                continue;
            }
        };

        // Se manca OUT → comunque creare il record
        let (end_time_str, effective_work_minutes) = match latest_out {
            Some(out_t) => {
                let diff_minutes = total_work_minutes;
                (out_t.format("%H:%M").to_string(), diff_minutes)
            }
            None => ("".to_string(), 0),
        };

        let start_time_str = earliest_in.format("%H:%M").to_string();

        // Applico il lunch break
        let mut work_duration = effective_work_minutes - total_lunch_break;
        if work_duration < 0 {
            work_duration = 0;
        }

        // Inserisci la giornata in work_sessions
        conn.execute(
            r#"
            INSERT INTO work_sessions
                (date, position, start_time, lunch_break, end_time, work_duration)
            VALUES
                (?1,  ?2,       ?3,         ?4,          ?5,        ?6)
            "#,
            params![
                date,
                position,
                start_time_str,
                total_lunch_break,
                end_time_str,
                work_duration
            ],
        )?;

        inserted_rows += 1;
    }

    // Se siamo arrivati qui, il rebuild è andato a buon fine.
    // È sicuro eliminare il backup.
    conn.execute("DROP TABLE IF EXISTS work_sessions_backup", [])?;

    conn.execute("COMMIT", [])?;
    Ok(inserted_rows)
}

/// Prova a parsare "HH:MM" o "HH:MM:SS".
fn parse_time(s: &str) -> Option<NaiveTime> {
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M:%S") {
        return Some(t);
    }
    if let Ok(t) = NaiveTime::parse_from_str(s, "%H:%M") {
        return Some(t);
    }
    None
}
