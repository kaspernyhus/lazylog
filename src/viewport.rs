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
}

impl Viewport {
    /// Updates the viewport dimensions.
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
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
            self.selected_line =
                (self.selected_line + page_size).min(self.total_lines.saturating_sub(1));
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
            self.horizontal_offset =
                (self.horizontal_offset + scroll_amount).min(max_line_length - self.width / 2);
        }
    }

    /// Resets horizontal scroll.
    pub fn reset_horizontal(&mut self) {
        self.horizontal_offset = 0;
    }
}
