/// Stores and navigates search query history.
#[derive(Debug, Default)]
pub struct SearchHistory {
    /// List of previous search queries.
    history: Vec<String>,
    /// Current position in history (None when not navigating history).
    index: Option<usize>,
}

/// Manages search pattern matching and navigation through search results.
#[derive(Debug, Default)]
pub struct Search {
    /// Active search pattern (set when user submits search).
    active_pattern: Option<String>,
    /// Whether search is case-sensitive.
    case_sensitive: bool,
    /// Index of the current match in match_indices.
    current_match_index: usize,
    /// Total number of matches found.
    total_matches: usize,
    /// Line indices where matches were found.
    match_indices: Vec<usize>,
    /// Search query history.
    pub history: SearchHistory,
}

impl SearchHistory {
    /// Adds a query to the search history if it doesn't already exist.
    pub fn add_query(&mut self, pattern: String) {
        if !self.history.contains(&pattern) {
            self.history.push(pattern);
        }
        self.index = None;
    }

    /// Navigates to the previous query in history.
    ///
    /// Returns `None` if already at the oldest entry.
    pub fn previous_query(&mut self) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }

        match self.index {
            None => {
                // First time navigating history, start from the end
                self.index = Some(self.history.len() - 1);
                Some(self.history[self.history.len() - 1].clone())
            }
            Some(current) => {
                if current > 0 {
                    self.index = Some(current - 1);
                    Some(self.history[current - 1].clone())
                } else {
                    // Already at the oldest entry
                    None
                }
            }
        }
    }

    /// Navigates to the next query in history.
    ///
    /// Returns an empty string when reaching the newest entry to clear input.
    /// Returns `None` if not currently in history mode.
    pub fn next_query(&mut self) -> Option<String> {
        if self.history.is_empty() {
            return None;
        }

        match self.index {
            None => None, // Not currently in history mode
            Some(current) => {
                if current < self.history.len() - 1 {
                    self.index = Some(current + 1);
                    Some(self.history[current + 1].clone())
                } else {
                    // At the newest entry, exit history mode
                    self.index = None;
                    Some(String::new()) // Return empty string to clear input
                }
            }
        }
    }

    /// Resets history navigation state.
    pub fn reset(&mut self) {
        self.index = None;
    }

    /// Returns the search history.
    pub fn get_history(&self) -> &[String] {
        &self.history
    }

    /// Restores search history from a vector of queries.
    pub fn restore_history(&mut self, queries: Vec<String>) {
        self.history = queries;
        self.index = None;
    }
}

impl Search {
    /// Applies a search pattern and updates matches.
    ///
    /// Adds the pattern to search history list.
    pub fn apply_pattern<'a>(&mut self, pattern: &str, lines: impl Iterator<Item = &'a str>) {
        if !pattern.is_empty() {
            self.active_pattern = Some(pattern.to_string());
            self.history.add_query(pattern.to_string());
            self.update_matches(pattern, lines);
        }
    }

    /// Clears all matches and active pattern.
    pub fn clear_matches(&mut self) {
        self.active_pattern = None;
        self.match_indices.clear();
        self.total_matches = 0;
        self.current_match_index = 0;
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
    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    /// Reset case sensitivity to false.
    pub fn reset_case_sensitive(&mut self) {
        self.case_sensitive = false;
    }

    /// Updates the list of matching line indices for a given pattern without storing in search history.
    pub fn update_matches<'a>(&mut self, pattern: &str, lines: impl Iterator<Item = &'a str>) {
        self.match_indices.clear();
        self.total_matches = 0;
        self.current_match_index = 0;

        if pattern.is_empty() {
            return;
        }

        for (line_index, line) in lines.enumerate() {
            let matching = if self.case_sensitive {
                line.contains(pattern)
            } else {
                Self::contains_ignore_case(line, pattern)
            };

            if matching {
                self.match_indices.push(line_index);
                self.total_matches += 1;
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
        if let Some(next_index) = self
            .match_indices
            .iter()
            .position(|&pos| pos > current_line)
        {
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
        if let Some(next_index) = self
            .match_indices
            .iter()
            .position(|&pos| pos >= current_line)
        {
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
        if let Some(prev_index) = self
            .match_indices
            .iter()
            .rposition(|&pos| pos < current_line)
        {
            self.current_match_index = prev_index;
            Some(self.match_indices[self.current_match_index])
        } else {
            // No match before current line, wrap to last match
            self.current_match_index = self.match_indices.len() - 1;
            Some(self.match_indices[self.current_match_index])
        }
    }

    /// Returns (current_match_number, total_matches).
    ///
    /// Returns (0, 0) if there are no matches.
    pub fn get_match_info(&self) -> (usize, usize) {
        if self.match_indices.is_empty() {
            (0, 0)
        } else {
            (self.current_match_index + 1, self.total_matches)
        }
    }

    fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        if needle.len() > haystack.len() {
            return false;
        }

        haystack
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    }
}
