use crate::app::AppState;
use crate::command::Command;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

type KeyBindingKey = (AppState, KeyCode, KeyModifiers);

/// Registry of all keybindings mapped to commands.
#[derive(Debug, Default)]
pub struct KeybindingRegistry {
    bindings: Vec<(KeyBindingKey, Command)>,
}

impl KeybindingRegistry {
    /// Creates a new keybinding registry with all default bindings.
    pub fn new() -> Self {
        let mut registry = Self {
            bindings: Vec::new(),
        };

        registry.register_log_view_bindings();
        registry.register_selection_mode_bindings();
        registry.register_search_mode_bindings();
        registry.register_filter_mode_bindings();
        registry.register_filter_list_bindings();
        registry.register_options_view_bindings();
        registry.register_events_view_bindings();
        registry.register_event_filter_view_bindings();
        registry.register_marks_view_bindings();
        registry.register_message_state_bindings();
        registry.register_error_state_bindings();

        registry.register_global_bindings(AppState::LogView);
        registry.register_global_bindings(AppState::SelectionMode);
        registry.register_global_bindings(AppState::SearchMode);
        registry.register_global_bindings(AppState::FilterMode);
        registry.register_global_bindings(AppState::FilterListView);
        registry.register_global_bindings(AppState::OptionsView);
        registry.register_global_bindings(AppState::EventsView);
        registry.register_global_bindings(AppState::EventsFilterView);
        registry.register_global_bindings(AppState::MarksView);
        registry.register_global_bindings(AppState::MarkNameInputMode);
        registry.register_global_bindings(AppState::MarkAddInputMode);
        registry.register_global_bindings(AppState::GotoLineMode);
        registry.register_global_bindings(AppState::EditFilterMode);
        registry.register_global_bindings(AppState::SaveToFileMode);
        registry.register_global_bindings(AppState::Message(String::new()));
        registry.register_global_bindings(AppState::ErrorState(String::new()));

        registry
    }

    /// Looks up a command for the given state and key event.
    pub fn lookup(&self, app_state: &AppState, key_event: KeyEvent) -> Option<Command> {
        if let Some((_, cmd)) = self.bindings.iter().find(|((state, kcode, kmod), _)| {
            state.matches(app_state) && *kcode == key_event.code && *kmod == key_event.modifiers
        }) {
            return Some(*cmd);
        }

        None
    }

    /// Returns all keybindings for a specific state, grouped and sorted.
    pub fn get_keybindings_for_state(&self, target_state: &AppState) -> Vec<(String, Command)> {
        let bindings: Vec<(String, Command)> = self
            .bindings
            .iter()
            .filter(|((state, _, _), _)| state.matches(target_state))
            .map(|((_, keycode, modifiers), cmd)| (Self::format_key(*keycode, *modifiers), *cmd))
            .collect();
        bindings
    }

    fn format_key(keycode: KeyCode, modifiers: KeyModifiers) -> String {
        let key_str = match keycode {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) if c.is_uppercase() => c.to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Up => "Up".to_string(),
            KeyCode::Down => "Down".to_string(),
            KeyCode::Left => "Left".to_string(),
            KeyCode::Right => "Right".to_string(),
            KeyCode::PageUp => "PageUp".to_string(),
            KeyCode::PageDown => "PageDown".to_string(),
            KeyCode::Enter => "Enter".to_string(),
            KeyCode::Esc => "Esc".to_string(),
            KeyCode::Backspace => "Backspace".to_string(),
            KeyCode::Delete => "Delete".to_string(),
            KeyCode::Tab => "Tab".to_string(),
            _ => format!("{:?}", keycode),
        };

        if modifiers.contains(KeyModifiers::CONTROL) {
            format!("Ctrl+{}", key_str)
        } else if modifiers.contains(KeyModifiers::SHIFT) {
            if let KeyCode::Char(c) = keycode {
                c.to_uppercase().to_string()
            } else {
                format!("Shift+{}", key_str)
            }
        } else if modifiers.contains(KeyModifiers::ALT) {
            format!("Alt+{}", key_str)
        } else if modifiers.is_empty() {
            key_str
        } else {
            format!("{:?}+{}", modifiers, key_str)
        }
    }

    /// Helper to register a single keybinding.
    fn bind(
        &mut self,
        state: AppState,
        keycode: KeyCode,
        modifiers: KeyModifiers,
        command: Command,
    ) {
        self.bindings.push(((state, keycode, modifiers), command));
    }

    /// Helper to register a keybinding without modifiers.
    fn bind_simple(&mut self, state: AppState, keycode: KeyCode, command: Command) {
        self.bind(state, keycode, KeyModifiers::empty(), command);
    }

    /// Helper to register a keybinding with SHIFT modifier.
    fn bind_shift(&mut self, state: AppState, c: char, command: Command) {
        self.bind(state, KeyCode::Char(c), KeyModifiers::SHIFT, command);
    }

    /// Registers global keybindings that work in all states.
    fn register_global_bindings(&mut self, state: AppState) {
        self.bind(
            state.clone(),
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            Command::Quit,
        );
        self.bind_simple(state.clone(), KeyCode::Esc, Command::Cancel);
        self.bind_simple(state.clone(), KeyCode::Enter, Command::Confirm);
        self.bind_simple(state.clone(), KeyCode::F(1), Command::ToggleHelp);
    }

    fn register_log_view_bindings(&mut self) {
        let state = AppState::LogView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(state.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(state.clone(), KeyCode::Char('g'), Command::GotoTop);
        self.bind_shift(state.clone(), 'G', Command::GotoBottom);
        self.bind_simple(state.clone(), KeyCode::Char('z'), Command::CenterSelected);
        self.bind_simple(state.clone(), KeyCode::Left, Command::ScrollLeft);
        self.bind_simple(state.clone(), KeyCode::Right, Command::ScrollRight);
        self.bind_simple(state.clone(), KeyCode::Char('h'), Command::ScrollLeft);
        self.bind_simple(state.clone(), KeyCode::Char('l'), Command::ScrollRight);
        self.bind_simple(state.clone(), KeyCode::Char('0'), Command::ResetHorizontal);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('/'),
            Command::ActivateSearchMode,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            Command::ActivateSearchMode,
        );
        self.bind_simple(state.clone(), KeyCode::Char('n'), Command::SearchNext);
        self.bind_shift(state.clone(), 'N', Command::SearchPrevious);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('f'),
            Command::ActivateFilterMode,
        );
        self.bind_shift(state.clone(), 'F', Command::ActivateFilterListView);
        self.bind_simple(
            state.clone(),
            KeyCode::Char(':'),
            Command::ActivateGotoLineMode,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('o'),
            Command::ActivateOptionsView,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('e'),
            Command::ActivateEventsView,
        );
        self.bind_simple(state.clone(), KeyCode::Char(' '), Command::ToggleMark);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('m'),
            Command::ActivateMarksView,
        );
        self.bind_simple(state.clone(), KeyCode::Char(']'), Command::MarkNext);
        self.bind_simple(state.clone(), KeyCode::Char('['), Command::MarkPrevious);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('c'),
            Command::ToggleCenterCursorMode,
        );
        self.bind_simple(state.clone(), KeyCode::Char('t'), Command::ToggleFollowMode);
        self.bind_simple(state.clone(), KeyCode::Char('p'), Command::TogglePauseMode);
        self.bind(
            state.clone(),
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
            Command::ClearLogBuffer,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
            Command::ActivateSaveToFileMode,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
            Command::HistoryBack,
        );
        self.bind_simple(state.clone(), KeyCode::Tab, Command::HistoryForward);
        self.bind_shift(state.clone(), 'V', Command::StartSelection);
    }

    fn register_selection_mode_bindings(&mut self) {
        let state = AppState::SelectionMode;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(state.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(state.clone(), KeyCode::Char('g'), Command::GotoTop);
        self.bind_shift(state.clone(), 'G', Command::GotoBottom);
        self.bind_simple(state.clone(), KeyCode::Char('y'), Command::CopySelection);
    }

    fn register_search_mode_bindings(&mut self) {
        let state = AppState::SearchMode;

        self.bind(
            state.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleCaseSearch,
        );
        self.bind_simple(state.clone(), KeyCode::Up, Command::SearchHistoryPrevious);
        self.bind_simple(state.clone(), KeyCode::Down, Command::SearchHistoryNext);
    }

    fn register_filter_mode_bindings(&mut self) {
        let state = AppState::FilterMode;
        self.bind(
            state.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleCaseFilter,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('e'),
            KeyModifiers::ALT,
            Command::ToggleFilterModeInOut,
        );
        self.bind_simple(state.clone(), KeyCode::Up, Command::FilterHistoryPrevious);
        self.bind_simple(state.clone(), KeyCode::Down, Command::FilterHistoryNext);
    }

    fn register_filter_list_bindings(&mut self) {
        let state = AppState::FilterListView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(
            state.clone(),
            KeyCode::Char(' '),
            Command::ToggleFilterPattern,
        );
        self.bind_simple(state.clone(), KeyCode::Delete, Command::RemoveFilterPattern);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('d'),
            Command::RemoveFilterPattern,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('e'),
            Command::ActivateEditFilterMode,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('f'),
            Command::ActivateFilterMode,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('a'),
            Command::ToggleAllFilterPatterns,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleFilterPatternCaseSensitive,
        );
        self.bind(
            state.clone(),
            KeyCode::Char('e'),
            KeyModifiers::ALT,
            Command::ToggleFilterPatternMode,
        );
    }

    fn register_options_view_bindings(&mut self) {
        let state = AppState::OptionsView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(
            state.clone(),
            KeyCode::Char(' '),
            Command::ToggleDisplayOption,
        );
    }

    fn register_events_view_bindings(&mut self) {
        let state = AppState::EventsView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_shift(state.clone(), 'F', Command::ActivateEventFilterView);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(state.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(
            state.clone(),
            KeyCode::Char(' '),
            Command::GotoSelectedEvent,
        );
    }

    fn register_event_filter_view_bindings(&mut self) {
        let state = AppState::EventsFilterView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(state.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(
            state.clone(),
            KeyCode::Char(' '),
            Command::ToggleEventFilter,
        );
        self.bind_simple(
            state.clone(),
            KeyCode::Char('a'),
            Command::ToggleAllEventFilters,
        );
    }

    fn register_marks_view_bindings(&mut self) {
        let state = AppState::MarksView;

        self.bind_simple(state.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(state.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(state.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(state.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(state.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(state.clone(), KeyCode::Char(' '), Command::GotoSelectedMark);
        self.bind_simple(state.clone(), KeyCode::Delete, Command::UnmarkSelected);
        self.bind_simple(state.clone(), KeyCode::Char('d'), Command::UnmarkSelected);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('e'),
            Command::ActivateMarkNameInputMode,
        );
        self.bind_simple(state.clone(), KeyCode::Char('c'), Command::ClearAllMarks);
        self.bind_simple(
            state.clone(),
            KeyCode::Char('n'),
            Command::ActivateMarkAddInputMode,
        );
    }

    fn register_message_state_bindings(&mut self) {
        let state = AppState::Message(String::new());

        self.bind_simple(state, KeyCode::Char('q'), Command::Quit);
    }

    fn register_error_state_bindings(&mut self) {
        let state = AppState::ErrorState(String::new());

        self.bind_simple(state, KeyCode::Char('q'), Command::Quit);
    }
}
