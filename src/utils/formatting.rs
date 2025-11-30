//! Formatting utilities used for CLI and export outputs.

pub fn bold(s: &str) -> String {
    format!("\x1b[1m{}\x1b[0m", s)
}

pub fn italic(s: &str) -> String {
    format!("\x1b[3m{}\x1b[0m", s)
}

pub fn pad_right(s: &str, width: usize) -> String {
    format!("{:<width$}", s, width = width)
}

pub fn pad_left(s: &str, width: usize) -> String {
    format!("{:>width$}", s, width = width)
}

pub fn mins2readable(mins: i64) -> String {
    let sign = if mins < 0 { "-" } else { "" };
    let m = mins.abs();
    format!("{}{:02}:{:02}", sign, m / 60, m % 60)
}
