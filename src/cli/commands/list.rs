use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::load_events_by_date;
use crate::errors::AppResult;
use crate::utils::date;
use chrono::NaiveDate;

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::List {
        period,
        pos: _,
        now: l_now,
        details,
        events: events_only,
        pairs: _,
        summary: _,
        ..
    } = cmd
    {
        let mut pool = DbPool::new(&cfg.database)?;

        let dates = if *l_now {
            vec![date::today()]
        } else {
            resolve_period(period)?
        };

        for d in dates {
            let d_str = d.to_string();
            let d = NaiveDate::parse_from_str(&d_str, "%Y-%m-%d")
                .map_err(|_| crate::errors::AppError::InvalidDate(d_str.clone()))?;
            let events = load_events_by_date(&mut pool, &d)?;

            if events.is_empty() {
                println!("No events for {}", d);
                continue;
            }

            let summary_model = Core::build_daily_summary(&events);

            if *events_only {
                print_events(&events);
            } else if summary_model.timeline.pairs.is_empty() {
                println!("No valid pairs for {}.", d);
            } else {
                let pair_count = summary_model.timeline.pairs.len();
                print_summary(&d, &summary_model, *details, pair_count);
            }
        }
    }
    Ok(())
}

fn resolve_period(period: &Option<String>) -> AppResult<Vec<NaiveDate>> {
    use crate::errors::AppError;

    if let Some(p) = period {
        if p == "all" {
            return date::generate_all_dates().map_err(AppError::InvalidDate);
        }

        if p.contains(':') {
            let parts: Vec<&str> = p.split(':').collect();
            if parts.len() == 2 {
                return date::generate_range(parts[0], parts[1]).map_err(AppError::InvalidDate);
            }
        }

        return date::generate_from_period(p).map_err(AppError::InvalidDate);
    }

    date::current_month_dates().map_err(AppError::InvalidDate)
}

fn print_events(events: &[crate::models::event::Event]) {
    println!("EVENTS:");
    for ev in events {
        println!(
            "- {} | {} | lunch={} | loc={:?}",
            ev.timestamp(),
            ev.kind.et_as_str(),
            ev.lunch.unwrap_or(0),
            ev.location,
        );
    }
}

fn print_summary(
    date: &NaiveDate,
    summary: &crate::models::day_summary::DaySummary,
    details: bool,
    pair_count: usize,
) {
    println!("\n=== {} ===", date);
    println!("Pairs: {}", pair_count);
    println!(
        "Worked: {} min | Expected: {} min | Surplus: {} min",
        summary.timeline.total_worked_minutes, summary.expected, summary.surplus
    );

    if details {
        println!("\nDetails:\n{:#?}", summary.timeline.pairs);
    }
}
