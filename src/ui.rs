use ratatui::widgets::ListState;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, List, StatefulWidget, Widget},
};

use crate::app::App;

pub const RIGHT_ARROW: &str = "▶ ";

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [top, middle, bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        let title = Block::default()
            .title(" Lazylog ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Indexed(237)));

        let footer = Block::default()
            .title(" Press 'q' to quit ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(Color::Indexed(237)));

        let items = [""];

        let mut list_state = ListState::default();
        list_state.select_first();

        let log_list = List::new(items)
            .highlight_symbol(RIGHT_ARROW)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD));

        title.render(top, buf);
        StatefulWidget::render(log_list, middle, buf, &mut list_state);
        footer.render(bottom, buf);
    }
}
