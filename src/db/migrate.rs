use crate::db::db_utils;
use crate::ui::messages::{error, success, warning};
use rusqlite::{Connection, Error, OptionalExtension, Result};

/// Ensure that the `log` table exists with the modern schema.
fn ensure_log_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS log (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            date      TEXT NOT NULL,
            operation TEXT NOT NULL,
            target    TEXT DEFAULT '',
            message   TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// Check if the `work_sessions` table exists.
fn work_sessions_table_exists(conn: &Connection) -> Result<bool> {
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let exists: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;
    Ok(exists.is_some())
}

/// Check if the `events` table exists.
fn events_table_exists(conn: &Connection) -> Result<bool> {
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='events'")?;
    let exists: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;
    Ok(exists.is_some())
}

/// Check if the `events` table has a `pair` column.
fn events_has_pair_column(conn: &Connection) -> Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info('events')")?;
    let cols = stmt.query_map([], |row| row.get::<_, String>(1))?;

    for c in cols {
        if c? == "pair" {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Create the `events` table with the modern schema (including `pair`).
fn create_events_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS events (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            date         TEXT NOT NULL,
            time         TEXT NOT NULL,
            kind         TEXT NOT NULL CHECK(kind IN ('in','out')),
            position     TEXT NOT NULL DEFAULT 'O' CHECK(position IN ('O','R','H','C','M')),
            lunch_break  INTEGER NOT NULL DEFAULT 0,
            pair         INTEGER NOT NULL DEFAULT 0,
            source       TEXT NOT NULL DEFAULT 'cli',
            meta         TEXT DEFAULT '',
            created_at   TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
        CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);
        "#,
    )?;
    Ok(())
}

/// Migrate an old `events` table to include `pair` column.
fn migrate_add_pair_to_events(conn: &Connection) -> Result<()> {
    if !events_table_exists(conn)? {
        return Ok(()); // nessuna tabella → niente da migrare
    }

    if events_has_pair_column(conn)? {
        return Ok(()); // già presente → OK
    }

    warning("Adding 'pair' column to events table...");

    conn.execute_batch(
        r#"
        PRAGMA foreign_keys=OFF;
        BEGIN;

        ALTER TABLE events RENAME TO events_old;

        CREATE TABLE events (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            date         TEXT NOT NULL,
            time         TEXT NOT NULL,
            kind         TEXT NOT NULL CHECK(kind IN ('in','out')),
            position     TEXT NOT NULL DEFAULT 'O' CHECK(position IN ('O','R','H','C','M')),
            lunch_break  INTEGER NOT NULL DEFAULT 0,
            pair         INTEGER NOT NULL DEFAULT 0,
            source       TEXT NOT NULL DEFAULT 'cli',
            meta         TEXT DEFAULT '',
            created_at   TEXT NOT NULL
        );

        INSERT INTO events (id, date, time, kind, position, lunch_break, source, meta, created_at)
        SELECT id, date, time, kind, position, lunch_break, source, meta, created_at
        FROM events_old;

        DROP TABLE events_old;

        CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
        CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);

        UPDATE sqlite_sequence
            SET seq = (SELECT IFNULL(MAX(id), 0) FROM events)
        WHERE name = 'events';

        COMMIT;
        PRAGMA foreign_keys=ON;
        "#,
    )?;

    success("'pair' column added.");

    warning("Rebuilding pairs using the new timeline logic...");

    // Ricaviamo il percorso del DB collegato
    let db_path: String = conn
        .query_row("PRAGMA database_list;", [], |row| row.get(2))
        .unwrap_or_else(|_| "".to_string());

    // Se il DB è in memoria o non ha un percorso valido, gestiamo il caso
    if db_path.is_empty() {
        warning("Could not determine DB path — skipping pair rebuild.");
        return Ok(());
    }

    // Creiamo DbPool
    let mut pool = match crate::db::pool::DbPool::new(&db_path) {
        Ok(p) => p,
        Err(e) => {
            error(format!("Failed to create DbPool for pair rebuild: {}", e));
            return Ok(());
        }
    };

    // Ricostruiamo i pair usando l’API moderna
    match db_utils::rebuild_all_pairs(&mut pool) {
        Ok(_) => success("Populated 'pair' column for existing events."),
        Err(e) => error(format!("Failed to rebuild pairs: {}", e)),
    }

    Ok(())
}

/// Drop obsolete tables as part of the 0.8.0 migration.
fn align_db_schemas_to_080_version(conn: &Connection) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let exists: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;

    if exists.is_some() {
        conn.execute_batch("DROP TABLE work_sessions;")?;
        success("Dropped obsolete work_sessions table.");
    }

    Ok(())
}

fn backup_before_migration(db_path: &str) -> Result<()> {
    use chrono::Local;
    use std::fs::{self, File};
    use std::io::Write;
    use zip::CompressionMethod;
    use zip::ZipWriter;
    use zip::write::FileOptions;

    // Nome file backup
    let backup_name = format!(
        "{}-backup_db_pre_080-beta1.zip",
        Local::now().format("%Y%m%d_%H%M%S")
    );

    let backup_path = std::path::Path::new(db_path)
        .parent()
        .unwrap()
        .join(&backup_name);

    // Apertura ZIP
    let file = File::create(&backup_path).map_err(|e| {
        Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            e.kind(),
            format!("Backup failed (create): {}", e),
        )))
    })?;

    let mut zip = ZipWriter::new(file);

    // Opzioni ZIP
    let options: FileOptions<'_, ()> =
        FileOptions::default().compression_method(CompressionMethod::Deflated);

    zip.start_file("database.sqlite", options).map_err(|e| {
        Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
            "Backup failed (start_file): {}",
            e
        ))))
    })?;

    // Leggi contenuto DB
    let db_content = fs::read(db_path).map_err(|e| {
        Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
            "Backup failed (read): {}",
            e
        ))))
    })?;

    zip.write_all(&db_content).map_err(|e| {
        Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
            "Backup failed (write_all): {}",
            e
        ))))
    })?;

    zip.finish().map_err(|e| {
        Error::ToSqlConversionFailure(Box::new(std::io::Error::other(format!(
            "Backup failed (finish): {}",
            e
        ))))
    })?;

    success(format!("📦 Backup created: {}", backup_path.display()));
    Ok(())
}

fn migrate_add_work_gap_column(conn: &Connection) -> Result<(), Error> {
    let version = "20250215_0012_add_work_gap_flag";

    // 1) Verifica se già applicata
    let mut chk = conn.prepare(
        "SELECT 1 FROM log 
         WHERE operation = 'migration_applied' AND target = ?1 
         LIMIT 1",
    )?;
    if chk.query_row([version], |_| Ok(())).optional()?.is_some() {
        return Ok(()); // già applicata
    }

    // 2) Esegui la migrazione
    conn.execute(
        "ALTER TABLE events ADD COLUMN work_gap INTEGER NOT NULL DEFAULT 0;",
        [],
    )
    .map_err(|e| {
        Error::SqliteFailure(
            rusqlite::ffi::Error::new(1),
            Some(format!("Failed to add 'work_gap' column: {}", e)),
        )
    })?;

    // 3) Marca come applicata
    conn.execute(
        "INSERT INTO log (date, operation, target, message)
         VALUES (datetime('now'), 'migration_applied', ?1, 'Added work_gap flag to events')",
        [version],
    )?;

    success(format!(
        "Migration applied: {} → added 'work_gap' to events table",
        version
    ));

    Ok(())
}

/// Public entry point: run all pending migrations.
///
/// Invocata da db::init_db().
pub fn run_pending_migrations(conn: &Connection) -> Result<()> {
    // 1) Ensure log table
    ensure_log_table(conn)?;

    // 2) Ensure events table exists (even without pair)
    let events_exists = events_table_exists(conn)?;
    let events_has_pair = if events_exists {
        events_has_pair_column(conn)?
    } else {
        false
    };

    // 3) Detect legacy schema (< 0.8.0-beta1)
    let work_sessions_exists = work_sessions_table_exists(conn)?;

    let is_legacy_schema = work_sessions_exists || !events_has_pair;

    // 4) If legacy → perform PRE-MIGRATION BACKUP
    if is_legacy_schema {
        warning("Legacy schema detected — creating safety backup before migration...");

        let db_path: String = conn
            .query_row("PRAGMA database_list;", [], |row| row.get::<_, String>(2))
            .unwrap_or_default();

        if !db_path.is_empty() {
            backup_before_migration(&db_path)?;
        } else {
            warning("Could not determine DB path — backup skipped.");
        }
    }

    // 5) Create events table if missing
    if !events_exists {
        create_events_table(conn)?;
        success("Created events table (modern schema).");
    } else if !events_has_pair {
        migrate_add_pair_to_events(conn)?;
    } else {
        conn.execute_batch(
            r#"
            CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
            CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);
            "#,
        )?;

        migrate_add_work_gap_column(conn)?;
    }

    // 6) Perform schema cleanup for 0.8.0+
    align_db_schemas_to_080_version(conn)?;

    Ok(())
}
