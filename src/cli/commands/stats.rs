use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::stats::calculate;
use crate::db::pool::DbPool;
use crate::db::queries::events::load_events_by_range;
use crate::errors::{AppError, AppResult};
use crate::models::work_stats::WorkStats;
use crate::utils::time::format_minutes;

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

    println!("WORK STATISTICS");
    println!("Period: {} -> {}", from, to);
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

fn print_table(rows: &[WorkStats]) {
    println!(
        "{:<9} | {:>8} | {:>8} | {:>10} | {:>10} | {:>11} | {:>10} | {:>9} | {:>8} | {:>15}",
        "Period",
        "Expected",
        "Worked",
        "Paid Leave",
        "Sick Leave",
        "Nat. Holiday",
        "Recognized",
        "Balance",
        "Avg/day",
        "Days W/P/S/N"
    );
    println!("{}", "-".repeat(125));

    for row in rows {
        println!(
            "{:<9} | {:>8} | {:>8} | {:>10} | {:>10} | {:>11} | {:>10} | {:>9} | {:>8} | {:>15}",
            row.label,
            duration(row.expected_minutes),
            duration(row.worked_minutes),
            duration(row.paid_leave_minutes),
            duration(row.sick_leave_minutes),
            duration(row.national_holiday_minutes),
            duration(row.recognized_minutes()),
            signed_duration(row.balance_minutes()),
            row.average_worked_minutes()
                .map(duration)
                .unwrap_or_else(|| "-".into()),
            format!(
                "{}/{}/{}/{}",
                row.worked_days,
                row.paid_leave_days,
                row.sick_leave_days,
                row.national_holiday_days
            ),
        );
    }

    println!("{}", "-".repeat(125));
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
    println!(
        "{:<9} | {:>8} | {:>8} | {:>10} | {:>10} | {:>11} | {:>10} | {:>9} | {:>8} | {:>15}",
        "TOTAL",
        duration(total.0),
        duration(total.1),
        duration(total.2),
        duration(total.3),
        duration(total.4),
        duration(recognized),
        signed_duration(recognized - total.0),
        average,
        format!("{}/{}/{}/{}", total.5, total.6, total.7, total.8)
    );
    println!("\nDays: W=Worked, P=Paid Leave, S=Sick Leave, N=National Holiday");
}
