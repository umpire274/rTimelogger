use crate::cli::parser::Commands;
use crate::config::Config;
use crate::db::migrate::run_pending_migrations;
use crate::db::pool::DbPool;
use crate::db::stats;
use crate::errors::AppResult;
use crate::ui::messages::{error, info, success};

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Db {
        migrate,
        check,
        vacuum,
        info: show_info,
    } = cmd
    {
        // Unica istanza condivisa
        let mut pool: Option<DbPool> = None;

        // Helper per ottenere il DbPool
        fn get_pool<'a>(pool: &'a mut Option<DbPool>, db_path: &str) -> AppResult<&'a mut DbPool> {
            if pool.is_none() {
                *pool = Some(DbPool::new(db_path)?);
            }
            Ok(pool.as_mut().unwrap())
        }

        // ------------------------------------------------------------
        // 1) MIGRATION
        // ------------------------------------------------------------
        if *migrate {
            let pool = get_pool(&mut pool, &cfg.database)?;

            info("Running database migrations…");

            run_pending_migrations(&pool.conn)?;

            success("Database migration completed successfully.\n");
        }

        // ------------------------------------------------------------
        // 2) SHOW INFO
        // ------------------------------------------------------------
        if *show_info {
            let pool = get_pool(&mut pool, &cfg.database)?;
            info("Database information:");
            stats::print_db_info(pool, &cfg.database)?;
        }

        // ------------------------------------------------------------
        // 3) INTEGRITY CHECK
        // ------------------------------------------------------------
        if *check {
            let pool = get_pool(&mut pool, &cfg.database)?;

            info("Running database integrity check…");

            let integrity: String = pool
                .conn
                .query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;

            if integrity == "ok" {
                success("Integrity check passed.\n");
            } else {
                error(format!("Integrity check failed:\n{}", integrity));
            }
        }

        // ------------------------------------------------------------
        // 4) VACUUM
        // ------------------------------------------------------------
        if *vacuum {
            let pool = get_pool(&mut pool, &cfg.database)?;

            info("Running VACUUM…");
            pool.conn.execute_batch("VACUUM;")?;
            success("VACUUM completed successfully.\n");
        }
    }

    Ok(())
}
