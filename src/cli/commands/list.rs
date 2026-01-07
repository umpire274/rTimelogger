use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::load_events_by_date;
use crate::errors::{AppError, AppResult};
use crate::models::day_summary::DaySummary;
use crate::models::event::Event;
use crate::models::location::Location;
use crate::ui::messages::{info, warning};
use crate::utils::date::get_day_position;
use crate::utils::formatting::FOOTER_INDENT;
use crate::utils::table::{DAILY_TABLE_WIDTH, EVENTS_TABLE_WIDTH};
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
        let show_wd = cfg.show_weekday.to_ascii_lowercase() != "n";

        // 1️⃣ Determina le date
        let dates = if *now {
            vec![date::today()]
        } else {
            resolve_period(period)?
        };

        if dates.is_empty() {
            warning("⚠️  No recorded sessions found");
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
                " {:^17} | {:^4} | {:^12} | {:^16} | {:^6} | {:^4} | {:^8}",
                "Date Time", "Type", "Lunch", "Position", "Source", "Pair", "Work Gap"
            );
            println!("{:-<etwidth$}", "-", etwidth = EVENTS_TABLE_WIDTH);
        }

        let mut printed_daily_header = false;

        for d in dates {
            let d_str = d.to_string();
            let parsed_date = NaiveDate::parse_from_str(&d_str, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidDate(d_str))?;

            // 🧨 Month separator
            let current_month = (parsed_date.year(), parsed_date.month());
            if let Some((ly, lm)) = last_month
                && (ly, lm) != current_month
            {
                if show_wd {
                    println!("{:-<twidth$}", "-", twidth = DAILY_TABLE_WIDTH);
                } else {
                    println!("{:-<twidth$}", "-", twidth = DAILY_TABLE_WIDTH - 4);
                }
                print_daily_table_header(cfg);
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
                info(format!("No valid pairs for {}.", parsed_date));
                continue;
            }

            // 5️⃣ Print daily row using timeline
            if !printed_daily_header {
                print_daily_table_header(cfg);
                printed_daily_header = true;
            }

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
            // separatore coerente con header tabella daily (adegua se hai cambiato la larghezza)
            let mut etwidth = FOOTER_INDENT;
            if show_wd {
                println!("{:-<twidth$}", "-", twidth = etwidth + 2);
            } else {
                println!("{:-<twidth$}", "-", twidth = etwidth - 2);
                etwidth -= 2;
            }

            let color = colors::color_for_surplus(total_surplus);
            let delta = format_delta_compact(total_surplus);

            // Plain (no ANSI) used ONLY for alignment length calculation
            let footer_plain = format!("Σ Total ΔWORK: {}", delta);
            let footer_styled = format!("Σ Total ΔWORK: {} {}{}", colors::RESET, color, delta);

            // Compute padding so footer ends at the right edge of the table
            let prefix = formatting::right_pad_prefix(etwidth, &footer_plain);

            // Footer “section bar” + valore colorato
            println!(
                "{}{} {}{}",
                prefix,
                colors::SECTION_BAR,
                footer_styled,
                colors::RESET
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
            info(format!(
                "📅 Saved sessions for {} {}\n",
                month_name,
                today.year()
            ));
            return;
        }
        match p.len() {
            4 => {
                // header for year
                info(format!("📅 Saved sessions for year {}\n", p));
            }
            7 => {
                // header for month
                let parts: Vec<&str> = p.split('-').collect();
                if parts.len() == 2 {
                    info(format!(
                        "📅 Saved sessions for {} {}\n",
                        date::month_name(parts[1]),
                        parts[0]
                    ));
                }
            }
            10 => {
                // header for single date
                info(format!("📅 Saved session for date {}\n", p));
            }
            15 => {
                // header for period between two dates
                let parts: Vec<&str> = p.split(':').collect();
                if parts.len() == 2 {
                    info(format!(
                        "📅 Saved sessions from {} to {}\n",
                        parts[0], parts[1]
                    ));
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
    let mut last_date: Option<String> = None;

    for ev in events {
        //eprintln!("event: {:?}", ev);
        let lunch = colors::colorize_optional(&format!("{:>2} min", ev.lunch.unwrap_or(0)));
        let pos_label = ev.location.label();
        let pos_color = ev.location.color();
        let pos_fmt = formatting::pad_right(pos_label, 16);

        let (dash, date_str) = if ev.kind.is_in() {
            let current_date = ev.date_str(); // String stabile

            match &last_date {
                Some(d) if d == &current_date => {
                    // stesso giorno → niente freccia, niente data
                    (" ", " ".repeat(10))
                }
                _ => {
                    // nuovo giorno → freccia + data
                    last_date = Some(current_date.clone());
                    ("→", current_date)
                }
            }
        } else {
            // OUT → mai freccia, mai data
            (" ", " ".repeat(10))
        };

        println!(
            "{} {:^10} {} | {:>4} | lunch {} | {}{}\x1b[0m | {:^6} | {:>3}  | {:^8}",
            dash,
            date_str,
            colors::colorize_in_out(&ev.time_str(), ev.kind.is_in()),
            ev.kind.et_as_str(),
            lunch,
            pos_color,
            pos_fmt,
            ev.source,
            ev.pair,
            if ev.work_gap { "YES" } else { "" }
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

    // Position of the day
    let day_position = get_day_position(timeline);

    let date_str = date.to_string();
    let wd_type = cfg
        .show_weekday
        .chars()
        .next()
        .unwrap_or('m')
        .to_ascii_lowercase();
    let weekday = date::weekday_str(&date_str, wd_type);

    let pos_label = day_position.label();
    let pos_color = day_position.color();
    let pos_fmt = formatting::pad_right(pos_label, 16);

    // Defaults (Holiday / N/A)
    let grey_time = format!("{}--:--{}", colors::GREY, colors::RESET);

    let mut first_in_str = grey_time.clone();
    let mut lunch_c = grey_time.clone();
    let mut end_c = grey_time.clone();
    let mut expected_exit_str = grey_time.clone();

    let mut surplus_opt: Option<i64> = Some(0); // Holiday contributes 0
    let mut surplus_display = "-".to_string();
    let mut surplus_color = colors::GREY;

    if day_position != Location::Holiday {
        let first_in = timeline.pairs[0].in_event.timestamp();
        first_in_str = first_in.format("%H:%M").to_string();

        let last_out_opt = timeline
            .pairs
            .iter()
            .filter_map(|p| p.out_event.as_ref())
            .map(|ev| ev.timestamp())
            .next_back();

        // Lunch total
        let mut lunch_total: i64 = timeline.pairs.iter().map(|p| p.lunch_minutes).sum();
        if lunch_total == 0 {
            lunch_total = events.iter().map(|ev| ev.lunch.unwrap_or(0) as i64).sum();
        }

        // Expected exit timestamp
        let expected_exit = first_in + chrono::Duration::minutes(summary.expected);
        expected_exit_str = expected_exit.format("%H:%M").to_string();

        // Lunch
        let lunch_str = if lunch_total > 0 {
            crate::utils::time::format_minutes(lunch_total)
        } else {
            "--:--".to_string()
        };
        lunch_c = colors::colorize_optional(&lunch_str);

        // End
        let end_str = last_out_opt
            .map(|ts| ts.format("%H:%M").to_string())
            .unwrap_or_else(|| "--:--".to_string());
        end_c = colors::colorize_optional(&end_str);

        // Surplus (worked)
        let non_work_gap_minutes: i64 = timeline
            .gaps
            .iter()
            .filter(|g| !g.is_work_gap)
            .map(|g| g.duration_minutes)
            .sum();

        surplus_opt =
            last_out_opt.map(|out| (out - expected_exit).num_minutes() - non_work_gap_minutes);

        match surplus_opt {
            None => {
                surplus_display = "-".to_string();
                surplus_color = colors::GREY;
            }
            Some(0) => {
                surplus_display = "0".to_string();
                surplus_color = colors::GREY;
            }
            Some(v) => {
                let abs = mins2readable(v.abs(), false, false); // "02h 04m"
                let compact = abs.replace(' ', ""); // "02h04m"
                surplus_display = format!("{}{}", if v < 0 { "-" } else { "+" }, compact);
                surplus_color = colors::color_for_surplus(v);
            }
        }
    }

    println!(
        " {:^10} | {:^2} | {}{}\x1b[0m | {:^5} | {:^5} | {:^5} | {:^5} | {}{:>7}\x1b[0m",
        date_str,
        weekday,
        pos_color,
        pos_fmt,
        first_in_str,
        lunch_c,
        end_c,
        expected_exit_str,
        surplus_color,
        surplus_display
    );

    surplus_opt
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Details
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_details(summary: &DaySummary) {
    // Se non ci sono pair, non stampo dettagli
    if summary.timeline.pairs.is_empty() {
        return;
    }

    println!();
    println!("    {} DETAILS {}", colors::SECTION_BAR, colors::RESET);
    println!(
        "    {:^4} | {:^5} | {:^5} | {:^6} | {:^5} | {:^16} | {:^2}",
        "PAIR", "IN", "OUT", "WORKED", "LUNCH", "POSITION", "WG"
    );
    println!("    {:-<72}", "-");

    for (idx, p) in summary.timeline.pairs.iter().enumerate() {
        let in_t = p.in_event.timestamp().format("%H:%M").to_string();
        let in_c = colors::colorize_in_out(&in_t, true);

        let out_t = p
            .out_event
            .as_ref()
            .map(|ev| ev.timestamp().format("%H:%M").to_string())
            .unwrap_or_else(|| "--:--".to_string());
        let out_c = colors::colorize_in_out(&out_t, false);

        // WORKED: già in formato leggibile. Lo compatto senza spazi (es: "02h04m") per stabilità colonne.
        let worked_raw = mins2readable(p.duration_minutes, false, false); // "00h 42m"
        let worked_compact = worked_raw.replace(' ', ""); // "00h42m"
        let worked_c = colors::colorize_optional(&worked_compact);

        // LUNCH: compatto "30m"
        let lunch_compact = format!("{:>2}m", p.lunch_minutes);
        let lunch_c = colors::colorize_optional(&lunch_compact);

        let pos_label = p.position.label();
        let pos_color = p.position.color();
        let pos_fmt = formatting::pad_right(pos_label, 16);

        let wg_str = if p.work_gap { "Y" } else { "" };

        println!(
            "    {:>4} | {:^5} | {:^5} | {:^6} | {:^5} | {}{}\x1b[0m | {:^2}",
            idx + 1,
            in_c,
            out_c,
            worked_c,
            lunch_c,
            pos_color,
            pos_fmt,
            wg_str
        );
    }

    println!();
}

fn print_daily_table_header(cfg: &Config) {
    // Weekday column is optional in your config; if disabled, keep header consistent with output
    let show_wd = cfg.show_weekday.to_ascii_lowercase() != "n"; // adattala se "n" non è il tuo flag
    if show_wd {
        println!(
            " {:^10} | {:^2} | {:^16} | {:^5} | {:^5} | {:^5} | {:^5} | {:^7}",
            "DATE", "WD", "POSITION", "IN", "LNCH", "OUT", "TGT", "ΔWORK"
        );
        println!("{:-<twidth$}", "-", twidth = DAILY_TABLE_WIDTH);
    } else {
        println!(
            " {:^10} | {:^16} | {:^5} | {:^5} | {:^5} | {:^5} | {:^7}",
            "DATE", "POSITION", "IN", "LNCH", "OUT", "TGT", "ΔWORK"
        );
        println!("{:-<twidth$}", "-", twidth = DAILY_TABLE_WIDTH - 4);
    }
}

fn format_delta_compact(minutes: i64) -> String {
    let abs = mins2readable(minutes.abs(), false, true); // es: "02h 04m"
    format!("{}{}", if minutes < 0 { "-" } else { "+" }, abs)
}
