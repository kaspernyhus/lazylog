use crate::app::App;
use crate::colors::{MESSAGE_BORDER, MESSAGE_ERROR_FG, MESSAGE_INFO_FG, WHITE_COLOR};
use ratatui::widgets::{BorderType, Padding};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

/// Calculates a centered popup area within the given rect.
///
/// The popup will be centered with at least 2 characters margin on all sides.
pub(super) fn popup_area(area: Rect, width: u16, height: u16) -> Rect {
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
    /// Renders a centered popup that adapts to content size.
    pub(super) fn render_popup(
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

        let border_color = if title == "Error" {
            MESSAGE_ERROR_FG
        } else {
            MESSAGE_BORDER
        };

        let block = Block::default()
            .title(format!(" {} ", title))
            .title_style(Style::default().fg(title_color))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .padding(Padding::uniform(1));

        let popup = Paragraph::new(message)
            .block(block)
            .alignment(Alignment::Center);

        popup.render(popup_area, buf);
    }

    /// Renders a centered message popup that adapts to content size.
    pub(super) fn render_message_popup(&self, message: &str, area: Rect, buf: &mut Buffer) {
        self.render_popup(message, "Message", MESSAGE_INFO_FG, area, buf);
    }

    /// Renders a centered error popup that adapts to content size.
    pub(super) fn render_error_popup(&self, error_msg: &str, area: Rect, buf: &mut Buffer) {
        self.render_popup(error_msg, "Error", MESSAGE_ERROR_FG, area, buf);
    }

    /// Renders the save to file bar footer in SaveToFileMode.
    pub(super) fn render_save_to_file_popup(&self, area: Rect, buf: &mut Buffer) {
        Clear.render(area, buf);

        let prompt = self.input_query.clone();
        let popup = Paragraph::new(prompt)
            .block(
                Block::default()
                    .title(" Save to file ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(WHITE_COLOR)),
            )
            .style(Style::default().fg(WHITE_COLOR))
            .alignment(Alignment::Left);

        popup.render(area, buf);
    }
}
