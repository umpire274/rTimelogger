use crate::db::pool::DbPool;
use crate::db::queries::{delete_event, load_events_by_date};
use crate::errors::{AppError, AppResult};
use crate::ui::messages::info;
use chrono::NaiveDate;

pub struct DeleteLogic;

impl DeleteLogic {
    pub fn apply(pool: &mut DbPool, date: NaiveDate, pair: Option<usize>) -> AppResult<()> {
        // la data è già un NaiveDate; se serve la stringa, formattiamola
        let date_str = date.format("%Y-%m-%d").to_string();
        let events = load_events_by_date(pool, &date)?;

        if events.is_empty() {
            return Err(AppError::NoEventsForDate(date_str));
        }

        if let Some(p) = pair {
            // Delete specific pair (in and out)
            let idx = p - 1;
            let pair_events = events
                .chunks(2)
                .nth(idx)
                .ok_or_else(|| AppError::InvalidPair(p))?;

            for ev in pair_events {
                delete_event(pool, ev.id)?;
            }

            info(format!("Deleted pair {} for {}", p, date));
            return Ok(());
        }

        // Delete all events for this date
        for ev in events {
            delete_event(pool, ev.id)?;
        }

        info(format!("Deleted all events for {}", date));
        Ok(())
    }
}
