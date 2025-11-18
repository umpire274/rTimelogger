use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::env;
use std::path::PathBuf;

mod common;
use common::rti;

/// Create a unique test DB path inside the system temp dir
fn setup_test_db(name: &str) -> String {
    // Cross-platform: /tmp su Linux/macOS, %TEMP% su Windows
    let mut path: PathBuf = env::temp_dir();
    path.push(format!("{}_rtimelogger.sqlite", name));

    let db_path = path.to_string_lossy().to_string();

    // Rimuove il file se esiste già (reset)
    std::fs::remove_file(&db_path).ok();

    db_path
}

#[test]
fn test_list_sessions_all() {
    let db_path = setup_test_db("export_events_csv_all");

    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-08-31",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-15",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2024-09-10",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args(["--db", &db_path, "list", "--period", "2024-09:2025-09"])
        .assert()
        .success()
        .stdout(contains("2025-08-31"))
        .stdout(contains("2025-09-15"))
        .stdout(contains("2024-09-10"));
}

#[test]
fn test_list_sessions_filter_year() {
    let db_path = setup_test_db("year");

    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-01-10",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-05-20",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2024-12-31",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args(["--db", &db_path, "list", "--period", "2025"])
        .assert()
        .success()
        .stdout(contains("2025-01-10"))
        .stdout(contains("2025-05-20"))
        .stdout(contains("📅 Saved sessions for year 2025:"))
        .stdout(
            predicates::str::is_match("2024-12-31")
                .expect("Invalid regex")
                .not(),
        );
}

#[test]
fn test_list_sessions_filter_year_month() {
    let db_path = setup_test_db("year_month");

    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-01",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-15",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-10-01",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2024-09-01",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args(["--db", &db_path, "list", "--period", "2025-09"])
        .assert()
        .success()
        .stdout(contains("2025-09-01"))
        .stdout(contains("2025-09-15"))
        .stdout(contains("📅 Saved sessions for September 2025:"))
        .stdout(
            predicates::str::is_match("2025-10-01")
                .expect("Invalid regex")
                .not(),
        )
        .stdout(
            predicates::str::is_match("2024-09-01")
                .expect("Invalid regex")
                .not(),
        );
}

#[test]
fn test_list_sessions_filter_position() {
    let db_path = setup_test_db("filter_position");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add Office (O)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-10",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // Add Remote (R)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-11",
            "R",
            "09:15",
            "0",
            "17:15",
        ])
        .assert()
        .success();

    // Add Holiday (H)
    rti()
        .args(["--db", &db_path, "--test", "add", "2025-09-12", "H"])
        .assert()
        .success();

    // Filter O
    rti()
        .args([
            "--db", &db_path, "--test", "list", "--period", "2025-09", "--pos", "O",
        ])
        .assert()
        .success()
        .stdout(contains("2025-09-10"))
        .stdout(contains("Office"))
        .stdout(contains("2025-09-11").not())
        .stdout(contains("2025-09-12").not());

    // Filter R
    rti()
        .args([
            "--db", &db_path, "--test", "list", "--period", "2025-09", "--pos", "R",
        ])
        .assert()
        .success()
        .stdout(contains("2025-09-11"))
        .stdout(contains("Remote"))
        .stdout(contains("2025-09-10").not())
        .stdout(contains("2025-09-12").not());

    // Filter H
    rti()
        .args([
            "--db", &db_path, "--test", "list", "--period", "2025-09", "--pos", "H",
        ])
        .assert()
        .success()
        .stdout(contains("2025-09-12"))
        .stdout(contains("Holiday"))
        .stdout(contains("2025-09-10").not())
        .stdout(contains("2025-09-11").not());
}

#[test]
fn test_list_sessions_invalid_period() {
    let db_path = setup_test_db("invalid_period");

    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-01",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args(["--db", &db_path, "list", "--period", "2025-9"])
        .assert()
        .failure()
        .stderr(contains("InvalidQuery"));
}

#[test]
fn test_add_and_list_with_company_position() {
    let db_path = setup_test_db("with_company_position");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add a session in company mode (A), crossing lunch window but without specifying lunch
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-14",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // List should show Pos A and Lunch 30 min (auto-applied)
    rti()
        .args(["--db", &db_path, "list", "--period", "2025-09-14"])
        .assert()
        .success()
        .stdout(contains("Office"))
        .stdout(contains("Lunch 00:30"))
        .stdout(contains("Expected"))
        .stdout(contains("Surplus"));
}

#[test]
fn test_add_and_list_with_remote_position_lunch_zero() {
    let db_path = setup_test_db("with_remote_position_lunch_zero");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add a session in remote mode (R), crossing lunch window, no lunch specified
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-15",
            "R",
            "09:00",
            "0",
            "17:00",
        ])
        .assert()
        .success();

    // List should show Pos R and Lunch 0 min (allowed)
    rti()
        .args(["--db", &db_path, "list", "--period", "2025-09-15"])
        .assert()
        .success()
        .stdout(contains("Remote"))
        .stdout(contains("Lunch   -"));
}

#[test]
fn test_add_and_list_incomplete_session() {
    let db_path = setup_test_db("incomplete_session");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add only start time (no end)
    rti()
        .args(["--db", &db_path, "add", "2025-09-16", "O", "09:00"])
        .assert()
        .success();

    // List should show Pos A and Start 09:00 but End "-"
    rti()
        .args(["--db", &db_path, "list", "--period", "2025-09-16"])
        .assert()
        .success()
        .stdout(contains("Office"))
        .stdout(contains("Start 09:00"))
        .stdout(contains("End   -"));
}

#[test]
fn test_add_and_list_holiday_position() {
    let db_path = setup_test_db("holiday_position");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Adding a day with Holiday position
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-21",
            "--pos",
            "H",
        ])
        .assert()
        .success()
        .stdout(contains("Position Holiday"));

    // List should show 'Holiday' as position and no more data's
    rti()
        .args(["--db", &db_path, "--test", "list", "--period", "2025-09-21"])
        .assert()
        .success()
        .stdout(contains("Holiday"));
}

#[test]
fn test_list_sessions_positions_with_colors() {
    // (Position, Label atteso, Codice ANSI atteso)
    let cases = vec![
        ("O", "Office", "\x1b[34m"),           // Office → blu
        ("R", "Remote", "\x1b[36m"),           // Remote → ciano
        ("C", "On-site (Client)", "\x1b[33m"), // Client → giallo
        ("H", "Holiday", "\x1b[45;97;1m"),     // Holiday → viola bg + bold
    ];

    for (pos, label, color) in cases {
        let db_path = setup_test_db(&format!("pos_{}", pos));

        // Init DB
        rti()
            .args(["--db", &db_path, "--test", "init"])
            .assert()
            .success();

        // Add session (Holiday non ha start/end, le altre sì)
        let mut args = vec!["--db", &db_path, "--test", "add", "2025-09-15", pos];
        if pos != "H" {
            args.extend(&["09:00", "30", "17:00"]);
        }

        rti().args(&args).assert().success();

        // List filtrato per posizione → deve contenere label e colore
        rti()
            .args([
                "--db", &db_path, "--test", "list", "--period", "2025-09", "--pos", pos,
            ])
            .assert()
            .success()
            .stdout(contains(label))
            .stdout(contains(color));
    }
}

#[test]
fn test_add_and_delete_session() {
    let db_path = setup_test_db("delete_session");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add a session
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-20",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // Verify session is listed
    rti()
        .args(["--db", &db_path, "--test", "list", "--period", "2025-09-20"])
        .assert()
        .success()
        .stdout(contains("2025-09-20"));

    // Delete by date (new behavior) -- answer 'y' to confirmation prompt
    rti()
        .args(["--db", &db_path, "--test", "del", "2025-09-20"])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(contains("Deleted").or(contains("deleted")));

    // Verify session no longer appears in list
    rti()
        .args(["--db", &db_path, "--test", "list", "--period", "2025-09-20"])
        .assert()
        .success()
        .stdout(contains("2025-09-20").not());
}

#[test]
fn test_delete_nonexistent_session() {
    let db_path = setup_test_db("delete_nonexistent");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Try to delete a date that does not exist: confirm with 'y' and expect 0 rows deleted
    rti()
        .args(["--db", &db_path, "--test", "del", "2099-01-01"])
        .write_stdin("y\n")
        .assert()
        .success() // the command should not error
        .stdout(contains("No events or work_sessions found for date"));
}

#[test]
fn test_separator_after_month_end() {
    let db_path = setup_test_db("separator_month_end");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add last day of September and first day of October
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-30",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-10-01",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // List and assert separator (25 '-' characters) is present after the 2025-09-30 line
    let sep25 = "-".repeat(25);

    rti()
        .args(["--db", &db_path, "list", "--period", "2025-09"])
        .assert()
        .success()
        .stdout(contains("2025-09-30"))
        .stdout(contains(sep25));
}

#[test]
fn test_list_events_filter_position_case_insensitive() {
    let db_path = setup_test_db("events_pos_case");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add Remote (R) session which creates two events (in/out)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-21",
            "R",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // Add Office (O) session
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-09-22",
            "O",
            "09:10",
            "30",
            "17:10",
        ])
        .assert()
        .success();

    // List events filtering with lowercase 'r' to verify case-insensitive normalization
    rti()
        .args(["--db", &db_path, "--test", "list", "--events", "--pos", "r"])
        .assert()
        .success()
        .stdout(contains("2025-09-21")) // remote date present
        .stdout(contains("2025-09-22").not()); // office date absent
}

#[test]
fn test_events_pair_column_and_grouping() {
    let db_path = setup_test_db("events_pair_col");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Prima sessione (in/out)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "R",
            "09:00",
            "30",
            "12:00",
        ])
        .assert()
        .success();

    // Seconda sessione (in/out)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "R",
            "13:00",
            "0",
            "17:00",
        ])
        .assert()
        .success();

    // Lista eventi e verifica intestazione Pair e presenza dei pair id 1 e 2
    rti()
        .args(["--db", &db_path, "--test", "list", "--events", "--pos", "R"])
        .assert()
        .success()
        .stdout(contains("Pair"))
        .stdout(contains("  1"))
        .stdout(contains("  2"));
}

#[test]
fn test_delete_existing_pair() {
    let db_path = setup_test_db("delete_existing_pair");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Pair 1 (09:00-12:00)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "R",
            "09:00",
            "30",
            "12:00",
        ])
        .assert()
        .success();

    // Pair 2 (13:00-17:00)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "R",
            "13:00",
            "0",
            "17:00",
        ])
        .assert()
        .success();

    // Delete pair 1 (confirm 'y')
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "del",
            "--pair",
            "1",
            "2025-10-02",
        ])
        .write_stdin("y\n")
        .assert()
        .success()
        .stdout(contains("Deleted").or(contains("deleted")));

    // List events and ensure pair1 times are gone, pair2 remains
    rti()
        .args(["--db", &db_path, "--test", "list", "--events"])
        .assert()
        .success()
        .stdout(contains("13:00"))
        .stdout(contains("17:00"))
        .stdout(contains("09:00").not())
        .stdout(contains("12:00").not());
}

#[test]
fn test_delete_nonexistent_pair() {
    let db_path = setup_test_db("delete_nonexistent_pair");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add a single pair (so pair id 1 exists)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-03",
            "O",
            "09:00",
            "30",
            "12:00",
        ])
        .assert()
        .success();

    // Try to delete a non-existent pair 5 on that date
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "del",
            "--pair",
            "5",
            "2025-10-03",
        ])
        .assert()
        .success()
        .stdout(contains("Pair 5 not found for date 2025-10-03"));
}

#[test]
fn test_delete_pair_updates_work_session() {
    let db_path = setup_test_db("delete_pair_updates_ws");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Pair 1: R 08:35 - 17:00 with 30 lunch
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "R",
            "08:35",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // Pair 2: C 17:45 - 20:00
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-02",
            "C",
            "17:45",
            "0",
            "20:00",
        ])
        .assert()
        .success();

    // Delete pair 2
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "del",
            "--pair",
            "2",
            "2025-10-02",
        ])
        .write_stdin("y\n")
        .assert()
        .success();

    // Open DB and assert work_sessions updated
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let mut stmt = conn
        .prepare("SELECT position, end_time FROM work_sessions WHERE date = ?1")
        .expect("prepare");
    let mut rows = stmt.query(["2025-10-02"]).expect("query");
    let row = rows.next().expect("next").expect("row");
    let position: String = row.get(0).expect("get pos");
    let end_time: String = row.get(1).expect("get end");

    // After deleting pair 2, only pair 1 remains -> position should be 'R' and end_time '17:00'
    assert_eq!(position, "R");
    assert_eq!(end_time, "17:00");
}

#[test]
fn test_delete_pair_with_mixed_positions_leaves_position_unchanged() {
    let db_path = setup_test_db("delete_pair_mixed_positions");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Pair 1: R 08:00 - 09:00
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-05",
            "R",
            "08:00",
            "0",
            "09:00",
        ])
        .assert()
        .success();

    // Pair 2: O 10:00 - 11:00
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-05",
            "O",
            "10:00",
            "0",
            "11:00",
        ])
        .assert()
        .success();

    // Pair 3: C 12:00 - 13:00
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "add",
            "2025-10-05",
            "C",
            "12:00",
            "0",
            "13:00",
        ])
        .assert()
        .success();

    // Confirm pre-delete position is Mixed (M)
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT position, start_time, end_time, lunch_break FROM work_sessions WHERE date = ?1",
        )
        .expect("prepare");
    let mut rows = stmt.query(["2025-10-05"]).expect("query");
    let row = rows.next().expect("next").expect("row");
    let position_before: String = row.get(0).expect("get pos");
    let start_before: String = row.get(1).expect("get start");
    let end_before: String = row.get(2).expect("get end");

    assert_eq!(position_before, "M");
    assert_eq!(start_before, "08:00");
    assert_eq!(end_before, "13:00");

    // Delete pair 2 (the middle one with position O) -> remaining positions are R and C (mixed)
    rti()
        .args([
            "--db",
            &db_path,
            "--test",
            "del",
            "--pair",
            "2",
            "2025-10-05",
        ])
        .write_stdin("y\n")
        .assert()
        .success();

    // Re-open DB and check work_sessions values
    let conn = rusqlite::Connection::open(&db_path).expect("open db");
    let mut stmt = conn
        .prepare(
            "SELECT position, start_time, end_time, lunch_break FROM work_sessions WHERE date = ?1",
        )
        .expect("prepare");
    let mut rows = stmt.query(["2025-10-05"]).expect("query");
    let row = rows.next().expect("next").expect("row");
    let position_after: String = row.get(0).expect("get pos");
    let start_after: String = row.get(1).expect("get start");
    let end_after: String = row.get(2).expect("get end");
    let lunch_after: i32 = row.get(3).expect("get lunch");

    // Position should remain unchanged (still 'M') because remaining are mixed
    assert_eq!(position_after, position_before);
    // start should be min among remaining events (08:00), end max (13:00) but since we removed pair2, max remains 13:00
    assert_eq!(start_after, "08:00");
    assert_eq!(end_after, "13:00");
    // lunch_break should be taken from latest remaining out (13:00) which was added with 0
    assert_eq!(lunch_after, 0);
}
