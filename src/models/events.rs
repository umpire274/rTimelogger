use chrono::{DateTime, Local};
use serde::Serialize;

use super::{event_type::EventType, location::Location};

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub id: i32,
    pub timestamp: DateTime<Local>,
    pub kind: EventType,
    pub location: Location,
    pub lunch: Option<i32>,
    pub work_gap: bool,
}
