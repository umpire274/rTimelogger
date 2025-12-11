// src/export/mod.rs

mod excel_date;
mod fs_utils;
mod json_csv;
pub mod logic;
mod model;
mod pdf;
mod pdf_export;
mod range;
mod xlsx;

pub use logic::ExportLogic;
pub use model::EventExport;

use crate::ui::messages::success;
use clap::ValueEnum;
use std::path::Path;

/// Helper comune per messaggi di completamento export.
pub(crate) fn notify_export_success(label: &str, path: &Path) {
    success(format!("{label} export completed: {}", path.display()));
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ExportFormat {
    Csv,
    Json,
    Xlsx,
    Pdf,
}

impl ExportFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Json => "json",
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Pdf => "pdf",
        }
    }
}
