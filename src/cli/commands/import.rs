use std::fs;

use crate::cli::parser::Commands;
use crate::config::Config;
use crate::errors::{AppError, AppResult};
use crate::import::{import_days_from_str, ImportInputFormat};
use crate::ui::messages::{info, success, warning};

use crate::utils::formatting::build_import_source;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct ImportDayJson {
    date: String,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportJsonRoot {
    Days { days: Vec<ImportDayJson> },
    Holidays { holidays: Vec<ImportDayJson> },
    Array(Vec<ImportDayJson>),
}

fn normalize_json_to_days(content: &str) -> AppResult<String> {
    let parsed: ImportJsonRoot = serde_json::from_str(content).map_err(|e| {
        AppError::InvalidArgs(format!(
            "Invalid JSON. Expected one of: {{\"days\":[...]}}, {{\"holidays\":[...]}}, or a root array [...]. Details: {}",
            e
        ))
    })?;

    let days: Vec<ImportDayJson> = match parsed {
        ImportJsonRoot::Days { days } => days,
        ImportJsonRoot::Holidays { holidays } => holidays,
        ImportJsonRoot::Array(v) => v,
    };

    // Re-emit canonical shape expected by the importer: { "days": [...] }
    serde_json::to_string(&serde_json::json!({ "days": days }))
        .map_err(|e| AppError::Other(format!("Internal error while normalizing JSON: {}", e)))
}

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    let Commands::Import {
        file,
        format,
        dry_run,
        replace,
        source,
    } = cmd
    else {
        return Ok(());
    };

    let mut content = fs::read_to_string(file)?;

    let input_format = match format.to_ascii_lowercase().as_str() {
        "json" => ImportInputFormat::Json,
        "csv" => ImportInputFormat::Csv,
        _ => {
            return Err(AppError::InvalidArgs(
                "Invalid --format. Use 'json' or 'csv'.".into(),
            ));
        }
    };

    // ✅ Normalize JSON shapes (days/holidays/array) into canonical {"days":[...]}
    if matches!(input_format, ImportInputFormat::Json) {
        content = normalize_json_to_days(&content)?;
    }

    let imp_source = build_import_source(source, format);

    let report = import_days_from_str(
        cfg,
        &content,
        input_format,
        *dry_run,
        *replace,
        imp_source.as_str(),
    )?;

    info(format!(
        "Import summary{}:\n- File: {}\n- Format: {}\n- Source: {}\n- Total rows: {}\n- Imported: {}\n- Skipped (already present): {}\n- Conflicts: {}\n- Invalid rows: {}",
        if *dry_run { " (dry-run)" } else { "" },
        file,
        format,
        source,
        report.total,
        report.imported,
        report.skipped_existing,
        report.conflicts,
        report.invalid
    ));

    if report.conflicts > 0 && !*replace {
        warning(
            "Some dates were skipped due to existing work events. Use --replace to override (dangerous).",
        );
    }

    if *dry_run {
        success("Dry-run completed. No changes were applied.");
    } else {
        success("Import completed.");
    }

    Ok(())
}
