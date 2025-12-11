// src/export/xlsx.rs

use crate::errors::{AppError, AppResult};
use crate::export::excel_date::parse_to_excel_date;
use crate::export::model::{event_to_row, get_headers};
use crate::export::{EventExport, notify_export_success};
use crate::ui::messages::info;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, FormatPattern, Workbook};
use std::io;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

/// Export XLSX con styling e auto-larghezza colonne.
pub(crate) fn export_xlsx(events: &[EventExport], path: &Path) -> AppResult<()> {
    info(format!("Exporting to XLSX: {}", path.display()));

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // ---------------------------
    // Caso dataset vuoto
    // ---------------------------
    if events.is_empty() {
        worksheet
            .write(0, 0, "No data available")
            .map_err(to_io_app_error)?;
        workbook.save(path_str(path)?).map_err(to_io_app_error)?;
        notify_export_success("XLSX (empty dataset)", path);
        return Ok(());
    }

    // ---------------------------
    // Header
    // ---------------------------
    let headers = get_headers();

    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x2F75B5))
        .set_pattern(FormatPattern::Solid)
        .set_border(FormatBorder::Thin);

    for (col, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0, col as u16, *header, &header_format)
            .map_err(to_io_app_error)?;
    }

    worksheet.set_freeze_panes(1, 0).ok();

    // ---------------------------
    // Calcolo larghezze colonne
    // ---------------------------
    let mut col_widths: Vec<usize> = headers.iter().map(|h| UnicodeWidthStr::width(*h)).collect();

    let band1 = Color::RGB(0xEAF3FB);
    let band2 = Color::RGB(0xFFFFFF);
    let num_align = FormatAlign::Right;

    // ---------------------------
    // Scrittura righe
    // ---------------------------
    for (row_index, ev) in events.iter().enumerate() {
        let row = (row_index + 1) as u32;
        let band_color = if row_index % 2 == 0 { band1 } else { band2 };

        // campi in ordine
        let values = event_to_row(ev);

        for (col, value) in values.iter().enumerate() {
            let v = value.as_str();

            write_xlsx_cell(worksheet, row, col as u16, v, band_color, num_align)?;

            col_widths[col] = col_widths[col].max(UnicodeWidthStr::width(v));
        }
    }

    // ---------------------------
    // Set column widths
    // ---------------------------
    for (c, w) in col_widths.iter().enumerate() {
        worksheet
            .set_column_width(c as u16, *w as f64 + 2.0)
            .map_err(to_io_app_error)?;
    }

    workbook.save(path_str(path)?).map_err(to_io_app_error)?;

    notify_export_success("XLSX", path);
    Ok(())
}

/// Scrive una singola cella, interpretando stringhe come data/ora/numero se possibile.
fn write_xlsx_cell(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    s: &str,
    bg: Color,
    num_align: FormatAlign,
) -> AppResult<()> {
    // Data / Ora in formato Excel
    if let Some((num_format, serial)) = parse_to_excel_date(s) {
        let fmt = Format::new()
            .set_num_format(num_format)
            .set_background_color(bg)
            .set_pattern(FormatPattern::Solid)
            .set_border(FormatBorder::Thin);

        worksheet
            .write_with_format(row, col, serial, &fmt)
            .map_err(to_io_app_error)?;
        return Ok(());
    }

    // Numero generico
    if let Ok(num) = s.parse::<f64>() {
        let fmt = Format::new()
            .set_align(num_align)
            .set_background_color(bg)
            .set_pattern(FormatPattern::Solid)
            .set_border(FormatBorder::Thin);

        worksheet
            .write_with_format(row, col, num, &fmt)
            .map_err(to_io_app_error)?;
        return Ok(());
    }

    // Testo
    let fmt = Format::new()
        .set_background_color(bg)
        .set_pattern(FormatPattern::Solid)
        .set_border(FormatBorder::Thin);

    worksheet
        .write_with_format(row, col, s, &fmt)
        .map_err(to_io_app_error)?;

    Ok(())
}

fn to_io_app_error<E: std::fmt::Display>(e: E) -> AppError {
    AppError::from(io::Error::other(e.to_string()))
}

fn path_str(path: &Path) -> AppResult<&str> {
    path.to_str()
        .ok_or_else(|| AppError::from(io::Error::other("invalid path")))
}
