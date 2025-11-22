use crate::{
    cli::Cli,
    colors::{
        FILTER_MODE_BG, FILTER_MODE_FG, MARK_MODE_BG, MARK_MODE_FG, SEARCH_MODE_BG, SEARCH_MODE_FG,
    },
    completion::CompletionEngine,
    config::{Config, Filters},
    event::{AppEvent, Event, EventHandler},
    filter::{Filter, FilterMode, FilterPattern},
    help::Help,
    highlighter::{Highlighter, PatternStyle},
    keybindings::KeybindingRegistry,
    log::{Interval, LogBuffer},
    log_event::LogEventTracker,
    log_processor::ProcessingContext,
    marking::{Mark, Marking},
    options::Options,
    persistence::{PersistedState, clear_all_state, load_state, save_state},
    search::Search,
    ui::popup_area,
    viewport::Viewport,
};
use crossterm::event::Event::Key;
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{KeyCode, KeyEvent},
};
use tui_input::{Input, InputRequest, backend::crossterm::EventHandler as TuiEventHandler};

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
    /// Active mode for entering a pattern for auto creating marks.
    MarkAddInputMode,
    /// Active mode for entering a file name for saving the current log buffer to a file.
    SaveToFileMode,
    /// Visual selection mode for selecting a range of lines.
    SelectionMode,
    /// Display a message to the user.
    Message(String),
    /// Display an error message to the user.
    ErrorState(String),
}

impl AppState {
    pub fn matches(&self, other: &AppState) -> bool {
        match (self, other) {
            (AppState::Message(_), AppState::Message(_)) => true,
            (AppState::ErrorState(_), AppState::ErrorState(_)) => true,
            _ => self == other,
        }
    }
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
    pub options: Options,
    /// Text input widget.
    pub input: Input,
    /// Indicates whether streaming is paused (only relevant in stdin/streaming mode).
    pub streaming_paused: bool,
    /// Log event tracker for managing log events.
    pub event_tracker: LogEventTracker,
    /// Log line marking manager
    pub marking: Marking,
    /// Selection range for visual selection mode.
    selection_range: Option<(usize, usize)>,
    /// Timestamp when a message was shown.
    message_timestamp: Option<std::time::Instant>,
    /// Tab completion.
    completion: CompletionEngine,
    /// Keybinding registry for all keybindings.
    keybindings: KeybindingRegistry,
    /// Indicates whether the screen needs to be redrawn.
    needs_redraw: bool,
    /// Whether persistence is enabled.
    persist_enabled: bool,
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let initial_state = if args.clear_state {
            match clear_all_state() {
                Ok(msg) => AppState::Message(msg),
                Err(err) => AppState::ErrorState(err),
            }
        } else {
            AppState::LogView
        };

        let use_stdin = args.should_use_stdin();

        let events = EventHandler::new(use_stdin);

        let config = Config::load(&args.config);
        let highlighter = config.build_highlighter();

        let mut filter_patterns = config.parse_filter_patterns();
        if let Some(filters_file) = Filters::load(&args.filters) {
            filter_patterns.extend(filters_file.parse_filter_patterns());
        }

        let keybindings = KeybindingRegistry::new();
        let mut help = Help::new();
        help.build_from_registry(&keybindings);

        let mut app = Self {
            running: true,
            config,
            help,
            app_state: initial_state,
            events,
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input: Input::default(),
            search: Search::default(),
            filter: Filter::with_patterns(filter_patterns),
            options: Options::default(),
            highlighter,
            streaming_paused: false,
            event_tracker: LogEventTracker::default(),
            marking: Marking::default(),
            selection_range: None,
            message_timestamp: None,
            completion: CompletionEngine::default(),
            keybindings,
            needs_redraw: true,
            persist_enabled: !args.no_persist,
        };

        if use_stdin {
            app.log_buffer.init_stdin_mode();
            app.viewport.follow_mode = true;
            app.update_view();
            return app;
        }

        if let Some(file_path) = args.file {
            match app.log_buffer.load_from_file(file_path.as_str()) {
                Ok(_) => {
                    app.update_view();
                    app.update_completion_words();

                    if app.persist_enabled
                        && let Some(state) = load_state(&file_path)
                    {
                        app.restore_state(state);
                    }

                    app.event_tracker
                        .scan_all_lines(&app.log_buffer, app.highlighter.events());
                }
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
            .viewport_to_log_index(self.viewport.selected_line);

        let marked_indices = self.marking.get_marked_indices();
        self.log_buffer.apply_filters(&self.filter, &marked_indices);
        let num_lines = self.log_buffer.get_active_lines_count();

        self.viewport.set_total_lines(num_lines);

        self.event_tracker.mark_needs_rescan();

        // Update search matches if there's an active search
        if let Some(pattern) = self.search.get_active_pattern().map(|p| p.to_string()) {
            let lines = self
                .log_buffer
                .get_lines_iter(Interval::All)
                .map(|log_line| log_line.content());
            self.search.update_matches(&pattern, lines);
        }

        self.update_processor_context();

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

    fn update_processor_context(&self) {
        if let Some(processor) = &self.events.processor {
            let context = ProcessingContext {
                filter_patterns: self.filter.get_filter_patterns().to_vec(),
                search_pattern: self.search.get_active_pattern().map(|p| p.to_string()),
                search_case_sensitive: self.search.is_case_sensitive(),
                event_patterns: self.highlighter.events().to_vec(),
            };
            processor.update_context(context);
        }
    }

    fn next_state(&mut self, state: AppState) {
        if matches!(state, AppState::Message(_)) {
            self.message_timestamp = Some(std::time::Instant::now());
        } else {
            self.message_timestamp = None;
        }
        self.app_state = state;
        self.update_temporary_highlights();
        self.mark_dirty();
    }

    /// Marks the screen as needing a redraw.
    fn mark_dirty(&mut self) {
        self.needs_redraw = true;
    }

    fn update_completion_words(&mut self) {
        self.completion
            .update(self.log_buffer.get_lines_iter(Interval::All));
    }

    pub fn apply_tab_completion(&mut self) {
        if !matches!(self.app_state, AppState::SearchMode | AppState::FilterMode) {
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
        match self.app_state {
            AppState::SearchMode => {
                let case_sensitive = if self.search.is_case_sensitive() {
                    "Aa"
                } else {
                    "aa"
                };
                format!("Search: [{}] ", case_sensitive)
            }
            AppState::FilterMode => {
                let filter_mode = match self.filter.get_mode() {
                    FilterMode::Include => "IN",
                    FilterMode::Exclude => "EX",
                };
                let case_sensitive = if self.filter.is_case_sensitive() {
                    "Aa"
                } else {
                    "aa"
                };
                format!("Filter: [{}] [{}] ", case_sensitive, filter_mode)
            }
            AppState::GotoLineMode => "Go to line: ".to_string(),
            AppState::SaveToFileMode => "Save to file: ".to_string(),
            AppState::MarkAddInputMode => "Add mark(s) from pattern: ".to_string(),
            _ => String::new(),
        }
    }

    fn update_temporary_highlights(&mut self) {
        self.highlighter.clear_temporary_highlights();

        // Add filter mode preview highlight
        if (self.app_state == AppState::FilterMode || self.app_state == AppState::EditFilterMode)
            && self.input.value().chars().count() >= 2
        {
            self.highlighter.add_temporary_highlight(
                self.input.value().to_string(),
                PatternStyle::new(Some(FILTER_MODE_FG), Some(FILTER_MODE_BG), true),
                self.filter.is_case_sensitive(),
            );
        }

        // Add search mode preview highlight
        if self.app_state == AppState::SearchMode && self.input.value().chars().count() >= 2 {
            self.highlighter.add_temporary_highlight(
                self.input.value().to_string(),
                PatternStyle::new(Some(SEARCH_MODE_FG), Some(SEARCH_MODE_BG), true),
                self.search.is_case_sensitive(),
            );
        }

        // Add mark add mode preview highlight
        if self.app_state == AppState::MarkAddInputMode && self.input.value().chars().count() >= 2 {
            self.highlighter.add_temporary_highlight(
                self.input.value().to_string(),
                PatternStyle::new(Some(MARK_MODE_FG), Some(MARK_MODE_BG), false),
                false,
            );
        }

        // Add active search highlight
        if let Some(pattern) = self.search.get_active_pattern()
            && !pattern.is_empty()
            && self.app_state != AppState::SearchMode
        {
            self.highlighter.add_temporary_highlight(
                pattern.to_string(),
                PatternStyle::new(Some(SEARCH_MODE_FG), Some(SEARCH_MODE_BG), false),
                self.search.is_case_sensitive(),
            );
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
            if self.needs_redraw {
                terminal.draw(|frame| {
                    frame.render_widget(&self, frame.area());

                    // Set cursor position for text input modes
                    let cursor_pos = match self.app_state {
                        AppState::SearchMode
                        | AppState::FilterMode
                        | AppState::GotoLineMode
                        | AppState::MarkAddInputMode => {
                            // Footer-based input modes
                            let footer_y = frame.area().height.saturating_sub(1);
                            let prefix_width = self.get_input_prefix().len();
                            let cursor_x = (prefix_width + self.input.visual_cursor()) as u16;
                            Some((cursor_x, footer_y))
                        }
                        AppState::EditFilterMode
                        | AppState::MarkNameInputMode
                        | AppState::SaveToFileMode => {
                            // Popup-based input modes (cursor at x=1+visual_cursor, y=1, accounting for border)
                            let popup_rect = popup_area(frame.area(), 60, 3);
                            let cursor_x = popup_rect.x + 1 + self.input.visual_cursor() as u16;
                            let cursor_y = popup_rect.y + 1;
                            Some((cursor_x, cursor_y))
                        }
                        _ => None,
                    };

                    if let Some((x, y)) = cursor_pos {
                        frame.set_cursor_position((x, y));
                    }
                })?;
                self.needs_redraw = false;
            }

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    Key(key_event) => {
                        self.handle_key_events(key_event)?;
                        self.mark_dirty();
                    }
                    crossterm::event::Event::Resize(x, y) => {
                        self.viewport
                            .resize(x.saturating_sub(1) as usize, y.saturating_sub(2) as usize);
                        self.mark_dirty();
                    }
                    _ => {}
                },
                Event::App(app_event) => {
                    self.handle_app_event(app_event)?;
                    self.mark_dirty();
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
            && matches!(self.app_state, AppState::Message(_))
        {
            self.next_state(AppState::LogView);
        }
    }

    /// Set running to false to quit the application.
    ///
    /// If not in streaming mode, persist current state to disk.
    pub fn quit(&mut self) {
        if self.persist_enabled
            && !self.log_buffer.streaming
            && let Some(ref file_path) = self.log_buffer.file_path
        {
            save_state(file_path, self);
        }

        self.running = false;
    }

    /// Restores application state from a persisted state.
    fn restore_state(&mut self, state: PersistedState) {
        self.options.restore(&state.options());

        self.search.history.restore(state.search_history().to_vec());
        self.filter.history.restore(state.filter_history().to_vec());

        for filter_state in state.filters() {
            if let Some(existing) =
                self.filter.get_filter_patterns_mut().iter_mut().find(|fp| {
                    fp.pattern == filter_state.pattern() && fp.mode == filter_state.mode()
                })
            {
                existing.case_sensitive = filter_state.case_sensitive();
                existing.enabled = filter_state.enabled();
            } else {
                let filter_pattern = FilterPattern::new(
                    filter_state.pattern().to_string(),
                    filter_state.mode(),
                    filter_state.case_sensitive(),
                );
                let mut pattern = filter_pattern;
                pattern.enabled = filter_state.enabled();
                self.filter.get_filter_patterns_mut().push(pattern);
            }
        }

        if !state.filters().is_empty() {
            self.update_view();
        }

        for mark_state in state.marks() {
            let line_index = mark_state.line_index();
            if line_index < self.log_buffer.get_total_lines_count() {
                self.marking.toggle_mark(line_index);
                if let Some(name) = mark_state.name() {
                    self.marking.set_mark_name(line_index, name.clone());
                }
            }
        }

        let marks = self.marking.get_marks();
        self.event_tracker.set_marks(&marks);

        let event_filter_states: Vec<(String, bool)> = state
            .event_filters()
            .iter()
            .map(|ef| (ef.name().to_string(), ef.enabled()))
            .collect();

        self.event_tracker
            .restore_filter_states(&event_filter_states);

        let filtered_lines = self.log_buffer.get_active_lines_count();
        if filtered_lines > 0 {
            self.viewport.selected_line = state.viewport_selected_line().min(filtered_lines - 1);
            self.viewport.top_line = state
                .viewport_top_line()
                .min(filtered_lines.saturating_sub(self.viewport.height));
            self.viewport.horizontal_offset = state.viewport_horizontal_offset();
        }

        self.viewport.center_cursor_mode = state.viewport_center_cursor_mode();

        self.update_temporary_highlights();
    }

    /// Handles application events and updates the state of [`App`].
    fn handle_app_event(&mut self, app_event: AppEvent) -> color_eyre::Result<()> {
        match app_event {
            AppEvent::NewLines(processed_lines) => {
                if self.streaming_paused {
                    return Ok(());
                }

                for pl in processed_lines {
                    let log_line_index = self.log_buffer.append_line(pl.line_content);

                    if pl.passes_filter {
                        self.log_buffer.add_to_active_lines(log_line_index);

                        let log_line = self.log_buffer.get_line(log_line_index).unwrap();
                        self.event_tracker.scan_single_line(
                            log_line,
                            self.highlighter.events(),
                            self.viewport.follow_mode,
                        );
                        self.completion.append_line(log_line);
                        self.search.append_line(log_line_index, log_line.content());
                    }
                }

                self.viewport
                    .set_total_lines(self.log_buffer.get_active_lines_count());

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
                | AppState::MarkAddInputMode
        )
    }

    /// Handles text input for input modes.
    fn handle_text_input(&mut self, key_event: KeyEvent) {
        if self.app_state == AppState::GotoLineMode {
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
        match self.app_state {
            AppState::SearchMode => {
                if self.input.value().is_empty() {
                    self.search.clear_matches();
                } else {
                    let lines = self
                        .log_buffer
                        .get_lines_iter(Interval::All)
                        .map(|log_line| log_line.content());

                    if let Some(matches) = self.search.apply_pattern(self.input.value(), lines)
                        && matches == 0
                    {
                        self.next_state(AppState::Message(format!(
                            "0 hits for '{}'",
                            self.input.value()
                        )));
                        return;
                    }

                    if !self.options.is_enabled("Search: Disable jumping to match") {
                        if let Some(line) =
                            self.search.first_match_from(self.viewport.selected_line)
                        {
                            self.push_viewport_line_to_history(line);
                            self.viewport.goto_line(line, false);
                        }
                        self.viewport.follow_mode = false;
                    }
                }
                self.next_state(AppState::LogView);
            }
            AppState::FilterMode => {
                if !self.input.value().is_empty() {
                    self.filter.add_filter(self.input.value().to_string());
                    self.update_view();
                }
                self.next_state(AppState::LogView);
            }
            AppState::EventsView => {
                if let Some(target_line) = self.event_tracker.get_selected_line_index()
                    && let Some(active_line) =
                        self.log_buffer.find_closest_line_by_index(target_line)
                {
                    self.viewport.goto_line(active_line, true);
                }
                self.next_state(AppState::LogView);
            }
            AppState::OptionsView => {
                self.options.enable_selected_option();
                self.next_state(AppState::LogView);
            }
            AppState::MarksView => {
                self.goto_selected_mark();
                self.next_state(AppState::LogView);
            }
            AppState::GotoLineMode => {
                if let Ok(line_number) = self.input.value().parse::<usize>() {
                    let viewport_index = line_number.saturating_sub(1);
                    if line_number > 0 && viewport_index < self.viewport.total_lines {
                        self.push_viewport_line_to_history(viewport_index);
                        self.viewport.goto_line(viewport_index, true);
                    }
                }
                self.next_state(AppState::LogView);
            }
            AppState::SaveToFileMode => {
                if !self.input.value().is_empty() {
                    match self.log_buffer.save_to_file(self.input.value()) {
                        Ok(_) => {
                            let abs_path = std::fs::canonicalize(self.input.value())
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| self.input.value().to_string());
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
                if !self.input.value().is_empty() {
                    self.filter
                        .update_selected_pattern(self.input.value().to_string());
                    self.update_view();
                }
                self.next_state(AppState::FilterListView);
            }
            AppState::MarkNameInputMode => {
                let filtered_marks = self.get_filtered_marks();
                if let Some(mark) = filtered_marks.get(self.marking.selected_index()) {
                    self.marking
                        .set_mark_name(mark.line_index, self.input.value().to_string());
                    let marks = self.marking.get_marks();
                    self.event_tracker.set_marks(&marks);
                }
                self.next_state(AppState::MarksView);
            }
            AppState::MarkAddInputMode => {
                if self.input.value().is_empty() {
                    self.next_state(AppState::MarksView);
                    return;
                }
                let count_before = self.marking.count();
                let lines = self.log_buffer.get_lines_iter(Interval::All);
                self.marking
                    .create_marks_from_pattern(self.input.value(), lines);
                let count_after = self.marking.count();
                let new_marks = count_after - count_before;
                if new_marks > 0 {
                    self.next_state(AppState::MarksView);
                } else {
                    self.next_state(AppState::Message(format!(
                        "No matches found for pattern '{}'",
                        self.input.value()
                    )));
                }
            }
            AppState::Message(_) => {
                self.next_state(AppState::LogView);
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
            AppState::SelectionMode => {
                self.cancel_selection();
                self.next_state(AppState::LogView);
            }
            AppState::LogView => {
                self.search.clear_matches();
                self.update_temporary_highlights();

                if self.filter.is_show_marked_only() {
                    self.filter.toggle_show_marked_only();
                    self.update_view();
                }
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
            AppState::MarkAddInputMode => {
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
            AppState::OptionsView => self.options.move_selection_up(),
            AppState::EventsView => {
                self.event_tracker.move_selection_up();
                self.viewport.follow_mode = false;
            }
            AppState::EventsFilterView => self.event_tracker.move_filter_selection_up(),
            AppState::MarksView => {
                let filtered_count = self.get_filtered_marks().len();
                self.marking.move_selection_up(filtered_count);
            }
            AppState::SelectionMode => {
                self.viewport.move_up();
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
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
            AppState::OptionsView => self.options.move_selection_down(),
            AppState::EventsView => self.event_tracker.move_selection_down(),
            AppState::EventsFilterView => self.event_tracker.move_filter_selection_down(),
            AppState::MarksView => {
                let filtered_count = self.get_filtered_marks().len();
                self.marking.move_selection_down(filtered_count);
            }
            AppState::SelectionMode => {
                self.viewport.move_down();
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.move_down();
            }
        }
    }

    pub fn page_up(&mut self) {
        match self.app_state {
            AppState::EventsView => {
                self.event_tracker.selection_page_up();
            }
            AppState::MarksView => {
                let filtered_count = self.get_filtered_marks().len();
                self.marking.selection_page_up(filtered_count);
            }
            AppState::SelectionMode => {
                self.viewport.page_up();
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.page_up();
                self.viewport.follow_mode = false;
            }
        }
    }

    pub fn page_down(&mut self) {
        match self.app_state {
            AppState::EventsView | AppState::EventsFilterView => {
                self.event_tracker.selection_page_down();
            }
            AppState::MarksView | AppState::MarkNameInputMode => {
                let filtered_count = self.get_filtered_marks().len();
                self.marking.selection_page_down(filtered_count);
            }
            AppState::SelectionMode => {
                self.viewport.page_down();
                self.viewport.follow_mode = false;
                self.update_selection_end();
            }
            _ => {
                self.viewport.page_down();
            }
        }
    }

    pub fn activate_search_mode(&mut self) {
        self.input.reset();
        self.search.clear_matches();
        self.search.reset_case_sensitive();
        self.search.history.reset();
        self.next_state(AppState::SearchMode);
    }

    pub fn activate_goto_line_mode(&mut self) {
        self.input.reset();
        self.next_state(AppState::GotoLineMode);
    }

    pub fn activate_filter_mode(&mut self) {
        self.input.reset();
        self.filter.reset_mode();
        self.filter.reset_case_sensitive();
        self.filter.history.reset();
        self.next_state(AppState::FilterMode);
    }

    pub fn activate_filter_list_view(&mut self) {
        self.next_state(AppState::FilterListView);
    }

    pub fn activate_edit_filter_mode(&mut self) {
        if let Some(filter) = self.filter.get_selected_pattern() {
            self.input = Input::new(filter.pattern.clone());
            self.next_state(AppState::EditFilterMode);
        }
    }

    pub fn activate_options_view(&mut self) {
        self.next_state(AppState::OptionsView);
    }

    pub fn activate_events_view(&mut self) {
        if self.event_tracker.needs_rescan() {
            self.event_tracker
                .scan_all_lines(&self.log_buffer, self.highlighter.events());
        }
        if let Some(line_index) = self
            .log_buffer
            .viewport_to_log_index(self.viewport.selected_line)
        {
            self.event_tracker.select_nearest_event(line_index);
        }
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
            .viewport_to_log_index(self.viewport.selected_line)
        {
            self.marking.select_nearest_mark(line_index);
        } else {
            self.marking.reset_selection();
        }
        self.next_state(AppState::MarksView);
    }

    pub fn activate_mark_name_input_mode(&mut self) {
        let filtered_marks = self.get_filtered_marks();
        if let Some(mark) = filtered_marks.get(self.marking.selected_index()) {
            if let Some(name) = &mark.name {
                self.input = Input::new(name.clone());
            } else {
                self.input.reset();
            }
            self.next_state(AppState::MarkNameInputMode);
        }
    }

    pub fn activate_mark_add_input_mode(&mut self) {
        self.input.reset();
        self.next_state(AppState::MarkAddInputMode);
    }

    pub fn activate_save_to_file_mode(&mut self) {
        if self.log_buffer.streaming {
            self.input.reset();
            self.next_state(AppState::SaveToFileMode);
        }
    }

    pub fn toggle_mark(&mut self) {
        if self.app_state == AppState::SelectionMode {
            if let Some((start, end)) = self.get_selection_range() {
                let log_indices: Vec<usize> = (start..=end)
                    .filter_map(|viewport_line| {
                        self.log_buffer.viewport_to_log_index(viewport_line)
                    })
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
        } else if let Some(line_index) = self
            .log_buffer
            .viewport_to_log_index(self.viewport.selected_line)
        {
            self.marking.toggle_mark(line_index);
        }

        let marks = self.marking.get_marks();
        self.event_tracker.set_marks(&marks);
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.search.toggle_case_sensitive();
        self.filter.toggle_case_sensitive();

        if self.app_state == AppState::SearchMode {
            let lines = self
                .log_buffer
                .get_lines_iter(Interval::All)
                .map(|log_line| log_line.content());
            self.search.update_matches(self.input.value(), lines);
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

    /// Helper to get filtered marks (only marks on visible lines).
    pub fn get_filtered_marks(&self) -> Vec<&Mark> {
        let active_lines = self.log_buffer.get_active_lines();
        self.marking
            .get_marks()
            .into_iter()
            .filter(|m| active_lines.binary_search(&m.line_index).is_ok())
            .collect()
    }

    /// Helper to get the next visible mark after the given line index.
    fn get_next_visible_mark(&self, line_index: usize) -> Option<&Mark> {
        let active_lines = self.log_buffer.get_active_lines();
        self.marking
            .get_marks()
            .into_iter()
            .filter(|m| m.line_index > line_index)
            .find(|m| active_lines.binary_search(&m.line_index).is_ok())
    }

    /// Helper to get the previous visible mark before the given line index.
    fn get_previous_visible_mark(&self, line_index: usize) -> Option<&Mark> {
        let active_lines = self.log_buffer.get_active_lines();
        self.marking
            .get_marks()
            .into_iter()
            .rev()
            .filter(|m| m.line_index < line_index)
            .find(|m| active_lines.binary_search(&m.line_index).is_ok())
    }

    pub fn mark_next(&mut self) {
        if let Some(line_index) = self
            .log_buffer
            .viewport_to_log_index(self.viewport.selected_line)
            && let Some(next_mark) = self.get_next_visible_mark(line_index)
        {
            let next_line = next_mark.line_index;
            self.viewport.push_history(next_line);
            self.goto_line(next_line);
        }
    }

    pub fn mark_previous(&mut self) {
        if let Some(line_index) = self
            .log_buffer
            .viewport_to_log_index(self.viewport.selected_line)
            && let Some(prev_mark) = self.get_previous_visible_mark(line_index)
        {
            let prev_line = prev_mark.line_index;
            self.viewport.push_history(prev_line);
            self.goto_line(prev_line);
        }
    }

    /// Helper to go to a log line by its log line index. If the line is not visible, it does nothing.
    pub fn goto_line(&mut self, log_index: usize) {
        if let Some(active_line) = self.log_buffer.find_line(log_index) {
            self.viewport.goto_line(active_line, false);
        }
    }

    /// Helper to record a viewport line in history by converting from viewport index to log index.
    fn push_viewport_line_to_history(&mut self, viewport_line: usize) {
        if let Some(line_index) = self.log_buffer.viewport_to_log_index(viewport_line) {
            self.viewport.push_history(line_index);
        }
    }

    pub fn goto_top(&mut self) {
        self.viewport.goto_top();
        self.push_viewport_line_to_history(self.viewport.selected_line);
    }

    pub fn goto_bottom(&mut self) {
        self.viewport.goto_bottom();
        self.push_viewport_line_to_history(self.viewport.selected_line);
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

    pub fn toggle_help(&mut self) {
        if self.help.is_visible() {
            self.help.toggle_visibility();
        } else {
            self.help.show_for_state(&self.app_state);
        }
    }

    pub fn history_back(&mut self) {
        if let Some(line_index) = self.viewport.history_back() {
            self.goto_line(line_index);
        }
        self.viewport.follow_mode = false;
    }

    pub fn history_forward(&mut self) {
        if let Some(line_index) = self.viewport.history_forward() {
            self.goto_line(line_index);
        }
        self.viewport.follow_mode = false;
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

    pub fn toggle_show_marked_only(&mut self) {
        if !self.filter.is_show_marked_only() && self.marking.count() == 0 {
            return;
        }

        self.filter.toggle_show_marked_only();
        self.update_view();
    }

    pub fn toggle_event_filter(&mut self) {
        self.event_tracker.toggle_selected_filter();
        self.event_tracker
            .scan_all_lines(&self.log_buffer, self.highlighter.events());
        self.event_tracker
            .select_nearest_event(self.viewport.selected_line);
    }

    pub fn toggle_show_marks_in_events(&mut self) {
        self.event_tracker.toggle_show_marks();
        let marks = self.marking.get_marks();
        self.event_tracker.set_marks(&marks);
    }

    pub fn toggle_all_event_filters(&mut self) {
        self.event_tracker.toggle_all_filters();
        self.event_tracker
            .scan_all_lines(&self.log_buffer, self.highlighter.events());
        self.event_tracker
            .select_nearest_event(self.viewport.selected_line);
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
            self.filter.set_case_sensitive(history_entry.case_sensitive);
            self.update_temporary_highlights();
        }
    }

    pub fn filter_history_next(&mut self) {
        if let Some(history_entry) = self.filter.history.next_record().cloned() {
            self.input = Input::new(history_entry.pattern);
            self.filter.set_mode(history_entry.mode);
            self.filter.set_case_sensitive(history_entry.case_sensitive);
            self.update_temporary_highlights();
        } else {
            self.input.reset();
            self.filter.reset_mode();
            self.filter.reset_case_sensitive();
            self.update_temporary_highlights();
        }
    }

    pub fn unmark_selected(&mut self) {
        let filtered_marks = self.get_filtered_marks();
        if let Some(mark) = filtered_marks.get(self.marking.selected_index()) {
            self.marking.unmark(mark.line_index);
            let marks = self.marking.get_marks();
            self.event_tracker.set_marks(&marks);
        }
    }

    pub fn goto_selected_event(&mut self) {
        if let Some(line_index) = self.event_tracker.get_selected_line_index() {
            self.viewport.push_history(line_index);
            self.goto_line(line_index);
        }
    }

    pub fn goto_selected_mark(&mut self) {
        let filtered_marks = self.get_filtered_marks();
        if let Some(mark) = filtered_marks.get(self.marking.selected_index()) {
            let mark_line = mark.line_index;
            self.viewport.push_history(mark_line);
            self.goto_line(mark_line);
        }
    }

    /// Enters selection mode and sets the start of the selection range.
    pub fn start_selection(&mut self) {
        let current_line = self.viewport.selected_line;
        self.selection_range = Some((current_line, current_line));
        self.next_state(AppState::SelectionMode);
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
        self.selection_range.map(|(start, end)| {
            if start <= end {
                (start, end)
            } else {
                (end, start)
            }
        })
    }

    /// Copies the selected lines to the clipboard.
    pub fn copy_selection_to_clipboard(&mut self) {
        if let Some((start, end)) = self.get_selection_range() {
            let lines: Vec<String> = (start..=end)
                .filter_map(|viewport_line| {
                    self.log_buffer
                        .viewport_to_log_index(viewport_line)
                        .and_then(|log_index| self.log_buffer.get_line(log_index))
                })
                .map(|log_line| log_line.content.clone())
                .collect();

            if !lines.is_empty() {
                let content = lines.join("\n");
                match arboard::Clipboard::new() {
                    Ok(mut clipboard) => match clipboard.set_text(content) {
                        Ok(_) => {
                            let num_lines = lines.len();
                            self.selection_range = None;
                            self.next_state(AppState::Message(format!(
                                "Copied {} line{} to clipboard",
                                num_lines,
                                if num_lines == 1 { "" } else { "s" }
                            )));
                        }
                        Err(e) => {
                            self.selection_range = None;
                            self.next_state(AppState::ErrorState(format!(
                                "Failed to copy to clipboard: {}",
                                e
                            )));
                        }
                    },
                    Err(e) => {
                        self.selection_range = None;
                        self.next_state(AppState::ErrorState(format!(
                            "Failed to access clipboard: {}",
                            e
                        )));
                    }
                }
            }
        }
    }
}
