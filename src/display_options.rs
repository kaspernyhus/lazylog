use regex::Regex;

#[derive(Debug, Clone)]
pub struct DisplayOption {
    pub name: String,
    pub enabled: bool,
    pub pattern: Regex,
}

impl DisplayOption {
    pub fn new(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: false,
            pattern: Regex::new(pattern).unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct DisplayOptions {
    pub options: Vec<DisplayOption>,
    pub selected_index: usize,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            options: vec![
                DisplayOption::new("Hide Date, Time & Hostname", r"^\w{3}\s+\d{2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+"),
                DisplayOption::new("Hide ISO8601 Timestamp", r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[+-]\d{4}\s+"),
            ],
            selected_index: 0,
        }
    }
}

impl DisplayOptions {
    pub fn move_selection_up(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.options.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.options.len();
        }
    }

    pub fn toggle_selected_option(&mut self) {
        if self.selected_index < self.options.len() {
            self.options[self.selected_index].enabled = !self.options[self.selected_index].enabled;
        }
    }

    pub fn apply_to_line(&self, line: &str) -> String {
        let mut result = line.to_string();

        for option in &self.options {
            if option.enabled {
                result = option.pattern.replace_all(&result, "").to_string();
            }
        }

        result
    }
}
