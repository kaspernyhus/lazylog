use crate::app::App;
use crate::ui::MAX_PATH_LENGTH;
use crate::ui::colors::{
    FILTER_MODE_BG, FILTER_MODE_FG, FOOTER_BG, SEARCH_MODE_BG, SEARCH_MODE_FG, TIME_FILTER_FG, TIME_FILTER_TEXT_FG,
};
use num_format::{Locale, ToFormattedString};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

impl App {
    /// Returns current line information (progression in the file).
    pub(super) fn get_progression(&self) -> (usize, usize, usize, usize) {
        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.visible_count(all_lines);
        let total_lines = self.log_buffer.get_total_lines_count();
        let current_line = self.viewport.selected_line + 1;
        let percent = if visible_lines > 0 {
            if current_line == visible_lines {
                100
            } else {
                (current_line * 100) / visible_lines
            }
        } else {
            0
        };
        (current_line, visible_lines, total_lines, percent)
    }

    /// Formats progression information for display in footers.
    pub(super) fn format_progression_text(&self) -> String {
        let (current_line, visible_lines, total_lines, percent) = self.get_progression();

        if visible_lines == total_lines {
            format!(
                "{}/{} {:3}%",
                current_line.to_formatted_string(&Locale::en_DK),
                total_lines.to_formatted_string(&Locale::en_DK),
                percent
            )
        } else {
            format!(
                "{}/{} ({}) {:3}%",
                current_line.to_formatted_string(&Locale::en_DK),
                visible_lines.to_formatted_string(&Locale::en_DK),
                total_lines.to_formatted_string(&Locale::en_DK),
                percent
            )
        }
    }

    pub(super) fn render_default_footer(&self, area: Rect, buf: &mut Buffer) {
        let max_width = MAX_PATH_LENGTH.min((self.viewport.width / 2).saturating_sub(13));

        let file_name = if self.file_manager.is_multi_file() {
            let formatted_paths: Vec<String> = self
                .file_manager
                .iter()
                .map(|file| {
                    let path_str = &file.path;
                    let max_path_len =
                        60usize.saturating_sub(9 * self.file_manager.count()) / self.file_manager.count();
                    let truncated = if path_str.chars().count() > max_path_len {
                        let skip = path_str.chars().count().saturating_sub(max_path_len);
                        let suffix: String = path_str.chars().skip(skip).collect();
                        format!("...{}", suffix)
                    } else {
                        format!(" {}", path_str)
                    };
                    format!("[{}]{}", file.file_id + 1, truncated)
                })
                .collect();

            let combined = formatted_paths.join(", ");
            if combined.chars().count() > max_width {
                let skip = combined.chars().count().saturating_sub(max_width);
                let suffix: String = combined.chars().skip(skip).collect();
                format!("...{}", suffix)
            } else {
                combined
            }
        } else if let Some(path) = self.file_manager.first_path() {
            if path.chars().count() > max_width {
                let skip = path.chars().count().saturating_sub(max_width);
                let suffix: String = path.chars().skip(skip).collect();
                format!("...{}", suffix)
            } else {
                path.to_string()
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
        if self.show_marked_lines_only {
            left_parts.push("| marked only".to_string());
        }
        if self.time_filter.is_some() {
            left_parts.push("| time-filtered".to_string());
        }
        let left = Line::from(left_parts.join(" "));
        let middle = Line::from("F1:View Help").centered();

        let (current_match, visible_matches, total_matches) = self.search.get_match_info();
        let progression_text = self.format_progression_text();

        let right = if visible_matches > 0 {
            let filtered_count = total_matches.saturating_sub(visible_matches);
            if filtered_count > 0 {
                Line::from(format!(
                    "{}/{} ({}) | {} ",
                    current_match, visible_matches, filtered_count, progression_text
                ))
                .right_aligned()
            } else {
                Line::from(format!("{}/{} | {} ", current_match, visible_matches, progression_text)).right_aligned()
            }
        } else {
            Line::from(progression_text + " ").right_aligned()
        };

        let footer = Block::default()
            .title_bottom(left)
            .title_bottom(middle)
            .title_bottom(right)
            .style(Style::default().bg(FOOTER_BG));
        footer.render(area, buf);
    }

    pub(super) fn render_search_footer(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = Line::from(format!("{}{}", self.get_input_prefix(), self.input.value())).left_aligned();
        let progression_text = self.format_progression_text();
        let progression = Line::from(progression_text + " ").right_aligned();

        let search_bar = Block::default()
            .title_bottom(search_prompt)
            .title_bottom(progression)
            .style(
                Style::default()
                    .fg(SEARCH_MODE_FG)
                    .bg(SEARCH_MODE_BG)
                    .add_modifier(Modifier::BOLD),
            );

        search_bar.render(area, buf);
    }

    pub(super) fn render_filter_footer(&self, area: Rect, buf: &mut Buffer) {
        let filter_prompt = Line::from(format!("{}{}", self.get_input_prefix(), self.input.value())).left_aligned();
        let progression_text = self.format_progression_text();
        let progression = Line::from(progression_text + " ").right_aligned();

        let filter_bar = Block::default()
            .title_bottom(filter_prompt)
            .title_bottom(progression)
            .style(
                Style::default()
                    .fg(FILTER_MODE_FG)
                    .bg(FILTER_MODE_BG)
                    .add_modifier(Modifier::BOLD),
            );

        filter_bar.render(area, buf);
    }

    pub(super) fn render_goto_line_footer(&self, area: Rect, buf: &mut Buffer) {
        let search_prompt = format!("{}{}", self.get_input_prefix(), self.input.value());
        let search_bar = Paragraph::new(search_prompt)
            .style(Style::default().bg(FOOTER_BG))
            .alignment(Alignment::Left);
        search_bar.render(area, buf);
    }

    pub(super) fn render_edit_time_filter_footer(&self, area: Rect, buf: &mut Buffer) {
        let prompt = Line::from(format!("{}{}", self.get_input_prefix(), self.input.value())).left_aligned();
        let progression_text = self.format_progression_text();
        let progression = Line::from(progression_text + " ").right_aligned();

        let bar = Block::default().title_bottom(prompt).title_bottom(progression).style(
            Style::default()
                .fg(TIME_FILTER_TEXT_FG)
                .bg(TIME_FILTER_FG)
                .add_modifier(Modifier::BOLD),
        );

        bar.render(area, buf);
    }

    pub(super) fn render_selection_footer(&self, area: Rect, buf: &mut Buffer) {
        let selection_text = if let Some((start, end)) = self.get_selection_range() {
            let num_lines = end - start + 1;
            format!(
                "-- VISUAL -- {} line{} selected ('y' to copy, Esc to cancel)",
                num_lines,
                if num_lines == 1 { "" } else { "s" }
            )
        } else {
            "-- VISUAL --".to_string()
        };

        let selection_prompt = Line::from(selection_text).left_aligned();
        let progression_text = self.format_progression_text();
        let progression = Line::from(progression_text + " ").right_aligned();

        let selection_bar = Block::default()
            .title_bottom(selection_prompt)
            .title_bottom(progression)
            .style(Style::default().bg(FOOTER_BG));

        selection_bar.render(area, buf);
    }
}
