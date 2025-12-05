use crate::config::Config;
use crate::core::calculator::timeline::Timeline;
use crate::core::logic::Core;

/// Expected = minuti teorici da lavorare in un giorno (da config).
pub fn calculate_expected(timeline: &Timeline, cfg: &Config) -> i64 {
    if timeline.pairs.is_empty() {
        return 0; // nessun evento ⇒ niente expected
    }

    // Es: "7h 36m" → 456
    Core::parse_work_duration_to_minutes(&cfg.min_work_duration)
}
