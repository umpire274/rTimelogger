use crate::Cli;
use crate::Commands;
use chrono::{Datelike, NaiveTime};
use rtimelogger::config::Config;
use rtimelogger::events::create_missing_event;
use rtimelogger::utils::{
    compress_backup, describe_position, mins2hhmm, print_separator, weekday_str,
};
use rtimelogger::{db, logic, utils};
use rusqlite::Connection;
use std::io::{Write, stdin};
use std::path::Path;
use std::process::Command;
use std::{fs, io};

fn print_help_add_command() {
    eprintln!("Usage:");
    eprintln!("  rtimelogger add <DATE> [<POS>] [<START>] [<LUNCH>] [<END>]\n");
    eprintln!("Positional arguments:");
    eprintln!("  DATE        Work date in format YYYY-MM-DD");
    eprintln!("  POS         O=Office, R=Remote, H=Holiday, C=On-Site Client");
    eprintln!("  START       Start time in HH:MM");
    eprintln!("  LUNCH       Lunch break minutes (0–90)");
    eprintln!("  END         End time in HH:MM\n");

    eprintln!("Examples:");
    eprintln!("  rtimelogger add 2025-10-11 O 08:55 30 17:10");
    eprintln!("  rtimelogger add 2025-10-11 --pos O --in 08:55 --lunch 30 --out 17:10");
    eprintln!("  rtimelogger add 2025-10-11 --edit --pair 1 --in 09:00\n");
}

pub fn handle_config(cmd: &Commands) -> rusqlite::Result<()> {
    if let Commands::Config {
        print_config,
        edit_config,
        editor,
    } = cmd
    {
        if *print_config {
            let config = Config::load();
            println!("📄 Current configuration:");
            println!("{}", serde_yaml::to_string(&config).unwrap());
        }

        if *edit_config {
            let path = Config::config_file();

            // User-requested editor (if provided)
            let requested_editor = editor.clone();

            // Default editor based on the platform
            let default_editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    if cfg!(target_os = "windows") {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });

            // Use the requested editor if available, otherwise fall back
            let editor_to_use = requested_editor.unwrap_or_else(|| default_editor.clone());

            let status = Command::new(&editor_to_use).arg(&path).status();

            match status {
                Ok(s) if s.success() => {
                    println!(
                        "✅ Configuration file edited successfully with '{}'",
                        editor_to_use
                    );
                }
                Ok(_) | Err(_) => {
                    eprintln!(
                        "⚠️  Editor '{}' not available, falling back to '{}'",
                        editor_to_use, default_editor
                    );
                    // Retry with the default editor
                    let fallback_status = Command::new(&default_editor).arg(&path).status();
                    match fallback_status {
                        Ok(s) if s.success() => {
                            println!(
                                "✅ Configuration file edited successfully with fallback '{}'",
                                default_editor
                            );
                        }
                        Ok(_) | Err(_) => {
                            eprintln!(
                                "❌ Failed to edit configuration file with fallback '{}'",
                                default_editor
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

/// Handle the `init` command
pub fn handle_init(cli: &Cli, db_path: &str) -> rusqlite::Result<()> {
    if let Some(custom) = &cli.db {
        Config::init_all(Some(custom.clone()), cli.test).unwrap();
    } else {
        Config::init_all(None, cli.test).unwrap();
    }

    if cli.test {
        // In test mode, use db_path directly
        let conn = Connection::open(db_path)?;
        // Initialize DB (creates tables) and run pending migrations
        db::init_db(&conn)?;
        println!("✅ Test database initialized at {}", db_path);
        // Log the init operation (non-fatal)
        if let Err(e) = db::ttlog(
            &conn,
            "init",
            "New DB test",
            &format!("Test DB initialized at {}", db_path),
        ) {
            eprintln!("⚠️ Failed to write internal log: {}", e);
        }
    } else {
        // Production mode: use the resolved db_path (do not reparse config from disk here)
        let conn = Connection::open(db_path)?;
        // Initialize DB (creates tables) and run pending migrations
        db::init_db(&conn)?;
        println!("✅ Database initialized at {}", db_path);
        if let Err(e) = db::ttlog(
            &conn,
            "init",
            "New prod DB",
            &format!("Database initialized at {}", db_path),
        ) {
            eprintln!("⚠️ Failed to write internal log: {}", e);
        }
    }

    Ok(())
}

pub fn handle_db(cmd: &Commands, conn: &Connection) -> rusqlite::Result<()> {
    if let Commands::Db { rebuild } = cmd {
        if *rebuild {
            match db::rebuild_work_sessions(&conn) {
                Ok(rows) => {
                    println!(
                        "✅ Rebuilt work_sessions from events ({:?} rows affected)",
                        rows
                    );
                    let _ = db::ttlog(
                        &conn,
                        "db",
                        "Rebuild work_sessions from events",
                        &format!("Rebuilt work_sessions from events ({:?} rows)", rows),
                    );
                }
                Err(e) => eprintln!("❌ Error rebuilding work_sessions: {}", e),
            }
        }
    }
    Ok(())
}

pub fn handle_del(cmd: &Commands, conn: &mut Connection) -> rusqlite::Result<()> {
    if let Commands::Del { pair, date } = cmd {
        let date = date.trim();

        // validate date
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            eprintln!(
                "\u{274c} Invalid date format: {} (expected YYYY-MM-DD)",
                date
            );
            return Ok(());
        }

        if let Some(pair_id) = pair {
            // Delete only a given pair for the specified date
            let events = db::list_events_by_date(conn, date)?;
            if events.is_empty() {
                println!("⚠️  No events found for date {}", date);
                return Ok(());
            }
            let enriched = compute_event_pairs(&events);
            let ids_to_delete: Vec<i32> = enriched
                .iter()
                .filter(|e| e.pair == *pair_id)
                .map(|e| e.event.id)
                .collect();

            if ids_to_delete.is_empty() {
                println!("⚠️  Pair {} not found for date {}", pair_id, date);
                return Ok(());
            }

            // Confirmation prompt
            print!(
                "Are you sure to delete the pair {} of the date {} (N/y) ? ",
                pair_id, date
            );
            let _ = io::stdout().flush();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap_or(0);
            let choice = input.trim().to_lowercase();
            if choice != "y" {
                println!("Aborted. No rows deleted.");
                return Ok(());
            }

            match db::delete_events_by_ids_and_recompute_sessions(conn, &ids_to_delete, date) {
                Ok(rows) => {
                    println!(
                        "🗑️  Deleted {} event(s) for pair {} on {}",
                        rows, pair_id, date
                    );
                    let _ = db::ttlog(
                        conn,
                        "del",
                        "Delete pair events on date",
                        &format!("Deleted {} events for date={} pair={}", rows, date, pair_id),
                    );
                }
                Err(e) => eprintln!("❌ Error deleting pair events: {}", e),
            }
        } else {
            // Delete the entire day records
            let ev_n = db::count_events_by_date(conn, date).unwrap_or(0);
            let ws_n = db::count_sessions_by_date(conn, date).unwrap_or(0);

            if ev_n == 0 && ws_n == 0 {
                println!("⚠️  No events or work_sessions found for date {}", date);
                return Ok(());
            }

            // Delete all records for the date (work_sessions + events)
            print!(
                "Are you sure to delete the records of the date {} (N/y) ? ",
                date
            );
            let _ = io::stdout().flush();
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap_or(0);
            let choice = input.trim().to_lowercase();
            if choice != "y" {
                println!("Aborted. No rows deleted.");
                return Ok(());
            }

            match db::delete_events_by_date(conn, date) {
                Ok(ev_rows) => match db::delete_sessions_by_date(conn, date) {
                    Ok(ws_rows) => {
                        println!(
                            "🗑️  Deleted {} event(s) and {} work_session(s) for date {}",
                            ev_rows, ws_rows, date
                        );
                        let _ = db::ttlog(
                            conn,
                            "del",
                            "Delete all events and sessions for date",
                            &format!(
                                "Deleted date={} events={} work_sessions={}",
                                date, ev_rows, ws_rows
                            ),
                        );
                    }
                    Err(e) => eprintln!("❌ Error deleting work_sessions for date {}: {}", date, e),
                },
                Err(e) => eprintln!("❌ Error deleting events for date {}: {}", date, e),
            }
        }
    }
    Ok(())
}

/// Handle the `add` command
pub fn handle_add(cmd: &Commands, conn: &mut Connection, config: &Config) -> rusqlite::Result<()> {
    if let Commands::Add {
        date,
        pos_pos,
        start_pos,
        lunch_pos,
        end_pos,
        pos,
        start,
        lunch,
        end,
        edit_pair,
        edit,
    } = cmd
    {
        // validate date
        if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
            eprintln!("❌ Invalid date format: {} (expected YYYY-MM-DD)\n", date);

            print_help_add_command();

            return Ok(());
        }

        // merge positional and option values
        let pos = pos.clone().or(pos_pos.clone());
        let start = start.clone().or(start_pos.clone());
        let lunch = (*lunch).or(*lunch_pos);
        let end = end.clone().or(end_pos.clone());

        // --------------------------------------------------
        // EDIT MODE (explicit only)
        // --------------------------------------------------
        if *edit {
            let pair_id = match edit_pair {
                Some(p) => *p,
                None => {
                    eprintln!("\u{26a0}\u{FE0F} Missing --pair <id> with --edit");
                    return Ok(());
                }
            };

            let events = db::list_events_by_date(conn, date)?;
            if events.is_empty() {
                eprintln!("\u{26a0}\u{FE0F} No events for date {} to edit", date);
                return Ok(());
            }

            let enriched = compute_event_pairs(&events);
            let mut in_event: Option<db::Event> = None;
            let mut out_event: Option<db::Event> = None;
            for ew in enriched.iter().filter(|e| e.pair == pair_id) {
                if ew.event.kind == "in" {
                    in_event = Some(ew.event.clone());
                } else if ew.event.kind == "out" {
                    out_event = Some(ew.event.clone());
                }
            }

            if in_event.is_none() && out_event.is_none() {
                eprintln!(
                    "\u{26a0}\u{FE0F} Pair {} not found for date {}",
                    pair_id, date
                );
                return Ok(());
            }

            // Basic time validations (condensed)
            if let Some(s) = start.as_ref()
                && NaiveTime::parse_from_str(s, "%H:%M").is_err()
            {
                eprintln!("\u{274c} Invalid start time: {}", s);
                print_help_add_command();
                return Ok(());
            }
            if let Some(e_t) = end.as_ref()
                && NaiveTime::parse_from_str(e_t, "%H:%M").is_err()
            {
                eprintln!("\u{274c} Invalid end time: {}", e_t);
                print_help_add_command();
                return Ok(());
            }
            if let (Some(s), Some(e_t)) = (start.as_ref(), end.as_ref())
                && let (Ok(ts), Ok(te)) = (
                    NaiveTime::parse_from_str(s, "%H:%M"),
                    NaiveTime::parse_from_str(e_t, "%H:%M"),
                )
                && te <= ts
            {
                eprintln!(
                    "\u{274c} End time must be after start time ({} >= {})",
                    e_t, s
                );
                return Ok(());
            }

            // Create missing events if the user tries to complete the pair
            if let Some(sv) = start.as_ref()
                && in_event.is_none()
            {
                in_event = create_missing_event(
                    conn,
                    date,
                    sv.as_str(),
                    "in",
                    &pos,
                    out_event.as_ref(),
                    config,
                )?;
            }

            // If user provided an end but the 'out' event is missing, create it using the shared helper
            if let Some(ev_t) = end.as_ref()
                && out_event.is_none()
            {
                out_event = create_missing_event(
                    conn,
                    date,
                    ev_t.as_str(),
                    "out",
                    &pos,
                    in_event.as_ref(),
                    config,
                )?;
            }

            // Apply edits on existing events
            let mut changes: Vec<String> = Vec::new();

            if let Some(p) = pos.as_ref() {
                let p_norm = p.trim().to_uppercase();
                if p_norm != "O" && p_norm != "R" && p_norm != "H" && p_norm != "C" && p_norm != "M"
                {
                    eprintln!("\u{274c} Invalid position: {}", p_norm);
                    print_help_add_command();
                    return Ok(());
                }
                if let Some(ie) = in_event.as_ref() {
                    let _ = db::set_event_position(conn, ie.id, &p_norm);
                }
                if let Some(oe) = out_event.as_ref() {
                    let _ = db::set_event_position(conn, oe.id, &p_norm);
                }
                // After updating event positions, compute aggregate across all events for that date
                match db::aggregate_position_from_events(conn, date) {
                    Ok(Some(agg)) => {
                        // If aggregate is a single char (O/R/H/C/M), force it into work_sessions
                        let _ = db::force_set_position(conn, date, &agg);
                        println!(
                            "\u{2705} Position {} set for {} (pair {})",
                            agg, date, pair_id
                        );
                    }
                    Ok(None) => {
                        // No events for this date (unlikely here) -> fall back to provided p_norm
                        let _ = db::force_set_position(conn, date, &p_norm);
                        println!(
                            "\u{2705} Position {} set for {} (pair {})",
                            p_norm, date, pair_id
                        );
                    }
                    Err(e) => eprintln!("\u{26a0}\u{FE0F} Failed to aggregate positions: {}", e),
                }
                changes.push(format!("pos={}", p_norm));
            }

            if let (Some(sv), Some(ie)) = (start.as_ref(), in_event.as_ref()) {
                let _ = db::set_event_time(conn, ie.id, sv.as_str());
                let _ = db::force_set_start(conn, date, sv.as_str());
                println!("\u{2705} Start {} updated (pair {})", sv, pair_id);
                changes.push(format!("start={}", sv));
            }

            if let (Some(ev_t), Some(oe)) = (end.as_ref(), out_event.as_ref()) {
                let _ = db::set_event_time(conn, oe.id, ev_t.as_str());
                let _ = db::force_set_end(conn, date, ev_t.as_str());
                println!("\u{2705} End {} updated (pair {})", ev_t, pair_id);
                changes.push(format!("end={}", ev_t));
            }

            if let Some(lv) = lunch {
                if !(0..=90).contains(&lv) {
                    eprintln!("\u{274c} Invalid lunch break: {}", lv);
                    print_help_add_command();
                    return Ok(());
                }
                if let Some(oe) = out_event.as_ref() {
                    let _ = db::set_event_lunch(conn, oe.id, lv);
                    let _ = db::force_set_lunch(conn, date, lv);
                    println!("\u{2705} Lunch {} min updated (pair {})", lv, pair_id);
                    changes.push(format!("lunch={}", lv));
                }
            }

            if changes.is_empty() {
                eprintln!(
                    "\u{26a0}\u{FE0F} No fields provided to edit (use --pos/--in/--out/--lunch)"
                );
                print_help_add_command();
            } else if let Err(e) = db::ttlog(
                conn,
                "edit",
                "Edit existing pair events",
                &format!("date={} pair={} | {}", date, pair_id, changes.join(", ")),
            ) {
                eprintln!("\u{26a0}\u{FE0F} Failed to log edit: {}", e);
            }

            return Ok(());
        }

        // --------------------------------------------------
        // NORMAL MODE (always create / upsert fields, never implicit edit of existing pair)
        // --------------------------------------------------

        // Apply edits on existing events
        let mut changes: Vec<String> = Vec::new();

        // Handle position
        if let Some(p) = pos.as_ref() {
            let ptrim = p.trim().to_uppercase();
            if ptrim != "O" && ptrim != "R" && ptrim != "H" && ptrim != "C" {
                eprintln!(
                    "\u{274c} Invalid position: {} (use O=office or R=remote or H=Holiday or C=On-Site)",
                    ptrim
                );
                return Ok(());
            }
            let _ = db::upsert_position(conn, date, &ptrim);
            let (pos_string, _) = describe_position(&ptrim);
            println!("\u{2705} Position {} set for {}", pos_string, date);
            changes.push(format!("position={}", p));
        }

        // Handle start time
        if let Some(sv) = start.as_ref() {
            if NaiveTime::parse_from_str(sv, "%H:%M").is_err() {
                eprintln!("\u{274c} Invalid start time: {} (expected HH:MM)", sv);
                return Ok(());
            }
            db::upsert_start(conn, date, sv.as_str())?;
            println!("\u{2705} Start time {} registered for {}", sv, date);
            changes.push(format!("start={}", sv));

            // event in
            let event_pos_owned: Option<String> = pos.as_ref().map(|p| p.trim().to_uppercase());
            let args = db::AddEventArgs {
                date,
                time: sv.as_str(),
                kind: "in",
                position: event_pos_owned.as_deref(),
                source: "cli",
                meta: None,
            };
            if let Err(e) = db::add_event(conn, &args, config) {
                eprintln!("\u{26a0}\u{FE0F} Failed to insert event (in): {}", e);
            }
            // After creating an event, recompute aggregated position and set work_sessions appropriately
            if let Ok(Some(agg)) = db::aggregate_position_from_events(conn, date) {
                let _ = db::force_set_position(conn, date, &agg);
            }
        }

        // Handle lunch
        if let Some(l) = lunch {
            if !(0..=90).contains(&l) {
                eprintln!(
                    "\u{274c} Invalid lunch break: {} (must be between 0 and 90 minutes)",
                    l
                );
                return Ok(());
            }
            db::upsert_lunch(conn, date, l)?;
            println!("\u{2705} Lunch {} min registered for {}", l, date);
            changes.push(format!("lunch={}", l));

            // Also, if there is an out event present, set its lunch_break for compatibility
            match db::last_out_before(conn, date, "23:59") {
                Ok(Some(out_ev)) => {
                    if out_ev.lunch_break == 0
                        && let Err(e) = db::set_event_lunch(conn, out_ev.id, l)
                    {
                        eprintln!(
                            "\u{26a0}\u{FE0F} Failed to set lunch on event {}: {}",
                            out_ev.id, e
                        );
                    }
                }
                Ok(None) => {}
                Err(e) => eprintln!(
                    "\u{26a0}\u{FE0F} Error while searching for last out event: {}",
                    e
                ),
            }
        }

        // Handle end time
        if let Some(ev_t) = end.as_ref() {
            if NaiveTime::parse_from_str(ev_t, "%H:%M").is_err() {
                eprintln!("\u{274c} Invalid end time: {} (expected HH:MM)", ev_t);
                return Ok(());
            }
            db::upsert_end(conn, date, ev_t.as_str())?;
            println!("\u{2705} End time {} registered for {}", ev_t, date);
            changes.push(format!("end={}", ev_t));

            let event_pos_owned: Option<String> = pos.as_ref().map(|p| p.trim().to_uppercase());
            let args = db::AddEventArgs {
                date,
                time: ev_t.as_str(),
                kind: "out",
                position: event_pos_owned.as_deref(),
                source: "cli",
                meta: None,
            };
            match db::add_event(conn, &args, config) {
                Ok(event_id) => {
                    if let Some(l) = lunch
                        && l > 0
                        && let Err(e) = db::set_event_lunch(conn, event_id as i32, l)
                    {
                        eprintln!(
                            "\u{26a0}\u{FE0F} Failed to set lunch on out event {}: {}",
                            event_id, e
                        );
                    }
                }
                Err(err) => {
                    eprintln!("\u{26a0}\u{FE0F} Failed to insert event (out): {}", err);
                }
            }

            // Recompute aggregate position after inserting out event
            if let Ok(Some(agg)) = db::aggregate_position_from_events(conn, date) {
                let _ = db::force_set_position(conn, date, &agg);
            }
        }

        if pos.is_none() && start.is_none() && lunch.is_none() && end.is_none() {
            eprintln!(
                "\u{26a0}\u{FE0F} Please provide at least one of: position, start, lunch, end (or use --edit --pair)"
            );
        }

        // Log the add operation if we recorded changes
        if !changes.is_empty() {
            let msg = format!("date={} | {}", date, changes.join(", "));
            if let Err(e) = db::ttlog(conn, "add", "Add record on events", &msg) {
                eprintln!("⚠️ Failed to write internal log: {}", e);
            }
        }

        // If the user provided only --pos (no events), keep existing behavior; otherwise aggregate handled above.
        // Retrieve the id of the last session for the given date and print
        match conn.prepare("SELECT id FROM work_sessions WHERE date = ?1 ORDER BY id DESC LIMIT 1")
        {
            Ok(mut stmt) => match stmt.query_row([date], |row| row.get::<_, i32>(0)) {
                Ok(last_id) => {
                    println!();
                    let _ = handle_list_with_highlight(None, None, conn, config, Some(last_id));
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => {}
                Err(e) => eprintln!("\u{274c} Error retrieving session id: {}", e),
            },
            Err(e) => eprintln!("\u{274c} Failed to prepare query for session id: {}", e),
        }
    }

    Ok(())
}

pub struct HandleListArgs {
    pub period: Option<String>,
    pub pos: Option<String>,
    pub now: bool,
    pub details: bool,
    pub events: bool,
    pub pairs: Option<usize>,
    pub summary: bool,
}

/// Compatible: wrapper that keeps the existing signature and calls the version with highlight = None
#[allow(clippy::too_many_arguments)]
pub fn handle_list(
    args: &HandleListArgs,
    conn: &Connection,
    config: &Config,
) -> rusqlite::Result<()> {
    // Calcola il "periodo effettivo":
    // - Se viene usato --now → ignoriamo il periodo e lavoriamo solo su oggi
    // - Se NON viene usato --now, NON viene usato --events e manca --period,
    //   allora di default usiamo il mese corrente (YYYY-MM).
    let mut effective_period = args.period.clone();

    if !args.now && !args.events && effective_period.is_none() {
        let today = chrono::Local::now().date_naive();
        effective_period = Some(format!("{:04}-{:02}", today.year(), today.month()));
    }

    if args.now {
        // Get today's date in YYYY-MM-DD
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let wd_type = match config.show_weekday.as_str() {
            "Short" => 's',
            "Long" => 'l',
            "None" => '\0',
            _ => 'm', // Medium default
        };

        // If user supplied --now --events but not --details, map to details for convenience
        if args.events && !args.details {
            let events_today = db::list_events_by_date(conn, &today)?;
            println!(
                "ℹ️  '--now --events' detected: use '--now --details'. Showing today's event details."
            );
            if events_today.is_empty() {
                println!("No events for today.");
                return Ok(());
            }
            print_events_table(&events_today, "Today's events");
            return Ok(());
        }

        return if args.details {
            // Show today's events (details)
            let events_today = db::list_events_by_date(conn, &today)?;
            if events_today.is_empty() {
                println!("No events for today.");
                return Ok(());
            }
            print_events_table(&events_today, "Today's events");
            Ok(())
        } else {
            // Default: show today's work_sessions (aggregated)
            let sessions = db::list_sessions_by_date(conn, &today)?;
            if sessions.is_empty() {
                println!("No record for today.");
                return Ok(());
            }
            println!("📅 Today's session(s):");
            let mut total_surplus = 0;
            let work_minutes = utils::parse_work_duration_to_minutes(&config.min_work_duration);
            let sep_ch = config.separator_char.chars().next().unwrap_or('-');
            for s in sessions {
                let (pos_string, pos_color) = describe_position(s.position.as_str());
                let has_start = !s.start.trim().is_empty();
                let has_end = !s.end.trim().is_empty();

                // Calculates the abbreviation of the weekday (default = medium → "Mon")
                let date_shown = if wd_type == '\0' {
                    s.date.clone()
                } else {
                    format!("{} ({})", s.date, weekday_str(&s.date, wd_type))
                };

                if has_start && !has_end {
                    let expected =
                        logic::calculate_expected_exit(&s.start, work_minutes, s.lunch, config);
                    let lunch_color = if s.lunch > 0 { "\x1b[0m" } else { "\x1b[90m" };
                    let lunch_str = if s.lunch > 0 {
                        mins2hhmm(s.lunch, None).unwrap_or_default()
                    } else {
                        "-".to_string()
                    };
                    let lunch_fmt = format!("{:^5}", lunch_str);
                    let end_color = if !s.end.is_empty() {
                        "\x1b[0m"
                    } else {
                        "\x1b[90m"
                    };
                    let end_str = if !s.end.is_empty() {
                        s.end
                    } else {
                        "-".to_string()
                    };
                    println!(
                        "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | {}Lunch {}\x1b[0m | {}End {}\x1b[0m | Expected {} | \x1b[90mSurplus {:^8}\x1b[0m",
                        s.id,
                        date_shown,
                        pos_color,
                        pos_string,
                        s.start,
                        lunch_color,
                        lunch_fmt,
                        end_color,
                        end_str,
                        expected.format("%H:%M"),
                        "-"
                    );
                    if utils::is_last_day_of_month(&s.date) {
                        print_separator(sep_ch, 25, 110);
                    }
                } else if has_start && has_end {
                    let _start_time = NaiveTime::parse_from_str(&s.start, "%H:%M").unwrap();
                    let _end_time = NaiveTime::parse_from_str(&s.end, "%H:%M").unwrap();
                    let pos_char = s.position.chars().next().unwrap_or('O');
                    let crosses_lunch = logic::crosses_lunch_window(&s.start, &s.end);
                    let effective_lunch =
                        logic::effective_lunch_minutes(s.lunch, &s.start, &s.end, pos_char, config);
                    if crosses_lunch && effective_lunch > 0 {
                        let expected = logic::calculate_expected_exit(
                            &s.start,
                            work_minutes,
                            effective_lunch,
                            config,
                        );
                        let surplus = logic::calculate_surplus(
                            &s.start,
                            effective_lunch,
                            &s.end,
                            work_minutes,
                            config,
                        );
                        let surplus_minutes = surplus.num_minutes();
                        total_surplus += surplus_minutes;
                        let color_code = if surplus_minutes < 0 {
                            "\x1b[31m"
                        } else {
                            "\x1b[32m"
                        };
                        println!(
                            "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | Lunch {:^5} | End {} | Expected {} | {}Surplus {:^8}\x1b[0m",
                            s.id,
                            date_shown,
                            pos_color,
                            pos_string,
                            s.start,
                            mins2hhmm(effective_lunch, None).unwrap_or_default(),
                            s.end,
                            expected.format("%H:%M"),
                            color_code,
                            format!("{}m", surplus_minutes)
                        );
                    } else {
                        let expected =
                            logic::calculate_expected_exit(&s.start, work_minutes, s.lunch, config);
                        let surplus = logic::calculate_surplus(
                            &s.start,
                            s.lunch,
                            &s.end,
                            work_minutes,
                            config,
                        );
                        let surplus_minutes = surplus.num_minutes();
                        total_surplus += surplus_minutes;
                        let color_code = if surplus_minutes < 0 {
                            "\x1b[31m"
                        } else {
                            "\x1b[32m"
                        };
                        println!(
                            "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | Lunch {:^5} | End {} | Expected {} | {}Surplus {:^8}\x1b[0m",
                            s.id,
                            date_shown,
                            pos_color,
                            pos_string,
                            s.start,
                            mins2hhmm(s.lunch, None).unwrap_or_default(),
                            s.end,
                            expected.format("%H:%M"),
                            color_code,
                            format!("{}m", surplus_minutes)
                        );
                    }
                    if utils::is_last_day_of_month(&s.date) {
                        print_separator(sep_ch, 25, 110);
                    }
                } else {
                    println!(
                        "{:>3}: {} | {}{:<16}\x1b[0m | -",
                        s.id, date_shown, pos_color, pos_string
                    );
                }
            }
            let (hh, mm) = utils::mins2readable(total_surplus as i32);
            let formatted_total = format!(
                "{}{}h {}m",
                if total_surplus < 0 { "-" } else { "" },
                hh,
                mm
            );
            println!("\nSummary surplus: {}", formatted_total);
            Ok(())
        };
    }

    // not `now`: if --events present, list all events; otherwise list work_sessions (legacy)
    if args.events {
        let events_all =
            db::list_events_filtered(conn, effective_period.as_deref(), args.pos.as_deref())?;
        if events_all.is_empty() {
            println!("No events recorded.");
            return Ok(());
        }
        // Compute pair/unmatched once
        let enriched = compute_event_pairs(&events_all);
        // --summary: produce aggregated rows per pair
        if args.summary {
            let mut summaries = compute_event_summaries(&enriched);
            if let Some(pf) = args.pairs {
                summaries.retain(|r| r.pair == pf);
            }
            print_events_summary(&summaries, "Event pairs summary");
            return Ok(());
        }
        // Filter by pairs if requested (detailed events' mode)
        let filtered: Vec<_> = if let Some(pfilter) = args.pairs {
            enriched.into_iter().filter(|e| e.pair == pfilter).collect()
        } else {
            enriched
        };
        let mut plain_events: Vec<db::Event> = Vec::with_capacity(filtered.len());
        let mut pair_map: Vec<(i32, usize, bool)> = Vec::with_capacity(filtered.len());
        for ewp in &filtered {
            plain_events.push(ewp.event.clone());
            pair_map.push((ewp.event.id, ewp.pair, ewp.unmatched));
        }
        print_events_table_with_pairs(&plain_events, &pair_map, "All events", args.pairs);
        return Ok(());
    }

    handle_list_with_highlight(effective_period, args.pos.clone(), conn, config, None)
}

/// New version: supports printing with `highlight_id: Option<i32>`
pub fn handle_list_with_highlight(
    period: Option<String>,
    pos: Option<String>,
    conn: &Connection,
    config: &Config,
    highlight_id: Option<i32>,
) -> rusqlite::Result<()> {
    // Normalize pos to uppercase
    let pos_upper = pos.as_ref().map(|p| p.trim().to_uppercase());

    let wd_type = match config.show_weekday.as_str() {
        "Short" => 's',
        "Long" => 'l',
        "None" => '\0',
        _ => 'm', // Medium default
    };

    // If highlight_id is Some(id) -> retrieve only that session (efficient single-row query).
    // Otherwise, retrieve the full list based on filters.
    let sessions = if let Some(id) = highlight_id {
        match db::get_session(conn, id)? {
            Some(s) => vec![s],
            None => Vec::new(),
        }
    } else {
        db::list_sessions(conn, period.as_deref(), pos_upper.as_deref())?
    };

    if highlight_id.is_none() {
        if let Some(p) = period {
            if p.len() == 4 {
                println!("📅 Saved sessions for year {}:", p);
            } else if p.len() == 7 {
                let parts: Vec<&str> = p.split('-').collect();
                let year = parts[0];
                let month = parts[1];
                println!(
                    "📅 Saved sessions for {} {}:",
                    logic::month_name(month),
                    year
                );
            }
        } else if let Some(p) = pos.as_deref() {
            println!("📅 Saved sessions for position {}:", p);
        } else {
            println!("📅 Saved sessions:");
        }
    } else {
        // When highlighting a single record (called from handle_add), avoid printing any header
        // to output exclusively the single record.
    }

    println!();

    if sessions.is_empty() {
        if highlight_id.is_some() {
            println!("⚠️  No recorded session found with the requested id");
        } else {
            println!("⚠️  No recorded sessions found");
        }
        return Ok(());
    }

    let mut total_surplus = 0;
    // Parse work_minutes once to avoid repeated parsing inside the loop
    let work_minutes = utils::parse_work_duration_to_minutes(&config.min_work_duration);
    // Separator character configurable from config (take first char, fallback to '-')
    let sep_ch = config.separator_char.chars().next().unwrap_or('-');

    for s in sessions {
        let (pos_string, pos_color) = describe_position(s.position.as_str());
        let has_start = !s.start.trim().is_empty();
        let has_end = !s.end.trim().is_empty();

        // Calculates the abbreviation of the weekday (default = medium → "Mon")
        let date_shown = if wd_type == '\0' {
            s.date.clone()
        } else {
            format!("{} ({})", s.date, weekday_str(&s.date, wd_type))
        };

        if has_start && !has_end {
            // Only start → calculate expected end
            let expected = logic::calculate_expected_exit(&s.start, work_minutes, s.lunch, config);

            let lunch_color = if s.lunch > 0 { "\x1b[0m" } else { "\x1b[90m" };
            let lunch_str = if s.lunch > 0 {
                mins2hhmm(s.lunch, None).unwrap_or_default()
            } else {
                "-".to_string()
            };
            let lunch_fmt = format!("{:^5}", lunch_str);

            let end_color = if !s.end.is_empty() {
                "\x1b[0m"
            } else {
                "\x1b[90m"
            };
            let end_str = if !s.end.is_empty() {
                s.end
            } else {
                "-".to_string()
            };
            let end_fmt = format!("{:^5}", end_str);

            println!(
                "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | {}Lunch {}\x1b[0m | {}End {}\x1b[0m | Expected {} | \x1b[90mSurplus {:^8}\x1b[0m",
                s.id,
                date_shown,
                pos_color,
                pos_string,
                s.start,
                lunch_color,
                lunch_fmt,
                end_color,
                end_fmt,
                expected.format("%H:%M"),
                "-",
            );
            // If this date is the last day of the month, print a separator after it
            if utils::is_last_day_of_month(&s.date) {
                print_separator(sep_ch, 25, 110);
            }
        } else if has_start && has_end {
            let _start_time = NaiveTime::parse_from_str(&s.start, "%H:%M").unwrap();
            let _end_time = NaiveTime::parse_from_str(&s.end, "%H:%M").unwrap();
            let pos_char = s.position.chars().next().unwrap_or('O');
            let crosses_lunch = logic::crosses_lunch_window(&s.start, &s.end);

            // Compute effective lunch
            let effective_lunch =
                logic::effective_lunch_minutes(s.lunch, &s.start, &s.end, pos_char, config);

            if crosses_lunch && effective_lunch > 0 {
                // Case with lunch (inserted or automatic)
                let expected =
                    logic::calculate_expected_exit(&s.start, work_minutes, effective_lunch, config);
                let surplus = logic::calculate_surplus(
                    &s.start,
                    effective_lunch,
                    &s.end,
                    work_minutes,
                    config,
                );
                let surplus_minutes = surplus.num_minutes();
                total_surplus += surplus_minutes;

                let color_code = if surplus_minutes < 0 {
                    "\x1b[31m" // red
                } else if surplus_minutes > 0 {
                    "\x1b[32m" // green
                } else {
                    "\x1b[0m"
                };

                let formatted_surplus = if surplus_minutes == 0 {
                    "0".to_string()
                } else {
                    format!("{:+}", surplus_minutes)
                };

                let lunch_str = if effective_lunch > 0 {
                    mins2hhmm(effective_lunch, None).unwrap_or_default()
                } else {
                    "-".to_string()
                };
                let lunch_fmt = format!("{:^5}", lunch_str);

                println!(
                    "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | Lunch {} | End {} | Expected {} | Surplus {}{:>4} min\x1b[0m",
                    s.id,
                    date_shown,
                    pos_color,
                    pos_string,
                    s.start,
                    lunch_fmt,
                    s.end,
                    expected.format("%H:%M"),
                    color_code,
                    formatted_surplus
                );
                if utils::is_last_day_of_month(&s.date) {
                    print_separator(sep_ch, 25, 110);
                }
            } else {
                let duration = _end_time - _start_time;
                let lunch_fmt = format!("{:^5}", "-".to_string());

                println!(
                    "{:>3}: {} | {}{:<16}\x1b[0m | Start {} | \x1b[90mLunch {}\x1b[0m | End {} | \x1b[36mWorked {:>2} h {:02} min\x1b[0m",
                    s.id,
                    date_shown,
                    pos_color,
                    pos_string,
                    s.start,
                    lunch_fmt,
                    s.end,
                    duration.num_hours(),
                    duration.num_minutes() % 60
                );
                if utils::is_last_day_of_month(&s.date) {
                    print_separator(sep_ch, 25, 110);
                }
            }
        } else {
            let lunch_str = if s.lunch > 0 {
                mins2hhmm(s.lunch, None).unwrap_or_default()
            } else {
                "-".to_string()
            };

            let lunch_fmt = format!("{:^5}", lunch_str);

            println!(
                "{:>3}: {} | {}{:<16}\x1b[0m | \x1b[90mStart {:^5} | Lunch {} | End {:^5} | Expected {:^5} | Surplus {:>4} min\x1b[0m",
                s.id,
                date_shown,
                pos_color,
                pos_string,
                if has_start { &s.start } else { "-" },
                lunch_fmt,
                if has_end { &s.end } else { "-" },
                "-",
                "-",
            );
            if utils::is_last_day_of_month(&s.date) {
                print_separator(sep_ch, 25, 110);
            }
        }
    }

    if highlight_id.is_none() {
        println!();
        print_separator(sep_ch, 25, 110);

        if total_surplus != 0 {
            let color_code = if total_surplus < 0 {
                "\x1b[31m" // red
            } else {
                "\x1b[32m" // green
            };

            let (hh, mm) = utils::mins2readable(total_surplus as i32);
            let formatted_total = format!(
                "{}{}h {}m",
                if total_surplus < 0 { "-" } else { "" },
                hh,
                mm
            );

            println!(
                "{:>119}",
                format!(
                    "Σ Total surplus: {}{:>4}\x1b[0m",
                    color_code, formatted_total
                ),
            );
        } else {
            println!("{:>119}", format!("Σ Total surplus: {:>4} min", 0));
        }
    }

    Ok(())
}

/// Print rows from the internal `log` table when requested
pub fn handle_log(cmd: &Commands, conn: &Connection) -> rusqlite::Result<()> {
    if matches!(cmd, Commands::Log { print: true }) {
        let mut stmt = conn.prepare_cached(
            "SELECT id, date, operation, target, message FROM log ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i32>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?;

        println!("📜 Internal log:");
        for r in rows {
            let (id, date, operation, target, message) = r?;
            if target.is_empty() {
                println!("{:>3}: {} | {} | {}", id, date, operation, message);
            } else {
                println!(
                    "{:>3}: {} | {} ({}) | {}",
                    id, date, operation, target, message
                );
            }
        }
    }
    Ok(())
}

pub fn handle_backup(config: &Config, file: &str, compress: &bool) -> io::Result<()> {
    let src = Path::new(&config.database);
    let dest = Path::new(file);

    if !src.exists() {
        eprintln!("❌ Source database not found at {:?}", src);
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::copy(src, dest)?;
    println!("✅ Backup created: {}", dest.display());

    // If compress is active → get the name of the compressed file
    let final_path = if *compress {
        // Compress the just-created backup file. If compression succeeds, remove the
        // original (uncompressed) backup file because it's no longer needed.
        let compressed = compress_backup(dest)?;
        // Try to remove the original backup file; do not fail the whole operation if
        // removal fails — just emit a warning.
        if compressed != dest.to_path_buf() {
            if let Err(e) = fs::remove_file(dest) {
                eprintln!(
                    "���️ Failed to delete original backup file {}: {}",
                    dest.display(),
                    e
                );
            } else {
                println!(
                    "🗑️ Original uncompressed backup deleted: {}",
                    dest.display()
                );
            }
        }
        compressed
    } else {
        dest.to_path_buf()
    };

    if let Ok(conn) = Connection::open(src) {
        let _ = db::ttlog(
            &conn,
            "backup",
            &final_path.to_string_lossy(),
            if *compress {
                "Database backup created and compressed"
            } else {
                "Database backup created"
            },
        );
    }

    Ok(())
}

/// Support struct to enrich JSON output and compute pair/unmatched
#[derive(serde::Serialize, Clone)]
struct EventWithPair {
    #[serde(flatten)]
    event: db::Event,
    pair: usize,
    unmatched: bool,
}

/// Compute pair ids (per-date sequence) and unmatched flag for a slice of events.
/// Rules:
///  - Every 'in' event opens a new pair with an incremented pair id (per date) and unmatched=true
///  - The first subsequent 'out' closes the earliest open pair (FIFO) and uses the same pair id,
///    setting unmatched=false for both the 'in' and the 'out'
///  - An 'out' without a preceding 'in' creates a new pair id with unmatched=true
fn compute_event_pairs(events: &[db::Event]) -> Vec<EventWithPair> {
    use std::collections::VecDeque;
    let mut result: Vec<EventWithPair> = Vec::with_capacity(events.len());
    let mut current_date = String::new();
    let mut open_in_queue: VecDeque<usize> = VecDeque::new();
    let mut pair_counter: usize = 0;
    for ev in events {
        if ev.date != current_date {
            // reset for a new date
            current_date = ev.date.clone();
            open_in_queue.clear();
            pair_counter = 0;
        }
        match ev.kind.as_str() {
            "in" => {
                pair_counter += 1;
                result.push(EventWithPair {
                    event: ev.clone(),
                    pair: pair_counter,
                    unmatched: true,
                });
                open_in_queue.push_back(result.len() - 1);
            }
            "out" => {
                if let Some(in_idx) = open_in_queue.pop_front() {
                    let pair_id = result[in_idx].pair;
                    result[in_idx].unmatched = false; // closed match
                    result.push(EventWithPair {
                        event: ev.clone(),
                        pair: pair_id,
                        unmatched: false,
                    });
                } else {
                    pair_counter += 1; // orphan out
                    result.push(EventWithPair {
                        event: ev.clone(),
                        pair: pair_counter,
                        unmatched: true,
                    });
                }
            }
            _ => {
                pair_counter += 1;
                result.push(EventWithPair {
                    event: ev.clone(),
                    pair: pair_counter,
                    unmatched: true,
                });
            }
        }
    }
    result
}

#[derive(serde::Serialize, Clone, Debug)]
struct SummaryRow {
    date: String,
    pair: usize,
    position: String,
    start: String,
    end: String,
    lunch_minutes: i32,
    duration_minutes: i32,
    unmatched: bool,
}

fn compute_event_summaries(enriched: &[EventWithPair]) -> Vec<SummaryRow> {
    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Accum {
        date: String,
        pair: usize,
        position: String,
        start: Option<String>,
        end: Option<String>,
        lunch: i32,
        unmatched_in: bool,
        unmatched_out: bool,
    }
    let mut map: BTreeMap<(String, usize), Accum> = BTreeMap::new();
    for e in enriched {
        let key = (e.event.date.clone(), e.pair);
        let acc = map.entry(key.clone()).or_insert_with(|| Accum {
            date: key.0.clone(),
            pair: key.1,
            position: String::new(),
            start: None,
            end: None,
            lunch: 0,
            unmatched_in: false,
            unmatched_out: false,
        });
        if e.event.kind == "in" {
            if acc.start.is_none() {
                acc.start = Some(e.event.time.clone());
            }
            if acc.position.is_empty() {
                acc.position = e.event.position.clone();
            }
            if e.unmatched {
                acc.unmatched_in = true;
            }
        } else if e.event.kind == "out" {
            if acc.end.is_none() {
                acc.end = Some(e.event.time.clone());
            }
            if acc.position.is_empty() {
                acc.position = e.event.position.clone();
            }
            if e.event.lunch_break > 0 {
                acc.lunch = e.event.lunch_break;
            }
            if e.unmatched {
                acc.unmatched_out = true;
            }
        }
    }
    let mut rows: Vec<SummaryRow> = Vec::new();
    for (_, acc) in map.into_iter() {
        let unmatched = (acc.start.is_some() && acc.end.is_none())
            || (acc.start.is_none() && acc.end.is_some());
        // Compute duration
        let mut duration_minutes = 0;
        if let (Some(s), Some(e)) = (acc.start.as_ref(), acc.end.as_ref())
            && let (Ok(st), Ok(et)) = (
                NaiveTime::parse_from_str(s, "%H:%M"),
                NaiveTime::parse_from_str(e, "%H:%M"),
            )
        {
            let mut diff = (et - st).num_minutes() as i32;
            if acc.lunch > 0 {
                diff -= acc.lunch;
            }
            if diff < 0 {
                diff = 0;
            }
            duration_minutes = diff;
        }
        rows.push(SummaryRow {
            date: acc.date,
            pair: acc.pair,
            position: acc.position,
            start: acc.start.unwrap_or_else(|| "-".to_string()),
            end: acc.end.unwrap_or_else(|| "-".to_string()),
            lunch_minutes: acc.lunch,
            duration_minutes,
            unmatched,
        });
    }
    rows
}

fn print_events_summary(rows: &[SummaryRow], title: &str) {
    println!("\u{1F4CA} {}:", title);
    if rows.is_empty() {
        println!("(no pairs)");
        return;
    }
    // Determine widths
    let mut w_date = 10usize;
    let mut w_pair = 4usize;
    let mut w_pos = 3usize;
    let mut w_start = 5usize;
    let mut w_end = 5usize;
    let mut w_lunch = 5usize;
    // We'll display duration as "XH YYM" (e.g. "8H 00M") so compute formatted strings first
    let mut formatted_dur: Vec<String> = Vec::with_capacity(rows.len());
    let mut w_dur = 3usize;
    for r in rows {
        w_date = w_date.max(r.date.len());
        w_pair = w_pair.max(format!("{}{}", r.pair, if r.unmatched { "*" } else { "" }).len());
        w_pos = w_pos.max(r.position.len());
        w_start = w_start.max(r.start.len());
        w_end = w_end.max(r.end.len());
        w_lunch = w_lunch.max(r.lunch_minutes.to_string().len());
        // prepare formatted duration
        let mins = r.duration_minutes.max(0);
        let hh = mins / 60;
        let mm = mins % 60;
        let dur_str = format!("{}H {:02}M", hh, mm);
        w_dur = w_dur.max(dur_str.len());
        formatted_dur.push(dur_str);
    }
    println!(
        "{:<date$}  {:>pair$}  {:<pos$}  {:>start$}  {:>end$}  {:>lunch$}  {:>dur$}",
        "Date",
        "Pair",
        "Pos",
        "Start",
        "End",
        "Lunch",
        "Dur",
        date = w_date,
        pair = w_pair,
        pos = w_pos,
        start = w_start,
        end = w_end,
        lunch = w_lunch,
        dur = w_dur
    );
    println!(
        "{}  {}  {}  {}  {}  {}  {}",
        "-".repeat(w_date),
        "-".repeat(w_pair),
        "-".repeat(w_pos),
        "-".repeat(w_start),
        "-".repeat(w_end),
        "-".repeat(w_lunch),
        "-".repeat(w_dur),
    );
    for (i, r) in rows.iter().enumerate() {
        let pair_disp = format!("{}{}", r.pair, if r.unmatched { "*" } else { "" });
        let dur_display = &formatted_dur[i];
        println!(
            "{:<date$}  {:>pair$}  {:<pos$}  {:>start$}  {:>end$}  {:>lunch$}  {:>dur$}",
            r.date,
            pair_disp,
            r.position,
            r.start,
            r.end,
            r.lunch_minutes,
            dur_display,
            date = w_date,
            pair = w_pair,
            pos = w_pos,
            start = w_start,
            end = w_end,
            lunch = w_lunch,
            dur = w_dur
        );
    }
}

// Helper to print events in aligned table format
fn print_events_table_with_pairs(
    events: &[db::Event],
    pair_map: &[(i32, usize, bool)],
    title: &str,
    filter_pair: Option<usize>,
) {
    println!("\u{1F4C5} {}:", title);
    if events.is_empty() {
        return;
    }
    println!();

    // Build lookup id -> (pair, unmatched)
    use std::collections::HashMap;
    let mut meta: HashMap<i32, (usize, bool)> = HashMap::with_capacity(pair_map.len());
    for (id, pair, un) in pair_map {
        meta.insert(*id, (*pair, *un));
    }

    // Determine columns (Pair column with possible suffix *)
    let mut w_id = 2usize;
    let mut w_date = 10usize;
    let mut w_time = 5usize;
    let mut w_kind = 4usize;
    let mut w_pos = 3usize;
    let mut w_lunch = 5usize;
    let mut w_src = 5usize;
    let mut w_pair = 4usize;
    for e in events {
        if let Some((pair, unmatched)) = meta.get(&e.id) {
            let tag = if *unmatched {
                format!("{}*", pair)
            } else {
                pair.to_string()
            };
            w_pair = w_pair.max(tag.len());
        }
        w_id = w_id.max(e.id.to_string().len());
        w_date = w_date.max(e.date.len());
        w_time = w_time.max(e.time.len());
        w_kind = w_kind.max(e.kind.len());
        w_pos = w_pos.max(e.position.len());
        w_lunch = w_lunch.max(e.lunch_break.to_string().len());
        w_src = w_src.max(e.source.len());
    }

    println!(
        "{:<id$}  {:<date$}  {:<time$}  {:<kind$}  {:<pos$}  {:>lunch$}  {:<src$}  {:>pair$}",
        "ID",
        "Date",
        "Time",
        "Kind",
        "Pos",
        "Lunch",
        "Src",
        "Pair",
        id = w_id,
        date = w_date,
        time = w_time,
        kind = w_kind,
        pos = w_pos,
        lunch = w_lunch,
        src = w_src,
        pair = w_pair
    );
    println!(
        "{:-<1$}  {:-<2$}  {:-<3$}  {:-<4$}  {:-<5$}  {:-<6$}  {:-<7$}  {:-<8$}",
        "", w_id, w_date, w_time, w_kind, w_pos, w_lunch, w_src, w_pair
    );

    for e in events {
        if let Some(fp) = filter_pair
            && let Some((pair_id, _)) = meta.get(&e.id)
            && *pair_id != fp
        {
            continue;
        }
        let (pair_id, unmatched) = meta.get(&e.id).cloned().unwrap_or((0, true));
        let pair_display = if unmatched {
            format!("{}*", pair_id)
        } else {
            pair_id.to_string()
        };
        println!(
            "{:<id$}  {:<date$}  {:<time$}  {:<kind$}  {:<pos$}  {:>lunch$}  {:<src$}  {:>pair$}",
            e.id,
            e.date,
            e.time,
            e.kind,
            e.position,
            e.lunch_break,
            e.source,
            pair_display,
            id = w_id,
            date = w_date,
            time = w_time,
            kind = w_kind,
            pos = w_pos,
            lunch = w_lunch,
            src = w_src,
            pair = w_pair
        );
    }
}

// Keep backward-compatible old function but delegate to the new enriched version
fn print_events_table(events: &[db::Event], title: &str) {
    let enriched = compute_event_pairs(events);
    let mut plain: Vec<db::Event> = Vec::with_capacity(enriched.len());
    let mut map: Vec<(i32, usize, bool)> = Vec::with_capacity(enriched.len());
    for e in enriched.iter() {
        plain.push(e.event.clone());
        map.push((e.event.id, e.pair, e.unmatched));
    }
    print_events_table_with_pairs(&plain, &map, title, None);
}
