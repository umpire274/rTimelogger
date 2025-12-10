use crate::config::Config;
use crate::core::calculator::timeline::Timeline;
use crate::core::logic::Core;
use crate::utils::time::parse_lunch_window;

/// Expected = work_minutes + effective_lunch (automatic or explicit)
pub fn calculate_expected(timeline: &Timeline, cfg: &Config) -> i64 {
    if timeline.pairs.is_empty() {
        return 0;
    }

    // Total minutes the user *must work*
    let work_minutes = Core::parse_work_duration_to_minutes(&cfg.min_work_duration);

    // Take lunch from the first IN of the day
    let first_pair = &timeline.pairs[0];
    let mut lunch = first_pair.lunch_minutes;

    // ---- Auto-lunch logic using lunch_window ----
    // If no lunch was specified, infer it from lunch_window based on the IN time.
    if lunch == 0
        && let Some((_win_start, win_end)) = parse_lunch_window(&cfg.lunch_window)
    {
        let start_time = first_pair.in_event.timestamp().time();

        // If IN time is before the lunch window ends → apply min lunch
        if start_time <= win_end {
            lunch = cfg.min_duration_lunch_break as i64;
        }
    }

    work_minutes + lunch
}
