use super::{event_type::EventType, location::Location};
use crate::db::pool::DbPool;
use chrono::{Local, NaiveDate, NaiveTime};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i32,
    pub date: NaiveDate,    // ⇔ events.date (TEXT "YYYY-MM-DD")
    pub time: NaiveTime,    // ⇔ events.time (TEXT "HH:MM")
    pub kind: EventType,    // ⇔ events.kind  ('in' | 'out')
    pub location: Location, // ⇔ events.position ('O','R','H','C','M')
    pub lunch: Option<i32>, // ⇔ events.lunch_break (INT, default 0)
    pub work_gap: bool,     // ⇔ events.meta/work_gap logica futura

    pub pair: i32,          // ⇔ events.pair (INT NOT NULL DEFAULT 0)
    pub source: String,     // ⇔ events.source (TEXT, default 'cli')
    pub meta: String,       // ⇔ events.meta (TEXT, default '')
    pub created_at: String, // ⇔ events.created_at (TEXT, ISO8601)
}

impl Event {
    /// Costruttore "di alto livello" per eventi creati dalla CLI.
    /// - Imposta `pair = 0` (sarà ricalcolato da recalc_all_pairs)
    /// - Imposta `source = "cli"`
    /// - Imposta `meta = ""`
    /// - Imposta `created_at = now() in ISO8601`
    pub fn new(
        id: i32,
        date: NaiveDate,
        time: NaiveTime,
        kind: EventType,
        location: Location,
        lunch: Option<i32>,
        work_gap: bool,
    ) -> Self {
        Self {
            id,
            date,
            time,
            kind,
            location,
            lunch,
            work_gap,
            pair: 0,
            source: "cli".to_string(),
            meta: String::new(),
            created_at: Local::now().to_rfc3339(),
        }
    }

    pub fn date_str(&self) -> String {
        self.date.format("%Y-%m-%d").to_string()
    }
    pub fn time_str(&self) -> String {
        self.time.format("%H:%M").to_string()
    }

    pub fn timestamp(&self) -> chrono::DateTime<Local> {
        let dt = self.date.and_time(self.time);
        // convert naive to Local
        dt.and_local_timezone(Local).unwrap()
    }

    pub fn get_date_time(&self) -> String {
        self.date
            .and_time(self.time)
            .format("%Y-%m-%d %H:%M")
            .to_string()
    }

    pub fn has_events_for_dates(pool: &mut DbPool, dates: &[NaiveDate]) -> rusqlite::Result<bool> {
        if dates.is_empty() {
            return Ok(false);
        }

        // Converti le date in stringhe "YYYY-MM-DD"
        let date_strings: Vec<String> = dates
            .iter()
            .map(|d| d.format("%Y-%m-%d").to_string())
            .collect();

        // Crea una lista di placeholder: ?, ?, ?, ...
        let placeholders = vec!["?"; date_strings.len()].join(",");

        // Query con IN (...)
        let sql = format!(
            "SELECT 1 FROM events WHERE date IN ({}) LIMIT 1",
            placeholders
        );

        // Converti in una lista di &dyn ToSql per rusqlite
        let params: Vec<&dyn rusqlite::ToSql> = date_strings
            .iter()
            .map(|s| s as &dyn rusqlite::ToSql)
            .collect();

        let exists = {
            let conn = &mut pool.conn;
            let mut stmt = conn.prepare(&sql)?;
            stmt.exists(rusqlite::params_from_iter(params))?
        };

        Ok(exists)
    }
}
