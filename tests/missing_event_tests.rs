use std::env;
use std::path::PathBuf;

mod common;
use common::rti;

/// Create a unique test DB path inside the system temp dir
fn setup_test_db(name: &str) -> String {
    let mut path: PathBuf = env::temp_dir();
    path.push(format!("{}_rtimelogger.sqlite", name));
    let db_path = path.to_string_lossy().to_string();
    let _ = std::fs::remove_file(&db_path);
    db_path
}

#[test]
fn test_create_missing_in_when_only_out_exists() {
    let db_path = setup_test_db("missing_event_in");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Create only an OUT event for the date (no start)
    rti()
        .args(["--db", &db_path, "add", "2025-10-01", "--out", "17:00"])
        .assert()
        .success();

    // Verify there is a single OUT event
    let out_only = rti()
        .args(["--db", &db_path, "--test", "list", "--events"])
        .output()
        .expect("failed to list events (out-only)");
    assert!(out_only.status.success());

    let stdout = String::from_utf8_lossy(&out_only.stdout);

    // Consider only event rows (those starting with a number)
    let event_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .collect();

    assert_eq!(
        event_lines.len(),
        1,
        "Expected exactly one event line before editing"
    );
    assert!(
        event_lines[0].contains("out"),
        "Expected the only event to be of kind OUT"
    );
    assert!(
        !event_lines[0].contains("in"),
        "Should not contain IN event before editing"
    );

    // Add the missing IN event
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-10-01",
            "--edit",
            "--pair",
            "1",
            "--in",
            "09:00",
        ])
        .assert()
        .success();

    // Re-list events and expect both IN and OUT
    let both = rti()
        .args(["--db", &db_path, "--test", "list", "--events"])
        .output()
        .expect("failed to list events (after creating in)");
    assert!(both.status.success());

    let stdout2 = String::from_utf8_lossy(&both.stdout);

    let event_lines2: Vec<&str> = stdout2
        .lines()
        .filter(|line| line.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .collect();

    assert_eq!(
        event_lines2.len(),
        2,
        "Expected 2 event lines after adding missing in"
    );
    assert!(
        event_lines2.iter().any(|l| l.contains("in")),
        "Expected an IN event after editing"
    );
    assert!(
        event_lines2.iter().any(|l| l.contains("out")),
        "Expected an OUT event after editing"
    );
}
