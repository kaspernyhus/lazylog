use crate::{
    cli::Cli,
    config::Config,
    display_options::DisplayOptions,
    event::{AppEvent, Event, EventHandler},
    filter::Filter,
    help::Help,
    highlighter::{Highlighter, PatternStyle},
    log::{Interval, LogBuffer},
    log_event::LogEventTracker,
    search::Search,
    viewport::Viewport,
};
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    style::Color,
};

#[derive(Debug, PartialEq)]
pub enum AppState {
    /// Normal mode for viewing logs.
    LogView,
    /// Active search mode where a search query is highlighted and can be navigated.
    SearchMode,
    /// Active goto line mode where the user can input a line number to jump to.
    GotoLineMode,
    /// Active filter mode where the user can input a filter pattern to filter log lines.
    FilterMode,
    /// View for managing existing filter patterns.
    FilterListView,
    /// Edit an existing filter pattern.
    EditFilterMode,
    /// View for adjusting display options.
    OptionsView,
    /// View for displaying all events found in the log.
    EventsView,
    /// View for filtering events in EventsView.
    EventsFilterView,
    /// Active mode for entering a file name for saving the current log buffer to a file.
    SaveToFileMode,
    /// Display a message to the user.
    Message(String),
    /// Display an error message to the user.
    ErrorState(String),
}

/// Application.
#[derive(Debug)]
pub struct App {
    /// Indicates whether the application is running.
    pub running: bool,
    /// Application configuration.
    pub config: Config,
    /// Current state of the application.
    pub app_state: AppState,
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
    /// Syntax highlighter.
    pub highlighter: Highlighter,
    /// Display options state.
    pub display_options: DisplayOptions,
    /// Current user input query (for search, filter, goto line, etc.).
    pub input_query: String,
    /// Indicates whether streaming is paused (only relevant in stdin/streaming mode).
    pub streaming_paused: bool,
    /// Log event tracker for managing log events.
    pub event_tracker: LogEventTracker,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let use_stdin = args.should_use_stdin();

        let events = EventHandler::new(use_stdin);

        let config = Config::load(&args.config);
        let highlighter = config.build_highlighter();
        let filter_patterns = config.parse_filter_patterns();

        let mut app = Self {
            running: true,
            config,
            help: Help::new(),
            app_state: AppState::LogView,
            events,
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input_query: String::new(),
            search: Search::default(),
            filter: Filter::with_patterns(filter_patterns),
            display_options: DisplayOptions::default(),
            highlighter,
            streaming_paused: false,
            event_tracker: LogEventTracker::default(),
        };

        if use_stdin {
            app.log_buffer.init_stdin_mode();
            app.viewport.follow_mode = true;
            app.update_view();
            return app;
        }

        if let Some(file_path) = args.file {
            match app.log_buffer.load_from_file(file_path.as_str()) {
                Ok(_) => app.update_view(),
                Err(e) => {
                    app.app_state = AppState::ErrorState(format!(
                        "Failed to load file: {}\nError: {}",
                        file_path, e
                    ))
                }
            }
        }

        app
    }

    fn update_view(&mut self) {
        let log_line_index = self
            .log_buffer
            .get_log_line_index(self.viewport.selected_line);

        self.log_buffer.apply_filters(&self.filter);
        let num_lines = self.log_buffer.get_lines_count();

        self.viewport.set_total_lines(num_lines);

        // Update search matches if there's an active search
        if let Some(pattern) = self.search.get_active_pattern().map(|p| p.to_string()) {
            let lines = self
                .log_buffer
                .get_lines_iter(Interval::All)
                .map(|log_line| log_line.content());
            self.search.update_matches(&pattern, lines);
        }

        if num_lines == 0 {
            self.viewport.selected_line = 0;
            return;
        }

        if self.log_buffer.streaming && self.viewport.follow_mode {
            self.viewport.goto_bottom();
        } else {
            let new_selected_line = if let Some(target_log_line_index) = log_line_index {
                self.log_buffer
                    .find_closest_line_by_index(target_log_line_index)
                    .unwrap_or_else(|| self.viewport.selected_line.min(num_lines - 1))
            } else {
                self.viewport.selected_line.min(num_lines - 1)
            };

            self.viewport.goto_line(new_selected_line, false);
        }
    }

    fn next_state(&mut self, state: AppState) {
        self.app_state = state;
    }

    fn update_temporary_highlights(&mut self) {
        self.highlighter.clear_temporary_highlights();

        // Add filter mode preview highlight
        if (self.app_state == AppState::FilterMode || self.app_state == AppState::EditFilterMode)
            && self.input_query.len() >= 2
        {
            self.highlighter.add_temporary_highlight(
                self.input_query.clone(),
                PatternStyle::new(Some(Color::Black), Some(Color::Cyan), false),
                self.filter.is_case_sensitive(),
            );
        }

        // Add search mode preview highlight
        if self.app_state == AppState::SearchMode && self.input_query.len() >= 2 {
            self.highlighter.add_temporary_highlight(
                self.input_query.clone(),
                PatternStyle::new(Some(Color::Black), Some(Color::Yellow), false),
                self.search.is_case_sensitive(),
            );
        }

        // Add active search highlight
        if let Some(pattern) = self.search.get_active_pattern() {
            if !pattern.is_empty() && self.app_state != AppState::SearchMode {
                self.highlighter.add_temporary_highlight(
                    pattern.to_string(),
                    PatternStyle::new(Some(Color::Black), Some(Color::Yellow), false),
                    self.search.is_case_sensitive(),
                );
            }
        }
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
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    crossterm::event::Event::Key(key_event) => self.handle_key_events(key_event)?,
                    crossterm::event::Event::Resize(x, y) => {
                        self.viewport
                            .resize(x.saturating_sub(1) as usize, y.saturating_sub(2) as usize);
                    }
                    _ => {}
                },
                Event::App(app_event) => self.handle_app_event(app_event)?,
            }
        }
        Ok(())
    }

    /// Handles application events and updates the state of [`App`].
    fn handle_app_event(&mut self, app_event: AppEvent) -> color_eyre::Result<()> {
        match app_event {
            AppEvent::Quit => self.quit(),
            AppEvent::Confirm => match self.app_state {
                AppState::SearchMode => {
                    if self.input_query.is_empty() {
                        self.search.clear_matches();
                    } else {
                        let lines = self
                            .log_buffer
                            .get_lines_iter(Interval::All)
                            .map(|log_line| log_line.content());
                        self.search.apply_pattern(&self.input_query, lines);
                        if let Some(line) =
                            self.search.first_match_from(self.viewport.selected_line)
                        {
                            self.viewport.goto_line(line, true);
                        }
                        self.viewport.follow_mode = false;
                    }
                    self.next_state(AppState::LogView);
                }
                AppState::FilterMode => {
                    if !self.input_query.is_empty() {
                        self.filter.add_filter(self.input_query.clone());
                        self.update_view();
                    }
                    self.next_state(AppState::LogView);
                }
                AppState::EventsView => {
                    if let Some(selected_event) = self.event_tracker.get_selected_event() {
                        let target_line = selected_event.line_index;
                        if let Some(active_line) =
                            self.log_buffer.find_closest_line_by_index(target_line)
                        {
                            self.viewport.goto_line(active_line, true);
                        }
                    }
                    self.next_state(AppState::LogView);
                }
                AppState::GotoLineMode => {
                    if let Ok(line_number) = self.input_query.parse::<usize>() {
                        if line_number > 0 && line_number <= self.log_buffer.lines.len() {
                            self.viewport.goto_line(line_number - 1, true);
                        }
                    }
                    self.next_state(AppState::LogView);
                }
                AppState::SaveToFileMode => {
                    if !self.input_query.is_empty() {
                        match self.log_buffer.save_to_file(&self.input_query) {
                            Ok(_) => {
                                let abs_path = std::fs::canonicalize(&self.input_query)
                                    .map(|p| p.to_string_lossy().to_string())
                                    .unwrap_or_else(|_| self.input_query.clone());
                                self.next_state(AppState::Message(format!(
                                    "Log saved to file:\n{}",
                                    abs_path
                                )));
                            }
                            Err(e) => {
                                self.next_state(AppState::ErrorState(format!(
                                    "Failed to save file:\n{}",
                                    e
                                )));
                            }
                        }
                    } else {
                        self.next_state(AppState::LogView);
                    }
                }
                AppState::EditFilterMode => {
                    if !self.input_query.is_empty() {
                        self.filter
                            .update_selected_pattern(self.input_query.clone());
                        self.update_view();
                    }
                    self.next_state(AppState::FilterListView);
                }
                _ => {}
            },
            AppEvent::Cancel => {
                if self.help.is_visible() {
                    self.help.toggle_visibility();
                    return Ok(());
                }
                match self.app_state {
                    AppState::SearchMode => {
                        self.search.clear_matches();
                        self.next_state(AppState::LogView);
                    }
                    AppState::GotoLineMode => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::FilterMode => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::LogView => {
                        self.search.clear_matches();
                    }
                    AppState::FilterListView => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::OptionsView => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::EventsView => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::EventsFilterView => {
                        self.next_state(AppState::EventsView);
                    }
                    AppState::SaveToFileMode => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::EditFilterMode => {
                        self.next_state(AppState::FilterListView);
                    }
                    AppState::Message(_) => {
                        self.next_state(AppState::LogView);
                    }
                    AppState::ErrorState(_) => {}
                };
            }
            AppEvent::MoveUp => {
                if self.help.is_visible() {
                    self.help.move_up();
                } else if self.app_state == AppState::FilterListView {
                    self.filter.move_selection_up();
                } else if self.app_state == AppState::OptionsView {
                    self.display_options.move_selection_up();
                } else if self.app_state == AppState::EventsView {
                    self.event_tracker.move_selection_up();
                } else if self.app_state == AppState::EventsFilterView {
                    self.event_tracker.move_filter_selection_up();
                } else {
                    self.viewport.move_up();
                    self.viewport.follow_mode = false;
                }
            }
            AppEvent::MoveDown => {
                if self.help.is_visible() {
                    self.help.move_down();
                } else if self.app_state == AppState::FilterListView {
                    self.filter.move_selection_down();
                } else if self.app_state == AppState::OptionsView {
                    self.display_options.move_selection_down();
                } else if self.app_state == AppState::EventsView {
                    self.event_tracker.move_selection_down();
                } else if self.app_state == AppState::EventsFilterView {
                    self.event_tracker.move_filter_selection_down();
                } else {
                    self.viewport.move_down();
                }
            }
            AppEvent::PageUp => self.viewport.page_up(),
            AppEvent::PageDown => self.viewport.page_down(),
            AppEvent::CenterSelected => self.viewport.center_selected(),
            AppEvent::GotoTop => self.viewport.goto_top(),
            AppEvent::GotoBottom => self.viewport.goto_bottom(),
            AppEvent::ScrollLeft => self.viewport.scroll_left(),
            AppEvent::ScrollRight => {
                let (start, end) = self.viewport.visible();
                let max_line_length = self
                    .log_buffer
                    .get_lines_max_length(Interval::Range(start, end));
                self.viewport.scroll_right(max_line_length)
            }
            AppEvent::ResetHorizontal => self.viewport.reset_horizontal(),
            AppEvent::ToggleHelp => {
                self.help.toggle_visibility();
            }
            AppEvent::ActivateSearchMode => {
                self.input_query.clear();
                self.search.clear_matches();
                self.search.reset_case_sensitive();
                self.search.history.reset();
                self.next_state(AppState::SearchMode);
            }
            AppEvent::ToggleCaseSensitive => {
                self.search.toggle_case_sensitive();
                self.filter.toggle_case_sensitive();
                if !self.input_query.is_empty() && self.app_state == AppState::SearchMode {
                    let lines = self
                        .log_buffer
                        .get_lines_iter(Interval::All)
                        .map(|log_line| log_line.content());
                    self.search.update_matches(&self.input_query, lines);
                }
            }
            AppEvent::ActivateGotoLineMode => {
                self.input_query.clear();
                self.next_state(AppState::GotoLineMode);
            }
            AppEvent::SearchNext => {
                if let Some(line) = self.search.next_match(self.viewport.selected_line) {
                    self.viewport.goto_line(line, false);
                }
            }
            AppEvent::SearchPrevious => {
                if let Some(line) = self.search.previous_match(self.viewport.selected_line) {
                    self.viewport.goto_line(line, false);
                }
            }
            AppEvent::GotoLine(line_index) => {
                if let Some(active_line) = self.log_buffer.find_closest_line_by_index(line_index) {
                    self.viewport.goto_line(active_line, true);
                }
            }
            AppEvent::ActivateFilterMode => {
                self.input_query.clear();
                self.filter.reset_mode();
                self.filter.reset_case_sensitive();
                self.next_state(AppState::FilterMode);
            }
            AppEvent::ToggleFilterMode => {
                self.filter.toggle_mode();
            }
            AppEvent::ActivateFilterListView => {
                self.next_state(AppState::FilterListView);
            }
            AppEvent::ToggleFilterPatternActive => {
                self.filter.toggle_selected_pattern();
                self.update_view();
            }
            AppEvent::RemoveFilterPattern => {
                self.filter.remove_selected_pattern();
                self.update_view();
            }
            AppEvent::ToggleFilterPatternCaseSensitive => {
                self.filter.toggle_selected_pattern_case_sensitive();
                self.update_view();
            }
            AppEvent::ToggleFilterPatternMode => {
                self.filter.toggle_selected_pattern_mode();
                self.update_view();
            }
            AppEvent::ToggleAllFilterPatterns => {
                self.filter.toggle_all_patterns();
                self.update_view();
            }
            AppEvent::ActivateEditFilterMode => {
                if let Some(pattern) = self.filter.get_selected_pattern() {
                    self.input_query = pattern.pattern.clone();
                    self.next_state(AppState::EditFilterMode);
                }
            }
            AppEvent::ActivateAddFilterMode => {
                self.input_query.clear();
                self.next_state(AppState::FilterMode);
            }
            AppEvent::SearchHistoryPrevious => {
                if let Some(history_query) = self.search.history.previous_query() {
                    self.input_query = history_query;
                }
            }
            AppEvent::SearchHistoryNext => {
                if let Some(history_query) = self.search.history.next_query() {
                    self.input_query = history_query;
                }
            }
            AppEvent::ToggleFollowMode => {
                if self.log_buffer.streaming {
                    self.viewport.follow_mode = !self.viewport.follow_mode;
                    if self.viewport.follow_mode {
                        self.viewport.goto_bottom();
                    }
                }
            }
            AppEvent::TogglePauseMode => {
                if self.log_buffer.streaming {
                    self.streaming_paused = !self.streaming_paused;
                }
            }
            AppEvent::ToggleCenterCursorMode => {
                self.viewport.center_cursor_mode = !self.viewport.center_cursor_mode;
                if self.viewport.center_cursor_mode {
                    self.viewport.center_selected();
                }
            }
            AppEvent::ActivateOptionsView => {
                self.next_state(AppState::OptionsView);
            }
            AppEvent::ActivateEventsView => {
                self.event_tracker
                    .scan(&self.log_buffer, self.highlighter.events());
                self.event_tracker
                    .select_nearest_event(self.viewport.selected_line);
                self.next_state(AppState::EventsView);
            }
            AppEvent::ActivateEventFilterView => {
                if self.app_state == AppState::EventsView {
                    self.next_state(AppState::EventsFilterView);
                }
            }
            AppEvent::ToggleEventFilter => {
                self.event_tracker.toggle_selected_filter();
                self.event_tracker
                    .scan(&self.log_buffer, self.highlighter.events());
                self.event_tracker
                    .select_nearest_event(self.viewport.selected_line);
            }
            AppEvent::ToggleAllEventFilters => {
                self.event_tracker.toggle_all_filters();
                self.event_tracker
                    .scan(&self.log_buffer, self.highlighter.events());
                self.event_tracker
                    .select_nearest_event(self.viewport.selected_line);
            }
            AppEvent::ToggleDisplayOption => {
                self.display_options.toggle_selected_option();
            }
            AppEvent::ActivateSaveToFileMode => {
                self.input_query.clear();
                self.next_state(AppState::SaveToFileMode);
            }
            AppEvent::ClearLogBuffer => {
                if self.log_buffer.streaming {
                    self.log_buffer.clear_all();
                    self.viewport.set_total_lines(0);
                    self.viewport.selected_line = 0;
                }
            }
            AppEvent::NewLine(line) => {
                if !self.streaming_paused {
                    let passes_filter = self.log_buffer.append_line(line, &self.filter);
                    if passes_filter {
                        let num_lines = self.log_buffer.get_lines_count();
                        self.viewport.set_total_lines(num_lines);

                        // Update search matches if there's an active search
                        if let Some(pattern) =
                            self.search.get_active_pattern().map(|p| p.to_string())
                        {
                            let lines = self
                                .log_buffer
                                .get_lines_iter(Interval::All)
                                .map(|log_line| log_line.content());
                            self.search.update_matches(&pattern, lines);
                        }

                        if self.viewport.follow_mode {
                            self.viewport.goto_bottom();
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        // Global keybindings
        match key_event.code {
            KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Char('l') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::ClearLogBuffer)
            }
            KeyCode::Char('s') if key_event.modifiers == KeyModifiers::CONTROL => {
                if self.log_buffer.streaming {
                    self.events.send(AppEvent::ActivateSaveToFileMode)
                }
            }
            KeyCode::Esc => self.events.send(AppEvent::Cancel),
            KeyCode::Enter => self.events.send(AppEvent::Confirm),
            _ => {}
        }

        match self.app_state {
            AppState::ErrorState(_) => {
                if let KeyCode::Char('q') = key_event.code {
                    self.events.send(AppEvent::Quit);
                }
            }

            // LogView (Normal Mode)
            AppState::LogView => match key_event.code {
                KeyCode::Up => self.events.send(AppEvent::MoveUp),
                KeyCode::Down => self.events.send(AppEvent::MoveDown),
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                KeyCode::PageUp => self.events.send(AppEvent::PageUp),
                KeyCode::PageDown => self.events.send(AppEvent::PageDown),
                KeyCode::Char('z') => self.events.send(AppEvent::CenterSelected),
                KeyCode::Char('g') => self.events.send(AppEvent::GotoTop),
                KeyCode::Char('G') => self.events.send(AppEvent::GotoBottom),
                KeyCode::Left => self.events.send(AppEvent::ScrollLeft),
                KeyCode::Right => self.events.send(AppEvent::ScrollRight),
                KeyCode::Char('0') => self.events.send(AppEvent::ResetHorizontal),
                KeyCode::Char('/') => self.events.send(AppEvent::ActivateSearchMode),
                KeyCode::Char('f') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::ActivateSearchMode)
                }
                KeyCode::Char('F') => self.events.send(AppEvent::ActivateFilterListView),
                KeyCode::Char(':') => self.events.send(AppEvent::ActivateGotoLineMode),
                KeyCode::Char('n') => self.events.send(AppEvent::SearchNext),
                KeyCode::Char('N') => self.events.send(AppEvent::SearchPrevious),
                KeyCode::Char('f') => self.events.send(AppEvent::ActivateFilterMode),
                KeyCode::Char('t') => self.events.send(AppEvent::ToggleFollowMode),
                KeyCode::Char('p') => self.events.send(AppEvent::TogglePauseMode),
                KeyCode::Char('c') => self.events.send(AppEvent::ToggleCenterCursorMode),
                KeyCode::Char('o') => self.events.send(AppEvent::ActivateOptionsView),
                KeyCode::Char('e') => self.events.send(AppEvent::ActivateEventsView),
                _ => {}
            },

            // SearchMode
            AppState::SearchMode => match key_event.code {
                KeyCode::Tab => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Up => self.events.send(AppEvent::SearchHistoryPrevious),
                KeyCode::Down => self.events.send(AppEvent::SearchHistoryNext),
                KeyCode::Backspace => {
                    self.input_query.pop();
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                }
                _ => {}
            },

            // FilterMode
            AppState::FilterMode => match key_event.code {
                KeyCode::Tab => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Left => self.events.send(AppEvent::ToggleFilterMode),
                KeyCode::Right => self.events.send(AppEvent::ToggleFilterMode),
                KeyCode::Delete => self.events.send(AppEvent::RemoveFilterPattern),
                KeyCode::Backspace => {
                    self.input_query.pop();
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                }
                _ => {}
            },

            // FilterListView
            AppState::FilterListView => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                KeyCode::Up => self.events.send(AppEvent::MoveUp),
                KeyCode::Down => self.events.send(AppEvent::MoveDown),
                KeyCode::Char(' ') => self.events.send(AppEvent::ToggleFilterPatternActive),
                KeyCode::Delete => self.events.send(AppEvent::RemoveFilterPattern),
                KeyCode::Char('e') => self.events.send(AppEvent::ActivateEditFilterMode),
                KeyCode::Char('f') => self.events.send(AppEvent::ActivateAddFilterMode),
                KeyCode::Char('a') => self.events.send(AppEvent::ToggleAllFilterPatterns),
                KeyCode::Tab => self.events.send(AppEvent::ToggleFilterPatternCaseSensitive),
                KeyCode::Char('m') => self.events.send(AppEvent::ToggleFilterPatternMode),
                _ => {}
            },

            // OptionsView
            AppState::OptionsView => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                KeyCode::Up => self.events.send(AppEvent::MoveUp),
                KeyCode::Down => self.events.send(AppEvent::MoveDown),
                KeyCode::Char(' ') => self.events.send(AppEvent::ToggleDisplayOption),
                _ => {}
            },

            // EventsView
            AppState::EventsView => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                KeyCode::Char('F') => self.events.send(AppEvent::ActivateEventFilterView),
                KeyCode::Up => self.events.send(AppEvent::MoveUp),
                KeyCode::Down => self.events.send(AppEvent::MoveDown),
                KeyCode::Char(' ') => {
                    if let Some(event) = self.event_tracker.get_selected_event() {
                        self.events.send(AppEvent::GotoLine(event.line_index));
                    };
                }
                _ => {}
            },

            // EventFilterView
            AppState::EventsFilterView => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                KeyCode::Up => self.events.send(AppEvent::MoveUp),
                KeyCode::Down => self.events.send(AppEvent::MoveDown),
                KeyCode::Char(' ') => self.events.send(AppEvent::ToggleEventFilter),
                KeyCode::Char('a') => self.events.send(AppEvent::ToggleAllEventFilters),
                _ => {}
            },

            // GotoLineMode
            AppState::GotoLineMode => match key_event.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.input_query.push(c);
                }
                KeyCode::Backspace => {
                    self.input_query.pop();
                }
                _ => {}
            },

            // SaveToFileMode
            AppState::SaveToFileMode => match key_event.code {
                KeyCode::Backspace => {
                    self.input_query.pop();
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                }
                _ => {}
            },

            // EditFilterMode
            AppState::EditFilterMode => match key_event.code {
                KeyCode::Backspace => {
                    self.input_query.pop();
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                }
                _ => {}
            },

            // Message
            AppState::Message(_) => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                _ => {}
            },
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {
        self.update_temporary_highlights()
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
