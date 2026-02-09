use std::collections::HashSet;
use std::sync::Arc;

use crate::log::LogLine;
use crate::utils::contains_ignore_case;
use crate::{history::History, resolver::VisibilityRule};
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
    pub fn new(pattern: String, mode: ActiveFilterMode, case_sensitive: bool, enabled: bool) -> Self {
        Self {
            pattern,
            mode,
            case_sensitive,
            enabled,
        }
    }
}

/// Manages filter patterns.
#[derive(Debug, Default)]
pub struct Filter {
    patterns: Vec<FilterPattern>,
    filter_mode: ActiveFilterMode,
    case_sensitive: bool,
    pub history: History<FilterHistoryEntry>,
}

const DEFAULT_CASE_SENSITIVITY: bool = false;

impl Filter {
    /// Creates a new Filter with preconfigured patterns.
    pub fn with_patterns(patterns: Vec<FilterPattern>) -> Self {
        Self {
            patterns,
            filter_mode: ActiveFilterMode::default(),
            case_sensitive: DEFAULT_CASE_SENSITIVITY,
            history: History::new(),
        }
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
    pub fn get_mode(&self) -> ActiveFilterMode {
        self.filter_mode
    }

    /// Sets the filter mode.
    pub fn set_mode(&mut self, mode: ActiveFilterMode) {
        self.filter_mode = mode;
    }

    /// Returns whether new filters will be case-sensitive.
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Toggles the case sensitivity for new filters.
    pub fn toggle_case_sensitivity(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    /// Sets case sensitivity.
    pub fn set_case_sensitivity(&mut self, case_sensitive: bool) {
        self.case_sensitive = case_sensitive;
    }

    /// Resets case sensitivity to default.
    pub fn reset_case_sensitivity(&mut self) {
        self.case_sensitive = DEFAULT_CASE_SENSITIVITY;
    }

    /// Adds a new filter pattern if it doesn't already exist.
    pub fn add_filter_from_pattern(&mut self, pattern: &str) {
        if !pattern.is_empty() && !self.pattern_exists(pattern, self.filter_mode) {
            let new_filter = FilterPattern::new(pattern.to_string(), self.filter_mode, self.case_sensitive, true);
            self.patterns.push(new_filter);

            self.history.add(FilterHistoryEntry {
                pattern: pattern.to_string(),
                mode: self.filter_mode,
                case_sensitive: self.case_sensitive,
            });
        }
    }

    /// Add a FilterPattern
    pub fn add_filter(&mut self, filter: &FilterPattern) {
        if !self.pattern_exists(&filter.pattern, filter.mode) {
            self.patterns.push(filter.clone());

            self.history.add(FilterHistoryEntry {
                pattern: filter.pattern.clone(),
                mode: filter.mode,
                case_sensitive: filter.case_sensitive,
            });
        }
    }

    /// Returns all filter patterns.
    pub fn get_filter_patterns(&self) -> &[FilterPattern] {
        &self.patterns
    }

    /// Returns the number of filter patterns.
    pub fn count(&self) -> usize {
        self.patterns.len()
    }

    /// Returns the pattern at the given index, if any.
    pub fn get_pattern(&self, index: usize) -> Option<&FilterPattern> {
        self.patterns.get(index)
    }

    /// Toggles the enabled state of the pattern at the given index.
    pub fn toggle_pattern_enabled(&mut self, index: usize) {
        if let Some(pattern) = self.patterns.get_mut(index) {
            pattern.enabled = !pattern.enabled;
        }
    }

    /// Disables all filter patterns.
    pub fn disable_all_patterns(&mut self) {
        for pattern in &mut self.patterns {
            pattern.enabled = false;
        }
    }

    /// Toggles all patterns between enabled and disabled.
    pub fn toggle_all_patterns_enabled(&mut self) {
        if self.patterns.is_empty() {
            return;
        }

        let all_enabled = self.patterns.iter().all(|p| p.enabled);
        for pattern in &mut self.patterns {
            pattern.enabled = !all_enabled;
        }
    }

    /// Removes the pattern at the given index.
    pub fn remove_pattern(&mut self, index: usize) {
        if index < self.patterns.len() {
            self.patterns.remove(index);
        }
    }

    /// Toggles case sensitivity for the pattern at the given index.
    pub fn toggle_pattern_case_sensitivity(&mut self, index: usize) {
        if let Some(pattern) = self.patterns.get_mut(index) {
            pattern.case_sensitive = !pattern.case_sensitive;
        }
    }

    /// Toggles the mode (Include/Exclude) of the pattern at the given index.
    pub fn toggle_pattern_mode(&mut self, index: usize) {
        if let Some(pattern) = self.patterns.get_mut(index) {
            pattern.mode = match pattern.mode {
                ActiveFilterMode::Include => ActiveFilterMode::Exclude,
                ActiveFilterMode::Exclude => ActiveFilterMode::Include,
            };
        }
    }

    /// Updates the pattern text at the given index.
    pub fn update_pattern(&mut self, index: usize, new_pattern: &str) -> bool {
        if let Some(pattern) = self.patterns.get(index) {
            let selected_mode = pattern.mode;
            let duplicate_exists = self
                .patterns
                .iter()
                .enumerate()
                .any(|(idx, fp)| idx != index && fp.pattern == new_pattern && fp.mode == selected_mode);

            if !duplicate_exists {
                if let Some(pattern) = self.patterns.get_mut(index) {
                    pattern.pattern = new_pattern.to_string();
                }
                return true;
            }
        }
        false
    }

    /// Checks if a pattern exists with the given mode.
    fn pattern_exists(&self, pattern: &str, mode: ActiveFilterMode) -> bool {
        self.patterns.iter().any(|fp| fp.pattern == pattern && fp.mode == mode)
    }

    /// Checks if content passes the filter patterns.
    pub fn apply_filters(&self, content: &str) -> bool {
        apply_filters(content, &self.patterns)
    }
}

/// Checks if content passes the given filter patterns.
pub fn apply_filters(content: &str, filter_patterns: &[FilterPattern]) -> bool {
    if filter_patterns.is_empty() {
        return true;
    }

    let mut has_include_filters = false;
    let mut include_matched = false;

    for filter in filter_patterns.iter().filter(|f| f.enabled) {
        let matches = if filter.case_sensitive {
            content.contains(&filter.pattern)
        } else {
            contains_ignore_case(content, &filter.pattern)
        };

        match filter.mode {
            ActiveFilterMode::Exclude => {
                if matches {
                    return false;
                }
            }
            ActiveFilterMode::Include => {
                has_include_filters = true;
                if matches {
                    include_matched = true;
                }
            }
        }
    }

    if has_include_filters { include_matched } else { true }
}

/// Rule that applies text filtering
pub struct FilterRule {
    patterns: Arc<Vec<FilterPattern>>,
    always_visible: Arc<HashSet<usize>>,
}

impl FilterRule {
    pub fn new(patterns: Arc<Vec<FilterPattern>>, always_visible: Arc<HashSet<usize>>) -> Self {
        Self {
            patterns,
            always_visible,
        }
    }
}

impl VisibilityRule for FilterRule {
    fn is_visible(&self, line: &LogLine) -> bool {
        if self.always_visible.contains(&line.index) {
            return true;
        }
        if self.patterns.is_empty() {
            true
        } else {
            apply_filters(line.content(), &self.patterns)
        }
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
        assert_eq!(filter.get_mode(), ActiveFilterMode::Include);
        filter.toggle_mode();
        assert_eq!(filter.get_mode(), ActiveFilterMode::Exclude);
        filter.toggle_mode();
        assert_eq!(filter.get_mode(), ActiveFilterMode::Include);
    }

    #[test]
    fn test_remove_pattern_deletes_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        filter.remove_pattern(1);
        assert_eq!(filter.get_filter_patterns().len(), 1);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
    }

    #[test]
    fn test_update_pattern_succeeds_with_unique_pattern() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        let success = filter.update_pattern(1, "INFO");
        assert!(success);
        assert_eq!(filter.get_filter_patterns()[1].pattern, "INFO");
    }

    #[test]
    fn test_update_pattern_prevents_duplicates() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR");
        filter.add_filter_from_pattern("WARNING");
        let success = filter.update_pattern(1, "ERROR");
        assert!(!success);
        assert_eq!(filter.get_filter_patterns()[0].pattern, "ERROR");
        assert_eq!(filter.get_filter_patterns()[1].pattern, "WARNING");
    }

    #[test]
    fn test_update_pattern_allows_same_pattern_different_mode() {
        let mut filter = Filter::default();
        filter.add_filter_from_pattern("ERROR"); // Include mode
        filter.toggle_mode();
        filter.add_filter_from_pattern("WARNING"); // Exclude mode
        let success = filter.update_pattern(1, "ERROR");
        assert!(success);
        assert_eq!(filter.get_filter_patterns()[1].pattern, "ERROR");
        assert_eq!(filter.get_filter_patterns()[1].mode, ActiveFilterMode::Exclude);
    }
}
