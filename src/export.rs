use crate::cli::Commands;
use crate::db;
use crate::pdf_manager::PdfManager;
use crate::utils::mins2readable;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use rusqlite::Connection;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, FormatPattern, Workbook};
use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use unicode_width::UnicodeWidthStr;

#[derive(Serialize, Clone, Debug)]
struct EventExport {
    id: i32,
    date: String,
    time: String,
    kind: String,
    position: String,
    lunch_break: i32,
    pair: i32,
    source: String,
}

#[derive(Serialize, Clone, Debug)]
struct SessionExport {
    id: i32,
    date: String,
    position: String,
    start: String,
    lunch_break: i32,
    end: String,
    work_duration: Option<String>,
}

fn get_headers(export_events: bool) -> Vec<&'static str> {
    if export_events {
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
    } else {
        vec![
            "id",
            "date",
            "position",
            "start_time",
            "lunch_break",
            "end_time",
            "work_duration",
        ]
    }
}

/// Convert events into &[Vec<String>]
fn events_to_table(events: &[EventExport]) -> Vec<Vec<String>> {
    events
        .iter()
        .map(|e| {
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
        })
        .collect()
}

/// Convert sessions into &[Vec<String>]
fn sessions_to_table(sessions: &[SessionExport]) -> Vec<Vec<String>> {
    sessions
        .iter()
        .map(|s| {
            vec![
                s.id.to_string(),
                s.date.clone(),
                s.position.clone(),
                s.start.clone(),
                s.lunch_break.to_string(),
                s.end.clone(),
                s.work_duration.clone().unwrap_or_default(),
            ]
        })
        .collect()
}

fn export_to_format<T: serde::Serialize + std::fmt::Debug>(
    fmt: &str,
    data: &[T],
    path: &Path,
    export_events: bool,
) -> Result<(), Box<dyn Error>> {
    match fmt {
        "csv" => export_csv(data, path)?,
        "json" => export_json(data, path)?,
        "xlsx" => export_xlsx(data, path)?,
        "pdf" => export_pdf(data, path, export_events)?,
        _ => unreachable!(),
    }
    Ok(())
}

/// Main export handler
pub fn handle_export(cmd: &Commands, conn: &Connection) -> Result<(), Box<dyn Error>> {
    if let Commands::Export {
        format,
        file,
        range,
        events,
        sessions,
        force,
    } = cmd
    {
        // Validate format
        let fmt = format.to_lowercase();
        if !["csv", "json", "xlsx", "pdf"].contains(&fmt.as_str()) {
            eprintln!("❌ Unsupported format '{}'. Use one of: csv, json", format);
            std::process::exit(1);
        }

        // Check output file path is absolute
        let path = Path::new(file);
        if !path.is_absolute() {
            eprintln!("❌ Output file path must be absolute: {}", file);
            std::process::exit(1);
        }

        // new check
        ensure_writable(path, *force)?;

        // Interpret --range all → no date filtering
        let date_bounds: Option<(String, String)> = if let Some(r) = range.as_deref() {
            if r.eq_ignore_ascii_case("all") {
                None
            } else {
                Some(parse_range(r).map_err(|e| format!("invalid --range: {e}"))?)
            }
        } else {
            None
        };

        // dataset selection (default: events)
        let export_events = if *events { true } else { !(*sessions) };

        if export_events {
            let data = load_events(conn, date_bounds)?;
            export_to_format(&fmt, &data, path, export_events)?;
        } else {
            let data = load_sessions(conn, date_bounds)?;
            export_to_format(&fmt, &data, path, export_events)?;
        }
    }

    Ok(())
}

fn build_query_with_range(
    base_select: &str,
    bounds: Option<(String, String)>,
    order_clause: &str,
) -> (String, Vec<String>) {
    let mut sql = String::from(base_select);
    let mut owned_params: Vec<String> = Vec::new();
    if let Some((start, end)) = bounds {
        sql.push_str(" WHERE date BETWEEN ?1 AND ?2");
        owned_params.push(start);
        owned_params.push(end);
    }
    sql.push_str(order_clause);
    (sql, owned_params)
}

fn load_events(
    conn: &Connection,
    bounds: Option<(String, String)>,
) -> rusqlite::Result<Vec<EventExport>> {
    let (sql, owned_params) = build_query_with_range(
        r#"
        SELECT id, date, time, kind, position, lunch_break, pair, source, meta, created_at
        FROM events
        "#,
        bounds,
        " ORDER BY date, time",
    );

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = owned_params
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        db::row_to_event(row).map(|ev| EventExport {
            id: ev.id,
            date: ev.date,
            time: ev.time,
            kind: ev.kind,
            position: ev.position,
            lunch_break: ev.lunch_break,
            pair: ev.pair,
            source: ev.source,
        })
    })?;

    rows.collect()
}

fn load_sessions(
    conn: &Connection,
    bounds: Option<(String, String)>,
) -> rusqlite::Result<Vec<SessionExport>> {
    let (sql, owned_params) = build_query_with_range(
        r#"
        SELECT
          id,
          date,
          position,
          start_time,
          COALESCE(lunch_break, 0) AS lunch_break,
          end_time
        FROM work_sessions
        "#,
        bounds,
        " ORDER BY date, start_time",
    );

    let mut stmt = conn.prepare(&sql)?;
    let param_refs: Vec<&dyn rusqlite::ToSql> = owned_params
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        db::row_to_worksession(row).map(|ws| SessionExport {
            id: ws.id,
            date: ws.date,
            position: ws.position,
            start: ws.start,
            lunch_break: ws.lunch,
            end: ws.end,
            work_duration: ws.work_duration.map(|m| {
                let (hh, mm) = mins2readable(m);
                format!("{}h {}m", hh, mm)
            }),
        })
    })?;

    rows.collect()
}

fn ensure_writable(path: &Path, force: bool) -> Result<(), Box<dyn Error>> {
    if !path.exists() {
        return Ok(());
    }
    if force {
        return Ok(());
    }

    // Interactive prompt
    eprint!(
        "⚠️  File '{}' already exists. Overwrite? [y/N]: ",
        path.display()
    );
    io::stderr().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let ans = answer.trim().to_ascii_lowercase();

    if ans == "y" || ans == "yes" {
        Ok(())
    } else {
        Err("Export cancelled: existing file not overwritten"
            .to_string()
            .into())
    }
}

/// Export data as JSON
fn export_json<T: Serialize>(data: &[T], path: &Path) -> Result<(), Box<dyn Error>> {
    let json_data = serde_json::to_string_pretty(data)?;
    let mut file = File::create(path)?;
    file.write_all(json_data.as_bytes())?;
    println!("✅ Exported data to {}", path.display());
    Ok(())
}

fn export_csv<T: Serialize>(data: &[T], path: &Path) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_path(path)?;
    for item in data {
        wtr.serialize(item)?;
    }
    wtr.flush()?;
    println!("✅ Exported data to {}", path.display());
    Ok(())
}

pub fn export_xlsx<T: Serialize>(data: &[T], path: &Path) -> Result<(), Box<dyn Error>> {
    println!("📘 Exporting to XLSX: {}", path.display());

    // Create a new workbook and add a worksheet.
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if data.is_empty() {
        worksheet.write(0, 0, "No data available")?;
        workbook.save(path.to_str().ok_or("invalid path")?)?;
        println!("✅ XLSX export completed (empty dataset).");
        return Ok(());
    }

    // Serialize dynamically to obtain headers and values
    let json = serde_json::to_value(data)?;
    let arr = json.as_array().ok_or("invalid data serialization")?;
    let first_obj = arr[0].as_object().ok_or("first row is not an object")?;

    let headers: Vec<String> = first_obj.keys().cloned().collect();

    // Visual table style: emulate Excel "Blue, Medium 16"
    // Header: deep blue background, white bold text, thin border
    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x2F75B5))
        .set_pattern(FormatPattern::Solid)
        .set_border(FormatBorder::Thin);

    // Banded row colors (light blue / white) and border will be applied per-cell
    let band1_color = Color::RGB(0xEAF3FB); // light blue
    let band2_color = Color::RGB(0xFFFFFF); // white

    // Base number format alignment (border will be added per-cell)
    let num_align = FormatAlign::Right;

    // Write headers (shifted one row down to leave room for the label)
    for (c, header) in headers.iter().enumerate() {
        worksheet.write_with_format(0u32, c as u16, header, &header_format)?;
    }

    // Freeze the first row so the header remains visible when scrolling.
    // Use the worksheet API to freeze panes if available. Try common method name
    // and ignore error at compile time if not present; we'll remove the call if
    // the compiler reports the method is absent.
    let _ = worksheet.set_freeze_panes(1, 0);

    // Track column widths (approximate, measured with Unicode width)
    let mut col_widths: Vec<usize> = headers
        .iter()
        .map(|h| UnicodeWidthStr::width(h.as_str()))
        .collect();

    // Write rows (data starts two rows down: label row + header row)
    for (r, item) in arr.iter().enumerate() {
        let row = (r + 1) as u32;
        let obj = item.as_object().ok_or("row is not an object")?;

        for (c, key) in headers.iter().enumerate() {
            let v = obj.get(key).unwrap_or(&Value::Null);
            match v {
                Value::String(s) => {
                    // Try to parse date/time strings and write as Excel dates
                    if let Some((num_format, serial)) = parse_to_excel_date(s) {
                        // Determine band color for this data row (use original index r)
                        let bg = if (r % 2) == 0 {
                            band1_color
                        } else {
                            band2_color
                        };
                        let fmt = Format::new()
                            .set_num_format(num_format)
                            .set_background_color(bg)
                            .set_pattern(FormatPattern::Solid)
                            .set_border(FormatBorder::Thin);
                        worksheet.write_with_format(row, c as u16, serial, &fmt)?;
                        col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(s.as_str()));
                    } else if !s.is_empty() {
                        let bg = if (r % 2) == 0 {
                            band1_color
                        } else {
                            band2_color
                        };
                        let fmt = Format::new()
                            .set_background_color(bg)
                            .set_pattern(FormatPattern::Solid)
                            .set_border(FormatBorder::Thin);
                        worksheet.write_with_format(row, c as u16, s, &fmt)?;
                        col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(s.as_str()));
                    } else {
                        // empty string: skip writing to avoid creating an explicit cell
                    }
                }
                Value::Number(n) => {
                    let num = n.as_f64().ok_or("invalid number")?;
                    // right align for numbers, keep band background and border
                    let bg = if (r % 2) == 0 {
                        band1_color
                    } else {
                        band2_color
                    };
                    let fmt = Format::new()
                        .set_align(num_align)
                        .set_background_color(bg)
                        .set_pattern(FormatPattern::Solid)
                        .set_border(FormatBorder::Thin);
                    worksheet.write_with_format(row, c as u16, num, &fmt)?;
                    col_widths[c] =
                        col_widths[c].max(UnicodeWidthStr::width(n.to_string().as_str()));
                }
                Value::Bool(b) => {
                    let s = b.to_string();
                    let bg = if (r % 2) == 0 {
                        band1_color
                    } else {
                        band2_color
                    };
                    let fmt = Format::new()
                        .set_background_color(bg)
                        .set_pattern(FormatPattern::Solid)
                        .set_border(FormatBorder::Thin);
                    worksheet.write_with_format(row, c as u16, &s, &fmt)?;
                    col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(s.as_str()));
                }
                _ => {
                    // Null / other: skip writing (no border)
                }
            }
        }
    }

    // Convert character widths to Excel column width units (approximation).
    // Excel column width roughly equals number of '0' chars that fit; we add a padding.
    for (c, w) in col_widths.iter().enumerate() {
        let width_chars = *w as f64 + 2.0; // padding
        worksheet.set_column_width(c as u16, width_chars)?;
    }

    workbook.save(path.to_str().ok_or("invalid path")?)?;
    println!("✅ XLSX export completed with styling.");
    Ok(())
}

pub fn export_pdf<T: Serialize>(
    data: &[T],
    path: &Path,
    export_events: bool,
) -> Result<(), Box<dyn Error>> {
    println!("📘 Exporting to PDF: {}", path.display());

    let headers = get_headers(export_events);
    let data_vec = if export_events {
        events_to_table(unsafe {
            // Safety: we ensure T is EventExport when export_events is true
            &*(data as *const [T] as *const [EventExport])
        })
    } else {
        sessions_to_table(unsafe {
            // Safety: we ensure T is SessionExport when export_events is false
            &*(data as *const [T] as *const [SessionExport])
        })
    };
    let mut pdf = PdfManager::new();
    pdf.write_table(&headers, &data_vec); // 'data' deve essere &[Vec<String>]
    pdf.save(path)?;

    println!("✅ PDF export completed.");
    Ok(())
}

/// Parse --range in the same format as --period:
/// - YYYY
/// - YYYY-MM
/// - YYYY-MM-DD
/// - YYYY:YYYY
/// - YYYY-MM:YYYY-MM
/// - YYYY-MM-DD:YYYY-MM-DD
fn parse_range(r: &str) -> Result<(String, String), String> {
    // Case 1: range with ":" → start:end
    if let Some((start_raw, end_raw)) = r.split_once(':') {
        let start = start_raw.trim();
        let end = end_raw.trim();

        if start.len() != end.len() {
            return Err("start and end must have same format".into());
        }

        match start.len() {
            // YYYY:YYYY
            4 => {
                let ys: i32 = start.parse().map_err(|_| "invalid start year")?;
                let ye: i32 = end.parse().map_err(|_| "invalid end year")?;
                Ok((format!("{ys}-01-01"), format!("{ye}-12-31")))
            }
            // YYYY-MM:YYYY-MM
            7 => {
                let ys: i32 = start[0..4].parse().map_err(|_| "invalid start year")?;
                let ms: u32 = start[5..7].parse().map_err(|_| "invalid start month")?;
                let ye: i32 = end[0..4].parse().map_err(|_| "invalid end year")?;
                let me: u32 = end[5..7].parse().map_err(|_| "invalid end month")?;

                let last = month_last_day(ye, me).ok_or("invalid end month")?;

                Ok((
                    format!("{ys}-{ms:02}-01"),
                    format!("{ye}-{me:02}-{last:02}"),
                ))
            }
            // YYYY-MM-DD:YYYY-MM-DD
            10 => {
                NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| "invalid start date")?;
                NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| "invalid end date")?;

                Ok((start.to_string(), end.to_string()))
            }
            _ => Err("unsupported range format".into()),
        }
    }
    // Case 2: single formats (exactly like `--period`)
    else {
        match r.len() {
            // YYYY
            4 => {
                let y: i32 = r.parse().map_err(|_| "invalid year")?;
                Ok((format!("{y}-01-01"), format!("{y}-12-31")))
            }
            // YYYY-MM
            7 => {
                let y: i32 = r[0..4].parse().map_err(|_| "invalid year")?;
                let m: u32 = r[5..7].parse().map_err(|_| "invalid month")?;
                let last = month_last_day(y, m).ok_or("invalid month")?;
                Ok((format!("{y}-{m:02}-01"), format!("{y}-{m:02}-{last:02}")))
            }
            // YYYY-MM-DD
            10 => {
                NaiveDate::parse_from_str(r, "%Y-%m-%d").map_err(|_| "invalid date")?;
                Ok((r.to_string(), r.to_string()))
            }
            _ => Err("unsupported --range format".into()),
        }
    }
}

fn month_last_day(y: i32, m: u32) -> Option<u32> {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => {
            let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
            Some(if leap { 29 } else { 28 })
        }
        _ => None,
    }
}

// Try parsing common date/time formats and convert to Excel serial number.
fn parse_to_excel_date(s: &str) -> Option<(&'static str, f64)> {
    // We'll return (format_string, serial)
    // Try datetime first
    let dt_formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];
    for fmt in dt_formats.iter() {
        if let Ok(dt) = NaiveDateTime::parse_from_str(s, fmt) {
            let serial = naive_datetime_to_excel_serial(&dt);
            return Some(("yyyy-mm-dd hh:mm", serial));
        }
    }

    // Date only
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0).unwrap();
        let serial = naive_datetime_to_excel_serial(&dt);
        return Some(("yyyy-mm-dd", serial));
    }

    // Time only
    let time_formats = ["%H:%M:%S", "%H:%M"];
    for fmt in time_formats.iter() {
        if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
            let seconds = t.num_seconds_from_midnight() as f64;
            let serial = seconds / 86400.0; // fraction of day
            return Some(("hh:mm", serial));
        }
    }

    None
}

fn naive_datetime_to_excel_serial(dt: &NaiveDateTime) -> f64 {
    // Excel uses 1899-12-30 as day 0 for the 1900 date system (with a known bug,
    // but this is the convention used by many libs). We'll compute days since
    // 1899-12-30 and add fractional day.
    let excel_epoch = NaiveDate::from_ymd_opt(1899, 12, 30)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let duration = *dt - excel_epoch;
    let days = duration.num_days() as f64;
    let secs = (duration.num_seconds() - duration.num_days() * 86400) as f64;
    days + secs / 86400.0
}
