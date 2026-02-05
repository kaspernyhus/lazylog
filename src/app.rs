use crate::file_manager::FileFilterRule;
use crate::filter::FilterRule;
use crate::list_view_state::ListViewState;
use crate::marking::{Mark, MarkOnlyVisibilityRule, MarkTagRule};
use crate::time_filter::{
    TimeFilter, TimeFilterFocus, compute_date_rollover_separator_indices, compute_gap_separator_indices,
};
use crate::{
    cli::Cli,
    completion::CompletionEngine,
    config::{Config, Filters},
    event::{AppEvent, Event, EventHandler},
    event_mark_view::{EventMarkView, EventOrMark},
    expansion::Expansions,
    file_manager::FileManager,
    filter::{ActiveFilterMode, Filter, FilterPattern},
    help::Help,
    highlighter::{Highlighter, PatternStyle},
    keybindings::KeybindingRegistry,
    live_processor::ProcessingContext,
    log::LogBuffer,
    log_event::{LogEvent, LogEventTracker},
    marking::Marking,
    options::{AppOption, AppOptions},
    persistence::{PersistedState, clear_all_state, load_state, save_state},
    resolver::{Tag, ViewportResolver},
    search::Search,
    ui::colors::{FILTER_MODE_BG, FILTER_MODE_FG, SEARCH_MODE_BG, SEARCH_MODE_FG},
    viewport::Viewport,
};
use chrono::{DateTime, Utc};
use crossterm::event::Event::Key;
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{KeyCode, KeyEvent},
};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use tracing::trace;
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler as TuiEventHandler};

/// Represents the main views.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ViewState {
    /// Normal mode for viewing logs.
    LogView,
    /// Active search mode where a search query is highlighted and can be navigated.
    ActiveSearchMode,
    /// Active goto line mode where the user can input a line number to jump to.
    GotoLineMode,
    /// Active filter mode where the user can input a filter pattern to filter log lines.
    ActiveFilterMode,
    /// View for managing existing filter patterns.
    FilterView,
    /// View for adjusting display options.
    OptionsView,
    /// View for displaying all events found in the log.
    EventsView,
    /// View for displaying marked log lines.
    MarksView,
    /// View for listing opened files in multi-file sessions.
    FilesView,
    /// Visual selection mode for selecting a range of lines.
    SelectionMode,
    /// View for applying time filter range.
    TimeFilterView,
}

/// Represents an overlay/modal that appears on top of the current view.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Overlay {
    /// Edit an existing filter pattern.
    EditFilter,
    /// Filter selection for events view.
    EventsFilter,
    /// Active mode for entering a name/tag for a mark.
    MarkName,
    /// Active mode for entering a file name for saving the current log buffer to a file.
    SaveToFile,
    /// Active mode for entering a custom event pattern.
    AddCustomEvent,
    /// Edit a time filter
    EditTimeFilter,
    /// Display a message to the user.
    Message(String),
    /// Display an error message to the user.
    Error(String),
}

impl ViewState {
    pub fn has_text_input(&self) -> bool {
        matches!(
            self,
            ViewState::ActiveSearchMode | ViewState::ActiveFilterMode | ViewState::GotoLineMode
        )
    }
}

impl Overlay {
    pub fn has_text_input(&self) -> bool {
        matches!(
            self,
            Overlay::EditFilter | Overlay::MarkName | Overlay::SaveToFile | Overlay::AddCustomEvent
        )
    }
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Indicates whether the application is running.
    pub running: bool,
    /// Application configuration.
    pub config: Config,
    /// Current view being displayed.
    pub view_state: ViewState,
    /// Optional overlay on top of the view.
    pub overlay: Option<Overlay>,
    /// Event handler for managing app events such as user input.
    pub events: EventHandler,
    /// Log buffer containing the log lines.
    pub log_buffer: LogBuffer,
    /// Viewport for displaying log lines.
    pub viewport: Viewport,
    /// Help menu state.
    pub help: Help,
    /// Search state.
    pub search: Search,
    /// Filter state.
    pub filter: Filter,
    /// Filter list state
    pub filter_list_state: ListViewState,
    /// Syntax highlighter.
    pub highlighter: Highlighter,
    /// App options.
    pub options: AppOptions,
    /// Text input widget.
    pub input: Input,
    /// Indicates whether streaming is paused (only relevant in stdin/streaming mode).
    pub streaming_paused: bool,
    /// Log event tracker for managing log events.
    pub event_tracker: LogEventTracker,
    /// Log line marking manager
    pub marking: Marking,
    /// Markings list state
    pub marking_list_state: ListViewState,
    /// Events list state
    pub events_list_state: ListViewState,
    /// Event filter list state
    pub event_filter_list_state: ListViewState,
    /// File manager for multi-file sessions
    pub file_manager: FileManager,
    /// Files list state
    pub files_list_state: ListViewState,
    /// Options list state
    pub options_list_state: ListViewState,
    /// Viewport resolver for determining visible lines
    pub resolver: ViewportResolver,
    /// Expansion state for showing otherwise filtered lines
    expansion: Expansions,
    /// Selection range for visual selection mode.
    selection_range: Option<(usize, usize)>,
    /// Timestamp when a message was shown.
    message_timestamp: Option<std::time::Instant>,
    /// Tab completion.
    completion: CompletionEngine,
    /// Keybinding registry for all keybindings.
    keybindings: KeybindingRegistry,
    /// Whether persistence is enabled.
    persist_enabled: bool,
    /// Whether to only show marked lines
    pub show_marked_lines_only: bool,
    /// Active time filter for timestamp-based filtering.
    pub time_filter: Option<TimeFilter>,
    /// Cached file time range (min, max timestamps).
    pub file_time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Time filter start input field.
    pub time_filter_input_start: Input,
    /// Time filter end input field.
    pub time_filter_input_end: Input,
    /// Current focus in time filter popup.
    pub time_filter_focus: TimeFilterFocus,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let initial_overlay = if args.clear_state {
            match clear_all_state() {
                Ok(msg) => Some(Overlay::Message(msg)),
                Err(err) => Some(Overlay::Error(err)),
            }
        } else {
            None
        };

        let use_stdin = args.should_use_stdin();

        let events = EventHandler::new(use_stdin);

        let (config, initial_overlay) = match Config::load(&args.config) {
            Ok(config) => (config, initial_overlay),
            Err(err) => {
                let overlay = initial_overlay.or(Some(Overlay::Message(err)));
                (Config::default(), overlay)
            }
        };

        let mut filter_patterns = config.parse_filter_patterns();
        if let Some(filters_file) = Filters::load(&args.filters) {
            filter_patterns.extend(filters_file.parse_filter_patterns());
        }

        let keybindings = KeybindingRegistry::new();
        let mut help = Help::new();
        help.build_from_registry(&keybindings);

        let filter = Filter::with_patterns(filter_patterns);
        let filter_count = filter.count();

        let highlight_patterns = config.parse_highlight_patterns();
        let highlight_events = config.parse_highlight_event_patterns();
        let highlighter = Highlighter::new(highlight_patterns, highlight_events);

        let event_patterns = config.parse_log_event_patterns();
        let event_tracker = LogEventTracker::new(event_patterns);

        let mut app = Self {
            running: true,
            config,
            help,
            view_state: ViewState::LogView,
            overlay: initial_overlay,
            events,
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input: Input::default(),
            search: Search::default(),
            filter,
            filter_list_state: ListViewState::new_with_count(filter_count),
            options: AppOptions::default(),
            highlighter,
            streaming_paused: false,
            event_tracker,
            marking: Marking::default(),
            marking_list_state: ListViewState::new(),
            events_list_state: ListViewState::new(),
            event_filter_list_state: ListViewState::new(),
            file_manager: FileManager::new(&args.files),
            files_list_state: ListViewState::new(),
            options_list_state: ListViewState::new(),
            resolver: ViewportResolver::new(),
            expansion: Expansions::new(),
            selection_range: None,
            message_timestamp: None,
            completion: CompletionEngine::default(),
            keybindings,
            persist_enabled: !args.no_persist,
            show_marked_lines_only: false,
            time_filter: None,
            file_time_range: None,
            time_filter_input_start: Input::default(),
            time_filter_input_end: Input::default(),
            time_filter_focus: TimeFilterFocus::Start,
        };

        // Set item counts for list states
        app.files_list_state.set_item_count(app.file_manager.count());
        app.options_list_state.set_item_count(app.options.count());

        // Apply config defaults for time gap
        app.options
            .apply_time_gap_config(app.config.time_gap_enabled(), app.config.time_gap_threshold_minutes());

        if use_stdin {
            app.log_buffer.init_stdin_mode();
            app.viewport.follow_mode = true;
            app.update_processor_context();
            app.update_view();
            return app;
        }

        if !use_stdin && app.file_manager.is_empty() {
            app.show_error("No file paths provided");
            return app;
        }

        let file_paths = app.file_manager.paths();
        let load_result = app.log_buffer.load_files(&file_paths);

        match load_result {
            Ok(skipped_lines) => {
                app.file_time_range = app.log_buffer.compute_time_range();
                app.update_view();
                app.update_completion_words();

                if app.persist_enabled
                    && let Some(state) = load_state(&app.file_manager.paths())
                {
                    app.restore_state(state);
                }

                app.event_tracker.scan_all_lines(&app.log_buffer);
                app.update_events_view_count();

                if skipped_lines > 0 {
                    app.show_message(format!(
                            "Warning: Failed to parse timestamps for {} line(s).\nThe line(s) will not be displayed in the correct order!",
                            skipped_lines
                        ).as_str());
                }
            }
            Err(e) => {
                app.show_error(format!("Failed to load file(s): {}\nError: {}", args.files.join(", "), e).as_str())
            }
        }

        app
    }

    fn update_view(&mut self) {
        let update_start = Instant::now();

        let all_lines = self.log_buffer.all_lines();
        let log_line_index = self.resolver.viewport_to_log(self.viewport.selected_line, all_lines);

        self.resolver.clear_rules();

        if self.file_manager.is_multi_file() {
            let enabled_ids = self.file_manager.enabled_file_ids();
            self.resolver
                .add_visibility_rule(Box::new(FileFilterRule::new(Arc::new(enabled_ids))));
        }

        let patterns = Arc::new(self.filter.get_filter_patterns().to_vec());

        let mut always_visible = HashSet::new();
        let marked_indices = self.marking.get_marked_indices();
        if self.options.is_enabled(AppOption::AlwaysShowMarkedLines) {
            always_visible.extend(&marked_indices);
        }
        if self.options.is_enabled(AppOption::AlwaysShowCriticalEvents) {
            always_visible.extend(self.event_tracker.get_critical_event_indices());
        }
        if self.options.is_enabled(AppOption::AlwaysShowCustomEvents) {
            always_visible.extend(self.event_tracker.get_custom_event_indices());
        }

        self.resolver
            .add_visibility_rule(Box::new(FilterRule::new(patterns, Arc::new(always_visible))));

        if let Some(ref time_filter) = self.time_filter {
            self.resolver.add_visibility_rule(Box::new(time_filter.clone()));
        }

        let marked_indices = Arc::new(marked_indices);

        if self.show_marked_lines_only {
            self.resolver
                .add_visibility_rule(Box::new(MarkOnlyVisibilityRule::new(marked_indices.clone())));
        }

        self.resolver.add_tag_rule(Box::new(MarkTagRule::new(marked_indices)));

        self.resolver.set_expanded_lines(self.expansion.get_all_expanded());

        if !self.log_buffer.streaming && self.options.is_enabled(AppOption::ShowDateRollover) {
            let date_rollover_indices = compute_date_rollover_separator_indices(all_lines);
            self.resolver.set_date_rollover_indices(date_rollover_indices);
        }

        if !self.log_buffer.streaming && self.options.is_enabled(AppOption::TimeGapThreshold) {
            let skip_date_rollovers = self.options.is_enabled(AppOption::ShowDateRollover);
            let gap_indices =
                compute_gap_separator_indices(all_lines, self.options.get_gap_threshold_minutes(), skip_date_rollovers);
            self.resolver.set_gap_separator_indices(gap_indices);
        }

        let num_lines = {
            let visible_lines = self.resolver.get_visible_lines(all_lines);
            let num_lines = visible_lines.len();

            // Update search matches if there's an active search
            if let Some(pattern) = self.search.get_active_pattern().map(str::to_string) {
                let visible_content = visible_lines.iter().map(|v| all_lines[v.log_index].content());
                let all_content = all_lines.iter().map(|log_line| log_line.content());
                self.search.update_matches(&pattern, visible_content, all_content);
            }

            num_lines
        };

        self.viewport.set_total_lines(num_lines);

        // Call after the all_lines scope ends
        self.update_events_view_count();

        if self.log_buffer.streaming {
            self.update_processor_context();
        }

        if num_lines == 0 {
            self.viewport.selected_line = 0;
            return;
        }

        if self.log_buffer.streaming && self.viewport.follow_mode {
            self.viewport.goto_bottom();
        } else {
            let new_selected_line = if let Some(target_log_line_index) = log_line_index {
                // Find closest visible line to the target
                let all_lines = self.log_buffer.all_lines();
                self.resolver
                    .log_to_viewport(target_log_line_index, all_lines)
                    .unwrap_or_else(|| {
                        // Find closest visible line
                        let visible = self.resolver.get_visible_lines(all_lines);
                        visible
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, v)| v.log_index.abs_diff(target_log_line_index))
                            .map(|(idx, _)| idx)
                            .unwrap_or(self.viewport.selected_line.min(num_lines - 1))
                    })
            } else {
                self.viewport.selected_line.min(num_lines - 1)
            };

            self.viewport.goto_line(new_selected_line, false);
        }
        trace!("update_view took: {:?}", update_start.elapsed());
    }

    fn update_processor_context(&self) {
        if let Some(processor) = &self.events.processor {
            let context = ProcessingContext {
                filter_patterns: self.filter.get_filter_patterns().to_vec(),
                search_pattern: self.search.get_active_pattern().map(|p| p.to_string()),
                search_case_sensitive: self.search.is_case_sensitive(),
            };
            processor.update_context(context);
        }
    }

    /// Transitions to a new view state, clearing any overlay.
    fn set_view_state(&mut self, view: ViewState) {
        self.view_state = view;
        self.overlay = None;
        self.update_temporary_highlights();
    }

    /// Shows a message overlay.
    fn show_message(&mut self, message: &str) {
        self.show_overlay(Overlay::Message(message.to_string()));
    }

    /// Shows an error overlay.
    fn show_error(&mut self, error: &str) {
        self.show_overlay(Overlay::Error(error.to_string()));
    }

    pub fn show_overlay(&mut self, overlay: Overlay) {
        if matches!(overlay, Overlay::Message(_)) {
            self.message_timestamp = Some(std::time::Instant::now());
        }
        self.overlay = Some(overlay);
    }

    pub fn close_overlay(&mut self) {
        self.overlay = None;
        self.message_timestamp = None;
    }

    fn update_completion_words(&mut self) {
        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(all_lines);
        let visible_iter = visible_lines.iter().map(|vl| &all_lines[vl.log_index]);
        self.completion.update(visible_iter);
    }

    pub fn apply_tab_completion(&mut self) {
        if !matches!(
            self.view_state,
            ViewState::ActiveSearchMode | ViewState::ActiveFilterMode
        ) {
            return;
        }

        if let Some(completion) = self.completion.find_completion(self.input.value()) {
            let full_text = format!("{}{}", self.input.value(), completion);
            self.input = Input::new(full_text);
            self.update_temporary_highlights();
        }
    }

    /// Returns the input prefix for the current state.
    /// This is the single source of truth for input prefixes used in both rendering and cursor positioning.
    pub fn get_input_prefix(&self) -> String {
        if let Some(ref overlay) = self.overlay {
            match overlay {
                Overlay::SaveToFile => return "Save to file: ".to_string(),
                Overlay::EditTimeFilter => {
                    return match self.time_filter_focus {
                        TimeFilterFocus::Start => "Start: ".to_string(),
                        TimeFilterFocus::End => "End: ".to_string(),
                    };
                }
                _ => {}
            }
        }

        // Check view states
        match self.view_state {
            ViewState::ActiveSearchMode => {
                let case_sensitive = if self.search.is_case_sensitive() { "Aa" } else { "aa" };
                format!("Search: [{}] ", case_sensitive)
            }
            ViewState::ActiveFilterMode => {
                let filter_mode = match self.filter.get_mode() {
                    ActiveFilterMode::Include => "IN",
                    ActiveFilterMode::Exclude => "EX",
                };
                let case_sensitive = if self.filter.is_case_sensitive() { "Aa" } else { "aa" };
                format!("Filter: [{}] [{}] ", case_sensitive, filter_mode)
            }
            ViewState::GotoLineMode => "Go to line: ".to_string(),
            _ => String::new(),
        }
    }

    fn update_temporary_highlights(&mut self) {
        self.highlighter.clear_temporary_highlights();

        // Add filter mode preview highlight
        if (self.view_state == ViewState::ActiveFilterMode || matches!(self.overlay, Some(Overlay::EditFilter)))
            && self.input.value().chars().count() >= 2
        {
            self.highlighter.add_temporary_highlight(
                self.input.value(),
                PatternStyle::new(Some(FILTER_MODE_FG), Some(FILTER_MODE_BG), true),
                self.filter.is_case_sensitive(),
            );
        }

        // Add search mode preview highlight
        if self.view_state == ViewState::ActiveSearchMode && self.input.value().chars().count() >= 2 {
            self.highlighter.add_temporary_highlight(
                self.input.value(),
                PatternStyle::new(Some(SEARCH_MODE_FG), Some(SEARCH_MODE_BG), true),
                self.search.is_case_sensitive(),
            );
        }

        // Add active search highlight
        if let Some(pattern) = self.search.get_active_pattern()
            && !pattern.is_empty()
            && self.view_state != ViewState::ActiveSearchMode
        {
            self.highlighter.add_temporary_highlight(
                pattern,
                PatternStyle::new(Some(SEARCH_MODE_FG), Some(SEARCH_MODE_BG), false),
                self.search.is_case_sensitive(),
            );
        }
    }

    fn calculate_cursor_pos(&self, width: u16, height: u16) -> Option<(u16, u16)> {
        if self.help.is_visible() {
            return None;
        }

        if !self.is_text_input_mode() {
            return None;
        }

        if let Some(overlay) = &self.overlay
            && overlay.has_text_input()
        {
            if let Some((popup_width, popup_height)) = overlay.popup_size() {
                let cursor_x = (width.saturating_sub(popup_width)) / 2 + 1 + self.input.visual_cursor() as u16;
                let cursor_y = (height.saturating_sub(popup_height)) / 2 + 1;
                return Some((cursor_x, cursor_y));
            }
            return None;
        }

        let footer_y = height.saturating_sub(1);
        let prefix_width = self.get_input_prefix().len();
        let cursor_x = (prefix_width + self.input.visual_cursor()) as u16;
        Some((cursor_x, footer_y))
    }

    /// Run the application's main loop.
    pub async fn run<B: Backend>(mut self, mut terminal: Terminal<B>) -> color_eyre::Result<()> {
        let terminal_size = terminal.size()?;
        self.viewport.resize(
            terminal_size.width.saturating_sub(1) as usize,
            terminal_size.height.saturating_sub(2) as usize,
        );
        self.viewport.scroll_margin = 2;

        while self.running {
            let draw_start = Instant::now();
            terminal.draw(|frame| {
                frame.render_widget(&self, frame.area());
                if let Some((x, y)) = self.calculate_cursor_pos(frame.area().width, frame.area().height) {
                    frame.set_cursor_position((x, y));
                }
            })?;
            let draw_elapsed = draw_start.elapsed();
            trace!("Screen draw took: {:?}", draw_elapsed);

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    Key(key_event) => {
                        self.handle_key_events(key_event)?;
                    }
                    crossterm::event::Event::Resize(x, y) => {
                        self.viewport
                            .resize(x.saturating_sub(1) as usize, y.saturating_sub(2) as usize);
                    }
                    _ => {}
                },
                Event::App(app_event) => {
                    self.handle_app_event(app_event)?;
                }
            }
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {
        if let Some(timestamp) = self.message_timestamp
            && timestamp.elapsed().as_secs() >= 3
            && matches!(self.overlay, Some(Overlay::Message(_)))
        {
            self.set_view_state(ViewState::LogView);
        }
    }

    /// Set running to false to quit the application.
    ///
    /// If not in streaming mode, persist current state to disk.
    pub fn quit(&mut self) {
        if self.persist_enabled && !self.log_buffer.streaming {
            save_state(&self.file_manager.paths(), self);
        }

        self.running = false;
    }

    /// Restores application state from a persisted state.
    fn restore_state(&mut self, state: PersistedState) {
        self.options.restore(&state.options());

        self.search.history.restore(state.search_history().to_vec());
        self.filter.history.restore(state.filter_history().to_vec());

        for filter_state in state.filters() {
            let new_filter = FilterPattern::new(
                filter_state.pattern().to_string(),
                filter_state.mode(),
                filter_state.case_sensitive(),
                filter_state.enabled(),
            );

            self.filter.add_filter(&new_filter);
        }

        self.filter_list_state.set_item_count(self.filter.count());

        for mark_state in state.marks() {
            let line_index = mark_state.line_index();
            if line_index < self.log_buffer.get_total_lines_count() {
                self.marking.toggle_mark(line_index);
                if let Some(name) = mark_state.name() {
                    self.marking.set_mark_name(line_index, name);
                }
            }
        }

        for custom_event in state.custom_events() {
            let pattern = custom_event.pattern();
            self.event_tracker.add_custom_event(pattern);

            let style = PatternStyle {
                fg_color: None,
                bg_color: Some(self.config.custom_event_bg_color()),
                bold: false,
            };
            self.highlighter.add_custom_event(pattern, style);
        }

        let event_filter_states: Vec<(String, bool)> = state
            .event_filters()
            .iter()
            .map(|ef| (ef.name().to_string(), ef.enabled()))
            .collect();

        self.event_tracker.restore_filter_states(&event_filter_states);

        let all_lines = self.log_buffer.all_lines();
        let filtered_lines = self.resolver.visible_count(all_lines);
        if filtered_lines > 0 {
            self.viewport.selected_line = state.viewport_selected_line().min(filtered_lines - 1);
            self.viewport.top_line = state
                .viewport_top_line()
                .min(filtered_lines.saturating_sub(self.viewport.height));
            self.viewport.horizontal_offset = state.viewport_horizontal_offset();
        }

        self.viewport.center_cursor_mode = state.viewport_center_cursor_mode();

        self.update_temporary_highlights();
        self.update_view();
    }

    /// Handles application events and updates the state of [`App`].
    fn handle_app_event(&mut self, app_event: AppEvent) -> color_eyre::Result<()> {
        match app_event {
            AppEvent::NewLines(processed_lines) => {
                if self.streaming_paused {
                    return Ok(());
                }

                let mut should_select = false;
                for pl in processed_lines {
                    let log_line_index = self.log_buffer.append_line(pl.line_content);
                    let log_line = self.log_buffer.get_line(log_line_index).unwrap();

                    let active_event = self.event_tracker.scan_single_line(log_line);
                    if active_event && self.viewport.follow_mode {
                        should_select = true;
                    }

                    if pl.passes_filter {
                        let lines = self.log_buffer.all_lines();
                        let viewport_index = self.resolver.log_to_viewport(log_line_index, lines).unwrap_or(0);
                        self.completion.append_line(log_line);
                        self.search.append_line(viewport_index, log_line.content());
                    }
                }

                self.update_view();

                if should_select {
                    self.events_list_state.select_last();
                }

                if self.viewport.follow_mode {
                    self.viewport.goto_bottom();
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        if self.is_text_input_mode() {
            self.handle_text_input(key_event);
            self.update_temporary_highlights();
        }

        if let Some(command) = self.keybindings.lookup(&self.view_state, &self.overlay, key_event) {
            command.execute(self)?;
        }

        Ok(())
    }

    /// Checks if the current state is a text input mode.
    fn is_text_input_mode(&self) -> bool {
        if self.help.is_visible() {
            return false;
        }
        self.view_state.has_text_input()
            || self.overlay.as_ref().is_some_and(|o| o.has_text_input())
            || matches!(self.overlay, Some(Overlay::EditTimeFilter))
    }

    /// Handles text input for input modes.
    fn handle_text_input(&mut self, key_event: KeyEvent) {
        if self.view_state == ViewState::GotoLineMode {
            match key_event.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.input.handle(InputRequest::InsertChar(c));
                }
                KeyCode::Char(_) => {
                    // Ignore non-digit characters
                }
                _ => {
                    self.input.handle_event(&Key(key_event));
                }
            }
            return;
        }

        self.input.handle_event(&Key(key_event));
    }

    pub fn confirm(&mut self) {
        if let Some(ref overlay) = self.overlay {
            match overlay {
                Overlay::EditFilter => {
                    if !self.input.value().is_empty() {
                        let selected_index = self.filter_list_state.selected_index();
                        self.filter.update_pattern(selected_index, self.input.value());
                        self.expansion.clear();
                        self.update_view();
                    }
                    self.close_overlay();
                    return;
                }
                Overlay::SaveToFile => {
                    if !self.input.value().is_empty() {
                        match self.log_buffer.save_to_file(self.input.value()) {
                            Ok(_) => {
                                let abs_path = std::fs::canonicalize(self.input.value())
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| self.input.value().to_string());
                                self.show_message(format!("Log saved to file:\n{}", abs_path).as_str());
                            }
                            Err(e) => {
                                self.show_error(format!("Failed to save file:\n{}", e).as_str());
                            }
                        }
                    } else {
                        self.close_overlay();
                    }
                    return;
                }
                Overlay::MarkName => {
                    if self.view_state == ViewState::EventsView && self.event_tracker.showing_marks() {
                        let (events, _) = self.get_events_for_list();
                        let visible_marks = self.get_visible_marks();
                        let merged_items = EventMarkView::merge(&events, &visible_marks, true);

                        if let Some(EventOrMark::Mark(mark)) = merged_items.get(self.events_list_state.selected_index())
                        {
                            self.marking.set_mark_name(mark.line_index, self.input.value());
                        }
                    } else if self.view_state == ViewState::MarksView
                        && let Some(mark) = self.get_selected_mark()
                    {
                        self.marking.set_mark_name(mark.line_index, self.input.value());
                    }

                    self.close_overlay();
                    return;
                }
                Overlay::AddCustomEvent => {
                    if !self.input.value().is_empty() {
                        let pattern = self.input.value().to_string();
                        if self.event_tracker.add_custom_event(&pattern) {
                            let style = PatternStyle {
                                fg_color: None,
                                bg_color: Some(self.config.custom_event_bg_color()),
                                bold: false,
                            };
                            self.highlighter.add_custom_event(&pattern, style);

                            self.event_tracker.scan_all_lines(&self.log_buffer);
                            self.update_events_view_count();
                        }
                    }
                    self.close_overlay();
                    return;
                }
                Overlay::EventsFilter => {
                    self.close_overlay();
                    // Don't change logview selection from the event filter list
                    self.set_view_state(ViewState::LogView);
                }
                Overlay::EditTimeFilter => {
                    let new_value = self.input.value().to_string();
                    if Self::parse_timestamp(&new_value).is_none() {
                        self.show_message("Invalid timestamp format.\nExpected: YYYY-MM-DD HH:MM:SS");
                        return;
                    }

                    if self.verify_time_filter_input() {
                        match self.time_filter_focus {
                            TimeFilterFocus::Start => self.time_filter_input_start = Input::new(new_value),
                            TimeFilterFocus::End => self.time_filter_input_end = Input::new(new_value),
                        }
                    }

                    self.close_overlay();
                    return;
                }
                Overlay::Message(_) => {
                    self.close_overlay();
                    return;
                }
                _ => {}
            }
        }

        match self.view_state {
            ViewState::ActiveSearchMode => {
                if self.input.value().is_empty() {
                    self.search.clear_matches();
                } else {
                    let all_lines = self.log_buffer.all_lines();
                    let visible_lines = self.resolver.get_visible_lines(all_lines);
                    let content_iter = visible_lines.iter().map(|vl| all_lines[vl.log_index].content());
                    let all_content_iter = all_lines.iter().map(|log_line| log_line.content());

                    let visible_matches = self
                        .search
                        .apply_pattern(self.input.value(), content_iter, all_content_iter);

                    if let Some(matches) = visible_matches
                        && matches == 0
                    {
                        let (_, _, total_matches) = self.search.get_match_info();
                        if total_matches > 0 {
                            self.show_message(
                                format!(
                                    "0 hits for '{}' ({} in filtered lines)",
                                    self.input.value(),
                                    total_matches
                                )
                                .as_str(),
                            );
                        } else {
                            self.show_message(format!("0 hits for '{}'", self.input.value()).as_str());
                        }
                        return;
                    }

                    if self.options.is_disabled(AppOption::SearchDisableJumping) && !self.viewport.follow_mode {
                        if let Some(line) = self.search.first_match_from(self.viewport.selected_line) {
                            self.push_viewport_line_to_history(line);
                            self.viewport.goto_line(line, false);
                        }
                        self.viewport.follow_mode = false;
                    }
                }
                self.set_view_state(ViewState::LogView);
            }
            ViewState::ActiveFilterMode => {
                if !self.input.value().is_empty() {
                    self.filter.add_filter_from_pattern(self.input.value());
                    self.filter_list_state.set_item_count(self.filter.count());
                    self.expansion.clear();
                    self.update_view();
                }
                self.set_view_state(ViewState::LogView);
            }
            ViewState::EventsView => {
                self.goto_selected_event(true);
                self.set_view_state(ViewState::LogView);
            }
            ViewState::OptionsView => {
                let selected_index = self.options_list_state.selected_index();
                self.options.enable_option(selected_index);
                self.highlighter.invalidate_cache();
                self.update_view();
                self.set_view_state(ViewState::LogView);
            }
            ViewState::MarksView => {
                self.goto_selected_mark(true);
                self.set_view_state(ViewState::LogView);
            }
            ViewState::GotoLineMode => {
                if let Ok(line_number) = self.input.value().parse::<usize>() {
                    let viewport_index = line_number.saturating_sub(1);
                    if line_number > 0 && viewport_index < self.viewport.total_lines {
                        self.push_viewport_line_to_history(viewport_index);
                        self.viewport.goto_line(viewport_index, true);
                    }
                }
                self.set_view_state(ViewState::LogView);
            }
            _ => {}
        }
    }

    pub fn cancel(&mut self) {
        // Handle overlays first
        if let Some(ref overlay) = self.overlay {
            match overlay {
                Overlay::EventsFilter => {
                    self.close_overlay();
                }
                Overlay::MarkName => {
                    self.close_overlay();
                }
                Overlay::EditFilter => {
                    self.set_view_state(ViewState::FilterView);
                }
                Overlay::SaveToFile => {
                    self.set_view_state(ViewState::LogView);
                }
                Overlay::AddCustomEvent => {
                    self.close_overlay();
                }
                Overlay::EditTimeFilter => {
                    self.close_overlay();
                }
                Overlay::Message(_) => {
                    self.set_view_state(ViewState::LogView);
                }
                Overlay::Error(_) => {}
            }
            return;
        }

        // Handle view states
        match self.view_state {
            ViewState::ActiveSearchMode => {
                self.search.clear_matches();
                self.set_view_state(ViewState::LogView);
            }
            ViewState::GotoLineMode | ViewState::ActiveFilterMode => {
                self.set_view_state(ViewState::LogView);
            }
            ViewState::SelectionMode => {
                self.cancel_selection();
                self.set_view_state(ViewState::LogView);
            }
            ViewState::LogView => {
                self.search.clear_matches();
                self.update_temporary_highlights();

                if self.show_marked_lines_only {
                    self.show_marked_lines_only = false;
                    self.update_view();
                }
            }
            ViewState::FilterView
            | ViewState::OptionsView
            | ViewState::EventsView
            | ViewState::MarksView
            | ViewState::TimeFilterView
            | ViewState::FilesView => {
                self.set_view_state(ViewState::LogView);
            }
        }
    }

    /// Checks if current viewport position is a TimeGap separator and skips it.
    /// Direction: true = moving down, false = moving up.
    fn skip_time_gap_separator(&mut self, direction_down: bool) {
        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(all_lines);

        while self.viewport.selected_line < visible_lines.len() {
            if let Some(vl) = visible_lines.get(self.viewport.selected_line) {
                if !(vl.tags.contains(&Tag::TimeGap) | vl.tags.contains(&Tag::DateRollover)) {
                    break;
                }
                if direction_down {
                    if self.viewport.selected_line + 1 < visible_lines.len() {
                        self.viewport.selected_line += 1;
                    } else {
                        break;
                    }
                } else if self.viewport.selected_line > 0 {
                    self.viewport.selected_line -= 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    pub fn move_up(&mut self) {
        // Handle overlay-specific navigation
        if let Some(Overlay::EventsFilter) = self.overlay {
            self.event_filter_list_state.move_up_wrap();
            return;
        }

        // Handle view-specific navigation
        match self.view_state {
            ViewState::FilterView => self.filter_list_state.move_up_wrap(),
            ViewState::OptionsView => self.options_list_state.move_up_wrap(),
            ViewState::EventsView => {
                self.events_list_state.move_up();
                self.viewport.follow_mode = false;
            }
            ViewState::MarksView => {
                self.marking_list_state.move_up();
            }
            ViewState::FilesView => {
                self.files_list_state.move_up();
            }
            ViewState::TimeFilterView => {
                self.time_filter_focus_next();
            }
            ViewState::SelectionMode => {
                self.viewport.move_up();
                self.skip_time_gap_separator(false);
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.move_up();
                self.skip_time_gap_separator(false);
                self.viewport.follow_mode = false;
            }
        }
    }

    pub fn move_down(&mut self) {
        // Handle overlay-specific navigation
        if let Some(Overlay::EventsFilter) = self.overlay {
            self.event_filter_list_state.move_down_wrap();
            return;
        }

        // Handle view-specific navigation
        match self.view_state {
            ViewState::FilterView => self.filter_list_state.move_down_wrap(),
            ViewState::OptionsView => self.options_list_state.move_down_wrap(),
            ViewState::EventsView => {
                self.events_list_state.move_down();
            }
            ViewState::MarksView => {
                self.marking_list_state.move_down();
            }
            ViewState::FilesView => {
                self.files_list_state.move_down();
            }
            ViewState::TimeFilterView => {
                self.time_filter_focus_next();
            }
            ViewState::SelectionMode => {
                self.viewport.move_down();
                self.skip_time_gap_separator(true);
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.move_down();
                self.skip_time_gap_separator(true);
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.view_state {
            ViewState::EventsView => {
                self.events_list_state.page_up();
            }
            ViewState::MarksView => {
                self.marking_list_state.page_up();
            }
            ViewState::FilesView => {
                self.files_list_state.page_up();
            }
            ViewState::SelectionMode => {
                self.viewport.page_up();
                self.skip_time_gap_separator(false);
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.page_up();
                self.skip_time_gap_separator(false);
                self.viewport.follow_mode = false;
            }
        }
    }

    pub fn page_down(&mut self) {
        match self.view_state {
            ViewState::EventsView => {
                self.events_list_state.page_down();
            }
            ViewState::MarksView => {
                self.marking_list_state.page_down();
            }
            ViewState::FilesView => {
                self.files_list_state.page_down();
            }
            ViewState::SelectionMode => {
                self.viewport.page_down();
                self.skip_time_gap_separator(true);
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.page_down();
                self.skip_time_gap_separator(true);
            }
        }
    }

    pub fn goto_top(&mut self) {
        self.viewport.goto_top();
        self.skip_time_gap_separator(true);
        self.push_viewport_line_to_history(self.viewport.selected_line);
        self.viewport.follow_mode = false;
    }

    pub fn goto_bottom(&mut self) {
        self.viewport.goto_bottom();
        self.skip_time_gap_separator(false);
        self.push_viewport_line_to_history(self.viewport.selected_line);
    }

    pub fn activate_search_mode(&mut self) {
        self.input.reset();
        self.search.clear_matches();
        self.search.reset_case_sensitivity();
        self.search.history.reset();
        self.set_view_state(ViewState::ActiveSearchMode);
    }

    pub fn activate_goto_line_mode(&mut self) {
        self.input.reset();
        self.set_view_state(ViewState::GotoLineMode);
        self.viewport.follow_mode = false;
    }

    pub fn activate_filter_mode(&mut self) {
        self.input.reset();
        self.filter.reset_mode();
        self.filter.reset_case_sensitivity();
        self.filter.history.reset();
        self.set_view_state(ViewState::ActiveFilterMode);
    }

    pub fn activate_filter_list_view(&mut self) {
        self.set_view_state(ViewState::FilterView);
    }

    pub fn activate_edit_filter_mode(&mut self) {
        let selected_index = self.filter_list_state.selected_index();
        if let Some(filter) = self.filter.get_pattern(selected_index) {
            self.input = Input::new(filter.pattern.clone());
            self.show_overlay(Overlay::EditFilter);
        }
    }

    pub fn activate_options_view(&mut self) {
        self.set_view_state(ViewState::OptionsView);
    }

    pub fn toggle_option(&mut self) {
        let selected_index = self.options_list_state.selected_index();
        self.options.toggle_option(selected_index);
        self.highlighter.invalidate_cache();
        self.update_view();
    }

    pub fn increment_option(&mut self) {
        let selected_index = self.options_list_state.selected_index();
        self.options.increment_option(selected_index);
        self.update_view();
    }

    pub fn decrement_option(&mut self) {
        let selected_index = self.options_list_state.selected_index();
        self.options.decrement_option(selected_index);
        self.update_view();
    }

    pub fn activate_events_view(&mut self) {
        // Scan events on first activation (events list is empty)
        if self.event_tracker.is_empty() {
            self.event_tracker.scan_all_lines(&self.log_buffer);
        }
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line) {
            if let Some(nearest_index) = self.find_nearest_event(line_index) {
                self.events_list_state.select_index(nearest_index);
            } else {
                self.events_list_state.select_index(0);
            }
        }
        self.set_view_state(ViewState::EventsView);
    }

    pub fn activate_event_filter_view(&mut self) {
        if self.view_state == ViewState::EventsView {
            self.show_overlay(Overlay::EventsFilter);
        }
    }

    pub fn activate_marks_view(&mut self) {
        let visible_mark_count = self.get_visible_marks().len();
        self.marking_list_state.set_item_count(visible_mark_count);

        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line) {
            self.select_nearest_mark(line_index);
        } else {
            self.marking_list_state.reset();
        }

        self.set_view_state(ViewState::MarksView);
    }

    pub fn activate_files_view(&mut self) {
        if self.file_manager.is_multi_file() {
            self.set_view_state(ViewState::FilesView);
        }
    }

    pub fn toggle_file(&mut self) {
        let selected_index = self.files_list_state.selected_index();
        self.file_manager.toggle_enabled(selected_index);
        self.expansion.clear();
        self.update_view();
    }

    pub fn activate_mark_name_overlay(&mut self) {
        // Handle EventsView with merged marks
        if self.view_state == ViewState::EventsView {
            if self.event_tracker.showing_marks() {
                let (events, _) = self.get_events_for_list();
                let visible_marks = self.get_visible_marks();
                let merged_items = EventMarkView::merge(&events, &visible_marks, true);

                if let Some(EventOrMark::Mark(mark)) = merged_items.get(self.events_list_state.selected_index()) {
                    if let Some(name) = &mark.name {
                        self.input = Input::new(name.clone());
                    } else {
                        self.input.reset();
                    }
                    self.show_overlay(Overlay::MarkName);
                }
            }
            return;
        }

        // Handle MarksView
        if self.view_state == ViewState::MarksView
            && let Some(mark) = self.get_selected_mark()
        {
            if let Some(name) = &mark.name {
                self.input = Input::new(name.clone());
            } else {
                self.input.reset();
            }
            self.show_overlay(Overlay::MarkName);
        }
    }

    pub fn activate_save_to_file_mode(&mut self) {
        if self.log_buffer.streaming {
            self.input.reset();
            self.show_overlay(Overlay::SaveToFile);
        }
    }

    pub fn activate_add_custom_event_mode(&mut self) {
        if self.view_state == ViewState::EventsView {
            self.input.reset();
            self.show_overlay(Overlay::AddCustomEvent);
        }
    }

    pub fn remove_custom_event(&mut self) {
        let event_name = if self.overlay == Some(Overlay::EventsFilter) {
            let event_stats = self.event_tracker.get_event_stats();
            event_stats
                .get(self.event_filter_list_state.selected_index())
                .map(|es| es.name.clone())
        } else if self.view_state == ViewState::EventsView {
            let (events, _) = self.get_events_for_list();
            let visible_marks = self.get_visible_marks();
            let merged = EventMarkView::merge(&events, &visible_marks, self.event_tracker.showing_marks());
            let selected_idx = self.events_list_state.selected_index();
            if let Some(EventOrMark::Event(event)) = merged.get(selected_idx) {
                Some(event.name.clone())
            } else {
                None
            }
        } else {
            // Not in EventsFilter or EventsView mode
            return;
        };

        if let Some(name) = event_name {
            if !self.event_tracker.is_custom_event(&name) {
                return;
            }

            if let Some(pattern) = self.event_tracker.remove_custom_event(&name) {
                self.highlighter.remove_custom_event(&pattern);
            }

            self.update_events_view_count();
        }
    }

    pub fn toggle_mark(&mut self) {
        if self.view_state == ViewState::SelectionMode {
            if let Some((start, end)) = self.get_selection_range() {
                let log_indices: Vec<usize> = (start..=end)
                    .filter_map(|viewport_line| self.viewport_to_log_line_index(viewport_line))
                    .collect();

                if log_indices.is_empty() {
                    return;
                }

                // Check if all lines are marked
                let all_marked = log_indices.iter().all(|&idx| self.marking.is_marked(idx));

                if all_marked {
                    for &idx in &log_indices {
                        self.marking.toggle_mark(idx);
                    }
                } else {
                    for &idx in &log_indices {
                        if !self.marking.is_marked(idx) {
                            self.marking.toggle_mark(idx);
                        }
                    }
                }
            }
        } else if self.view_state == ViewState::EventsView {
            let (events, _) = self.get_events_for_list();
            let visible_marks = self.get_visible_marks();
            let merged = EventMarkView::merge(&events, &visible_marks, self.event_tracker.showing_marks());
            let selected_idx = self.events_list_state.selected_index();
            if let Some(line_index) = merged.get(selected_idx).map(|item| item.line_index()) {
                self.marking.toggle_mark(line_index);
            }
        } else if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line) {
            self.marking.toggle_mark(line_index);
        }

        let new_count = self.marking.count();
        self.marking_list_state.set_item_count(new_count);

        if self.show_marked_lines_only {
            self.update_view();
        } else {
            let marked_indices = self.marking.get_marked_indices();
            self.resolver.update_mark_tags(&marked_indices);
        }
    }

    pub fn unmark_selected(&mut self) {
        if let Some(mark) = self.get_selected_mark() {
            let line_index = mark.line_index;
            self.marking.unmark(line_index);

            let new_count = self.marking.count();
            self.marking_list_state.set_item_count(new_count);

            if self.show_marked_lines_only {
                self.update_view();
            } else {
                let marked_indices = self.marking.get_marked_indices();
                self.resolver.update_mark_tags(&marked_indices);
            }
        }
    }

    /// Converts viewport index to actual log line index.
    fn viewport_to_log_line_index(&mut self, viewport_idx: usize) -> Option<usize> {
        let all_lines = self.log_buffer.all_lines();
        self.resolver.viewport_to_log(viewport_idx, all_lines)
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.search.toggle_case_sensitivity();
        self.filter.toggle_case_sensitivity();

        if self.view_state == ViewState::ActiveSearchMode {
            let all_lines = self.log_buffer.all_lines();
            let visible_lines = self.resolver.get_visible_lines(all_lines);
            let content_iter = visible_lines.iter().map(|vl| all_lines[vl.log_index].content());
            let all_content_iter = all_lines.iter().map(|log_line| log_line.content());
            self.search
                .update_matches(self.input.value(), content_iter, all_content_iter);
        }

        self.update_temporary_highlights();
    }

    pub fn search_next(&mut self) {
        if let Some(line) = self.search.next_match(self.viewport.selected_line) {
            self.push_viewport_line_to_history(line);
            self.viewport.goto_line(line, false);
        }
    }

    pub fn search_previous(&mut self) {
        if let Some(line) = self.search.previous_match(self.viewport.selected_line) {
            self.push_viewport_line_to_history(line);
            self.viewport.goto_line(line, false);
        }
    }

    pub fn mark_next(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(next_mark_line) = self.get_next_mark(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(next_mark_line, all_lines) {
                self.viewport.push_history(next_mark_line);
                self.viewport.goto_line(viewport_idx, false);
            }
        }
    }

    pub fn mark_previous(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(prev_mark_line) = self.get_previous_mark(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(prev_mark_line, all_lines) {
                self.viewport.push_history(prev_mark_line);
                self.viewport.goto_line(viewport_idx, false);
            }
        }
    }

    pub fn event_next(&mut self) {
        let line_index = self.viewport_to_log_line_index(self.viewport.selected_line);
        let next_line = match line_index {
            Some(line_idx) if self.overlay == Some(Overlay::EventsFilter) => {
                self.get_next_event_line_by_filter(line_idx)
            }
            Some(line_idx) => self.get_next_event_line(line_idx),
            None => None,
        };
        if let Some(next_event_line) = next_line {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(next_event_line, all_lines) {
                self.viewport.push_history(next_event_line);
                self.viewport.goto_line(viewport_idx, false);
            }
        }
    }

    pub fn event_previous(&mut self) {
        let line_index = self.viewport_to_log_line_index(self.viewport.selected_line);
        let prev_line = match line_index {
            Some(line_idx) if self.overlay == Some(Overlay::EventsFilter) => {
                self.get_previous_event_line_by_filter(line_idx)
            }
            Some(line_idx) => self.get_previous_event_line(line_idx),
            None => None,
        };
        if let Some(prev_event_line) = prev_line {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(prev_event_line, all_lines) {
                self.viewport.push_history(prev_event_line);
                self.viewport.goto_line(viewport_idx, false);
            }
        }
    }

    pub fn select_to_event_next(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(next_event_line) = self.get_next_event_line(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(next_event_line, all_lines) {
                self.viewport.goto_line(viewport_idx, false);
                self.update_selection_end();
            }
        }
    }

    pub fn select_to_event_previous(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(prev_event_line) = self.get_previous_event_line(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(prev_event_line, all_lines) {
                self.viewport.goto_line(viewport_idx, false);
                self.update_selection_end();
            }
        }
    }

    pub fn select_to_mark_next(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(next_mark_line) = self.get_next_mark(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(next_mark_line, all_lines) {
                self.viewport.goto_line(viewport_idx, false);
                self.update_selection_end();
            }
        }
    }

    pub fn select_to_mark_previous(&mut self) {
        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(prev_mark_line) = self.get_previous_mark(line_index)
        {
            let all_lines = self.log_buffer.all_lines();
            if let Some(viewport_idx) = self.resolver.log_to_viewport(prev_mark_line, all_lines) {
                self.viewport.goto_line(viewport_idx, false);
                self.update_selection_end();
            }
        }
    }

    /// Helper to go to a log line by its log line index. If the line is not visible, it does nothing.
    pub fn goto_line(&mut self, log_index: usize, center: bool) {
        let all_lines = self.log_buffer.all_lines();
        if let Some(viewport_idx) = self.resolver.log_to_viewport(log_index, all_lines) {
            self.viewport.goto_line(viewport_idx, center);
        }
    }

    /// Helper to record a viewport line in history by converting from viewport index to log index.
    fn push_viewport_line_to_history(&mut self, viewport_line: usize) {
        if let Some(line_index) = self.viewport_to_log_line_index(viewport_line) {
            self.viewport.push_history(line_index);
        }
    }

    pub fn scroll_right(&mut self, small_increment: bool) {
        let (start, end) = self.viewport.visible();

        let all_lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(all_lines);

        // Calculate max length in the visible viewport range
        let max_line_length = if start < visible_lines.len() {
            let range_end = end.min(visible_lines.len());
            visible_lines[start..range_end]
                .iter()
                .map(|vl| all_lines[vl.log_index].content.len())
                .max()
                .unwrap_or(0)
        } else {
            0
        };

        if small_increment {
            self.viewport.scroll_right_small(max_line_length);
        } else {
            self.viewport.scroll_right(max_line_length);
        }
    }

    pub fn toggle_follow_mode(&mut self) {
        if self.log_buffer.streaming {
            self.viewport.follow_mode = !self.viewport.follow_mode;
            if self.viewport.follow_mode {
                self.viewport.goto_bottom();
            }
        }
    }

    pub fn toggle_pause_mode(&mut self) {
        if self.log_buffer.streaming {
            self.streaming_paused = !self.streaming_paused;
        }
    }

    pub fn toggle_center_cursor_mode(&mut self) {
        self.viewport.center_cursor_mode = !self.viewport.center_cursor_mode;
        if self.viewport.center_cursor_mode {
            self.viewport.center_selected();
        }
    }

    pub fn toggle_help(&mut self) {
        if self.help.is_visible() {
            self.help.toggle_visibility();
        } else {
            self.help.show_for_context(&self.view_state, &self.overlay);
        }
    }

    pub fn history_back(&mut self) {
        if let Some(line_index) = self.viewport.history_back() {
            self.goto_line(line_index, false);
        }
        self.viewport.follow_mode = false;
    }

    pub fn history_forward(&mut self) {
        if let Some(line_index) = self.viewport.history_forward() {
            self.goto_line(line_index, false);
        }
        self.viewport.follow_mode = false;
    }

    pub fn clear_log_buffer(&mut self) {
        if self.log_buffer.streaming {
            self.log_buffer.clear_all();
            self.marking.clear_all();
            self.event_tracker.clear_all();
            self.highlighter.invalidate_cache();
            self.viewport.reset_view();
            self.update_view();
        }
    }

    pub fn clear_all_marks(&mut self) {
        self.marking.clear_all();

        if self.show_marked_lines_only {
            self.update_view();
        } else {
            let marked_indices = self.marking.get_marked_indices();
            self.resolver.update_mark_tags(&marked_indices);
        }
    }

    pub fn toggle_filter_pattern_active(&mut self) {
        let selected_index = self.filter_list_state.selected_index();
        self.filter.toggle_pattern_enabled(selected_index);
        self.expansion.clear();
        self.update_view();
    }

    pub fn remove_filter_pattern(&mut self) {
        let selected_index = self.filter_list_state.selected_index();
        self.filter.remove_pattern(selected_index);
        self.filter_list_state.set_item_count(self.filter.count());
        self.expansion.clear();
        self.update_view();
    }

    pub fn toggle_filter_pattern_case_sensitive(&mut self) {
        let selected_index = self.filter_list_state.selected_index();
        self.filter.toggle_pattern_case_sensitivity(selected_index);
        self.expansion.clear();
        self.update_view();
    }

    pub fn toggle_filter_pattern_mode(&mut self) {
        let selected_index = self.filter_list_state.selected_index();
        self.filter.toggle_pattern_mode(selected_index);
        self.expansion.clear();
        self.update_view();
    }

    pub fn toggle_all_filter_patterns(&mut self) {
        self.filter.toggle_all_patterns_enabled();
        self.expansion.clear();
        self.update_view();
    }

    pub fn toggle_show_marked_only(&mut self) {
        self.show_marked_lines_only = !self.show_marked_lines_only;
        self.update_view();
    }

    pub fn toggle_event_filter(&mut self) {
        let selected_index = self.event_filter_list_state.selected_index();
        let event_stats = self.event_tracker.get_event_stats();

        if let Some(event_stat) = event_stats.get(selected_index) {
            self.event_tracker.toggle_event_enabled(&event_stat.name);
            self.update_events_view_count();

            if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
                && let Some(nearest_index) = self.find_nearest_event(line_index)
            {
                self.events_list_state.select_index(nearest_index);
            }
        }
    }

    pub fn toggle_all_event_filters(&mut self) {
        self.event_tracker.toggle_all_filters();
        self.update_events_view_count();

        if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
            && let Some(nearest_index) = self.find_nearest_event(line_index)
        {
            self.events_list_state.select_index(nearest_index);
        }
    }

    pub fn solo_event_filter(&mut self) {
        let selected_index = self.event_filter_list_state.selected_index();
        let event_stats = self.event_tracker.get_event_stats();

        if let Some(event_stat) = event_stats.get(selected_index) {
            self.event_tracker.solo_event_filter(&event_stat.name);
            self.update_events_view_count();

            if let Some(line_index) = self.viewport_to_log_line_index(self.viewport.selected_line)
                && let Some(nearest_index) = self.find_nearest_event(line_index)
            {
                self.events_list_state.select_index(nearest_index);
            }
        }
    }

    pub fn toggle_events_show_marks(&mut self) {
        self.event_tracker.toggle_show_marks();
        self.update_events_view_count();
    }

    fn update_events_view_count(&mut self) {
        let (events, _) = self.get_events_for_list();
        let visible_marks = self.get_visible_marks();
        let merged_items = EventMarkView::merge(&events, &visible_marks, self.event_tracker.showing_marks());
        self.events_list_state.set_item_count(merged_items.len());

        let filter_count = self.event_tracker.filter_count();
        self.event_filter_list_state.set_item_count(filter_count);
    }

    pub fn toggle_expansion(&mut self) {
        let all_lines = self.log_buffer.all_lines();

        let Some(current_log_index) = self.resolver.viewport_to_log(self.viewport.selected_line, all_lines) else {
            return;
        };

        let visible_lines = self.resolver.get_visible_lines(all_lines);
        let current_viewport_index = self.viewport.selected_line;

        // Check if the current line is an expanded line
        if let Some(current_visible_line) = visible_lines.get(current_viewport_index)
            && current_visible_line.tags.contains(&Tag::Expanded)
        {
            if let Some(parent_log_index) = self.expansion.find_parent(current_log_index) {
                self.expansion.toggle(parent_log_index, Vec::new());
                self.update_view();
            }
            return;
        }

        // If line is already expanded, collapse it
        if self.expansion.is_expanded(current_log_index) {
            self.expansion.toggle(current_log_index, Vec::new());
            self.update_view();
            return;
        }

        let next_log_index = if current_viewport_index + 1 < visible_lines.len() {
            Some(visible_lines[current_viewport_index + 1].log_index)
        } else {
            None
        };

        let hidden_indices: Vec<usize> = if let Some(next_index) = next_log_index {
            ((current_log_index + 1)..next_index).collect()
        } else {
            Vec::new()
        };

        if hidden_indices.is_empty() {
            return;
        }

        self.expansion.toggle(current_log_index, hidden_indices);
        self.update_view();
    }

    pub fn collapse_all_expansions(&mut self) {
        self.expansion.clear();
        self.update_view();
    }

    pub fn search_history_previous(&mut self) {
        if let Some(history_query) = self.search.history.previous_record().cloned() {
            self.input = Input::new(history_query);
            self.update_temporary_highlights();
        }
    }

    pub fn search_history_next(&mut self) {
        if let Some(history_query) = self.search.history.next_record().cloned() {
            self.input = Input::new(history_query);
            self.update_temporary_highlights();
        } else {
            self.input.reset();
            self.update_temporary_highlights();
        }
    }

    pub fn filter_history_previous(&mut self) {
        if let Some(history_entry) = self.filter.history.previous_record().cloned() {
            self.input = Input::new(history_entry.pattern);
            self.filter.set_mode(history_entry.mode);
            self.filter.set_case_sensitivity(history_entry.case_sensitive);
            self.update_temporary_highlights();
        }
    }

    pub fn filter_history_next(&mut self) {
        if let Some(history_entry) = self.filter.history.next_record().cloned() {
            self.input = Input::new(history_entry.pattern);
            self.filter.set_mode(history_entry.mode);
            self.filter.set_case_sensitivity(history_entry.case_sensitive);
            self.update_temporary_highlights();
        } else {
            self.input.reset();
            self.filter.reset_mode();
            self.filter.reset_case_sensitivity();
            self.update_temporary_highlights();
        }
    }

    pub fn goto_selected_event(&mut self, center: bool) {
        let (events, filtered_indices) = self.get_events_for_list();
        let visible_marks = self.get_visible_marks();
        let merged = EventMarkView::merge(&events, &visible_marks, self.event_tracker.showing_marks());
        let selected_idx = self.events_list_state.selected_index();
        let line_index = merged.get(selected_idx).map(|item| item.line_index());

        if let Some(line_index) = line_index {
            if filtered_indices.contains(&line_index) {
                self.filter.disable_all_patterns();
                self.update_view();
            }
            self.viewport.push_history(line_index);
            self.goto_line(line_index, center);
        }
    }

    pub fn goto_selected_mark(&mut self, center: bool) {
        if let Some(mark) = self.get_selected_mark() {
            let line_index = mark.line_index;
            self.viewport.push_history(line_index);
            self.goto_line(line_index, center);
        }
    }

    /// Enters selection mode and sets the start of the selection range.
    pub fn start_selection(&mut self) {
        let current_line = self.viewport.selected_line;
        self.selection_range = Some((current_line, current_line));
        self.set_view_state(ViewState::SelectionMode);
    }

    /// Updates the end of the selection range as the cursor moves.
    pub fn update_selection_end(&mut self) {
        if let Some((start, _)) = self.selection_range {
            self.selection_range = Some((start, self.viewport.selected_line));
        }
    }

    /// Cancels the current selection.
    pub fn cancel_selection(&mut self) {
        self.selection_range = None;
    }

    /// Gets the selection range, ensuring start <= end.
    pub fn get_selection_range(&self) -> Option<(usize, usize)> {
        self.selection_range
            .map(|(start, end)| if start <= end { (start, end) } else { (end, start) })
    }

    /// Copies the selected lines to the clipboard.
    pub fn copy_selection_to_clipboard(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let all_lines = self.log_buffer.all_lines();
            let lines: Vec<String> = (start..=end)
                .filter_map(|viewport_line| {
                    self.resolver
                        .viewport_to_log(viewport_line, all_lines)
                        .and_then(|log_index| self.log_buffer.get_line(log_index))
                })
                .map(|log_line| {
                    if self.file_manager.is_multi_file() {
                        if let Some(file_id) = log_line.log_file_id
                            && self.options.is_disabled(AppOption::HideFileIds)
                        {
                            format!("[{}] {}", file_id + 1, log_line.content)
                        } else {
                            log_line.content.clone()
                        }
                    } else {
                        log_line.content.clone()
                    }
                })
                .collect();

            if !lines.is_empty() {
                let content = lines.join("\n");
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => match clipboard.set_text(content) {
                        Ok(_) => {
                            let num_lines = lines.len();
                            self.selection_range = None;
                            self.show_message(
                                format!(
                                    "Copied {} line{} to clipboard",
                                    num_lines,
                                    if num_lines == 1 { "" } else { "s" }
                                )
                                .as_str(),
                            );
                        }
                        Err(e) => {
                            self.selection_range = None;
                            self.show_error(format!("Failed to copy to clipboard: {}", e).as_str());
                        }
                    },
                    Err(e) => {
                        self.selection_range = None;
                        self.show_error(format!("Failed to access clipboard: {}", e).as_str());
                    }
                }
            }
        }
    }

    /// Returns marks that are currently visible based on active filters.
    pub fn get_visible_marks(&self) -> Vec<Mark> {
        let lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(lines);
        let visible_indices: HashSet<usize> = visible_lines.iter().map(|vl| vl.log_index).collect();

        self.marking
            .get_marks()
            .iter()
            .filter(|mark| visible_indices.contains(&mark.line_index))
            .cloned()
            .collect()
    }

    /// Returns events that are currently visible based on active filters and enabled.
    pub fn get_visible_events(&self) -> Vec<LogEvent> {
        let lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(lines);
        let visible_indices: HashSet<usize> = visible_lines.iter().map(|vl| vl.log_index).collect();

        self.event_tracker
            .get_enabled_events()
            .into_iter()
            .filter(|event| visible_indices.contains(&event.line_index))
            .cloned()
            .collect()
    }

    /// Returns enabled events whose lines are NOT visible (filtered out by text filters).
    fn get_filtered_events(&self) -> Vec<LogEvent> {
        let lines = self.log_buffer.all_lines();
        let visible_lines = self.resolver.get_visible_lines(lines);
        let visible_indices: HashSet<usize> = visible_lines.iter().map(|vl| vl.log_index).collect();

        self.event_tracker
            .get_enabled_events()
            .into_iter()
            .filter(|event| !visible_indices.contains(&event.line_index))
            .cloned()
            .collect()
    }

    /// Returns all events for the events list plus a set of filtered-out line indices.
    /// When event filtering is active, includes both visible and filtered-out events.
    pub fn get_events_for_list(&self) -> (Vec<LogEvent>, HashSet<usize>) {
        let visible = self.get_visible_events();
        if self.event_tracker.has_event_filtering() {
            let filtered = self.get_filtered_events();
            let filtered_indices: HashSet<usize> = filtered.iter().map(|e| e.line_index).collect();
            let mut all = visible;
            all.extend(filtered);
            all.sort_by_key(|e| e.line_index);
            (all, filtered_indices)
        } else {
            (visible, HashSet::new())
        }
    }

    /// Gets the currently selected mark based on marking_list_state selection.
    fn get_selected_mark(&self) -> Option<Mark> {
        let marks = self.get_visible_marks();
        marks.get(self.marking_list_state.selected_index()).cloned()
    }

    /// Gets the next mark after the given line index.
    fn get_next_mark(&self, current_line_index: usize) -> Option<usize> {
        let visible_marks = self.get_visible_marks();
        visible_marks
            .iter()
            .find(|mark| mark.line_index > current_line_index)
            .map(|mark| mark.line_index)
    }

    /// Gets the previous mark before the given line index.
    fn get_previous_mark(&self, current_line_index: usize) -> Option<usize> {
        let visible_marks = self.get_visible_marks();
        visible_marks
            .iter()
            .rev()
            .find(|mark| mark.line_index < current_line_index)
            .map(|mark| mark.line_index)
    }

    /// Finds the index in the marks list that is nearest to the given line index.
    fn find_nearest_mark(&self, line_index: usize) -> Option<usize> {
        let marks = self.get_visible_marks();
        if marks.is_empty() {
            return None;
        }

        match marks.binary_search_by_key(&line_index, |m| m.line_index) {
            Ok(idx) => Some(idx),
            Err(0) => Some(0),
            Err(idx) if idx >= marks.len() => Some(marks.len() - 1),
            Err(idx) => {
                let dist_before = line_index - marks[idx - 1].line_index;
                let dist_after = marks[idx].line_index - line_index;
                Some(if dist_before <= dist_after { idx - 1 } else { idx })
            }
        }
    }

    /// Selects the nearest mark to the given line index in the marks list.
    fn select_nearest_mark(&mut self, line_index: usize) {
        if let Some(nearest_index) = self.find_nearest_mark(line_index) {
            self.marking_list_state.select_index(nearest_index);
        } else {
            self.marking_list_state.select_index(0);
        }
    }

    /// Finds the index in the filtered events list that is nearest to the given line index.
    fn find_nearest_event(&self, line_index: usize) -> Option<usize> {
        let (enabled_events, _) = self.get_events_for_list();
        if enabled_events.is_empty() {
            return None;
        }

        let mut nearest_idx = 0;
        let mut min_distance = enabled_events[0].line_index.abs_diff(line_index);

        for (idx, event) in enabled_events.iter().enumerate() {
            let distance = event.line_index.abs_diff(line_index);
            if distance < min_distance {
                min_distance = distance;
                nearest_idx = idx;
            }
        }

        Some(nearest_idx)
    }

    /// Returns the line index of the next event after the given line index.
    fn get_next_event_line(&self, line_index: usize) -> Option<usize> {
        let enabled_events = self.get_visible_events();
        enabled_events
            .iter()
            .find(|event| event.line_index > line_index)
            .map(|event| event.line_index)
    }

    /// Returns the line index of the previous event before the given line index.
    fn get_previous_event_line(&self, line_index: usize) -> Option<usize> {
        let enabled_events = self.get_visible_events();
        enabled_events
            .iter()
            .rev()
            .find(|event| event.line_index < line_index)
            .map(|event| event.line_index)
    }

    fn selected_filter_event_name(&self) -> Option<String> {
        let event_stats = self.event_tracker.get_event_stats();
        event_stats
            .get(self.event_filter_list_state.selected_index())
            .map(|es| es.name.clone())
    }

    fn get_next_event_line_by_filter(&self, line_index: usize) -> Option<usize> {
        let name = self.selected_filter_event_name()?;
        self.event_tracker
            .get_events_by_name(&name)
            .into_iter()
            .find(|e| e.line_index > line_index)
            .map(|e| e.line_index)
    }

    fn get_previous_event_line_by_filter(&self, line_index: usize) -> Option<usize> {
        let name = self.selected_filter_event_name()?;
        self.event_tracker
            .get_events_by_name(&name)
            .into_iter()
            .rev()
            .find(|e| e.line_index < line_index)
            .map(|e| e.line_index)
    }

    /// Activates the time filter mode.
    pub fn activate_time_filter_mode(&mut self) {
        if self.log_buffer.streaming {
            self.show_message("Time filter not available in streaming mode");
            return;
        }

        self.time_filter_focus = TimeFilterFocus::Start;

        if self.time_filter_input_start.value().is_empty() || self.time_filter_input_end.value().is_empty() {
            self.reset_time_filter_to_file_range();
        }

        self.set_view_state(ViewState::TimeFilterView);
    }

    /// Resets the time filter inputs to the file's time range and clears any active filter.
    pub fn reset_time_filter(&mut self) {
        self.reset_time_filter_to_file_range();
        self.time_filter = None;
        self.update_view();
    }

    fn reset_time_filter_to_file_range(&mut self) {
        let format = "%Y-%m-%d %H:%M:%S";
        if let Some((start, end)) = self.file_time_range {
            self.time_filter_input_start = Input::new(start.format(format).to_string());
            self.time_filter_input_end = Input::new(end.format(format).to_string());
        } else {
            self.time_filter_input_start = Input::default();
            self.time_filter_input_end = Input::default();
        }
    }

    /// Edit time filters
    pub fn edit_time_filter(&mut self) {
        let current_value = match self.time_filter_focus {
            TimeFilterFocus::Start => self.time_filter_input_start.value(),
            TimeFilterFocus::End => self.time_filter_input_end.value(),
        };
        self.input = Input::new(current_value.to_string());
        self.show_overlay(Overlay::EditTimeFilter);
    }

    /// Clears the time filter.
    pub fn clear_time_filter(&mut self) {
        self.time_filter = None;
        self.update_view();
    }

    /// Switches focus between start and end input fields.
    pub fn time_filter_focus_next(&mut self) {
        self.time_filter_focus = self.time_filter_focus.next();
    }

    fn parse_timestamp(s: &str) -> Option<DateTime<Utc>> {
        let formats = ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M", "%Y-%m-%d"];

        for format in &formats {
            if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, format) {
                return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
            }
        }

        if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
            return Some(DateTime::from_naive_utc_and_offset(
                naive_date.and_hms_opt(0, 0, 0)?,
                Utc,
            ));
        }

        None
    }

    /// Verifies both time filter inputs and applies the filter if valid.
    fn verify_time_filter_input(&mut self) -> bool {
        let start_str = self.time_filter_input_start.value();
        let end_str = self.time_filter_input_end.value();

        match (Self::parse_timestamp(start_str), Self::parse_timestamp(end_str)) {
            (Some(start), Some(end)) => {
                if start > end {
                    self.show_message("Start time must be before end time");
                    return false;
                }
                self.time_filter = Some(TimeFilter::new(start, end));
                self.close_overlay();
                self.update_view();
                true
            }
            (None, _) => {
                self.show_message("Invalid start timestamp format");
                false
            }
            (_, None) => {
                self.show_message("Invalid end timestamp format");
                false
            }
        }
    }

    /// Gets the current input for time filter (based on focus).
    pub fn get_active_time_filter_input(&self) -> &Input {
        match self.time_filter_focus {
            TimeFilterFocus::Start => &self.time_filter_input_start,
            TimeFilterFocus::End => &self.time_filter_input_end,
        }
    }

    /// Gets mutable reference to the current input for time filter.
    pub fn get_active_time_filter_input_mut(&mut self) -> &mut Input {
        match self.time_filter_focus {
            TimeFilterFocus::Start => &mut self.time_filter_input_start,
            TimeFilterFocus::End => &mut self.time_filter_input_end,
        }
    }
}
