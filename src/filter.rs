use serde::{Deserialize, Serialize};

/// Filter mode - include or exclude matching lines.
#[derive(Debug, Copy, Clone, PartialEq, Default, Serialize, Deserialize)]
pub enum FilterMode {
    /// Include only lines matching the pattern.
    #[default]
    Include,
    /// Exclude lines matching the pattern.
    Exclude,
}

/// A single filter pattern.
#[derive(Debug, Clone)]
pub struct FilterPattern {
    /// The pattern to match.
    pub pattern: String,
    /// Whether to include or exclude matching lines.
    pub mode: FilterMode,
    /// Whether the pattern matching is case-sensitive.
    pub case_sensitive: bool,
    /// Whether this pattern is currently active.
    pub enabled: bool,
}

impl FilterPattern {
    /// Creates a new filter pattern.
    pub fn new(pattern: String, mode: FilterMode, case_sensitive: bool) -> Self {
        Self {
            pattern,
            mode,
            case_sensitive,
            enabled: true,
        }
    }
}

#[derive(Debug, Default)]
struct FilterList {
    /// All filter patterns.
    patterns: Vec<FilterPattern>,
    /// Index of the currently selected pattern in the overview.
    selected_index: usize,
}

impl FilterList {
    fn patterns(&self) -> &[FilterPattern] {
        &self.patterns
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn move_up(&mut self) {
        if !self.patterns.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.patterns.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    fn move_down(&mut self) {
        if !self.patterns.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.patterns.len();
        }
    }

    fn toggle_selected(&mut self) {
        if self.selected_index < self.patterns.len() {
            self.patterns[self.selected_index].enabled =
                !self.patterns[self.selected_index].enabled;
        }
    }

    fn remove_selected(&mut self) {
        if self.selected_index < self.patterns.len() {
            self.patterns.remove(self.selected_index);
            if self.selected_index >= self.patterns.len() && !self.patterns.is_empty() {
                self.selected_index = self.patterns.len() - 1;
            }
        }
    }

    fn toggle_selected_case_sensitive(&mut self) {
        if self.selected_index < self.patterns.len() {
            self.patterns[self.selected_index].case_sensitive =
                !self.patterns[self.selected_index].case_sensitive;
        }
    }

    fn toggle_selected_mode(&mut self) {
        if self.selected_index < self.patterns.len() {
            self.patterns[self.selected_index].mode = match self.patterns[self.selected_index].mode
            {
                FilterMode::Include => FilterMode::Exclude,
                FilterMode::Exclude => FilterMode::Include,
            };
        }
    }

    fn toggle_all_patterns(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        let all_enabled = self.patterns.iter().all(|p| p.enabled);
        for pattern in &mut self.patterns {
            pattern.enabled = !all_enabled;
        }
    }

    fn get_selected(&self) -> Option<&FilterPattern> {
        if self.selected_index < self.patterns.len() {
            Some(&self.patterns[self.selected_index])
        } else {
            None
        }
    }

    fn update_selected(&mut self, new_pattern: String) {
        if self.selected_index < self.patterns.len() {
            self.patterns[self.selected_index].pattern = new_pattern;
        }
    }

    fn add_pattern(&mut self, pattern: FilterPattern) {
        self.patterns.push(pattern);
    }

    fn pattern_exists(&self, pattern: &str, mode: FilterMode) -> bool {
        self.patterns
            .iter()
            .any(|fp| fp.pattern == pattern && fp.mode == mode)
    }
}

/// Manages filter patterns.
#[derive(Debug, Default)]
pub struct Filter {
    filter_list: FilterList,
    filter_mode: FilterMode,
    case_sensitive: bool,
}

impl Filter {
    /// Creates a new Filter with preconfigured patterns.
    pub fn with_patterns(patterns: Vec<FilterPattern>) -> Self {
        Self {
            filter_list: FilterList {
                patterns,
                selected_index: 0,
            },
            filter_mode: FilterMode::default(),
            case_sensitive: false,
        }
    }
}

impl Filter {
    /// Toggles the filter mode between Include and Exclude.
    pub fn toggle_mode(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::Include => FilterMode::Exclude,
            FilterMode::Exclude => FilterMode::Include,
        };
    }

    /// Resets the filter mode to Include.
    pub fn reset_mode(&mut self) {
        self.filter_mode = FilterMode::Include;
    }

    /// Returns the current filter mode.
    pub fn get_mode(&self) -> &FilterMode {
        &self.filter_mode
    }

    /// Returns whether new filters will be case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Toggles the case sensitivity for new filters.
    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    /// Resets case sensitivity to false.
    pub fn reset_case_sensitive(&mut self) {
        self.case_sensitive = false;
    }

    /// Adds a new filter pattern if it doesn't already exist.
    pub fn add_filter(&mut self, pattern: String) {
        if !self.filter_list.pattern_exists(&pattern, self.filter_mode) {
            self.filter_list.add_pattern(FilterPattern::new(
                pattern,
                self.filter_mode,
                self.case_sensitive,
            ));
        }
    }

    /// Returns all filter patterns.
    pub fn get_filter_patterns(&self) -> &[FilterPattern] {
        self.filter_list.patterns()
    }

    /// Returns mutable access to filter patterns for restoration.
    pub fn get_filter_patterns_mut(&mut self) -> &mut Vec<FilterPattern> {
        &mut self.filter_list.patterns
    }

    /// Returns the index of the currently selected pattern in the overview.
    pub fn get_selected_pattern_index(&self) -> usize {
        self.filter_list.selected_index()
    }

    /// Moves the filter view selection to the previous pattern, wrapping to the end.
    pub fn move_selection_up(&mut self) {
        self.filter_list.move_up();
    }

    /// Moves the filter view selection to the next pattern, wrapping to the beginning.
    pub fn move_selection_down(&mut self) {
        self.filter_list.move_down();
    }

    /// Toggles the enabled state of the selected pattern.
    pub fn toggle_selected_pattern(&mut self) {
        self.filter_list.toggle_selected();
    }

    /// Removes the currently selected pattern and adjusts selection.
    pub fn remove_selected_pattern(&mut self) {
        self.filter_list.remove_selected();
    }

    /// Toggles case sensitivity for the selected pattern.
    pub fn toggle_selected_pattern_case_sensitive(&mut self) {
        self.filter_list.toggle_selected_case_sensitive();
    }

    /// Toggles the mode (Include/Exclude) of the selected pattern.
    pub fn toggle_selected_pattern_mode(&mut self) {
        self.filter_list.toggle_selected_mode();
    }

    /// Toggles all patterns between enabled and disabled.
    pub fn toggle_all_patterns(&mut self) {
        self.filter_list.toggle_all_patterns();
    }

    /// Returns the currently selected pattern, if any.
    pub fn get_selected_pattern(&self) -> Option<&FilterPattern> {
        self.filter_list.get_selected()
    }

    /// Updates the pattern text of the currently selected filter.
    pub fn update_selected_pattern(&mut self, new_pattern: String) {
        self.filter_list.update_selected(new_pattern);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_filter_creates_new_pattern() {
        let mut filter = Filter::default();
        filter.add_filter("ERROR".to_string());
        assert_eq!(filter.get_filter_patterns().len(), 1);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
    }

    #[test]
    fn test_add_filter_prevents_duplicates() {
        let mut filter = Filter::default();
        filter.add_filter("ERROR".to_string());
        filter.add_filter("ERROR".to_string());
        assert_eq!(filter.get_filter_patterns().len(), 1);
    }

    #[test]
    fn test_add_filter_allows_same_pattern_different_mode() {
        let mut filter = Filter::default();
        filter.add_filter("ERROR".to_string());
        filter.toggle_mode();
        filter.add_filter("ERROR".to_string());
        assert_eq!(filter.get_filter_patterns().len(), 2);
    }

    #[test]
    fn test_toggle_mode_switches_between_include_and_exclude() {
        let mut filter = Filter::default();
        assert_eq!(*filter.get_mode(), FilterMode::Include);
        filter.toggle_mode();
        assert_eq!(*filter.get_mode(), FilterMode::Exclude);
        filter.toggle_mode();
        assert_eq!(*filter.get_mode(), FilterMode::Include);
    }

    #[test]
    fn test_remove_selected_pattern_deletes_pattern() {
        let mut filter = Filter::default();
        filter.add_filter("ERROR".to_string());
        filter.add_filter("WARNING".to_string());
        filter.remove_selected_pattern();
        assert_eq!(filter.get_filter_patterns().len(), 1);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "WARNING");
    }

    #[test]
    fn test_get_selected_pattern_returns_current_pattern() {
        let mut filter = Filter::default();
        filter.add_filter("ERROR".to_string());
        filter.add_filter("WARNING".to_string());
        let selected = filter.get_selected_pattern().unwrap();
        assert_eq!(selected.pattern, "ERROR");
        filter.move_selection_down();
        let selected = filter.get_selected_pattern().unwrap();
        assert_eq!(selected.pattern, "WARNING");
    }
}
