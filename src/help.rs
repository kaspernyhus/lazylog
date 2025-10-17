use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};

use crate::app::AppState;
use crate::command::Command;
use crate::keybindings::KeybindingRegistry;

/// Manages the help popup display with keybindings and navigation.
#[derive(Debug, Default)]
pub struct Help {
    selected_index: usize,
    help_items: Vec<HelpItem>,
    visible: bool,
}

/// Type of help item.
#[derive(Debug, Default, PartialEq)]
pub enum HelpItemType {
    /// A keybinding with key and description.
    #[default]
    Keybind,
    /// A section header.
    Header,
    /// An empty line for spacing.
    Empty,
}

/// A single help item entry.
#[derive(Debug, Default)]
pub struct HelpItem {
    /// The key or header text.
    pub key: String,
    /// Description of what the key does.
    pub description: String,
    /// Type of this help item.
    pub item_type: HelpItemType,
}

impl HelpItem {
    /// Creates a new help item.
    pub fn new(key: &str, description: &str, item_type: HelpItemType) -> Self {
        Self {
            key: key.to_string(),
            description: description.to_string(),
            item_type,
        }
    }

    /// Returns whether this item can be selected (only keybindings are selectable).
    pub fn is_selectable(&self) -> bool {
        self.item_type == HelpItemType::Keybind
    }

    /// Returns whether this item is a header.
    pub fn is_header(&self) -> bool {
        self.item_type == HelpItemType::Header
    }
}

impl Help {
    /// Creates a new Help instance with all keybinding documentation.
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            help_items: Vec::new(),
            visible: false,
        }
    }

    /// Builds the help items from the keybinding registry.
    pub fn build_from_registry(&mut self, registry: &KeybindingRegistry) {
        use crate::app::AppState;

        let mut help_items = vec![
            // Global bindings section
            HelpItem::new("Global", "", HelpItemType::Header),
            HelpItem::new("Ctrl+c/q", "Quit", HelpItemType::Keybind),
            HelpItem::new("esc", "Cancel/Exit mode", HelpItemType::Keybind),
            HelpItem::new("enter", "Confirm", HelpItemType::Keybind),
            HelpItem::new("Ctrl+l", "Clear buffer (stdin)", HelpItemType::Keybind),
            HelpItem::new("Ctrl+s", "Save to file (stdin)", HelpItemType::Keybind),
            HelpItem::new("h", "Show help", HelpItemType::Keybind),
        ];

        // LogView section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("LogView", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::LogView);

        // Search Mode section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Search", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::SearchMode);

        // Filter Mode section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Filter", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::FilterMode);

        // Filter List section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Filter List", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::FilterListView);

        // Events View section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Events View", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::EventsView);

        // Event Filters section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Event Filters", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::EventsFilterView);

        // Display Options section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Display Options", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::OptionsView);

        // Marks View section
        help_items.push(HelpItem::new("", "", HelpItemType::Empty));
        help_items.push(HelpItem::new("Marks View", "", HelpItemType::Header));
        self.add_state_bindings(&mut help_items, registry, &AppState::MarksView);

        self.help_items = help_items;
        self.reset();
    }

    /// Adds keybindings for a specific state to the help items.
    fn add_state_bindings(
        &self,
        help_items: &mut Vec<HelpItem>,
        registry: &KeybindingRegistry,
        state: &AppState,
    ) {
        let bindings = registry.get_keybindings_for_state(state);
        for (key, command) in bindings {
            if command == Command::Quit || command == Command::ToggleHelp {
                continue;
            }

            help_items.push(HelpItem::new(
                &key,
                command.description(),
                HelpItemType::Keybind,
            ));
        }
    }

    /// Toggles help visibility and resets selection.
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
        self.reset();
    }

    /// Returns whether the help popup is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Finds the next selectable item in the given direction.
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

    /// Moves selection to the previous selectable item.
    pub fn move_up(&mut self) {
        if let Some(new_index) = self.find_next_selectable(self.selected_index, -1) {
            self.selected_index = new_index;
        }
    }

    /// Moves selection to the next selectable item.
    pub fn move_down(&mut self) {
        if let Some(new_index) = self.find_next_selectable(self.selected_index, 1) {
            self.selected_index = new_index;
        }
    }

    /// Resets selection to the first selectable item.
    pub fn reset(&mut self) {
        for i in 0..self.help_items.len() {
            if self.help_items[i].is_selectable() {
                self.selected_index = i;
                return;
            }
        }
        self.selected_index = 0;
    }

    /// Returns formatted display lines for rendering.
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

    /// Renders the help popup to the buffer.
    pub fn render(&self, popup_area: Rect, buf: &mut Buffer) {
        Clear.render(popup_area, buf);

        let block = Block::default()
            .title(" Help ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Blue));

        let inner_area = block.inner(popup_area);

        let [help_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        block.render(popup_area, buf);

        let help_list = List::new(self.get_display_lines())
            .highlight_symbol("")
            .highlight_style(Style::default().bg(Color::LightBlue));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        StatefulWidget::render(help_list, help_area, buf, &mut list_state);

        let selectable_count = self
            .help_items
            .iter()
            .filter(|item| item.is_selectable())
            .count();
        let selectable_position = self.help_items[..=self.selected_index]
            .iter()
            .filter(|item| item.is_selectable())
            .count()
            - 1;

        let mut scrollbar_state = ScrollbarState::new(selectable_count)
            .position(selectable_position)
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);
    }
}
