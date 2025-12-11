use crate::cli::parser::Commands;
use crate::config::Config;
use crate::core::del::DeleteLogic;
use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::ui::messages::{info, success, warning};
use crate::utils::date;

use std::io::{self, Write};

/// Ask a yes/no confirmation from the user
fn ask_confirmation(prompt: &str) -> bool {
    warning(prompt);
    print!("Confirm [y/N]: ");
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
        let d = date::parse_date(date_str).ok_or_else(|| AppError::InvalidDate(date_str.into()))?;

        //
        // Confirmation prompt
        //
        let prompt = if let Some(p) = pair {
            format!("Delete pair #{} for {}? This action is irreversible.", p, d)
        } else {
            format!("Delete ALL events for {}? This action is irreversible.", d)
        };

        if !ask_confirmation(&prompt) {
            info("Operation cancelled.");
            return Ok(());
        }

        //
        // Execute deletion
        //
        let mut pool = DbPool::new(&cfg.database)?;

        match DeleteLogic::apply(&mut pool, d, *pair) {
            Ok(_) => {
                if let Some(p) = pair {
                    success(format!("Pair #{} for {} has been deleted.", p, d));
                } else {
                    success(format!("All events for {} have been deleted.", d));
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(())
}
