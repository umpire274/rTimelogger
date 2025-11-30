use crate::core::calculator::timeline::Timeline;

pub fn calculate_surplus(timeline: &Timeline, expected: i64) -> i64 {
    timeline.total_worked_minutes - expected
}
