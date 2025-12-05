use chrono::{Datelike, NaiveDate, Weekday};

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

/// Nome mese in inglese (per header stile 0.7.7)
pub fn month_name(m: &str) -> &'static str {
    match m {
        "01" => "January",
        "02" => "February",
        "03" => "March",
        "04" => "April",
        "05" => "May",
        "06" => "June",
        "07" => "July",
        "08" => "August",
        "09" => "September",
        "10" => "October",
        "11" => "November",
        "12" => "December",
        _ => "Unknown",
    }
}

/// Returns the day of the week in various formats.
/// - `type_wd = 's'` → short, e.g. "Mo"
/// - `type_wd = 'm'` → medium, e.g. "Mon"
/// - `type_wd = 'l'` → long, e.g. "Monday"
pub fn weekday_str(date_str: &str, type_wd: char) -> String {
    if let Ok(ndate) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        let wd = ndate.weekday();
        match type_wd {
            // 's' → short
            's' => match wd {
                Weekday::Mon => "Mo",
                Weekday::Tue => "Tu",
                Weekday::Wed => "We",
                Weekday::Thu => "Th",
                Weekday::Fri => "Fr",
                Weekday::Sat => "Sa",
                Weekday::Sun => "Su",
            }
            .to_string(),
            // 'l' → long
            'l' => match wd {
                Weekday::Mon => "Monday",
                Weekday::Tue => "Tuesday",
                Weekday::Wed => "Wednesday",
                Weekday::Thu => "Thursday",
                Weekday::Fri => "Friday",
                Weekday::Sat => "Saturday",
                Weekday::Sun => "Sunday",
            }
            .to_string(),
            // default → medium
            _ => match wd {
                Weekday::Mon => "Mon",
                Weekday::Tue => "Tue",
                Weekday::Wed => "Wed",
                Weekday::Thu => "Thu",
                Weekday::Fri => "Fri",
                Weekday::Sat => "Sat",
                Weekday::Sun => "Sun",
            }
            .to_string(),
        }
    } else {
        String::new() // if the date is invalid, return an empty string
    }
}
