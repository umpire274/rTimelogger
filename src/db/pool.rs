//! SQLite connection pool wrapper (lightweight for CLI usage).

use rusqlite::{Connection, Result};
use std::path::Path;

pub struct DbPool {
    pub conn: Connection,
}

impl DbPool {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(Path::new(path))?;
        Ok(Self { conn })
    }

    /// Helper to execute a closure with a mutable connection reference.
    pub fn with_conn<F, T>(&mut self, func: F) -> Result<T>
    where
        F: FnOnce(&mut Connection) -> Result<T>,
    {
        func(&mut self.conn)
    }
}
