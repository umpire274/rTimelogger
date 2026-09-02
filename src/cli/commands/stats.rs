use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::stats::calculate;
use crate::db::pool::DbPool;
use crate::db::queries::events::load_events_by_range;
use crate::errors::{AppError, AppResult};
use crate::models::work_stats::WorkStats;
use crate::utils::colors;
use crate::utils::time::format_minutes;

const BOLD: &str = "\x1b[1m";
const BOLD_CYAN: &str = "\x1b[1;36m";
const TITLE_WORK: &str = "\x1b[1;48;2;20;50;100;97m";
const TITLE_DAYS: &str = "\x1b[1;45;97m";

#[derive(Clone, Copy)]
enum Alignment {
    Left,
    Right,
}

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    let Commands::Stats { from, to, group_by } = cmd else {
        return Err(AppError::InvalidOperation("Expected stats command".into()));
    };

    if from > to {
        return Err(AppError::InvalidDateRange {
            from: *from,
            to: *to,
        });
    }

    let mut pool = DbPool::new(&cfg.database)?;
    let events = load_events_by_range(&mut pool, from, to)?;
    let rows = calculate(&events, *from, *to, *group_by, cfg)?;

    println!("{BOLD_CYAN}WORK STATISTICS{}", colors::RESET);
    println!("Period: {from} → {to}");
    let grouping_label = match group_by {
        crate::cli::parser::StatsGrouping::Week => "week",
        crate::cli::parser::StatsGrouping::Month => "month",
        crate::cli::parser::StatsGrouping::Year => "year",
    };
    println!("Grouped by: {}\n", grouping_label);
    print_table(&rows);
    Ok(())
}

fn duration(minutes: i64) -> String {
    format_minutes(minutes)
}

fn signed_duration(minutes: i64) -> String {
    if minutes == 0 {
        "0:00".to_string()
    } else {
        format!(
            "{}{}",
            if minutes > 0 { "+" } else { "-" },
            duration(minutes.abs())
        )
    }
}

fn outer_border(left: char, right: char, widths: &[usize]) -> String {
    format!("{left}{}{right}", "─".repeat(table_width(widths) - 2))
}

fn border(left: char, middle: char, right: char, widths: &[usize]) -> String {
    let segments = widths
        .iter()
        .map(|width| "─".repeat(width + 2))
        .collect::<Vec<_>>()
        .join(&middle.to_string());
    format!("{left}{segments}{right}")
}

fn table_width(widths: &[usize]) -> usize {
    widths.iter().sum::<usize>() + widths.len() * 3 + 1
}

fn title_row(title: &str, widths: &[usize], style: &str) -> String {
    format!(
        "│{style} {title:<width$} {reset}│",
        width = table_width(widths) - 4,
        reset = colors::RESET,
    )
}

fn row(values: &[String], widths: &[usize], alignments: &[Alignment], styles: &[&str]) -> String {
    let mut output = String::from("│");
    for (((value, width), alignment), style) in
        values.iter().zip(widths).zip(alignments).zip(styles)
    {
        let padded = match alignment {
            Alignment::Left => format!("{value:<width$}"),
            Alignment::Right => format!("{value:>width$}"),
        };
        output.push_str(&format!(" {style}{padded}{} │", colors::RESET));
    }
    output
}

fn print_table(rows: &[WorkStats]) {
    let time_widths = [9, 8, 8, 10, 10, 12, 10, 9];
    let time_alignments = [
        Alignment::Left,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
    ];

    println!("{}", outer_border('╭', '╮', &time_widths));
    println!(
        "{}",
        title_row(
            "WORK TIME · Expected and recognized hours",
            &time_widths,
            TITLE_WORK,
        )
    );
    println!("{}", border('├', '┬', '┤', &time_widths));
    println!(
        "{}",
        row(
            &[
                "Period".into(),
                "Expected".into(),
                "Worked".into(),
                "Paid Leave".into(),
                "Sick Leave".into(),
                "Nat. Holiday".into(),
                "Recognized".into(),
                "Balance".into(),
            ],
            &time_widths,
            &time_alignments,
            &[BOLD_CYAN; 8],
        )
    );
    println!("{}", border('├', '┼', '┤', &time_widths));

    for stats in rows {
        let balance = stats.balance_minutes();
        println!(
            "{}",
            row(
                &[
                    stats.label.clone(),
                    duration(stats.expected_minutes),
                    duration(stats.worked_minutes),
                    duration(stats.paid_leave_minutes),
                    duration(stats.sick_leave_minutes),
                    duration(stats.national_holiday_minutes),
                    duration(stats.recognized_minutes()),
                    signed_duration(balance),
                ],
                &time_widths,
                &time_alignments,
                &[
                    colors::RESET,
                    colors::RESET,
                    colors::CYAN,
                    colors::MAGENTA,
                    colors::GREY,
                    colors::RED,
                    colors::RESET,
                    colors::color_for_surplus(balance),
                ],
            )
        );
    }

    let total = rows.iter().fold(
        (
            0_i64, 0_i64, 0_i64, 0_i64, 0_i64, 0_u32, 0_u32, 0_u32, 0_u32,
        ),
        |acc, row| {
            (
                acc.0 + row.expected_minutes,
                acc.1 + row.worked_minutes,
                acc.2 + row.paid_leave_minutes,
                acc.3 + row.sick_leave_minutes,
                acc.4 + row.national_holiday_minutes,
                acc.5 + row.worked_days,
                acc.6 + row.paid_leave_days,
                acc.7 + row.sick_leave_days,
                acc.8 + row.national_holiday_days,
            )
        },
    );
    let recognized = total.1 + total.2 + total.3 + total.4;
    let average = if total.5 > 0 {
        duration(total.1 / i64::from(total.5))
    } else {
        "-".into()
    };
    let total_balance = recognized - total.0;

    println!("{}", border('├', '┼', '┤', &time_widths));
    println!(
        "{}",
        row(
            &[
                "TOTAL".into(),
                duration(total.0),
                duration(total.1),
                duration(total.2),
                duration(total.3),
                duration(total.4),
                duration(recognized),
                signed_duration(total_balance),
            ],
            &time_widths,
            &time_alignments,
            &[
                BOLD,
                BOLD,
                BOLD,
                BOLD,
                BOLD,
                BOLD,
                BOLD,
                colors::color_for_surplus(total_balance),
            ],
        )
    );
    println!("{}", border('╰', '┴', '╯', &time_widths));

    let day_widths = [9, 8, 10, 10, 12, 8];
    let day_alignments = [
        Alignment::Left,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
        Alignment::Right,
    ];

    println!();
    println!("{}", outer_border('╭', '╮', &day_widths));
    println!(
        "{}",
        title_row("DAYS · Distribution and average", &day_widths, TITLE_DAYS,)
    );
    println!("{}", border('├', '┬', '┤', &day_widths));
    println!(
        "{}",
        row(
            &[
                "Period".into(),
                "Worked".into(),
                "Paid Leave".into(),
                "Sick Leave".into(),
                "Nat. Holiday".into(),
                "Avg/day".into(),
            ],
            &day_widths,
            &day_alignments,
            &[BOLD_CYAN; 6],
        )
    );
    println!("{}", border('├', '┼', '┤', &day_widths));

    for stats in rows {
        println!(
            "{}",
            row(
                &[
                    stats.label.clone(),
                    stats.worked_days.to_string(),
                    stats.paid_leave_days.to_string(),
                    stats.sick_leave_days.to_string(),
                    stats.national_holiday_days.to_string(),
                    stats
                        .average_worked_minutes()
                        .map(duration)
                        .unwrap_or_else(|| "-".into()),
                ],
                &day_widths,
                &day_alignments,
                &[
                    colors::RESET,
                    colors::CYAN,
                    colors::MAGENTA,
                    colors::GREY,
                    colors::RED,
                    colors::RESET,
                ],
            )
        );
    }

    println!("{}", border('├', '┼', '┤', &day_widths));
    println!(
        "{}",
        row(
            &[
                "TOTAL".into(),
                total.5.to_string(),
                total.6.to_string(),
                total.7.to_string(),
                total.8.to_string(),
                average,
            ],
            &day_widths,
            &day_alignments,
            &[BOLD; 6],
        )
    );
    println!("{}", border('╰', '┴', '╯', &day_widths));
}
