mod footer;
mod lists;
mod logview;
mod popups;
mod scrollable_list;

use crate::app::{App, Overlay, ViewState};
use crate::colors::{GRAY_COLOR, WHITE_COLOR};
pub use popups::popup_area;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Widget},
};

/// Maximum length for file path display in footer.
const MAX_PATH_LENGTH: usize = 60;

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [top, middle, bottom] =
            Layout::vertical([Constraint::Length(1), Constraint::Fill(1), Constraint::Length(1)]).areas(area);

        let [log_view_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(middle);

        // Title
        let title_middle = Line::from(" Lazylog ").centered();
        let title_right = Line::from(format!("v{}", env!("CARGO_PKG_VERSION")))
            .right_aligned()
            .style(Style::default().fg(WHITE_COLOR));
        let title = Block::default()
            .title_bottom(title_middle)
            .title_bottom(title_right)
            .style(Style::default().bg(GRAY_COLOR));
        title.render(top, buf);

        // Main view
        self.render_logview(log_view_area, buf);
        self.render_scrollbar(scrollbar_area, buf);

        // Footer
        match (&self.view_state, &self.overlay) {
            (ViewState::ActiveSearchMode, _) => self.render_search_footer(bottom, buf),
            (ViewState::GotoLineMode, _) => self.render_goto_line_footer(bottom, buf),
            (ViewState::ActiveFilterMode, _) => self.render_filter_footer(bottom, buf),
            (ViewState::SelectionMode, _) => self.render_selection_footer(bottom, buf),
            _ => self.render_default_footer(bottom, buf),
        }

        // Popups
        match self.view_state {
            ViewState::FilterView => {
                let filter_area = popup_area(area, 118, 35);
                self.render_filter_list(filter_area, buf);
            }
            ViewState::OptionsView => {
                let options_area = popup_area(area, 42, 10);
                self.render_options(options_area, buf);
            }
            ViewState::EventsView => {
                let events_area = popup_area(area, 118, 35);
                self.render_events_list(events_area, buf);
            }
            ViewState::MarksView => {
                let marks_area = popup_area(area, 118, 35);
                self.render_marks_list(marks_area, buf);
            }
            _ => {}
        }

        // Overlays
        if let Some(ref overlay) = self.overlay {
            match overlay {
                Overlay::EditFilter => {
                    let edit_area = popup_area(area, 60, 3);
                    self.render_edit_filter_popup(edit_area, buf);
                }
                Overlay::EventsFilter => {
                    let event_filter_area = popup_area(area, 50, 25);
                    self.render_event_filter_popup(event_filter_area, buf);
                }
                Overlay::MarkName => {
                    let name_input_area = popup_area(area, 60, 3);
                    self.render_mark_name_input_popup(name_input_area, buf);
                }
                Overlay::SaveToFile => {
                    let save_area = popup_area(area, 60, 3);
                    self.render_save_to_file_popup(save_area, buf);
                }
                Overlay::Message(message) => {
                    self.render_message_popup(message, area, buf);
                }
                Overlay::Error(error_msg) => {
                    self.render_error_popup(error_msg, area, buf);
                }
            }
        }

        // Help popup
        if self.help.is_visible() {
            let help_area = popup_area(area, 50, 32);
            self.help.render(help_area, buf);
        }
    }
}
