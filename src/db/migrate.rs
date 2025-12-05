use rusqlite::{Connection, OptionalExtension, Result};

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
        let name = c?;
        if name == "pair" {
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

/// Add `pair` column to `events` by rebuilding the table (if the table exists but lacks `pair`).
fn migrate_add_pair_to_events(conn: &Connection) -> Result<()> {
    // Controlla se esiste la tabella `events`
    if !events_table_exists(conn)? {
        return Ok(());
    }

    // Se ha già la colonna `pair` non c'è niente da fare
    if events_has_pair_column(conn)? {
        return Ok(());
    }

    println!("⚠️  Adding 'pair' column to events table...");

    conn.execute_batch(
        r#"
        PRAGMA foreign_keys=OFF;
        BEGIN;

        ALTER TABLE events RENAME TO events_old;

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

        INSERT INTO events (id, date, time, kind, position, lunch_break, source, meta, created_at)
        SELECT id, date, time, kind, position, lunch_break, source, meta, created_at
        FROM events_old;

        DROP TABLE events_old;

        CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
        CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);

        -- Allinea la sequenza AUTOINCREMENT con il max(id)
        UPDATE sqlite_sequence
          SET seq = (SELECT IFNULL(MAX(id), 0) FROM events)
        WHERE name = 'events';

        COMMIT;
        PRAGMA foreign_keys=ON;
        "#,
    )?;

    println!("✅ 'pair' column added to events table.");
    recalc_all_pairs(conn)?;
    println!("✅ Populated 'pair' column for existing events.");

    Ok(())
}

/// Recalculate `pair` values for all events of a given date.
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
        .collect::<Result<Vec<_>>>()?;

    let mut current_pair = 1;
    let mut last_in: Option<i32> = None;

    for (id, kind, _time) in events {
        match kind.as_str() {
            "in" => {
                last_in = Some(id);
                conn.execute(
                    "UPDATE events SET pair = ?1 WHERE id = ?2",
                    (current_pair, id),
                )?;
            }
            "out" => {
                conn.execute(
                    "UPDATE events SET pair = ?1 WHERE id = ?2",
                    (current_pair, id),
                )?;
                if last_in.is_some() {
                    current_pair += 1;
                    last_in = None;
                }
            }
            _ => {
                // Qualsiasi altro valore non previsto → pair = 0
                conn.execute("UPDATE events SET pair = 0 WHERE id = ?1", (id,))?;
            }
        }
    }

    Ok(())
}

/// Recalculate pairs for all dates present in the `events` table.
fn recalc_all_pairs(conn: &Connection) -> Result<()> {
    let mut stmt = conn.prepare("SELECT DISTINCT date FROM events ORDER BY date ASC")?;
    let dates = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;

    for d in dates {
        recalc_pairs_for_date(conn, &d)?;
    }

    Ok(())
}

/// Public entry point: run all pending (idempotent) migrations.
///
/// Chiamata tipicamente da `db::init_db()`.
pub fn run_pending_migrations(conn: &Connection) -> Result<()> {
    // 1) Garantiamo sempre la presenza della tabella log
    ensure_log_table(conn)?;

    // 2) Garantiamo sempre la presenza della tabella work_sessions
    ensure_work_sessions_table(conn)?;

    // 3) Garantiamo la tabella events con schema moderno
    if !events_table_exists(conn)? {
        create_events_table(conn)?;
        println!("✅ Created events table (modern schema).");
    } else {
        // Se esiste ma non ha `pair`, migriamo
        if !events_has_pair_column(conn)? {
            migrate_add_pair_to_events(conn)?;
        } else {
            // Assicuriamo comunque gli indici
            conn.execute_batch(
                r#"
                CREATE INDEX IF NOT EXISTS idx_events_date_time ON events(date, time);
                CREATE INDEX IF NOT EXISTS idx_events_date_kind ON events(date, kind);
                "#,
            )?;
        }
    }

    Ok(())
}

fn ensure_work_sessions_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS work_sessions (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            date        TEXT                NOT NULL,
            position    TEXT    DEFAULT 'O' NOT NULL,
            start_time  TEXT    DEFAULT ''  NOT NULL,
            lunch_break INTEGER DEFAULT 0   NOT NULL,
            end_time    TEXT    DEFAULT ''  NOT NULL,
            work_duration INTEGER DEFAULT 0  -- minuti netti: (end-start)-lunch
            CHECK (position IN ('O', 'R', 'H', 'C', 'M'))
        );

        CREATE INDEX IF NOT EXISTS idx_work_sessions_date ON work_sessions(date);
        CREATE INDEX IF NOT EXISTS idx_work_sessions_position ON work_sessions(position);
        "#,
    )?;
    Ok(())
}
