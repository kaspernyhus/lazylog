use crate::highlighter::HighlightPattern;
use crate::log::{Interval, LogBuffer};

/// Information about a matched event occurrence.
#[derive(Debug, Clone)]
pub struct LogEvent {
    /// Name of the event that matched.
    pub event_name: String,
    /// Line number where the event occurred.
    pub line_index: usize,
}

/// Manages log event tracking and scanning.
#[derive(Debug, Default)]
pub struct LogEventTracker {
    events: Vec<LogEvent>,
    selected_index: usize,
}

impl LogEventTracker {
    /// Creates a new empty log event tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scans all log lines for event occurrences and stores them.
    pub fn scan(&mut self, log_buffer: &LogBuffer, event_patterns: &[HighlightPattern]) {
        self.events.clear();
        for log_line in log_buffer.get_lines_iter(Interval::All) {
            for event in event_patterns {
                if event.matcher.matches(log_line.content()) {
                    if let Some(name) = &event.name {
                        self.events.push(LogEvent {
                            event_name: name.clone(),
                            line_index: log_line.index,
                        });
                    }
                    break;
                }
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
        if !self.events.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.events.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Moves selection down (wraps to top).
    pub fn move_selection_down(&mut self) {
        if !self.events.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.events.len();
        }
    }
}
