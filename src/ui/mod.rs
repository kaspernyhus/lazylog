pub mod colors;
mod footer;
mod lists;
mod logview;
mod popups;
mod scrollable_list;

use crate::app::{App, Overlay, ViewState};
use colors::{GRAY_COLOR, WHITE_COLOR};
pub use popups::popup_area;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Widget},
};

/// Maximum length for file path display in footer.
const MAX_PATH_LENGTH: usize = 90;

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
        self.render_log_view(log_view_area, buf);
        self.render_scrollbar(scrollbar_area, buf);

        // Footer
        match (&self.view_state, &self.overlay) {
            (ViewState::ActiveSearchMode, _) => self.render_search_footer(bottom, buf),
            (ViewState::GotoLineMode, _) => self.render_goto_line_footer(bottom, buf),
            (ViewState::ActiveFilterMode, _) => self.render_filter_footer(bottom, buf),
            (ViewState::SelectionMode, _) => self.render_selection_footer(bottom, buf),
            (_, Some(Overlay::EditTimeFilter)) => self.render_edit_time_filter_footer(bottom, buf),
            _ => self.render_default_footer(bottom, buf),
        }

        // Popups
        if let Some((w, h)) = self.view_state.popup_size() {
            let popup = popup_area(area, w, h);
            match self.view_state {
                ViewState::FilterView => self.render_filter_list(popup, buf),
                ViewState::OptionsView => self.render_options(popup, buf),
                ViewState::EventsView => self.render_events_list(popup, buf),
                ViewState::MarksView => self.render_marks_list(popup, buf),
                ViewState::FilesView => self.render_files_list(popup, buf),
                ViewState::TimeFilterView => self.render_time_filter_popup(popup, buf),
                _ => {}
            }
        }

        // Overlays
        if let Some(ref overlay) = self.overlay {
            let overlay_area = overlay.popup_size().map(|(w, h)| popup_area(area, w, h));

            match overlay {
                Overlay::EditFilter => {
                    self.render_edit_filter_popup(overlay_area.unwrap(), buf);
                }
                Overlay::EventsFilter => {
                    self.render_event_filter_popup(overlay_area.unwrap(), buf);
                }
                Overlay::MarkName => {
                    self.render_mark_name_input_popup(overlay_area.unwrap(), buf);
                }
                Overlay::SaveToFile => {
                    self.render_save_to_file_popup(overlay_area.unwrap(), buf);
                }
                Overlay::AddCustomEvent => {
                    self.render_add_custom_event_popup(overlay_area.unwrap(), buf);
                }
                Overlay::EditTimeFilter => {}
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
