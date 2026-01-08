use crate::db::pool::DbPool;
use rusqlite::Result;

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
