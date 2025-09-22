#[derive(Debug, Default)]
pub struct Search {
    search_pattern: Option<String>,
    search_history: Vec<String>,
    case_sensitive: bool,
}

impl Search {
    pub fn set_search_pattern(&mut self, pattern: String) {
        self.search_pattern = Some(pattern.clone());
        if !self.search_history.contains(&pattern) {
            self.search_history.push(pattern);
        }
    }

    pub fn update(&mut self, input: &str, min_chars: usize) {
        if input.len() >= min_chars {
            self.set_search_pattern(input.to_string());
        } else {
            self.clear_search_pattern();
        }
    }

    pub fn get_search_pattern(&self) -> Option<String> {
        self.search_pattern.clone()
    }

    pub fn clear_search_pattern(&mut self) {
        self.search_pattern = None;
    }

    pub fn next(&self) -> Option<String> {
        None
    }

    pub fn previous(&self) -> Option<String> {
        // Placeholder for actual search logic
        None
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }
}
