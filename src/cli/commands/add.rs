use crate::cli::parser::Commands;
use crate::core::add::AddLogic;
use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::location::Location;
use crate::utils::date;
use crate::utils::time::parse_optional_time;
use chrono::NaiveDate;

fn validate_sickleave_args(
    pos: Location,
    date: Option<NaiveDate>, // di fatto: Some(d)
    to: Option<NaiveDate>,
) -> Result<Option<(NaiveDate, NaiveDate)>, AppError> {
    let d = match date {
        Some(d) => d,
        None => return Ok(None), // difensivo, ma nel tuo handle passi sempre Some(d)
    };

    match pos {
        Location::SickLeave => {
            let t = to.unwrap_or(d);
            if d > t {
                return Err(AppError::InvalidDateRange { from: d, to: t });
            }
            Ok(Some((d, t)))
        }
        _ => {
            if to.is_some() {
                return Err(AppError::InvalidArgs(
                    "--to can only be used with --pos s".into(),
                ));
            }
            Ok(None)
        }
    }
}

/// Add or update a work session.
pub fn handle(cmd: &Commands, cfg: &crate::config::Config) -> AppResult<()> {
    if let Commands::Add {
        date,
        pos,
        start,
        lunch,
        work_gap,
        no_work_gap,
        end,
        edit_pair,
        edit,
        to,
    } = cmd
    {
        //
        // 1. Parse position (default = Office)
        //
        let pos_final = match pos {
            Some(code) => Location::from_code(code).ok_or_else(|| {
                AppError::InvalidPosition(format!(
                    "Invalid location code '{}'. Use a valid code such as 'office', 'remote', 'customer', ...",
                    code
                ))
            })?,
            None => Location::Office,
        };

        //
        // 2. Parse date (mandatory for normal ADD)
        //    (per SickLeave puoi anche ignorarla, ma se CLI la richiede, la parse qui va bene)
        //
        let d = date::parse_date(date).map_err(|_| AppError::InvalidDate(date.to_string()))?;

        //
        // 3. Parse times (optional input)
        //
        let start_parsed = parse_optional_time(start.as_ref())?;

        //
        // 4. Parse OUT time (optional)
        //
        let end_parsed = parse_optional_time(end.as_ref())?;

        //
        // 4. Lunch break (optional)
        //
        let lunch_opt = *lunch;

        //
        // 5. Open DB
        //
        let mut pool = DbPool::new(&cfg.database)?;

        //
        // 6. work_gap flag
        //
        let work_gap: Option<bool> = if *work_gap {
            Some(true)
        } else if *no_work_gap {
            Some(false)
        } else {
            None
        };

        //
        // 7. SickLeave range validation (only if pos == SickLeave or from/to used)
        //
        let sick_range = validate_sickleave_args(pos_final, Some(d), *to)?;

        match sick_range {
            Some((_from_date, to_date)) => {
                // (opzionale ma consigliato) vieta start/end nel range malattia
                if start_parsed.is_some() || end_parsed.is_some() {
                    return Err(AppError::InvalidArgs(
                        "--in/--out cannot be used with --pos s (use only --to)".into(),
                    ));
                }

                AddLogic::apply(
                    cfg,
                    &mut pool,
                    d,
                    pos_final,
                    None,
                    None,
                    None,
                    None,
                    *edit,
                    *edit_pair,
                    Some(to_date),
                    pos.clone(),
                )?;
            }
            None => {
                AddLogic::apply(
                    cfg,
                    &mut pool,
                    d,
                    pos_final,
                    start_parsed,
                    lunch_opt,
                    work_gap,
                    end_parsed,
                    *edit,
                    *edit_pair,
                    None,
                    pos.clone(),
                )?;
            }
        }
    }

    Ok(())
}
