use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, List, ListState, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

/// Helper for rendering a scrollable list with a scrollbar.
pub struct ScrollableList<'a> {
    items: Vec<Line<'a>>,
    selected_index: Option<usize>,
    viewport_offset: usize,
    total_count: usize,
    highlight_symbol: &'a str,
    highlight_style: Style,
}

impl<'a> ScrollableList<'a> {
    /// Creates a new scrollable list.
    pub fn new(items: Vec<Line<'a>>) -> Self {
        Self {
            items,
            selected_index: None,
            viewport_offset: 0,
            total_count: 0,
            highlight_symbol: "",
            highlight_style: Style::default(),
        }
    }

    /// Sets the selected index and viewport offset.
    pub fn selection(mut self, selected_index: usize, viewport_offset: usize) -> Self {
        self.selected_index = Some(selected_index);
        self.viewport_offset = viewport_offset;
        self
    }

    /// Sets the total count for the scrollbar (defaults to items.len()).
    pub fn total_count(mut self, count: usize) -> Self {
        self.total_count = count;
        self
    }

    /// Sets the highlight symbol.
    pub fn highlight_symbol(mut self, symbol: &'a str) -> Self {
        self.highlight_symbol = symbol;
        self
    }

    /// Sets the highlight style.
    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    /// Renders the list with a scrollbar into the given area.
    /// Returns the list_area and scrollbar_area for custom rendering if needed.
    pub fn render(self, area: Rect, buf: &mut Buffer, block: Block<'a>) -> (Rect, Rect) {
        let inner_area = block.inner(area);

        let [list_area, scrollbar_area] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(1)]).areas(inner_area);

        block.render(area, buf);

        let items_len = self.items.len();
        let list = List::new(self.items)
            .highlight_symbol(self.highlight_symbol)
            .highlight_style(self.highlight_style);

        let mut list_state = ListState::default();
        if let Some(selected_index) = self.selected_index {
            list_state.select(Some(selected_index));
            *list_state.offset_mut() = self.viewport_offset;
        }

        StatefulWidget::render(list, list_area, buf, &mut list_state);

        let total_count = if self.total_count > 0 {
            self.total_count
        } else {
            items_len
        };

        let mut scrollbar_state = ScrollbarState::new(total_count)
            .position(self.selected_index.unwrap_or(0))
            .viewport_content_length(0);

        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None);

        StatefulWidget::render(scrollbar, scrollbar_area, buf, &mut scrollbar_state);

        (list_area, scrollbar_area)
    }
}
