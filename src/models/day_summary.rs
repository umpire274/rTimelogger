use crate::core::calculator::gaps::GapInfo;
use crate::core::calculator::timeline::Timeline;

#[derive(Debug, Default)]
pub struct DaySummary {
    pub timeline: Timeline,
    pub gaps: GapInfo,
    pub expected: i64,
    pub surplus: i64,
}
