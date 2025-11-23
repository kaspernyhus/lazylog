use std::cell::Cell;

/// Viewport management for any list view.
/// Handles selection, scrolling, and viewport tracking.
#[derive(Debug, Default)]
pub struct ListViewState {
    /// Currently selected item index.
    selected_index: usize,
    /// Viewport offset for scrolling the list.
    viewport_offset: usize,
    /// Total number of items in the list.
    item_count: usize,
    /// Last rendered viewport height. Set in UI rendering, needs interior mutability.
    viewport_height: Cell<usize>,
}

impl ListViewState {
    /// Creates a new list view state with selection at index 0.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Gets the current viewport offset.
    pub fn viewport_offset(&self) -> usize {
        self.viewport_offset
    }

    /// Sets the viewport height (called from UI rendering).
    pub fn set_viewport_height(&self, height: usize) {
        self.viewport_height.set(height);
    }

    /// Gets the total item count.
    pub fn item_count(&self) -> usize {
        self.item_count
    }

    /// Sets the total item count.
    pub fn set_item_count(&mut self, count: usize) {
        self.item_count = count;
        if count > 0 && self.selected_index >= count {
            self.selected_index = count - 1;
        } else if count == 0 {
            self.selected_index = 0;
        }
        self.adjust_viewport();
    }

    /// Adjusts the viewport offset to keep the selected item visible.
    fn adjust_viewport(&mut self) {
        if self.item_count == 0 {
            self.viewport_offset = 0;
            return;
        }

        let viewport_height = self.viewport_height.get();
        if viewport_height == 0 {
            return;
        }

        // Scroll up if selection moved above viewport
        if self.selected_index < self.viewport_offset {
            self.viewport_offset = self.selected_index;
        }

        // Scroll down if selection moved below viewport
        let bottom_threshold = self.viewport_offset + viewport_height.saturating_sub(1);
        if self.selected_index > bottom_threshold {
            self.viewport_offset = self.selected_index + 1 - viewport_height;
        }

        // Ensure viewport doesn't go past the end
        let max_offset = self.item_count.saturating_sub(viewport_height);
        self.viewport_offset = self.viewport_offset.min(max_offset);
    }

    /// Moves selection up by 1 without wrapping.
    pub fn move_up(&mut self) {
        if self.item_count > 0 && self.selected_index > 0 {
            self.selected_index -= 1;
            self.adjust_viewport();
        }
    }

    /// Moves selection down by 1 without wrapping.
    pub fn move_down(&mut self) {
        if self.item_count > 0 && self.selected_index < self.item_count - 1 {
            self.selected_index += 1;
            self.adjust_viewport();
        }
    }

    /// Moves selection up by 1 with wrapping.
    pub fn move_up_wrap(&mut self) {
        if self.item_count > 0 {
            self.selected_index = if self.selected_index == 0 {
                self.item_count - 1
            } else {
                self.selected_index - 1
            };
            self.adjust_viewport();
        }
    }

    /// Moves selection down by 1 with wrapping.
    pub fn move_down_wrap(&mut self) {
        if self.item_count > 0 {
            self.selected_index = (self.selected_index + 1) % self.item_count;
            self.adjust_viewport();
        }
    }

    /// Moves selection up by half a page.
    pub fn page_up(&mut self) {
        if self.item_count > 0 {
            let page_size = self.viewport_height.get().saturating_sub(1).max(1) / 2;
            self.selected_index = self.selected_index.saturating_sub(page_size);
            self.adjust_viewport();
        }
    }

    /// Moves selection down by half a page.
    pub fn page_down(&mut self) {
        if self.item_count > 0 {
            let page_size = self.viewport_height.get().saturating_sub(1).max(1) / 2;
            self.selected_index = (self.selected_index + page_size).min(self.item_count - 1);
            self.adjust_viewport();
        }
    }

    /// Selects the last (most recent) item in the list.
    pub fn select_last(&mut self) {
        if self.item_count > 0 {
            self.selected_index = self.item_count - 1;
            self.adjust_viewport();
        }
    }

    /// Selects the first item in the list.
    pub fn select_first(&mut self) {
        if self.item_count > 0 {
            self.selected_index = 0;
            self.adjust_viewport();
        }
    }

    /// Selects a specific index (clamped to valid range).
    pub fn select_index(&mut self, index: usize) {
        if self.item_count > 0 {
            self.selected_index = index.min(self.item_count - 1);
            self.adjust_viewport();
        }
    }

    /// Resets to initial state (selection at 0).
    pub fn reset(&mut self) {
        self.selected_index = 0;
        self.viewport_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state_starts_at_zero() {
        let state = ListViewState::new();
        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.viewport_offset(), 0);
    }

    #[test]
    fn test_reset() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(20);
        state.select_index(10);

        state.reset();
        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.viewport_offset(), 0);
    }

    #[test]
    fn test_move_down() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);

        state.move_down();
        assert_eq!(state.selected_index(), 1);

        state.move_down();
        assert_eq!(state.selected_index(), 2);
    }

    #[test]
    fn test_move_down_at_end_does_nothing() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);
        state.select_index(4); // Last item of 5

        state.move_down();
        assert_eq!(state.selected_index(), 4); // Still at end
    }

    #[test]
    fn test_move_up() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);
        state.select_index(3);

        state.move_up();
        assert_eq!(state.selected_index(), 2);

        state.move_up();
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn test_move_up_at_start_does_nothing() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);

        state.move_up();
        assert_eq!(state.selected_index(), 0); // Still at start
    }

    #[test]
    fn test_move_down_wrap() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);
        state.select_index(4); // Last item

        state.move_down_wrap();
        assert_eq!(state.selected_index(), 0); // Wrapped to start
    }

    #[test]
    fn test_move_up_wrap() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(5);

        state.move_up_wrap();
        assert_eq!(state.selected_index(), 4); // Wrapped to end
    }

    #[test]
    fn test_page_down() {
        let mut state = ListViewState::new();
        state.set_viewport_height(20); // Half page = 9
        state.set_item_count(50);

        state.page_down();
        assert_eq!(state.selected_index(), 9);

        state.page_down();
        assert_eq!(state.selected_index(), 18);
    }

    #[test]
    fn test_page_down_near_end() {
        let mut state = ListViewState::new();
        state.set_viewport_height(20);
        state.set_item_count(20);
        state.select_index(15);

        state.page_down();
        assert_eq!(state.selected_index(), 19); // Clamped to last
    }

    #[test]
    fn test_page_up() {
        let mut state = ListViewState::new();
        state.set_viewport_height(20);
        state.set_item_count(50);
        state.select_index(20);

        state.page_up();
        assert_eq!(state.selected_index(), 11); // 20 - 9 = 11
    }

    #[test]
    fn test_page_up_near_start() {
        let mut state = ListViewState::new();
        state.set_viewport_height(20);
        state.set_item_count(50);
        state.select_index(5);

        state.page_up();
        assert_eq!(state.selected_index(), 0); // Clamped to start
    }

    #[test]
    fn test_select_last() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(20);

        state.select_last();
        assert_eq!(state.selected_index(), 19);
    }

    #[test]
    fn test_select_first() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(20);
        state.select_index(10);

        state.select_first();
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn test_select_index_clamps_to_range() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(20);

        state.select_index(100);
        assert_eq!(state.selected_index(), 19); // Clamped to max
    }

    #[test]
    fn test_viewport_scrolls_down_with_selection() {
        let mut state = ListViewState::new();
        state.set_viewport_height(5); // Small viewport
        state.set_item_count(20);

        // Move down past viewport
        for _ in 0..10 {
            state.move_down();
        }

        // Viewport should have scrolled to keep selection visible
        assert_eq!(state.selected_index(), 10);
        assert!(state.viewport_offset() > 0);
        assert!(state.selected_index() >= state.viewport_offset());
        assert!(state.selected_index() < state.viewport_offset() + 5);
    }

    #[test]
    fn test_viewport_scrolls_up_with_selection() {
        let mut state = ListViewState::new();
        state.set_viewport_height(5);
        state.set_item_count(20);
        state.select_index(10);

        // Move up
        for _ in 0..5 {
            state.move_up();
        }

        // Viewport should have scrolled up
        assert_eq!(state.selected_index(), 5);
        assert!(state.selected_index() >= state.viewport_offset());
        assert!(state.selected_index() < state.viewport_offset() + 5);
    }

    #[test]
    fn test_operations_on_empty_list_do_nothing() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);
        state.set_item_count(0);

        state.move_down();

        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.viewport_offset(), 0);

        state.move_up();

        assert_eq!(state.selected_index(), 0);
        assert_eq!(state.viewport_offset(), 0);
    }

    #[test]
    fn test_move_up_clamps_when_selected_beyond_range() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);

        // Simulate having 10 items, selecting the last one (index 9)
        state.set_item_count(10);
        state.select_index(9);
        assert_eq!(state.selected_index(), 9);

        // Reduces items to 5
        // set_item_count automatically clamps selection to 4 (last valid index)
        state.set_item_count(5);
        assert_eq!(state.selected_index(), 4); // Automatically clamped

        // move_up should move from 4 to 3
        state.move_up();
        assert_eq!(state.selected_index(), 3);
    }

    #[test]
    fn test_move_down_clamps_when_selected_beyond_range() {
        let mut state = ListViewState::new();
        state.set_viewport_height(10);

        // Simulate having 10 items, selecting item at index 7
        state.set_item_count(10);
        state.select_index(7);
        assert_eq!(state.selected_index(), 7);

        // Reduces items to 5
        // Selection is at index 7, which is beyond the new range
        // move_down should clamp to 4 (last valid index) and stay there
        state.set_item_count(5);
        state.move_down();
        assert_eq!(state.selected_index(), 4); // Should clamp to valid range

        // Another move_down should stay at 4 (end of list)
        state.move_down();
        assert_eq!(state.selected_index(), 4);
    }
}
