use crate::{
    cli::Cli,
    config::Config,
    display_options::DisplayOptions,
    event::{AppEvent, Event, EventHandler},
    filter::Filter,
    help::Help,
    highlighter::{Highlighter, PatternStyle},
    keybindings::KeybindingRegistry,
    log::{Interval, LogBuffer},
    log_event::LogEventTracker,
    marking::Marking,
    search::Search,
    viewport::Viewport,
};
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{KeyCode, KeyEvent},
    style::Color,
};

#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// View for displaying marked log lines.
    MarksView,
    /// Active mode for entering a name/tag for a mark.
    MarkNameInputMode,
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
    /// Log line marking manager
    pub marking: Marking,
    /// Keybinding registry for all keybindings.
    keybindings: KeybindingRegistry,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let use_stdin = args.should_use_stdin();

        let events = EventHandler::new(use_stdin);

        let config = Config::load(&args.config);
        let highlighter = config.build_highlighter();
        let filter_patterns = config.parse_filter_patterns();

        let keybindings = KeybindingRegistry::new();
        let mut help = Help::new();
        help.build_from_registry(&keybindings);

        let mut app = Self {
            running: true,
            config,
            help,
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
            marking: Marking::default(),
            keybindings,
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

    /// Handles application events and updates the state of [`App`].
    fn handle_app_event(&mut self, app_event: AppEvent) -> color_eyre::Result<()> {
        match app_event {
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
        if self.is_text_input_mode() {
            self.handle_text_input(key_event);
        }

        if let Some(command) = self.keybindings.lookup(&self.app_state, key_event) {
            command.execute(self)?;
        }

        Ok(())
    }

    /// Checks if the current state is a text input mode.
    fn is_text_input_mode(&self) -> bool {
        matches!(
            self.app_state,
            AppState::SearchMode
                | AppState::FilterMode
                | AppState::GotoLineMode
                | AppState::SaveToFileMode
                | AppState::EditFilterMode
                | AppState::MarkNameInputMode
        )
    }

    /// Handles text input for input modes.
    fn handle_text_input(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Backspace => {
                self.input_query.pop();
            }
            KeyCode::Char(c) => match self.app_state {
                AppState::GotoLineMode => {
                    if c.is_ascii_digit() {
                        self.input_query.push(c);
                    }
                }
                _ => {
                    self.input_query.push(c);
                }
            },
            _ => {}
        }
    }

    pub fn confirm(&mut self) {
        match self.app_state {
            AppState::SearchMode => {
                if self.input_query.is_empty() {
                    self.search.clear_matches();
                } else {
                    let lines = self
                        .log_buffer
                        .get_lines_iter(Interval::All)
                        .map(|log_line| log_line.content());
                    self.search.apply_pattern(&self.input_query, lines);
                    if let Some(line) = self.search.first_match_from(self.viewport.selected_line) {
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
            AppState::MarksView => {
                if let Some(selected_mark) = self.marking.get_selected_mark() {
                    let target_line = selected_mark.line_index;
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
            AppState::MarkNameInputMode => {
                if let Some(line_index) = self.marking.get_selected_marked_line() {
                    if !self.input_query.is_empty() {
                        self.marking
                            .set_mark_name(line_index, self.input_query.clone());
                    }
                }
                self.next_state(AppState::MarksView);
            }
            _ => {}
        }
    }

    pub fn cancel(&mut self) {
        if self.help.is_visible() {
            self.help.toggle_visibility();
            return;
        }

        match self.app_state {
            AppState::SearchMode => {
                self.search.clear_matches();
                self.next_state(AppState::LogView);
            }
            AppState::GotoLineMode | AppState::FilterMode | AppState::SaveToFileMode => {
                self.next_state(AppState::LogView);
            }
            AppState::LogView => {
                self.search.clear_matches();
            }
            AppState::FilterListView
            | AppState::OptionsView
            | AppState::EventsView
            | AppState::MarksView => {
                self.next_state(AppState::LogView);
            }
            AppState::EventsFilterView => {
                self.next_state(AppState::EventsView);
            }
            AppState::MarkNameInputMode => {
                self.next_state(AppState::MarksView);
            }
            AppState::EditFilterMode => {
                self.next_state(AppState::FilterListView);
            }
            AppState::Message(_) => {
                self.next_state(AppState::LogView);
            }
            AppState::ErrorState(_) => {}
        }
    }

    pub fn move_up(&mut self) {
        if self.help.is_visible() {
            self.help.move_up();
            return;
        }

        match self.app_state {
            AppState::FilterListView => self.filter.move_selection_up(),
            AppState::OptionsView => self.display_options.move_selection_up(),
            AppState::EventsView => self.event_tracker.move_selection_up(),
            AppState::EventsFilterView => self.event_tracker.move_filter_selection_up(),
            AppState::MarksView => self.marking.move_selection_up(),
            _ => {
                self.viewport.move_up();
                self.viewport.follow_mode = false;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.help.is_visible() {
            self.help.move_down();
            return;
        }

        match self.app_state {
            AppState::FilterListView => self.filter.move_selection_down(),
            AppState::OptionsView => self.display_options.move_selection_down(),
            AppState::EventsView => self.event_tracker.move_selection_down(),
            AppState::EventsFilterView => self.event_tracker.move_filter_selection_down(),
            AppState::MarksView => self.marking.move_selection_down(),
            _ => self.viewport.move_down(),
        }
    }

    pub fn activate_search_mode(&mut self) {
        self.input_query.clear();
        self.search.clear_matches();
        self.search.reset_case_sensitive();
        self.search.history.reset();
        self.next_state(AppState::SearchMode);
    }

    pub fn activate_goto_line_mode(&mut self) {
        self.input_query.clear();
        self.next_state(AppState::GotoLineMode);
    }

    pub fn activate_filter_mode(&mut self) {
        self.input_query.clear();
        self.filter.reset_mode();
        self.filter.reset_case_sensitive();
        self.next_state(AppState::FilterMode);
    }

    pub fn activate_filter_list_view(&mut self) {
        self.next_state(AppState::FilterListView);
    }

    pub fn activate_edit_filter_mode(&mut self) {
        if let Some(pattern) = self.filter.get_selected_pattern() {
            self.input_query = pattern.pattern.clone();
            self.next_state(AppState::EditFilterMode);
        }
    }

    pub fn activate_options_view(&mut self) {
        self.next_state(AppState::OptionsView);
    }

    pub fn activate_events_view(&mut self) {
        self.event_tracker
            .scan(&self.log_buffer, self.highlighter.events());
        self.event_tracker
            .select_nearest_event(self.viewport.selected_line);
        self.next_state(AppState::EventsView);
    }

    pub fn activate_event_filter_view(&mut self) {
        if self.app_state == AppState::EventsView {
            self.next_state(AppState::EventsFilterView);
        }
    }

    pub fn activate_marks_view(&mut self) {
        if let Some(line_index) = self
            .log_buffer
            .get_log_line_index(self.viewport.selected_line)
        {
            self.marking.select_nearest_mark(line_index);
        } else {
            self.marking.reset_selection();
        }
        self.next_state(AppState::MarksView);
    }

    pub fn activate_mark_name_input_mode(&mut self) {
        self.input_query.clear();
        self.next_state(AppState::MarkNameInputMode);
    }

    pub fn activate_save_to_file_mode(&mut self) {
        self.input_query.clear();
        self.next_state(AppState::SaveToFileMode);
    }

    pub fn toggle_mark(&mut self) {
        if let Some(line_index) = self
            .log_buffer
            .get_log_line_index(self.viewport.selected_line)
        {
            self.marking.toggle_mark(line_index);
        }
    }

    pub fn toggle_case_sensitive(&mut self) {
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

    pub fn search_next(&mut self) {
        if let Some(line) = self.search.next_match(self.viewport.selected_line) {
            self.viewport.goto_line(line, false);
        }
    }

    pub fn search_previous(&mut self) {
        if let Some(line) = self.search.previous_match(self.viewport.selected_line) {
            self.viewport.goto_line(line, false);
        }
    }

    pub fn goto_line(&mut self, line_index: usize) {
        if let Some(active_line) = self.log_buffer.find_closest_line_by_index(line_index) {
            self.viewport.goto_line(active_line, true);
        }
    }

    pub fn scroll_right(&mut self) {
        let (start, end) = self.viewport.visible();
        let max_line_length = self
            .log_buffer
            .get_lines_max_length(Interval::Range(start, end));
        self.viewport.scroll_right(max_line_length);
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

    pub fn clear_log_buffer(&mut self) {
        if self.log_buffer.streaming {
            self.log_buffer.clear_all();
            self.viewport.set_total_lines(0);
            self.viewport.selected_line = 0;
        }
    }

    pub fn toggle_filter_pattern_active(&mut self) {
        self.filter.toggle_selected_pattern();
        self.update_view();
    }

    pub fn remove_filter_pattern(&mut self) {
        self.filter.remove_selected_pattern();
        self.update_view();
    }

    pub fn toggle_filter_pattern_case_sensitive(&mut self) {
        self.filter.toggle_selected_pattern_case_sensitive();
        self.update_view();
    }

    pub fn toggle_filter_pattern_mode(&mut self) {
        self.filter.toggle_selected_pattern_mode();
        self.update_view();
    }

    pub fn toggle_all_filter_patterns(&mut self) {
        self.filter.toggle_all_patterns();
        self.update_view();
    }

    pub fn toggle_event_filter(&mut self) {
        self.event_tracker.toggle_selected_filter();
        self.event_tracker
            .scan(&self.log_buffer, self.highlighter.events());
        self.event_tracker
            .select_nearest_event(self.viewport.selected_line);
    }

    pub fn toggle_all_event_filters(&mut self) {
        self.event_tracker.toggle_all_filters();
        self.event_tracker
            .scan(&self.log_buffer, self.highlighter.events());
        self.event_tracker
            .select_nearest_event(self.viewport.selected_line);
    }

    pub fn search_history_previous(&mut self) {
        if let Some(history_query) = self.search.history.previous_query() {
            self.input_query = history_query;
        }
    }

    pub fn search_history_next(&mut self) {
        if let Some(history_query) = self.search.history.next_query() {
            self.input_query = history_query;
        }
    }

    pub fn unmark_selected(&mut self) {
        if let Some(line_index) = self.marking.get_selected_marked_line() {
            self.marking.unmark(line_index);
        }
    }

    pub fn goto_selected_event(&mut self) {
        if let Some(event) = self.event_tracker.get_selected_event() {
            self.goto_line(event.line_index);
        }
    }

    pub fn goto_selected_mark(&mut self) {
        if let Some(line_index) = self.marking.get_selected_marked_line() {
            self.goto_line(line_index);
        }
    }
}
