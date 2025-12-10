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

pub(crate) fn parse_lunch_window(s: &str) -> Option<(NaiveTime, NaiveTime)> {
    let (start_s, end_s) = s.split_once('-')?;
    let start = NaiveTime::parse_from_str(start_s.trim(), "%H:%M").ok()?;
    let end = NaiveTime::parse_from_str(end_s.trim(), "%H:%M").ok()?;
    Some((start, end))
}

pub fn crosses_lunch_window(
    start: NaiveTime,
    end: NaiveTime,
    win_start: NaiveTime,
    win_end: NaiveTime,
) -> bool {
    // intervallo di lavoro [start, end] interseca [win_start, win_end]
    start < win_end && end > win_start
}

/// Determine if a start time crosses the lunch window.
/// If start ≤ window_end → Expected exit must consider a lunch break.
pub fn start_crosses_lunch_window(start: NaiveTime, win_end: NaiveTime) -> bool {
    start <= win_end
}

pub fn hhmm2minutes(s: &str) -> i64 {
    // Accepts: "8h", "7h 36m", "7h36m", "  6h   15m ", "45m"
    let cleaned = s.trim().to_lowercase();
    let mut hours: i64 = 0;
    let mut minutes: i64 = 0;

    // parsing without regex: number followed by 'h' or 'm'
    let mut num = String::new();
    for ch in cleaned.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
        } else if ch == 'h' {
            if let Ok(h) = num.parse::<i64>() {
                hours = h;
            }
            num.clear();
        } else if ch == 'm' {
            if let Ok(m) = num.parse::<i64>() {
                minutes = m;
            }
            num.clear();
        } else {
            // separator: discard orphan numbers
            if !num.is_empty() {
                num.clear();
            }
        }
    }
    hours * 60 + minutes
}
