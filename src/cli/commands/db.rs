use crate::cli::parser::Commands;
use crate::config::Config;
use crate::db::migrate::run_pending_migrations;
use crate::db::pool::DbPool;
use crate::db::stats;
use crate::errors::AppResult;
use crate::utils::colors::{CYAN, GREEN, RED, RESET};

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Db {
        migrate,
        check,
        vacuum,
        info,
    } = cmd
    {
        // Unica istanza condivisa
        let mut pool: Option<DbPool> = None;

        // Helper per ottenere il DbPool (NON closure!)
        fn get_pool<'a>(pool: &'a mut Option<DbPool>, db_path: &str) -> AppResult<&'a mut DbPool> {
            if pool.is_none() {
                *pool = Some(DbPool::new(db_path)?);
            }
            Ok(pool.as_mut().unwrap())
        }

        //
        // 1) MIGRATE
        //
        if *migrate {
            let pool = get_pool(&mut pool, &cfg.database)?;
            println!("{}▶ Running migrations…{}", CYAN, RESET);
            run_pending_migrations(&pool.conn)?;
            println!("{}✔ Migration completed.{}\n", GREEN, RESET);
        }

        //
        // 2) INFO
        //
        if *info {
            let pool = get_pool(&mut pool, &cfg.database)?;
            stats::print_db_info(pool, &cfg.database)?;
        }

        //
        // 3) CHECK
        //
        if *check {
            let pool = get_pool(&mut pool, &cfg.database)?;

            println!("{}▶ Running integrity check…{}", CYAN, RESET);

            let integrity: String = pool
                .conn
                .query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;

            if integrity == "ok" {
                println!("{}✔ Integrity check passed.{}\n", GREEN, RESET);
            } else {
                println!("{}✘ Integrity check failed:{} {}\n", RED, RESET, integrity);
            }
        }

        //
        // 4) VACUUM
        //
        if *vacuum {
            let pool = get_pool(&mut pool, &cfg.database)?;
            println!("{}▶ Running VACUUM…{}", CYAN, RESET);

            pool.conn.execute_batch("VACUUM;")?;

            println!("{}✔ Vacuum completed.{}\n", GREEN, RESET);
        }
    }

    Ok(())
}
