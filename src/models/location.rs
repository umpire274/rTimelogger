use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Location {
    Office,  // O
    Remote,  // R
    Holiday, // H
    OnSite,  // C (Customer)
    Mixed,   // M
}

impl Location {
    pub fn code(&self) -> &str {
        match self {
            Location::Office => "O",
            Location::Remote => "R",
            Location::Holiday => "H",
            Location::OnSite => "C",
            Location::Mixed => "M",
        }
    }

    /// Convert enum → DB string
    pub fn to_db_str(&self) -> &str {
        self.code()
    }

    /// Convert DB string → enum
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "O" => Some(Location::Office),
            "R" => Some(Location::Remote),
            "H" => Some(Location::Holiday),
            "C" => Some(Location::OnSite),
            "M" => Some(Location::Mixed),
            _ => None,
        }
    }

    /// Helper: convert input code from CLI (lowercase or uppercase)
    pub fn from_code(code: &str) -> Option<Self> {
        Location::from_db_str(&code.to_uppercase())
    }
}
