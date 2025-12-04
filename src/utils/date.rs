use chrono::{Datelike, NaiveDate};

pub fn today() -> NaiveDate {
    chrono::Local::now().date_naive()
}

pub fn generate_from_period(p: &str) -> Result<Vec<NaiveDate>, String> {
    // YYYY-MM-DD
    if let Ok(d) = NaiveDate::parse_from_str(p, "%Y-%m-%d") {
        return Ok(vec![d]);
    }

    // YYYY-MM
    if let Ok(dm) = chrono::NaiveDate::parse_from_str(&(p.to_string() + "-01"), "%Y-%m-%d") {
        return Ok(all_days_of_month(dm.year(), dm.month()));
    }

    // YYYY
    if let Ok(year) = p.parse::<i32>() {
        return Ok(all_days_of_year(year));
    }

    Err(format!("Invalid period: {}", p))
}

pub fn generate_range(start: &str, end: &str) -> Result<Vec<NaiveDate>, String> {
    let s = generate_from_period(start)?;
    let e = generate_from_period(end)?;

    let start_date = *s.first().unwrap();
    let end_date = *e.last().unwrap();

    let mut out = Vec::new();
    let mut d = start_date;

    while d <= end_date {
        out.push(d);
        d = d.succ_opt().unwrap();
    }

    Ok(out)
}

pub fn current_month_dates() -> Result<Vec<NaiveDate>, String> {
    let today = today();
    Ok(all_days_of_month(today.year(), today.month()))
}

pub fn all_days_of_month(year: i32, month: u32) -> Vec<NaiveDate> {
    let mut out = Vec::new();
    let mut d = NaiveDate::from_ymd_opt(year, month, 1).unwrap();

    while d.month() == month {
        out.push(d);
        d = d.succ_opt().unwrap();
    }

    out
}

pub fn all_days_of_year(year: i32) -> Vec<NaiveDate> {
    let mut v = Vec::new();

    let mut d = NaiveDate::from_ymd_opt(year, 1, 1).unwrap();
    while d.year() == year {
        v.push(d);
        d = d.succ_opt().unwrap();
    }

    v
}

pub fn generate_all_dates() -> Result<Vec<NaiveDate>, String> {
    // placeholder: load all dates for current year
    Ok(all_days_of_year(today().year()))
}

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}
