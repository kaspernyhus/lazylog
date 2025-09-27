use crate::{
    cli::Cli,
    event::{AppEvent, Event, EventHandler},
    filter::Filter,
    log::LogBuffer,
    search::Search,
    viewport::Viewport,
};
use ratatui::{
    DefaultTerminal,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
};
use tracing::{debug, info};

#[derive(Debug, PartialEq)]
pub enum AppState {
    LogView,
    SearchMode,
    GotoLineMode,
    FilterMode,
    FilterListView,
}

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub show_help: bool,
    pub app_state: AppState,
    pub events: EventHandler,
    pub log_buffer: LogBuffer,
    pub viewport: Viewport,
    pub input_query: String,
    pub search: Search,
    pub filter: Filter,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            show_help: false,
            app_state: AppState::LogView,
            events: EventHandler::new(),
            log_buffer: LogBuffer::default(),
            viewport: Viewport::default(),
            input_query: String::new(),
            search: Search::default(),
            filter: Filter::default(),
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(args: Cli) -> Self {
        let mut app = Self::default();
        if let Some(file_path) = args.file {
            let _ = app.load_file(file_path.as_str());
        }
        app
    }

    pub fn load_file(&mut self, file_path: &str) -> color_eyre::Result<()> {
        info!("Loading file: {}", file_path);
        self.log_buffer.load_from_file(file_path)?;
        self.update_filters();
        self.update_filters();
        info!(
            lines_loaded = self.log_buffer.lines.len(),
            file_path = file_path,
            "File loaded successfully"
        );
        Ok(())
    }

    fn update_filters(&mut self) {
        let filter_patterns = self.filter.get_filter_patterns();
        debug!("Applying {} filters", filter_patterns.len());

        self.log_buffer.apply_filters(&self.filter);
        let filtered_count = self.log_buffer.get_lines_count();
        let (total_lines, filtered_lines) = self.log_buffer.debug_filter_state();
        debug!("Filter result: {}/{} lines visible", filtered_lines, total_lines);

        self.viewport.set_total_lines(filtered_count);

        // Ensure selected line is within bounds after filtering
        if filtered_count == 0 {
            self.viewport.selected_line = 0;
        } else if self.viewport.selected_line >= filtered_count {
            // Position at the end of filtered results
            self.viewport.goto_line(filtered_count - 1, false);
        }
        // If selected line is still valid, keep it as is - no need to adjust
    }

    fn next_state(&mut self, state: AppState) {
        debug!("Next state: {:?}", state);
        self.app_state = state;
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
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
                                self.search.clear_search_pattern();
                            } else {
                                let lines: Vec<&str> = self
                                    .log_buffer
                                    .get_lines_iter(None)
                                    .collect();
                                self.search.update_matches(&lines);
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
                                self.filter.add_filter();
                                self.update_filters();
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
                        if self.show_help {
                            self.show_help = false;
                            continue;
                        }
                        match self.app_state {
                            AppState::SearchMode => {
                                self.search.clear_search_pattern();
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
                                self.search.clear_search_pattern();
                            }
                            AppState::FilterListView => {
                                self.next_state(AppState::LogView);
                            }
                        };
                    }
                    AppEvent::MoveUp => {
                        if self.app_state == AppState::FilterListView {
                            self.filter.move_selection_up();
                        } else {
                            self.viewport.move_up();
                        }
                    }
                    AppEvent::MoveDown => {
                        if self.app_state == AppState::FilterListView {
                            self.filter.move_selection_down();
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
                        let max_line_length = self.log_buffer.get_lines_max_length(start, end);
                        self.viewport.scroll_right(max_line_length)
                    }
                    AppEvent::ResetHorizontal => self.viewport.reset_horizontal(),
                    AppEvent::ToggleHelp => self.show_help = !self.show_help,
                    AppEvent::ActivateSearchMode => {
                        self.input_query.clear();
                        self.search.clear_search_pattern();
                        self.next_state(AppState::SearchMode);
                    }
                    AppEvent::ToggleCaseSensitive => {
                        self.search.toggle_case_sensitive();
                        self.filter.toggle_case_sensitive();
                        if self.search.get_search_pattern().is_some() {
                            let lines: Vec<&str> = self
                                .log_buffer
                                .get_lines_iter(None)
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
                            // self.apply_filter();
                        }
                    }
                    AppEvent::ActivateFilterListView => {
                        self.next_state(AppState::FilterListView);
                    }
                    AppEvent::ToggleFilterPatternActive => {
                        self.filter.toggle_selected_pattern();
                        self.update_filters();
                    }
                    AppEvent::RemoveFilterPattern => {
                        self.filter.remove_selected_pattern();
                        self.update_filters();
                    }
                },
            }
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        debug!("Key event: {:?}", key_event);

        // Global keybindings
        match key_event.code {
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            KeyCode::Esc => self.events.send(AppEvent::Cancel),
            KeyCode::Enter => self.events.send(AppEvent::Confirm),
            _ => {}
        }

        match self.app_state {
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
                _ => {}
            },

            // SearchMode
            AppState::SearchMode => match key_event.code {
                KeyCode::Up => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Down => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Backspace => {
                    self.input_query.pop();
                    self.search.update_search_pattern(&self.input_query, 2);
                }
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                    self.search.update_search_pattern(&self.input_query, 2);
                }
                _ => {}
            },

            // FilterMode
            AppState::FilterMode => match key_event.code {
                KeyCode::Up => self.events.send(AppEvent::ToggleCaseSensitive),
                KeyCode::Down => self.events.send(AppEvent::ToggleCaseSensitive),
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
