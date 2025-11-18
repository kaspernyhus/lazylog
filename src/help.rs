use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState,
    StatefulWidget, Widget,
};
use std::cell::Cell;

use crate::app::AppState;
use crate::colors::{HELP_BG, HELP_HEADER_FG, HELP_HIGHLIGHT_FG};
use crate::command::Command;
use crate::keybindings::KeybindingRegistry;

/// Manages the help popup display with keybindings and navigation.
#[derive(Debug, Default)]
pub struct Help {
    selected_index: usize,
    help_items: Vec<HelpItem>,
    visible: bool,
    /// Viewport offset for scrolling the list
    viewport_offset: usize,
    /// Last rendered viewport height, set in ui rendering, therefore need interior mutability.
    viewport_height: Cell<usize>,
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
    /// Optional AppState this header represents (only for headers)
    pub state: Option<AppState>,
}

impl HelpItem {
    /// Creates a new help item.
    pub fn new(key: &str, description: &str, item_type: HelpItemType) -> Self {
        Self {
            key: key.to_string(),
            description: description.to_string(),
            item_type,
            state: None,
        }
    }

    /// Creates a new header help item.
    pub fn new_header(key: &str, state: Option<AppState>) -> Self {
        Self {
            key: key.to_string(),
            description: String::new(),
            item_type: HelpItemType::Header,
            state,
        }
    }

    /// Creates a new empty help item (an empty line).
    pub fn new_empty() -> Self {
        Self {
            key: String::new(),
            description: String::new(),
            item_type: HelpItemType::Empty,
            state: None,
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
        Self::default()
    }

    /// Builds the help items from the keybinding registry.
    pub fn build_from_registry(&mut self, registry: &KeybindingRegistry) {
        use crate::app::AppState;

        let mut help_items = vec![
            // Global bindings section
            HelpItem::new_header("Global", None),
            HelpItem::new("Ctrl+c/q", "Quit", HelpItemType::Keybind),
            HelpItem::new("Esc", "Cancel/Exit mode", HelpItemType::Keybind),
            HelpItem::new("Enter", "Confirm", HelpItemType::Keybind),
            HelpItem::new("Ctrl+l", "Clear buffer (stdin)", HelpItemType::Keybind),
            HelpItem::new("Ctrl+s", "Save to file (stdin)", HelpItemType::Keybind),
            HelpItem::new("h", "Show help", HelpItemType::Keybind),
        ];

        // LogView section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header("LogView", None));
        self.add_state_bindings(&mut help_items, registry, &AppState::LogView);
        help_items.push(HelpItem::new(
            "y",
            Command::CopySelection.description(),
            HelpItemType::Keybind,
        ));

        // Search Mode section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header("Search", Some(AppState::SearchMode)));
        self.add_state_bindings(&mut help_items, registry, &AppState::SearchMode);

        // Filter Mode section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header("Filter", Some(AppState::FilterMode)));
        self.add_state_bindings(&mut help_items, registry, &AppState::FilterMode);

        // Filter List section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header(
            "Filter List",
            Some(AppState::FilterListView),
        ));
        self.add_state_bindings(&mut help_items, registry, &AppState::FilterListView);

        // Events View section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header(
            "Events View",
            Some(AppState::EventsView),
        ));
        self.add_state_bindings(&mut help_items, registry, &AppState::EventsView);

        // Event Filters section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header(
            "Event Filters",
            Some(AppState::EventsFilterView),
        ));
        self.add_state_bindings(&mut help_items, registry, &AppState::EventsFilterView);

        // Display Options section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header(
            "Display Options",
            Some(AppState::OptionsView),
        ));
        self.add_state_bindings(&mut help_items, registry, &AppState::OptionsView);

        // Marks View section
        help_items.push(HelpItem::new_empty());
        help_items.push(HelpItem::new_header(
            "Marks View",
            Some(AppState::MarksView),
        ));
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
            // Skip commands that should not be shown in help
            if matches!(
                command,
                Command::Quit
                    | Command::Confirm
                    | Command::Cancel
                    | Command::ToggleHelp
                    | Command::PageUp
                    | Command::PageDown
                    | Command::MoveUp
                    | Command::MoveDown
            ) {
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

    /// Shows help and jumps to the section for the given AppState.
    pub fn show_for_state(&mut self, state: &AppState) {
        self.visible = true;
        self.jump_to_state(state);
    }

    /// Jumps to the section header for the given AppState.
    fn jump_to_state(&mut self, target_state: &AppState) {
        for (index, item) in self.help_items.iter().enumerate() {
            if item.item_type == HelpItemType::Header
                && let Some(ref item_state) = item.state
                    && item_state.matches(target_state) {
                        self.selected_index = index;
                        let viewport_height = self.viewport_height.get();
                        let max_offset = self.help_items.len().saturating_sub(viewport_height);
                        self.viewport_offset = index.min(max_offset);
                        return;
                    }
        }
        // if not found
        self.reset();
    }

    /// Returns whether the help popup is currently visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Gets the current viewport offset.
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Sets the viewport height (should be called when rendering the popup).
    pub fn set_viewport_height(&self, height: usize) {
        self.viewport_height.set(height);
    }

    /// Adjusts the viewport offset to keep the selected item visible.
    fn adjust_viewport(&mut self) {
        if self.help_items.is_empty() {
            self.viewport_offset = 0;
            return;
        }

        let viewport_height = self.viewport_height.get();

        // Scroll up
        if self.selected_index < self.viewport_offset {
            self.viewport_offset = self.selected_index;
        }

        // Scroll down
        let bottom_threshold = self.viewport_offset + viewport_height.saturating_sub(1);
        if self.selected_index > bottom_threshold {
            self.viewport_offset = self.selected_index + 1 - viewport_height;
        }

        // Ensure viewport doesn't go past the end
        let max_offset = self.help_items.len().saturating_sub(viewport_height);
        self.viewport_offset = self.viewport_offset.min(max_offset);
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
            self.adjust_viewport();
        }
    }

    /// Moves selection to the next selectable item.
    pub fn move_down(&mut self) {
        if let Some(new_index) = self.find_next_selectable(self.selected_index, 1) {
            self.selected_index = new_index;
            self.adjust_viewport();
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
                        .fg(HELP_HEADER_FG)
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
            .style(Style::default().bg(HELP_BG));

        let inner_area = block.inner(popup_area);

        let [help_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        self.set_viewport_height(help_area.height as usize);

        block.render(popup_area, buf);

        let help_list = List::new(self.get_display_lines())
            .highlight_symbol("")
            .highlight_style(Style::default().bg(HELP_HIGHLIGHT_FG));

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));
        *list_state.offset_mut() = self.viewport_offset;

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
