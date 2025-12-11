// src/export/pdf_export.rs

use crate::errors::{AppError, AppResult};
use crate::export::model::{events_to_table, get_headers};
use crate::export::pdf::PdfManager;
// già esistente nel tuo progetto
use crate::export::{EventExport, notify_export_success};
use crate::ui::messages::info;
use std::io;
use std::path::Path;

/// Export PDF usando PdfManager e la tabella generata.
pub(crate) fn export_pdf(events: &[EventExport], path: &Path, title: &str) -> AppResult<()> {
    info(format!("Exporting to PDF: {}", path.display()));

    let headers = get_headers();
    let data_vec = events_to_table(events);

    let mut pdf = PdfManager::new();
    pdf.write_table(title, &headers, &data_vec);

    pdf.save(path)
        .map_err(|e| AppError::from(io::Error::other(format!("PDF export error: {e}"))))?;

    notify_export_success("PDF", path);
    Ok(())
}
