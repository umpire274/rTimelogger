use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::del::DeleteLogic;
use crate::db::pool::DbPool;
use crate::errors::AppResult;
use crate::utils::date;
use std::io;
use std::io::Write;

/// Simple yes/no confirmation
fn ask_confirmation(prompt: &str) -> bool {
    print!("{prompt} [y/N]: ");
    let _ = io::stdout().flush();

    let mut s = String::new();
    if io::stdin().read_line(&mut s).is_ok() {
        matches!(s.trim().to_lowercase().as_str(), "y" | "yes")
    } else {
        false
    }
}

pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Del {
        pair,
        date: date_str,
    } = cmd
    {
        let d = date::parse_date(date_str)
            .ok_or_else(|| crate::errors::AppError::InvalidDate(date_str.into()))?;

        //
        // Conferma all’utente
        //
        if let Some(p) = pair {
            // deleting a specific pair
            let prompt = format!("⚠️  Delete pair #{p} for {d}? This cannot be undone.");
            if !ask_confirmation(&prompt) {
                println!("Operation cancelled.");
                return Ok(());
            }
        } else {
            // deleting whole day
            let prompt = format!("⚠️  Delete ALL events for {d}? This cannot be undone.");
            if !ask_confirmation(&prompt) {
                println!("Operation cancelled.");
                return Ok(());
            }
        }

        let mut pool = DbPool::new(&cfg.database)?;

        DeleteLogic::apply(&mut pool, d, *pair).expect("TODO: panic message");
    }

    Ok(())
}
