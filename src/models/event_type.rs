use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum EventType {
    In,
    Out,
}

impl EventType {
    pub fn et_from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "in" => Some(Self::In),
            "out" => Some(Self::Out),
            _ => None,
        }
    }

    pub fn et_as_str(&self) -> &'static str {
        match self {
            EventType::In => "in",
            EventType::Out => "out",
        }
    }

    /// Convert enum → DB string
    pub fn to_db_str(&self) -> &'static str {
        match self {
            EventType::In => "in",
            EventType::Out => "out",
        }
    }

    /// Convert DB string → enum
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "in" => Some(EventType::In),
            "out" => Some(EventType::Out),
            _ => None,
        }
    }

    pub fn is_in(&self) -> bool {
        matches!(self, EventType::In)
    }

    pub fn is_out(&self) -> bool {
        matches!(self, EventType::Out)
    }
}
