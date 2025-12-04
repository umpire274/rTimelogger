use clap::{Parser, Subcommand};

/// Command-line interface definition for rTimelogger
/// CLI application to track working hours with SQLite
#[derive(Parser)]
#[command(
    name = "rtimelogger",
    version = env!("CARGO_PKG_VERSION"),
    about = "A simple time logging CLI: track working hours and calculate surplus using SQLite",
    long_about = None
)]
pub struct Cli {
    /// Override database path (useful for tests or custom DB)
    #[arg(global = true, long = "db")]
    pub db: Option<String>,

    /// Run in test mode (no config file update)
    #[arg(global = true, long = "test", hide = true)]
    pub test: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize the database and configuration
    Init,

    /// Manage the configuration file (view or edit)
    Config {
        #[arg(long = "print", help = "Print the current configuration file")]
        print_config: bool,

        #[arg(
            long = "edit",
            help = "Edit the configuration file (default editor: $EDITOR, or nano/vim/notepad)"
        )]
        edit_config: bool,

        #[arg(
            long = "editor",
            help = "Specify the editor to use (vim, nano, or custom path)"
        )]
        editor: Option<String>,
    },

    /// Print or manage the internal log table
    Log {
        #[arg(long = "print", help = "Print rows from the internal log table")]
        print: bool,
    },

    /// Add or update a work session
    Add {
        date: String,
        pos_pos: Option<String>,
        start_pos: Option<String>,
        lunch_pos: Option<i32>,
        end_pos: Option<String>,

        #[arg(long = "pos")]
        pos: Option<String>,
        #[arg(long = "in")]
        start: Option<String>,
        #[arg(long = "lunch")]
        lunch: Option<i32>,
        #[arg(long = "out")]
        end: Option<String>,

        #[arg(long = "pair", help = "Pair id to edit (with --edit)")]
        edit_pair: Option<usize>,

        #[arg(long = "edit", help = "Edit existing pair (use with --pair)")]
        edit: bool,
    },

    /// Delete a work session by ID
    Del {
        #[arg(long = "pair", help = "Pair id to delete for the given date")]
        pair: Option<usize>,

        date: String,
    },

    /// List sessions
    List {
        #[arg(long, short, help = "Filter by year/month/day or a custom range")]
        period: Option<String>,

        #[arg(long)]
        pos: Option<String>,

        #[arg(long = "today", help = "Show only today's record")]
        now: bool,

        #[arg(long = "details", help = "Show today's detailed events")]
        details: bool,

        #[arg(long = "events", help = "List all events (in/out)")]
        events: bool,

        #[arg(long = "pairs", help = "Filter by pair id (only with --events)")]
        pairs: Option<usize>,

        #[arg(long = "summary", help = "Show summarized per-pair rows")]
        summary: bool,
    },

    /// Create a backup copy of the database
    Backup {
        #[arg(long, value_name = "FILE")]
        file: String,

        #[arg(long)]
        compress: bool,
    },

    /// Export work session data
    Export {
        #[arg(long, value_name = "FORMAT", default_value = "csv")]
        format: String,

        #[arg(long, value_name = "FILE")]
        file: String,

        #[arg(
            long,
            value_name = "RANGE",
            help = "Filter export by year/month/day or a custom range"
        )]
        range: Option<String>,

        #[arg(long, short = 'e')]
        events: bool,

        #[arg(long, short = 'f')]
        force: bool,
    },
}
