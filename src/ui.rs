use crate::app::{App, AppState};
use crate::highlighter::HighlightedLine;
use crate::log::Interval;
use ratatui::text::{Line, Span};
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

/// Symbol used to indicate the selected line.
pub const RIGHT_ARROW: &str = "▶ ";
/// Maximum length for file path display in footer.
const MAX_PATH_LENGTH: usize = 42;
/// Background color for footer and title bars.
const GRAY_COLOR: Color = Color::Indexed(237);

/// Calculates a centered popup area within the given rect.
///
/// The popup will be centered with at least 2 characters margin on all sides.
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
    /// Returns current line information (progression in the file).
    ///
    /// Returns (current_line, total_lines, percent).
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

    /// Renders the default footer bar in LogView mode.
    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let file_name = if let Some(path) = &self.log_buffer.file_path {
            let abs_path = std::fs::canonicalize(path)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| path.to_string());

            if abs_path.len() > MAX_PATH_LENGTH {
                let skip = abs_path.len() - MAX_PATH_LENGTH;
                format!("...{}", &abs_path[skip..])
            } else {
                abs_path
            }
        } else {
            "".to_string()
        };

        let mut left_parts = vec![file_name];
        if self.streaming_paused && self.log_buffer.streaming {
            left_parts.push("PAUSED".to_string());
        }
        if self.viewport.follow_mode && self.log_buffer.streaming {
            left_parts.push("| follow".to_string());
        }
        if self.viewport.center_cursor_mode {
            left_parts.push("| center".to_string());
        }
        let left = Line::from(left_parts.join(" "));
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

    /// Renders the search bar footer in SearchMode.
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

    /// Renders the goto line bar footer in GotoLineMode.
    fn render_goto_line_bar(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = format!("Go to line: {}", self.input_query);
        let search_bar = Paragraph::new(search_prompt)
            .style(Style::default().bg(GRAY_COLOR))
            .alignment(Alignment::Left);
        search_bar.render(area, buf);
    }

    /// Renders the save to file bar footer in SaveToFileMode.
    fn render_save_to_file_bar(&self, area: Rect, buf: &mut Buffer) {
        let save_prompt = format!("Save to file: {}", self.input_query);
        let save_bar = Paragraph::new(save_prompt)
            .style(Style::default().bg(GRAY_COLOR))
            .alignment(Alignment::Left);
        save_bar.render(area, buf);
    }

    /// Renders the filter bar footer in FilterMode.
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

    /// Renders the edit filter popup in EditFilterMode.
    fn render_edit_filter_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let edit_prompt = self.input_query.clone();
        let popup = Paragraph::new(edit_prompt)
            .block(
                Block::default()
                    .title(" Edit Filter ")
                    .title_alignment(Alignment::Center)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Left);

        popup.render(area, buf);
    }

    /// Renders the filter list popup in FilterListView mode.
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
            return;
        }

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
            .highlight_symbol("")
            .highlight_style(
                Style::default()
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(filter_list, area, buf, &mut list_state);
    }

    /// Renders the events popup in EventsView mode.
    fn render_events_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        if self.event_tracker.is_empty() {
            let no_events_text = vec![Line::from("No events found")];
            let popup = Paragraph::new(no_events_text)
                .block(
                    Block::default()
                        .title(" Log Events ")
                        .title_alignment(Alignment::Center)
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(Style::default().fg(Color::White))
                        .style(Style::default().bg(Color::Blue)),
                )
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        // Get log lines for preview
        let mut items: Vec<Line> = Vec::new();
        for event in self.event_tracker.events() {
            // Get the log line content
            let log_line = self
                .log_buffer
                .get_lines_iter(Interval::All)
                .find(|line| line.index == event.line_index);

            if let Some(log_line) = log_line {
                let content = log_line.content();
                // Truncate long lines for preview
                let preview = if content.len() > 80 {
                    format!("{}...", &content[..77])
                } else {
                    content.to_string()
                };

                // Format: [Event Name] Line preview with colored event name
                let spans = vec![
                    Span::raw("["),
                    Span::styled(
                        event.event_name.clone(),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("] "),
                    Span::styled(preview, Style::default().fg(Color::White)),
                ];

                items.push(Line::from(spans));
            }
        }

        // Render block first
        let block = Block::default()
            .title(" Log Events ")
            .title_alignment(Alignment::Center)
            .borders(ratatui::widgets::Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        let inner_area = block.inner(area);

        // Split inner area for list and scrollbar
        let [list_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        block.render(area, buf);

        // Create list without block (block is rendered separately above)
        let events_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(
                Style::default()
                    .bg(Color::LightBlue)
                    .fg(Color::Rgb(255, 255, 255))
                    .add_modifier(Modifier::BOLD),
            );

        // Create list state with current selection
        let mut list_state = ListState::default();
        if !self.event_tracker.is_empty() {
            list_state.select(Some(self.event_tracker.selected_index()));
        }

        StatefulWidget::render(events_list, list_area, buf, &mut list_state);

        // Render scrollbar
        let mut scrollbar_state = ScrollbarState::new(self.event_tracker.count())
            .position(self.event_tracker.selected_index())
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }

    /// Renders the display options popup in OptionsView mode.
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

    /// Renders the event filter popup in EventFilterView mode.
    fn render_event_filter_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let event_filters = self.event_tracker.get_event_filters();
        if event_filters.is_empty() {
            let no_filters_text = vec![Line::from("No event filters available")];
            let popup = Paragraph::new(no_filters_text)
                .block(
                    Block::default()
                        .title(" Event Filters ")
                        .title_alignment(Alignment::Center)
                        .borders(ratatui::widgets::Borders::ALL)
                        .border_style(Style::default().fg(Color::White)),
                )
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        let items: Vec<Line> = event_filters
            .iter()
            .map(|filter| {
                let checkbox = if filter.enabled { "[x]" } else { "[ ]" };
                let count = self.event_tracker.get_event_count(&filter.name);
                let content = format!("{} {} ({})", checkbox, filter.name, count);

                if filter.enabled {
                    Line::from(content).style(Style::default().fg(Color::Green))
                } else {
                    Line::from(content).style(Style::default().fg(Color::Gray))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        if !event_filters.is_empty() {
            list_state.select(Some(self.event_tracker.filter_selected_index()));
        }

        let event_filter_list = List::new(items)
            .block(
                Block::default()
                    .title(" Event Filters ")
                    .title_alignment(Alignment::Center)
                    .borders(ratatui::widgets::Borders::ALL)
                    .border_style(Style::default().fg(Color::White)),
            )
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        StatefulWidget::render(event_filter_list, area, buf, &mut list_state);
    }

    /// Renders a centered popup that adapts to content size.
    fn render_popup(
        &self,
        message: &str,
        title: &str,
        title_color: Color,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let lines: Vec<&str> = message.split('\n').collect();
        let max_line_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);

        let popup_width = (max_line_width as u16 + 6).min(area.width.saturating_sub(4));
        let popup_height = (lines.len() as u16 + 4).min(area.height.saturating_sub(4));
        let popup_area = popup_area(area, popup_width, popup_height);

        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(Style::default().fg(title_color))
            .title_alignment(Alignment::Center)
            .borders(ratatui::widgets::Borders::ALL)
            .padding(ratatui::widgets::Padding::uniform(1));

        let popup = Paragraph::new(message)
            .block(block)
            .alignment(Alignment::Center);

        popup.render(popup_area, buf);
    }

    /// Renders a centered message popup that adapts to content size.
    fn render_message_popup(&self, message: &str, area: Rect, buf: &mut Buffer) {
        self.render_popup(message, "Message", Color::White, area, buf);
    }

    /// Renders a centered error popup that adapts to content size.
    fn render_error_popup(&self, error_msg: &str, area: Rect, buf: &mut Buffer) {
        self.render_popup(error_msg, "Error", Color::Red, area, buf);
    }

    /// Renders the vertical scrollbar.
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

    /// Renders the main log view.
    fn render_logview(&self, area: Rect, buf: &mut Buffer) {
        let (start, end) = self.viewport.visible();

        let viewport_lines: Vec<String> = self
            .log_buffer
            .get_lines_iter(Interval::Range(start, end))
            .map(|log_line| self.display_options.apply_to_line(log_line.content()))
            .collect();

        let items: Vec<Line> = viewport_lines
            .iter()
            .map(|line| {
                let text = if self.viewport.horizontal_offset >= line.len() {
                    ""
                } else {
                    &line[self.viewport.horizontal_offset..]
                };
                self.process_line(line, text, self.viewport.horizontal_offset)
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

    /// Applies syntax highlighting to a single line.
    fn process_line<'a>(
        &self,
        full_line: &'a str,
        visible_text: &'a str,
        line_offset: usize,
    ) -> Line<'a> {
        let enable_colors = !self.display_options.is_enabled("Disable Colors");

        let highlighted = self
            .highlighter
            .highlight_line(full_line, line_offset, enable_colors);

        if highlighted.segments.is_empty() {
            return Line::from(visible_text);
        }

        build_line_from_highlighted(visible_text, highlighted)
    }
}

/// Builds a styled Line from a HighlightedLine.
fn build_line_from_highlighted<'a>(content: &'a str, highlighted: HighlightedLine) -> Line<'a> {
    // Build spans from segments
    let mut spans = Vec::new();
    let mut pos = 0;

    for segment in highlighted.segments {
        // Add unhighlighted text before this segment
        if segment.start > pos {
            spans.push(Span::raw(&content[pos..segment.start]));
        }

        // Add the segment with style
        spans.push(Span::styled(
            &content[segment.start..segment.end],
            segment.style.to_ratatui(),
        ));

        pos = segment.end;
    }

    // Add any remaining text after the last segment
    if pos < content.len() {
        spans.push(Span::raw(&content[pos..]));
    }

    Line::from(spans)
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
        let title_middle = Line::from(" Lazylog ").centered();
        let title_right = Line::from(format!("v{}", env!("CARGO_PKG_VERSION")))
            .right_aligned()
            .style(Style::default().fg(Color::DarkGray));
        let title = Block::default()
            .title_bottom(title_middle)
            .title_bottom(title_right)
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
            AppState::SaveToFileMode => self.render_save_to_file_bar(bottom, buf),
            _ => self.render_footer(bottom, buf),
        }

        // Popups
        if self.app_state == AppState::FilterListView {
            let filter_area = popup_area(area, 50, 20);
            self.render_filter_list_popup(filter_area, buf);
        }
        if self.app_state == AppState::EditFilterMode {
            let edit_area = popup_area(area, 60, 3);
            self.render_edit_filter_popup(edit_area, buf);
        }
        if self.app_state == AppState::OptionsView {
            let options_area = popup_area(area, 40, 10);
            self.render_options_popup(options_area, buf);
        }
        if self.app_state == AppState::EventsView {
            let events_area = popup_area(area, 118, 35);
            self.render_events_popup(events_area, buf);
        }
        if self.app_state == AppState::EventsFilterView {
            let events_area = popup_area(area, 118, 35);
            let event_filter_area = popup_area(area, 40, 15);
            self.render_events_popup(events_area, buf);
            self.render_event_filter_popup(event_filter_area, buf);
        }
        if self.help.is_visible() {
            let help_area = popup_area(area, 45, 30);
            self.help.render(help_area, buf);
        }
        if let AppState::Message(ref message) = self.app_state {
            self.render_message_popup(message, area, buf);
        }
        if let AppState::ErrorState(ref error_msg) = self.app_state {
            self.render_error_popup(error_msg, area, buf);
        }
    }
}
