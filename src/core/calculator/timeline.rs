use crate::models::event::Event;
use crate::models::event_type::EventType;
use crate::models::location::Location;
use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct Pair {
    pub in_event: Event,
    pub out_event: Option<Event>,
    pub duration_minutes: i64,
    pub lunch_minutes: i64,
    pub position: Location,
}

#[derive(Debug, Clone)]
pub struct Gap {
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub duration_minutes: i64,
    pub is_work_gap: bool, // will be computed in 0.8.0-beta1
}

#[derive(Debug, Default, Clone)]
pub struct Timeline {
    pub events: Vec<Event>,
    pub pairs: Vec<Pair>,
    pub gaps: Vec<Gap>,
    pub total_worked_minutes: i64,
}

pub fn build_timeline(events: &[Event]) -> Timeline {
    if events.is_empty() {
        return Timeline::default();
    }

    // -----------------------------
    // Sort events chronologically
    // -----------------------------
    let mut sorted = events.to_vec();
    sorted.sort_by_key(|e| e.timestamp());

    let mut pairs = Vec::new();
    let mut gaps = Vec::new();
    let mut total = 0;

    let mut i = 0;

    // -----------------------------
    // Build Pairs
    // -----------------------------
    while i < sorted.len() {
        let ev = &sorted[i];

        if ev.kind == EventType::In {
            // Case: IN followed by OUT → valid pair
            if i + 1 < sorted.len() && sorted[i + 1].kind == EventType::Out {
                let in_ev = ev.clone();
                let out_ev = sorted[i + 1].clone();

                let duration = (out_ev.timestamp() - in_ev.timestamp()).num_minutes()
                    - in_ev.lunch.unwrap_or(0) as i64;

                total += duration;

                pairs.push(Pair {
                    in_event: in_ev.clone(),
                    out_event: Some(out_ev.clone()),
                    duration_minutes: duration,
                    lunch_minutes: in_ev.lunch.unwrap_or(0) as i64,
                    position: in_ev.location,
                });

                i += 2;
                continue;
            }

            // Case: IN without OUT → open pair
            pairs.push(Pair {
                in_event: ev.clone(),
                out_event: None,
                duration_minutes: 0,
                lunch_minutes: ev.lunch.unwrap_or(0) as i64,
                position: ev.location,
            });
        }

        i += 1;
    }

    // -----------------------------
    // Compute GAPS between pairs
    // -----------------------------
    for w in pairs.windows(2) {
        let p1 = &w[0];
        let p2 = &w[1];

        if let (Some(out1), _) = (&p1.out_event, &p2.in_event) {
            let start = out1.timestamp();
            let end = p2.in_event.timestamp();

            if end > start {
                gaps.push(Gap {
                    start,
                    end,
                    duration_minutes: (end - start).num_minutes(),
                    is_work_gap: false, // in alpha everything is NON working
                });
            }
        }
    }

    Timeline {
        events: sorted,
        pairs,
        gaps,
        total_worked_minutes: total,
    }
}
