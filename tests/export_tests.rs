mod common;
use common::{init_db_with_data, rti, setup_test_db, temp_out};
use std::fs;

#[test]
fn test_export_events_csv_all() {
    let db_path = setup_test_db("export_events_csv_all");
    init_db_with_data(&db_path);

    let out = temp_out("export_events_csv_all", "csv");

    rti()
        .args([
            "--db", &db_path, "export", "--format", "csv", "--file", &out, "--events",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported csv");
    assert!(content.contains("2025-09-01"));
    assert!(content.contains("2025-09-15"));
}

#[test]
fn test_export_events_json_range() {
    let db_path = setup_test_db("export_events_json_range");
    init_db_with_data(&db_path);

    let out = temp_out("export_events_json_range", "json");

    rti()
        .args([
            "--db", &db_path, "export", "--format", "json", "--file", &out, "--events", "--range",
            "2025-09",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported json");
    assert!(content.contains("2025-09-01"));
    assert!(content.contains("2025-09-15"));
}

#[test]
fn test_export_sessions_csv_range() {
    let db_path = setup_test_db("export_sessions_csv_range");
    init_db_with_data(&db_path);

    let out = temp_out("export_sessions_csv_range", "csv");

    rti()
        .args([
            "--db",
            &db_path,
            "export",
            "--format",
            "csv",
            "--file",
            &out,
            "--sessions",
            "--range",
            "2025-09",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported csv");
    assert!(content.contains("2025-09-01") || content.contains("2025-09-15"));
}

#[test]
fn test_export_sessions_json_all() {
    let db_path = setup_test_db("export_sessions_json_all");
    init_db_with_data(&db_path);

    let out = temp_out("export_sessions_json_all", "json");

    rti()
        .args([
            "--db",
            &db_path,
            "export",
            "--format",
            "json",
            "--file",
            &out,
            "--sessions",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported json");
    assert!(content.contains("2025-09-01"));
    assert!(content.contains("2025-09-15"));
}
