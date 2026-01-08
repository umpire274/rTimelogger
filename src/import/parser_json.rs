use crate::errors::{AppError, AppResult};
use crate::import::types::{ImportDay, ImportDayJson};
use crate::models::location::Location;
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportJsonRoot {
    Days { days: Vec<ImportDayJson> },
    Holidays { holidays: Vec<ImportDayJson> },
    Array(Vec<ImportDayJson>),
}

pub(crate) fn parse_json_days(input: &str) -> Vec<AppResult<ImportDay>> {
    let root: ImportJsonRoot = match serde_json::from_str(input) {
        Ok(v) => v,
        Err(e) => {
            return vec![Err(AppError::InvalidArgs(format!(
                "Invalid JSON. Expected one of: {{\"days\":[...]}}, {{\"holidays\":[...]}}, or a root array [...]. Details: {}",
                e
            )))];
        }
    };

    let rows: Vec<ImportDayJson> = match root {
        ImportJsonRoot::Days { days } => days,
        ImportJsonRoot::Holidays { holidays } => holidays,
        ImportJsonRoot::Array(v) => v,
    };

    rows.into_iter()
        .map(|r| {
            let date = NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidDate(r.date.clone()))?;

            let position = match r.position.as_deref() {
                Some(code) => Location::from_db_str(&code.to_uppercase()).ok_or_else(|| {
                    AppError::InvalidPosition(format!("Invalid position '{}'", code))
                })?,
                None => Location::NationalHoliday, // ✅ default
            };

            let meta = r
                .name
                .map(|n| n.trim().to_string())
                .filter(|s| !s.is_empty());

            Ok(ImportDay {
                date,
                position,
                meta,
            })
        })
        .collect()
}
