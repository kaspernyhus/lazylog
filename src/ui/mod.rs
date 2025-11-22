mod footer;
mod lists;
mod logview;
mod popups;
mod scrollable_list;

use crate::app::{App, AppState};
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
        match self.app_state {
            AppState::SearchMode => self.render_search_footer(bottom, buf),
            AppState::MarkAddInputMode => self.render_mark_footer(bottom, buf),
            AppState::GotoLineMode => self.render_goto_line_footer(bottom, buf),
            AppState::FilterMode => self.render_filter_footer(bottom, buf),
            AppState::SelectionMode => self.render_selection_footer(bottom, buf),
            _ => self.render_default_footer(bottom, buf),
        }

        // Popups
        if self.app_state == AppState::FilterListView {
            let filter_area = popup_area(area, 118, 35);
            self.render_filter_list(filter_area, buf);
        }
        if self.app_state == AppState::EditFilterMode {
            let filter_area = popup_area(area, 118, 35);
            self.render_filter_list(filter_area, buf);
            let edit_area = popup_area(area, 60, 3);
            self.render_edit_filter_popup(edit_area, buf);
        }
        if self.app_state == AppState::OptionsView {
            let options_area = popup_area(area, 42, 10);
            self.render_options(options_area, buf);
        }
        if self.app_state == AppState::EventsView {
            let events_area = popup_area(area, 118, 35);
            self.render_events_list(events_area, buf);
        }
        if self.app_state == AppState::EventsFilterView {
            let events_area = popup_area(area, 118, 35);
            let event_filter_area = popup_area(area, 40, 15);
            self.render_events_list(events_area, buf);
            self.render_event_filter_popup(event_filter_area, buf);
        }
        if self.app_state == AppState::MarksView || self.app_state == AppState::MarkAddInputMode {
            let marks_area = popup_area(area, 118, 35);
            self.render_marks_list(marks_area, buf);
        }
        if self.app_state == AppState::MarkNameInputMode {
            let marks_area = popup_area(area, 118, 35);
            self.render_marks_list(marks_area, buf);
            let name_input_area = popup_area(area, 60, 3);
            self.render_mark_name_input_popup(name_input_area, buf);
        }
        if self.app_state == AppState::SaveToFileMode {
            let save_area = popup_area(area, 60, 3);
            self.render_save_to_file_popup(save_area, buf);
        }
        if self.help.is_visible() {
            let help_area = popup_area(area, 50, 32);
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
