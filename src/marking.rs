use crate::log::LogLine;
use crate::resolver::{Tag, TagRule, VisibilityRule};
use crate::utils::contains_ignore_case;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

/// A log line mark with an optional name/tag.
#[derive(Debug, Clone)]
pub struct Mark {
    /// Optional name/tag for the mark.
    pub name: Option<String>,
    /// The original log line index.
    pub line_index: usize,
}

impl Mark {
    /// Creates a new mark without a name.
    pub fn new(line_index: usize) -> Self {
        Self { name: None, line_index }
    }

    pub fn new_with_name(line_index: usize, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            line_index,
        }
    }

    /// Sets or updates the mark's name.
    pub fn set_name(&mut self, name: &str) {
        self.name = Some(name.to_string());
    }
}

/// Manages marked log lines.
#[derive(Debug, Default)]
pub struct Marking {
    /// All marks sorted by line index.
    marks: Vec<Mark>,
}

impl Marking {
    /// Toggles the mark status of a log line.
    pub fn toggle_mark(&mut self, line_index: usize) {
        match self.marks.binary_search_by_key(&line_index, |mark| mark.line_index) {
            Ok(pos) => {
                self.marks.remove(pos);
            }
            Err(pos) => {
                self.marks.insert(pos, Mark::new(line_index));
            }
        }
    }

    /// Add a new named mark or update existing mark name
    pub fn add_named_mark(&mut self, line_index: usize, name: &str) {
        match self.marks.binary_search_by_key(&line_index, |mark| mark.line_index) {
            Ok(pos) => {
                self.set_mark_name(pos, name);
            }
            Err(pos) => {
                self.marks.insert(pos, Mark::new_with_name(line_index, name));
            }
        }
    }

    /// Sets or updates the name of an existing mark.
    pub fn set_mark_name(&mut self, line_index: usize, name: &str) {
        if let Some(mark) = self.marks.iter_mut().find(|mark| mark.line_index == line_index) {
            mark.set_name(name);
        }
    }

    /// Unmarks a log line.
    pub fn unmark(&mut self, line_index: usize) {
        if let Ok(pos) = self.marks.binary_search_by_key(&line_index, |mark| mark.line_index) {
            self.marks.remove(pos);
        }
    }

    /// Creates marks for all lines matching the given pattern (case-insensitive).
    pub fn create_marks_from_pattern<'a>(&mut self, pattern: &str, lines: impl Iterator<Item = &'a LogLine>) {
        if pattern.is_empty() {
            return;
        }

        let marked_set: HashSet<usize> = self.marks.iter().map(|m| m.line_index).collect();

        let lines_vec: Vec<_> = lines.collect();
        let pattern_str = pattern.to_string();

        let new_marks: Vec<Mark> = lines_vec
            .par_iter()
            .filter_map(|log_line| {
                if contains_ignore_case(log_line.content(), &pattern_str) && !marked_set.contains(&log_line.index) {
                    Some(Mark::new_with_name(log_line.index, &pattern_str))
                } else {
                    None
                }
            })
            .collect();

        self.marks.extend(new_marks);
        self.marks.sort_by_key(|mark| mark.line_index);
    }

    /// Returns whether a log line is marked.
    pub fn is_marked(&self, line_index: usize) -> bool {
        self.marks
            .binary_search_by_key(&line_index, |mark| mark.line_index)
            .is_ok()
    }

    /// Returns all marked line indices.
    pub fn get_marked_indices(&self) -> HashSet<usize> {
        self.marks.iter().map(|m| m.line_index).collect()
    }

    /// Returns the total number of marked lines.
    pub fn count(&self) -> usize {
        self.marks.len()
    }

    /// Returns whether there are any marked lines.
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    /// Returns all marks.
    pub fn get_marks(&self) -> &[Mark] {
        &self.marks
    }

    /// Clears all marks.
    pub fn clear_all(&mut self) {
        self.marks.clear();
    }
}

/// Tag rule that marks lines as marked
pub struct MarkTagRule {
    marked_indices: Arc<HashSet<usize>>,
}

impl MarkTagRule {
    pub fn new(marked_indices: Arc<HashSet<usize>>) -> Self {
        Self { marked_indices }
    }
}

impl TagRule for MarkTagRule {
    fn get_tags(&self, line: &LogLine) -> Option<Tag> {
        if self.marked_indices.contains(&line.index) {
            Some(Tag::Marked)
        } else {
            None
        }
    }
}

/// Rule that only shows lines that are marked
pub struct MarkOnlyVisibilityRule {
    marked_indices: Arc<HashSet<usize>>,
}

impl MarkOnlyVisibilityRule {
    pub fn new(marked_indices: Arc<HashSet<usize>>) -> Self {
        Self { marked_indices }
    }
}

impl VisibilityRule for MarkOnlyVisibilityRule {
    fn is_visible(&self, line: &LogLine) -> bool {
        self.marked_indices.contains(&line.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mark_new_creates_mark_without_name() {
        let mark = Mark::new(42);
        assert_eq!(mark.line_index, 42);
        assert_eq!(mark.name, None);
    }

    #[test]
    fn test_mark_set_name_updates_name() {
        let mut mark = Mark::new(42);
        mark.set_name("important");
        assert_eq!(mark.name, Some("important".to_string()));
    }

    #[test]
    fn test_toggle_mark_adds_mark() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        assert!(marking.is_marked(10));
        assert_eq!(marking.count(), 1);
    }

    #[test]
    fn test_toggle_mark_removes_existing_mark() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(10);
        assert!(!marking.is_marked(10));
        assert_eq!(marking.count(), 0);
    }

    #[test]
    fn test_is_marked_returns_false_for_unmarked_line() {
        let marking = Marking::default();
        assert!(!marking.is_marked(10));
    }

    #[test]
    fn test_count_returns_number_of_marks() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(20);
        marking.toggle_mark(30);
        assert_eq!(marking.count(), 3);
    }

    #[test]
    fn test_create_marks_from_pattern_case_insensitive() {
        let log_lines = [
            LogLine::new("ERROR in caps", 10),
            LogLine::new("error in lower", 20),
            LogLine::new("Error in mixed", 30),
            LogLine::new("info message", 40),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("ErRoR", log_lines.iter());

        assert_eq!(marking.count(), 3);
        let marks = marking.get_marks();
        assert_eq!(marks[0].line_index, 10);
        assert_eq!(marks[1].line_index, 20);
        assert_eq!(marks[2].line_index, 30);
        assert_eq!(marks[0].name, Some("ErRoR".to_string()));
    }

    #[test]
    fn test_create_marks_from_pattern_uses_original_indices() {
        let log_lines = [
            LogLine::new("ERROR in module A", 5),
            LogLine::new("info message", 12),
            LogLine::new("Error in module B", 23),
            LogLine::new("debug output", 45),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("error", log_lines.iter());

        assert_eq!(marking.count(), 2);
        let marks = marking.get_marks();
        assert_eq!(marks[0].line_index, 5);
        assert_eq!(marks[1].line_index, 23);
        assert_eq!(marks[0].name, Some("error".to_string()));
        assert_eq!(marks[1].name, Some("error".to_string()));
    }
}
