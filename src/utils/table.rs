//! Table rendering utilities for CLI outputs.

pub struct Column {
    pub header: String,
    pub width: usize,
}

pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

pub const DAILY_TABLE_WIDTH: usize = 80;
pub const DAILY_TABLE_NO_WEEKDAY_WIDTH: usize = 74;
pub const DAILY_TABLE_WEEKDAYS_SHORT_WIDTH: usize = 79;
pub const DAILY_TABLE_WEEKDAYS_MEDIUM_WIDTH: usize = 80;
pub const DAILY_TABLE_WEEKDAYS_LONG_WIDTH: usize = 86;
pub const DAILY_TABLE_COMPACT_WIDTH: usize = 75;
pub const EVENTS_TABLE_WIDTH: usize = 88;

impl Table {
    pub fn new(columns: Vec<Column>) -> Self {
        Self {
            columns,
            rows: Vec::new(),
        }
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    pub fn render(&self) -> String {
        let mut out = String::new();

        // Header
        for col in &self.columns {
            out.push_str(&format!("{:<width$} ", col.header, width = col.width));
        }
        out.push('\n');

        // Rows
        for row in &self.rows {
            for (i, col) in self.columns.iter().enumerate() {
                out.push_str(&format!("{:<width$} ", row[i], width = col.width));
            }
            out.push('\n');
        }

        out
    }
}
