// src/export/range.rs

use crate::errors::{AppError, AppResult};
use chrono::NaiveDate;

/// Parse --range (year / month / day / intervallo).
///
/// Supporta:
/// - YYYY
/// - YYYY-MM
/// - YYYY-MM-DD
/// - YYYY:YYYY
/// - YYYY-MM:YYYY-MM
/// - YYYY-MM-DD:YYYY-MM-DD
pub(crate) fn parse_range(r: &str) -> AppResult<(NaiveDate, NaiveDate)> {
    if let Some((start_raw, end_raw)) = r.split_once(':') {
        let start = start_raw.trim();
        let end = end_raw.trim();

        if start.len() != end.len() {
            return Err(AppError::from(std::io::Error::other(
                "start and end must have same format",
            )));
        }

        match start.len() {
            // YYYY:YYYY
            4 => {
                let ys: i32 = start
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid start year")))?;
                let ye: i32 = end
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid end year")))?;

                let d1 = NaiveDate::from_ymd_opt(ys, 1, 1)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid start date")))?;
                let d2 = NaiveDate::from_ymd_opt(ye, 12, 31)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid end date")))?;
                Ok((d1, d2))
            }
            // YYYY-MM:YYYY-MM
            7 => {
                let ys: i32 = start[0..4]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid start year")))?;
                let ms: u32 = start[5..7]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid start month")))?;
                let ye: i32 = end[0..4]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid end year")))?;
                let me: u32 = end[5..7]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid end month")))?;

                let last = month_last_day(ye, me)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid end month")))?;

                let d1 = NaiveDate::from_ymd_opt(ys, ms, 1)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid start date")))?;
                let d2 = NaiveDate::from_ymd_opt(ye, me, last)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid end date")))?;
                Ok((d1, d2))
            }
            // YYYY-MM-DD:YYYY-MM-DD
            10 => {
                let d1 = NaiveDate::parse_from_str(start, "%Y-%m-%d")
                    .map_err(|_| AppError::from(std::io::Error::other("invalid start date")))?;
                let d2 = NaiveDate::parse_from_str(end, "%Y-%m-%d")
                    .map_err(|_| AppError::from(std::io::Error::other("invalid end date")))?;
                Ok((d1, d2))
            }
            _ => Err(AppError::from(std::io::Error::other(
                "unsupported range format",
            ))),
        }
    } else {
        match r.len() {
            // YYYY
            4 => {
                let y: i32 = r
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid year")))?;
                let d1 = NaiveDate::from_ymd_opt(y, 1, 1)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid start date")))?;
                let d2 = NaiveDate::from_ymd_opt(y, 12, 31)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid end date")))?;
                Ok((d1, d2))
            }
            // YYYY-MM
            7 => {
                let y: i32 = r[0..4]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid year")))?;
                let m: u32 = r[5..7]
                    .parse()
                    .map_err(|_| AppError::from(std::io::Error::other("invalid month")))?;
                let last = month_last_day(y, m)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid month")))?;

                let d1 = NaiveDate::from_ymd_opt(y, m, 1)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid start date")))?;
                let d2 = NaiveDate::from_ymd_opt(y, m, last)
                    .ok_or_else(|| AppError::from(std::io::Error::other("invalid end date")))?;
                Ok((d1, d2))
            }
            // YYYY-MM-DD
            10 => {
                let d = NaiveDate::parse_from_str(r, "%Y-%m-%d")
                    .map_err(|_| AppError::from(std::io::Error::other("invalid date")))?;
                Ok((d, d))
            }
            _ => Err(AppError::from(std::io::Error::other(
                "unsupported --range format",
            ))),
        }
    }
}

fn month_last_day(y: i32, m: u32) -> Option<u32> {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => {
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            Some(if leap { 29 } else { 28 })
        }
        _ => None,
    }
}
