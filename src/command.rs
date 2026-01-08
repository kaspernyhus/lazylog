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
    ScrollLeftSmall,
    ScrollRightSmall,
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
    ActivateActiveSearchMode,
    SearchNext,
    SearchPrevious,
    ToggleCaseSearch,
    SearchHistoryPrevious,
    SearchHistoryNext,
    TabCompletion,

    // Filter
    ActivateActiveFilterMode,
    ActivateFilterView,
    ActivateEditActiveFilterMode,
    ToggleFilterPattern,
    RemoveFilterPattern,
    ToggleAllFilterPatterns,
    ToggleFilterPatternCaseSensitive,
    ToggleFilterPatternMode,
    ToggleCaseFilter,
    ToggleActiveFilterModeInOut,
    FilterHistoryPrevious,
    FilterHistoryNext,

    // Goto Line
    ActivateGotoLineMode,

    // Display Options
    ActivateOptionsView,
    ToggleOption,

    // Events
    ActivateEventsView,
    ActivateEventFilterView,
    GotoSelectedEvent,
    ToggleEventFilter,
    ToggleAllEventFilters,
    ToggleEventsShowMarks,
    EventNext,
    EventPrevious,

    // Marks
    ToggleMark,
    ActivateMarksView,
    GotoSelectedMark,
    ActivateMarkNameMode,
    UnmarkSelected,
    ClearAllMarks,
    MarkNext,
    MarkPrevious,
    ToggleShowMarkedOnly,

    // Files
    ActivateFilesView,
    ToggleFile,

    // Expansion
    ToggleExpansion,
    CollapseAll,

    // Streaming
    ToggleFollowMode,
    TogglePauseMode,
    ToggleCenterCursorMode,
    ActivateSaveToFileMode,

    // Selection
    StartSelection,
    CopySelection,
    SelectToEventNext,
    SelectToEventPrevious,
    SelectToMarkNext,
    SelectToMarkPrevious,
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
            Command::ScrollLeftSmall => "Scroll left (small)",
            Command::ScrollRightSmall => "Scroll right (small)",
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
            Command::ActivateActiveSearchMode => "Start search",
            Command::SearchNext => "Next match",
            Command::SearchPrevious => "Previous match",
            Command::ToggleCaseSearch => "Toggle case sensitivity",
            Command::SearchHistoryPrevious => "Previous search from history",
            Command::SearchHistoryNext => "Next search from history",
            Command::TabCompletion => "Tab completion",

            // Filter
            Command::ActivateActiveFilterMode => "Start filter",
            Command::ActivateFilterView => "View filter list",
            Command::ActivateEditActiveFilterMode => "Edit selected filter",
            Command::ToggleFilterPattern => "Toggle filter on/off",
            Command::RemoveFilterPattern => "Remove selected filter",
            Command::ToggleAllFilterPatterns => "Toggle all filters",
            Command::ToggleFilterPatternCaseSensitive => "Toggle case sensitive",
            Command::ToggleFilterPatternMode => "Toggle include/exclude",
            Command::ToggleCaseFilter => "Toggle case sensitivity",
            Command::ToggleActiveFilterModeInOut => "Toggle include/exclude",
            Command::FilterHistoryPrevious => "Previous filter from history",
            Command::FilterHistoryNext => "Next filter from history",

            // Goto Line
            Command::ActivateGotoLineMode => "Go to line",

            // Display Options
            Command::ActivateOptionsView => "Display options",
            Command::ToggleOption => "Toggle option on/off",

            // Events
            Command::ActivateEventsView => "View log events",
            Command::ActivateEventFilterView => "Filter events",
            Command::GotoSelectedEvent => "Go to selected event",
            Command::ToggleEventFilter => "Toggle event filter",
            Command::ToggleAllEventFilters => "Toggle all event filters",
            Command::ToggleEventsShowMarks => "Toggle showing marks in events view",
            Command::EventNext => "Go to next event",
            Command::EventPrevious => "Go to previous event",

            // Marks
            Command::ToggleMark => "Toggle mark on line",
            Command::ActivateMarksView => "View marked lines",
            Command::GotoSelectedMark => "Go to selected mark",
            Command::ActivateMarkNameMode => "Name the mark",
            Command::UnmarkSelected => "Remove selected mark",
            Command::ClearAllMarks => "Clear all marks",
            Command::MarkNext => "Go to next mark",
            Command::MarkPrevious => "Go to previous mark",
            Command::ToggleShowMarkedOnly => "Show marked lines only on/off",

            // Files
            Command::ActivateFilesView => "View files list",
            Command::ToggleFile => "Toggle file visibility",

            // Expansion
            Command::ToggleExpansion => "Expand/collapse hidden lines",
            Command::CollapseAll => "Collapse all expansions",

            // Streaming
            Command::ToggleFollowMode => "Toggle follow mode (stdin)",
            Command::TogglePauseMode => "Toggle pause mode (stdin)",
            Command::ToggleCenterCursorMode => "Toggle center cursor mode",
            Command::ActivateSaveToFileMode => "Save to file (stdin)",

            // Selection
            Command::StartSelection => "Start visual selection",
            Command::CopySelection => "Copy selection to clipboard",
            Command::SelectToEventNext => "Select to next event",
            Command::SelectToEventPrevious => "Select to previous event",
            Command::SelectToMarkNext => "Select to next mark",
            Command::SelectToMarkPrevious => "Select to previous mark",
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
            Command::ScrollRight => app.scroll_right(false),
            Command::ScrollLeftSmall => app.viewport.scroll_left_small(),
            Command::ScrollRightSmall => app.scroll_right(true),
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
            Command::ActivateActiveSearchMode => app.activate_search_mode(),
            Command::SearchNext => app.search_next(),
            Command::SearchPrevious => app.search_previous(),
            Command::ToggleCaseSearch => app.toggle_case_sensitive(),
            Command::SearchHistoryPrevious => app.search_history_previous(),
            Command::SearchHistoryNext => app.search_history_next(),
            Command::TabCompletion => app.apply_tab_completion(),

            // Filter
            Command::ActivateActiveFilterMode => app.activate_filter_mode(),
            Command::ActivateFilterView => app.activate_filter_list_view(),
            Command::ActivateEditActiveFilterMode => app.activate_edit_filter_mode(),
            Command::ToggleFilterPattern => app.toggle_filter_pattern_active(),
            Command::RemoveFilterPattern => app.remove_filter_pattern(),
            Command::ToggleAllFilterPatterns => app.toggle_all_filter_patterns(),
            Command::ToggleFilterPatternCaseSensitive => app.toggle_filter_pattern_case_sensitive(),
            Command::ToggleFilterPatternMode => app.toggle_filter_pattern_mode(),
            Command::ToggleCaseFilter => app.toggle_case_sensitive(),
            Command::ToggleActiveFilterModeInOut => app.filter.toggle_mode(),
            Command::FilterHistoryPrevious => app.filter_history_previous(),
            Command::FilterHistoryNext => app.filter_history_next(),

            // Goto Line
            Command::ActivateGotoLineMode => app.activate_goto_line_mode(),

            // Display Options
            Command::ActivateOptionsView => app.activate_options_view(),
            Command::ToggleOption => app.toggle_option(),

            // Events
            Command::ActivateEventsView => app.activate_events_view(),
            Command::ActivateEventFilterView => app.activate_event_filter_view(),
            Command::GotoSelectedEvent => app.goto_selected_event(false),
            Command::ToggleEventFilter => app.toggle_event_filter(),
            Command::ToggleAllEventFilters => app.toggle_all_event_filters(),
            Command::ToggleEventsShowMarks => app.toggle_events_show_marks(),
            Command::EventNext => app.event_next(),
            Command::EventPrevious => app.event_previous(),

            // Marks
            Command::ToggleMark => app.toggle_mark(),
            Command::ActivateMarksView => app.activate_marks_view(),
            Command::GotoSelectedMark => app.goto_selected_mark(false),
            Command::ActivateMarkNameMode => app.activate_mark_name_overlay(),
            Command::UnmarkSelected => app.unmark_selected(),
            Command::ClearAllMarks => app.clear_all_marks(),
            Command::MarkNext => app.mark_next(),
            Command::MarkPrevious => app.mark_previous(),
            Command::ToggleShowMarkedOnly => app.toggle_show_marked_only(),

            // Files
            Command::ActivateFilesView => app.activate_files_view(),
            Command::ToggleFile => app.toggle_file(),

            // Expansion
            Command::ToggleExpansion => app.toggle_expansion(),
            Command::CollapseAll => app.collapse_all_expansions(),

            // Streaming
            Command::ToggleFollowMode => app.toggle_follow_mode(),
            Command::TogglePauseMode => app.toggle_pause_mode(),
            Command::ToggleCenterCursorMode => app.toggle_center_cursor_mode(),
            Command::ActivateSaveToFileMode => app.activate_save_to_file_mode(),

            // Selection
            Command::StartSelection => app.start_selection(),
            Command::CopySelection => app.copy_selection_to_clipboard(),
            Command::SelectToEventNext => app.select_to_event_next(),
            Command::SelectToEventPrevious => app.select_to_event_previous(),
            Command::SelectToMarkNext => app.select_to_mark_next(),
            Command::SelectToMarkPrevious => app.select_to_mark_previous(),
        }
        Ok(())
    }
}
