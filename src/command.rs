use crate::app::App;
use color_eyre::Result;

/// Represents actions that can be performed in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    // Navigation
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    GotoTop,
    GotoBottom,
    CenterSelected,
    ScrollLeft,
    ScrollRight,
    ResetHorizontal,
    HistoryBack,
    HistoryForward,

    // Application Control
    Quit,
    ToggleHelp,
    ClearLogBuffer,
    Cancel,
    Confirm,

    // Search
    ActivateSearchMode,
    SearchNext,
    SearchPrevious,
    ToggleCaseSearch,
    SearchHistoryPrevious,
    SearchHistoryNext,

    // Filter
    ActivateFilterMode,
    ActivateFilterListView,
    ActivateEditFilterMode,
    ToggleFilterPattern,
    RemoveFilterPattern,
    ToggleAllFilterPatterns,
    ToggleFilterPatternCaseSensitive,
    ToggleFilterPatternMode,
    ToggleCaseFilter,
    ToggleFilterModeInOut,
    FilterHistoryPrevious,
    FilterHistoryNext,

    // Goto Line
    ActivateGotoLineMode,

    // Display Options
    ActivateOptionsView,
    ToggleDisplayOption,

    // Events
    ActivateEventsView,
    ActivateEventFilterView,
    GotoSelectedEvent,
    ToggleEventFilter,
    ToggleAllEventFilters,

    // Marks
    ToggleMark,
    ActivateMarksView,
    GotoSelectedMark,
    ActivateMarkNameInputMode,
    ActivateMarkAddInputMode,
    UnmarkSelected,
    ClearAllMarks,
    MarkNext,
    MarkPrevious,

    // Streaming
    ToggleFollowMode,
    TogglePauseMode,
    ToggleCenterCursorMode,
    ActivateSaveToFileMode,

    // Selection
    StartSelection,
    CopySelection,
}

impl Command {
    /// Returns a human-readable description of this command.
    pub fn description(&self) -> &'static str {
        match self {
            // Navigation
            Command::MoveUp => "Move up",
            Command::MoveDown => "Move down",
            Command::PageUp => "Page up",
            Command::PageDown => "Page down",
            Command::GotoTop => "Go to start",
            Command::GotoBottom => "Go to end",
            Command::CenterSelected => "Center selected line",
            Command::ScrollLeft => "Scroll left",
            Command::ScrollRight => "Scroll right",
            Command::ResetHorizontal => "Reset horizontal scroll",
            Command::HistoryBack => "Go back in history",
            Command::HistoryForward => "Go forward in history",

            // Application Control
            Command::Quit => "Quit",
            Command::ToggleHelp => "Toggle help",
            Command::ClearLogBuffer => "Clear buffer (stdin)",
            Command::Cancel => "Cancel/Exit mode",
            Command::Confirm => "Confirm",

            // Search
            Command::ActivateSearchMode => "Start search",
            Command::SearchNext => "Next match",
            Command::SearchPrevious => "Previous match",
            Command::ToggleCaseSearch => "Toggle case sensitivity",
            Command::SearchHistoryPrevious => "Previous search from history",
            Command::SearchHistoryNext => "Next search from history",

            // Filter
            Command::ActivateFilterMode => "Start filter",
            Command::ActivateFilterListView => "View filter list",
            Command::ActivateEditFilterMode => "Edit selected filter",
            Command::ToggleFilterPattern => "Toggle filter on/off",
            Command::RemoveFilterPattern => "Remove selected filter",
            Command::ToggleAllFilterPatterns => "Toggle all filters",
            Command::ToggleFilterPatternCaseSensitive => "Toggle case sensitive",
            Command::ToggleFilterPatternMode => "Toggle include/exclude",
            Command::ToggleCaseFilter => "Toggle case sensitivity",
            Command::ToggleFilterModeInOut => "Toggle include/exclude",
            Command::FilterHistoryPrevious => "Previous filter from history",
            Command::FilterHistoryNext => "Next filter from history",

            // Goto Line
            Command::ActivateGotoLineMode => "Go to line",

            // Display Options
            Command::ActivateOptionsView => "Display options",
            Command::ToggleDisplayOption => "Toggle option on/off",

            // Events
            Command::ActivateEventsView => "View log events",
            Command::ActivateEventFilterView => "Filter events",
            Command::GotoSelectedEvent => "Go to selected event",
            Command::ToggleEventFilter => "Toggle event filter",
            Command::ToggleAllEventFilters => "Toggle all event filters",

            // Marks
            Command::ToggleMark => "Toggle mark on line",
            Command::ActivateMarksView => "View marked lines",
            Command::GotoSelectedMark => "Go to selected mark",
            Command::ActivateMarkNameInputMode => "Name the mark",
            Command::ActivateMarkAddInputMode => "Add mark(s) from a pattern",
            Command::UnmarkSelected => "Remove selected mark",
            Command::ClearAllMarks => "Clear all marks",
            Command::MarkNext => "Go to next mark",
            Command::MarkPrevious => "Go to previous mark",

            // Streaming
            Command::ToggleFollowMode => "Toggle follow mode (stdin)",
            Command::TogglePauseMode => "Toggle pause mode (stdin)",
            Command::ToggleCenterCursorMode => "Toggle center cursor mode",
            Command::ActivateSaveToFileMode => "Save to file (stdin)",

            // Selection
            Command::StartSelection => "Start visual selection",
            Command::CopySelection => "Copy selection to clipboard",
        }
    }

    /// Executes this command on the given application.
    pub fn execute(&self, app: &mut App) -> Result<()> {
        match self {
            // Navigation
            Command::MoveUp => app.move_up(),
            Command::MoveDown => app.move_down(),
            Command::PageUp => app.page_up(),
            Command::PageDown => app.page_down(),
            Command::GotoTop => app.goto_top(),
            Command::GotoBottom => app.goto_bottom(),
            Command::CenterSelected => app.viewport.center_selected(),
            Command::ScrollLeft => app.viewport.scroll_left(),
            Command::ScrollRight => app.scroll_right(),
            Command::ResetHorizontal => app.viewport.reset_horizontal(),
            Command::HistoryBack => app.history_back(),
            Command::HistoryForward => app.history_forward(),

            // Application Control
            Command::Quit => app.quit(),
            Command::ToggleHelp => app.toggle_help(),
            Command::ClearLogBuffer => app.clear_log_buffer(),
            Command::Cancel => app.cancel(),
            Command::Confirm => app.confirm(),

            // Search
            Command::ActivateSearchMode => app.activate_search_mode(),
            Command::SearchNext => app.search_next(),
            Command::SearchPrevious => app.search_previous(),
            Command::ToggleCaseSearch => app.toggle_case_sensitive(),
            Command::SearchHistoryPrevious => app.search_history_previous(),
            Command::SearchHistoryNext => app.search_history_next(),

            // Filter
            Command::ActivateFilterMode => app.activate_filter_mode(),
            Command::ActivateFilterListView => app.activate_filter_list_view(),
            Command::ActivateEditFilterMode => app.activate_edit_filter_mode(),
            Command::ToggleFilterPattern => app.toggle_filter_pattern_active(),
            Command::RemoveFilterPattern => app.remove_filter_pattern(),
            Command::ToggleAllFilterPatterns => app.toggle_all_filter_patterns(),
            Command::ToggleFilterPatternCaseSensitive => app.toggle_filter_pattern_case_sensitive(),
            Command::ToggleFilterPatternMode => app.toggle_filter_pattern_mode(),
            Command::ToggleCaseFilter => app.toggle_case_sensitive(),
            Command::ToggleFilterModeInOut => app.filter.toggle_mode(),
            Command::FilterHistoryPrevious => app.filter_history_previous(),
            Command::FilterHistoryNext => app.filter_history_next(),

            // Goto Line
            Command::ActivateGotoLineMode => app.activate_goto_line_mode(),

            // Display Options
            Command::ActivateOptionsView => app.activate_options_view(),
            Command::ToggleDisplayOption => app.options.toggle_selected_option(),

            // Events
            Command::ActivateEventsView => app.activate_events_view(),
            Command::ActivateEventFilterView => app.activate_event_filter_view(),
            Command::GotoSelectedEvent => app.goto_selected_event(),
            Command::ToggleEventFilter => app.toggle_event_filter(),
            Command::ToggleAllEventFilters => app.toggle_all_event_filters(),

            // Marks
            Command::ToggleMark => app.toggle_mark(),
            Command::ActivateMarksView => app.activate_marks_view(),
            Command::GotoSelectedMark => app.goto_selected_mark(),
            Command::ActivateMarkNameInputMode => app.activate_mark_name_input_mode(),
            Command::ActivateMarkAddInputMode => app.activate_mark_add_input_mode(),
            Command::UnmarkSelected => app.unmark_selected(),
            Command::ClearAllMarks => app.marking.clear_all(),
            Command::MarkNext => app.mark_next(),
            Command::MarkPrevious => app.mark_previous(),

            // Streaming
            Command::ToggleFollowMode => app.toggle_follow_mode(),
            Command::TogglePauseMode => app.toggle_pause_mode(),
            Command::ToggleCenterCursorMode => app.toggle_center_cursor_mode(),
            Command::ActivateSaveToFileMode => app.activate_save_to_file_mode(),

            // Selection
            Command::StartSelection => app.start_selection(),
            Command::CopySelection => app.copy_selection_to_clipboard(),
        }
        Ok(())
    }
}
