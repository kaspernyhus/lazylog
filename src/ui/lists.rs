use crate::app::App;
use crate::colors::{
    EVENT_LINE_PREVIEW, EVENT_LIST_BG, EVENT_LIST_HIGHLIGHT_BG, EVENT_NAME_FG, FILTER_DISABLED_FG,
    FILTER_ENABLED_FG, FILTER_LIST_HIGHLIGHT_BG, FILTER_MODE_BG, MARK_LINE_PREVIEW,
    MARK_LIST_HIGHLIGHT_BG, MARK_MODE_BG, MARK_NAME_FG, OPTION_DISABLED_FG, OPTION_ENABLED_FG,
    RIGHT_ARROW, WHITE_COLOR,
};
use crate::filter::FilterMode;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

impl App {
    pub(super) fn render_options(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let items: Vec<Line> = self
            .options
            .options
            .iter()
            .map(|option| {
                let checkbox = if option.enabled { "[x]" } else { "[ ]" };
                let content = format!("{} {}", checkbox, option.name);

                if option.enabled {
                    Line::from(content).style(Style::default().fg(OPTION_ENABLED_FG))
                } else {
                    Line::from(content).style(Style::default().fg(OPTION_DISABLED_FG))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        if !self.options.options.is_empty() {
            list_state.select(Some(self.options.selected_index));
        }

        let options_list = List::new(items)
            .block(
                Block::default()
                    .title(" Display Options ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(WHITE_COLOR)),
            )
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        StatefulWidget::render(options_list, area, buf, &mut list_state);
    }

    pub(super) fn render_filter_list(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let filter_patterns = self.filter.get_filter_patterns();

        let block = Block::default()
            .title(" Filters ")
            .title_alignment(Alignment::Center)
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(FILTER_MODE_BG));

        if filter_patterns.is_empty() {
            let popup = Paragraph::new("No filters configured")
                .block(block)
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        let items: Vec<Line> = filter_patterns
            .iter()
            .map(|pattern| {
                let mode_str = match pattern.mode {
                    FilterMode::Include => "IN",
                    FilterMode::Exclude => "EX",
                };
                let case_str = if pattern.case_sensitive { "Aa" } else { "aa" };

                let content = format!(" [{}] [{}] {}", mode_str, case_str, pattern.pattern);

                if pattern.enabled {
                    Line::from(content).style(Style::default().fg(FILTER_ENABLED_FG))
                } else {
                    Line::from(content).style(Style::default().fg(FILTER_DISABLED_FG))
                }
            })
            .collect();

        let mut list_state = ListState::default();
        if !filter_patterns.is_empty() {
            list_state.select(Some(self.filter.get_selected_pattern_index()));
        }

        let filter_list = List::new(items)
            .block(block)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(
                Style::default()
                    .bg(FILTER_LIST_HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD),
            );

        StatefulWidget::render(filter_list, area, buf, &mut list_state);
    }

    pub(super) fn render_edit_filter_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let edit_prompt = self.input.value();
        let popup = Paragraph::new(edit_prompt)
            .block(
                Block::default()
                    .title(" Edit Filter ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(FILTER_MODE_BG)),
            )
            .style(Style::default().fg(WHITE_COLOR))
            .alignment(Alignment::Left);

        popup.render(area, buf);
    }

    pub(super) fn render_events_list(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let block = Block::default()
            .title(" Log Events ")
            .title_alignment(Alignment::Center)
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(EVENT_LIST_BG));

        if self.event_tracker.is_empty() {
            let popup = Paragraph::new("No events found")
                .block(block)
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        let inner_area = block.inner(area);

        let [list_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        let event_items: Vec<_> = self.event_tracker.iter_items().collect();

        let max_name_length = event_items
            .iter()
            .map(|item| item.name().len())
            .max()
            .unwrap_or(0);

        let available_width = list_area
            .width
            .saturating_sub(max_name_length as u16)
            .saturating_sub(4)
            .max(20) as usize; // Minimum 20 characters

        let mut items: Vec<Line> = Vec::new();
        for item in &event_items {
            let log_line = self.log_buffer.get_line(item.line_index());

            if let Some(log_line) = log_line {
                let content = log_line.content();
                let preview = if content.len() > available_width {
                    format!("{}...", &content[..available_width.saturating_sub(3)])
                } else {
                    content.to_string()
                };

                let padding = " ".repeat(max_name_length - item.name().len());

                let (name_color, preview_color) = if item.is_mark() {
                    (MARK_MODE_BG, MARK_LINE_PREVIEW)
                } else {
                    (EVENT_NAME_FG, EVENT_LINE_PREVIEW)
                };

                let spans = vec![
                    Span::raw(" "),
                    Span::raw(padding),
                    Span::styled(
                        item.name().to_string(),
                        Style::default().fg(name_color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(preview, Style::default().fg(preview_color)),
                ];

                items.push(Line::from(spans));
            }
        }

        self.event_tracker
            .set_viewport_height(list_area.height as usize);

        block.render(area, buf);

        let events_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(
                Style::default()
                    .bg(EVENT_LIST_HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD),
            );

        let mut list_state = ListState::default();
        if !self.event_tracker.is_empty() {
            list_state.select(Some(self.event_tracker.selected_index()));
            *list_state.offset_mut() = self.event_tracker.viewport_offset();
        }

        StatefulWidget::render(events_list, list_area, buf, &mut list_state);

        let mut scrollbar_state = ScrollbarState::new(self.event_tracker.count())
            .position(self.event_tracker.selected_index())
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }

    pub(super) fn render_event_filter_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let event_filters = self.event_tracker.get_event_filters();

        let block = Block::default()
            .title(" Event Filters ")
            .title_alignment(Alignment::Center)
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(EVENT_LIST_BG));

        if event_filters.is_empty() {
            let popup = Paragraph::new("No event filters available")
                .block(block)
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        let inner_area = block.inner(area);

        let [list_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        let list_items: Vec<Line> = event_filters
            .iter()
            .map(|filter| {
                let checkbox = if filter.enabled { "[x]" } else { "[ ]" };
                let count = self.event_tracker.get_event_count(&filter.name);
                let content = format!("{} {} ({})", checkbox, filter.name, count);

                if filter.enabled {
                    Line::from(content).style(Style::default().fg(FILTER_ENABLED_FG))
                } else {
                    Line::from(content).style(Style::default().fg(FILTER_DISABLED_FG))
                }
            })
            .collect();

        self.event_tracker
            .set_filter_viewport_height(list_area.height as usize);

        block.render(area, buf);

        let event_filter_list = List::new(list_items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        let mut list_state = ListState::default();
        if !event_filters.is_empty() {
            list_state.select(Some(self.event_tracker.filter_selected_index()));
            *list_state.offset_mut() = self.event_tracker.filter_viewport_offset();
        }

        StatefulWidget::render(event_filter_list, list_area, buf, &mut list_state);

        let mut scrollbar_state = ScrollbarState::new(self.event_tracker.filter_count())
            .position(self.event_tracker.filter_selected_index())
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }

    pub(super) fn render_marks_list(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let filtered_marks = self.get_filtered_marks();

        let block = Block::default()
            .title(" Marked Lines ")
            .title_alignment(Alignment::Center)
            .title_style(Style::default().bold())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(MARK_MODE_BG));

        if filtered_marks.is_empty() {
            let popup = Paragraph::new("No marked lines")
                .block(block)
                .alignment(Alignment::Center);
            popup.render(area, buf);
            return;
        }

        let max_name_length = filtered_marks
            .iter()
            .filter_map(|m| m.name.as_ref().map(|n| n.len()))
            .max()
            .unwrap_or(0);

        let inner_area = block.inner(area);

        let [list_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        let available_width = list_area
            .width
            .saturating_sub(max_name_length as u16)
            .saturating_sub(4)
            .max(20) as usize; // Minimum 20 characters

        let items: Vec<Line> = filtered_marks
            .iter()
            .map(|mark| {
                let log_line = self
                    .log_buffer
                    .get_line(mark.line_index)
                    .map(|l| l.content.as_str())
                    .unwrap_or("");

                let preview = if log_line.len() > available_width {
                    format!("{}...", &log_line[..available_width.saturating_sub(3)])
                } else {
                    log_line.to_string()
                };

                if let Some(name) = &mark.name {
                    let padding = " ".repeat(max_name_length - name.len());

                    let spans = vec![
                        Span::raw(" "),
                        Span::raw(padding),
                        Span::styled(
                            name.clone(),
                            Style::default()
                                .fg(MARK_NAME_FG)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(" "),
                        Span::styled(preview, Style::default().fg(MARK_LINE_PREVIEW)),
                    ];
                    Line::from(spans)
                } else {
                    let padding = " ".repeat(max_name_length);

                    let spans = vec![
                        Span::raw(" "),
                        Span::raw(padding),
                        Span::raw(" "),
                        Span::styled(preview, Style::default().fg(MARK_LINE_PREVIEW)),
                    ];
                    Line::from(spans)
                }
            })
            .collect();

        self.marking.set_viewport_height(list_area.height as usize);

        block.render(area, buf);

        let marks_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(
                Style::default()
                    .bg(MARK_LIST_HIGHLIGHT_BG)
                    .add_modifier(Modifier::BOLD),
            );

        let mut list_state = ListState::default();
        if !filtered_marks.is_empty() {
            list_state.select(Some(self.marking.selected_index()));
            *list_state.offset_mut() = self.marking.viewport_offset();
        }

        StatefulWidget::render(marks_list, list_area, buf, &mut list_state);

        let mut scrollbar_state = ScrollbarState::new(filtered_marks.len())
            .position(self.marking.selected_index())
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }

    pub(super) fn render_mark_name_input_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let input_text = self.input.value();
        let popup = Paragraph::new(input_text)
            .block(
                Block::default()
                    .title(" Name Mark ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(MARK_MODE_BG)),
            )
            .style(Style::default().fg(WHITE_COLOR))
            .alignment(Alignment::Left);

        popup.render(area, buf);
    }
}
