use tracing::info;

#[derive(Debug, Default)]
pub struct Viewport {
    pub top_line: usize,
    pub height: usize,
    pub selected_line: usize,
    pub scroll_margin: usize,
}

impl Viewport {
    pub fn resize(&mut self, height: usize) {
        self.height = height;
        info!("Viewport resized: height={}", height);
    }

    pub fn move_up(&mut self, total_lines: usize) {
        if self.selected_line > 0 {
            self.selected_line -= 1;
            self.adjust_visible(total_lines);
        }
    }

    pub fn move_down(&mut self, total_lines: usize) {
        if self.selected_line + 1 < total_lines {
            self.selected_line += 1;
            self.adjust_visible(total_lines);
        }
    }

    pub fn page_up(&mut self, total_lines: usize) {
        if self.selected_line > 0 {
            let page_size = self.height.saturating_sub(1);
            self.selected_line = self.selected_line.saturating_sub(page_size);
            self.adjust_visible(total_lines);
            self.center_selected(total_lines);
        }
    }

    pub fn page_down(&mut self, total_lines: usize) {
        if self.selected_line + 1 < total_lines {
            let page_size = self.height.saturating_sub(1);
            self.selected_line =
                (self.selected_line + page_size).min(total_lines.saturating_sub(1));
            self.adjust_visible(total_lines);
            self.center_selected(total_lines);
        }
    }

    pub fn center_selected(&mut self, total_lines: usize) {
        if total_lines == 0 {
            self.top_line = 0;
            self.selected_line = 0;
            return;
        }

        let half_height = self.height / 2;
        if self.selected_line >= half_height {
            self.top_line = self.selected_line - half_height;
            if self.top_line + self.height > total_lines {
                self.top_line = total_lines.saturating_sub(self.height);
            }
        } else {
            self.top_line = 0;
        }
    }

    pub fn visible(&self) -> (usize, usize) {
        let start = self.top_line;
        let end = self.top_line + self.height;
        (start, end)
    }

    fn adjust_visible(&mut self, total_lines: usize) {
        if total_lines == 0 {
            self.top_line = 0;
            self.selected_line = 0;
            return;
        }

        // Scroll up if selection gets too close to top
        if self.selected_line < self.top_line + self.scroll_margin {
            self.top_line = self.selected_line.saturating_sub(self.scroll_margin);
        }

        // Scroll down if selection gets too close to bottom
        let bottom_margin_line = self.top_line + self.height - self.scroll_margin - 1;
        if self.selected_line > bottom_margin_line {
            self.top_line = (self.selected_line + self.scroll_margin + 1)
                .saturating_sub(self.height)
                .min(total_lines.saturating_sub(self.height));
        }

        if total_lines <= self.height {
            self.top_line = 0;
        }
    }
}
