use std::collections::HashSet;

use super::colors::{
    EXPANDED_LINE_FG, EXPANSION_PREFIX, FILE_ID_COLORS, MARK_INDICATOR, MARK_INDICATOR_COLOR, RIGHT_ARROW,
    SCROLLBAR_FG, SCROLLBAR_MARK_INDICATOR, SCROLLBAR_SEARCH_INDICATOR, SELECTION_BG,
};
use crate::highlighter::HighlightedLine;
use crate::options::AppOption;
use crate::resolver::Tag;
use crate::{app::App, log::LogLine};
use ratatui::symbols::line::{VERTICAL, VERTICAL_LEFT};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

/// Represents an indicator to display on the scrollbar
struct ScrollbarIndicator {
    /// Position on scrollbar (0.0 to 1.0 representing top to bottom)
    position: f64,
    /// Color of the indicator
    color: Color,
}

impl App {
    /// Renders the vertical scrollbar.
    pub(super) fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let mut scrollbar_state = ScrollbarState::new(self.viewport.total_lines)
            .position(self.viewport.selected_line)
            .viewport_content_length(1);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .track_symbol(Some(VERTICAL))
            .track_style(Style::default().fg(SCROLLBAR_FG))
            .thumb_style(Style::new().bg(Color::Indexed(253)))
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, area, buf, &mut scrollbar_state);

        for indicator in self.collect_scrollbar_indicators() {
            let y_offset = (indicator.position * area.height as f64).round() as u16;
            let y = area.y + y_offset;

            if y >= area.y + area.height {
                continue;
            }

            let indicator_x = area.x;
            buf[(indicator_x, y)]
                .set_symbol(VERTICAL_LEFT)
                .set_style(Style::default().fg(indicator.color));
        }
    }

    /// Collects all scrollbar indicators for search matches, marks, and events.
    fn collect_scrollbar_indicators(&self) -> Vec<ScrollbarIndicator> {
        let mut indicators = Vec::new();

        let total_viewport_lines = self.viewport.total_lines;
        if total_viewport_lines == 0 {
            return indicators;
        }

        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(all_lines);

        // Add search match indicators
        if self.search.get_active_pattern().is_some() {
            for &match_idx in self.search.get_match_indices() {
                let position = match_idx as f64 / total_viewport_lines as f64;
                indicators.push(ScrollbarIndicator {
                    position,
                    color: SCROLLBAR_SEARCH_INDICATOR,
                });
            }
        }

        // Add mark indicators
        for mark in self.marking.get_marks() {
            // Find viewport index for this mark's log index
            if let Some(viewport_idx) = visible_lines.iter().position(|v| v.log_index == mark.line_index) {
                let position = viewport_idx as f64 / total_viewport_lines as f64;
                indicators.push(ScrollbarIndicator {
                    position,
                    color: SCROLLBAR_MARK_INDICATOR,
                });
            }
        }

        // Sort by position
        indicators.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap_or(std::cmp::Ordering::Equal));

        indicators
    }

    /// Renders the main log view.
    pub(super) fn render_log_view(&self, area: Rect, buf: &mut Buffer) {
        let (start, end) = self.viewport.visible();
        let selection_range = self.get_selection_range();

        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(all_lines);

        let viewport_data = if start < visible_lines.len() {
            let range_end = end.min(visible_lines.len());
            visible_lines[start..range_end].iter().collect()
        } else {
            Vec::new()
        };

        let horizontal_offset = self.viewport.horizontal_offset;
        let enable_colors = !self.options.is_enabled(AppOption::DisableColors);

        let items: Vec<Line> = viewport_data
            .iter()
            .enumerate()
            .map(|(offset, vl)| {
                let log_line = &all_lines[vl.log_index];
                let viewport_line = self.options.apply_to_line(log_line.content());
                let text = if horizontal_offset >= viewport_line.len() {
                    ""
                } else {
                    &viewport_line[horizontal_offset..]
                };

                let viewport_line_index = start + offset;
                let is_selected = if let Some((sel_start, sel_end)) = selection_range {
                    viewport_line_index >= sel_start && viewport_line_index <= sel_end
                } else {
                    false
                };

                let mut tags = vl.tags.clone();
                if is_selected {
                    tags.insert(Tag::Selected);
                }

                self.process_line_impl(log_line, viewport_line, text, horizontal_offset, &tags, enable_colors)
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
    fn process_line_impl<'a>(
        &self,
        log_line: &LogLine,
        transformed_line: &'a str,
        visible_text: &'a str,
        line_offset: usize,
        tags: &HashSet<Tag>,
        enable_colors: bool,
    ) -> Line<'a> {
        let highlighted = self.highlighter.highlight_line(log_line.index, transformed_line);
        let highlighted = self.highlighter.adjust_for_viewport_offset(highlighted, line_offset);

        let mark_indicator = if tags.contains(&Tag::Marked) {
            Span::styled(MARK_INDICATOR, Style::default().fg(MARK_INDICATOR_COLOR))
        } else {
            Span::raw(" ")
        };

        let file_id_indicator = if let Some(id) = log_line.log_file_id {
            let indicator = format!("[{}] ", id + 1);
            let color = if log_line.timestamp.is_some() {
                FILE_ID_COLORS[id % FILE_ID_COLORS.len()]
            } else {
                Color::Red
            };
            Span::styled(indicator, Style::default().fg(color)).add_modifier(Modifier::BOLD)
        } else {
            Span::raw("")
        };

        let is_expanded = tags.contains(&Tag::Expanded);

        let expansion_indicator = if is_expanded {
            Span::styled(EXPANSION_PREFIX, Style::default().fg(EXPANDED_LINE_FG))
        } else {
            Span::raw("")
        };

        let mut line = if highlighted.segments.is_empty() {
            let mut spans = vec![mark_indicator, file_id_indicator, expansion_indicator];
            if !visible_text.is_empty() {
                let text_style = if is_expanded {
                    Style::default().fg(EXPANDED_LINE_FG)
                } else {
                    Style::default()
                };
                spans.push(Span::styled(visible_text, text_style));
            }
            Line::from(spans)
        } else {
            let mut line = build_line_from_highlighted(visible_text, highlighted, enable_colors);
            if is_expanded {
                // Dim if the span has no explicit foreground color
                for span in &mut line.spans {
                    if span.style.fg.is_none() {
                        span.style = span.style.fg(EXPANDED_LINE_FG);
                    }
                }
            }
            line.spans.insert(0, expansion_indicator);
            line.spans.insert(0, file_id_indicator);
            line.spans.insert(0, mark_indicator);
            line
        };

        if tags.contains(&Tag::Selected) {
            line = line.style(Style::default().bg(SELECTION_BG));
        }

        line
    }
}

/// Builds a styled Line from a HighlightedLine.
pub(super) fn build_line_from_highlighted<'a>(
    content: &'a str,
    highlighted: HighlightedLine,
    enable_colors: bool,
) -> Line<'a> {
    if !enable_colors {
        return Line::raw(content);
    }

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
