use crate::config::Config;
use crate::db;
use chrono::Utc;
use rusqlite::{Connection, Error, OptionalExtension, Result, ffi, params};
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;

pub struct Migration {
    pub version: &'static str,
    pub description: &'static str,
    pub up: fn(&Connection) -> Result<()>, // migration function
}

/// Upgrade the legacy `log` table (that used a `function` column) to the
/// new schema using `operation` and `target` columns.
fn upgrade_legacy_log_schema(conn: &Connection) -> Result<()> {
    // Check whether the log table exists
    let mut exists_stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='log'")?;
    let exists = exists_stmt
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?;
    if exists.is_none() {
        return Ok(()); // nothing to upgrade
    }

    // Inspect columns
    let mut col_stmt = conn.prepare("PRAGMA table_info('log')")?;
    let cols_iter = col_stmt.query_map([], |row| {
        Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?))
    })?;
    let mut has_function = false;
    let mut has_operation = false;
    let mut has_target = false;
    for c in cols_iter {
        let (name, _ty) = c?;
        match name.as_str() {
            "function" => has_function = true,
            "operation" => has_operation = true,
            "target" => has_target = true,
            _ => {}
        }
    }

    // Case 1: old schema (has function and no operation) -> rebuild table
    if has_function && !has_operation {
        // Rename and recreate
        conn.execute_batch(
            r#"
            ALTER TABLE log RENAME TO log_old;
            CREATE TABLE IF NOT EXISTS log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                operation TEXT NOT NULL,
                target TEXT DEFAULT '',
                message TEXT NOT NULL
            );
            INSERT INTO log (id, date, operation, message)
            SELECT id, date, function, message FROM log_old;
            DROP TABLE log_old;
            "#,
        )?;
        println!("🔄 Upgraded legacy log table: added columns operation/target and migrated data.");
        return Ok(());
    }

    // Case 2: table has operation but lacks target -> add target column
    if has_operation && !has_target {
        conn.execute("ALTER TABLE log ADD COLUMN target TEXT DEFAULT ''", [])?;
        println!("🔄 Added missing 'target' column to log table.");
    }

    Ok(())
}

fn query_pairs(stmt: &mut rusqlite::Statement<'_>) -> Result<Vec<(String, String)>, Error> {
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

/// Ensure the table used to track applied migrations exists.
fn ensure_migrations_table(conn: &Connection) -> Result<(), Error> {
    // First attempt to upgrade any legacy log table schema
    upgrade_legacy_log_schema(conn).map_err(|e| {
        Error::SqliteFailure(
            ffi::Error::new(1),
            Some(format!("Failed to upgrade legacy log table: {}", e)),
        )
    })?;
    // With the new strategy we use the `log` table to track migrations
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            operation TEXT NOT NULL,
            target TEXT DEFAULT '',
            message TEXT NOT NULL
        );
        "#,
    )
}

/// Read already-applied migration versions
fn applied_versions(conn: &Connection) -> Result<HashSet<String>, Error> {
    let mut set = HashSet::new();

    // 1) If legacy schema_migrations table exists, read versions from it
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
    )?;
    if stmt
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?
        .is_some()
    {
        let mut stmt2 = conn.prepare("SELECT version FROM schema_migrations")?;
        let rows = stmt2.query_map([], |row| row.get::<_, String>(0))?;
        for r in rows {
            set.insert(r?);
        }
    }

    // 2) If `log` table exists, detect its columns and read migration markers accordingly
    let mut stmt_log =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='log'")?;
    if stmt_log
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?
        .is_some()
    {
        // inspect columns of log
        let mut col_stmt = conn.prepare("PRAGMA table_info('log')")?;
        let mut has_target = false;
        // PRAGMA table_info returns rows with columns: cid, name, type, notnull, dflt_value, pk
        let cols = col_stmt.query_map([], |row| row.get::<_, String>(1))?;
        for c in cols {
            let name = c?;
            if name == "target" {
                has_target = true;
                break;
            }
        }

        if has_target {
            // safe to query target column
            let mut stmt3 = conn.prepare(
                "SELECT target FROM log WHERE operation IN ('migration_applied','migration')",
            )?;
            let rows3 = stmt3.query_map([], |row| row.get::<_, String>(0))?;
            for r in rows3 {
                let v = r?;
                if !v.is_empty() {
                    set.insert(v);
                }
            }
        } else {
            // Fallback for old log schema: try to infer applied migrations from `function` and `message` content
            let mut stmt_fallback = conn.prepare("SELECT function, message FROM log")?;
            let rows = query_pairs(&mut stmt_fallback)?;

            for r in rows {
                let (func, msg) = r;
                // If message contains "Applied migration <version>", extract version
                if let Some(idx) = msg.find("Applied migration ") {
                    let after = msg[idx + "Applied migration ".len()..].trim();
                    if !after.is_empty() {
                        let ver = after
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .trim_matches(|c: char| !c.is_ascii());
                        if !ver.is_empty() {
                            set.insert(ver.to_string());
                            continue;
                        }
                    }
                }
                // If the function column itself looks like a migration version (starts with digits and contains '_'), take it
                if !func.is_empty()
                    && func
                        .chars()
                        .next()
                        .map(|ch| ch.is_ascii_digit())
                        .unwrap_or(false)
                    && func.contains('_')
                {
                    set.insert(func);
                }
            }
        }
    }

    Ok(set)
}

/// Mark a migration as applied (only after success)
fn mark_applied(conn: &Connection, version: &str) -> Result<(), Error> {
    // Instead of writing into a dedicated schema_migrations table, insert a marker into `log`.
    conn.execute(
        "INSERT INTO log (date, operation, target, message) VALUES (?1, ?2, ?3, ?4)",
        params![
            Utc::now().to_rfc3339(),
            "migration_applied",
            version,
            format!("Applied migration {}", version)
        ],
    )?;
    Ok(())
}

/// Execute only migrations that are not yet applied
static ALL_MIGRATIONS: &[Migration] = &[
    Migration {
        version: "20250915_0001_create_log_table",
        description: "Create log table to track operations and migrations",
        up: migrate_to_030_rel,
    },
    Migration {
        version: "20250920_0002_add_C_position_to_work_sessions",
        description: "Add 'C' (On-Site) to position CHECK in work_sessions table",
        up: migrate_to_032_rel,
    },
    Migration {
        version: "20250919_0003_add_lunch_break_to_config",
        description: "Add min_duration_lunch_break and max_duration_lunch_break to config file if missing",
        up: migrate_to_033_rel,
    },
    Migration {
        version: "20250925_0004_add_indexes_to_work_sessions",
        description: "Add indexes to work_sessions on date and position for faster queries",
        up: migrate_to_034_rel,
    },
    Migration {
        version: "20251001_0005_add_separator_char_to_config",
        description: "Add separator_char default to configuration file if missing",
        up: migrate_to_035_rel,
    },
    Migration {
        version: "20251010_0006_create_events_table",
        description: "Create events table to store time punches (in/out) with position and lunch",
        up: migrate_to_036_create_events,
    },
    Migration {
        version: "20251015_0007_migrate_work_sessions_to_events",
        description: "Migrate existing work_sessions rows into events (idempotent, source='migration')",
        up: migrate_to_037_migrate_work_sessions_to_events,
    },
    Migration {
        version: "20251020_0008_add_M",
        description: "Extend position CHECK to include 'M' (Mixed) and migrate existing tables if necessary",
        up: migrate_to_038_add_m,
    },
    Migration {
        version: "20251030_0009_unify_schema_migrations_into_log",
        description: "Import schema_migrations rows into the unified log table and drop schema_migrations",
        up: migrate_to_unify_schema_migrations,
    },
    Migration {
        version: "20251006_0010_rename_rtimelog_to_rtimelogger",
        description: "Rename configuration directory/file and DB from 'rtimelog' to 'rtimelogger'",
        up: crate::config::migrate::run_config_migration,
    },
    Migration {
        version: "20251008_0011_add_show_weekday",
        description: "Add `show_weekday` parameter to configuration file",
        up: crate::config::migrate::migrate_add_show_weekday,
    },
    Migration {
        version: "20251008_0012_add_field_pair_to_events",
        description: "Add `pair` field to events table to group in/out pairs",
        up: migrate_add_pair_to_events,
    },
];

pub fn run_pending_migrations(conn: &Connection) -> Result<(), Error> {
    // Ensure base tables exist (defensive): create work_sessions and log if missing so migrations can reference them.
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS work_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            position TEXT NOT NULL DEFAULT 'O' CHECK (position IN ('O','R','H','C','M')),
            start_time TEXT NOT NULL DEFAULT '',
            lunch_break INTEGER NOT NULL DEFAULT 0,
            end_time TEXT NOT NULL DEFAULT '',
            work_duration INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            operation TEXT NOT NULL,
            target TEXT DEFAULT '',
            message TEXT NOT NULL
        );
        ",
    )?;

    ensure_migrations_table(conn)?;

    let applied = applied_versions(conn)?;
    for m in ALL_MIGRATIONS {
        if !applied.contains(m.version) {
            // Apply the migration
            (m.up)(conn)?;
            // Mark as applied
            mark_applied(conn, m.version)?;
            println!("✅ Migration applied: {} — {}", m.version, m.description);
        }
    }
    println!();
    Ok(())
}

fn migrate_to_030_rel(conn: &Connection) -> Result<()> {
    // create new table log
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            operation TEXT NOT NULL,
            target TEXT DEFAULT '',
            message TEXT NOT NULL
        );
        ",
    )?;

    let mut stmt =
        conn.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let table_sql: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;

    if let Some(sql) = table_sql
        && sql.contains("CHECK (position IN ('O','R'))")
    {
        println!("⚠️  Old schema detected, migrating work_sessions to support 'H' (Holiday)...");

        conn.execute_batch(
            "
                ALTER TABLE work_sessions RENAME TO work_sessions_old;

                CREATE TABLE IF NOT EXISTS work_sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    date TEXT NOT NULL,
                    position TEXT NOT NULL DEFAULT 'O' CHECK (position IN ('O','R','H','C','M')),
                    start_time TEXT NOT NULL DEFAULT '',
                    lunch_break INTEGER NOT NULL DEFAULT 0,
                    end_time TEXT NOT NULL DEFAULT '',
                    work_duration INTEGER NOT NULL DEFAULT 0
                );

                INSERT INTO work_sessions (id, date, position, start_time, lunch_break, end_time)
                SELECT id, date, position, start_time, lunch_break, end_time
                FROM work_sessions_old;

                DROP TABLE work_sessions_old;
                ",
        )?;

        db::ttlog(
            conn,
            "migration_applied",
            "migrate_to_030_rel",
            "Migration table \'work_sessions\' completed.",
        )?;
        println!("✅ Migration completed successfully.");
    }

    Ok(())
}

fn migrate_to_032_rel(conn: &Connection) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let table_sql: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;

    if let Some(sql) = table_sql
        && sql.contains("CHECK(position IN ('O','R','H'))")
    {
        println!("⚠️  Old schema detected, migrating work_sessions to support 'C' (On-Site)...");

        conn.execute_batch(
            "
                ALTER TABLE work_sessions RENAME TO work_sessions_old;

                CREATE TABLE IF NOT EXISTS work_sessions (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    date TEXT NOT NULL,
                    position TEXT NOT NULL DEFAULT 'O' CHECK (position IN ('O','R','H','C','M')),
                    start_time TEXT NOT NULL DEFAULT '',
                    lunch_break INTEGER NOT NULL DEFAULT 0,
                    end_time TEXT NOT NULL DEFAULT '',
                    work_duration INTEGER NOT NULL DEFAULT 0
                );

                INSERT INTO work_sessions (id, date, position, start_time, lunch_break, end_time)
                SELECT id, date, position, start_time, lunch_break, end_time
                FROM work_sessions_old;

                DROP TABLE work_sessions_old;
                ",
        )?;

        db::ttlog(
            conn,
            "migration_applied",
            "migrate_to_032_rel",
            "Migration table \'work_sessions\' completed.",
        )?;
        println!("✅ Migration completed successfully.");
    }

    Ok(())
}

pub fn migrate_to_033_rel(conn: &Connection) -> Result<(), Error> {
    let path = Config::config_file();
    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path).map_err(|e| {
        Error::SqliteFailure(
            ffi::Error::new(1), // code "Unknown error"
            Some(format!("Failed to read config: {}", e)),
        )
    })?;
    let mut value: Value = serde_yaml::from_str(&content).map_err(|e| {
        Error::SqliteFailure(
            ffi::Error::new(1),
            Some(format!("Failed to parse config: {}", e)),
        )
    })?;

    // If the YAML root is not a mapping (unexpected), skip the migration instead of failing.
    if let Some(obj) = value.as_mapping_mut() {
        if !obj.contains_key(Value::String("min_duration_lunch_break".to_string())) {
            obj.insert(
                Value::String("min_duration_lunch_break".to_string()),
                Value::Number(30.into()),
            );
        }
        if !obj.contains_key(Value::String("max_duration_lunch_break".to_string())) {
            obj.insert(
                Value::String("max_duration_lunch_break".to_string()),
                Value::Number(90.into()),
            );
        }
    } else {
        println!(
            "⚠️  Config file exists but is not a mapping; skipping config migration (20250919_0003)"
        );
        return Ok(());
    }

    let new_yaml = serde_yaml::to_string(&value).map_err(|e| {
        Error::SqliteFailure(
            ffi::Error::new(1),
            Some(format!("Failed to serialize config: {}", e)),
        )
    })?;

    fs::write(&path, new_yaml).map_err(|e| {
        Error::SqliteFailure(
            ffi::Error::new(1),
            Some(format!("Failed to write config: {}", e)),
        )
    })?;

    db::ttlog(
        conn,
        "migration_applied",
        "migrate_to_033_rel",
        "Migration configuration file completed.",
    )?;
    println!("✅ Config file migrated: {:?}", path);

    Ok(())
}

fn migrate_to_034_rel(conn: &Connection) -> Result<()> {
    // Create indexes to speed up queries filtering by date and position, but only if the
    // `work_sessions` table exists in the database (avoids 'no such table' errors).
    let mut stmt =
        conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let exists: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;
    if exists.is_some() {
        conn.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS idx_work_sessions_date ON work_sessions(date);
            CREATE INDEX IF NOT EXISTS idx_work_sessions_position ON work_sessions(position);
            ",
        )?;
    } else {
        println!("⚠️  work_sessions table not found; skipping index creation");
    }

    db::ttlog(
        conn,
        "migration_applied",
        "migrate_to_034_rel",
        "Added indexes idx_work_sessions_date and idx_work_sessions_position",
    )?;
    println!("✅ Created indexes for work_sessions (date, position)");
    Ok(())
}

fn migrate_to_035_rel(conn: &Connection) -> Result<()> {
    // Ensure config file exists and add separator_char if missing
    use serde_yaml::Value;
    let path = Config::config_file();
    if !path.exists() {
        // nothing to do
        return Ok(());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;
    let mut value: Value =
        serde_yaml::from_str(&content).map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;

    if let Some(map) = value.as_mapping_mut() {
        let key = Value::String("separator_char".to_string());
        if !map.contains_key(&key) {
            map.insert(key.clone(), Value::String("-".to_string()));
            // write back
            let new_yaml = serde_yaml::to_string(&map)
                .map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;
            fs::write(&path, new_yaml).map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;
            db::ttlog(
                conn,
                "migration_applied",
                "migrate_to_035_rel",
                "Inserted separator_char into config file",
            )?;
            println!("✅ Config file updated with separator_char: {:?}", path);
        }
    }

    Ok(())
}

fn migrate_to_036_create_events(conn: &Connection) -> Result<()> {
    // Create a flexible events table that stores in/out punches, associated position and an optional lunch value
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            date TEXT NOT NULL,
            time TEXT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('in','out')),
            position TEXT NOT NULL DEFAULT 'O' CHECK(position IN ('O','R','H','C','M')),
            lunch_break INTEGER NOT NULL DEFAULT 0,
            source TEXT NOT NULL DEFAULT 'cli',
            meta TEXT DEFAULT '',
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
        CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);
        ",
    )?;

    db::ttlog(
        conn,
        "migration_applied",
        "migrate_to_036_create_events",
        "Created events table",
    )?;
    println!("✅ Created events table");

    Ok(())
}

fn migrate_to_037_migrate_work_sessions_to_events(conn: &Connection) -> Result<()> {
    // Idempotent migration: for each work_sessions row, insert corresponding in/out events
    // only if they don't already exist in events. Mark source='migration'.
    let mut select_ws = conn.prepare(
        "SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions",
    )?;
    let ws_rows = select_ws.query_map([], |row| {
        Ok((
            row.get::<_, i32>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i32>(4)?,
            row.get::<_, String>(5)?,
        ))
    })?;

    let mut inserted = 0i32;
    for r in ws_rows {
        let (_id, date, position, start_time, lunch_break, end_time) = r?;

        // Insert start event if present and not already migrated
        if !start_time.trim().is_empty() {
            let exists: Option<i32> = conn
                .query_row(
                    "SELECT id FROM events WHERE date = ?1 AND time = ?2 AND kind = 'in' AND source = 'migration' LIMIT 1",
                    params![&date, &start_time],
                    |row| row.get(0),
                )
                .optional()?;
            if exists.is_none() {
                conn.execute(
                    "INSERT INTO events (date, time, kind, position, lunch_break, source, meta, created_at) VALUES (?1, ?2, 'in', ?3, 0, 'migration', '', ?4)",
                    params![&date, &start_time, &position, Utc::now().to_rfc3339()],
                )?;
                inserted += 1;
            }
        }

        // Insert end event if present and not already migrated
        if !end_time.trim().is_empty() {
            let exists2: Option<i32> = conn
                .query_row(
                    "SELECT id FROM events WHERE date = ?1 AND time = ?2 AND kind = 'out' AND source = 'migration' LIMIT 1",
                    params![&date, &end_time],
                    |row| row.get(0),
                )
                .optional()?;
            if exists2.is_none() {
                conn.execute(
                    "INSERT INTO events (date, time, kind, position, lunch_break, source, meta, created_at) VALUES (?1, ?2, 'out', ?3, ?4, 'migration', '', ?5)",
                    params![&date, &end_time, &position, lunch_break, Utc::now().to_rfc3339()],
                )?;
                inserted += 1;
            }
        }
    }

    if inserted > 0 {
        db::ttlog(
            conn,
            "migration_applied",
            "migrate_to_037_migrate_work_sessions_to_events",
            &format!("Inserted {} events from work_sessions migration", inserted),
        )?;
    }

    Ok(())
}

fn migrate_to_038_add_m(conn: &Connection) -> Result<()> {
    // This migration extends the CHECK to include 'M' for Mixed; only applies if work_sessions exists
    let mut stmt =
        conn.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='work_sessions'")?;
    let table_sql: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;

    if let Some(sql) = table_sql
        && sql.contains("CHECK(position IN ('O','R','H','C'))")
    {
        println!("⚠️  Old schema detected, migrating work_sessions to support 'M' (Mixed)...");
        conn.execute_batch(
            "
            ALTER TABLE work_sessions RENAME TO work_sessions_old;

            CREATE TABLE IF NOT EXISTS work_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                position TEXT NOT NULL DEFAULT 'O' CHECK (position IN ('O','R','H','C','M')),
                start_time TEXT NOT NULL DEFAULT '',
                lunch_break INTEGER NOT NULL DEFAULT 0,
                end_time TEXT NOT NULL DEFAULT '',
                work_duration INTEGER NOT NULL DEFAULT 0
            );

            INSERT INTO work_sessions (id, date, position, start_time, lunch_break, end_time)
            SELECT id, date, position, start_time, lunch_break, end_time FROM work_sessions_old;

            DROP TABLE work_sessions_old;
            ",
        )?;

        db::ttlog(
            conn,
            "migration_applied",
            "migrate_to_038_add_m",
            "Migration table 'work_sessions' to include 'M' completed.",
        )?;
    }

    Ok(())
}

/// New migration: import schema_migrations into unified log and drop legacy table
fn migrate_to_unify_schema_migrations(conn: &Connection) -> Result<()> {
    // Check if schema_migrations exists
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
    )?;
    if stmt
        .query_row([], |row| row.get::<_, String>(0))
        .optional()?
        .is_some()
    {
        // Read all rows
        let mut sel = conn.prepare("SELECT version, applied_at FROM schema_migrations")?;
        let rows = query_pairs(&mut sel)?;

        for r in rows {
            let (version, applied_at) = r;
            // Insert into log
            conn.execute(
                "INSERT INTO log (date, operation, target, message) VALUES (?1, ?2, ?3, ?4)",
                params![
                    applied_at,
                    "migration_applied",
                    version,
                    format!("Imported migration {} from schema_migrations", version)
                ],
            )?;
        }

        // Drop legacy table
        conn.execute_batch("DROP TABLE IF EXISTS schema_migrations;")?;

        db::ttlog(
            conn,
            "migration_applied",
            "migrate_to_unify_schema_migrations",
            "Imported schema_migrations into unified log and dropped legacy table",
        )?;
        println!("✅ schema_migrations imported into log and dropped.");
    }

    Ok(())
}

fn migrate_add_pair_to_events(conn: &Connection) -> Result<()> {
    // Add `pair` column to events table if missing
    let mut stmt =
        conn.prepare("SELECT sql FROM sqlite_master WHERE type='table' AND name='events'")?;
    let table_sql: Option<String> = stmt.query_row([], |row| row.get(0)).optional()?;

    if let Some(sql) = table_sql
        && !sql.contains("pair INTEGER NOT NULL DEFAULT 0")
    {
        println!("⚠️  Adding 'pair' column to events table...");

        conn.execute_batch(
            r#"
            PRAGMA foreign_keys=OFF;
            BEGIN;

            ALTER TABLE events RENAME TO events_old;

            CREATE TABLE events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                date TEXT NOT NULL,
                time TEXT NOT NULL,
                kind TEXT NOT NULL CHECK(kind IN ('in','out')),
                position TEXT NOT NULL DEFAULT 'O' CHECK(position IN ('O','R','H','C','M')),
                lunch_break INTEGER NOT NULL DEFAULT 0,
                pair INTEGER NOT NULL DEFAULT 0,
                source TEXT NOT NULL DEFAULT 'cli',
                meta TEXT DEFAULT '',
                created_at TEXT NOT NULL
            );

            INSERT INTO events (id, date, time, kind, position, lunch_break, source, meta, created_at)
            SELECT id, date, time, kind, position, lunch_break, source, meta, created_at
            FROM events_old;

            DROP TABLE events_old;

            CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
            CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);

            -- Align AUTOINCREMENT sequence with max(id)
            UPDATE sqlite_sequence
            SET seq = (SELECT IFNULL(MAX(id), 0) FROM events)
            WHERE name = 'events';

            COMMIT;
            PRAGMA foreign_keys=ON;
            "#
        )?;

        println!("✅ 'pair' column added to events table.");

        // Populate the 'pair' field for all existing records
        recalc_all_pairs(conn)?;
        println!("✅ Populated 'pair' column for existing events");

        db::ttlog(
            conn,
            "Populated 'pair' column",
            "migrate_add_pair_to_events",
            "Populated 'pair' column for existing events",
        )?;
    }

    Ok(())
}

/// Recalculate pair numbers for a given date
fn recalc_pairs_for_date(conn: &Connection, date: &str) -> Result<()> {
    let mut stmt =
        conn.prepare("SELECT id, kind, time FROM events WHERE date = ?1 ORDER BY time ASC")?;

    let events = stmt
        .query_map([date], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut current_pair = 1;
    let mut last_in: Option<i32> = None;

    for (id, kind, _time) in events {
        if kind == "in" {
            last_in = Some(id);
            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                (current_pair, id),
            )?;
        } else if kind == "out" {
            conn.execute(
                "UPDATE events SET pair = ?1 WHERE id = ?2",
                (current_pair, id),
            )?;
            if last_in.is_some() {
                current_pair += 1;
                last_in = None;
            }
        } else {
            conn.execute("UPDATE events SET pair = 0 WHERE id = ?1", (id,))?;
        }
    }

    Ok(())
}

/// Recalculate pairs for all dates in the DB
fn recalc_all_pairs(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT DISTINCT date FROM events ORDER BY date ASC")?;
    let dates = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    for d in dates {
        recalc_pairs_for_date(conn, &d)?;
    }

    Ok(())
}
