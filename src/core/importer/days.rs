use crate::config::Config;
use crate::errors::AppResult;

#[derive(Debug, Clone, Copy)]
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

pub fn import_days(
    _cfg: &Config,
    _content: &str,
    _format: ImportInputFormat,
    _dry_run: bool,
    _replace: bool,
    _source: &str,
) -> AppResult<ImportReport> {
    // TODO v0.8.3: implement parsing + DB writes
    Ok(ImportReport::default())
}
