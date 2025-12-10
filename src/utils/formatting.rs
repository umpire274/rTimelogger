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
        format!("{}{:02}:{:02}", sign, hours, minutes)
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
