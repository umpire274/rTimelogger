use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::backup::BackupLogic;
use crate::db::pool::DbPool;
use crate::errors::AppResult;

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Backup { file, compress } = cmd {
        let mut pool = DbPool::new(&cfg.database)?;
        BackupLogic::backup(&mut pool, cfg, file, *compress)?;
    }

    Ok(())
}
