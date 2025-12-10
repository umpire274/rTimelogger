use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::load_events_by_date;
use crate::errors::{AppError, AppResult};
use crate::models::day_summary::DaySummary;
use crate::models::event::Event;
use crate::utils::date::get_day_position;
use crate::utils::{colors, date, formatting, mins2readable};
use chrono::{Datelike, NaiveDate};

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::List {
        period,
        now,
        details,
        events: events_only,
        ..
    } = cmd
    {
        let mut pool = DbPool::new(&cfg.database)?;

        // 1️⃣ Determina le date
        let dates = if *now {
            vec![date::today()]
        } else {
            resolve_period(period)?
        };

        if dates.is_empty() {
            println!("⚠️  No recorded sessions found");
            return Ok(());
        }

        // 2️⃣ Stampa intestazione se non in modalità --now
        if !*now {
            if period.is_some() {
                print_header(period);
            } else {
                print_header(&Some("this_month".to_string()));
            }
        }

        let mut total_surplus: i64 = 0;
        let mut any_output = false;
        let mut last_month: Option<(i32, u32)> = None;

        if *events_only && Event::has_events_for_dates(&mut pool, &dates)? {
            println!("EVENTS:");
            println!();
            println!(
                " {:^17} | {:^4} | {:^12} | {:^16} | {:^6} | {:^4}",
                "Date Time", "Type", "Lunch", "Position", "Source", "Pair"
            );
            println!("{:-<76}", "-");
        }

        for d in dates {
            let d_str = d.to_string();
            let parsed_date = NaiveDate::parse_from_str(&d_str, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidDate(d_str))?;

            // 🧨 Month separator
            let current_month = (parsed_date.year(), parsed_date.month());
            if let Some((ly, lm)) = last_month
                && (ly, lm) != current_month
            {
                println!("{:>108}", "--------------------------");
            }
            last_month = Some(current_month);

            // 3️⃣ Load events for date
            let events = load_events_by_date(&mut pool, &parsed_date)?;
            if events.is_empty() {
                continue;
            }

            if *events_only {
                print_raw_events(&events);
                continue;
            }

            // 4️⃣ Build summary
            let day_summary = Core::build_daily_summary(&events, cfg);

            if day_summary.timeline.pairs.is_empty() {
                println!("No valid pairs for {}.", parsed_date);
                continue;
            }

            // 5️⃣ Print daily row using timeline
            if let Some(day_surplus) = print_daily_row(&parsed_date, &events, &day_summary, cfg) {
                total_surplus += day_surplus;
            }

            // 6️⃣ Optional details
            if *details && (*now || period.as_ref().is_some_and(|p| p.len() == 10)) {
                print_details(&day_summary);
            }

            any_output = true;
        }

        // 7️⃣ Totale finale
        if any_output && !*events_only {
            println!("{:>108}", "--------------------------");

            let color = colors::color_for_surplus(total_surplus);
            let formatted = mins2readable(total_surplus, false, false);

            println!(
                "{:>116}",
                format!("Σ Total surplus: {}{}{}", color, formatted, colors::RESET)
            );
        }

        Ok(())
    } else {
        Ok(())
    }
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Helper: Resolve period → Vec<NaiveDate>
// ───────────────────────────────────────────────────────────────────────────────
//

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

//
// ───────────────────────────────────────────────────────────────────────────────
// Header
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_header(period: &Option<String>) {
    if let Some(p) = period {
        if p == "this_month" {
            let today = date::today();
            let month_name = date::month_name(&format!("{:02}", today.month()));
            println!("📅 Saved sessions for {} {}\n", month_name, today.year());
            return;
        }
        match p.len() {
            4 => {
                // header for year
                println!("📅 Saved sessions for year {}\n", p);
            }
            7 => {
                // header for month
                let parts: Vec<&str> = p.split('-').collect();
                if parts.len() == 2 {
                    println!(
                        "📅 Saved sessions for {} {}\n",
                        date::month_name(parts[1]),
                        parts[0]
                    );
                }
            }
            10 => {
                // header for single date
                println!("📅 Saved session for date {}\n", p);
            }
            15 => {
                // header for period between two dates
                let parts: Vec<&str> = p.split(':').collect();
                if parts.len() == 2 {
                    println!("📅 Saved sessions from {} to {}\n", parts[0], parts[1]);
                }
            }
            _ => {}
        }
    }
}
//
// ───────────────────────────────────────────────────────────────────────────────
// Modalità list --events
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_raw_events(events: &[Event]) {
    for ev in events {
        //eprintln!("event: {:?}", ev);
        let lunch = colors::colorize_optional(&format!("{:>2} min", ev.lunch.unwrap_or(0)));
        let pos_label = ev.location.label();
        let pos_color = ev.location.color();
        let pos_fmt = formatting::pad_right(pos_label, 16);

        let dash = if ev.kind.is_in() { "→" } else { " " };
        let date_str = if ev.kind.is_in() {
            ev.date_str()
        } else {
            String::new()
        };

        println!(
            "{} {:^10} {} | {:>4} | lunch {} | {}{}\x1b[0m | {:^6} | {:>2}",
            dash,
            date_str,
            colors::colorize_in_out(&ev.time_str(), ev.kind.is_in()),
            ev.kind.et_as_str(),
            lunch,
            pos_color,
            pos_fmt,
            ev.source,
            ev.pair
        );
    }
}
//
// ───────────────────────────────────────────────────────────────────────────────
// Daily row (the core of the command)
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_daily_row(
    date: &NaiveDate,
    events: &[Event],
    summary: &DaySummary,
    cfg: &Config,
) -> Option<i64> {
    let timeline = &summary.timeline;
    if timeline.pairs.is_empty() {
        return None;
    }

    let first_in = timeline.pairs[0].in_event.timestamp();
    let first_in_str = first_in.format("%H:%M").to_string();

    let last_out_opt = timeline
        .pairs
        .iter()
        .filter_map(|p| p.out_event.as_ref())
        .map(|ev| ev.timestamp())
        .next_back();

    // Position of the day
    let day_position = get_day_position(timeline);

    // Lunch total
    let mut lunch_total: i64 = timeline.pairs.iter().map(|p| p.lunch_minutes).sum();
    if lunch_total == 0 {
        lunch_total = events.iter().map(|ev| ev.lunch.unwrap_or(0) as i64).sum();
    }

    // Expected exit timestamp: first_in + expected_minutes (min_work_duration + lunch)
    let expected_exit = first_in + chrono::Duration::minutes(summary.expected);
    let expected_exit_str = expected_exit.format("%H:%M").to_string();

    // Weekday formatting
    let wd_type = cfg
        .show_weekday
        .chars()
        .next()
        .unwrap_or('m')
        .to_ascii_lowercase();
    let weekday = date::weekday_str(&date.to_string(), wd_type);

    let date_shown = format!("{} ({})", date, weekday);
    let pos_label = day_position.label();
    let pos_color = day_position.color();
    let pos_fmt = formatting::pad_right(pos_label, 16);

    // Lunch
    let lunch_str = if lunch_total > 0 {
        crate::utils::time::format_minutes(lunch_total)
    } else {
        "--:--".to_string()
    };
    let lunch_c = colors::colorize_optional(&lunch_str);

    // End
    let end_str = last_out_opt
        .map(|ts| ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string());
    let end_c = colors::colorize_optional(&end_str);

    // Surplus
    let surplus_opt = last_out_opt.map(|out| (out - expected_exit).num_minutes());

    let (surplus_str, surplus_color) = match surplus_opt {
        None => ("-".to_string(), colors::GREY),
        Some(0) => ("0".to_string(), colors::GREY),
        Some(v) => {
            let color = colors::color_for_surplus(v);
            (format!("{:+}", v), color)
        }
    };

    println!(
        "{} | {}{}\x1b[0m | Start {:^5} | Lunch {:^5} | End {:^5} | Expected {:^5} | Surplus {}{:>5} min\x1b[0m",
        date_shown,
        pos_color,
        pos_fmt,
        first_in_str,
        lunch_c,
        end_c,
        expected_exit_str,
        surplus_color,
        surplus_str,
    );

    surplus_opt
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Details
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_details(summary: &DaySummary) {
    println!("    Details:");
    for (idx, p) in summary.timeline.pairs.iter().enumerate() {
        let in_t = p.in_event.timestamp().format("%H:%M");
        let in_c = colors::colorize_in_out(&in_t.to_string(), true);

        let out_t = p
            .out_event
            .as_ref()
            .map(|ev| ev.timestamp().format("%H:%M").to_string())
            .unwrap_or("--:--".to_string());
        let out_c = colors::colorize_in_out(&out_t, false);

        let worked = colors::colorize_optional(&mins2readable(p.duration_minutes, false, false));
        let lunch = colors::colorize_optional(&format!("{:>2} min", p.lunch_minutes));
        let day_position = get_day_position(&summary.timeline);
        let pos_label = day_position.label();
        let pos_color = day_position.color();
        let pos_fmt = formatting::pad_right(pos_label, 16);

        println!(
            "      Pair {:>2}: IN {:^5} | OUT {:^5} | worked {:^7} | lunch {} | {}{}\x1b[0m",
            idx + 1,
            in_c,
            out_c,
            worked,
            lunch,
            pos_color,
            pos_fmt
        );
    }
    println!();
}
