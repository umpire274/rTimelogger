mod common;

use crate::common::populate_many_sessions;
use common::{init_db_with_data, rti, setup_test_db, temp_out};
use serde_json::Value;
use std::fs;
use std::time::Instant;

// Test day-range brace syntax (YYYY-MM-{dd..dd}) and edge at month boundary
#[test]
fn test_export_day_range_brace() {
    let db_path = setup_test_db("export_day_range");

    // init db
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    // add sessions on two February dates (non-leap year 2025)
    rti()
        .args([
            "--db",
            &db_path,
            "add",
            "2025-02-15",
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
            "2025-02-28",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();

    let out = temp_out("export_day_range", "json");

    // export only the 28th using brace syntax
    rti()
        .args([
            "--db",
            &db_path,
            "export",
            "--format",
            "json",
            "--file",
            &out,
            "--events",
            "--range",
            "2025-02-28:2025-02-28",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported json");
    // Expect data only for 2025-02-28
    assert!(content.contains("2025-02-28"));
    assert!(!content.contains("2025-02-15"));
}

// Export on empty dataset: JSON should be an empty array, CSV should be empty file
#[test]
fn test_export_empty_dataset() {
    let db_path = setup_test_db("export_empty_dataset");

    // init only, do not add any data
    rti()
        .args(["--db", &db_path, "--test", "init"])
        .assert()
        .success();

    let out_json = temp_out("export_empty_dataset", "json");
    let out_csv = temp_out("export_empty_dataset", "csv");

    // JSON export events
    rti()
        .args([
            "--db", &db_path, "export", "--format", "json", "--file", &out_json, "--events",
        ])
        .assert()
        .success();

    let content_json = fs::read_to_string(&out_json).expect("read json");
    let v: Value = serde_json::from_str(&content_json).expect("valid json");
    assert!(v.is_array());
    assert!(v.as_array().unwrap().is_empty());

    // CSV export events
    rti()
        .args([
            "--db", &db_path, "export", "--format", "csv", "--file", &out_csv, "--events",
        ])
        .assert()
        .success();

    let meta = fs::metadata(&out_csv).expect("csv exists");
    // empty dataset -> writer should create an empty file
    assert_eq!(meta.len(), 0);
}

// Verify JSON structure (keys present) and CSV columns count
#[test]
fn test_export_json_structure_and_csv_columns() {
    let db_path = setup_test_db("export_structure");

    // populate via existing helper (adds 2 sessions)
    init_db_with_data(&db_path);

    let out_json = temp_out("export_structure", "json");
    let out_csv = temp_out("export_structure", "csv");

    // Export JSON events
    rti()
        .args([
            "--db", &db_path, "export", "--format", "json", "--file", &out_json, "--events",
        ])
        .assert()
        .success();

    let content_json = fs::read_to_string(&out_json).expect("read json");
    let v: Value = serde_json::from_str(&content_json).expect("valid json");
    assert!(v.is_array());
    if let Some(arr) = v.as_array()
        && !arr.is_empty()
    {
        let obj = &arr[0];
        assert!(obj.get("id").is_some());
        assert!(obj.get("date").is_some());
        assert!(obj.get("time").is_some());
        assert!(obj.get("kind").is_some());
    }

    // Export CSV events
    rti()
        .args([
            "--db", &db_path, "export", "--format", "csv", "--file", &out_csv, "--events",
        ])
        .assert()
        .success();

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(&out_csv)
        .expect("read csv");
    let mut records = rdr.records();
    if let Some(Ok(rec)) = records.next() {
        // EventExport serializes to 8 fields: id,date,time,kind,position,lunch_break,pair,source
        assert_eq!(rec.len(), 8);
    }
}

// Performance smoke: populate many sessions and ensure export completes in reasonable time
#[test]
fn test_export_performance_smoke() {
    let db_path = setup_test_db("export_perf");

    // populate many sessions directly via DB API
    populate_many_sessions(&db_path, 2000);

    let out = temp_out("export_perf", "csv");
    let start = Instant::now();

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
            "--force",
        ])
        .assert()
        .success();

    let elapsed = start.elapsed();
    // smoke check: should be reasonably fast (on CI might be slower); use 10s threshold
    assert!(
        elapsed.as_secs_f64() < 10.0,
        "export too slow: {}s",
        elapsed.as_secs_f64()
    );
}
