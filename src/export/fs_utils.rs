// src/export/fs_utils.rs

use crate::errors::{AppError, AppResult};
use crate::ui::messages::{info, warning};
use std::io::{self, Write};
use std::path::Path;

/// Verifica se un file può essere creato o sovrascritto.
///
/// - Se il file NON esiste → Ok
/// - Se esiste ed è abilitato `force` → Ok
/// - Se esiste e `force == false` → chiede conferma all'utente.
pub(crate) fn ensure_writable(path: &Path, force: bool) -> AppResult<()> {
    if !path.exists() || force {
        return Ok(());
    }

    warning(format!("The file '{}' already exists.", path.display()));

    print!("Overwrite? [y/N]: ");
    io::stdout().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).map_err(AppError::from)?;
    let ans = answer.trim().to_ascii_lowercase();

    if ans == "y" || ans == "yes" {
        info("Existing file will be overwritten.");
        Ok(())
    } else {
        Err(AppError::from(io::Error::other(
            "Export cancelled: existing file not overwritten",
        )))
    }
}
