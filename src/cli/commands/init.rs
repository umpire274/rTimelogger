use crate::config::Config;
use crate::db::log;
use crate::errors::AppResult;

use crate::cli::parser::Cli;
use crate::db::initialize::init_db;
use crate::ui::messages::{info, success, warning};

use rusqlite::Connection;

/// Handle the `init` command
///
/// Responsibilities:
///  - Create config directory (if missing)
///  - Create config file (if missing)
///  - Initialize SQLite database
///  - Run migrations
pub fn handle(cli: &Cli) -> AppResult<()> {
    //
    // 1️⃣ INITIALIZE CONFIGURATION
    //
    if let Some(custom) = &cli.db {
        Config::init_all(Some(custom.clone()), cli.test)?;
    } else {
        Config::init_all(None, cli.test)?;
    }

    let config_path = Config::config_file();
    let cfg = Config::load();
    let db_path = cfg.database.clone();

    info("Initializing rTimelogger…");
    info(format!("Config file : {}", config_path.display()));
    info(format!("Database     : {}", &db_path));

    //
    // 2️⃣ OPEN DATABASE
    //
    let conn = Connection::open(&db_path)?;

    //
    // 3️⃣ INITIALIZE DB STRUCTURE + RUN MIGRATIONS
    //
    init_db(&conn)?;
    success(format!("Database initialized at {}", &db_path));

    //
    // 4️⃣ INTERNAL LOG (best-effort)
    //
    if let Err(e) = log::ttlog(
        &conn,
        "init",
        "Database initialized",
        &format!("Database initialized at {}", &db_path),
    ) {
        warning(format!("Failed to write internal log: {}", e));
    }

    success("rTimelogger initialization completed!");
    Ok(())
}
