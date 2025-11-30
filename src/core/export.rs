use crate::db::pool::DbPool;
use crate::errors::{AppError, AppResult};
use crate::export::pdf::PdfManager;
use chrono::Timelike;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rusqlite::params;
use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, FormatPattern, Workbook};
use serde::Serialize;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

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
fn get_headers() -> Vec<&'static str> {
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

pub struct ExportLogic;

impl ExportLogic {
    /// Export degli eventi.
    ///
    /// - `format`: "csv" | "json" | "xlsx" | "pdf"
    /// - `file`: path assoluto del file di output
    /// - `range`: `None`, `"all"` oppure espressioni tipo `YYYY`, `YYYY-MM`, `YYYY-MM-DD`,
    ///            `YYYY:YYYY`, `YYYY-MM:YYYY-MM`, `YYYY-MM-DD:YYYY-MM-DD`
    pub fn export(
        pool: &mut DbPool,
        format: &str,
        file: &str,
        range: &Option<String>,
        _events: bool,
        force: bool,
    ) -> AppResult<()> {
        use std::io::ErrorKind;

        let fmt = format.to_lowercase();
        if !["csv", "json", "xlsx", "pdf"].contains(&fmt.as_str()) {
            // Usiamo AppError via io::Error generico.
            return Err(AppError::from(io::Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Unsupported format '{}'. Use one of: csv, json, xlsx, pdf",
                    format
                ),
            )));
        }

        let path = Path::new(file);
        if !path.is_absolute() {
            return Err(AppError::from(io::Error::new(
                ErrorKind::InvalidInput,
                format!("Output file path must be absolute: {file}"),
            )));
        }

        // Gestione sovrascrittura
        ensure_writable(path, force)?;

        // Interpreta --range
        let date_bounds: Option<(NaiveDate, NaiveDate)> = match range {
            None => None,
            Some(r) if r.eq_ignore_ascii_case("all") => None,
            Some(r) => Some(parse_range(r)?),
        };

        // Carica gli eventi dal DB
        let events_vec = load_events(pool, date_bounds)?;

        if events_vec.is_empty() {
            println!("⚠️  No events found for the selected range. Nothing to export.");
            return Ok(());
        }

        // Esporta nel formato richiesto
        export_to_format(&fmt, &events_vec, path)?;

        Ok(())
    }
}

/// Controlla se il file esiste già; in caso affermativo chiede l'autorizzazione all'utente a sovrascrivere il file attuale con quello nuovo.
/// Se viene passato `force=true`, non viene chiesta alcuna conferma e si procede direttamente.
fn ensure_writable(path: &Path, force: bool) -> AppResult<()> {
    use std::io::ErrorKind;

    if !path.exists() {
        return Ok(());
    }
    if force {
        return Ok(());
    }

    eprint!(
        "⚠️  File '{}' already exists. Overwrite? [y/N]: ",
        path.display()
    );
    io::stderr().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).map_err(AppError::from)?;
    let ans = answer.trim().to_ascii_lowercase();

    if ans == "y" || ans == "yes" {
        Ok(())
    } else {
        Err(AppError::from(io::Error::new(
            ErrorKind::Other,
            "Export cancelled: existing file not overwritten",
        )))
    }
}

/// Carica gli eventi dal DB, opzionalmente filtrati per range di date.
fn load_events(
    pool: &mut DbPool,
    bounds: Option<(NaiveDate, NaiveDate)>,
) -> AppResult<Vec<EventExport>> {
    let conn = &mut pool.conn;

    let mut events = Vec::new();

    match bounds {
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, date, time, kind, position, lunch_break, pair, source
                 FROM events
                 ORDER BY date ASC, time ASC",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(EventExport {
                    id: row.get(0)?,
                    date: row.get::<_, String>(1)?,
                    time: row.get::<_, String>(2)?,
                    kind: row.get::<_, String>(3)?,
                    position: row.get::<_, String>(4)?,
                    lunch_break: row.get::<_, i32>(5)?,
                    pair: row.get::<_, i32>(6)?,
                    source: row.get::<_, String>(7)?,
                })
            })?;

            for r in rows {
                events.push(r?);
            }
        }
        Some((start, end)) => {
            let start_str = start.format("%Y-%m-%d").to_string();
            let end_str = end.format("%Y-%m-%d").to_string();

            let mut stmt = conn.prepare(
                "SELECT id, date, time, kind, position, lunch_break, pair, source
                 FROM events
                 WHERE date BETWEEN ?1 AND ?2
                 ORDER BY date ASC, time ASC",
            )?;

            let rows = stmt.query_map(params![start_str, end_str], |row| {
                Ok(EventExport {
                    id: row.get(0)?,
                    date: row.get::<_, String>(1)?,
                    time: row.get::<_, String>(2)?,
                    kind: row.get::<_, String>(3)?,
                    position: row.get::<_, String>(4)?,
                    lunch_break: row.get::<_, i32>(5)?,
                    pair: row.get::<_, i32>(6)?,
                    source: row.get::<_, String>(7)?,
                })
            })?;

            for r in rows {
                events.push(r?);
            }
        }
    }

    Ok(events)
}

/// Dispatch per i vari formati.
fn export_to_format(fmt: &str, events: &[EventExport], path: &Path) -> AppResult<()> {
    match fmt {
        "csv" => export_csv(events, path)?,
        "json" => export_json(events, path)?,
        "xlsx" => export_xlsx(events, path)?,
        "pdf" => export_pdf(events, path)?,
        _ => unreachable!(),
    }
    Ok(())
}

/// Export JSON
fn export_json(events: &[EventExport], path: &Path) -> AppResult<()> {
    use std::io::ErrorKind;

    let json_data = serde_json::to_string_pretty(events).map_err(|e| {
        AppError::from(io::Error::new(
            ErrorKind::Other,
            format!("JSON serialization error: {e}"),
        ))
    })?;
    let mut file = File::create(path)?;
    file.write_all(json_data.as_bytes())?;
    println!("✅ Exported data to {}", path.display());
    Ok(())
}

/// Export CSV
fn export_csv(events: &[EventExport], path: &Path) -> AppResult<()> {
    use std::io::ErrorKind;

    let mut wtr = csv::Writer::from_path(path).map_err(|e| {
        AppError::from(io::Error::new(
            ErrorKind::Other,
            format!("CSV open error: {e}"),
        ))
    })?;
    for item in events {
        wtr.serialize(item).map_err(|e| {
            AppError::from(io::Error::new(
                ErrorKind::Other,
                format!("CSV write error: {e}"),
            ))
        })?;
    }
    wtr.flush().map_err(|e| {
        AppError::from(io::Error::new(
            ErrorKind::Other,
            format!("CSV flush error: {e}"),
        ))
    })?;
    println!("✅ Exported data to {}", path.display());
    Ok(())
}

/// Export XLSX (solo events)
pub fn export_xlsx(events: &[EventExport], path: &Path) -> AppResult<()> {
    use std::io::ErrorKind;

    println!("📘 Exporting to XLSX: {}", path.display());

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    if events.is_empty() {
        worksheet
            .write(0, 0, "No data available")
            .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
        workbook
            .save(
                path.to_str().ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::Other, "invalid path"))
                })?,
            )
            .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
        println!("✅ XLSX export completed (empty dataset).");
        return Ok(());
    }

    // headers fissi dal nostro EventExport
    let headers = get_headers();

    // Header style
    let header_format = Format::new()
        .set_bold()
        .set_font_color(Color::RGB(0xFFFFFF))
        .set_background_color(Color::RGB(0x2F75B5))
        .set_pattern(FormatPattern::Solid)
        .set_border(FormatBorder::Thin);

    let band1_color = Color::RGB(0xEAF3FB);
    let band2_color = Color::RGB(0xFFFFFF);
    let num_align = FormatAlign::Right;

    // Header row
    for (c, header) in headers.iter().enumerate() {
        worksheet
            .write_with_format(0u32, c as u16, *header, &header_format)
            .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
    }

    let _ = worksheet.set_freeze_panes(1, 0);

    let mut col_widths: Vec<usize> = headers
        .iter()
        .map(|h| UnicodeWidthStr::width(h.to_string().as_str()))
        .collect();

    for (r, ev) in events.iter().enumerate() {
        let row = (r + 1) as u32;

        let values: Vec<(usize, String)> = vec![
            (0, ev.id.to_string()),
            (1, ev.date.clone()),
            (2, ev.time.clone()),
            (3, ev.kind.clone()),
            (4, ev.position.clone()),
            (5, ev.lunch_break.to_string()),
            (6, ev.pair.to_string()),
            (7, ev.source.clone()),
        ];

        for (c, v) in values {
            let s = v;
            // prova a interpretare date / datetime / time
            if let Some((num_format, serial)) = parse_to_excel_date(&s) {
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
                worksheet
                    .write_with_format(row, c as u16, serial, &fmt)
                    .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
                col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(s.as_str()));
            } else if let Ok(num) = s.parse::<f64>() {
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
                worksheet
                    .write_with_format(row, c as u16, num, &fmt)
                    .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
                col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(num.to_string().as_str()));
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
                worksheet
                    .write_with_format(row, c as u16, &s, &fmt)
                    .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
                col_widths[c] = col_widths[c].max(UnicodeWidthStr::width(s.as_str()));
            }
        }
    }

    for (c, w) in col_widths.iter().enumerate() {
        let width_chars = *w as f64 + 2.0;
        worksheet
            .set_column_width(c as u16, width_chars)
            .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;
    }

    workbook
        .save(
            path.to_str()
                .ok_or_else(|| AppError::from(io::Error::new(ErrorKind::Other, "invalid path")))?,
        )
        .map_err(|e| AppError::from(io::Error::new(ErrorKind::Other, e.to_string())))?;

    println!("✅ XLSX export completed with styling.");
    Ok(())
}

/// Export PDF (solo events)
pub fn export_pdf(events: &[EventExport], path: &Path) -> AppResult<()> {
    use std::io::ErrorKind;

    println!("📘 Exporting to PDF: {}", path.display());

    let headers = get_headers();
    let data_vec = events_to_table(events);

    let mut pdf = PdfManager::new();
    pdf.write_table(&headers, &data_vec);

    pdf.save(path).map_err(|e| {
        AppError::from(io::Error::new(
            ErrorKind::Other,
            format!("PDF export error: {e}"),
        ))
    })?;

    println!("✅ PDF export completed.");
    Ok(())
}

/// Parse --range come nella vecchia logica, ma ritorna NaiveDate.
fn parse_range(r: &str) -> AppResult<(NaiveDate, NaiveDate)> {
    use std::io::ErrorKind;

    // range con ":" → start:end
    if let Some((start_raw, end_raw)) = r.split_once(':') {
        let start = start_raw.trim();
        let end = end_raw.trim();

        if start.len() != end.len() {
            return Err(AppError::from(io::Error::new(
                ErrorKind::InvalidInput,
                "start and end must have same format",
            )));
        }

        match start.len() {
            // YYYY:YYYY
            4 => {
                let ys: i32 = start
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid start year"))?;
                let ye: i32 = end
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid end year"))?;
                let d1 = NaiveDate::from_ymd_opt(ys, 1, 1).ok_or_else(|| {
                    AppError::from(io::Error::new(
                        ErrorKind::InvalidInput,
                        "invalid start date",
                    ))
                })?;
                let d2 = NaiveDate::from_ymd_opt(ye, 12, 31).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end date"))
                })?;
                Ok((d1, d2))
            }
            // YYYY-MM:YYYY-MM
            7 => {
                let ys: i32 = start[0..4]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid start year"))?;
                let ms: u32 = start[5..7]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid start month"))?;
                let ye: i32 = end[0..4]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid end year"))?;
                let me: u32 = end[5..7]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid end month"))?;

                let last = month_last_day(ye, me).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end month"))
                })?;

                let d1 = NaiveDate::from_ymd_opt(ys, ms, 1).ok_or_else(|| {
                    AppError::from(io::Error::new(
                        ErrorKind::InvalidInput,
                        "invalid start date",
                    ))
                })?;
                let d2 = NaiveDate::from_ymd_opt(ye, me, last).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end date"))
                })?;
                Ok((d1, d2))
            }
            // YYYY-MM-DD:YYYY-MM-DD
            10 => {
                let d1 = NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| {
                    AppError::from(io::Error::new(
                        ErrorKind::InvalidInput,
                        "invalid start date",
                    ))
                })?;
                let d2 = NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end date"))
                })?;
                Ok((d1, d2))
            }
            _ => Err(AppError::from(io::Error::new(
                ErrorKind::InvalidInput,
                "unsupported range format",
            ))),
        }
    } else {
        // Singolo valore (come --period)
        match r.len() {
            // YYYY
            4 => {
                let y: i32 = r
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid year"))?;
                let d1 = NaiveDate::from_ymd_opt(y, 1, 1).ok_or_else(|| {
                    AppError::from(io::Error::new(
                        ErrorKind::InvalidInput,
                        "invalid start date",
                    ))
                })?;
                let d2 = NaiveDate::from_ymd_opt(y, 12, 31).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end date"))
                })?;
                Ok((d1, d2))
            }
            // YYYY-MM
            7 => {
                let y: i32 = r[0..4]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid year"))?;
                let m: u32 = r[5..7]
                    .parse()
                    .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "invalid month"))?;
                let last = month_last_day(y, m).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid month"))
                })?;
                let d1 = NaiveDate::from_ymd_opt(y, m, 1).ok_or_else(|| {
                    AppError::from(io::Error::new(
                        ErrorKind::InvalidInput,
                        "invalid start date",
                    ))
                })?;
                let d2 = NaiveDate::from_ymd_opt(y, m, last).ok_or_else(|| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid end date"))
                })?;
                Ok((d1, d2))
            }
            // YYYY-MM-DD
            10 => {
                let d = NaiveDate::parse_from_str(r, "%Y-%m-%d").map_err(|_| {
                    AppError::from(io::Error::new(ErrorKind::InvalidInput, "invalid date"))
                })?;
                Ok((d, d))
            }
            _ => Err(AppError::from(io::Error::new(
                ErrorKind::InvalidInput,
                "unsupported --range format",
            ))),
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

// Parsing date/ora per XLSX
fn parse_to_excel_date(s: &str) -> Option<(&'static str, f64)> {
    // datetime
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

    // date
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        let dt = d.and_hms_opt(0, 0, 0).unwrap();
        let serial = naive_datetime_to_excel_serial(&dt);
        return Some(("yyyy-mm-dd", serial));
    }

    // time
    let time_formats = ["%H:%M:%S", "%H:%M"];
    for fmt in time_formats.iter() {
        if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
            let seconds = t.num_seconds_from_midnight() as f64;
            let serial = seconds / 86400.0;
            return Some(("hh:mm", serial));
        }
    }

    None
}

fn naive_datetime_to_excel_serial(dt: &NaiveDateTime) -> f64 {
    let excel_epoch = NaiveDate::from_ymd_opt(1899, 12, 30)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let duration = *dt - excel_epoch;
    let days = duration.num_days() as f64;
    let secs = (duration.num_seconds() - duration.num_days() * 86400) as f64;
    days + secs / 86400.0
}
