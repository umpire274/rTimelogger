use crate::models::event::Event;
use csv::Writer;

/// Scrive gli eventi in CSV nel file indicato.
pub fn write_csv(path: &str, events: &[Event]) -> std::io::Result<()> {
    let mut wtr = Writer::from_path(path)?;

    wtr.write_record(["timestamp", "kind", "position", "lunch"])?;

    for ev in events {
        wtr.write_record(&[
            ev.timestamp().to_rfc3339(),
            ev.kind.et_as_str().to_string(),
            ev.location.code().to_string(),
            ev.lunch.unwrap_or(0).to_string(),
        ])?;
    }

    wtr.flush()?;
    Ok(())
}
