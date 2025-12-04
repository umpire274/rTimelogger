//! Time utilities: parsing HH:MM, duration computations, formatting minutes, etc.

use crate::errors::{AppError, AppResult};
use chrono::NaiveTime;

pub fn parse_time(t: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(t, "%H:%M").ok()
}

pub fn minutes_between(start: NaiveTime, end: NaiveTime) -> i64 {
    let duration = end - start;
    duration.num_minutes()
}

pub fn format_minutes(mins: i64) -> String {
    let sign = if mins < 0 { "-" } else { "" };
    let m = mins.abs();
    format!("{}{:02}:{:02}", sign, m / 60, m % 60)
}

pub fn parse_optional_time(input: Option<&String>) -> AppResult<Option<NaiveTime>> {
    if let Some(s) = input {
        let t = parse_time(s).ok_or_else(|| AppError::InvalidTime(s.to_string()))?;
        Ok(Some(t))
    } else {
        Ok(None)
    }
}
