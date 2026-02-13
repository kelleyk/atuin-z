use crate::db::DirEntry;
use crate::exclusions;
use crate::frecency::{self, Mode};
use std::path::Path;

/// A scored directory result.
pub struct ScoredDir {
    pub path: String,
    pub score: f64,
}

/// Filter, score, and rank directory entries against the given keywords.
///
/// Checks that directories exist on disk. See [`rank_with`] for details.
pub fn rank(
    entries: Vec<DirEntry>,
    keywords: &[String],
    mode: &Mode,
    now_ns: i64,
    exclusions: &[String],
) -> Vec<ScoredDir> {
    rank_with(entries, keywords, mode, now_ns, exclusions, |p| {
        Path::new(p).is_dir()
    })
}

/// Filter, score, and rank directory entries against the given keywords.
///
/// Rules:
/// - All keywords must match as case-insensitive substrings of the path (AND logic)
/// - Directories where the last keyword matches the basename get a score boost
/// - Directories that fail `dir_exists` are filtered out
/// - Excluded directories are filtered out
fn rank_with<F: Fn(&str) -> bool>(
    entries: Vec<DirEntry>,
    keywords: &[String],
    mode: &Mode,
    now_ns: i64,
    exclusions: &[String],
    dir_exists: F,
) -> Vec<ScoredDir> {
    let keywords_lower: Vec<String> = keywords.iter().map(|k| k.to_lowercase()).collect();

    let mut results: Vec<ScoredDir> = entries
        .iter()
        .filter(|e| {
            // All keywords must match as case-insensitive substrings
            let path_lower = e.cwd.to_lowercase();
            keywords_lower.iter().all(|kw| path_lower.contains(kw))
        })
        .filter(|e| {
            // Filter out excluded directories
            !exclusions::is_excluded(&e.cwd, exclusions)
        })
        .filter(|e| {
            // Filter out directories that no longer exist
            dir_exists(&e.cwd)
        })
        .map(|e| {
            let mut s = frecency::score(e, now_ns, mode);

            // Boost if the last keyword matches the basename
            if let Some(last_kw) = keywords_lower.last() {
                if let Some(basename) = Path::new(&e.cwd).file_name() {
                    if basename.to_string_lossy().to_lowercase().contains(last_kw) {
                        s *= 1.5;
                    }
                }
            }

            ScoredDir {
                path: e.cwd.clone(),
                score: s,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DirEntry;
    use crate::frecency::Mode;

    fn make_entry(cwd: &str, freq: i64, last_visit_ns: i64) -> DirEntry {
        DirEntry {
            cwd: cwd.to_string(),
            freq,
            last_visit_ns,
        }
    }

    const NOW: i64 = 1_000_000_000_000_000_000; // 1e18 ns

    fn rank_all_exist(
        entries: Vec<DirEntry>,
        keywords: &[String],
        mode: &Mode,
        now_ns: i64,
        exclusions: &[String],
    ) -> Vec<ScoredDir> {
        rank_with(entries, keywords, mode, now_ns, exclusions, |_| true)
    }

    #[test]
    fn all_keywords_must_match() {
        let entries = vec![
            make_entry("/home/user/projects/foo", 10, NOW),
            make_entry("/home/user/documents/bar", 10, NOW),
            make_entry("/home/user/projects/bar", 10, NOW),
        ];
        let keywords: Vec<String> = vec!["projects".into(), "bar".into()];
        let results = rank_all_exist(entries, &keywords, &Mode::Frequency, NOW, &[]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/home/user/projects/bar");
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let entries = vec![make_entry("/home/user/MyProject", 10, NOW)];
        let keywords: Vec<String> = vec!["myproject".into()];
        let results = rank_all_exist(entries, &keywords, &Mode::Frequency, NOW, &[]);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn no_keywords_matches_all() {
        let entries = vec![
            make_entry("/a", 5, NOW),
            make_entry("/b", 3, NOW),
        ];
        let results = rank_all_exist(entries, &[], &Mode::Frequency, NOW, &[]);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn excluded_directories_are_filtered() {
        let entries = vec![
            make_entry("/home/user/keep", 10, NOW),
            make_entry("/home/user/remove", 10, NOW),
        ];
        let exclusions: Vec<String> = vec!["/home/user/remove".into()];
        let results = rank_all_exist(entries, &[], &Mode::Frequency, NOW, &exclusions);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/home/user/keep");
    }

    #[test]
    fn nonexistent_directories_are_filtered() {
        let entries = vec![
            make_entry("/exists", 10, NOW),
            make_entry("/gone", 10, NOW),
        ];
        let results = rank_with(
            entries,
            &[],
            &Mode::Frequency,
            NOW,
            &[],
            |p| p == "/exists",
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "/exists");
    }

    #[test]
    fn basename_match_gets_boost() {
        // Both match keyword "proj", but only the second has "proj" in the basename.
        let entries = vec![
            make_entry("/home/proj/code", 10, NOW),
            make_entry("/home/user/proj", 10, NOW),
        ];
        let keywords: Vec<String> = vec!["proj".into()];
        let results = rank_all_exist(entries, &keywords, &Mode::Frequency, NOW, &[]);
        assert_eq!(results.len(), 2);
        // The basename match should rank first due to the 1.5x boost.
        assert_eq!(results[0].path, "/home/user/proj");
        assert_eq!(results[1].path, "/home/proj/code");
    }

    #[test]
    fn results_sorted_descending_by_score() {
        let entries = vec![
            make_entry("/low", 1, NOW),
            make_entry("/high", 100, NOW),
            make_entry("/mid", 10, NOW),
        ];
        let results = rank_all_exist(entries, &[], &Mode::Frequency, NOW, &[]);
        assert_eq!(results[0].path, "/high");
        assert_eq!(results[1].path, "/mid");
        assert_eq!(results[2].path, "/low");
    }

    #[test]
    fn frequency_mode_ignores_recency() {
        let old = NOW - 100_000_000_000_000_000; // very old
        let entries = vec![
            make_entry("/frequent", 100, old),
            make_entry("/recent", 1, NOW),
        ];
        let results = rank_all_exist(entries, &[], &Mode::Frequency, NOW, &[]);
        assert_eq!(results[0].path, "/frequent");
    }

    #[test]
    fn recency_mode_ignores_frequency() {
        let entries = vec![
            make_entry("/old-frequent", 1000, NOW - 1_000_000_000),
            make_entry("/new-rare", 1, NOW),
        ];
        let results = rank_all_exist(entries, &[], &Mode::Recency, NOW, &[]);
        assert_eq!(results[0].path, "/new-rare");
    }
}
