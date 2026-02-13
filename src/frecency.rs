use crate::db::DirEntry;

const NANOS_PER_SECOND: i64 = 1_000_000_000;
const HOUR_NS: i64 = 3600 * NANOS_PER_SECOND;
const DAY_NS: i64 = 24 * HOUR_NS;
const WEEK_NS: i64 = 7 * DAY_NS;

/// Scoring mode.
pub enum Mode {
    /// Frequency weighted by recency bucket (default).
    Frecency,
    /// Score = count (frequency only).
    Frequency,
    /// Score = last_visit timestamp (recency only).
    Recency,
}

/// Score a directory entry.
pub fn score(entry: &DirEntry, now_ns: i64, mode: &Mode) -> f64 {
    match mode {
        Mode::Frecency => {
            let age = now_ns.saturating_sub(entry.last_visit_ns);
            let weight = if age < HOUR_NS {
                4.0
            } else if age < DAY_NS {
                2.0
            } else if age < WEEK_NS {
                0.5
            } else {
                0.25
            };
            entry.freq as f64 * weight
        }
        Mode::Frequency => entry.freq as f64,
        Mode::Recency => entry.last_visit_ns as f64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_000_000_000_000_000_000; // 1e18 ns

    fn make_entry(freq: i64, last_visit_ns: i64) -> DirEntry {
        DirEntry {
            cwd: "/test".to_string(),
            freq,
            last_visit_ns,
        }
    }

    // --- Frecency mode: bucket boundaries ---

    #[test]
    fn frecency_within_hour() {
        let entry = make_entry(10, NOW - HOUR_NS + 1);
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 40.0); // 10 * 4
    }

    #[test]
    fn frecency_at_exactly_one_hour() {
        let entry = make_entry(10, NOW - HOUR_NS);
        // age == HOUR_NS, so falls into the "< DAY_NS" bucket
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 20.0); // 10 * 2
    }

    #[test]
    fn frecency_within_day() {
        let entry = make_entry(10, NOW - DAY_NS + 1);
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 20.0); // 10 * 2
    }

    #[test]
    fn frecency_at_exactly_one_day() {
        let entry = make_entry(10, NOW - DAY_NS);
        // age == DAY_NS, so falls into the "< WEEK_NS" bucket
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 5.0); // 10 * 0.5
    }

    #[test]
    fn frecency_within_week() {
        let entry = make_entry(10, NOW - WEEK_NS + 1);
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 5.0); // 10 * 0.5
    }

    #[test]
    fn frecency_at_exactly_one_week() {
        let entry = make_entry(10, NOW - WEEK_NS);
        // age == WEEK_NS, falls into the "older" bucket
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 2.5); // 10 * 0.25
    }

    #[test]
    fn frecency_older_than_week() {
        let entry = make_entry(10, NOW - WEEK_NS * 52);
        assert_eq!(score(&entry, NOW, &Mode::Frecency), 2.5); // 10 * 0.25
    }

    // --- Frequency mode ---

    #[test]
    fn frequency_mode_returns_freq() {
        let entry = make_entry(42, 0);
        assert_eq!(score(&entry, NOW, &Mode::Frequency), 42.0);
    }

    #[test]
    fn frequency_mode_ignores_timestamp() {
        let old = make_entry(10, 0);
        let new = make_entry(10, NOW);
        assert_eq!(
            score(&old, NOW, &Mode::Frequency),
            score(&new, NOW, &Mode::Frequency),
        );
    }

    // --- Recency mode ---

    #[test]
    fn recency_mode_returns_timestamp() {
        let ts = NOW - 12345;
        let entry = make_entry(999, ts);
        assert_eq!(score(&entry, NOW, &Mode::Recency), ts as f64);
    }

    #[test]
    fn recency_mode_ignores_freq() {
        let entry_low = make_entry(1, NOW);
        let entry_high = make_entry(1000, NOW);
        assert_eq!(
            score(&entry_low, NOW, &Mode::Recency),
            score(&entry_high, NOW, &Mode::Recency),
        );
    }
}
