use crate::cli::parser::Commands;
use crate::config::{Config, migrate};
use crate::errors::{AppError, AppResult};
use crate::ui::messages::{error, info, success, warning};

use std::process::Command;

/// Handle the `config` subcommand
pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Config {
        print_config,
        check,
        migrate,
        edit_config,
        editor,
    } = cmd
    {
        let path = Config::config_file();

        // ------------------------------------------------------------
        // PRINT CONFIG
        // ------------------------------------------------------------
        if *print_config {
            info("Current configuration:");
            println!("{}", serde_yaml::to_string(&cfg).unwrap());
        }

        // ------------------------------------------------------------
        // CHECK CONFIG
        // ------------------------------------------------------------
        if *check {
            info("🔧 Checking configuration…");

            let cfg = Config::load();

            info(format!("Config file: {:?}", Config::config_file()));
            info(format!("Database   : {:?}", cfg.database));

            let db_exists = std::path::Path::new(&cfg.database).exists();

            if !db_exists {
                warning("⚠ Database file is missing.");
            } else {
                success("✔ Database file exists.");
            }

            // qui puoi aggiungere altre verifiche, tipo valori malformati ecc.

            return Ok(());
        }

        // ------------------------------------------------------------
        // MIGRATE CONFIG
        // ------------------------------------------------------------
        if *migrate {
            info("🔧 Running configuration migration…");

            match migrate::run_fs_migration() {
                Ok(_) => success("✔ Filesystem migration completed."),
                Err(e) => error(format!("Migration error: {}", e)),
            }

            return Ok(());
        }

        // ------------------------------------------------------------
        // EDIT CONFIG
        // ------------------------------------------------------------
        if *edit_config {
            // Requested editor via --editor
            let requested_editor = editor.clone();

            // Determine default editor
            let default_editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    if cfg!(target_os = "windows") {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });

            // If --editor supplied → use it, otherwise fallback to default
            let editor_to_use = requested_editor.unwrap_or_else(|| default_editor.clone());

            info(format!(
                "Opening configuration file with editor '{}'",
                editor_to_use
            ));

            // Try primary editor
            let status = Command::new(&editor_to_use).arg(&path).status();

            match status {
                Ok(s) if s.success() => {
                    success(format!(
                        "Configuration file edited successfully using '{}'.",
                        editor_to_use
                    ));
                }

                // Editor not usable → fallback
                Ok(_) | Err(_) => {
                    warning(format!(
                        "Editor '{}' not available or failed to start. Falling back to '{}'.",
                        editor_to_use, default_editor
                    ));

                    let fallback_status = Command::new(&default_editor).arg(&path).status();

                    match fallback_status {
                        Ok(s) if s.success() => {
                            success(format!(
                                "Configuration file edited successfully using fallback editor '{}'.",
                                default_editor
                            ));
                        }

                        Ok(_) | Err(_) => {
                            return Err(AppError::InvalidOperation(format!(
                                "Unable to edit configuration file.\nAttempted editors:\n  • Primary: '{}'\n  • Fallback: '{}'\nBoth failed to start or exited with an error.",
                                editor_to_use, default_editor
                            )));
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
