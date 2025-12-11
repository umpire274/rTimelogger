use std::fmt;

/// ANSI colors
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";

const FG_BLUE: &str = "\x1b[34m";
const FG_GREEN: &str = "\x1b[32m";
const FG_YELLOW: &str = "\x1b[33m";
const FG_RED: &str = "\x1b[31m";

/// Icons
const ICON_INFO: &str = "ℹ️";
const ICON_OK: &str = "✅";
const ICON_WARN: &str = "⚠️";
const ICON_ERR: &str = "❌";

pub fn info<T: fmt::Display>(msg: T) {
    println!("{}{}{} {}{}", FG_BLUE, BOLD, ICON_INFO, RESET, msg);
}

pub fn success<T: fmt::Display>(msg: T) {
    println!("{}{}{} {}{}", FG_GREEN, BOLD, ICON_OK, RESET, msg);
}

pub fn warning<T: fmt::Display>(msg: T) {
    println!("{}{}{} {}{}", FG_YELLOW, BOLD, ICON_WARN, RESET, msg);
}

pub fn error<T: fmt::Display>(msg: T) {
    eprintln!("{}{}{} {}{}", FG_RED, BOLD, ICON_ERR, RESET, msg);
}

/// Optional: formatted section header
pub fn header<T: fmt::Display>(msg: T) {
    println!(
        "{}{}====================== {}\n{}",
        FG_BLUE, BOLD, msg, RESET
    );
}
