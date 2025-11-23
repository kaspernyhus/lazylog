use crate::list_view_state::ListViewState;
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
    /// View state for the options list
    view_state: ListViewState,
}

impl Default for Options {
    fn default() -> Self {
        let mut options = Self {
            options: vec![
                DisplayOption::new_hide_pattern(
                    "Hide Timestamp & Hostname",
                    r"^(?:\w{3}\s+\d{2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[+-]\d{4}\s+)",
                ),
                DisplayOption::new_toggle("Disable Colors"),
                DisplayOption::new_toggle("Search: Disable jumping to match"),
            ],
            view_state: ListViewState::new(),
        };

        options.view_state.set_item_count(options.count());

        options
    }
}

impl Options {
    /// Number of options
    pub fn count(&self) -> usize {
        self.options.len()
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.view_state.selected_index()
    }

    /// Moves the selection to the previous option, wrapping to the end.
    pub fn move_selection_up(&mut self) {
        self.view_state.move_up_wrap();
    }

    /// Moves the selection to the next option, wrapping to the beginning.
    pub fn move_selection_down(&mut self) {
        self.view_state.move_down_wrap();
    }

    /// Toggles the enabled state of the currently selected option.
    pub fn toggle_selected_option(&mut self) {
        let selected = self.view_state.selected_index();
        if selected < self.options.len() {
            self.options[selected].enabled = !self.options[selected].enabled;
        }
    }

    /// Enables the currently selected option (sets it to true).
    pub fn enable_selected_option(&mut self) {
        let selected = self.view_state.selected_index();
        if selected < self.options.len() {
            self.options[selected].enabled = true;
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
            if option.enabled
                && let DisplayOptionType::HidePattern(pattern) = &option.option_type
            {
                // Only process patterns that match from the start
                if let Some(m) = pattern.find(line)
                    && m.start() == 0
                {
                    offset = offset.max(m.end());
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

    /// Restore options from a saved state.
    pub fn restore(&mut self, saved_options: &[(String, bool)]) {
        for (name, enabled) in saved_options {
            if let Some(option) = self.options.iter_mut().find(|o| o.name == *name) {
                option.enabled = *enabled;
            }
        }
    }
}
