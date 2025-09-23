use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget};

pub fn render_help_popup(popup_area: Rect, buf: &mut Buffer) {
    Clear.render(popup_area, buf);

    let help_text = vec![
        Line::from("LogView").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from("q            Quit"),
        Line::from("Down/Up      Navigate"),
        Line::from("g/G          Go to start/end"),
        Line::from("PageUp/Down  Page up/down"),
        Line::from("z            Center selected line"),
        Line::from("Left/Right   Scroll horizontally"),
        Line::from("0            Reset horizontal scroll"),
        Line::from("/            Search"),
        Line::from(":            Go to line"),
        Line::from("Search").style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Line::from("Left/Right   Toggle case sensitive"),
    ];

    let block = Block::default()
        .title(" Help ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .style(Style::default().bg(Color::Blue));

    let help_popup = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: true });

    help_popup.render(popup_area, buf);
}
