use crate::cli::parser::StatsGrouping;
use crate::config::Config;
use crate::core::calculator::timeline::build_timeline;
use crate::core::logic::Core;
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::location::Location;
use crate::models::work_stats::WorkStats;
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum GroupKey {
    Week(i32, u32),
    Month(i32, u32),
    Year(i32),
}

pub fn calculate(
    events: &[Event],
    from: NaiveDate,
    to: NaiveDate,
    grouping: StatsGrouping,
    cfg: &Config,
) -> AppResult<Vec<WorkStats>> {
    if from > to {
        return Err(AppError::InvalidDateRange { from, to });
    }

    let contractual_minutes = Core::parse_work_duration_to_minutes(&cfg.min_work_duration);
    let mut events_by_date: BTreeMap<NaiveDate, Vec<Event>> = BTreeMap::new();
    for event in events {
        if event.date >= from && event.date <= to {
            events_by_date
                .entry(event.date)
                .or_default()
                .push(event.clone());
        }
    }

    let mut groups: BTreeMap<GroupKey, WorkStats> = BTreeMap::new();
    let mut day = from;
    while day <= to {
        let key = group_key(day, grouping);
        let stats = groups.entry(key).or_insert_with(|| WorkStats {
            label: group_label(key),
            period_start: day,
            period_end: day,
            expected_minutes: 0,
            worked_minutes: 0,
            paid_leave_minutes: 0,
            sick_leave_minutes: 0,
            national_holiday_minutes: 0,
            worked_days: 0,
            paid_leave_days: 0,
            sick_leave_days: 0,
            national_holiday_days: 0,
        });
        stats.period_end = day;

        if is_contractual_day(day) {
            stats.expected_minutes += contractual_minutes;
        }

        if let Some(day_events) = events_by_date.get(&day) {
            let marker = day_events.iter().find_map(|event| match event.location {
                Location::Holiday | Location::NationalHoliday | Location::SickLeave => {
                    Some(event.location)
                }
                _ => None,
            });

            match marker {
                Some(Location::Holiday) => {
                    stats.paid_leave_days += 1;
                    stats.paid_leave_minutes += contractual_credit(day, contractual_minutes);
                }
                Some(Location::SickLeave) => {
                    stats.sick_leave_days += 1;
                    stats.sick_leave_minutes += contractual_credit(day, contractual_minutes);
                }
                Some(Location::NationalHoliday) => {
                    stats.national_holiday_days += 1;
                    stats.national_holiday_minutes += contractual_credit(day, contractual_minutes);
                }
                _ => {
                    let timeline = build_timeline(day_events);
                    if !timeline.pairs.is_empty() {
                        stats.worked_days += 1;
                        stats.worked_minutes += timeline.total_worked_minutes
                            + timeline
                                .gaps
                                .iter()
                                .filter(|gap| gap.is_work_gap)
                                .map(|gap| gap.duration_minutes)
                                .sum::<i64>();
                    }
                }
            }
        }

        day = day
            .checked_add_signed(Duration::days(1))
            .ok_or_else(|| AppError::Other("Date range exceeds supported values.".into()))?;
    }

    Ok(groups.into_values().collect())
}

fn contractual_credit(day: NaiveDate, contractual_minutes: i64) -> i64 {
    if is_contractual_day(day) {
        contractual_minutes
    } else {
        0
    }
}

fn is_contractual_day(day: NaiveDate) -> bool {
    !matches!(day.weekday(), Weekday::Sat | Weekday::Sun)
}

fn group_key(day: NaiveDate, grouping: StatsGrouping) -> GroupKey {
    match grouping {
        StatsGrouping::Week => {
            let iso = day.iso_week();
            GroupKey::Week(iso.year(), iso.week())
        }
        StatsGrouping::Month => GroupKey::Month(day.year(), day.month()),
        StatsGrouping::Year => GroupKey::Year(day.year()),
    }
}

fn group_label(key: GroupKey) -> String {
    match key {
        GroupKey::Week(year, week) => format!("{year}-W{week:02}"),
        GroupKey::Month(year, month) => format!("{year}-{month:02}"),
        GroupKey::Year(year) => year.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::event::{Event, EventExtras};
    use crate::models::event_type::EventType;
    use chrono::NaiveTime;

    fn event(date: NaiveDate, time: &str, kind: EventType, location: Location) -> Event {
        Event::new(
            0,
            date,
            NaiveTime::parse_from_str(time, "%H:%M").unwrap(),
            kind,
            location,
            EventExtras::default(),
        )
    }

    #[test]
    fn protected_absences_fulfil_contractual_time() {
        let from = NaiveDate::from_ymd_opt(2026, 8, 3).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 8, 5).unwrap();
        let events = vec![
            event(from, "09:00", EventType::In, Location::Holiday),
            event(
                from.succ_opt().unwrap(),
                "00:00",
                EventType::In,
                Location::SickLeave,
            ),
            event(
                from.checked_add_signed(Duration::days(2)).unwrap(),
                "00:00",
                EventType::In,
                Location::NationalHoliday,
            ),
        ];

        let stats = calculate(&events, from, to, StatsGrouping::Week, &Config::default()).unwrap();
        assert_eq!(stats.len(), 1);
        assert_eq!(stats[0].expected_minutes, 3 * 480);
        assert_eq!(stats[0].recognized_minutes(), 3 * 480);
        assert_eq!(stats[0].balance_minutes(), 0);
        assert_eq!(
            (
                stats[0].paid_leave_days,
                stats[0].sick_leave_days,
                stats[0].national_holiday_days
            ),
            (1, 1, 1)
        );
    }

    #[test]
    fn work_gaps_count_as_worked_time() {
        let day = NaiveDate::from_ymd_opt(2026, 8, 3).unwrap();
        let mut out = event(day, "12:00", EventType::Out, Location::Office);
        out.work_gap = true;
        let events = vec![
            event(day, "09:00", EventType::In, Location::Office),
            out,
            event(day, "13:00", EventType::In, Location::Office),
            event(day, "18:00", EventType::Out, Location::Office),
        ];

        let stats = calculate(&events, day, day, StatsGrouping::Month, &Config::default()).unwrap();
        assert_eq!(stats[0].worked_minutes, 9 * 60);
    }

    #[test]
    fn iso_week_groups_across_calendar_year() {
        let from = NaiveDate::from_ymd_opt(2026, 12, 28).unwrap();
        let to = NaiveDate::from_ymd_opt(2027, 1, 4).unwrap();
        let stats = calculate(&[], from, to, StatsGrouping::Week, &Config::default()).unwrap();
        assert_eq!(
            stats
                .iter()
                .map(|stats| stats.label.as_str())
                .collect::<Vec<_>>(),
            vec!["2026-W53", "2027-W01"]
        );
    }
}
