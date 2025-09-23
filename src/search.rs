#[derive(Debug, Default)]
pub struct Search {
    search_pattern: Option<String>,
    search_history: Vec<String>,
    case_sensitive: bool,
    current_match_index: usize,
    total_matches: usize,
    match_indices: Vec<usize>,
}

impl Search {
    pub fn set_search_pattern(&mut self, pattern: String) {
        self.search_pattern = Some(pattern.clone());
        if !self.search_history.contains(&pattern) {
            self.search_history.push(pattern);
        }
        self.reset_matches();
    }

    pub fn update_search_pattern(&mut self, input: &str, min_chars: usize) {
        if input.is_empty() {
            self.clear_search_pattern();
            return;
        }

        if input.len() >= min_chars {
            self.set_search_pattern(input.to_string());
        }
    }

    pub fn get_search_pattern(&self) -> Option<String> {
        self.search_pattern.clone()
    }

    pub fn clear_search_pattern(&mut self) {
        self.search_pattern = None;
        self.reset_matches();
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn update_matches<'a>(&mut self, lines: impl Iterator<Item = &'a str>) {
        self.match_indices.clear();
        self.total_matches = 0;
        self.current_match_index = 0;

        let Some(pattern) = &self.search_pattern else {
            return;
        };

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
