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
use crate::utils::table::EVENTS_TABLE_WIDTH;
use crate::utils::{colors, date, formatting, mins2readable};
use chrono::{Datelike, NaiveDate};

//
// ───────────────────────────────────────────────────────────────────────────────
// Local Enums and Layout
// ───────────────────────────────────────────────────────────────────────────────
//

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WeekdayMode {
    None,
    Short,
    Medium,
    Long,
}

fn weekday_mode(cfg: &Config) -> WeekdayMode {
    match cfg.show_weekday.to_ascii_lowercase().as_str() {
        "none" => WeekdayMode::None,
        "short" => WeekdayMode::Short,
        "medium" => WeekdayMode::Medium,
        "long" => WeekdayMode::Long,
        // fallback conservativo
        _ => WeekdayMode::Medium,
    }
}

fn weekday_type_char(mode: WeekdayMode) -> Option<char> {
    match mode {
        WeekdayMode::None => None,
        WeekdayMode::Short => Some('s'),
        WeekdayMode::Medium => Some('m'),
        WeekdayMode::Long => Some('l'),
    }
}

/// Nel compact: se weekday abilitato, forza sempre Short (2 lettere) per mantenere layout stabile.
fn effective_weekday_mode(mode: WeekdayMode, compact: bool) -> WeekdayMode {
    if !compact {
        return mode;
    }
    match mode {
        WeekdayMode::None => WeekdayMode::None,
        _ => WeekdayMode::Short,
    }
}

/// Larghezza della colonna DATE in base al weekday.
/// - None:   "YYYY-MM-DD"               = 10
/// - Short:  "YYYY-MM-DD (Fr)"          = 15
/// - Medium: "YYYY-MM-DD (Fri)"         = 16
/// - Long:   "YYYY-MM-DD (Wednesday)"   = 22 (max 9 chars weekday)
fn date_col_width(mode: WeekdayMode) -> usize {
    match mode {
        WeekdayMode::None => 10,
        WeekdayMode::Short => 15,
        WeekdayMode::Medium => 16,
        WeekdayMode::Long => 22,
    }
}

// Column widths (daily standard table)
const POS_W: usize = 16;
const TIME_W: usize = 5; // IN / LNCH / OUT / TGT
const DWORK_W: usize = 7;

/// Daily table total width, computed from column widths.
/// Format used:
/// " {DATE} | {POSITION} | {IN} | {LNCH} | {OUT} | {TGT} | {ΔWORK}"
fn daily_table_width(mode: WeekdayMode) -> usize {
    let dw = date_col_width(mode);
    // 1 leading space + cols + separators (" | " = 3 chars) between 7 columns
    // Total = 1 + date + 3 + pos + 3 + in + 3 + lnch + 3 + out + 3 + tgt + 3 + dwork
    1 + dw + 3 + POS_W + 3 + TIME_W + 3 + TIME_W + 3 + TIME_W + 3 + TIME_W + 3 + DWORK_W + 1
}

// Compact table widths
const CPOS_W: usize = 12;
const TRIPLE_W: usize = 21; // "IN / LNCH / OUT"
const CTGT_W: usize = 5;
const CDWORK_W: usize = 7;

/// Compact table total width.
/// Format used:
/// "{DATE} | {POSITION} | {IN/LNCH/OUT} | {TGT} | {ΔWORK}"
fn compact_table_width(mode: WeekdayMode) -> usize {
    let dw = date_col_width(mode);
    // date + 3 + pos + 3 + triple + 3 + tgt + 3 + dwork
    dw + 3 + CPOS_W + 3 + TRIPLE_W + 3 + CTGT_W + 3 + CDWORK_W + 3
}

fn format_date_with_weekday(date: &NaiveDate, mode: WeekdayMode) -> String {
    let date_str = date.to_string();
    if let Some(ch) = weekday_type_char(mode) {
        let wd = date::weekday_str(&date_str, ch);
        format!("{} ({})", date_str, wd)
    } else {
        date_str
    }
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Public entry
// ───────────────────────────────────────────────────────────────────────────────
//

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::List {
        compact,
        period,
        now,
        details,
        events: events_only,
        ..
    } = cmd
    {
        if *compact && *details {
            return Err(AppError::InvalidArgs(
                "--compact cannot be used together with --details.".into(),
            ));
        }

        let mut pool = DbPool::new(&cfg.database)?;
        let wd_mode_cfg = weekday_mode(cfg);
        let wd_mode = effective_weekday_mode(wd_mode_cfg, *compact);

        // 1️⃣ Determine dates
        let dates = if *now {
            vec![date::today()]
        } else {
            resolve_period(period)?
        };

        if dates.is_empty() {
            warning("⚠️  No recorded sessions found");
            return Ok(());
        }

        // 2️⃣ Header (only if not --now)
        if !*now {
            if period.is_some() {
                print_header(period);
            } else {
                print_header(&Some("this_month".to_string()));
            }
        }

        let mut total_surplus: i64 = 0;
        let mut any_output = false;

        // Month separator state (only for daily summaries)
        let mut last_month: Option<(i32, u32)> = None;
        let mut printed_daily_header = false;

        // EVENTS header if requested
        if *events_only && Event::has_events_for_dates(&mut pool, &dates)? {
            println!("EVENTS:");
            println!();
            println!(
                " {:^17} | {:^4} | {:^12} | {:^16} | {:^6} | {:^4} | {:^8}",
                "Date Time", "Type", "Lunch", "Position", "Source", "Pair", "Work Gap"
            );
            println!("{:-<w$}", "-", w = EVENTS_TABLE_WIDTH);
        }

        for day in dates {
            // Month separator (daily summaries only)
            if !*events_only {
                let current_month = (day.year(), day.month());
                if let Some((ly, lm)) = last_month
                    && (ly, lm) != current_month
                {
                    let twidth = if *compact {
                        compact_table_width(wd_mode)
                    } else {
                        daily_table_width(wd_mode)
                    };
                    println!("{:-<w$}", "-", w = twidth);

                    // reprint table header at month boundary
                    if *compact {
                        print_compact_header(wd_mode);
                    } else {
                        print_daily_table_header(wd_mode);
                    }
                    printed_daily_header = true;
                }
                last_month = Some(current_month);
            }

            // Load events
            let events = load_events_by_date(&mut pool, &day)?;
            if events.is_empty() {
                continue;
            }

            if *events_only {
                print_raw_events(&events);
                continue;
            }

            // Build summary
            let day_summary = Core::build_daily_summary(&events, cfg);
            if day_summary.timeline.pairs.is_empty() {
                info(format!("No valid pairs for {}.", day));
                continue;
            }

            // Print header once
            if !printed_daily_header {
                if *compact {
                    print_compact_header(wd_mode);
                } else {
                    print_daily_table_header(wd_mode);
                }
                printed_daily_header = true;
            }

            // Print row
            let day_surplus = if *compact {
                print_daily_row_compact(&day, &events, &day_summary, cfg, wd_mode)
            } else {
                print_daily_row(&day, &events, &day_summary, cfg, wd_mode)
            };

            if let Some(v) = day_surplus {
                total_surplus += v;
            }

            // Optional details (not allowed in compact)
            if *details && (*now || period.as_ref().is_some_and(|p| p.len() == 10)) {
                print_details(&day_summary);
            }

            any_output = true;
        }

        // Footer total
        if any_output && !*events_only {
            let twidth = if *compact {
                compact_table_width(wd_mode)
            } else {
                daily_table_width(wd_mode)
            };
            println!("{:-<w$}", "-", w = twidth);

            let color = colors::color_for_surplus(total_surplus);
            let delta = format_delta_compact(total_surplus);

            // background (SECTION_BAR) only on label
            let footer_plain = format!("Σ Total ΔWORK: {}", delta);
            let prefix = formatting::right_pad_prefix(
                twidth.saturating_sub(if *compact { 1 } else { 3 }),
                &footer_plain,
            );

            if *compact {
                println!(
                    "{}Σ Total ΔWORK: {}{}{}",
                    prefix,
                    color,
                    delta,
                    colors::RESET
                );
            } else {
                println!(
                    "{}{} Σ Total ΔWORK: {} {}{}{}",
                    prefix,
                    colors::SECTION_BAR, // background ON (label)
                    colors::RESET,       // background OFF
                    color,               // value color
                    delta,               // value
                    colors::RESET        // final reset
                );
            }
        }

        Ok(())
    } else {
        Ok(())
    }
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Period resolver
// ───────────────────────────────────────────────────────────────────────────────
//

fn resolve_period(period: &Option<String>) -> AppResult<Vec<NaiveDate>> {
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
            4 => info(format!("📅 Saved sessions for year {}\n", p)),
            7 => {
                let parts: Vec<&str> = p.split('-').collect();
                if parts.len() == 2 {
                    info(format!(
                        "📅 Saved sessions for {} {}\n",
                        date::month_name(parts[1]),
                        parts[0]
                    ));
                }
            }
            10 => info(format!("📅 Saved session for date {}\n", p)),
            15 => {
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
// list --events
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_raw_events(events: &[Event]) {
    let mut last_date: Option<String> = None;

    for ev in events {
        let lunch = colors::colorize_optional(&format!("{:>2} min", ev.lunch.unwrap_or(0)));
        let pos_label = ev.location.label();
        let pos_color = ev.location.color();
        let pos_fmt = formatting::pad_right(pos_label, POS_W);

        let (dash, date_str) = if ev.kind.is_in() {
            let current_date = ev.date_str();
            match &last_date {
                Some(d) if d == &current_date => (" ", " ".repeat(10)),
                _ => {
                    last_date = Some(current_date.clone());
                    ("→", current_date)
                }
            }
        } else {
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
// Daily standard table
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_daily_table_header(wd_mode: WeekdayMode) {
    let dw = date_col_width(wd_mode);
    let twidth = daily_table_width(wd_mode);

    println!(
        " {:^dw$} | {:^16} | {:^5} | {:^5} | {:^5} | {:^5} | {:^7}",
        "DATE",
        "POSITION",
        "IN",
        "LNCH",
        "OUT",
        "TGT",
        "ΔWORK",
        dw = dw
    );

    println!("{:-<w$}", "-", w = twidth);
}

fn print_daily_row(
    date: &NaiveDate,
    events: &[Event],
    summary: &DaySummary,
    _cfg: &Config,
    wd_mode: WeekdayMode,
) -> Option<i64> {
    let timeline = &summary.timeline;
    if timeline.pairs.is_empty() {
        return None;
    }

    let day_position = get_day_position(timeline);
    let date_str = format_date_with_weekday(date, wd_mode);
    let dw = date_col_width(wd_mode);

    let pos_label = day_position.label();
    let pos_color = day_position.color();
    let pos_fmt = formatting::pad_right(pos_label, POS_W);

    // Defaults (Holiday / N/A)
    let grey_time = format!("{}--:--{}", colors::GREY, colors::RESET);
    let mut first_in_str = grey_time.clone();
    let mut lunch_c = grey_time.clone();
    let mut end_c = grey_time.clone();
    let mut expected_exit_str = grey_time.clone();

    // Defaults for surplus
    let mut surplus_opt: Option<i64> = Some(0); // Holiday contributes 0
    let mut surplus_display = "-".to_string();
    let mut surplus_color = colors::GREY;

    if day_position != Location::Holiday && day_position != Location::NationalHoliday {
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

        // Target end
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
        " {:<dw$} | {}{}\x1b[0m | {:^5} | {:^5} | {:^5} | {:^5} | {}{:>7}\x1b[0m",
        date_str,
        pos_color,
        pos_fmt,
        first_in_str,
        lunch_c,
        end_c,
        expected_exit_str,
        surplus_color,
        surplus_display,
        dw = dw
    );

    surplus_opt
}

//
// ───────────────────────────────────────────────────────────────────────────────
// Details
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_details(summary: &DaySummary) {
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

        let worked_raw = mins2readable(p.duration_minutes, false, false);
        let worked_compact = worked_raw.replace(' ', "");
        let worked_c = colors::colorize_optional(&worked_compact);

        let lunch_compact = format!("{:>2}m", p.lunch_minutes);
        let lunch_c = colors::colorize_optional(&lunch_compact);

        let pos_label = p.position.label();
        let pos_color = p.position.color();
        let pos_fmt = formatting::pad_right(pos_label, POS_W);

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

//
// ───────────────────────────────────────────────────────────────────────────────
// Compact table
// ───────────────────────────────────────────────────────────────────────────────
//

fn print_compact_header(wd_mode: WeekdayMode) {
    let dw = date_col_width(wd_mode);
    let twidth = compact_table_width(wd_mode);

    println!(
        "{:^dw$} | {:^12} | {:^21} | {:^5} | {:^7}",
        "DATE",
        "POSITION",
        "IN / LNCH / OUT",
        "TGT",
        "ΔWORK",
        dw = dw
    );

    println!("{:-<w$}", "-", w = twidth);
}

fn format_delta_compact(minutes: i64) -> String {
    let abs = mins2readable(minutes.abs(), false, true); // già compatto
    format!("{}{}", if minutes < 0 { "-" } else { "+" }, abs)
}

fn print_daily_row_compact(
    date: &NaiveDate,
    events: &[Event],
    summary: &DaySummary,
    _cfg: &Config,
    wd_mode: WeekdayMode,
) -> Option<i64> {
    let timeline = &summary.timeline;
    if timeline.pairs.is_empty() {
        return None;
    }

    let dw = date_col_width(wd_mode);
    let date_str = format_date_with_weekday(date, wd_mode);

    let day_position = get_day_position(timeline);
    let pos_label = day_position.label();
    let pos_color = day_position.color();

    if day_position == Location::Holiday || day_position == Location::NationalHoliday {
        println!(
            "{:<dw$} | {}{:<12}{}\x1b[0m | {:<21} | {:^5} | {}Δ -{}\x1b[0m",
            date_str,
            pos_color,
            pos_label,
            colors::RESET,
            format!("{}--:-- / --:-- / --:--{}", colors::GREY, colors::RESET),
            format!("{}--:--{}", colors::GREY, colors::RESET),
            colors::GREY,
            colors::RESET,
            dw = dw
        );
        return Some(0);
    }

    let first_in = timeline.pairs[0].in_event.timestamp();
    let first_in_str = first_in.format("%H:%M").to_string();

    let last_out_opt = timeline
        .pairs
        .iter()
        .filter_map(|p| p.out_event.as_ref())
        .map(|ev| ev.timestamp())
        .next_back();

    let end_str = last_out_opt
        .map(|ts| ts.format("%H:%M").to_string())
        .unwrap_or_else(|| "--:--".to_string());

    let mut lunch_total: i64 = timeline.pairs.iter().map(|p| p.lunch_minutes).sum();
    if lunch_total == 0 {
        lunch_total = events.iter().map(|ev| ev.lunch.unwrap_or(0) as i64).sum();
    }
    let lunch_str = if lunch_total > 0 {
        crate::utils::time::format_minutes(lunch_total)
    } else {
        "--:--".to_string()
    };

    let expected_exit = first_in + chrono::Duration::minutes(summary.expected);
    let target_end_str = expected_exit.format("%H:%M").to_string();

    let non_work_gap_minutes: i64 = timeline
        .gaps
        .iter()
        .filter(|g| !g.is_work_gap)
        .map(|g| g.duration_minutes)
        .sum();

    let surplus_opt =
        last_out_opt.map(|out| (out - expected_exit).num_minutes() - non_work_gap_minutes);

    let (delta_str, delta_color) = match surplus_opt {
        None => ("-".to_string(), colors::GREY),
        Some(0) => ("0".to_string(), colors::GREY),
        Some(v) => {
            let abs = mins2readable(v.abs(), false, true);
            let sign = if v < 0 { "-" } else { "+" };
            (format!("{}{}", sign, abs), colors::color_for_surplus(v))
        }
    };

    let times_string = format!("{} / {} / {}", first_in_str, lunch_str, end_str);
    let delta_value = format!("Δ {}", delta_str);
    println!(
        "{:<dw$} | {}{:<12}{}\x1b[0m | {:<21} | {:^5} | {}{}{}\x1b[0m",
        date_str,
        pos_color,
        pos_label,
        colors::RESET,
        times_string,
        target_end_str,
        delta_color,
        delta_value,
        colors::RESET,
        dw = dw
    );

    surplus_opt
}
