use crate::config::Config;
use crate::db::log;
use crate::errors::AppResult;

use crate::cli::parser::Cli;
use crate::db::initialize::init_db;
use rusqlite::Connection;

/// Handle the `init` command
///
/// This initializes:
///  - the config directory (if missing)
///  - the configuration file
///  - the SQLite database (prod or test mode)
///  - all pending DB migrations
pub fn handle(cli: &Cli) -> AppResult<()> {
    //
    // 1️⃣ PREPARA CONFIGURAZIONE
    //
    // Config::init_all crea:
    //   ~/.rtimelogger/
    //   ~/.rtimelogger/config.yml
    // e ritorna il path del DB configurato.
    //
    // Nel nuovo design, test-mode non è gestito qui ma nel dispatcher.
    //

    if let Some(custom) = &cli.db {
        Config::init_all(Some(custom.clone()), cli.test)?;
    } else {
        Config::init_all(None, cli.test)?;
    }

    let path = Config::config_file();
    let cfg = Config::load();
    let db_path = cfg.database.clone();

    println!("⚙️  Initializing rTimelogger…");
    println!("📄 Config file : {}", path.display());
    println!("🗄️  Database   : {}", &db_path);

    //
    // 2️⃣ APERTURA DB
    //
    let conn = Connection::open(&db_path)?;

    //
    // 3️⃣ INIZIALIZZAZIONE DB (tabelle + migrazioni)
    //
    init_db(&conn)?;

    println!("✅ Database initialized at {}", &db_path);

    //
    // 4️⃣ LOG INTERNO (non bloccante)
    //
    if let Err(e) = log::ttlog(
        &conn,
        "init",
        "Database initialized",
        &format!("Database initialized at {}", &db_path),
    ) {
        eprintln!("⚠️ Failed to write internal log: {}", e);
    }

    println!("🎉 rTimelogger initialization completed!");
    Ok(())
}
