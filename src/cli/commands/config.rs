use crate::config::Config;
use crate::errors::AppResult;

use crate::cli::parser::Commands;
use std::process::Command;

/// Handle the `config` subcommand
pub fn handle(cmd: &Commands, cfg: &Config) -> AppResult<()> {
    if let Commands::Config {
        print_config,
        edit_config,
        editor,
    } = cmd
    {
        // Path del file di configurazione
        let path = Config::config_file();

        // ---- PRINT CONFIG ----
        if *print_config {
            println!("📄 Current configuration:\n");
            println!("{}", serde_yaml::to_string(&cfg).unwrap());
        }

        // ---- EDIT CONFIG ----
        if *edit_config {
            // User-requested editor (e.g. --editor vim)
            let requested_editor = editor.clone();

            // Default editor basato sulla piattaforma
            let default_editor = std::env::var("EDITOR")
                .or_else(|_| std::env::var("VISUAL"))
                .unwrap_or_else(|_| {
                    if cfg!(target_os = "windows") {
                        "notepad".to_string()
                    } else {
                        "nano".to_string()
                    }
                });

            // Se l’utente ha passato --editor, usiamo quello
            let editor_to_use = requested_editor.unwrap_or_else(|| default_editor.clone());

            // Primo tentativo: editor richiesto
            let status = Command::new(&editor_to_use).arg(&path).status();

            match status {
                Ok(s) if s.success() => {
                    println!(
                        "✅ Configuration file edited successfully using '{}'",
                        editor_to_use
                    );
                }
                Ok(_) | Err(_) => {
                    eprintln!(
                        "⚠️  Editor '{}' not available, falling back to '{}'",
                        editor_to_use, default_editor
                    );

                    // Fallback
                    let fallback_status = Command::new(&default_editor).arg(&path).status();
                    match fallback_status {
                        Ok(s) if s.success() => {
                            println!(
                                "✅ Configuration file edited successfully using fallback '{}'",
                                default_editor
                            );
                        }
                        Ok(_) | Err(_) => {
                            eprintln!(
                                "❌ Failed to edit configuration file using fallback '{}'",
                                default_editor
                            );
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
