use crate::core::calculator::timeline::Timeline;

pub fn calculate_expected(timeline: &Timeline) -> i64 {
    if timeline.pairs.is_empty() {
        return 0;
    }

    let first_in = &timeline.pairs[0].in_event.timestamp();
    let last_out = timeline
        .pairs
        .iter()
        .filter_map(|p| p.out_event.as_ref())
        .next_back()
        .map(|ev| ev.timestamp());

    if let Some(end) = last_out {
        let total = (end - *first_in).num_minutes();

        // Lunch: sum of all lunches
        let lunch_total: i64 = timeline
            .pairs
            .iter()
            .map(|p| p.in_event.lunch.unwrap_or(0) as i64)
            .sum();

        total - lunch_total
    } else {
        0
    }
}
