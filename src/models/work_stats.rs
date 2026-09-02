use chrono::NaiveDate;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WorkStats {
    pub label: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub expected_minutes: i64,
    pub worked_minutes: i64,
    pub paid_leave_minutes: i64,
    pub sick_leave_minutes: i64,
    pub national_holiday_minutes: i64,
    pub worked_days: u32,
    pub paid_leave_days: u32,
    pub sick_leave_days: u32,
    pub national_holiday_days: u32,
}

impl WorkStats {
    pub fn recognized_minutes(&self) -> i64 {
        self.worked_minutes
            + self.paid_leave_minutes
            + self.sick_leave_minutes
            + self.national_holiday_minutes
    }

    pub fn balance_minutes(&self) -> i64 {
        self.recognized_minutes() - self.expected_minutes
    }

    pub fn average_worked_minutes(&self) -> Option<i64> {
        (self.worked_days > 0).then(|| self.worked_minutes / i64::from(self.worked_days))
    }
}
