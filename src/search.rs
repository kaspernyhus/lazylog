#[derive(Debug, Default)]
pub struct SearchHistory {
    history: Vec<String>,
    index: Option<usize>,
}

#[derive(Debug, Default)]
pub struct Search {
    search_pattern: Option<String>,
    case_sensitive: bool,
    current_match_index: usize,
    total_matches: usize,
    match_indices: Vec<usize>,
    pub history: SearchHistory,
}

impl SearchHistory {
    pub fn add_query(&mut self, pattern: String) {
        if !self.history.contains(&pattern) {
            self.history.push(pattern);
        }
        self.index = None;
    }

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

    pub fn reset(&mut self) {
        self.index = None;
    }
}

impl Search {
    pub fn apply_pattern(&mut self, pattern: String, lines: &[&str]) {
        self.history.add_query(pattern.clone());
        self.set_pattern(pattern);
        self.update_matches(lines);
    }

    pub fn set_pattern(&mut self, pattern: String) {
        self.search_pattern = Some(pattern.clone());
        self.reset_matches();
    }

    pub fn update_pattern(&mut self, input: &str, min_chars: usize) {
        if input.is_empty() {
            self.clear_pattern();
            return;
        }

        if input.len() >= min_chars {
            self.set_pattern(input.to_string());
        }
    }

    pub fn get_pattern(&self) -> Option<&str> {
        self.search_pattern.as_deref()
    }

    pub fn clear_pattern(&mut self) {
        self.search_pattern = None;
        self.reset_matches();
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn update_matches(&mut self, lines: &[&str]) {
        self.match_indices.clear();
        self.total_matches = 0;
        self.current_match_index = 0;

        let Some(pattern) = &self.search_pattern else {
            return;
        };

        if pattern.is_empty() {
            return;
        }

        for (line_index, line) in lines.iter().enumerate() {
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

    fn reset_matches(&mut self) {
        self.current_match_index = 0;
        self.total_matches = 0;
        self.match_indices.clear();
    }

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
