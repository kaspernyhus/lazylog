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

    /// Gets the currently selected index in the marks view.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Moves selection up in the marks view (wraps to bottom).
    pub fn move_selection_up(&mut self) {
        let count = self.count();
        if count > 0 {
            self.selected_index = if self.selected_index == 0 {
                count - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Moves selection down in the marks view (wraps to top).
    pub fn move_selection_down(&mut self) {
        let count = self.count();
        if count > 0 {
            self.selected_index = (self.selected_index + 1) % count;
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
