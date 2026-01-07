//! Formatting utilities used for CLI and export outputs.

pub const FOOTER_INDENT: usize = 75;

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

pub fn mins2readable(mins: i64, want_sign: bool, short: bool) -> String {
    let abs_m = mins.abs();
    let hours = abs_m / 60;
    let minutes = abs_m % 60;

    // NEW: aggiunta del segno "+" per i valori positivi
    let sign = if mins > 0 && want_sign {
        "+"
    } else if mins < 0 && want_sign {
        "-"
    } else {
        "" // zero → nessun segno
    };

    if short {
        // es: +02:25 oppure -01:10
        format!("{}{:02}h{:02}m", sign, hours, minutes)
    } else {
        // es: +02h 25m oppure -01h 10m
        format!("{}{:02}h {:02}m", sign, hours, minutes)
    }
}

/// Restituisce una descrizione testuale e un colore ANSI per la posizione.
/// Usata nei test e in eventuali output human-readable.
pub fn describe_position(code: &str) -> (String, &'static str) {
    match code.to_uppercase().as_str() {
        "O" => ("Office".into(), "\x1b[34m"),
        "R" => ("Remote".into(), "\x1b[36m"),
        "C" => ("On-site (Client)".into(), "\x1b[33m"),
        "H" => ("Holiday".into(), "\x1b[45;97;1m"),
        "M" => ("Mixed".into(), "\x1b[35m"),
        other => (other.to_string(), "\x1b[0m"),
    }
}

/// Returns the visible length of a string by ignoring ANSI escape sequences.
pub fn visible_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    let mut i = 0usize;
    let mut count = 0usize;

    while i < bytes.len() {
        // ANSI escape starts with ESC [
        if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2; // skip ESC[
            // skip until a final byte in the range @..~ (CSI terminator). Most common is 'm'.
            while i < bytes.len() {
                let b = bytes[i];
                i += 1;
                if (0x40..=0x7E).contains(&b) {
                    break;
                }
            }
        } else {
            // Count UTF-8 chars properly
            let ch = s[i..].chars().next().unwrap();
            count += 1;
            i += ch.len_utf8();
        }
    }

    count
}

/// Pads the given visible text to the right edge of a fixed-width box (table).
/// `box_width` is the target visible width.
/// Returns a string of spaces to prefix.
pub fn right_pad_prefix(box_width: usize, visible_text: &str) -> String {
    let len = visible_len(visible_text);
    let pad = box_width.saturating_sub(len);
    " ".repeat(pad)
}
