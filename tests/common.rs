#![allow(dead_code)]
use assert_cmd::{Command, cargo_bin_cmd};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn rti() -> Command {
    cargo_bin_cmd!("rtimelogger")
}

/// Create a unique test DB path inside the system temp dir and remove any existing file
pub fn setup_test_db(name: &str) -> String {
    let mut path: PathBuf = env::temp_dir();
    path.push(format!("{}_rtimelogger.sqlite", name));
    let db_path = path.to_string_lossy().to_string();
    fs::remove_file(&db_path).ok();
    db_path
}

/// Create a temporary output file path inside tempdir and ensure it's removed
pub fn temp_out(name: &str, ext: &str) -> String {
    let mut path: PathBuf = env::temp_dir();
    path.push(format!("{}_out.{}", name, ext));
    let p = path.to_string_lossy().to_string();
    fs::remove_file(&p).ok();
    p
}

/// Initialize DB and add a small dataset useful for many tests
pub fn init_db_with_data(db_path: &str) {
    // init DB (creates tables)
    rti()
        .args(["--db", db_path, "--test", "init"]) // uses --test init to create schema
        .assert()
        .success();

    // add a couple of sessions via CLI (which will also populate events)
    rti()
        .args([
            "--db",
            db_path,
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
            db_path,
            "add",
            "2025-09-15",
            "O",
            "09:00",
            "30",
            "17:00",
        ])
        .assert()
        .success();
}

/// Helper to populate many sessions directly via the library DB API for performance tests
pub fn populate_many_sessions(db_path: &str, n: usize) {
    let conn = rusqlite::Connection::open(db_path).expect("open db");
    // ensure initialized
    rtimelogger::db::initialize::init_db(&conn).expect("init db");
    for i in 0..n {
        // generate dates in a range
        let day = (i % 28) + 1; // 1..28
        let date = format!("2025-11-{day:02}");
        rtimelogger::db::add_session(&conn, &date, "O", "09:00", 30, "17:00").expect("add session");
    }
}
