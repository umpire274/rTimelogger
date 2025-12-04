use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::log::LogLogic;
use crate::db::pool::DbPool;
use crate::errors::AppResult;

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if matches!(cmd, Commands::Log { print: true }) {
        let mut pool = DbPool::new(&cfg.database)?;
        LogLogic::print_log(&mut pool, cfg)?;
    }

    Ok(())
}
