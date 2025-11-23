use crate::list_view_state::ListViewState;
use crate::log::LogLine;
use crate::utils::contains_ignore_case;
use rayon::prelude::*;
use std::collections::HashSet;

/// A mark with an optional name/tag.
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
        Self {
            name: None,
            line_index,
        }
    }
    pub fn new_with_name(line_index: usize, name: &str) -> Self {
        Self {
            name: Some(name.to_string()),
            line_index,
        }
    }

    /// Sets or updates the mark's name.
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }
}

/// Manages marked log lines.
#[derive(Debug, Default)]
pub struct Marking {
    /// All marks sorted by absolute line index.
    marks: Vec<Mark>,
    /// Cached active lines indices from LogBuffer (lines that pass filters).
    active_lines: HashSet<usize>,
    /// View state for the marks list.
    view_state: ListViewState,
}

impl Marking {
    /// Set the active lines
    pub fn update_active_lines(&mut self, active_lines: &[usize]) {
        self.active_lines = active_lines.iter().copied().collect();
        let count = self.get_filtered_marks().len();
        self.view_state.set_item_count(count);
    }

    /// Returns filtered marks.
    pub fn get_filtered_marks(&self) -> Vec<&Mark> {
        self.marks
            .iter()
            .filter(|mark| self.active_lines.contains(&mark.line_index))
            .collect()
    }

    /// Toggles the mark status of a log line.
    pub fn toggle_mark(&mut self, line_index: usize) {
        match self
            .marks
            .binary_search_by_key(&line_index, |mark| mark.line_index)
        {
            Ok(pos) => {
                self.marks.remove(pos);
            }
            Err(pos) => {
                self.marks.insert(pos, Mark::new(line_index));
            }
        }
        let count = self.get_filtered_marks().len();
        self.view_state.set_item_count(count);
    }

    /// Sets or updates the name of an existing mark.
    pub fn set_mark_name(&mut self, line_index: usize, name: String) {
        if let Some(mark) = self
            .marks
            .iter_mut()
            .find(|mark| mark.line_index == line_index)
        {
            mark.set_name(name);
        }
    }

    /// Unmarks a log line.
    pub fn unmark(&mut self, line_index: usize) {
        if let Ok(pos) = self
            .marks
            .binary_search_by_key(&line_index, |mark| mark.line_index)
        {
            self.marks.remove(pos);
            let count = self.get_filtered_marks().len();
            self.view_state.set_item_count(count);
        }
    }

    /// Creates marks for all lines matching the given pattern (case-insensitive).
    pub fn create_marks_from_pattern<'a>(
        &mut self,
        pattern: &str,
        lines: impl Iterator<Item = &'a LogLine>,
    ) {
        if pattern.is_empty() {
            return;
        }

        let marked_set: HashSet<usize> = self.marks.iter().map(|m| m.line_index).collect();

        let lines_vec: Vec<_> = lines.collect();
        let pattern_str = pattern.to_string();

        let new_marks: Vec<Mark> = lines_vec
            .par_iter()
            .filter_map(|log_line| {
                if contains_ignore_case(log_line.content(), &pattern_str)
                    && !marked_set.contains(&log_line.index)
                {
                    Some(Mark::new_with_name(log_line.index, &pattern_str))
                } else {
                    None
                }
            })
            .collect();

        self.marks.extend(new_marks);
        self.marks.sort_by_key(|mark| mark.line_index);
        let count = self.get_filtered_marks().len();
        self.view_state.set_item_count(count);
    }

    /// Returns whether a log line is marked.
    pub fn is_marked(&self, line_index: usize) -> bool {
        self.marks
            .binary_search_by_key(&line_index, |mark| mark.line_index)
            .is_ok()
    }

    /// Returns a vector of all marked line indices.
    pub fn get_marked_indices(&self) -> Vec<usize> {
        self.marks.iter().map(|m| m.line_index).collect()
    }

    /// Returns the number of marked lines.
    pub fn count(&self) -> usize {
        self.marks.len()
    }

    /// Returns whether there are any marked lines.
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    /// Returns all marks.
    pub fn get_all_marks(&self) -> &[Mark] {
        &self.marks
    }

    /// Clears all marks.
    pub fn clear_all(&mut self) {
        self.marks.clear();
        self.view_state.reset();
    }

    /// Get next mark after the given line index.
    pub fn get_next_mark(&self, line_index: usize) -> Option<usize> {
        let active_marks = self.get_filtered_marks();
        active_marks
            .iter()
            .find(|m| m.line_index > line_index)
            .map(|m| m.line_index)
    }

    /// Get previous mark before the given line index.
    pub fn get_previous_mark(&self, line_index: usize) -> Option<usize> {
        let active_marks = self.get_filtered_marks();
        active_marks
            .iter()
            .rev()
            .find(|m| m.line_index < line_index)
            .map(|m| m.line_index)
    }

    /// Gets the mark at the given index.
    pub fn get_selected_mark(&self) -> Option<&Mark> {
        let active_marks = self.get_filtered_marks();
        let index = self.view_state.selected_index();
        active_marks.get(index).copied()
    }

    /// Gets the currently selected marked line index.
    pub fn get_selected_marked_line(&self) -> Option<usize> {
        self.get_selected_mark().map(|m| m.line_index)
    }

    /// Selects the mark closest to the given line index (in filtered marks).
    pub fn select_nearest_mark(&mut self, line_index: usize) {
        let active_marks = self.get_filtered_marks();
        if active_marks.is_empty() {
            return;
        }

        let closest_index = match active_marks.binary_search_by_key(&line_index, |m| m.line_index) {
            Ok(idx) => idx,
            Err(0) => 0,
            Err(idx) if idx >= active_marks.len() => active_marks.len() - 1,
            Err(idx) => {
                let dist_before = line_index - active_marks[idx - 1].line_index;
                let dist_after = active_marks[idx].line_index - line_index;
                if dist_before <= dist_after {
                    idx - 1
                } else {
                    idx
                }
            }
        };

        self.view_state.select_index(closest_index);
    }

    /// Gets the currently selected index in the filtered marks view.
    pub fn selected_index(&self) -> usize {
        self.view_state.selected_index()
    }

    /// Gets the viewport offset.
    pub fn viewport_offset(&self) -> usize {
        self.view_state.viewport_offset()
    }

    /// Sets the viewport height.
    pub fn set_viewport_height(&self, height: usize) {
        self.view_state.set_viewport_height(height);
    }

    /// Moves selection up in the filtered marks view.
    pub fn move_selection_up(&mut self) {
        self.view_state.move_up();
    }

    /// Moves selection down in the filtered marks view.
    pub fn move_selection_down(&mut self) {
        self.view_state.move_down();
    }

    /// Moves selection up by half a page in the filtered marks view.
    pub fn page_up(&mut self) {
        self.view_state.page_up();
    }

    /// Moves selection down by half a page in the filtered marks view.
    pub fn page_down(&mut self) {
        self.view_state.page_down();
    }

    /// Resets the view state.
    pub fn reset_view(&mut self) {
        self.view_state.reset();
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
        mark.set_name("important".to_string());
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
    fn test_get_selected_mark_returns_currently_selected() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(20);
        // Need to set active_lines for filtered marks
        marking.update_active_lines(&[10, 20]);
        marking.move_selection_down();
        let mark = marking.get_selected_mark().unwrap();
        assert_eq!(mark.line_index, 20);
    }

    #[test]
    fn test_get_selected_mark_returns_none_when_empty() {
        let marking = Marking::default();
        assert!(marking.get_selected_mark().is_none());
    }

    #[test]
    fn test_get_selected_marked_line_returns_line_index() {
        let mut marking = Marking::default();
        marking.toggle_mark(42);
        marking.update_active_lines(&[42]);
        let line_index = marking.get_selected_marked_line().unwrap();
        assert_eq!(line_index, 42);
    }

    #[test]
    fn test_select_nearest_mark_selects_closest() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(50);
        marking.toggle_mark(100);
        marking.update_active_lines(&[10, 50, 100]);
        marking.select_nearest_mark(48);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 50);
    }

    #[test]
    fn test_select_nearest_mark_works_with_exact_match() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(50);
        marking.toggle_mark(100);
        marking.update_active_lines(&[10, 50, 100]);
        marking.select_nearest_mark(50);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 50);
    }

    #[test]
    fn test_select_nearest_mark_selects_first_when_equal_distance() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(30);
        marking.update_active_lines(&[10, 30]);
        marking.select_nearest_mark(20);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 10);
    }

    #[test]
    fn test_select_nearest_mark() {
        let mut marking = Marking::default();
        // Insert in non-sorted order
        marking.toggle_mark(100);
        marking.toggle_mark(10);
        marking.toggle_mark(50);

        // Set active_lines so marks are visible
        marking.update_active_lines(&[10, 50, 100]);

        // Select nearest to 48 (should be line 50)
        marking.select_nearest_mark(48);

        // The selected mark should be at line 50
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 50);

        // When sorted, line 50 should be at index 1: [10, 50, 100]
        let sorted = marking.get_all_marks();
        assert_eq!(sorted[0].line_index, 10);
        assert_eq!(sorted[1].line_index, 50);
        assert_eq!(sorted[2].line_index, 100);
    }

    #[test]
    fn test_create_marks_from_pattern_case_insensitive() {
        let log_lines = [
            LogLine::new("ERROR in caps".to_string(), 10),
            LogLine::new("error in lower".to_string(), 20),
            LogLine::new("Error in mixed".to_string(), 30),
            LogLine::new("info message".to_string(), 40),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("ErRoR", log_lines.iter());

        assert_eq!(marking.count(), 3);
        let marks = marking.get_all_marks();
        assert_eq!(marks[0].line_index, 10);
        assert_eq!(marks[1].line_index, 20);
        assert_eq!(marks[2].line_index, 30);
        assert_eq!(marks[0].name, Some("ErRoR".to_string()));
    }

    #[test]
    fn test_create_marks_from_pattern_uses_original_indices() {
        let log_lines = [
            LogLine::new("ERROR in module A".to_string(), 5),
            LogLine::new("info message".to_string(), 12),
            LogLine::new("Error in module B".to_string(), 23),
            LogLine::new("debug output".to_string(), 45),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("error", log_lines.iter());

        assert_eq!(marking.count(), 2);
        let marks = marking.get_all_marks();
        assert_eq!(marks[0].line_index, 5);
        assert_eq!(marks[1].line_index, 23);
        assert_eq!(marks[0].name, Some("error".to_string()));
        assert_eq!(marks[1].name, Some("error".to_string()));
    }
}
