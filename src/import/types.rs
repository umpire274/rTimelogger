use crate::models::location::Location;
use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportInputFormat {
    Json,
    Csv,
}

#[derive(Default, Debug)]
pub struct ImportReport {
    pub total: usize,
    pub imported: usize,
    pub skipped_existing: usize,
    pub conflicts: usize,
    pub invalid: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportDay {
    pub date: NaiveDate,
    pub position: Location,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ImportDayJson {
    pub date: String,
    #[serde(default)]
    pub position: Option<String>, // opzionale
    #[serde(default)]
    pub name: Option<String>, // opzionale, informativo
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum ImportJson {
    Days { days: Vec<ImportDayJson> },
    Holidays { holidays: Vec<ImportDayJson> },
    Array(Vec<ImportDayJson>),
}
