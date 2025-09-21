use ratatui::text::Line;
use ratatui::widgets::{Borders, ListState};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Clear, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget,
    },
};

use crate::app::{App, AppState};

pub const RIGHT_ARROW: &str = "▶ ";
const MAX_PATH_LENGTH: usize = 42;
const GRAY_COLOR: Color = Color::Indexed(237);

fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
    let min_margin = 2;

    let max_width = area.width.saturating_sub(2 * min_margin);
    let max_height = area.height.saturating_sub(2 * min_margin);

    let popup_width = width.min(max_width);
    let popup_height = height.min(max_height);

    let x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect {
        x,
        y,
        width: popup_width,
        height: popup_height,
    }
}

impl App {
    fn render_help_popup(&self, area: Rect, buf: &mut Buffer) {
        let popup_area = popup_area(area, 38, 18);
        Clear.render(popup_area, buf);

        let help_text = vec![
            Line::from("q            Quit"),
            Line::from("Down/Up      Navigate"),
            Line::from("g/G          Go to start/end"),
            Line::from("PageUp/Down  Page up/down"),
            Line::from("z            Center selected line"),
            Line::from("Left/Right   Scroll horizontally"),
            Line::from("0            Reset horizontal scroll"),
        ];

        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        let help_popup = Paragraph::new(help_text)
            .block(block)
            .alignment(Alignment::Left)
            .wrap(ratatui::widgets::Wrap { trim: true });

        help_popup.render(popup_area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let total_lines = self.viewport.total_lines;
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
            if name.len() > MAX_PATH_LENGTH {
                format!("{}...", &name[..MAX_PATH_LENGTH])
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
            .style(Style::default().bg(GRAY_COLOR));
        footer.render(area, buf);
    }

    fn render_search_bar(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = format!("Search: {}", self.input_query);
        let search_bar = Paragraph::new(search_prompt)
            .style(Style::default().bg(GRAY_COLOR))
            .alignment(Alignment::Left);
        search_bar.render(area, buf);
    }

    fn render_goto_line_bar(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = format!("Go to line: {}", self.input_query);
        let search_bar = Paragraph::new(search_prompt)
            .style(Style::default().bg(GRAY_COLOR))
            .alignment(Alignment::Left);
        search_bar.render(area, buf);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let mut scrollbar_state = ScrollbarState::new(self.log_buffer.lines.len())
            .position(self.viewport.selected_line)
            .viewport_content_length(1);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(GRAY_COLOR))
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
    }

    fn render_logview(&self, area: Rect, buf: &mut Buffer) {
        let (start, end) = self.viewport.visible();
        let visible_lines = self.log_buffer.get_lines(start, end);

        let items: Vec<&str> = visible_lines
            .iter()
            .map(|line| {
                let content = &line.content;
                if self.viewport.horizontal_offset >= content.len() {
                    ""
                } else {
                    &content[self.viewport.horizontal_offset..]
                }
            })
            .collect();

        let mut list_state = ListState::default();
        if self.viewport.selected_line >= start && self.viewport.selected_line < end {
            list_state.select(Some(self.viewport.selected_line - start));
        }

        let log_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        StatefulWidget::render(log_list, area, buf, &mut list_state);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [top, middle, bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let [log_view_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(middle);

        // Title
        let title = Block::default()
            .title(" Lazylog ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(GRAY_COLOR));
        title.render(top, buf);

        // Main view
        self.render_logview(log_view_area, buf);
        self.render_scrollbar(scrollbar_area, buf);

        // Footer
        match self.app_state {
            AppState::SearchView => self.render_search_bar(bottom, buf),
            AppState::GotoLineView => self.render_goto_line_bar(bottom, buf),
            _ => self.render_footer(bottom, buf),
        }

        // Popup
        if self.app_state == AppState::HelpView {
            self.render_help_popup(area, buf);
        }
    }
}
