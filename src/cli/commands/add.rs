use crate::cli::parser::Commands;
use crate::core::add::AddLogic;
use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::location::Location;
use crate::utils::date;
use crate::utils::time::parse_optional_time;

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
    } = cmd
    {
        //
        // 1. Parse date (mandatory)
        //
        let d = date::parse_date(date).ok_or_else(|| AppError::InvalidDate(date.to_string()))?;

        //
        // 2. Parse position (default = Office)
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
        // 3. Parse IN time (optional)
        //
        let start_parsed = parse_optional_time(start.as_ref());

        //
        // 4. Parse OUT time (optional)
        //
        let end_parsed = parse_optional_time(end.as_ref());

        //
        // 5. Parse lunch break (optional)
        //
        let lunch_opt = *lunch; // lunch è Option<i32>

        //
        // 6. Open DB
        //
        let mut pool = DbPool::new(&cfg.database)?;

        let work_gap: Option<bool> = if *work_gap {
            Some(true)
        } else if *no_work_gap {
            Some(false)
        } else {
            None
        };

        //
        // 7. Execute logic
        //
        AddLogic::apply(
            &mut pool,
            d,
            pos_final,
            start_parsed.unwrap(),
            lunch_opt,
            work_gap,
            end_parsed.unwrap(),
            *edit,
            *edit_pair,
            pos.clone(), // used for audit logging
        )?;
    }

    Ok(())
}
