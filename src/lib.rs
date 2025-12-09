//! rTimeLogger library root.
//! Exposes CLI parser, high-level run() function, and internal modules.

pub mod cli;
pub mod config;
pub mod core;
pub mod db;
pub mod errors;
pub mod export;
pub mod models;
pub mod utils;

use clap::Parser;
use cli::parser::{Cli, Commands};
use config::Config;
use errors::AppResult;

/// Central command dispatcher
pub fn dispatch(cli: &Cli, cfg: &Config) -> AppResult<()> {
    match &cli.command {
        Commands::Init => cli::commands::init::handle(cli),
        Commands::Config { .. } => cli::commands::config::handle(&cli.command, cfg),
        Commands::Add { .. } => cli::commands::add::handle(&cli.command, cfg),
        Commands::List { .. } => cli::commands::list::handle(&cli.command, cfg),
        Commands::Del { .. } => cli::commands::del::handle(&cli.command, cfg),
        Commands::Backup { .. } => cli::commands::backup::handle(&cli.command, cfg),
        Commands::Log { .. } => cli::commands::log::handle(&cli.command, cfg),
        Commands::Export { .. } => cli::commands::export::handle(&cli.command, cfg),
    }
}

/// Entry point usato da main.rs
pub fn run() -> AppResult<()> {
    // 1️⃣ parse CLI
    let cli = Cli::parse();

    // 2️⃣ carica config UNA sola volta
    let mut cfg = Config::load();

    // 3️⃣ applica eventuale override del DB da riga di comando
    if let Some(custom_db) = &cli.db {
        cfg.database = custom_db.clone();
    }

    // (per ora `cli.test` lo ignoriamo qui; lo usi solo dove serve davvero)

    // 4️⃣ passa tutto al dispatcher
    dispatch(&cli, &cfg)
}
