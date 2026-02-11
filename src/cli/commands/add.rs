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
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
) -> Result<Option<(NaiveDate, NaiveDate)>, AppError> {
    match (from, to) {
        (Some(f), Some(t)) => {
            if pos != Location::SickLeave {
                return Err(AppError::InvalidArgs(
                    "--from/--to can only be used with --pos s".into(),
                ));
            }
            if f > t {
                return Err(AppError::InvalidDateRange { from: f, to: t });
            }
            Ok(Some((f, t)))
        }
        (None, None) => Ok(None),
        _ => Err(AppError::InvalidArgs(
            "Both --from and --to must be provided together.".into(),
        )),
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
        from,
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
        let sick_range = validate_sickleave_args(pos_final, *from, *to)?;

        match sick_range {
            Some((from_date, to_date)) => {
                let default_in = chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap();
                let default_out = chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap();

                // (opzionale ma consigliato) vieta start/end nel range malattia
                if start_parsed.is_some() || end_parsed.is_some() {
                    return Err(AppError::InvalidArgs(
                        "--start/--end cannot be used with --pos s (use only --from/--to)".into(),
                    ));
                }

                let s = start_parsed.unwrap_or(default_in);
                let e = end_parsed.unwrap_or(default_out);

                AddLogic::apply(
                    cfg,
                    &mut pool,
                    d,
                    pos_final,
                    Some(s),
                    lunch_opt,
                    work_gap,
                    Some(e),
                    *edit,
                    *edit_pair,
                    Some(from_date),
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
                    None,
                    pos.clone(),
                )?;
            }
        }
    }

    Ok(())
}
