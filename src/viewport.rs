/// Maximum number of history entries to keep.
const MAX_HISTORY: usize = 20;

/// Manages the visible window and cursor position for viewing log lines.
#[derive(Debug, Default)]
pub struct Viewport {
    /// Width of the viewport in characters.
    pub width: usize,
    /// Height of the viewport in lines.
    pub height: usize,
    /// Index of the first visible line.
    pub top_line: usize,
    /// Index of the currently selected line.
    pub selected_line: usize,
    /// Number of lines to maintain as margin when scrolling.
    pub scroll_margin: usize,
    /// Total number of lines available to display.
    pub total_lines: usize,
    /// Horizontal scroll offset for wide lines.
    pub horizontal_offset: usize,
    /// Whether to automatically scroll to bottom when new lines arrive in streaming mode.
    pub follow_mode: bool,
    /// Whether to keep the cursor centered in the viewport when scrolling.
    pub center_cursor_mode: bool,
    /// History stack of log line indices.
    history: Vec<usize>,
    /// Current position in the history stack.
    history_position: usize,
}

impl Viewport {
    pub fn reset_view(&mut self) {
        self.total_lines = 0;
        self.top_line = 0;
        self.selected_line = 0;
        self.horizontal_offset = 0;
        self.history = Vec::new();
        self.history_position = 0;
    }

    /// Updates the viewport dimensions.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.adjust_visible();
    }

    /// Sets the total number of available lines.
    pub fn set_total_lines(&mut self, total_lines: usize) {
        self.total_lines = total_lines;
    }

    /// Moves the selection up by one line.
    pub fn move_up(&mut self) {
        if self.selected_line > 0 {
            self.selected_line -= 1;
            self.adjust_visible();
        }
    }

    /// Moves the selection down by one line.
    pub fn move_down(&mut self) {
        if self.selected_line + 1 < self.total_lines {
            self.selected_line += 1;
            self.adjust_visible();
        }
    }

    /// Moves the selection up by one page and center the viewport on that line.
    pub fn page_up(&mut self) {
        if self.selected_line > 0 {
            let page_size = self.height.saturating_sub(1);
            self.selected_line = self.selected_line.saturating_sub(page_size);
            self.adjust_visible();
            self.center_selected();
        }
    }

    /// Moves the selection down by one page and center the viewport on that line.
    pub fn page_down(&mut self) {
        if self.selected_line + 1 < self.total_lines {
            let page_size = self.height.saturating_sub(1);
            self.selected_line = (self.selected_line + page_size).min(self.total_lines.saturating_sub(1));
            self.adjust_visible();
            self.center_selected();
        }
    }

    /// Moves the selection to the first line.
    pub fn goto_top(&mut self) {
        self.selected_line = 0;
        self.adjust_visible();
    }

    /// Moves the selection to the last line.
    pub fn goto_bottom(&mut self) {
        if self.total_lines > 0 {
            self.selected_line = self.total_lines - 1;
        } else {
            self.selected_line = 0;
        }
        self.adjust_visible();
    }

    /// Moves the selection to a specific line.
    ///
    /// If `center` is true, the line will be centered in the viewport.
    pub fn goto_line(&mut self, line: usize, center: bool) {
        if line < self.total_lines {
            self.selected_line = line;
            if center {
                self.center_selected();
            } else {
                self.adjust_visible();
            }
            self.follow_mode = false;
        }
    }

    /// Centers the selected line in the viewport.
    pub fn center_selected(&mut self) {
        if self.total_lines == 0 {
            self.top_line = 0;
            self.selected_line = 0;
            return;
        }

        let half_height = self.height / 2;
        if self.selected_line >= half_height {
            self.top_line = self.selected_line - half_height;
            if self.top_line + self.height > self.total_lines {
                self.top_line = self.total_lines.saturating_sub(self.height);
            }
        } else {
            self.top_line = 0;
        }
    }

    /// Returns the range of visible lines (start, end).
    pub fn visible(&self) -> (usize, usize) {
        let start = self.top_line;
        let end = self.top_line + self.height;
        (start, end)
    }

    /// Adjusts the visible window to keep the selected line visible with scroll margin.
    fn adjust_visible(&mut self) {
        if self.total_lines == 0 {
            self.top_line = 0;
            self.selected_line = 0;
            return;
        }

        if self.center_cursor_mode {
            self.center_selected();
            return;
        }

        // Scroll up if selection gets too close to top
        if self.selected_line < self.top_line + self.scroll_margin {
            self.top_line = self.selected_line.saturating_sub(self.scroll_margin);
        }

        // Scroll down if selection gets too close to bottom
        let bottom_margin_line = self.top_line + self.height.saturating_sub(self.scroll_margin + 1);
        if self.selected_line > bottom_margin_line {
            self.top_line = (self.selected_line + self.scroll_margin + 1)
                .saturating_sub(self.height)
                .min(self.total_lines.saturating_sub(self.height));
        }

        if self.total_lines <= self.height {
            self.top_line = 0;
        }
    }

    /// Scrolls left horizontally by half the viewport width.
    pub fn scroll_left(&mut self) {
        let scroll_amount = self.width / 2;
        self.horizontal_offset = self.horizontal_offset.saturating_sub(scroll_amount);
    }

    /// Scrolls right horizontally by half the viewport width.
    ///
    /// The scroll amount is bounded by the maximum line length.
    pub fn scroll_right(&mut self, max_line_length: usize) {
        if max_line_length > self.width {
            let scroll_amount = self.width / 2;
            self.horizontal_offset = (self.horizontal_offset + scroll_amount).min(max_line_length - self.width / 2);
        }
    }

    /// Resets horizontal scroll.
    pub fn reset_horizontal(&mut self) {
        self.horizontal_offset = 0;
    }

    /// Records a log line index in the navigation history.
    pub fn push_history(&mut self, line_index: usize) {
        // Truncate forward history when making a new jump
        if self.history_position + 1 < self.history.len() {
            self.history.truncate(self.history_position + 1);
        }

        if self.history.last() != Some(&line_index) {
            self.history.push(line_index);

            if self.history.len() > MAX_HISTORY {
                self.history.remove(0);
            }

            self.history_position = self.history.len() - 1;
        }
    }

    /// Navigate back in history.
    /// Returns the log line index to jump to, or None if at the beginning.
    pub fn history_back(&mut self) -> Option<usize> {
        if self.history_position > 0 {
            self.history_position -= 1;
            self.history.get(self.history_position).copied()
        } else {
            None
        }
    }

    /// Navigate forward in history.
    /// Returns the log line index to jump to, or None if at the end.
    pub fn history_forward(&mut self) -> Option<usize> {
        if self.history_position + 1 < self.history.len() {
            self.history_position += 1;
            self.history.get(self.history_position).copied()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_viewport(height: usize, total_lines: usize) -> Viewport {
        let mut viewport = Viewport {
            width: 80,
            height,
            scroll_margin: 2,
            total_lines,
            ..Default::default()
        };
        viewport.set_total_lines(total_lines);
        viewport
    }

    #[test]
    fn test_move_down_increments_selected_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 5;
        viewport.move_down();
        assert_eq!(viewport.selected_line, 6);
    }

    #[test]
    fn test_move_down_stops_at_last_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 99;
        viewport.move_down();
        assert_eq!(viewport.selected_line, 99);
    }

    #[test]
    fn test_move_up_decrements_selected_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 5;
        viewport.move_up();
        assert_eq!(viewport.selected_line, 4);
    }

    #[test]
    fn test_move_up_stops_at_zero() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 0;
        viewport.move_up();
        assert_eq!(viewport.selected_line, 0);
    }

    #[test]
    fn test_goto_top_moves_to_first_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 50;
        viewport.goto_top();
        assert_eq!(viewport.selected_line, 0);
    }

    #[test]
    fn test_goto_bottom_moves_to_last_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 0;
        viewport.goto_bottom();
        assert_eq!(viewport.selected_line, 99);
    }

    #[test]
    fn test_goto_bottom_handles_empty_buffer() {
        let mut viewport = create_viewport(10, 0);
        viewport.goto_bottom();
        assert_eq!(viewport.selected_line, 0);
    }

    #[test]
    fn test_goto_line_moves_to_specific_line() {
        let mut viewport = create_viewport(10, 100);
        viewport.goto_line(42, false);
        assert_eq!(viewport.selected_line, 42);
    }

    #[test]
    fn test_goto_line_ignores_out_of_bounds() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 50;
        viewport.goto_line(150, false);
        assert_eq!(viewport.selected_line, 50);
    }

    #[test]
    fn test_center_selected_handles_lines_near_start() {
        let mut viewport = create_viewport(10, 100);
        viewport.selected_line = 2;
        viewport.center_selected();
        assert_eq!(viewport.top_line, 0);
    }

    #[test]
    fn test_center_selected_handles_lines_near_end() {
        let mut viewport = create_viewport(10, 20);
        viewport.selected_line = 18;
        viewport.center_selected();
        assert_eq!(viewport.top_line, 10); // 20 - 10 (can't center beyond end)
    }

    #[test]
    fn test_visible_returns_correct_range() {
        let mut viewport = create_viewport(10, 100);
        viewport.top_line = 25;
        let (start, end) = viewport.visible();
        assert_eq!(start, 25);
        assert_eq!(end, 35);
    }

    #[test]
    fn test_adjust_visible_handles_empty_buffer() {
        let mut viewport = create_viewport(10, 0);
        viewport.selected_line = 5;
        viewport.adjust_visible();
        assert_eq!(viewport.selected_line, 0);
        assert_eq!(viewport.top_line, 0);
    }

    #[test]
    fn test_resize_updates_dimensions() {
        let mut viewport = create_viewport(10, 100);
        viewport.resize(120, 25);
        assert_eq!(viewport.width, 120);
        assert_eq!(viewport.height, 25);
    }
}
