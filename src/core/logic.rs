use crate::core::calculator::{expected, surplus, timeline};
use crate::models::{day_summary::DaySummary, event::Event};

pub struct Core;

impl Core {
    pub fn build_daily_summary(events: &[Event]) -> DaySummary {
        let timeline = timeline::build_timeline(events);
        let expected = expected::calculate_expected(&timeline);
        let surplus = surplus::calculate_surplus(&timeline, expected);

        DaySummary {
            timeline,
            expected,
            surplus,
            gaps: Default::default(), // sarà implementato per work_gap
        }
    }
}
