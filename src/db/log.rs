use crate::errors::AppResult;
use chrono::Local;
use rusqlite::Connection;
use rusqlite::params;

/// Write an internal log line into the `log` table.
pub fn ttlog(conn: &Connection, operation: &str, target: &str, message: &str) -> AppResult<()> {
    // Timestamp locale, formattato in ISO 8601
    let now = Local::now().to_rfc3339();

    let mut stmt = conn.prepare_cached(
        "INSERT INTO log (date, operation, target, message)
         VALUES (?1, ?2, ?3, ?4)",
    )?;

    stmt.execute(params![now, operation, target, message])?;

    Ok(())
}
