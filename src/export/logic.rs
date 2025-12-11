// src/export/logic.rs

use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::export::ExportFormat;
use crate::export::fs_utils::ensure_writable;
use crate::export::model::EventExport;
use crate::export::range::parse_range;
use crate::ui::messages::warning;

use crate::export::json_csv::{export_csv, export_json};
use crate::export::pdf_export::export_pdf;
use crate::export::xlsx::export_xlsx;
use chrono::NaiveDate;
use rusqlite::Row;
use rusqlite::params;
use std::io;
use std::path::Path;

/// Logica di alto livello per l'export.
pub struct ExportLogic;

impl ExportLogic {
    /// Export degli eventi.
    ///
    /// - `format`: "csv" | "json" | "xlsx" | "pdf"
    /// - `file`: path assoluto del file di output
    /// - `range`: `None`, `"all"` oppure espressioni come:
    ///   - `YYYY`
    ///   - `YYYY-MM`
    ///   - `YYYY-MM-DD`
    ///   - `YYYY:YYYY`
    ///   - `YYYY-MM:YYYY-MM`
    ///   - `YYYY-MM-DD:YYYY-MM-DD`
    pub fn export(
        pool: &mut DbPool,
        format: ExportFormat,
        file: &str,
        range: &Option<String>,
        _events: bool,
        force: bool,
    ) -> AppResult<()> {
        let path = Path::new(file);

        if !path.is_absolute() {
            return Err(AppError::from(io::Error::other(format!(
                "Output file path must be absolute: {file}"
            ))));
        }

        ensure_writable(path, force)?;

        let date_bounds: Option<(NaiveDate, NaiveDate)> = match range {
            None => None,
            Some(r) if r.eq_ignore_ascii_case("all") => None,
            Some(r) => Some(parse_range(r)?),
        };

        let events_vec = load_events(pool, date_bounds)?;

        if events_vec.is_empty() {
            warning("⚠️  No events found for selected range.");
            return Ok(());
        }

        match format {
            ExportFormat::Csv => export_csv(&events_vec, path)?,
            ExportFormat::Json => export_json(&events_vec, path)?,
            ExportFormat::Xlsx => export_xlsx(&events_vec, path)?,
            ExportFormat::Pdf => {
                let title = build_pdf_title(range);
                export_pdf(&events_vec, path, &title)?
            }
        }

        Ok(())
    }
}

/// Costruisce il titolo del PDF in base al periodo selezionato.
fn build_pdf_title(period: &Option<String>) -> String {
    // Nessun periodo → titolo generico
    if period.is_none() {
        return "Saved sessions".to_string();
    }

    let p = period.as_ref().unwrap();

    match p.len() {
        4 => {
            // YYYY
            format!("Saved sessions for year {}", p)
        }

        7 => {
            // YYYY-MM
            let parts: Vec<&str> = p.split('-').collect();
            if parts.len() == 2 {
                let month = crate::utils::date::month_name(parts[1]);
                format!("Saved sessions for {} {}", month, parts[0])
            } else {
                "Saved sessions".to_string()
            }
        }

        10 => {
            // YYYY-MM-DD
            format!("Saved session for date {}", p)
        }

        15 => {
            // YYYY-MM-DD:YYYY-MM-DD
            let parts: Vec<&str> = p.split(':').collect();
            if parts.len() == 2 {
                format!("Saved sessions from {} to {}", parts[0], parts[1])
            } else {
                "Saved sessions".to_string()
            }
        }

        _ => "Saved sessions".to_string(),
    }
}

/// Carica gli eventi dal DB secondo i bounds.
fn load_events(
    pool: &mut DbPool,
    bounds: Option<(NaiveDate, NaiveDate)>,
) -> AppResult<Vec<EventExport>> {
    let conn = &mut pool.conn;

    let mut events = Vec::new();

    match bounds {
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, date, time, kind, position, lunch_break, pair, source
                 FROM events
                 ORDER BY date ASC, time ASC",
            )?;

            let rows = stmt.query_map([], map_row)?;

            for r in rows {
                events.push(r?);
            }
        }
        Some((start, end)) => {
            let start_str = start.format("%Y-%m-%d").to_string();
            let end_str = end.format("%Y-%m-%d").to_string();

            let mut stmt = conn.prepare(
                "SELECT id, date, time, kind, position, lunch_break, pair, source
                 FROM events
                 WHERE date BETWEEN ?1 AND ?2
                 ORDER BY date ASC, time ASC",
            )?;

            let rows = stmt.query_map(params![start_str, end_str], map_row)?;

            for r in rows {
                events.push(r?);
            }
        }
    }

    Ok(events)
}

/// Mapping DB → EventExport (riusato per tutte le query).
fn map_row(row: &Row<'_>) -> rusqlite::Result<EventExport> {
    Ok(EventExport {
        id: row.get(0)?,
        date: row.get(1)?,
        time: row.get(2)?,
        kind: row.get(3)?,
        position: row.get(4)?,
        lunch_break: row.get(5)?,
        pair: row.get(6)?,
        source: row.get(7)?,
    })
}
