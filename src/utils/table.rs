//! Table rendering utilities for CLI outputs.

pub struct Column {
    pub header: String,
    pub width: usize,
}

pub struct Table {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<String>>,
}

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
