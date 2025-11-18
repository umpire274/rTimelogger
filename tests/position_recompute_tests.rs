mod common;
use common::{rti, setup_test_db};

// Ensure that when multiple events remain with the same position, work_sessions position
// is updated to that single position and end_time is the max remaining time.
#[test]
fn test_recompute_sets_single_position_and_end() {
    let db_path = setup_test_db("pos_recompute_single");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Pair 1: R 08:35 - 17:00
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

    assert_eq!(position, "R");
    assert_eq!(end_time, "17:00");
}

// Robustness test: repeat the mixed-positions delete scenario multiple times across separate DB files
// to catch potential flakiness caused by filesystem/test harness interactions.
#[test]
fn test_recompute_robustness_multiple_runs() {
    for i in 0..10 {
        let db_path = setup_test_db(&format!("pos_recompute_robust_{i}"));

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

        // Delete pair 2 (the middle one with position O)
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

        assert_eq!(position_after, position_before);
        assert_eq!(start_after, "08:00");
        assert_eq!(end_after, "13:00");
        assert_eq!(lunch_after, 0);
    }
}
