use crate::history::History;
use crate::utils::contains_ignore_case;
use rayon::prelude::*;

/// Manages search pattern matching and navigation through search results.
#[derive(Debug, Default)]
pub struct Search {
    /// Active search pattern (set when user submits search).
    active_pattern: Option<String>,
    /// Whether search is case-sensitive.
    case_sensitive: bool,
    /// Index of the current match in match_indices.
    current_match_index: usize,
    /// Line indices where matches were found (in visible lines).
    match_indices: Vec<usize>,
    /// Total number of matches including filtered-out lines.
    total_match_count: usize,
    /// Search query history.
    pub history: History<String>,
}

impl Search {
    /// Applies a search pattern and updates matches.
    pub fn apply_pattern<'a>(&mut self, pattern: &str, lines: impl Iterator<Item = &'a str>) -> Option<usize> {
        if pattern.is_empty() {
            return None;
        }
        self.active_pattern = Some(pattern.to_string());
        self.history.add(pattern.to_string());
        self.update_matches(pattern, lines);
        Some(self.match_indices.len())
    }

    /// Clears all matches and active pattern.
    pub fn clear_matches(&mut self) {
        self.active_pattern = None;
        self.match_indices.clear();
        self.current_match_index = 0;
        self.total_match_count = 0;
    }

    /// Returns the active search pattern (submitted search).
    pub fn get_active_pattern(&self) -> Option<&str> {
        self.active_pattern.as_deref()
    }

    /// Returns whether search is case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Toggles case sensitivity.
    pub fn toggle_case_sensitivity(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    /// Reset case sensitivity to false.
    pub fn reset_case_sensitivity(&mut self) {
        self.case_sensitive = false;
    }

    /// Updates the list of matching line indices for a given pattern without storing in search history.
    pub fn update_matches<'a>(&mut self, pattern: &str, lines: impl Iterator<Item = &'a str>) {
        self.match_indices.clear();
        self.current_match_index = 0;

        if pattern.is_empty() {
            return;
        }

        // Collect lines into a vector for parallel processing
        let lines_vec: Vec<&str> = lines.collect();
        let case_sensitive = self.case_sensitive;
        let pattern_str = pattern.to_string();

        self.match_indices = lines_vec
            .par_iter()
            .enumerate()
            .filter_map(|(line_index, line)| {
                let matching = if case_sensitive {
                    line.contains(&pattern_str)
                } else {
                    contains_ignore_case(line, &pattern_str)
                };

                if matching { Some(line_index) } else { None }
            })
            .collect();
    }

    /// Appends a single line to matches if it matches the active pattern.
    pub fn append_line(&mut self, line_index: usize, line_content: &str) {
        if let Some(pattern) = &self.active_pattern {
            let matching = if self.case_sensitive {
                line_content.contains(pattern)
            } else {
                contains_ignore_case(line_content, pattern)
            };

            if matching {
                self.match_indices.push(line_index);
            }
        }
    }

    /// Finds the next match after the current line.
    ///
    /// Wraps to the first match if no match is found after current line.
    /// Returns `None` if there are no matches.
    pub fn next_match(&mut self, current_line: usize) -> Option<usize> {
        if self.match_indices.is_empty() {
            return None;
        }

        // Find the first match after the current line
        if let Some(next_index) = self.match_indices.iter().position(|&pos| pos > current_line) {
            self.current_match_index = next_index;
            Some(self.match_indices[self.current_match_index])
        } else {
            // No match after current line, wrap to first match
            self.current_match_index = 0;
            Some(self.match_indices[self.current_match_index])
        }
    }

    /// Finds the first match at or after the current line.
    ///
    /// Wraps to the first match if no match is found at or after current line.
    /// Returns `None` if there are no matches.
    pub fn first_match_from(&mut self, current_line: usize) -> Option<usize> {
        if self.match_indices.is_empty() {
            return None;
        }

        // Find the first match at or after the current line
        if let Some(next_index) = self.match_indices.iter().position(|&pos| pos >= current_line) {
            self.current_match_index = next_index;
            Some(self.match_indices[self.current_match_index])
        } else {
            // No match at or after current line, wrap to first match
            self.current_match_index = 0;
            Some(self.match_indices[self.current_match_index])
        }
    }

    /// Finds the previous match before the current line.
    ///
    /// Wraps to the last match if no match is found before current line.
    /// Returns `None` if there are no matches.
    pub fn previous_match(&mut self, current_line: usize) -> Option<usize> {
        if self.match_indices.is_empty() {
            return None;
        }

        // Find the last match before the current line
        if let Some(prev_index) = self.match_indices.iter().rposition(|&pos| pos < current_line) {
            self.current_match_index = prev_index;
            Some(self.match_indices[self.current_match_index])
        } else {
            // No match before current line, wrap to last match
            self.current_match_index = self.match_indices.len() - 1;
            Some(self.match_indices[self.current_match_index])
        }
    }

    /// Returns (current_match_number, visible_matches, total_matches).
    ///
    /// Returns (0, 0, 0) if there are no matches.
    pub fn get_match_info(&self) -> (usize, usize, usize) {
        if self.match_indices.is_empty() {
            (0, 0, self.total_match_count)
        } else {
            (
                self.current_match_index + 1,
                self.match_indices.len(),
                self.total_match_count,
            )
        }
    }

    /// Sets the total match count (including filtered-out lines).
    pub fn set_total_match_count(&mut self, count: usize) {
        self.total_match_count = count;
    }

    /// Counts matches in the given lines without storing indices.
    pub fn count_matches<'a>(&self, pattern: &str, lines: impl Iterator<Item = &'a str>) -> usize {
        if pattern.is_empty() {
            return 0;
        }

        let lines_vec: Vec<&str> = lines.collect();
        let case_sensitive = self.case_sensitive;
        let pattern_str = pattern.to_string();

        lines_vec
            .par_iter()
            .filter(|line| {
                if case_sensitive {
                    line.contains(&pattern_str)
                } else {
                    contains_ignore_case(line, &pattern_str)
                }
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_pattern_sets_active_pattern() {
        let mut search = Search::default();
        let lines = ["WARNING: baz", "ERROR: foo", "INFO: bar"];
        search.apply_pattern("ERROR", lines.iter().copied());
        assert_eq!(search.get_active_pattern(), Some("ERROR"));
    }

    #[test]
    fn test_apply_pattern_adds_to_history() {
        let mut search = Search::default();
        let lines = ["ERROR: foo"];
        search.apply_pattern("ERROR", lines.iter().copied());
        assert_eq!(search.history.get_history().len(), 1);
        assert_eq!(search.history.get_history()[0], "ERROR");
    }

    #[test]
    fn test_apply_pattern_finds_matches() {
        let mut search = Search::default();
        let lines = ["ERROR: foo", "INFO: bar", "ERROR: baz"];
        search.apply_pattern("ERROR", lines.iter().copied());
        let (_current, visible, _total) = search.get_match_info();
        assert_eq!(visible, 2);
    }

    #[test]
    fn test_clear_matches_clears_pattern_and_matches() {
        let mut search = Search::default();
        let lines = ["ERROR: foo"];
        search.apply_pattern("ERROR", lines.iter().copied());
        search.clear_matches();
        assert_eq!(search.get_active_pattern(), None);
        assert_eq!(search.get_match_info(), (0, 0, 0));
    }

    #[test]
    fn test_update_matches_case_insensitive() {
        let mut search = Search::default();
        let lines = ["ERROR: foo", "error: bar", "Error: baz"];
        search.update_matches("error", lines.iter().copied());
        let (_, visible, _) = search.get_match_info();
        assert_eq!(visible, 3);
    }

    #[test]
    fn test_update_matches_case_sensitive() {
        let mut search = Search::default();
        search.toggle_case_sensitivity();
        let lines = ["ERROR: foo", "error: bar", "Error: baz"];
        search.update_matches("error", lines.iter().copied());
        let (_, visible, _) = search.get_match_info();
        assert_eq!(visible, 1);
    }

    #[test]
    fn test_get_match_info_returns_correct_values() {
        let mut search = Search::default();
        let lines = ["ERROR: foo", "INFO: bar", "ERROR: baz"];
        search.update_matches("ERROR", lines.iter().copied());
        search.next_match(0);
        let (current, visible, _total) = search.get_match_info();
        assert_eq!(current, 2);
        assert_eq!(visible, 2);
    }

    #[test]
    fn test_contains_ignore_case_finds_different_cases() {
        assert!(contains_ignore_case("ERROR: foo", "error"));
        assert!(contains_ignore_case("error: foo", "ERROR"));
        assert!(contains_ignore_case("Error: foo", "eRrOr"));
    }

    #[test]
    fn test_contains_ignore_case_returns_false_for_no_match() {
        assert!(!contains_ignore_case("INFO: foo", "error"));
    }

    #[test]
    fn test_contains_ignore_case_handles_empty_needle() {
        assert!(contains_ignore_case("foo", ""));
    }

    #[test]
    fn test_contains_ignore_case_handles_needle_longer_than_haystack() {
        assert!(!contains_ignore_case("foo", "foobar"));
    }
}
