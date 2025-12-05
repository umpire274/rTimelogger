use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::calculator::surplus;
use crate::core::logic::Core;
use crate::db::pool::DbPool;
use crate::db::queries::load_events_by_date;
use crate::errors::{AppError, AppResult};
use crate::models::event::Event;
use crate::models::location::Location;
use crate::utils::colors;
use crate::utils::colors::{GREY, RESET};
use crate::utils::date;
use crate::utils::date::{month_name, weekday_str};
use crate::utils::formatting;
use chrono::Datelike;
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

        // Determina l’insieme di date da elaborare
        let dates = if *l_now {
            vec![date::today()]
        } else {
            resolve_period(period)?
        };

        if dates.is_empty() {
            println!("⚠️  No recorded sessions found");
            return Ok(());
        }

        let mut total_surplus: i64 = 0;
        let mut any_row = false;

        // Durata lavoro prevista dal config (es. "8h")
        let work_minutes = Core::parse_work_duration_to_minutes(&cfg.min_work_duration);

        // intestazione “mensile/annuale” approssimativa, come nella 0.7.x
        if !*l_now && let Some(p) = period {
            if p.len() == 4 {
                println!("📅 Saved sessions for year {}:\n", p);
            } else if p.len() == 7 {
                let parts: Vec<&str> = p.split('-').collect();
                if parts.len() == 2 {
                    let year = parts[0];
                    let month = parts[1];
                    println!("📅 Saved sessions for {} {}:\n", month_name(month), year);
                }
            }
        }

        let mut last_month: Option<(i32, u32)> = None;

        for d in dates {
            let d_str = d.to_string();
            let d = NaiveDate::parse_from_str(&d_str, "%Y-%m-%d")
                .map_err(|_| AppError::InvalidDate(d_str.clone()))?;

            // --- 🔥 Controllo cambio mese ---
            let current_month = (d.year(), d.month());
            if let Some((ly, lm)) = last_month
                && (lm != current_month.1 || ly != current_month.0)
            {
                // mese differente → separatore
                println!("{:>105}", "-------------------------");
            }
            last_month = Some(current_month);
            // --- 🔥 Fine controllo cambio mese ---

            let events = load_events_by_date(&mut pool, &d)?;

            if events.is_empty() {
                continue;
            }

            if *events_only {
                print_events(&events);
                continue;
            }

            let summary_model = Core::build_daily_summary(&events, cfg);

            if summary_model.timeline.pairs.is_empty() {
                println!("No valid pairs for {}.", d);
                continue;
            }

            if let Some(day_surplus) =
                print_daily_summary_row(&d, &events, &summary_model, work_minutes, cfg)
            {
                total_surplus += day_surplus;
            }

            if *details {
                println!("    Details:");
                for (idx, p) in summary_model.timeline.pairs.iter().enumerate() {
                    let in_ts = p.in_event.timestamp();
                    let out_ts = p
                        .out_event
                        .as_ref()
                        .map(|ev| ev.timestamp().format("%H:%M").to_string())
                        .unwrap_or_else(|| "-".to_string());

                    let str_worked = formatting::mins2readable(p.duration_minutes, false, false);
                    let (position_label, position_color) =
                        formatting::describe_position(p.position.code());

                    println!(
                        "      Pair {:>2}: IN {} | OUT {} | worked {} | lunch {} min | {}{}\x1b[0m",
                        idx + 1,
                        in_ts.format("%H:%M"),
                        out_ts,
                        str_worked,
                        p.lunch_minutes,
                        position_color,
                        position_label,
                    );
                }
                println!();
            }

            any_row = true;
        }

        // riepilogo totale
        if any_row && !*events_only {
            println!();
            println!("{:>105}", "-------------------------");
            let total_color = colors::color_for_surplus(total_surplus);
            println!(
                "{:>114}",
                format!(
                    "Σ Total surplus: {}{}{}",
                    total_color,
                    formatting::mins2readable(total_surplus, true, false),
                    "\x1b[0m",
                )
            );
        } else if !any_row && !*events_only {
            println!("⚠️  No recorded sessions found");
        }
    }

    Ok(())
}

/// Risolve --period in un vettore di NaiveDate
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

    // default: mese corrente
    date::current_month_dates().map_err(AppError::InvalidDate)
}

/// Stampa gli eventi grezzi (modalità `list --events`)
fn print_events(events: &[Event]) {
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
    println!();
}

/// Stampa la riga aggregata stile 0.7.7 per una singola giornata.
/// Ritorna Some(surplus_minutes) se è presente un OUT, altrimenti None.
fn print_daily_summary_row(
    date: &NaiveDate,
    events: &[Event],
    summary: &crate::models::day_summary::DaySummary,
    work_minutes: i64,
    cfg: &Config,
) -> Option<i64> {
    use crate::core::calculator::timeline::Pair;

    let timeline = &summary.timeline;

    if timeline.pairs.is_empty() {
        return None;
    }

    // primo IN e ultimo OUT
    let first_in_pair: &Pair = &timeline.pairs[0];
    let first_in_ts = first_in_pair.in_event.timestamp();
    let first_in_time = first_in_ts.format("%H:%M").to_string();

    let last_out_ts_opt = timeline
        .pairs
        .iter()
        .filter_map(|p| p.out_event.as_ref())
        .next_back()
        .map(|ev| ev.timestamp());

    // posizione giornaliera: se tutte uguali → quella; altrimenti Mixed
    let day_position = {
        let mut iter = timeline.pairs.iter().map(|p| p.position);
        if let Some(first) = iter.next() {
            if iter.all(|pos| pos == first) {
                first
            } else {
                Location::Mixed
            }
        } else {
            Location::Mixed
        }
    };

    // lunch totale del giorno, secondo la logica della timeline
    let mut lunch_total: i64 = summary.timeline.pairs.iter().map(|p| p.lunch_minutes).sum();

    // Fallback: se per qualche motivo la logica avanzata restituisce 0
    // ma ci sono lunch espliciti sugli eventi, usali.
    if lunch_total == 0 {
        lunch_total = events.iter().map(|ev| ev.lunch.unwrap_or(0) as i64).sum();
    }

    // Calcolo dell'expected usando la funzione "ufficiale"
    let expected_nt = Core::calculate_expected_exit(
        *date,
        &first_in_time,      // "HH:MM"
        work_minutes as i32, // minuti lavorativi da config
        lunch_total as i32,  // lunch totale effettivo
    );
    let expected_exit_str = expected_nt.format("%H:%M").to_string();

    // formattazioni varie
    let fmt_weekday = cfg
        .show_weekday
        .chars()
        .next()
        .map(|c| c.to_ascii_lowercase())
        .unwrap_or('m');

    let weekday = weekday_str(&date.to_string(), fmt_weekday);
    let date_shown = format!("{} ({})", date, weekday);

    let pos_label = day_position.label();
    let pos_color = day_position.color();
    let pos_fmt = formatting::pad_right(pos_label, 16);

    let maybe_lunch = if lunch_total > 0 {
        crate::utils::time::format_minutes(lunch_total)
    } else {
        "--:--".to_string()
    };
    let lunch_str = colors::colorize_optional(&maybe_lunch);

    let maybe_end = last_out_ts_opt
        .map(|ts| ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string());
    let end_str = colors::colorize_optional(&maybe_end);

    let surplus_minutes_opt = surplus::daily_surplus_from_times(&end_str, &expected_exit_str);

    let surplus_str = match surplus_minutes_opt {
        None => "-".to_string(),
        Some(0) => "0".to_string(),
        Some(v) => format!("{:+}", v),
    };

    let surplus_color = match surplus_minutes_opt {
        None => GREY,    // nessun valore → grigio
        Some(0) => GREY, // zero → grigio
        Some(v) if v < 0 => colors::RED,
        Some(v) if v > 0 => colors::GREEN,
        _ => RESET,
    };
    println!(
        "{} | {}{}\x1b[0m | Start {} | Lunch {} | End {} | Expected {} | Surplus {}{:>4} min\x1b[0m",
        date_shown,
        pos_color,
        pos_fmt,
        first_in_time,
        lunch_str,
        end_str,
        expected_exit_str,
        surplus_color,
        surplus_str
    );

    surplus_minutes_opt
}
