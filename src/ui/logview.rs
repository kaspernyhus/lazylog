use crate::app::App;
use crate::colors::{
    MARK_INDICATOR, MARK_INDICATOR_COLOR, RIGHT_ARROW, SCROLLBAR_FG, SELECTION_BG,
};
use crate::highlighter::HighlightedLine;
use crate::log::Interval;
use crate::options::AppOption;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

impl App {
    /// Renders the vertical scrollbar.
    pub(super) fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let mut scrollbar_state = ScrollbarState::new(self.viewport.total_lines)
            .position(self.viewport.selected_line)
            .viewport_content_length(1);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .track_style(Style::default().fg(SCROLLBAR_FG))
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);
    }

    /// Renders the main log view.
    pub(super) fn render_logview(&self, area: Rect, buf: &mut Buffer) {
        let (start, end) = self.viewport.visible();
        let selection_range = self.get_selection_range();

        let viewport_lines: Vec<(&str, usize)> = self
            .log_buffer
            .get_active_lines_iter(Interval::Range(start, end))
            .map(|log_line| {
                (
                    self.options.apply_to_line(log_line.content()),
                    log_line.index,
                )
            })
            .collect();

        let items: Vec<Line> = viewport_lines
            .iter()
            .enumerate()
            .map(|(viewport_idx, (line, line_index))| {
                let text = if self.viewport.horizontal_offset >= line.len() {
                    ""
                } else {
                    &line[self.viewport.horizontal_offset..]
                };
                let is_marked = self.marking.is_marked(*line_index);
                let viewport_line = start + viewport_idx;
                let is_selected = if let Some((sel_start, sel_end)) = selection_range {
                    viewport_line >= sel_start && viewport_line <= sel_end
                } else {
                    false
                };
                self.process_line(
                    line,
                    text,
                    self.viewport.horizontal_offset,
                    is_marked,
                    is_selected,
                )
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
        is_marked: bool,
        is_selected: bool,
    ) -> Line<'a> {
        let enable_colors = !self.options.is_enabled(AppOption::DisableColors);

        let highlighted = self
            .highlighter
            .highlight_line(full_line, line_offset, enable_colors);

        let mark_indicator = if is_marked {
            Span::styled(MARK_INDICATOR, Style::default().fg(MARK_INDICATOR_COLOR))
        } else {
            Span::raw(" ")
        };

        let mut line = if highlighted.segments.is_empty() {
            let mut spans = vec![mark_indicator];
            if !visible_text.is_empty() {
                spans.push(Span::raw(visible_text));
            }
            Line::from(spans)
        } else {
            let mut line = build_line_from_highlighted(visible_text, highlighted);
            line.spans.insert(0, mark_indicator);
            line
        };

        if is_selected {
            line = line.style(Style::default().bg(SELECTION_BG));
        }

        line
    }
}

/// Builds a styled Line from a HighlightedLine.
pub(super) fn build_line_from_highlighted<'a>(
    content: &'a str,
    highlighted: HighlightedLine,
) -> Line<'a> {
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
