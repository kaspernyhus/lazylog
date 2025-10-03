use crate::{
    cli::Cli,
    display_options::DisplayOptions,
    event::{AppEvent, Event, EventHandler},
    filter::Filter,
    help::Help,
    highlighter::Highlighter,
    log::{Interval, LogBuffer},
    search::Search,
    viewport::Viewport,
};
use ratatui::{
    Terminal,
    backend::Backend,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};

#[derive(Debug, PartialEq)]
pub enum AppState {
    LogView,
    SearchMode,
    GotoLineMode,
    FilterMode,
    FilterListView,
    OptionsView,
    ErrorState(String),
}

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub help: Help,
    pub app_state: AppState,
    pub events: EventHandler,
    pub log_buffer: LogBuffer,
    pub viewport: Viewport,
    pub input_query: String,
    pub search: Search,
    pub filter: Filter,
    pub display_options: DisplayOptions,
    pub highlighter: Highlighter,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            help: Help::new(),
            app_state: AppState::LogView,
            events: EventHandler::new(),
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input_query: String::new(),
            search: Search::default(),
            filter: Filter::default(),
            display_options: DisplayOptions::default(),
            highlighter: Highlighter::new(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let use_stdin = args.should_use_stdin();

        let events = if use_stdin {
            EventHandler::new_with_stdin()
        } else {
            EventHandler::new()
        };

        let mut app = Self {
            running: true,
            help: Help::new(),
            app_state: AppState::LogView,
            events,
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input_query: String::new(),
            search: Search::default(),
            filter: Filter::default(),
            display_options: DisplayOptions::default(),
            highlighter: Highlighter::new(),
        };

        if use_stdin {
            app.log_buffer.init_stdin_mode();
            app.viewport.follow_mode = true;
        } else if let Some(file_path) = args.file {
            let error = app.log_buffer.load_from_file(file_path.as_str());
            if let Err(e) = error {
                app.app_state = AppState::ErrorState(format!(
                    "Failed to load file: {}\nError: {}",
                    file_path, e
                ));
            } else {
                app.update_view();
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
                Event::App(app_event) => match app_event {
                    AppEvent::Quit => self.quit(),
                    AppEvent::Confirm => match self.app_state {
                        AppState::SearchMode => {
                            if self.input_query.is_empty() {
                                self.search.clear_pattern();
                            } else {
                                let lines: Vec<&str> = self
                                    .log_buffer
                                    .get_lines_iter(Interval::All)
                                    .map(|log_line| log_line.content())
                                    .collect();
                                self.search.apply_pattern(self.input_query.clone(), &lines);
                                if let Some(line) =
                                    self.search.next_match(self.viewport.selected_line)
                                {
                                    self.viewport.goto_line(line, true);
                                }
                            }
                            self.next_state(AppState::LogView);
                        }
                        AppState::FilterMode => {
                            if self.input_query.is_empty() {
                                self.filter.clear_filter_pattern();
                            } else {
                                self.filter.add_filter(self.input_query.clone());
                                self.update_view();
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
                        _ => {}
                    },
                    AppEvent::Cancel => {
                        if self.help.is_visible() {
                            self.help.toggle_visibility();
                            continue;
                        }
                        match self.app_state {
                            AppState::SearchMode => {
                                self.search.clear_pattern();
                                self.next_state(AppState::LogView);
                            }
                            AppState::GotoLineMode => {
                                self.next_state(AppState::LogView);
                            }
                            AppState::FilterMode => {
                                self.filter.clear_filter_pattern();
                                self.next_state(AppState::LogView);
                            }
                            AppState::LogView => {
                                self.search.clear_pattern();
                            }
                            AppState::FilterListView => {
                                self.next_state(AppState::LogView);
                            }
                            AppState::OptionsView => {
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
                        self.search.clear_pattern();
                        self.search.history.reset();
                        self.next_state(AppState::SearchMode);
                    }
                    AppEvent::ToggleCaseSensitive => {
                        self.search.toggle_case_sensitive();
                        self.filter.toggle_case_sensitive();
                        if self.search.get_pattern().is_some() {
                            let lines: Vec<&str> = self
                                .log_buffer
                                .get_lines_iter(Interval::All)
                                .map(|log_line| log_line.content())
                                .collect();
                            self.search.update_matches(&lines);
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
                        if let Some(line) = self.search.previous_match(self.viewport.selected_line)
                        {
                            self.viewport.goto_line(line, false);
                        }
                    }
                    AppEvent::ActivateFilterMode => {
                        self.input_query.clear();
                        self.next_state(AppState::FilterMode);
                    }
                    AppEvent::ToggleFilterMode => {
                        self.filter.toggle_mode();
                        if self.filter.get_filter_pattern().is_some() {
                            self.log_buffer.apply_filters(&self.filter);
                        }
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
                    AppEvent::SearchHistoryPrevious => {
                        if let Some(history_query) = self.search.history.previous_query() {
                            self.input_query = history_query;
                            self.search.update_pattern(&self.input_query, 0);
                        }
                    }
                    AppEvent::SearchHistoryNext => {
                        if let Some(history_query) = self.search.history.next_query() {
                            self.input_query = history_query;
                            self.search.update_pattern(&self.input_query, 0);
                        }
                    }
                    AppEvent::ToggleFollowMode => {
                        self.viewport.follow_mode = !self.viewport.follow_mode;
                        if self.viewport.follow_mode {
                            self.viewport.goto_bottom();
                        }
                    }
                    AppEvent::ActivateOptionsView => {
                        self.next_state(AppState::OptionsView);
                    }
                    AppEvent::ToggleDisplayOption => {
                        self.display_options.toggle_selected_option();
                    }
                    AppEvent::ClearLogBuffer => {
                        if self.log_buffer.streaming {
                            self.log_buffer.clear_all();
                            self.viewport.set_total_lines(0);
                            self.viewport.selected_line = 0;
                        }
                    }
                    AppEvent::NewLine(line) => {
                        let passes_filter = self.log_buffer.append_line(line, &self.filter);
                        if passes_filter {
                            let num_lines = self.log_buffer.get_lines_count();
                            self.viewport.set_total_lines(num_lines);
                            if self.viewport.follow_mode {
                                self.viewport.goto_bottom();
                            }
                        }
                    }
                },
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
                KeyCode::Char('o') => self.events.send(AppEvent::ActivateOptionsView),
                _ => {}
            },

            // SearchMode
            AppState::SearchMode => match key_event.code {
                KeyCode::Tab => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Up => self.events.send(AppEvent::SearchHistoryPrevious),
                KeyCode::Down => self.events.send(AppEvent::SearchHistoryNext),
                KeyCode::Backspace => {
                    self.input_query.pop();
                    self.search.update_pattern(&self.input_query, 2);
                    self.search.history.reset();
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                    self.search.update_pattern(&self.input_query, 2);
                    self.search.history.reset();
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
                    self.filter.update_filter_pattern(&self.input_query, 2);
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                    self.filter.update_filter_pattern(&self.input_query, 2);
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
                KeyCode::Char('e') => {} // TODO: Edit selected filter pattern
                KeyCode::Char('a') => {} // TODO: Add "add new filter pattern" functionality
                KeyCode::Char('A') => {} // TODO: All filters On/Off
                KeyCode::Char('c') => {} // TODO: toggle case sensitive for selected filter
                KeyCode::Char('m') => {} // TODO: toggle mode for selected filter
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
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
