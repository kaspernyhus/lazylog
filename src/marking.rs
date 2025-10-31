use crate::utils::contains_ignore_case;

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
    /// Vector of marks (original line indices, not filtered).
    marked_lines: Vec<Mark>,
    /// Currently selected index in the marks view.
    selected_index: usize,
}

impl Marking {
    /// Toggles the mark status of a log line (creates mark without name).
    pub fn toggle_mark(&mut self, line_index: usize) {
        if let Some(pos) = self
            .marked_lines
            .iter()
            .position(|m| m.line_index == line_index)
        {
            self.marked_lines.remove(pos);
        } else {
            self.marked_lines.push(Mark::new(line_index));
        }
    }

    /// Sets or updates the name of an existing mark.
    pub fn set_mark_name(&mut self, line_index: usize, name: String) {
        if let Some(mark) = self
            .marked_lines
            .iter_mut()
            .find(|m| m.line_index == line_index)
        {
            mark.set_name(name);
        }
    }

    /// Unmarks a log line.
    pub fn unmark(&mut self, line_index: usize) {
        if let Some(pos) = self
            .marked_lines
            .iter()
            .position(|m| m.line_index == line_index)
        {
            self.marked_lines.remove(pos);
        }
    }

    /// Creates marks for all lines matching the given pattern (case-insensitive).
    pub fn create_marks_from_pattern<'a>(
        &mut self,
        pattern: &str,
        lines: impl Iterator<Item = &'a crate::log::LogLine>,
    ) {
        if !pattern.is_empty() {
            for log_line in lines {
                if contains_ignore_case(log_line.content(), pattern)
                    && !self.is_marked(log_line.index)
                {
                    self.marked_lines
                        .push(Mark::new_with_name(log_line.index, pattern));
                }
            }
        }
    }

    /// Returns whether a log line is marked.
    pub fn is_marked(&self, line_index: usize) -> bool {
        self.marked_lines.iter().any(|m| m.line_index == line_index)
    }

    /// Returns the number of marked lines.
    pub fn count(&self) -> usize {
        self.marked_lines.len()
    }

    /// Returns whether there are any marked lines.
    pub fn is_empty(&self) -> bool {
        self.marked_lines.is_empty()
    }

    /// Returns a sorted vector of all marks.
    pub fn get_sorted_marks(&self) -> Vec<&Mark> {
        let mut marks: Vec<&Mark> = self.marked_lines.iter().collect();
        marks.sort_by_key(|m| m.line_index);
        marks
    }

    /// Clears all marks.
    pub fn clear_all(&mut self) {
        self.marked_lines.clear();
        self.selected_index = 0;
    }

    /// Get next mark after the given line index.
    pub fn get_next_mark(&self, line_index: usize) -> Option<&Mark> {
        self.marked_lines
            .iter()
            .filter(|m| m.line_index > line_index)
            .min_by_key(|m| m.line_index)
    }

    /// Get previous mark before the given line index.
    pub fn get_previous_mark(&self, line_index: usize) -> Option<&Mark> {
        self.marked_lines
            .iter()
            .filter(|m| m.line_index < line_index)
            .max_by_key(|m| m.line_index)
    }

    /// Gets the currently selected index in the marks view.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Moves selection up in the marks view (does not wrap).
    pub fn move_selection_up(&mut self, count: usize) {
        if count > 0 && self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Moves selection down in the marks view (does not wrap).
    pub fn move_selection_down(&mut self, count: usize) {
        if count > 0 && self.selected_index < count - 1 {
            self.selected_index += 1;
        }
    }

    /// Moves selection up by page_size entries.
    pub fn selection_page_up(&mut self, page_size: usize, count: usize) {
        if count > 0 {
            self.selected_index = self.selected_index.saturating_sub(page_size);
        }
    }

    /// Moves selection down by page_size entries.
    pub fn selection_page_down(&mut self, page_size: usize, count: usize) {
        if count > 0 {
            self.selected_index = (self.selected_index + page_size).min(count - 1);
        }
    }

    /// Gets the mark at the given index in the marks list (insertion order).
    pub fn get_mark_at(&self, index: usize) -> Option<&Mark> {
        self.marked_lines.get(index)
    }

    /// Gets the currently selected mark.
    pub fn get_selected_mark(&self) -> Option<&Mark> {
        self.get_mark_at(self.selected_index)
    }

    /// Gets the currently selected marked line index.
    pub fn get_selected_marked_line(&self) -> Option<usize> {
        self.get_selected_mark().map(|m| m.line_index)
    }

    /// Resets the selected index to 0.
    pub fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    /// Selects the mark closest to the given line index.
    /// If there are no marks, does nothing.
    pub fn select_nearest_mark(&mut self, line_index: usize) {
        if self.marked_lines.is_empty() {
            return;
        }

        let mut closest_index = 0;
        let mut min_distance = usize::MAX;

        for (index, mark) in self.marked_lines.iter().enumerate() {
            let distance = if mark.line_index > line_index {
                mark.line_index - line_index
            } else {
                line_index - mark.line_index
            };

            if distance < min_distance {
                min_distance = distance;
                closest_index = index;
            }
        }

        self.selected_index = closest_index;
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
    fn test_get_mark_at_returns_none_for_invalid_index() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        assert!(marking.get_mark_at(5).is_none());
    }

    #[test]
    fn test_get_selected_mark_returns_currently_selected() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(20);
        marking.move_selection_down(2);
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
        let line_index = marking.get_selected_marked_line().unwrap();
        assert_eq!(line_index, 42);
    }

    #[test]
    fn test_select_nearest_mark_selects_closest() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(50);
        marking.toggle_mark(100);
        marking.select_nearest_mark(48);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 50);
    }

    #[test]
    fn test_select_nearest_mark_works_with_exact_match() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(50);
        marking.toggle_mark(100);
        marking.select_nearest_mark(50);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 50);
    }

    #[test]
    fn test_select_nearest_mark_selects_first_when_equal_distance() {
        let mut marking = Marking::default();
        marking.toggle_mark(10);
        marking.toggle_mark(30);
        marking.select_nearest_mark(20);
        assert_eq!(marking.get_selected_mark().unwrap().line_index, 10);
    }

    #[test]
    fn test_create_marks_from_pattern_case_insensitive() {
        use crate::log::LogLine;

        let log_lines = [
            LogLine::new("ERROR in caps".to_string(), 10),
            LogLine::new("error in lower".to_string(), 20),
            LogLine::new("Error in mixed".to_string(), 30),
            LogLine::new("info message".to_string(), 40),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("ErRoR", log_lines.iter());

        assert_eq!(marking.count(), 3);
        let marks = marking.get_sorted_marks();
        assert_eq!(marks[0].line_index, 10);
        assert_eq!(marks[1].line_index, 20);
        assert_eq!(marks[2].line_index, 30);
        assert_eq!(marks[0].name, Some("ErRoR".to_string()));
    }

    #[test]
    fn test_create_marks_from_pattern_uses_original_indices() {
        use crate::log::LogLine;

        let log_lines = [
            LogLine::new("ERROR in module A".to_string(), 5),
            LogLine::new("info message".to_string(), 12),
            LogLine::new("Error in module B".to_string(), 23),
            LogLine::new("debug output".to_string(), 45),
        ];

        let mut marking = Marking::default();
        marking.create_marks_from_pattern("error", log_lines.iter());

        assert_eq!(marking.count(), 2);
        let marks = marking.get_sorted_marks();
        assert_eq!(marks[0].line_index, 5);
        assert_eq!(marks[1].line_index, 23);
        assert_eq!(marks[0].name, Some("error".to_string()));
        assert_eq!(marks[1].name, Some("error".to_string()));
    }
}
