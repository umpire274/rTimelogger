use crate::db::pool::DbPool;
use crate::utils::colors::{CYAN, GREEN, GREY, RESET, YELLOW};
use chrono::NaiveDate;
use rusqlite::OptionalExtension;
use std::fs;

pub fn print_db_info(pool: &mut DbPool, db_path: &str) -> rusqlite::Result<()> {
    println!();

    //
    // 1) FILE SIZE
    //
    let file_size = fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
    let file_mb = (file_size as f64) / (1024.0 * 1024.0);

    println!("{}• File:{} {}{}{}", CYAN, RESET, YELLOW, db_path, RESET);
    println!("{}• Size:{} {:.2} MB", CYAN, RESET, file_mb);

    //
    // 2) TOTAL EVENTS
    //
    let count: i64 = pool
        .conn
        .query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
    println!(
        "{}• Total events:{} {}{}{}",
        CYAN, RESET, GREEN, count, RESET
    );

    //
    // 3) DATE RANGE
    //
    let first_date: Option<String> = pool
        .conn
        .query_row(
            "SELECT date FROM events ORDER BY date ASC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    let last_date: Option<String> = pool
        .conn
        .query_row(
            "SELECT date FROM events ORDER BY date DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    let fmt_first = first_date
        .clone()
        .unwrap_or_else(|| format!("{GREY}--{RESET}"));
    let fmt_last = last_date
        .clone()
        .unwrap_or_else(|| format!("{GREY}--{RESET}"));

    println!("{}• Date range:{}", CYAN, RESET);
    println!("    from: {}", fmt_first);
    println!("    to:   {}", fmt_last);

    //
    // 4) AVERAGE EVENTS/DAY
    //
    if let (Some(f), Some(l)) = (first_date, last_date) {
        let d1 = parse_date(&f)?;
        let d2 = parse_date(&l)?;
        let days = (d2 - d1).num_days().max(1);

        let avg = count as f64 / days as f64;
        println!("{}• Average events/day:{} {:.2}", CYAN, RESET, avg);
    }

    println!();
    Ok(())
}

fn parse_date(date_str: &str) -> rusqlite::Result<NaiveDate> {
    NaiveDate::parse_from_str(date_str, "%Y-%m-%d").map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
