//! Database row models for events, sessions, logs, etc.
//! These are thin wrappers around SQLite rows.

use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct DbEventRow {
    pub id: i32,
    pub timestamp: DateTime<Local>,
    pub kind: String,
    pub position: String,
    pub lunch: Option<i32>,
    pub work_gap: bool, // Prepared for v0.8 logic
}
