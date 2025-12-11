// src/export/excel_date.rs

use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};

/// Prova a interpretare una stringa come data/ora o solo data/ora,
/// restituendo il *seriale Excel* + formattazione numerica.
pub(crate) fn parse_to_excel_date(s: &str) -> Option<(&'static str, f64)> {
    let dt_formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];

    for fmt in dt_formats.iter() {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            let serial = naive_datetime_to_excel_serial(&dt);
            return Some(("yyyy-mm-dd hh:mm", serial));
        }
    }

    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0).unwrap();
        let serial = naive_datetime_to_excel_serial(&dt);
        return Some(("yyyy-mm-dd", serial));
    }

    let time_formats = ["%H:%M:%S", "%H:%M"];

    for fmt in time_formats.iter() {
        if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
            let seconds = t.num_seconds_from_midnight() as f64;
            return Some(("hh:mm", seconds / 86400.0));
        }
    }

    None
}

fn naive_datetime_to_excel_serial(dt: &NaiveDateTime) -> f64 {
    let excel_epoch = NaiveDate::from_ymd_opt(1899, 12, 30)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let duration = *dt - excel_epoch;

    let days = duration.num_days() as f64;
    let secs = (duration.num_seconds() - duration.num_days() * 86400) as f64;

    days + secs / 86400.0
}
