/// ANSI color helper utilities for terminal output.
pub const RESET: &str = "\x1b[0m";

pub const GREY: &str = "\x1b[90m";
pub const WHITE: &str = "\x1b[37m";
pub const BLACK: &str = "\x1b[40m";

pub const RED: &str = "\x1b[31m";
pub const GREEN: &str = "\x1b[32m";

pub const YELLOW: &str = "\x1b[33m";
pub const BLUE: &str = "\x1b[34m";
pub const CYAN: &str = "\x1b[36m";
pub const MAGENTA: &str = "\x1b[35m";

/// Returns GREY when the field is empty (None or "" or "--:--"),
/// and RESET otherwise.
pub fn color_for_optional_field<T: AsRef<str>>(value: Option<T>) -> &'static str {
    match value {
        Some(v) if !v.as_ref().trim().is_empty() && v.as_ref() != "--:--" => RESET,
        _ => GREY,
    }
}

/// Surplus color:
/// \>0 → green
/// \<0 → red
/// 0 → reset
pub fn color_for_surplus(value: i64) -> &'static str {
    if value > 0 {
        GREEN
    } else if value < 0 {
        RED
    } else {
        RESET
    }
}

/// Ritorna formattazione colorata di un valore opzionale.
///
/// Esempio:
/// `colorize_optional("--:--")` → "<grey>--:--<reset>"
pub fn colorize_optional(value: &str) -> String {
    if value.trim().is_empty()
        || value.trim() == "--:--"
        || value.trim() == "00h 00m"
        || value.trim() == "0 min"
    {
        format!("{GREY}{value}{RESET}")
    } else {
        value.to_string()
    }
}

pub fn colorize_in_out(value: &str, is_in: bool) -> String {
    if value.trim().is_empty()
        || value.trim() == "--:--"
        || value.trim() == "00h 00m"
        || value.trim() == "0 min"
    {
        return format!("{GREY}{value}{RESET}");
    }

    if is_in {
        format!("{GREEN}{value}{RESET}")
    } else {
        format!("{RED}{value}{RESET}")
    }
}
