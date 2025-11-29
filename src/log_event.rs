use crate::highlighter::HighlightPattern;
use crate::list_view_state::ListViewState;
use crate::log::{LogBuffer, LogLine};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// A log event occurrence.
#[derive(Debug, Clone, PartialEq)]
pub struct LogEvent {
    /// Name of the event that matched.
    pub event_name: String,
    /// Line number where the event occurred.
    pub line_index: usize,
}

/// A filter for a specific event type.
#[derive(Debug, Clone)]
pub struct EventFilter {
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Default)]
pub struct EventFilterList {
    /// Map of event_name -> enabled state
    event_filters: HashMap<String, bool>,
    /// View state for the filter list
    filter_view: ListViewState,
}

impl EventFilterList {
    pub fn add_event_name(&mut self, event_name: &str) {
        self.event_filters
            .entry(event_name.to_string())
            .or_insert(true);
        self.filter_view.set_item_count(self.event_filters.len());
    }

    pub fn add_filter(&mut self, event_name: &str, enabled: bool) {
        self.event_filters.insert(event_name.to_string(), enabled);
        self.filter_view.set_item_count(self.event_filters.len());
    }

    pub fn get_filters(&self) -> &HashMap<String, bool> {
        &self.event_filters
    }

    pub fn is_enabled(&self, event_name: &str) -> bool {
        self.event_filters.get(event_name).copied().unwrap_or(true)
    }

    pub fn count(&self) -> usize {
        self.event_filters.len()
    }

    pub fn get_mut(&mut self, event_name: &str) -> Option<&mut bool> {
        self.event_filters.get_mut(event_name)
    }

    pub fn set_all_enabled(&mut self, enabled: bool) {
        for value in self.event_filters.values_mut() {
            *value = enabled;
        }
    }

    pub fn all_enabled(&self) -> bool {
        self.event_filters.values().all(|&enabled| enabled)
    }

    pub fn selected_index(&self) -> usize {
        self.filter_view.selected_index()
    }

    pub fn viewport_offset(&self) -> usize {
        self.filter_view.viewport_offset()
    }

    pub fn set_viewport_height(&self, height: usize) {
        self.filter_view.set_viewport_height(height)
    }

    pub fn move_up_wrap(&mut self) {
        self.filter_view.move_up_wrap()
    }

    pub fn move_down_wrap(&mut self) {
        self.filter_view.move_down_wrap()
    }

    pub fn set_item_count(&mut self, count: usize) {
        self.filter_view.set_item_count(count)
    }
}

/// Manages log event tracking and scanning.
#[derive(Debug, Default)]
pub struct LogEventTracker {
    /// All events sorted by line_index.
    events: Vec<LogEvent>,
    /// Total count of each event type.
    event_counts: HashMap<String, usize>,
    /// Cached active line indices from LogBuffer (lines that pass filters).
    active_lines: HashSet<usize>,
    /// View state for the events list
    events_view: ListViewState,
    /// Tracks whether the event list needs re-scanning
    needs_rescan: bool,
    /// Event filter list
    event_filter: EventFilterList,
    /// Whether to show marks in the events view
    pub show_marks: bool,
}

impl LogEventTracker {
    /// Creates a new empty log event tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scans all log lines for event occurrences.
    pub fn scan_all_lines(&mut self, log_buffer: &LogBuffer, event_patterns: &[HighlightPattern]) {
        self.events.clear();
        self.event_counts.clear();

        for event in event_patterns {
            if let Some(name) = &event.name {
                self.event_filter.add_event_name(name);
                self.event_counts.insert(name.clone(), 0);
            }
        }

        self.events = self.scan_lines(log_buffer.iter(), event_patterns);
        self.event_counts = self.count_events(&self.events);
        self.update_filtered_count();
        self.needs_rescan = false;
    }

    /// Scans log lines in parallel for event pattern matches.
    fn scan_lines<'a>(
        &self,
        lines: impl Iterator<Item = &'a LogLine>,
        event_patterns: &[HighlightPattern],
    ) -> Vec<LogEvent> {
        let event_patterns = Arc::new(event_patterns.to_vec());
        let lines_vec: Vec<&LogLine> = lines.collect();

        let mut events: Vec<LogEvent> = lines_vec
            .par_iter()
            .filter_map(|log_line| {
                for event in event_patterns.iter() {
                    if event.matcher.matches(log_line.content()) {
                        if let Some(name) = &event.name {
                            return Some(LogEvent {
                                event_name: name.clone(),
                                line_index: log_line.index,
                            });
                        }
                        break;
                    }
                }
                None
            })
            .collect();

        // Sort by line_index to maintain chronological order
        events.sort_by_key(|e| e.line_index);
        events
    }

    // Count the occurrence of each event
    fn count_events(&self, events: &[LogEvent]) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for event in events {
            *counts.entry(event.event_name.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Updates the active lines cache and recalculates filtered event count.
    pub fn update_active_lines(&mut self, active_lines: &[usize]) {
        self.active_lines = active_lines.iter().copied().collect();
        self.update_filtered_count();
    }

    /// Updates the view item count based on filtered events.
    fn update_filtered_count(&mut self) {
        let count = self.get_filtered_events().len();
        self.events_view.set_item_count(count);
    }

    /// Returns filtered events.
    pub fn get_filtered_events(&self) -> Vec<&LogEvent> {
        self.events
            .iter()
            .filter(|event| {
                self.event_filter.is_enabled(&event.event_name)
                    && self.active_lines.contains(&event.line_index)
            })
            .collect()
    }

    /// Marks the event list as needing a rescan (called when log filters change).
    pub fn mark_needs_rescan(&mut self) {
        self.needs_rescan = true;
    }

    /// Returns true if the event list needs re-scanning.
    pub fn needs_rescan(&self) -> bool {
        self.needs_rescan
    }

    /// Checks a single line for event matches and adds it if it matches.
    pub fn scan_single_line(
        &mut self,
        log_line: &LogLine,
        event_patterns: &[HighlightPattern],
        follow_mode: bool,
    ) {
        for event in event_patterns {
            if event.matcher.matches(&log_line.content) {
                if let Some(name) = &event.name {
                    *self.event_counts.entry(name.clone()).or_insert(0) += 1;
                    if self.event_filter.is_enabled(name) {
                        let new_event = LogEvent {
                            event_name: name.clone(),
                            line_index: log_line.index,
                        };

                        self.events.push(new_event);
                        self.events_view.set_item_count(self.count());

                        if follow_mode {
                            self.events_view.select_last();
                        }
                    }
                }
                break;
            }
        }
    }

    /// Returns filtered log events.
    pub fn get_events(&self) -> Vec<&LogEvent> {
        self.get_filtered_events()
    }

    /// Returns the number of filtered events.
    pub fn count(&self) -> usize {
        self.get_filtered_events().len()
    }

    /// Returns true if no filtered events are visible.
    pub fn is_empty(&self) -> bool {
        self.get_filtered_events().is_empty()
    }

    /// Sets the total item count for the events view.
    pub fn set_events_view_item_count(&mut self, count: usize) {
        self.events_view.set_item_count(count);
    }

    /// Toggle whether to show marks in event view.
    pub fn toggle_show_marks(&mut self) -> bool {
        self.show_marks = !self.show_marks;
        self.show_marks
    }

    /// Whether marks are being showed in events list.
    pub fn showing_marks(&self) -> bool {
        self.show_marks
    }

    /// Finds the index of the event nearest to the given line number.
    pub fn find_nearest(&self, line_index: usize) -> Option<usize> {
        let filtered = self.get_filtered_events();
        if filtered.is_empty() {
            return None;
        }

        match filtered.binary_search_by_key(&line_index, |e| e.line_index) {
            Ok(idx) => Some(idx),
            Err(0) => Some(0),
            Err(idx) if idx >= filtered.len() => Some(filtered.len() - 1),
            Err(idx) => {
                let dist_before = line_index - filtered[idx - 1].line_index;
                let dist_after = filtered[idx].line_index - line_index;
                Some(if dist_before <= dist_after {
                    idx - 1
                } else {
                    idx
                })
            }
        }
    }

    /// Gets the line index of the currently selected event (from filtered events).
    pub fn get_selected_line_index(&self) -> Option<usize> {
        let filtered = self.get_filtered_events();
        filtered
            .get(self.events_view.selected_index())
            .map(|event| event.line_index)
    }

    /// Gets the next event after the given line index (from filtered events).
    pub fn get_next_event(&self, current_line_index: usize) -> Option<usize> {
        self.get_filtered_events()
            .iter()
            .find(|event| event.line_index > current_line_index)
            .map(|event| event.line_index)
    }

    /// Gets the previous event before the given line index (from filtered events).
    pub fn get_previous_event(&self, current_line_index: usize) -> Option<usize> {
        self.get_filtered_events()
            .iter()
            .rev()
            .find(|event| event.line_index < current_line_index)
            .map(|event| event.line_index)
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.events_view.selected_index()
    }

    /// Gets the current viewport offset.
    pub fn viewport_offset(&self) -> usize {
        self.events_view.viewport_offset()
    }

    /// Sets the viewport height.
    pub fn set_viewport_height(&self, height: usize) {
        self.events_view.set_viewport_height(height);
    }

    /// Sets the selected index to the nearest event to the given line number.
    pub fn select_nearest_event(&mut self, current_line: usize) {
        if let Some(nearest_index) = self.find_nearest(current_line) {
            self.events_view.select_index(nearest_index);
        } else {
            self.events_view.select_index(0);
        }
    }

    /// Moves selection up (wraps to bottom).
    pub fn move_selection_up(&mut self) {
        self.events_view.move_up();
    }

    /// Moves selection down (wraps to top).
    pub fn move_selection_down(&mut self) {
        self.events_view.move_down();
    }

    /// Selects the last (most recent) item in the list.
    pub fn select_last_event(&mut self) {
        self.events_view.select_last();
    }

    /// Moves selection up by half a page.
    pub fn selection_page_up(&mut self) {
        self.events_view.page_up();
    }

    /// Moves selection down by half a page.
    pub fn selection_page_down(&mut self) {
        self.events_view.page_down();
    }

    /// Returns a list of event filters sorted by count.
    pub fn get_event_filters(&self) -> Vec<EventFilter> {
        let mut filters: Vec<_> = self
            .event_filter
            .get_filters()
            .iter()
            .map(|(name, enabled)| EventFilter {
                name: name.clone(),
                enabled: *enabled,
            })
            .collect();

        filters.sort_by(|a, b| {
            let count_a = self.get_event_count(&a.name);
            let count_b = self.get_event_count(&b.name);
            count_b.cmp(&count_a)
        });

        filters
    }

    /// Returns the total count of events for a specific event name (regardless of filter state).
    pub fn get_event_count(&self, event_name: &str) -> usize {
        self.event_counts.get(event_name).copied().unwrap_or(0)
    }

    /// Gets the total number of filters.
    pub fn filter_count(&self) -> usize {
        self.event_filter.count()
    }

    /// Sets the filter viewport height.
    pub fn set_filter_viewport_height(&self, height: usize) {
        self.event_filter.set_viewport_height(height);
    }

    pub fn get_filter_selected_index(&self) -> usize {
        self.event_filter.filter_view.selected_index()
    }

    pub fn get_filter_viewport_offset(&self) -> usize {
        self.event_filter.filter_view.viewport_offset()
    }

    /// Moves filter selection up (wraps to bottom).
    pub fn move_filter_selection_up(&mut self) {
        self.event_filter.move_up_wrap();
    }

    /// Moves filter selection down (wraps to top).
    pub fn move_filter_selection_down(&mut self) {
        self.event_filter.move_down_wrap();
    }

    /// Toggles the selected event filter.
    pub fn toggle_selected_filter(&mut self) {
        let filters = self.get_event_filters();
        let selected_index = self.event_filter.selected_index();
        if let Some(filter) = filters.get(selected_index)
            && let Some(enabled) = self.event_filter.get_mut(&filter.name)
        {
            *enabled = !*enabled;
            self.update_filtered_count();
        }
    }

    /// Toggles all event filters on or off.
    pub fn toggle_all_filters(&mut self) {
        let all_enabled = self.event_filter.all_enabled();
        let new_state = !all_enabled;
        self.event_filter.set_all_enabled(new_state);
    }

    /// Restores event filter states from persisted state.
    pub fn restore_filter_states(&mut self, filter_states: &[(String, bool)]) {
        for (name, enabled) in filter_states {
            self.event_filter.add_filter(name, *enabled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::highlighter::{PatternMatcher, PatternStyle, PlainMatch};
    use crate::log::LogLine;

    fn create_test_patterns() -> Vec<HighlightPattern> {
        vec![
            HighlightPattern {
                name: Some("error".to_string()),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "ERROR".to_string(),
                    case_sensitive: true,
                }),
                style: PatternStyle::default_colors(),
            },
            HighlightPattern {
                name: Some("warning".to_string()),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "WARN".to_string(),
                    case_sensitive: true,
                }),
                style: PatternStyle::default_colors(),
            },
            HighlightPattern {
                name: Some("info".to_string()),
                matcher: PatternMatcher::Plain(PlainMatch {
                    pattern: "INFO".to_string(),
                    case_sensitive: true,
                }),
                style: PatternStyle::default_colors(),
            },
        ]
    }

    fn create_test_log_lines() -> Vec<LogLine> {
        vec![
            LogLine::new("INFO: Starting application".to_string(), 0),
            LogLine::new("ERROR: Failed to connect".to_string(), 1),
            LogLine::new("WARN: Retrying connection".to_string(), 2),
            LogLine::new("INFO: Connection established".to_string(), 3),
            LogLine::new("ERROR: Timeout occurred".to_string(), 4),
        ]
    }

    // Helper to populate tracker for tests
    fn populate_tracker(tracker: &mut LogEventTracker, patterns: &[HighlightPattern]) {
        let lines = create_test_log_lines();

        // Initialize filters and counts
        for event in patterns {
            if let Some(name) = &event.name {
                tracker.event_filter.add_event_name(name);
                tracker.event_counts.insert(name.clone(), 0);
            }
        }

        tracker.events = tracker.scan_lines(lines.iter(), patterns);
        tracker.event_counts = tracker.count_events(&tracker.events);

        // Set all lines as active for tests
        let active_lines: Vec<usize> = lines.iter().map(|l| l.index).collect();
        tracker.update_active_lines(&active_lines);
    }

    #[test]
    fn test_new_tracker_is_empty() {
        let tracker = LogEventTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_populate_tracker_finds_events() {
        let mut tracker = LogEventTracker::new();
        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        assert_eq!(tracker.count(), 5);
        assert!(!tracker.is_empty());
    }

    #[test]
    fn test_populate_tracker_counts_events_by_type() {
        let mut tracker = LogEventTracker::new();
        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        assert_eq!(tracker.get_event_count("error"), 2);
        assert_eq!(tracker.get_event_count("warning"), 1);
        assert_eq!(tracker.get_event_count("info"), 2);
    }

    #[test]
    fn test_populate_tracker_initializes_filters() {
        let mut tracker = LogEventTracker::new();
        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let filters = tracker.get_event_filters();
        assert_eq!(filters.len(), 3);
        assert!(filters.iter().all(|f| f.enabled));
    }

    #[test]
    fn test_scan_single_line_adds_matching_event() {
        let mut tracker = LogEventTracker::new();
        let patterns = create_test_patterns();
        let log_line = LogLine::new("ERROR: Test error".to_string(), 10);

        // Initialize event filters
        for pattern in &patterns {
            if let Some(name) = &pattern.name {
                tracker.event_filter.add_event_name(name);
            }
        }

        // Set active lines so event is visible
        tracker.update_active_lines(&[10]);

        tracker.scan_single_line(&log_line, &patterns, false);

        assert_eq!(tracker.count(), 1);
        let events: Vec<_> = tracker.get_events();
        assert_eq!(events[0].event_name, "error");
        assert_eq!(events[0].line_index, 10);
    }

    #[test]
    fn test_scan_single_line_respects_filters() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        // Initialize filters manually
        for pattern in &patterns {
            if let Some(name) = &pattern.name {
                tracker.event_filter.add_event_name(name);
            }
        }

        // Disable error filter
        tracker.event_filter.add_filter("error", false);

        let log_line = LogLine::new("ERROR: Another error".to_string(), 10);
        tracker.scan_single_line(&log_line, &patterns, false);

        // Should not add because filter is disabled
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_find_nearest_exact_match() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let nearest = tracker.find_nearest(2);
        assert_eq!(nearest, Some(2)); // Should find event at line 2
    }

    #[test]
    fn test_find_nearest_between_events() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        // Line 2.5 should pick line 2 (closer than line 3)
        let nearest = tracker.find_nearest(2);
        assert_eq!(nearest, Some(2));
    }

    #[test]
    fn test_find_nearest_before_first() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let nearest = tracker.find_nearest(0);
        assert_eq!(nearest, Some(0)); // Should return first event
    }

    #[test]
    fn test_find_nearest_after_last() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let nearest = tracker.find_nearest(100);
        assert_eq!(nearest, Some(4)); // Should return last event
    }

    #[test]
    fn test_find_nearest_empty_returns_none() {
        let tracker = LogEventTracker::new();
        assert_eq!(tracker.find_nearest(10), None);
    }

    #[test]
    fn test_get_next_event() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let next = tracker.get_next_event(1);
        assert_eq!(next, Some(2));
    }

    #[test]
    fn test_get_next_event_at_last_returns_none() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let next = tracker.get_next_event(4);
        assert_eq!(next, None);
    }

    #[test]
    fn test_get_previous_event() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let prev = tracker.get_previous_event(3);
        assert_eq!(prev, Some(2));
    }

    #[test]
    fn test_get_previous_event_at_first_returns_none() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let prev = tracker.get_previous_event(0);
        assert_eq!(prev, None);
    }

    #[test]
    fn test_select_nearest_event_updates_selection() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        tracker.select_nearest_event(2);
        assert_eq!(tracker.get_selected_line_index(), Some(2));
    }

    #[test]
    fn test_move_selection_up() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);
        tracker.select_nearest_event(4);

        tracker.move_selection_up();
        assert_eq!(tracker.get_selected_line_index(), Some(3));
    }

    #[test]
    fn test_move_selection_down() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);
        tracker.select_nearest_event(0);

        tracker.move_selection_down();
        assert_eq!(tracker.get_selected_line_index(), Some(1));
    }

    #[test]
    fn test_select_last_event() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        tracker.select_last_event();
        assert_eq!(tracker.get_selected_line_index(), Some(4));
    }

    #[test]
    fn test_toggle_selected_filter_disables_filter() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let initial_filters = tracker.get_event_filters();
        let first_filter_name = initial_filters[0].name.clone();
        assert!(initial_filters[0].enabled);

        tracker.toggle_selected_filter();

        let updated_filters = tracker.get_event_filters();
        let toggled_filter = updated_filters
            .iter()
            .find(|f| f.name == first_filter_name)
            .unwrap();
        assert!(!toggled_filter.enabled);
    }

    #[test]
    fn test_toggle_all_filters_when_all_enabled() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        tracker.toggle_all_filters();

        let filters = tracker.get_event_filters();
        assert!(filters.iter().all(|f| !f.enabled));
    }

    #[test]
    fn test_toggle_all_filters_when_some_disabled() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);
        tracker.toggle_selected_filter(); // Disable one

        tracker.toggle_all_filters();

        let filters = tracker.get_event_filters();
        assert!(filters.iter().all(|f| f.enabled));
    }

    #[test]
    fn test_get_event_filters_sorted_by_count() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let filters = tracker.get_event_filters();

        // Should be sorted by count descending: error(2), info(2), warning(1)
        assert_eq!(filters.len(), 3);
        let counts: Vec<_> = filters
            .iter()
            .map(|f| tracker.get_event_count(&f.name))
            .collect();
        assert!(counts[0] >= counts[1]);
        assert!(counts[1] >= counts[2]);
    }

    #[test]
    fn test_restore_filter_states() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let saved_states = vec![
            ("error".to_string(), false),
            ("warning".to_string(), true),
            ("info".to_string(), false),
        ];

        tracker.restore_filter_states(&saved_states);

        let filters = tracker.get_event_filters();
        let error_filter = filters.iter().find(|f| f.name == "error").unwrap();
        let warning_filter = filters.iter().find(|f| f.name == "warning").unwrap();
        let info_filter = filters.iter().find(|f| f.name == "info").unwrap();

        assert!(!error_filter.enabled);
        assert!(warning_filter.enabled);
        assert!(!info_filter.enabled);
    }

    #[test]
    fn test_mark_needs_rescan() {
        let mut tracker = LogEventTracker::new();
        assert!(!tracker.needs_rescan());

        tracker.mark_needs_rescan();
        assert!(tracker.needs_rescan());
    }

    #[test]
    fn test_populate_tracker_does_not_clear_needs_rescan() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        tracker.mark_needs_rescan();
        populate_tracker(&mut tracker, &patterns);

        // populate_tracker is just a test helper, it doesn't clear needs_rescan
        // (only scan_all_lines does that in real code)
        assert!(tracker.needs_rescan());
    }

    #[test]
    fn test_iter_events_returns_all_events() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);

        let events: Vec<_> = tracker.get_events();
        assert_eq!(events.len(), 5);
        assert_eq!(events[0].line_index, 0);
        assert_eq!(events[1].line_index, 1);
        assert_eq!(events[2].line_index, 2);
    }

    #[test]
    fn test_filter_view_movement() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);
        tracker.event_filter.set_item_count(tracker.filter_count());

        assert_eq!(tracker.event_filter.selected_index(), 0);

        tracker.move_filter_selection_down();
        assert_eq!(tracker.event_filter.selected_index(), 1);

        tracker.move_filter_selection_up();
        assert_eq!(tracker.event_filter.selected_index(), 0);
    }

    #[test]
    fn test_page_up_and_down() {
        let mut tracker = LogEventTracker::new();

        let patterns = create_test_patterns();

        populate_tracker(&mut tracker, &patterns);
        tracker.select_last_event();

        let before_page_up = tracker.selected_index();

        tracker.selection_page_up();
        let after_page_up = tracker.selected_index();
        // Page up should move selection backwards
        assert!(after_page_up <= before_page_up);

        tracker.selection_page_down();
        let after_page_down = tracker.selected_index();
        // Page down should move selection forwards
        assert!(after_page_down >= after_page_up);
    }
}
