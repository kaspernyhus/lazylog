use regex::Regex;

/// Type of display option.
#[derive(Debug, Clone)]
pub enum DisplayOptionType {
    /// Hides text matching the regex pattern from display.
    /// HidePattern only support hiding prefixes (patterns that match from the start)
    HidePattern(Regex),
    /// Simple toggle option (e.g., disable colors).
    Toggle,
}

/// A single display option with its configuration.
#[derive(Debug, Clone)]
pub struct DisplayOption {
    /// Display name of the option.
    pub name: String,
    /// Whether this option is currently enabled.
    pub enabled: bool,
    /// The type and behavior of this option.
    pub option_type: DisplayOptionType,
}

impl DisplayOption {
    /// Creates a new pattern-based display option that hides matching text.
    pub fn new_hide_pattern(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: false,
            option_type: DisplayOptionType::HidePattern(Regex::new(pattern).unwrap()),
        }
    }

    /// Creates a new toggle-based display option.
    pub fn new_toggle(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: false,
            option_type: DisplayOptionType::Toggle,
        }
    }
}

/// Manages display options for customizing log line rendering.
#[derive(Debug)]
pub struct Options {
    /// All available display options.
    pub options: Vec<DisplayOption>,
    /// Index of the currently selected option.
    pub selected_index: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            options: vec![
                DisplayOption::new_hide_pattern(
                    "Hide Timestamp & Hostname",
                    r"^(?:\w{3}\s+\d{2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[+-]\d{4}\s+)",
                ),
                DisplayOption::new_toggle("Disable Colors"),
                DisplayOption::new_toggle("Search: Disable jumping to match"),
            ],
            selected_index: 0,
        }
    }
}

impl Options {
    /// Moves the selection to the previous option, wrapping to the end.
    pub fn move_selection_up(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.options.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Moves the selection to the next option, wrapping to the beginning.
    pub fn move_selection_down(&mut self) {
        if !self.options.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.options.len();
        }
    }

    /// Toggles the enabled state of the currently selected option.
    pub fn toggle_selected_option(&mut self) {
        if self.selected_index < self.options.len() {
            self.options[self.selected_index].enabled = !self.options[self.selected_index].enabled;
        }
    }

    /// Applies all options to a line.
    pub fn apply_to_line<'a>(&self, line: &'a str) -> &'a str {
        let has_enabled = self.options.iter().any(|option| option.enabled);
        if !has_enabled {
            return line;
        }

        // Find the maximum offset to skip (longest prefix match)
        let mut offset = 0;
        for option in &self.options {
            if option.enabled {
                if let DisplayOptionType::HidePattern(pattern) = &option.option_type {
                    // Only process patterns that match from the start
                    if let Some(m) = pattern.find(line) {
                        if m.start() == 0 {
                            offset = offset.max(m.end());
                        }
                    }
                }
            }
        }

        &line[offset..]
    }

    /// Returns whether a named option is currently enabled.
    pub fn is_enabled(&self, option_name: &str) -> bool {
        self.options
            .iter()
            .find(|o| o.name == option_name)
            .map(|o| o.enabled)
            .unwrap_or(false)
    }
}
