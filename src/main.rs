use clap::Parser;
use rtimelogger::config::Config;
use rtimelogger::{db, export};
use rusqlite::Connection;

mod commands;
use rtimelogger::cli::{Cli, Commands};

fn main() -> rusqlite::Result<()> {
    let cli = Cli::parse();

    // Ensure filesystem migration ran early (before any DB open). This moves old "%APPDATA%/rtimelog" or
    // "$HOME/.rtimelog" to the new location and renames config/db references if needed.
    if let Err(e) = rtimelogger::config::migrate::run_fs_migration() {
        eprintln!("⚠️  Filesystem migration warning: {}", e);
    }

    // Ensure config dir exists so Connection::open can create the DB file inside it.
    if let Err(e) = std::fs::create_dir_all(Config::config_dir()) {
        eprintln!("❌ Failed to create config directory: {}", e);
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(format!("Failed to create config dir: {}", e)),
        ));
    }

    // Determine DB path without loading the full config (Config::load may read files under
    // $HOME or %APPDATA% which tests may control); prefer to avoid reading it when --test is set.
    // Cache a loaded Config when needed to avoid loading it twice later.
    let mut maybe_loaded_config: Option<Config> = None;
    let db_path = if let Some(custom) = &cli.db {
        let custom_path = std::path::Path::new(custom);
        if custom_path.is_absolute() {
            custom.to_string()
        } else {
            Config::config_dir()
                .join(custom_path)
                .to_string_lossy()
                .to_string()
        }
    } else if cli.test {
        // In test mode: use the default file name under the test config dir, but DO NOT call Config::load()
        Config::config_dir()
            .join("rtimelogger.sqlite")
            .to_string_lossy()
            .to_string()
    } else {
        // Production: load the configuration and use the database path from it
        let cfg = Config::load();
        let path = cfg.database.clone();
        maybe_loaded_config = Some(cfg);
        path
    };

    // Now prepare a `config` object for use by commands; when running under --test or when --db is
    // provided we construct a default config (matching Config::load() defaults) and point its
    // `database` to the resolved db_path. Only when neither `--db` nor `--test` are used we call
    // `Config::load()` to read possible overrides from disk. If we already loaded it above, reuse it.
    let config = if cli.test || cli.db.is_some() {
        Config {
            database: db_path.clone(),
            default_position: "O".to_string(),
            min_work_duration: "8h".to_string(),
            min_duration_lunch_break: 30,
            max_duration_lunch_break: 90,
            separator_char: "-".to_string(),
            show_weekday: "None".to_string(),
        }
    } else {
        // For production, prefer to reuse an already-loaded Config when available
        maybe_loaded_config.unwrap_or_else(Config::load)
    };

    println!();

    // Handle `init` separately because it may need to create config/db files first
    if let Commands::Init = &cli.command {
        return commands::handle_init(&cli, &db_path);
    }

    // For other commands, open a single shared connection, set useful PRAGMA and ensure DB is initialized (creates
    // base tables and runs pending migrations).
    // Try to open the DB; if opening fails (e.g. CannotOpen), attempt remediation once: run FS migration,
    // create parent directories and try to touch the DB file, then retry.
    let mut conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!(
                "⚠️  Failed to open DB at {:?}: {} -- attempting remediation",
                db_path, e
            );

            // Diagnostic: print parent/exists/info to help debugging
            let p = std::path::Path::new(&db_path);
            if let Some(parent) = p.parent() {
                eprintln!("   -> DB parent exists: {}", parent.exists());
                if parent.exists() {
                    match std::fs::metadata(parent) {
                        Ok(md) => eprintln!(
                            "      parent metadata: is_dir={} readonly={}",
                            md.is_dir(),
                            md.permissions().readonly()
                        ),
                        Err(me) => eprintln!("      parent metadata error: {}", me),
                    }
                }
            }
            eprintln!(
                "   -> DB file exists: {}",
                std::path::Path::new(&db_path).exists()
            );

            // Re-run filesystem migration (best-effort)
            if let Err(e2) = rtimelogger::config::migrate::run_fs_migration() {
                eprintln!("⚠️  Filesystem migration (retry) warning: {}", e2);
            }
            // Ensure parent dir exists
            if let Some(parent) = std::path::Path::new(&db_path).parent()
                && let Err(e3) = std::fs::create_dir_all(parent)
            {
                eprintln!(
                    "❌ Failed to create parent directory for DB {:?}: {}",
                    parent, e3
                );
            }
            // Try to create (touch) the DB file so sqlite can open it
            if let Err(e4) = std::fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&db_path)
            {
                eprintln!("⚠️  Could not create DB file {:?}: {}", db_path, e4);
            }

            // Retry opening once
            match Connection::open(&db_path) {
                Ok(c2) => c2,
                Err(e_final) => {
                    eprintln!("❌ Final attempt to open DB failed: {}", e_final);
                    // Extra diagnostic: list config dir contents
                    if let Some(parent) = std::path::Path::new(&db_path).parent() {
                        match std::fs::read_dir(parent) {
                            Ok(rd) => {
                                let names: Vec<String> = rd
                                    .filter_map(|r| {
                                        r.ok().and_then(|e| e.file_name().into_string().ok())
                                    })
                                    .collect();
                                eprintln!("   -> Contents of {:?}: {:?}", parent, names);
                            }
                            Err(re) => {
                                eprintln!("   -> Could not read parent dir {:?}: {}", parent, re)
                            }
                        }
                    }
                    return Err(e_final);
                }
            }
        }
    };

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    db::init_db(&conn)?;

    match &cli.command {
        Commands::Add { .. } => commands::handle_add(&cli.command, &mut conn, &config)?,
        Commands::Del { .. } => commands::handle_del(&cli.command, &mut conn)?,
        Commands::List {
            period,
            pos,
            now,
            details,
            events,
            pairs,
            summary,
        } => {
            let args = commands::HandleListArgs {
                period: period.clone(),
                pos: pos.clone(),
                now: *now,
                details: *details,
                events: *events,
                pairs: *pairs,
                summary: *summary,
            };
            commands::handle_list(&args, &conn, &config)?
        }
        Commands::Config { .. } => commands::handle_config(&cli.command)?,
        Commands::Log { .. } => commands::handle_log(&cli.command, &conn)?,
        Commands::Db { .. } => commands::handle_db(&cli.command, &conn)?,
        Commands::Init => {
            // Already handled, but included for exhaustiveness
        }
        Commands::Backup { file, compress } => {
            if let Err(e) = commands::handle_backup(&config, file, compress) {
                eprintln!("❌ Backup failed: {}", e);
            }
        }
        Commands::Export { .. } => {
            if let Err(e) = export::handle_export(&cli.command, &conn) {
                eprintln!("❌ Export failed: {}", e);
            };
        }
    }

    Ok(())
}
