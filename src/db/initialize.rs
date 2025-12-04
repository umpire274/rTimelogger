use crate::db::migrate::run_pending_migrations;
use crate::errors::AppResult;
use rusqlite::Connection;

/// Initialize the database.
/// Delegates all schema creation / upgrades to the migration engine.
pub fn init_db(conn: &Connection) -> AppResult<()> {
    // NO direct CREATE TABLE here.
    // All schema is guaranteed by migrations.

    run_pending_migrations(conn)?;
    Ok(())
}
