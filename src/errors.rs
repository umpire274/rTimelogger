//! Unified application error type.
//! All modules (db, core, cli, utils) return AppError to keep the error
//! handling consistent and easy to manage.

use std::io;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    // ---------------------------
    // IO
    // ---------------------------
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    // ---------------------------
    // Database-related
    // ---------------------------
    #[error("Database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("Database migration error: {0}")]
    Migration(String),

    // ---------------------------
    // Parsing errors
    // ---------------------------
    #[error("Invalid date format: {0}")]
    InvalidDate(String),

    #[error("Invalid time format: {0}")]
    InvalidTime(String),

    #[error("Invalid position code: {0}")]
    InvalidPosition(String),

    #[error("Invalid event type: {0}")]
    InvalidEventType(String),

    // ---------------------------
    // Logic errors
    // ---------------------------
    #[error("No events found for date {0}")]
    NoEventsForDate(String),

    #[error("Invalid pair index: {0}")]
    InvalidPair(usize),

    #[error("Timeline error: {0}")]
    Timeline(String),

    #[error("Gap analysis error: {0}")]
    Gap(String),

    // ---------------------------
    // Config errors
    // ---------------------------
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Failed to load configuration")]
    ConfigLoad,

    #[error("Failed to save configuration")]
    ConfigSave,

    // ---------------------------
    // Export errors
    // ---------------------------
    #[error("Export format not supported: {0}")]
    InvalidExportFormat(String),

    #[error("Export error: {0}")]
    Export(String),

    // ---------------------------
    // Generic fallback
    // ---------------------------
    #[error("Internal error: {0}")]
    Other(String),
}

pub type AppResult<T> = Result<T, AppError>;
