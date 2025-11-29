use crate::history::History;
use crate::list_view_state::ListViewState;
use serde::{Deserialize, Serialize};

/// Filter mode - include or exclude matching lines.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ActiveFilterMode {
    /// Include only lines matching the pattern.
    #[default]
    Include,
    /// Exclude lines matching the pattern.
    Exclude,
}

/// A filter history entry containing the complete state of a filter.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FilterHistoryEntry {
    pub pattern: String,
    pub mode: ActiveFilterMode,
    pub case_sensitive: bool,
}

/// A single filter pattern.
#[derive(Debug, Clone)]
pub struct FilterPattern {
    /// The pattern to match.
    pub pattern: String,
    /// Whether to include or exclude matching lines.
    pub mode: ActiveFilterMode,
    /// Whether the pattern matching is case-sensitive.
    pub case_sensitive: bool,
    /// Whether this pattern is currently active.
    pub enabled: bool,
}

impl FilterPattern {
    /// Creates a new filter pattern.
    pub fn new(
        pattern: String,
        mode: ActiveFilterMode,
        case_sensitive: bool,
        enabled: bool,
    ) -> Self {
        Self {
            pattern,
            mode,
            case_sensitive,
            enabled,
        }
    }
}

#[derive(Debug, Default)]
struct FilterList {
    /// All filter patterns.
    patterns: Vec<FilterPattern>,
    /// View state for the filter list
    view: ListViewState,
}

impl FilterList {
    fn patterns(&self) -> &[FilterPattern] {
        &self.patterns
    }

    fn selected_index(&self) -> usize {
        self.view.selected_index()
    }

    fn move_up(&mut self) {
        self.view.move_up_wrap();
    }

    fn move_down(&mut self) {
        self.view.move_down_wrap();
    }

    fn toggle_selected(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            self.patterns[selected].enabled = !self.patterns[selected].enabled;
        }
    }

    fn remove_selected(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            self.patterns.remove(selected);
            let count = self.patterns.len();
            self.view.set_item_count(count);
        }
    }

    fn toggle_selected_case_sensitive(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            self.patterns[selected].case_sensitive = !self.patterns[selected].case_sensitive;
        }
    }

    fn toggle_selected_mode(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            self.patterns[selected].mode = match self.patterns[selected].mode {
                ActiveFilterMode::Include => ActiveFilterMode::Exclude,
                ActiveFilterMode::Exclude => ActiveFilterMode::Include,
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
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            Some(&self.patterns[selected])
        } else {
            None
        }
    }

    fn update_selected(&mut self, new_pattern: &str) -> bool {
        let selected = self.view.selected_index();
        if selected < self.patterns.len() {
            let selected_mode = self.patterns[selected].mode;
            let duplicate_exists = self.patterns.iter().enumerate().any(|(idx, fp)| {
                idx != selected && fp.pattern == new_pattern && fp.mode == selected_mode
            });

            if !duplicate_exists {
                self.patterns[selected].pattern = new_pattern.to_string();
                return true;
            }
        }
        false
    }

    fn add_pattern(&mut self, pattern: &FilterPattern) {
        self.patterns.push(pattern.clone());
        // Select the newly added pattern
        let count = self.patterns.len();
        self.view.set_item_count(count);
        self.view.select_last();
    }

    fn pattern_exists(&self, pattern: &str, mode: ActiveFilterMode) -> bool {
        self.patterns
            .iter()
            .any(|fp| fp.pattern == pattern && fp.mode == mode)
    }
}

/// Manages filter patterns.
#[derive(Debug, Default)]
pub struct Filter {
    filter_list: FilterList,
    filter_mode: ActiveFilterMode,
    case_sensitive: bool,
    pub history: History<FilterHistoryEntry>,
}

impl Filter {
    /// Creates a new Filter with preconfigured patterns.
    pub fn with_patterns(patterns: Vec<FilterPattern>) -> Self {
        let mut filter = Self {
            filter_list: FilterList {
                patterns,
                view: ListViewState::new(),
            },
            filter_mode: ActiveFilterMode::default(),
            case_sensitive: false,
            history: History::new(),
        };

        filter
            .filter_list
            .view
            .set_item_count(filter.filter_list.patterns.len());

        filter
    }
}

impl Filter {
    /// Toggles the filter mode between Include and Exclude.
    pub fn toggle_mode(&mut self) {
        self.filter_mode = match self.filter_mode {
            ActiveFilterMode::Include => ActiveFilterMode::Exclude,
            ActiveFilterMode::Exclude => ActiveFilterMode::Include,
        };
    }

    /// Resets the filter mode to Include.
    pub fn reset_mode(&mut self) {
        self.filter_mode = ActiveFilterMode::Include;
    }

    /// Returns the current filter mode.
    pub fn get_mode(&self) -> &ActiveFilterMode {
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

    /// Sets the filter mode.
    pub fn set_mode(&mut self, mode: ActiveFilterMode) {
        self.filter_mode = mode;
    }

    /// Sets case sensitivity.
    pub fn set_case_sensitive(&mut self, case_sensitive: bool) {
        self.case_sensitive = case_sensitive;
    }

    /// Adds a new filter pattern if it doesn't already exist.
    pub fn add_filter_from_pattern(&mut self, pattern: &str) {
        if !pattern.is_empty() && !self.filter_list.pattern_exists(pattern, self.filter_mode) {
            let new_filter = FilterPattern::new(
                pattern.to_string(),
                self.filter_mode,
                self.case_sensitive,
                true,
            );

            self.filter_list.add_pattern(&new_filter);

            self.history.add(FilterHistoryEntry {
                pattern: pattern.to_string(),
                mode: self.filter_mode,
                case_sensitive: self.case_sensitive,
            });
        }
    }

    /// Add a FilterPattern
    pub fn add_filter(&mut self, filter: &FilterPattern) {
        if !self
            .filter_list
            .pattern_exists(&filter.pattern, filter.mode)
        {
            self.filter_list.add_pattern(filter);

            self.history.add(FilterHistoryEntry {
                pattern: filter.pattern.clone(),
                mode: filter.mode,
                case_sensitive: filter.case_sensitive,
            });
        }
    }

    /// Returns all filter patterns.
    pub fn get_filter_patterns(&self) -> &[FilterPattern] {
        self.filter_list.patterns()
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
    pub fn update_selected_pattern(&mut self, new_pattern: &str) -> bool {
        self.filter_list.update_selected(new_pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_filter_creates_new_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        assert_eq!(filter.get_filter_patterns().len(), 1);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
    }

    #[test]
    fn test_add_filter_prevents_duplicates() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("ERROR");
        assert_eq!(filter.get_filter_patterns().len(), 1);
    }

    #[test]
    fn test_add_filter_allows_same_pattern_different_mode() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.toggle_mode();
        filter.add_filter_from_pattern("ERROR");
        assert_eq!(filter.get_filter_patterns().len(), 2);
    }

    #[test]
    fn test_toggle_mode_switches_between_include_and_exclude() {
        let mut filter = Filter::default();
        assert_eq!(*filter.get_mode(), ActiveFilterMode::Include);
        filter.toggle_mode();
        assert_eq!(*filter.get_mode(), ActiveFilterMode::Exclude);
        filter.toggle_mode();
        assert_eq!(*filter.get_mode(), ActiveFilterMode::Include);
    }

    #[test]
    fn test_remove_selected_pattern_deletes_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        // WARNING is selected (newly added), remove it
        filter.remove_selected_pattern();
        assert_eq!(filter.get_filter_patterns().len(), 1);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
    }

    #[test]
    fn test_get_selected_pattern_returns_current_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        // WARNING is selected (newly added)
        let selected = filter.get_selected_pattern().unwrap();
        assert_eq!(selected.pattern, "WARNING");
        filter.move_selection_up();
        let selected = filter.get_selected_pattern().unwrap();
        assert_eq!(selected.pattern, "ERROR");
    }

    #[test]
    fn test_update_selected_pattern_succeeds_with_unique_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        // WARNING is selected (newly added), update it to INFO
        let success = filter.update_selected_pattern("INFO");
        assert!(success);
        assert_eq!(filter.get_filter_patterns()[1].pattern, "INFO");
    }

    #[test]
    fn test_update_selected_pattern_prevents_duplicates() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        // WARNING is selected (newly added), try to update it to ERROR (duplicate)
        let success = filter.update_selected_pattern("ERROR");
        assert!(!success);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
        assert_eq!(filter.get_filter_patterns()[1].pattern, "WARNING");
    }

    #[test]
    fn test_update_selected_pattern_allows_same_pattern_different_mode() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR"); // Include mode
        filter.toggle_mode();
        filter.add_filter_from_pattern("WARNING"); // Exclude mode
        // WARNING (Exclude) is already selected (newly added)
        let success = filter.update_selected_pattern("ERROR");
        assert!(success); // Should succeed because mode is different
        assert_eq!(filter.get_filter_patterns()[1].pattern, "ERROR");
        assert_eq!(
            filter.get_filter_patterns()[1].mode,
            ActiveFilterMode::Exclude
        );
    }

    #[test]
    fn test_add_filter_selects_newly_added_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        assert_eq!(filter.get_selected_pattern_index(), 0);
        assert_eq!(filter.get_selected_pattern().unwrap().pattern, "ERROR");

        filter.add_filter_from_pattern("WARNING");
        assert_eq!(filter.get_selected_pattern_index(), 1);
        assert_eq!(filter.get_selected_pattern().unwrap().pattern, "WARNING");

        filter.add_filter_from_pattern("INFO");
        assert_eq!(filter.get_selected_pattern_index(), 2);
        assert_eq!(filter.get_selected_pattern().unwrap().pattern, "INFO");
    }
}
