use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListState, StatefulWidget, Widget};

#[derive(Debug, Default)]
pub struct Help {
    selected_index: usize,
    help_items: Vec<HelpItem>,
    visible: bool,
}

#[derive(Debug, Default, PartialEq)]
pub enum HelpItemType {
    #[default]
    Keybind,
    Header,
    Empty,
}

#[derive(Debug, Default)]
pub struct HelpItem {
    pub key: String,
    pub description: String,
    pub item_type: HelpItemType,
}

impl HelpItem {
    pub fn new(key: &str, description: &str, item_type: HelpItemType) -> Self {
        Self {
            key: key.to_string(),
            description: description.to_string(),
            item_type,
        }
    }

    pub fn is_selectable(&self) -> bool {
        self.item_type == HelpItemType::Keybind
    }

    pub fn is_header(&self) -> bool {
        self.item_type == HelpItemType::Header
    }
}

impl Help {
    pub fn new() -> Self {
        let help_items = vec![
            // LogView Mode section (no empty line above first header)
            HelpItem::new("LogView Mode", "", HelpItemType::Header),
            HelpItem::new("q, Ctrl+C", "Quit", HelpItemType::Keybind),
            HelpItem::new("h", "Toggle help", HelpItemType::Keybind),
            HelpItem::new("Down/Up", "Navigate lines", HelpItemType::Keybind),
            HelpItem::new("g/G", "Go to start/end", HelpItemType::Keybind),
            HelpItem::new("PageUp/Down", "Page up/down", HelpItemType::Keybind),
            HelpItem::new("z", "Center selected line", HelpItemType::Keybind),
            HelpItem::new("Left/Right", "Scroll horizontally", HelpItemType::Keybind),
            HelpItem::new("0", "Reset horizontal scroll", HelpItemType::Keybind),
            HelpItem::new("/,Ctrl+F", "Start search", HelpItemType::Keybind),
            HelpItem::new("n/N", "Next/previous match", HelpItemType::Keybind),
            HelpItem::new(":", "Go to line", HelpItemType::Keybind),
            HelpItem::new("f", "Start filter", HelpItemType::Keybind),
            HelpItem::new("F", "View filter list", HelpItemType::Keybind),
            // Search Mode section
            HelpItem::new("", "", HelpItemType::Empty),
            HelpItem::new("Search Mode", "", HelpItemType::Header),
            HelpItem::new("Tab", "Toggle case sensitivity", HelpItemType::Keybind),
            HelpItem::new("Up/Down", "Navigate search history", HelpItemType::Keybind),
            // Filter Mode section
            HelpItem::new("", "", HelpItemType::Empty),
            HelpItem::new("Filter Mode", "", HelpItemType::Header),
            HelpItem::new("Tab", "Toggle case sensitivity", HelpItemType::Keybind),
            HelpItem::new(
                "Left/Right",
                "Toggle include/exclude",
                HelpItemType::Keybind,
            ),
            HelpItem::new("Enter", "Apply filter", HelpItemType::Keybind),
            HelpItem::new("Delete", "Remove filter pattern", HelpItemType::Keybind),
            // Filter List section
            HelpItem::new("", "", HelpItemType::Empty),
            HelpItem::new("Filter List", "", HelpItemType::Header),
            HelpItem::new("Up/Down", "Navigate filters", HelpItemType::Keybind),
            HelpItem::new("Space", "Toggle filter on/off", HelpItemType::Keybind),
            HelpItem::new("Delete", "Remove selected filter", HelpItemType::Keybind),
        ];

        Self {
            selected_index: 0,
            help_items,
            visible: false,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        self.reset();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    fn find_next_selectable(&self, start: usize, direction: i32) -> Option<usize> {
        let len = self.help_items.len();
        let mut current = start as i32;

        loop {
            current += direction;

            if current < 0 || current >= len as i32 {
                return None;
            }

            let index = current as usize;
            if self.help_items[index].is_selectable() {
                return Some(index);
            }
        }
    }

    pub fn move_up(&mut self) {
        if let Some(new_index) = self.find_next_selectable(self.selected_index, -1) {
            self.selected_index = new_index;
        }
    }

    pub fn move_down(&mut self) {
        if let Some(new_index) = self.find_next_selectable(self.selected_index, 1) {
            self.selected_index = new_index;
        }
    }

    pub fn reset(&mut self) {
        for i in 0..self.help_items.len() {
            if self.help_items[i].is_selectable() {
                self.selected_index = i;
                return;
            }
        }
        self.selected_index = 0;
    }

    pub fn get_display_lines(&self) -> Vec<Line<'static>> {
        self.help_items
            .iter()
            .map(|item| match item.item_type {
                HelpItemType::Header => Line::from(item.key.clone()).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                HelpItemType::Empty => Line::from(""),
                HelpItemType::Keybind => {
                    let formatted = format!("{:<15} {}", item.key, item.description);
                    Line::from(formatted)
                }
            })
            .collect()
    }

    pub fn render(&self, popup_area: Rect, buf: &mut Buffer) {
        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        let help_list = List::new(self.get_display_lines())
            .block(block)
            .highlight_symbol("")
            .highlight_style(Style::default().bg(Color::LightBlue));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        StatefulWidget::render(help_list, popup_area, buf, &mut list_state);
    }
}
