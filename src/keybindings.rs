use crate::app::{Overlay, ViewState};
use crate::command::Command;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Represents the context for a keybinding.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeybindingContext {
    View(ViewState),
    Overlay(Overlay),
}

type KeyBindingKey = (KeybindingContext, KeyCode, KeyModifiers);

/// Registry of all keybindings mapped to commands.
#[derive(Debug, Default)]
pub struct KeybindingRegistry {
    bindings: Vec<(KeyBindingKey, Command)>,
}

impl KeybindingRegistry {
    /// Creates a new keybinding registry with all default bindings.
    pub fn new() -> Self {
        let mut registry = Self { bindings: Vec::new() };

        registry.register_log_view_bindings();
        registry.register_selection_mode_bindings();
        registry.register_search_mode_bindings();
        registry.register_filter_mode_bindings();
        registry.register_filter_list_bindings();
        registry.register_options_view_bindings();
        registry.register_events_view_bindings();
        registry.register_event_filter_view_bindings();
        registry.register_marks_view_bindings();
        registry.register_files_view_bindings();
        registry.register_message_state_bindings();
        registry.register_error_state_bindings();

        // Register global bindings for all view states
        registry.register_global_bindings(KeybindingContext::View(ViewState::LogView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::SelectionMode));
        registry.register_global_bindings(KeybindingContext::View(ViewState::ActiveSearchMode));
        registry.register_global_bindings(KeybindingContext::View(ViewState::ActiveFilterMode));
        registry.register_global_bindings(KeybindingContext::View(ViewState::FilterView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::OptionsView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::EventsView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::MarksView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::FilesView));
        registry.register_global_bindings(KeybindingContext::View(ViewState::GotoLineMode));

        // Register global bindings for all overlay types
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::EditFilter));
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::EventsFilter));
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::MarkName));
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::SaveToFile));
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::Message(String::new())));
        registry.register_global_bindings(KeybindingContext::Overlay(Overlay::Error(String::new())));

        registry
    }

    fn find_cmd(
        bindings: &[((KeybindingContext, KeyCode, KeyModifiers), Command)],
        expected_context: &KeybindingContext,
        key_event: KeyEvent,
    ) -> Option<Command> {
        bindings
            .iter()
            .find(|((context, kcode, kmod), _)| {
                context == expected_context && *kcode == key_event.code && *kmod == key_event.modifiers
            })
            .map(|(_, cmd)| *cmd)
    }

    pub fn lookup(&self, view_state: &ViewState, overlay: &Option<Overlay>, key_event: KeyEvent) -> Option<Command> {
        // Check for overlay specific bindings if an overlay is active
        if let Some(ov) = overlay {
            return Self::find_cmd(
                &self.bindings,
                &KeybindingContext::Overlay(self.get_overlay_type(ov)),
                key_event,
            );
        }

        // Check for bindings relating to views
        Self::find_cmd(&self.bindings, &KeybindingContext::View(view_state.clone()), key_event)
    }

    // Replace the string with empty one to be able to match on the enum value
    fn get_overlay_type(&self, overlay: &Overlay) -> Overlay {
        match overlay {
            Overlay::Message(_) => Overlay::Message(String::new()),
            Overlay::Error(_) => Overlay::Error(String::new()),
            other => other.clone(),
        }
    }

    /// Returns all keybindings for a specific context, grouped and sorted.
    pub fn get_keybindings_for_context(&self, target_context: &KeybindingContext) -> Vec<(String, Command)> {
        let bindings: Vec<(String, Command)> = self
            .bindings
            .iter()
            .filter(|((context, _, _), _)| context == target_context)
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
    fn bind(&mut self, context: KeybindingContext, keycode: KeyCode, modifiers: KeyModifiers, command: Command) {
        self.bindings.push(((context, keycode, modifiers), command));
    }

    /// Helper to register a keybinding without modifiers.
    fn bind_simple(&mut self, context: KeybindingContext, keycode: KeyCode, command: Command) {
        self.bind(context, keycode, KeyModifiers::empty(), command);
    }

    /// Helper to register a keybinding with SHIFT modifier.
    fn bind_shift(&mut self, context: KeybindingContext, c: char, command: Command) {
        self.bind(context, KeyCode::Char(c), KeyModifiers::SHIFT, command);
    }

    /// Registers global keybindings that work in all states.
    fn register_global_bindings(&mut self, context: KeybindingContext) {
        self.bind(
            context.clone(),
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
            Command::Quit,
        );
        self.bind_simple(context.clone(), KeyCode::Esc, Command::Cancel);
        self.bind_simple(context.clone(), KeyCode::Enter, Command::Confirm);
        self.bind_simple(context.clone(), KeyCode::F(1), Command::ToggleHelp);
    }

    fn register_log_view_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::LogView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char('g'), Command::GotoTop);
        self.bind_shift(context.clone(), 'G', Command::GotoBottom);
        self.bind_simple(context.clone(), KeyCode::Char('z'), Command::CenterSelected);
        self.bind_simple(context.clone(), KeyCode::Left, Command::ScrollLeft);
        self.bind_simple(context.clone(), KeyCode::Right, Command::ScrollRight);
        self.bind_simple(context.clone(), KeyCode::Char('h'), Command::ScrollLeft);
        self.bind_simple(context.clone(), KeyCode::Char('l'), Command::ScrollRight);
        self.bind_simple(context.clone(), KeyCode::Char('0'), Command::ResetHorizontal);
        self.bind_simple(context.clone(), KeyCode::Char('/'), Command::ActivateActiveSearchMode);
        self.bind(
            context.clone(),
            KeyCode::Char('f'),
            KeyModifiers::CONTROL,
            Command::ActivateActiveSearchMode,
        );
        self.bind_simple(context.clone(), KeyCode::Char('n'), Command::SearchNext);
        self.bind_shift(context.clone(), 'N', Command::SearchPrevious);
        self.bind_simple(context.clone(), KeyCode::Char('f'), Command::ActivateActiveFilterMode);
        self.bind_shift(context.clone(), 'F', Command::ActivateFilterView);
        self.bind_simple(context.clone(), KeyCode::Char(':'), Command::ActivateGotoLineMode);
        self.bind_simple(context.clone(), KeyCode::Char('o'), Command::ActivateOptionsView);
        self.bind_simple(context.clone(), KeyCode::Char('e'), Command::ActivateEventsView);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleMark);
        self.bind_simple(context.clone(), KeyCode::Char('m'), Command::ActivateMarksView);
        self.bind_simple(context.clone(), KeyCode::Char('i'), Command::ActivateFilesView);
        self.bind_simple(context.clone(), KeyCode::Char(']'), Command::MarkNext);
        self.bind_simple(context.clone(), KeyCode::Char('['), Command::MarkPrevious);
        self.bind_simple(context.clone(), KeyCode::Char('}'), Command::EventNext);
        self.bind_simple(context.clone(), KeyCode::Char('{'), Command::EventPrevious);
        self.bind_simple(context.clone(), KeyCode::Char('x'), Command::ToggleExpansion);
        self.bind_shift(context.clone(), 'X', Command::CollapseAll);
        self.bind_simple(context.clone(), KeyCode::Char('c'), Command::ToggleCenterCursorMode);
        self.bind_simple(context.clone(), KeyCode::Char('t'), Command::ToggleFollowMode);
        self.bind_simple(context.clone(), KeyCode::Char('p'), Command::TogglePauseMode);
        self.bind(
            context.clone(),
            KeyCode::Char('l'),
            KeyModifiers::CONTROL,
            Command::ClearLogBuffer,
        );
        self.bind(
            context.clone(),
            KeyCode::Char('s'),
            KeyModifiers::CONTROL,
            Command::ActivateSaveToFileMode,
        );
        self.bind(
            context.clone(),
            KeyCode::Char('o'),
            KeyModifiers::CONTROL,
            Command::HistoryBack,
        );
        self.bind_simple(context.clone(), KeyCode::Tab, Command::HistoryForward);
        self.bind_shift(context.clone(), 'V', Command::StartSelection);
    }

    fn register_selection_mode_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::SelectionMode);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char('g'), Command::GotoTop);
        self.bind_shift(context.clone(), 'G', Command::GotoBottom);
        self.bind_simple(context.clone(), KeyCode::Char('y'), Command::CopySelection);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleMark);
    }

    fn register_search_mode_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::ActiveSearchMode);

        self.bind_simple(context.clone(), KeyCode::Tab, Command::TabCompletion);
        self.bind(
            context.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleCaseSearch,
        );
        self.bind_simple(context.clone(), KeyCode::Up, Command::SearchHistoryPrevious);
        self.bind_simple(context.clone(), KeyCode::Down, Command::SearchHistoryNext);
    }

    fn register_filter_mode_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::ActiveFilterMode);

        self.bind_simple(context.clone(), KeyCode::Tab, Command::TabCompletion);
        self.bind(
            context.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleCaseFilter,
        );
        self.bind(
            context.clone(),
            KeyCode::Char('e'),
            KeyModifiers::ALT,
            Command::ToggleActiveFilterModeInOut,
        );
        self.bind_simple(context.clone(), KeyCode::Up, Command::FilterHistoryPrevious);
        self.bind_simple(context.clone(), KeyCode::Down, Command::FilterHistoryNext);
    }

    fn register_filter_list_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::FilterView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleFilterPattern);
        self.bind_simple(context.clone(), KeyCode::Delete, Command::RemoveFilterPattern);
        self.bind_simple(context.clone(), KeyCode::Char('d'), Command::RemoveFilterPattern);
        self.bind_simple(
            context.clone(),
            KeyCode::Char('e'),
            Command::ActivateEditActiveFilterMode,
        );
        self.bind_simple(context.clone(), KeyCode::Char('f'), Command::ActivateActiveFilterMode);
        self.bind_simple(context.clone(), KeyCode::Char('a'), Command::ToggleAllFilterPatterns);
        self.bind(
            context.clone(),
            KeyCode::Char('a'),
            KeyModifiers::ALT,
            Command::ToggleFilterPatternCaseSensitive,
        );
        self.bind(
            context.clone(),
            KeyCode::Char('e'),
            KeyModifiers::ALT,
            Command::ToggleFilterPatternMode,
        );
    }

    fn register_options_view_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::OptionsView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleOption);
    }

    fn register_events_view_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::EventsView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_shift(context.clone(), 'F', Command::ActivateEventFilterView);
        self.bind_shift(context.clone(), 'M', Command::ToggleEventsShowMarks);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::GotoSelectedEvent);
        self.bind_simple(context.clone(), KeyCode::Char('e'), Command::ActivateMarkNameMode);
    }

    fn register_event_filter_view_bindings(&mut self) {
        let context = KeybindingContext::Overlay(Overlay::EventsFilter);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleEventFilter);
        self.bind_simple(context.clone(), KeyCode::Char('a'), Command::ToggleAllEventFilters);
    }

    fn register_marks_view_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::MarksView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::GotoSelectedMark);
        self.bind_simple(context.clone(), KeyCode::Delete, Command::UnmarkSelected);
        self.bind_simple(context.clone(), KeyCode::Char('d'), Command::UnmarkSelected);
        self.bind_simple(context.clone(), KeyCode::Char('e'), Command::ActivateMarkNameMode);
        self.bind_simple(context.clone(), KeyCode::Char('c'), Command::ClearAllMarks);
        self.bind_shift(context.clone(), 'F', Command::ToggleShowMarkedOnly)
    }

    fn register_files_view_bindings(&mut self) {
        let context = KeybindingContext::View(ViewState::FilesView);

        self.bind_simple(context.clone(), KeyCode::Char('q'), Command::Quit);
        self.bind_simple(context.clone(), KeyCode::Up, Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Down, Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::Char('k'), Command::MoveUp);
        self.bind_simple(context.clone(), KeyCode::Char('j'), Command::MoveDown);
        self.bind_simple(context.clone(), KeyCode::PageUp, Command::PageUp);
        self.bind_simple(context.clone(), KeyCode::PageDown, Command::PageDown);
        self.bind_simple(context.clone(), KeyCode::Char(' '), Command::ToggleFile);
    }

    fn register_message_state_bindings(&mut self) {
        let context = KeybindingContext::Overlay(Overlay::Message(String::new()));

        self.bind_simple(context, KeyCode::Char('q'), Command::Quit);
    }

    fn register_error_state_bindings(&mut self) {
        let context = KeybindingContext::Overlay(Overlay::Error(String::new()));

        self.bind_simple(context, KeyCode::Char('q'), Command::Quit);
    }
}
