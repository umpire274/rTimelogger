use crate::cli::parser::Commands;
use crate::core::add::AddLogic;
use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::models::location::Location;
use crate::utils::{date, time};

/// Add or update a work session.
pub fn handle(cmd: &Commands, cfg: &crate::config::Config) -> AppResult<()> {
    if let Commands::Add {
        date,
        pos_pos,
        start_pos,
        lunch_pos,
        end_pos,
        pos,
        start,
        lunch,
        end,
        edit_pair,
        edit,
    } = cmd
    {
        //
        // 1. Parse date (obbligatoria)
        //
        let d = date::parse_date(date).ok_or_else(|| AppError::InvalidDate(date.to_string()))?;

        //
        // 2. Determina posizione finale
        //
        let pos_raw = pos
            .as_ref()
            .map(|s| s.as_str())
            .or(pos_pos.as_ref().map(|s| s.as_str()));

        let pos_final = match pos_raw {
            Some(code) => Location::from_code(code)
                .ok_or_else(|| AppError::InvalidPosition(code.to_string()))?,
            None => Location::Office, // default se nulla è specificato
        };

        //
        // 3. Parse orario di IN
        //
        let start_raw = start
            .as_ref()
            .map(|s| s.as_str())
            .or(start_pos.as_ref().map(|s| s.as_str()));

        let start_parsed = match start_raw {
            Some(s) => {
                let t = time::parse_time(s).ok_or_else(|| AppError::InvalidTime(s.to_string()))?;
                Some(t)
            }
            None => None,
        };

        //
        // 4. Parse orario di OUT
        //
        let end_raw = end
            .as_ref()
            .map(|s| s.as_str())
            .or(end_pos.as_ref().map(|s| s.as_str()));

        let end_parsed = match end_raw {
            Some(s) => {
                let t = time::parse_time(s).ok_or_else(|| AppError::InvalidTime(s.to_string()))?;
                Some(t)
            }
            None => None,
        };

        //
        // 5. Pausa pranzo: qui vogliamo sapere se l'utente l'ha specificata davvero
        //
        let lunch_opt: Option<i32> = lunch.or(*lunch_pos);

        //
        // 6. Apri DB dal path configurato
        //
        let mut pool = DbPool::new(&cfg.database)?;

        //
        // 7. Delego la logica business
        //
        AddLogic::apply(
            &mut pool,
            d,
            pos_final,
            start_parsed,
            lunch_opt,
            end_parsed,
            *edit,
            *edit_pair,
            pos.clone(),
        )?;
    }

    Ok(())
}
