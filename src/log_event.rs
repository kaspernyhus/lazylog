use crate::highlighter::HighlightPattern;
use crate::log::{Interval, LogBuffer, LogLine};
use crate::marking::Mark;
use crate::processing::{count_events, scan_for_events};
use std::cell::Cell;
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

/// Display item that can be either an event or a mark
#[derive(Debug, Clone)]
pub enum EventOrMark<'a> {
    Event(&'a LogEvent),
    Mark(&'a Mark),
}

impl EventOrMark<'_> {
    pub fn line_index(&self) -> usize {
        match self {
            EventOrMark::Event(e) => e.line_index,
            EventOrMark::Mark(m) => m.line_index,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            EventOrMark::Event(e) => &e.event_name,
            EventOrMark::Mark(m) => m.name.as_deref().unwrap_or("MARK"),
        }
    }

    pub fn is_mark(&self) -> bool {
        matches!(self, EventOrMark::Mark(_))
    }
}

/// Manages log event tracking and scanning.
#[derive(Debug, Default)]
pub struct LogEventTracker {
    /// Events sorted by line_index.
    events: Vec<LogEvent>,
    /// Marks to show in the events view (cached)
    marks: Vec<Mark>,
    /// Currently selected event index
    selected_index: usize,
    /// Viewport offset for scrolling the list
    viewport_offset: usize,
    /// Last rendered viewport height. Set in ui rendering, therefor need interior mutability.
    viewport_height: Cell<usize>,
    /// Event filters - maps event name to enabled state
    event_filters: HashMap<String, bool>,
    /// Event names in config file order
    event_order: Vec<String>,
    /// Total count of each event type (regardless of filter state)
    event_counts: HashMap<String, usize>,
    /// Selected index in the filter list
    filter_selected_index: usize,
    /// Tracks whether the event list needs rescanning
    needs_rescan: bool,
    /// Whether to show marks in the events view
    show_marks: bool,
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

        let lines: Vec<LogLine> = log_buffer.get_lines_iter(Interval::All).cloned().collect();
        self.event_counts = count_events(&lines, event_patterns);
        self.events = scan_for_events(&lines, event_patterns, &self.event_filters);
        self.needs_rescan = false;
    }

    /// Marks the event list as needing a rescan (called when log filters change).
    pub fn mark_needs_rescan(&mut self) {
        self.needs_rescan = true;
    }

    /// Returns true if the event list needs rescanning.
    pub fn needs_rescan(&self) -> bool {
        self.needs_rescan
    }

    /// Checks a single line for event matches and adds it if it matches.
    pub fn scan_line(
        &mut self,
        log_line: &LogLine,
        event_patterns: &[HighlightPattern],
        follow_mode: bool,
    ) {
        for event in event_patterns {
            if event.matcher.matches(&log_line.content) {
                if let Some(name) = &event.name {
                    *self.event_counts.entry(name.clone()).or_insert(0) += 1;
                    if *self.event_filters.get(name).unwrap_or(&true) {
                        let new_event = LogEvent {
                            event_name: name.clone(),
                            line_index: log_line.index,
                        };

                        self.events.push(new_event);

                        if follow_mode {
                            self.select_last_event();
                        }
                    }
                }
                break;
            }
        }
    }

    /// Updates the cached marks.
    pub fn set_marks(&mut self, marks: &[&Mark]) {
        self.marks = marks.iter().map(|&m| m.clone()).collect();
    }

    /// Returns a vector of combined events and marks in sorted order by line_index.
    fn get_combined_items(&self) -> Vec<EventOrMark> {
        let mut result = Vec::new();

        if self.show_marks {
            let mut event_idx = 0;
            let mut mark_idx = 0;

            while event_idx < self.events.len() || mark_idx < self.marks.len() {
                match (self.events.get(event_idx), self.marks.get(mark_idx)) {
                    (Some(e), Some(m)) if e.line_index <= m.line_index => {
                        result.push(EventOrMark::Event(e));
                        event_idx += 1;
                    }
                    (Some(_), Some(m)) => {
                        result.push(EventOrMark::Mark(m));
                        mark_idx += 1;
                    }
                    (Some(e), None) => {
                        result.push(EventOrMark::Event(e));
                        event_idx += 1;
                    }
                    (None, Some(m)) => {
                        result.push(EventOrMark::Mark(m));
                        mark_idx += 1;
                    }
                    (None, None) => break,
                }
            }
        } else {
            for event in &self.events {
                result.push(EventOrMark::Event(event));
            }
        }

        result
    }

    /// Returns an iterator over events and marks combined in sorted order by line_index.
    pub fn iter_items(&self) -> impl Iterator<Item = EventOrMark> {
        self.get_combined_items().into_iter()
    }

    /// Toggles whether marks are shown in the events view.
    pub fn toggle_show_marks(&mut self) {
        self.show_marks = !self.show_marks;
    }

    /// Returns whether marks are shown in the events view.
    pub fn showing_marks(&self) -> bool {
        self.show_marks
    }

    /// Returns the number of items in the combined view.
    pub fn count(&self) -> usize {
        if self.show_marks {
            self.events.len() + self.marks.len()
        } else {
            self.events.len()
        }
    }

    /// Returns true if no events/marks are in the combined view.
    pub fn is_empty(&self) -> bool {
        if self.show_marks {
            self.events.is_empty() && self.marks.is_empty()
        } else {
            self.events.is_empty()
        }
    }

    /// Finds the index of the item nearest to the given line number.
    pub fn find_nearest(&self, line_index: usize) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let items = self.get_combined_items();
        match items.binary_search_by_key(&line_index, |item| item.line_index()) {
            Ok(idx) => Some(idx),
            Err(0) => Some(0),
            Err(idx) if idx >= items.len() => Some(items.len() - 1),
            Err(idx) => {
                let dist_before = line_index - items[idx - 1].line_index();
                let dist_after = items[idx].line_index() - line_index;
                Some(if dist_before <= dist_after {
                    idx - 1
                } else {
                    idx
                })
            }
        }
    }

    /// Gets the line index of the currently selected item.
    pub fn get_selected_line_index(&self) -> Option<usize> {
        let items = self.get_combined_items();
        items.get(self.selected_index).map(|item| item.line_index())
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Gets the current viewport offset.
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Sets the viewport height.
    pub fn set_viewport_height(&self, height: usize) {
        self.viewport_height.set(height);
    }

    /// Adjusts the viewport offset to keep the selected item visible.
    fn adjust_viewport(&mut self) {
        let total_count = self.count();
        if total_count == 0 {
            self.viewport_offset = 0;
            return;
        }

        let viewport_height = self.viewport_height.get();

        if viewport_height == 0 {
            return;
        }

        // scroll up
        if self.selected_index < self.viewport_offset {
            self.viewport_offset = self.selected_index;
        }

        // scroll down
        let bottom_threshold = self.viewport_offset + viewport_height.saturating_sub(1);
        if self.selected_index > bottom_threshold {
            self.viewport_offset = self.selected_index + 1 - viewport_height;
        }

        // Ensure viewport doesn't go past the end
        let max_offset = total_count.saturating_sub(viewport_height);
        self.viewport_offset = self.viewport_offset.min(max_offset);
    }

    /// Sets the selected index to the nearest event to the given line number.
    ///
    /// This also returns the index that was set, or `None` if there are no events.
    pub fn select_nearest_event(&mut self, current_line: usize) -> Option<usize> {
        if let Some(nearest_index) = self.find_nearest(current_line) {
            self.selected_index = nearest_index;
            self.adjust_viewport();
            Some(nearest_index)
        } else {
            self.selected_index = 0;
            self.adjust_viewport();
            None
        }
    }

    /// Moves selection up (wraps to bottom).
    pub fn move_selection_up(&mut self) {
        let total_count = self.count();
        if total_count > 0 && self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_viewport();
        }
    }

    /// Moves selection down (wraps to top).
    pub fn move_selection_down(&mut self) {
        let total_count = self.count();
        if total_count > 0 && self.selected_index < total_count - 1 {
            self.selected_index += 1;
            self.adjust_viewport();
        }
    }

    /// Selects the last (most recent) item in the list.
    pub fn select_last_event(&mut self) {
        let total_count = self.count();
        if total_count > 0 {
            self.selected_index = total_count - 1;
            self.adjust_viewport();
        }
    }

    /// Moves selection up by half a page.
    pub fn selection_page_up(&mut self) {
        let total_count = self.count();
        if total_count > 0 {
            let page_size = self.viewport_height.get().saturating_sub(1).max(1) / 2; // At least 1
            self.selected_index = self.selected_index.saturating_sub(page_size);
            self.adjust_viewport();
        }
    }

    /// Moves selection down by half a page.
    pub fn selection_page_down(&mut self) {
        let total_count = self.count();
        if total_count > 0 {
            let page_size = self.viewport_height.get().saturating_sub(1).max(1) / 2; // At least 1
            self.selected_index = (self.selected_index + page_size).min(total_count - 1);
            self.adjust_viewport();
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
