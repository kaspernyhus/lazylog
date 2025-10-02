use crate::log::Interval;
use ratatui::text::Line;
use ratatui::widgets::{Clear, ListState};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{
        Block, List, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
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
    fn get_progression(&self) -> (usize, usize, usize) {
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
        (current_line, total_lines, percent)
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
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

        let left = Line::from(file_name).left_aligned();
        let middle = Line::from("h:View Help").centered();

        let (current_match, total_matches) = self.search.get_match_info();
        let (current_line, total_lines, percent) = self.get_progression();
        let right = if total_matches > 0 {
            Line::from(format!(
                "{}/{} | {}/{} {:3}% ",
                current_match, total_matches, current_line, total_lines, percent
            ))
            .right_aligned()
        } else {
            Line::from(format!("{}/{} {:3}% ", current_line, total_lines, percent)).right_aligned()
        };

        let footer = Block::default()
            .title_bottom(left)
            .title_bottom(middle)
            .title_bottom(right)
            .style(Style::default().bg(GRAY_COLOR));
        footer.render(area, buf);
    }

    fn render_search_bar(&self, area: Rect, buf: &mut Buffer) {
        let case_sensitive = if self.search.is_case_sensitive() {
            "Aa"
        } else {
            "aa"
        };
        let search_prompt =
            Line::from(format!("Search: [{}] {}", case_sensitive, self.input_query)).left_aligned();
        let (current_line, total_lines, percent) = self.get_progression();
        let progression =
            Line::from(format!("{}/{} {:3}% ", current_line, total_lines, percent)).right_aligned();

        let search_bar = Block::default()
            .title_bottom(search_prompt)
            .title_bottom(progression)
            .style(Style::default().bg(GRAY_COLOR));

        search_bar.render(area, buf);
    }

    fn render_goto_line_bar(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = format!("Go to line: {}", self.input_query);
        let search_bar = Paragraph::new(search_prompt)
            .style(Style::default().bg(GRAY_COLOR))
            .alignment(Alignment::Left);
        search_bar.render(area, buf);
    }

    fn render_filter_bar(&self, area: Rect, buf: &mut Buffer) {
        let filter_mode = match self.filter.get_mode() {
            crate::filter::FilterMode::Include => "IN",
            crate::filter::FilterMode::Exclude => "EX",
        };

        let case_sensitive = if self.filter.is_case_sensitive() {
            "Aa"
        } else {
            "aa"
        };

        let filter_prompt = Line::from(format!(
            "Filter: [{}] [{}] {}",
            case_sensitive, filter_mode, self.input_query
        ))
        .left_aligned();
        let (current_line, total_lines, percent) = self.get_progression();
        let progression =
            Line::from(format!("{}/{} {:3}% ", current_line, total_lines, percent)).right_aligned();

        let filter_bar = Block::default()
            .title_bottom(filter_prompt)
            .title_bottom(progression)
            .style(Style::default().bg(GRAY_COLOR));

        filter_bar.render(area, buf);
    }

    fn render_filter_list_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let filter_patterns = self.filter.get_filter_patterns();
        if filter_patterns.is_empty() {
            let no_filters_text = vec![Line::from("No filters configured")];
            let popup = Paragraph::new(no_filters_text)
                .block(
                    Block::default()
                        .title(" Filters ")
                        .title_alignment(Alignment::Center)
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                )
                .alignment(Alignment::Center);
            popup.render(area, buf);
        } else {
            let items: Vec<Line> = filter_patterns
                .iter()
                .map(|pattern| {
                    let mode_str = match pattern.mode {
                        crate::filter::FilterMode::Include => "IN",
                        crate::filter::FilterMode::Exclude => "EX",
                    };
                    let case_str = if pattern.case_sensitive { "Aa" } else { "aa" };

                    let content = format!("[{}] [{}] {}", mode_str, case_str, pattern.pattern);

                    if pattern.enabled {
                        Line::from(content).style(Style::default().fg(Color::Green))
                    } else {
                        Line::from(content).style(Style::default().fg(Color::Gray))
                    }
                })
                .collect();

            let mut list_state = ListState::default();
            if !filter_patterns.is_empty() {
                list_state.select(Some(self.filter.get_selected_pattern_index()));
            }

            let filter_list = List::new(items)
                .block(
                    Block::default()
                        .title(" Filters ")
                        .title_alignment(Alignment::Center)
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                )
                .highlight_symbol(RIGHT_ARROW)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD));

            StatefulWidget::render(filter_list, area, buf, &mut list_state);
        }
    }

    fn render_options_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let items: Vec<Line> = self
            .display_options
            .options
            .iter()
            .map(|option| {
                let checkbox = if option.enabled { "[x]" } else { "[ ]" };
                let content = format!("{} {}", checkbox, option.name);

                if option.enabled {
                    Line::from(content).style(Style::default().fg(Color::Green))
                } else {
                    Line::from(content).style(Style::default().fg(Color::White))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        if !self.display_options.options.is_empty() {
            list_state.select(Some(self.display_options.selected_index));
        }

        let options_list = List::new(items)
            .block(
                Block::default()
                    .title(" Display Options ")
                    .title_alignment(Alignment::Center)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        StatefulWidget::render(options_list, area, buf, &mut list_state);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let mut scrollbar_state = ScrollbarState::new(self.viewport.total_lines)
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
        let lines: Vec<String> = self
            .log_buffer
            .get_lines_iter(Interval::Range(start, end))
            .map(|log_line| self.display_options.apply_to_line(log_line.content()))
            .collect();

        let items: Vec<Line> = lines
            .iter()
            .map(|line| {
                let text = if self.viewport.horizontal_offset >= line.len() {
                    ""
                } else {
                    &line[self.viewport.horizontal_offset..]
                };
                self.highlight_line(text)
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

    fn highlight_line<'a>(&self, content: &'a str) -> Line<'a> {
        let mut patterns_to_highlight = Vec::new();

        for highlight in self.highlighter.get_patterns() {
            patterns_to_highlight.push((
                highlight.pattern.clone(),
                false,
                highlight.color,
            ));
        }

        if self.app_state == AppState::FilterMode {
            if let Some(pattern) = self.filter.get_filter_pattern() {
                if !pattern.is_empty() {
                    patterns_to_highlight.push((
                        pattern.to_string(),
                        self.filter.is_case_sensitive(),
                        Color::Cyan,
                    ));
                }
            }
        }

        if let Some(pattern) = self.search.get_pattern() {
            if !pattern.is_empty() {
                patterns_to_highlight.push((
                    pattern.to_string(),
                    self.search.is_case_sensitive(),
                    Color::Yellow,
                ));
            }
        }

        if patterns_to_highlight.is_empty() {
            return Line::from(content);
        }

        self.apply_highlights(content, patterns_to_highlight)
    }

    fn apply_highlights<'a>(
        &self,
        content: &'a str,
        patterns: Vec<(String, bool, Color)>,
    ) -> Line<'a> {
        let mut highlight_ranges = Vec::new();

        for (pattern, case_sensitive, color) in patterns {
            let (search_content, search_pattern) = if case_sensitive {
                (content.to_string(), pattern)
            } else {
                (content.to_lowercase(), pattern.to_lowercase())
            };

            let mut start_pos = 0;
            while let Some(index) = search_content[start_pos..].find(&search_pattern) {
                let start = start_pos + index;
                let end = start + search_pattern.len();
                highlight_ranges.push((start, end, color));
                start_pos = start + 1; // Move forward to find overlapping matches
            }
        }

        if highlight_ranges.is_empty() {
            return Line::from(content);
        }

        // Sort ranges by start position
        highlight_ranges.sort_by(|a, b| a.0.cmp(&b.0));

        let mut spans = Vec::new();
        let mut last_index = 0;

        for (start, end, color) in highlight_ranges {
            // Add unhighlighted text before this range
            if start > last_index {
                spans.push(ratatui::text::Span::raw(&content[last_index..start]));
            }

            // Add highlighted text (only if we haven't already passed this range)
            if end > last_index {
                let highlight_start = start.max(last_index);
                spans.push(ratatui::text::Span::styled(
                    &content[highlight_start..end],
                    Style::default()
                        .fg(color)
                        .add_modifier(Modifier::BOLD),
                ));
                last_index = end;
            }
        }

        // Add any remaining unhighlighted text
        if last_index < content.len() {
            spans.push(ratatui::text::Span::raw(&content[last_index..]));
        }

        Line::from(spans)
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
            AppState::SearchMode => self.render_search_bar(bottom, buf),
            AppState::GotoLineMode => self.render_goto_line_bar(bottom, buf),
            AppState::FilterMode => self.render_filter_bar(bottom, buf),
            _ => self.render_footer(bottom, buf),
        }

        // Popups
        if self.app_state == AppState::FilterListView {
            let filter_area = popup_area(area, 50, 20);
            self.render_filter_list_popup(filter_area, buf);
        }
        if self.app_state == AppState::OptionsView {
            let options_area = popup_area(area, 40, 10);
            self.render_options_popup(options_area, buf);
        }
        if self.help.is_visible() {
            let help_area = popup_area(area, 45, 30);
            self.help.render(help_area, buf);
        }
        if let AppState::ErrorState(ref error_msg) = self.app_state {
            let error_area = popup_area(area, 70, 6);
            Clear.render(error_area, buf);
            let error_popup = Paragraph::new(error_msg.as_str())
                .block(
                    Block::default()
                        .title(" Error ")
                        .title_alignment(Alignment::Center)
                        .borders(ratatui::widgets::Borders::ALL),
                )
                .alignment(Alignment::Center);

            error_popup.render(error_area, buf);
        }
    }
}
