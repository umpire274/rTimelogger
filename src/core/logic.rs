use crate::config::Config;
use crate::core::calculator::{expected, surplus, timeline};
use crate::models::{day_summary::DaySummary, event::Event};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};

pub struct Core;

impl Core {
    pub fn build_daily_summary(events: &[Event], cfg: &Config) -> DaySummary {
        let timeline = timeline::build_timeline(events);

        // expected = minuti teorici da lavorare (da config)
        let expected = expected::calculate_expected(&timeline, cfg);

        // surplus = worked - expected
        let surplus = surplus::calculate_surplus(&timeline, expected);

        DaySummary {
            timeline,
            expected,
            surplus,
            gaps: Default::default(), // per future work_gap
        }
    }

    pub fn calculate_expected_exit(
        date: NaiveDate,   // aggiunto!
        time_in: &str,     // "HH:MM"
        work_minutes: i32, // minuti lavorativi da config
        lunch_total: i32,  // pausa totale effettiva
    ) -> NaiveDateTime {
        // 1. Parse time_in
        let (hours, minutes) = time_in
            .split_once(':')
            .map(|(h, m)| {
                let h = h.parse::<i32>().unwrap_or(0);
                let m = m.parse::<i32>().unwrap_or(0);
                (h, m)
            })
            .expect("Invalid time_in format");

        // 2. Convert IN → minuti dal giorno
        let start_total_min = hours * 60 + minutes;

        // 3. Calcola il totale minuti fine lavoro
        let exit_total_min = start_total_min + work_minutes + lunch_total;

        // 4. Calcolo ore/minuti con overflow oltre 24h gestito
        let exit_hours = (exit_total_min / 60) % 24;
        let exit_minutes = exit_total_min % 60;

        // 5. Parsing in NaiveTime
        let exit_time =
            NaiveTime::parse_from_str(&format!("{:02}:{:02}", exit_hours, exit_minutes), "%H:%M")
                .expect("Invalid generated time");

        // 6. Avanza la data se si supera mezzanotte
        let days_to_add = exit_total_min / (24 * 60);
        let final_date = date + chrono::Duration::days(days_to_add as i64);

        // 7. Crea il NaiveDateTime finale
        NaiveDateTime::new(final_date, exit_time)
    }

    /// Parsing minimale della durata lavoro dal config (es. "8h", "7h30", "08:00")
    pub fn parse_work_duration_to_minutes(s: &str) -> i64 {
        let s = s.trim();

        if s.is_empty() {
            return 8 * 60;
        }

        // Formati tipo "7h 36m", "7h36m", "7h", "7h 0m"
        if let Some(h_pos) = s.find('h') {
            let (h_part, rest) = s.split_at(h_pos);
            let hours: i64 = h_part.trim().parse().unwrap_or(8);

            let mut minutes: i64 = 0;
            let rest = rest[1..].trim(); // quello che viene dopo la 'h'

            if !rest.is_empty() {
                // Possibili formati di "rest":
                // "36m", "36", "36m qualcosa", "36 m"
                let rest_no_m = if let Some(m_pos) = rest.find('m') {
                    let (m_part, _) = rest.split_at(m_pos);
                    m_part.trim()
                } else {
                    rest
                };

                if !rest_no_m.is_empty() {
                    minutes = rest_no_m.parse::<i64>().unwrap_or(0);
                }
            }

            return hours * 60 + minutes;
        }

        // Formato "HH:MM"
        if let Some(colon_pos) = s.find(':') {
            let (h_part, m_part) = s.split_at(colon_pos);
            let hours: i64 = h_part.trim().parse().unwrap_or(8);
            let minutes: i64 = m_part[1..].trim().parse().unwrap_or(0);

            return hours * 60 + minutes;
        }

        // Solo minuti? Solo ore? Qui mantengo la tua logica: numero secco = ore
        if let Ok(h) = s.parse::<i64>() {
            let total = h * 60;
            return total;
        }

        // Fallback: 8h
        8 * 60
    }
}
