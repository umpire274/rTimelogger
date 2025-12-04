use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::export::ExportLogic;
use crate::db::pool::DbPool;
use crate::errors::AppResult;

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Export {
        format,
        file,
        range,
        events,
        force,
    } = cmd
    {
        let mut pool = DbPool::new(&cfg.database)?;
        ExportLogic::export(&mut pool, format, file, range, *events, *force)?;
    }
    Ok(())
}
