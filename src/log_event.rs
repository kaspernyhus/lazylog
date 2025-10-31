use crate::highlighter::HighlightPattern;
use crate::log::{Interval, LogBuffer, LogLine};
use std::collections::HashMap;

/// Information about a matched event occurrence.
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// Name of the event that matched.
    pub event_name: String,
    /// Line number where the event occurred.
    pub line_index: usize,
}

/// Filter state for an event type.
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Name of the event.
    pub name: String,
    /// Whether this event type is enabled in the filter.
    pub enabled: bool,
}

/// Manages log event tracking and scanning.
#[derive(Debug, Default)]
pub struct LogEventTracker {
    events: Vec<LogEvent>,
    selected_index: usize,
    /// Event filters - maps event name to enabled state
    event_filters: HashMap<String, bool>,
    /// Event names in config file order
    event_order: Vec<String>,
    /// Total count of each event type (regardless of filter state)
    event_counts: HashMap<String, usize>,
    /// Selected index in the filter list
    filter_selected_index: usize,
}

impl LogEventTracker {
    /// Creates a new empty log event tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scans all log lines for event occurrences and stores them.
    pub fn scan(&mut self, log_buffer: &LogBuffer, event_patterns: &[HighlightPattern]) {
        self.events.clear();
        self.event_counts.clear();

        for event in event_patterns {
            if let Some(name) = &event.name {
                if !self.event_filters.contains_key(name) {
                    self.event_order.push(name.clone());
                    self.event_filters.insert(name.clone(), true);
                }
                self.event_counts.insert(name.clone(), 0);
            }
        }

        for log_line in log_buffer.get_lines_iter(Interval::All) {
            for event in event_patterns {
                if event.matcher.matches(log_line.content()) {
                    if let Some(name) = &event.name {
                        *self.event_counts.entry(name.clone()).or_insert(0) += 1;
                        if *self.event_filters.get(name).unwrap_or(&true) {
                            self.events.push(LogEvent {
                                event_name: name.clone(),
                                line_index: log_line.index,
                            });
                        }
                    }
                    break;
                }
            }
        }
    }

    /// Checks a single line for event matches and adds it if it matches.
    pub fn scan_line(&mut self, log_line: &LogLine, event_patterns: &[HighlightPattern]) {
        for event in event_patterns {
            if event.matcher.matches(&log_line.content) {
                if let Some(name) = &event.name {
                    *self.event_counts.entry(name.clone()).or_insert(0) += 1;
                    if *self.event_filters.get(name).unwrap_or(&true) {
                        self.events.push(LogEvent {
                            event_name: name.clone(),
                            line_index: log_line.index,
                        });
                    }
                }
                break;
            }
        }
    }

    /// Returns a slice of all tracked events.
    pub fn events(&self) -> &[LogEvent] {
        &self.events
    }

    /// Returns the number of tracked events.
    pub fn count(&self) -> usize {
        self.events.len()
    }

    /// Returns true if no events are tracked.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Finds the index of the event nearest to the given line number.
    pub fn find_nearest_event(&self, current_line: usize) -> Option<usize> {
        if self.events.is_empty() {
            return None;
        }

        let mut min_distance = usize::MAX;
        let mut nearest_index = 0;

        for (idx, event) in self.events.iter().enumerate() {
            let distance = if event.line_index >= current_line {
                event.line_index - current_line
            } else {
                current_line - event.line_index
            };

            if distance < min_distance {
                min_distance = distance;
                nearest_index = idx;
            }
        }

        Some(nearest_index)
    }

    /// Gets the event at the specified index.
    pub fn get(&self, index: usize) -> Option<&LogEvent> {
        self.events.get(index)
    }

    /// Gets the currently selected event.
    pub fn get_selected_event(&self) -> Option<&LogEvent> {
        self.events.get(self.selected_index)
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Sets the selected index to the nearest event to the given line number.
    ///
    /// This also returns the index that was set, or `None` if there are no events.
    pub fn select_nearest_event(&mut self, current_line: usize) -> Option<usize> {
        if let Some(nearest_index) = self.find_nearest_event(current_line) {
            self.selected_index = nearest_index;
            Some(nearest_index)
        } else {
            self.selected_index = 0;
            None
        }
    }

    /// Moves selection up (wraps to bottom).
    pub fn move_selection_up(&mut self) {
        if !self.events.is_empty() && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves selection down (wraps to top).
    pub fn move_selection_down(&mut self) {
        if !self.events.is_empty() && self.selected_index < self.events.len() - 1 {
            self.selected_index += 1;
        }
    }

    /// Returns a list of event filters in config file order.
    pub fn get_event_filters(&self) -> Vec<EventFilter> {
        self.event_order
            .iter()
            .filter_map(|name| {
                self.event_filters.get(name).map(|enabled| EventFilter {
                    name: name.clone(),
                    enabled: *enabled,
                })
            })
            .collect()
    }

    /// Returns the total count of events for a specific event name (regardless of filter state).
    pub fn get_event_count(&self, event_name: &str) -> usize {
        self.event_counts.get(event_name).copied().unwrap_or(0)
    }

    /// Gets the selected filter index.
    pub fn filter_selected_index(&self) -> usize {
        self.filter_selected_index
    }

    /// Moves filter selection up (wraps to bottom).
    pub fn move_filter_selection_up(&mut self) {
        let filter_count = self.event_order.len();
        if filter_count > 0 {
            self.filter_selected_index = if self.filter_selected_index == 0 {
                filter_count - 1
            } else {
                self.filter_selected_index - 1
            };
        }
    }

    /// Moves filter selection down (wraps to top).
    pub fn move_filter_selection_down(&mut self) {
        let filter_count = self.event_order.len();
        if filter_count > 0 {
            self.filter_selected_index = (self.filter_selected_index + 1) % filter_count;
        }
    }

    /// Toggles the selected event filter.
    pub fn toggle_selected_filter(&mut self) {
        if let Some(event_name) = self.event_order.get(self.filter_selected_index) {
            if let Some(enabled) = self.event_filters.get_mut(event_name) {
                *enabled = !*enabled;
            }
        }
    }

    /// Toggles all event filters on or off.
    pub fn toggle_all_filters(&mut self) {
        let all_enabled = self.event_filters.values().all(|&enabled| enabled);
        let new_state = !all_enabled;
        for enabled in self.event_filters.values_mut() {
            *enabled = new_state;
        }
    }

    /// Restores event filter states from persisted state.
    pub fn restore_filter_states(&mut self, filter_states: &[(String, bool)]) {
        for (name, enabled) in filter_states {
            self.event_filters.insert(name.clone(), *enabled);
            if !self.event_order.contains(name) {
                self.event_order.push(name.clone());
            }
        }
    }
}
