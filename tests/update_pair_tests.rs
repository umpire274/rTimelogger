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
fn test_update_does_not_create_new_pair() {
    let db_path = setup_test_db("update_pair");

    // Init DB
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // Add initial full session (IN + OUT)
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-20",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    // Capture initial list output
    let initial_output = rti()
        .args(["--db", &db_path, "--test", "list", "--events"])
        .output()
        .expect("failed to list events (initial)");
    assert!(initial_output.status.success());

    let stdout = String::from_utf8_lossy(&initial_output.stdout);
    let initial_lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    assert_eq!(
        initial_lines.len(),
        2,
        "Expected exactly 2 events after first add"
    );
    assert!(
        initial_lines.iter().any(|l| l.contains("in")),
        "Missing initial IN event"
    );
    assert!(
        initial_lines.iter().any(|l| l.contains("out")),
        "Missing initial OUT event"
    );

    // Update ONLY start time via explicit edit (pair 1)
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-20",
            "--edit",
            "--pair",
            "1",
            "--in",
            "09:15",
        ])
        .assert()
        .success();

    // Update ONLY end time via explicit edit (pair 1)
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-20",
            "--edit",
            "--pair",
            "1",
            "--out",
            "17:05",
        ])
        .assert()
        .success();

    // Update ONLY lunch via explicit edit (pair 1)
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-09-20",
            "--edit",
            "--pair",
            "1",
            "--lunch",
            "45",
        ])
        .assert()
        .success();

    // Re-capture events after updates
    let final_output = rti()
        .args(["--db", &db_path, "--test", "list", "--events"])
        .output()
        .expect("failed to list events (final)");
    assert!(final_output.status.success());

    let stdout2 = String::from_utf8_lossy(&final_output.stdout);
    let final_lines: Vec<&str> = stdout2
        .lines()
        .filter(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    assert_eq!(
        final_lines.len(),
        2,
        "Editing fields must NOT create extra events/pairs"
    );
    assert!(
        final_lines
            .iter()
            .any(|l| l.contains("in") && l.contains("09:15")),
        "Start time not updated in place"
    );
    assert!(
        final_lines
            .iter()
            .any(|l| l.contains("out") && l.contains("17:05")),
        "End time not updated in place"
    );
}
