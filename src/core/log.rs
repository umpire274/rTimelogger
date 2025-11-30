use crate::config::Config;
use crate::db::pool::DbPool;
use crate::errors::AppResult;
use ansi_term::Colour;

fn strip_ansi(s: &str) -> String {
    let re = regex::Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
    re.replace_all(s, "").into_owned()
}

/// Restituisce il colore ANSI in base all'operazione
fn color_for_operation(op: &str) -> Colour {
    match op {
        "add" => Colour::Green,
        "del" => Colour::Red,
        "edit" => Colour::Yellow,
        "migration_applied" => Colour::Purple,
        "backup" => Colour::Blue,
        "init" => Colour::RGB(255, 153, 51), // arancione
        other if other.starts_with("migrate_to_") => Colour::Purple,
        _ => Colour::White,
    }
}

pub struct LogLogic;

impl LogLogic {
    pub fn print_log(pool: &mut DbPool, _cfg: &Config) -> AppResult<()> {
        let mut stmt = pool.conn.prepare_cached(
            "SELECT id, date, operation, target, message FROM log ORDER BY id ASC",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: i32 = row.get(0)?;
            let raw_date: String = row.get(1)?;
            let operation: String = row.get(2)?;
            let target: String = row.get(3)?;
            let message: String = row.get(4)?;

            let date = chrono::DateTime::parse_from_rfc3339(&raw_date)
                .map(|dt| dt.format("%FT%T%:z").to_string())
                .unwrap_or(raw_date);

            // Unica colonna op+target
            let op_target = if target.is_empty() {
                operation.clone()
            } else {
                format!("{operation} ({target})")
            };

            Ok((id, date, operation, op_target, message))
        })?;

        let mut entries = Vec::new();
        for r in rows {
            entries.push(r?);
        }

        // Calcoliamo larghezza max ma con limite a 60
        let raw_max = entries
            .iter()
            .map(|(_, _, _, op_target, _)| op_target.len())
            .max()
            .unwrap_or(10);

        let op_w = raw_max.min(60);

        let id_w = entries
            .iter()
            .map(|(id, _, _, _, _)| id.to_string().len())
            .max()
            .unwrap();
        let date_w = entries
            .iter()
            .map(|(_, date, _, _, _)| date.len())
            .max()
            .unwrap();

        println!("📜 Internal log:\n");

        for (id, date, operation_raw, op_target, message) in entries {
            let color = color_for_operation(&operation_raw);

            // separa operation da target
            let (op, rest) = if let Some((op_part, rest)) = op_target.split_once(' ') {
                (op_part.to_string(), Some(rest.to_string()))
            } else {
                (op_target.clone(), None)
            };

            // parte colorata
            let mut colored = color.paint(op).to_string();
            if let Some(r) = rest {
                colored.push(' ');
                colored.push_str(&r);
            }

            // --- TRUNCATE a 60 caratteri SENZA ANSI ---
            let visible = strip_ansi(&colored);
            let truncated_visible = if visible.len() > 60 {
                // taglio a 57 + "..."
                let mut s = visible.chars().take(57).collect::<String>();
                s.push_str("...");
                s
            } else {
                visible.clone()
            };

            // ricostruzione con ANSI (solo l'op rimane colorato)
            // => dobbiamo ricolorare solo la prima parola
            let recolored = {
                if let Some((op_word, rest)) = truncated_visible.split_once(' ') {
                    format!("{} {}", color.paint(op_word), rest)
                } else {
                    color.paint(truncated_visible.as_str()).to_string()
                }
            };

            // padding (calcolato sulle dimensioni reali SENZA ANSI)
            let padding = " ".repeat(op_w.saturating_sub(strip_ansi(&recolored).len()));

            println!(
                "{:>id_w$}: {:<date_w$} | {}{} => {}",
                id,
                date,
                recolored,
                padding,
                message,
                id_w = id_w,
                date_w = date_w
            );
        }

        Ok(())
    }
}
