//! Module responsible for analyzing gaps between pairs and determining
//! which gaps should be counted as work (work_gap = true).

use crate::core::calculator::timeline::Timeline;

/// Information about daily gaps (normal and work gaps)
#[derive(Debug, Default)]
pub struct GapInfo {
    pub total_gap_minutes: i64,
    pub work_gap_minutes: i64,
    pub non_work_gap_minutes: i64,
}

pub fn analyze_gaps(_timeline: &Timeline) -> GapInfo {
    // TODO: implement analysis of gap timing between pairs
    GapInfo::default()
}
