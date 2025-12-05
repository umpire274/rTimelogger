use crate::core::calculator::timeline::Timeline;
use chrono::NaiveTime;

pub fn calculate_surplus(timeline: &Timeline, expected: i64) -> i64 {
    timeline.total_worked_minutes - expected
}

/// Calcola il surplus giornaliero come differenza in minuti:
/// surplus = end - expected (in minuti).
pub(crate) fn daily_surplus_from_times(end_str: &str, expected_str: &str) -> Option<i64> {
    if end_str == "--:--" {
        return None;
    }

    let end_t = NaiveTime::parse_from_str(end_str, "%H:%M").ok()?;
    let expected_t = NaiveTime::parse_from_str(expected_str, "%H:%M").ok()?;

    let diff = end_t - expected_t;
    Some(diff.num_minutes())
}
