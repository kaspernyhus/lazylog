use crate::log_event::LogEvent;
use crate::marking::Mark;

/// Display item that can be either an event or a mark.
#[derive(Debug, Clone)]
pub enum EventOrMark<'a> {
    Event(&'a LogEvent),
    Mark(&'a Mark),
}

impl<'a> EventOrMark<'a> {
    /// Returns the line index of this item.
    pub fn line_index(&self) -> usize {
        match self {
            EventOrMark::Event(e) => e.line_index,
            EventOrMark::Mark(m) => m.line_index,
        }
    }

    /// Returns the name of this item.
    pub fn name(&self) -> &str {
        match self {
            EventOrMark::Event(e) => &e.event_name,
            EventOrMark::Mark(m) => m.name.as_deref().unwrap_or("MARK"),
        }
    }

    /// Returns true if this is a mark (not an event).
    pub fn is_mark(&self) -> bool {
        matches!(self, EventOrMark::Mark(_))
    }
}

/// View that merges events and marks in sorted order by line_index.
/// Pure logic for combining two sorted lists - no UI state.
pub struct EventMarkView;

impl EventMarkView {
    /// Merges events and marks into a single sorted vector.
    /// Both input slices must be sorted by line_index.
    pub fn merge<'a>(
        events: &[&'a LogEvent],
        marks: &[&'a Mark],
        show_marks: bool,
    ) -> Vec<EventOrMark<'a>> {
        if !show_marks {
            return events.iter().map(|&e| EventOrMark::Event(e)).collect();
        }

        let mut result = Vec::with_capacity(events.len() + marks.len());
        let mut event_idx = 0;
        let mut mark_idx = 0;

        while event_idx < events.len() || mark_idx < marks.len() {
            match (events.get(event_idx), marks.get(mark_idx)) {
                (Some(&e), Some(&m)) if e.line_index <= m.line_index => {
                    result.push(EventOrMark::Event(e));
                    event_idx += 1;
                }
                (Some(_), Some(&m)) => {
                    result.push(EventOrMark::Mark(m));
                    mark_idx += 1;
                }
                (Some(&e), None) => {
                    result.push(EventOrMark::Event(e));
                    event_idx += 1;
                }
                (None, Some(&m)) => {
                    result.push(EventOrMark::Mark(m));
                    mark_idx += 1;
                }
                (None, None) => break,
            }
        }

        result
    }
}
