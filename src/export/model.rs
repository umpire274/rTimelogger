// src/export/model.rs

use serde::Serialize;

/// Struttura “piatta” per export degli eventi.
#[derive(Serialize, Clone, Debug)]
pub struct EventExport {
    pub id: i32,
    pub date: String,
    pub time: String,
    pub kind: String,
    pub position: String,
    pub lunch_break: i32,
    pub pair: i32,
    pub source: String,
}

/// Header per CSV / JSON / XLSX / PDF
pub(crate) fn get_headers() -> Vec<&'static str> {
    vec![
        "id",
        "date",
        "time",
        "kind",
        "position",
        "lunch_break",
        "pair",
        "source",
    ]
}

/// Convert events in una tabella di stringhe (per PDF).
pub(crate) fn event_to_row(e: &EventExport) -> Vec<String> {
    vec![
        e.id.to_string(),
        e.date.clone(),
        e.time.clone(),
        e.kind.clone(),
        e.position.clone(),
        e.lunch_break.to_string(),
        e.pair.to_string(),
        e.source.clone(),
    ]
}

pub(crate) fn events_to_table(events: &[EventExport]) -> Vec<Vec<String>> {
    events.iter().map(event_to_row).collect()
}
