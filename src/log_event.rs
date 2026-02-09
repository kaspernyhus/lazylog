use crate::log::{LogBuffer, LogLine};
use crate::matcher::{PatternMatcher, PlainMatch};

use rayon::prelude::*;
use std::collections::HashSet;
use std::iter::once;
use std::sync::Arc;

/// A log event occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEvent {
    /// Name of the event that matched.
    pub name: String,
    /// Line number where the event occurred.
    pub line_index: usize,
}

/// An event pattern for matching and tracking.
#[derive(Debug, Clone)]
pub struct EventPattern {
    pub name: String,
    pub matcher: PatternMatcher,
    pub enabled: bool,
    pub count: usize,
    /// Whether this event is critical (shown with special indicators).
    pub critical: bool,
    /// Whether this is a custom event.
    pub is_custom: bool,
}

#[derive(Debug)]
pub struct EventState {
    pub name: String,
    pub enabled: bool,
    pub count: usize,
}

/// Manages log event tracking and scanning.
#[derive(Debug, Default)]
pub struct LogEventTracker {
    /// Event patterns
    patterns: Vec<EventPattern>,
    /// All events sorted by line_index.
    events: Vec<LogEvent>,
    /// Whether to show marks in the events view
    pub show_marks: bool,
}

impl LogEventTracker {
    /// Creates a new empty log event tracker.
    pub fn new(patterns: Vec<EventPattern>) -> Self {
        Self {
            patterns,
            events: Vec::new(),
            show_marks: false,
        }
    }

    /// Scans all log lines for event matches.
    pub fn scan_all_lines(&mut self, log_buffer: &LogBuffer) {
        self.events.clear();
        self.reset_event_counts();

        self.events = self.scan_lines(log_buffer.iter());

        for event in &self.events {
            if let Some(pattern) = self.patterns.iter_mut().find(|p| p.name == event.name) {
                pattern.count += 1;
            }
        }
    }

    /// Checks a single line for event matches and adds it if it matches.
    ///
    /// Returns true if an event was added and should be selected in the events list
    pub fn scan_single_line(&mut self, log_line: &LogLine) -> bool {
        let new_events = self.scan_lines(once(log_line));

        if new_events.is_empty() {
            return false;
        }

        let mut should_select = false;
        for event in new_events {
            // Update count for this pattern
            if let Some(pattern) = self.patterns.iter_mut().find(|p| p.name == event.name) {
                pattern.count += 1;
                if pattern.enabled {
                    should_select = true;
                }
            }
            self.events.push(event);
        }

        should_select
    }

    // Scans log lines in parallel for event pattern matches.
    // Returns ALL matching events regardless of enabled state (filtering happens elsewhere).
    fn scan_lines<'a>(&self, lines: impl Iterator<Item = &'a LogLine>) -> Vec<LogEvent> {
        let patterns = Arc::new(self.patterns.clone());
        let lines_vec: Vec<&LogLine> = lines.collect();

        let mut events: Vec<LogEvent> = lines_vec
            .par_iter()
            .filter_map(|log_line| {
                // Scan all patterns to find matches (not just enabled ones)
                for pattern in patterns.iter() {
                    if pattern.matcher.matches(log_line.content()) {
                        return Some(LogEvent {
                            name: pattern.name.clone(),
                            line_index: log_line.index,
                        });
                    }
                }
                None
            })
            .collect();

        // Sort by line_index to maintain chronological order
        events.sort_by_key(|e| e.line_index);
        events
    }

    /// Reset event counts
    fn reset_event_counts(&mut self) {
        for pattern in &mut self.patterns {
            pattern.count = 0;
        }
    }

    /// Returns all log events.
    pub fn get_events(&self) -> &[LogEvent] {
        &self.events
    }

    /// Returns enabled events.
    pub fn get_enabled_events(&self) -> Vec<&LogEvent> {
        let enabled_names: HashSet<&str> = self
            .patterns
            .iter()
            .filter(|p| p.enabled)
            .map(|p| p.name.as_str())
            .collect();

        self.events
            .iter()
            .filter(|e| enabled_names.contains(e.name.as_str()))
            .collect()
    }

    /// Returns a set of all line indices that contain events.
    pub fn get_event_indices(&self) -> HashSet<usize> {
        self.events.iter().map(|e| e.line_index).collect()
    }

    /// Returns a set of line indices that contain critical events.
    pub fn get_critical_event_indices(&self) -> HashSet<usize> {
        let critical_names: HashSet<&str> = self
            .patterns
            .iter()
            .filter(|p| p.critical)
            .map(|p| p.name.as_str())
            .collect();

        self.events
            .iter()
            .filter(|e| critical_names.contains(e.name.as_str()))
            .map(|e| e.line_index)
            .collect()
    }

    /// Returns a set of line indices that contain custom events.
    pub fn get_custom_event_indices(&self) -> HashSet<usize> {
        let custom_names: HashSet<&str> = self
            .patterns
            .iter()
            .filter(|p| p.is_custom)
            .map(|p| p.name.as_str())
            .collect();

        self.events
            .iter()
            .filter(|e| custom_names.contains(e.name.as_str()))
            .map(|e| e.line_index)
            .collect()
    }

    /// Returns true if an event with the given name is marked as critical.
    pub fn is_critical_event(&self, event_name: &str) -> bool {
        self.patterns.iter().any(|p| p.name == event_name && p.critical)
    }

    /// Returns true if an event with the given name is a custom event.
    pub fn is_custom_event(&self, event_name: &str) -> bool {
        self.patterns.iter().any(|p| p.name == event_name && p.is_custom)
    }

    pub fn clear_all(&mut self) {
        self.events.clear();
        for pattern in &mut self.patterns {
            pattern.count = 0;
        }
    }

    /// Returns the number of filtered events.
    pub fn count(&self) -> usize {
        self.get_events().len()
    }

    /// Returns true if no filtered events are visible.
    pub fn is_empty(&self) -> bool {
        self.get_events().is_empty()
    }

    /// Toggle whether to show marks in event view.
    pub fn toggle_show_marks(&mut self) -> bool {
        self.show_marks = !self.show_marks;
        self.show_marks
    }

    /// Returns true if any event pattern is disabled ie event filtering is active.
    pub fn has_event_filtering(&self) -> bool {
        self.patterns.iter().any(|p| !p.enabled)
    }

    /// Whether marks are being showed in events list.
    pub fn showing_marks(&self) -> bool {
        self.show_marks
    }

    /// Returns a list of events sorted by count: (name, enabled, count).
    pub fn get_event_stats(&self) -> Vec<EventState> {
        let mut event_stats: Vec<EventState> = self
            .patterns
            .iter()
            .map(|p| EventState {
                name: p.name.clone(),
                enabled: p.enabled,
                count: p.count,
            })
            .collect();

        // Sort by count (descending)
        event_stats.sort_by(|a, b| {
            let count_a = a.count;
            let count_b = b.count;
            count_b.cmp(&count_a)
        });

        event_stats
    }

    /// Returns the total count of events for a specific event name.
    pub fn get_event_count(&self, event_name: &str) -> usize {
        self.patterns
            .iter()
            .find(|p| p.name == event_name)
            .map(|p| p.count)
            .unwrap_or(0)
    }

    /// Gets the total number of filters.
    pub fn filter_count(&self) -> usize {
        self.patterns.len()
    }

    /// Toggles the event enabled status.
    pub fn toggle_event_enabled(&mut self, event_name: &str) {
        if let Some(pattern) = self.patterns.iter_mut().find(|p| p.name == *event_name) {
            pattern.enabled = !pattern.enabled;
        }
    }

    /// Toggles all event filters on or off.
    pub fn toggle_all_filters(&mut self) {
        let all_enabled = self.patterns.iter().all(|p| p.enabled);
        let new_state = !all_enabled;
        for pattern in &mut self.patterns {
            pattern.enabled = new_state;
        }
    }

    /// Enables only the specified event filter, disabling all others.
    pub fn solo_event_filter(&mut self, event_name: &str) {
        for pattern in &mut self.patterns {
            pattern.enabled = pattern.name == event_name;
        }
    }

    /// Restores event filter states from persisted state.
    pub fn restore_filter_states(&mut self, filter_states: &[(String, bool)]) {
        for (name, enabled) in filter_states {
            if let Some(pattern) = self.patterns.iter_mut().find(|p| p.name == *name) {
                pattern.enabled = *enabled;
            }
        }
    }

    /// Adds a custom event pattern. Returns false if the pattern already exists.
    pub fn add_custom_event(&mut self, pattern: &str) -> bool {
        if pattern.is_empty() {
            return false;
        }

        // Check if pattern already exists (either as custom or config event)
        let pattern_exists = self.patterns.iter().any(|p| {
            if let PatternMatcher::Plain(plain) = &p.matcher {
                plain.pattern == pattern
            } else {
                false
            }
        });

        if pattern_exists {
            return false;
        }

        // Create name from pattern, capped at 16 characters
        let name = if pattern.len() > 16 {
            format!("{}...", &pattern[..13])
        } else {
            pattern.to_string()
        };

        let event_pattern = EventPattern {
            name,
            matcher: PatternMatcher::Plain(PlainMatch {
                pattern: pattern.to_string(),
                case_sensitive: true,
            }),
            enabled: true,
            count: 0,
            critical: false,
            is_custom: true,
        };

        self.patterns.push(event_pattern);
        true
    }

    /// Returns the patterns of all custom events (for persistence).
    pub fn get_custom_event_patterns(&self) -> Vec<&str> {
        self.patterns
            .iter()
            .filter(|p| p.is_custom)
            .filter_map(|p| {
                if let PatternMatcher::Plain(plain) = &p.matcher {
                    Some(plain.pattern.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Removes a custom event by name. Returns the pattern string if found.
    pub fn remove_custom_event(&mut self, name: &str) -> Option<String> {
        let pattern_str = self
            .patterns
            .iter()
            .find(|p| p.is_custom && p.name == name)
            .and_then(|p| {
                if let PatternMatcher::Plain(plain) = &p.matcher {
                    Some(plain.pattern.clone())
                } else {
                    None
                }
            });

        self.patterns.retain(|p| !(p.is_custom && p.name == name));
        self.events.retain(|e| e.name != name);

        pattern_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::LogBuffer;

    fn create_test_patterns() -> Vec<EventPattern> {
        vec![
            EventPattern {
                name: "error".to_string(),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "ERROR".to_string(),
                    case_sensitive: true,
                }),
                enabled: true,
                count: 0,
                critical: false,
                is_custom: false,
            },
            EventPattern {
                name: "warning".to_string(),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "WARN".to_string(),
                    case_sensitive: true,
                }),
                enabled: true,
                count: 0,
                critical: false,
                is_custom: false,
            },
            EventPattern {
                name: "info".to_string(),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "INFO".to_string(),
                    case_sensitive: true,
                }),
                enabled: true,
                count: 0,
                critical: false,
                is_custom: false,
            },
        ]
    }

    fn create_test_log_buffer() -> LogBuffer {
        let mut buffer = LogBuffer::default();
        buffer.append_line("INFO: Starting application".to_string());
        buffer.append_line("ERROR: Failed to connect".to_string());
        buffer.append_line("WARN: Retrying connection".to_string());
        buffer.append_line("INFO: Connection established".to_string());
        buffer.append_line("ERROR: Timeout occurred".to_string());
        buffer
    }

    #[test]
    fn test_new_tracker_is_empty() {
        let patterns = create_test_patterns();
        let tracker = LogEventTracker::new(patterns);
        assert!(tracker.is_empty());
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_scan_all_lines_finds_events() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        assert_eq!(tracker.count(), 5);
        assert!(!tracker.is_empty());
    }

    #[test]
    fn test_scan_all_lines_counts_events_by_type() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        assert_eq!(tracker.get_event_count("error"), 2);
        assert_eq!(tracker.get_event_count("warning"), 1);
        assert_eq!(tracker.get_event_count("info"), 2);
    }

    #[test]
    fn test_get_event_stats_sorted_by_count() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        let stats = tracker.get_event_stats();
        assert_eq!(stats.len(), 3);

        // Should be sorted by count descending
        let counts: Vec<_> = stats.iter().map(|s| s.count).collect();
        assert!(counts[0] >= counts[1]);
        assert!(counts[1] >= counts[2]);

        // error and info both have count 2, warning has 1
        assert!(stats.iter().any(|s| s.name == "error" && s.count == 2));
        assert!(stats.iter().any(|s| s.name == "warning" && s.count == 1));
        assert!(stats.iter().any(|s| s.name == "info" && s.count == 2));
    }

    #[test]
    fn test_toggle_event_enabled() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        // Initially all events should be enabled
        let enabled_before = tracker.get_enabled_events();
        assert_eq!(enabled_before.len(), 5);

        // Toggle error off
        tracker.toggle_event_enabled("error");

        // Now only 3 events should be enabled (2 info, 1 warning)
        let enabled_after = tracker.get_enabled_events();
        assert_eq!(enabled_after.len(), 3);

        // Counts should still be accurate
        assert_eq!(tracker.get_event_count("error"), 2);
    }

    #[test]
    fn test_toggle_all_filters() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        // Toggle all off
        tracker.toggle_all_filters();

        let stats = tracker.get_event_stats();
        assert!(stats.iter().all(|s| !s.enabled));

        // Toggle all back on
        tracker.toggle_all_filters();

        let stats = tracker.get_event_stats();
        assert!(stats.iter().all(|s| s.enabled));
    }

    #[test]
    fn test_restore_filter_states() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);

        let saved_states = vec![
            ("error".to_string(), false),
            ("warning".to_string(), true),
            ("info".to_string(), false),
        ];

        tracker.restore_filter_states(&saved_states);

        let stats = tracker.get_event_stats();
        let error_state = stats.iter().find(|s| s.name == "error").unwrap();
        let warning_state = stats.iter().find(|s| s.name == "warning").unwrap();
        let info_state = stats.iter().find(|s| s.name == "info").unwrap();

        assert!(!error_state.enabled);
        assert!(warning_state.enabled);
        assert!(!info_state.enabled);
    }

    #[test]
    fn test_toggle_preserves_events() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);
        let initial_count = tracker.count();

        // Toggling a filter should not affect the total event count
        tracker.toggle_event_enabled("error");
        assert_eq!(tracker.count(), initial_count);

        // But it should affect enabled events
        let enabled = tracker.get_enabled_events();
        assert_eq!(enabled.len(), 3); // 2 info + 1 warning, no error
    }

    #[test]
    fn test_get_enabled_events_filters_correctly() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        // Disable warning
        tracker.toggle_event_enabled("warning");

        let enabled = tracker.get_enabled_events();
        // Should have 4 events (2 error + 2 info, no warning)
        assert_eq!(enabled.len(), 4);
        assert!(enabled.iter().all(|e| e.name != "warning"));
    }

    #[test]
    fn test_clear_all() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);
        assert_eq!(tracker.count(), 5);

        tracker.clear_all();
        assert_eq!(tracker.count(), 0);
        assert!(tracker.is_empty());

        // Counts should be reset
        assert_eq!(tracker.get_event_count("error"), 0);
        assert_eq!(tracker.get_event_count("warning"), 0);
        assert_eq!(tracker.get_event_count("info"), 0);
    }

    #[test]
    fn test_filter_count() {
        let patterns = create_test_patterns();
        let tracker = LogEventTracker::new(patterns);

        assert_eq!(tracker.filter_count(), 3);
    }

    #[test]
    fn test_scan_single_line_increments_count() {
        let patterns = create_test_patterns();
        let mut tracker = LogEventTracker::new(patterns);
        let buffer = create_test_log_buffer();

        tracker.scan_all_lines(&buffer);

        let initial_error_count = tracker.get_event_count("error");

        // Add another ERROR line
        let mut temp_buffer = LogBuffer::default();
        temp_buffer.append_line("ERROR: Another error".to_string());
        let log_line = temp_buffer.get_line(0).unwrap();

        tracker.scan_single_line(log_line);

        assert_eq!(tracker.get_event_count("error"), initial_error_count + 1);
    }
}
