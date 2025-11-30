use crate::models::event::Event;

/// Scrive gli eventi in JSON formattato.
pub fn write_json(path: &str, events: &[Event]) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(events).unwrap();
    std::fs::write(path, json)
}
