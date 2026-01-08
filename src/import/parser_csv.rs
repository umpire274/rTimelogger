use crate::errors::{AppError, AppResult};
use crate::models::location::Location;
use chrono::NaiveDate;
use serde::Deserialize;

use super::types::ImportDay;

#[derive(Debug, Deserialize)]
struct CsvDay {
    date: String,
    position: String,
    #[serde(default)]
    name: Option<String>,
}

pub(crate) fn parse_csv_days(input: &str) -> Vec<AppResult<ImportDay>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(input.as_bytes());

    let mut out = Vec::new();

    for rec in rdr.deserialize::<CsvDay>() {
        match rec {
            Ok(r) => {
                let date = match NaiveDate::parse_from_str(&r.date, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        out.push(Err(AppError::InvalidDate(format!(
                            "Invalid date: {}",
                            r.date
                        ))));
                        continue;
                    }
                };

                let pos = match Location::from_code(&r.position) {
                    Some(p) => p,
                    None => {
                        out.push(Err(AppError::InvalidPosition(format!(
                            "Invalid position: {}",
                            r.position
                        ))));
                        continue;
                    }
                };

                out.push(Ok(ImportDay {
                    date,
                    position: pos,
                    meta: r.name,
                }));
            }
            Err(e) => out.push(Err(AppError::InvalidArgs(format!("Invalid CSV row: {e}")))),
        }
    }

    out
}
