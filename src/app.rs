use crate::{
    cli::Cli,
    event::{AppEvent, Event, EventHandler},
    log::LogBuffer,
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
    HelpView,
    SearchView,
    GotoLineView,
}

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub app_state: AppState,
    pub events: EventHandler,
    pub log_buffer: LogBuffer,
    pub filtered_lines: Vec<usize>,
    pub viewport: Viewport,
    pub input_query: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            app_state: AppState::LogView,
            events: EventHandler::new(),
            log_buffer: LogBuffer::default(),
            filtered_lines: Vec::new(),
            viewport: Viewport::default(),
            input_query: String::new(),
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
        self.filtered_lines = (0..self.log_buffer.lines.len()).collect();
        self.viewport.set_total_lines(self.log_buffer.lines.len());
        info!(
            lines_loaded = self.log_buffer.lines.len(),
            file_path = file_path,
            "File loaded successfully"
        );
        Ok(())
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
                    AppEvent::Confirm => {
                        debug!("Confirm");
                        if self.app_state == AppState::SearchView {
                            self.next_state(AppState::LogView);
                        } else if self.app_state == AppState::GotoLineView {
                            if let Ok(line_number) = self.input_query.parse::<usize>() {
                                if line_number > 0 && line_number <= self.log_buffer.lines.len() {
                                    self.viewport.goto_line(line_number - 1);
                                }
                            }
                            self.next_state(AppState::LogView);
                        }
                    }
                    AppEvent::Cancel => {
                        debug!("Cancel");
                        match self.app_state {
                            AppState::HelpView => {
                                self.next_state(AppState::LogView);
                            }
                            AppState::SearchView => {
                                self.next_state(AppState::LogView);
                            }
                            AppState::GotoLineView => {
                                self.next_state(AppState::LogView);
                            }
                            AppState::LogView => {}
                        }
                    }
                    AppEvent::MoveUp => self.viewport.move_up(),
                    AppEvent::MoveDown => self.viewport.move_down(),
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
                    AppEvent::ToggleHelp => {
                        if self.app_state == AppState::HelpView {
                            self.next_state(AppState::LogView);
                        } else {
                            self.next_state(AppState::HelpView);
                        }
                    }
                    AppEvent::SearchMode => {
                        self.input_query.clear();
                        self.next_state(AppState::SearchView);
                    }
                    AppEvent::GotoLineMode => {
                        self.input_query.clear();
                        self.next_state(AppState::GotoLineView);
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
                KeyCode::Char('/') => self.events.send(AppEvent::SearchMode),
                KeyCode::Char('f') if key_event.modifiers == KeyModifiers::CONTROL => {
                    self.events.send(AppEvent::SearchMode)
                }
                KeyCode::Char(':') => self.events.send(AppEvent::GotoLineMode),
                _ => {}
            },

            // HelpView
            AppState::HelpView => match key_event.code {
                KeyCode::Char('q') => self.events.send(AppEvent::Quit),
                KeyCode::Char('h') => self.events.send(AppEvent::ToggleHelp),
                _ => {}
            },

            // SearchView
            AppState::SearchView => match key_event.code {
                KeyCode::Char(c) => {
                    self.input_query.push(c);
                    debug!("Search query: {}", self.input_query);
                }
                KeyCode::Backspace => {
                    self.input_query.pop();
                    debug!("Search query: {}", self.input_query);
                }
                _ => {}
            },

            // GotoLineView
            AppState::GotoLineView => match key_event.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    self.input_query.push(c);
                    debug!("Goto line query: {}", self.input_query);
                }
                KeyCode::Backspace => {
                    self.input_query.pop();
                    debug!("Goto line query: {}", self.input_query);
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
