// src/export/json_csv.rs

use crate::errors::{AppError, AppResult};
use crate::export::{EventExport, notify_export_success};
use crate::ui::messages::info;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

/// Export JSON pretty-printed.
pub(crate) fn export_json(events: &[EventExport], path: &Path) -> AppResult<()> {
    info(format!("Exporting to JSON: {}", path.display()));

    let json_data = serde_json::to_string_pretty(events)
        .map_err(|e| AppError::from(io::Error::other(format!("JSON serialization error: {e}"))))?;

    let mut file = File::create(path)?;
    file.write_all(json_data.as_bytes())?;

    notify_export_success("JSON", path);
    Ok(())
}

/// Export CSV (header incluso grazie a serde).
pub(crate) fn export_csv(events: &[EventExport], path: &Path) -> AppResult<()> {
    info(format!("Exporting to CSV: {}", path.display()));

    let mut wtr = csv::Writer::from_path(path)
        .map_err(|e| AppError::from(io::Error::other(format!("CSV open error: {e}"))))?;

    for item in events {
        wtr.serialize(item)
            .map_err(|e| AppError::from(io::Error::other(format!("CSV write error: {e}"))))?;
    }

    wtr.flush()
        .map_err(|e| AppError::from(io::Error::other(format!("CSV flush error: {e}"))))?;

    notify_export_success("CSV", path);
    Ok(())
}
