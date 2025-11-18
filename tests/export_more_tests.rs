mod common;
use common::{init_db_with_data, rti, setup_test_db, temp_out};
use predicates::str::contains;
use std::fs;

#[test]
fn test_export_invalid_format_fails() {
    let db_path = setup_test_db("export_invalid_format");
    init_db_with_data(&db_path);

    let out = temp_out("export_invalid_format", "csv");

    rti()
        .args([
            "--db", &db_path, "export", "--format", "xml", "--file", &out, "--events",
        ])
        .assert()
        .failure()
        .stderr(contains("Unsupported format"));
}

#[test]
fn test_export_non_absolute_path_fails() {
    let db_path = setup_test_db("export_non_abs");
    init_db_with_data(&db_path);

    // relative path
    let out = "relative_out.csv";

    rti()
        .args([
            "--db", &db_path, "export", "--format", "csv", "--file", out, "--events",
        ])
        .assert()
        .failure()
        .stderr(contains("Output file path must be absolute"));
}

#[test]
fn test_export_force_overwrite() {
    let db_path = setup_test_db("export_force_overwrite");
    init_db_with_data(&db_path);

    let out = temp_out("export_force_overwrite", "csv");

    // create preexisting file with known content
    fs::write(&out, "OLD_CONTENT").expect("create file");

    rti()
        .args([
            "--db", &db_path, "export", "--format", "csv", "--file", &out, "--events", "--force",
        ])
        .assert()
        .success();

    let content = fs::read_to_string(&out).expect("read exported csv");
    // The file must have been overwritten: it should not equal the original placeholder,
    // and should be non-empty (CSV writer created actual output).
    assert_ne!(content, "OLD_CONTENT");
    assert!(!content.is_empty());
}

#[test]
fn test_export_cancel_overwrite_keeps_file() {
    let db_path = setup_test_db("export_cancel_overwrite");
    init_db_with_data(&db_path);

    let out = temp_out("export_cancel_overwrite", "json");

    // create preexisting file with known content
    fs::write(&out, "ORIGINAL").expect("create file");

    let assert = rti()
        .args([
            "--db", &db_path, "export", "--format", "json", "--file", &out, "--events",
        ])
        .write_stdin("n\n")
        .assert();

    // The CLI will print an error about cancelled export
    assert.stderr(contains("Export cancelled"));

    // The file must be unchanged
    let content = fs::read_to_string(&out).expect("read existing file");
    assert_eq!(content, "ORIGINAL");
}
