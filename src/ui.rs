use ratatui::text::Line;
use ratatui::widgets::ListState;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, List, StatefulWidget, Widget},
};

use crate::app::App;

pub const RIGHT_ARROW: &str = "▶ ";

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [top, middle, bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let title = Block::default()
            .title(" Lazylog ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Indexed(237)));

        let total_lines = self.log_buffer.lines.len();
        let current_line = self.viewport.selected_line + 1;
        let percent = if total_lines > 0 {
            if current_line == total_lines {
                100
            } else {
                (current_line * 100) / (total_lines)
            }
        } else {
            0
        };
        let file_name = if let Some(path) = &self.log_buffer.file_path {
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            if name.len() > 42 {
                format!("{}...", &name[..42])
            } else {
                name.to_string()
            }
        } else {
            "".to_string()
        };

        let footer = Block::default()
            .title_bottom(Line::from(file_name).left_aligned())
            .title_bottom(Line::from("h:View Help").centered())
            .title_bottom(
                Line::from(format!("{}/{} {:3}% ", current_line, total_lines, percent))
                    .right_aligned(),
            )
            .style(Style::default().bg(Color::Indexed(237)));

        let (start, end) = self.viewport.visible();
        let visible_lines = self.log_buffer.get_lines(start, end);
        let items: Vec<&str> = visible_lines
            .iter()
            .map(|line| line.content.as_str())
            .collect();

        let mut list_state = ListState::default();
        if self.viewport.selected_line >= start && self.viewport.selected_line < end {
            list_state.select(Some(self.viewport.selected_line - start));
        }

        let log_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        title.render(top, buf);
        StatefulWidget::render(log_list, middle, buf, &mut list_state);
        footer.render(bottom, buf);
    }
}
