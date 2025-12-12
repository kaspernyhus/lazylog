use crate::list_view_state::ListViewState;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppOption {
    HideTimestamp,
    DisableColors,
    SearchDisableJumping,
    AlwaysShowMarkedLines,
}

#[derive(Debug, Clone)]
pub enum OptionAction {
    LineTransform(Regex),
    Toggle,
}

#[derive(Debug, Clone)]
pub struct AppOptionDef {
    pub option: AppOption,
    pub description: &'static str,
    pub action: OptionAction,
    pub enabled: bool,
}

impl AppOptionDef {
    pub fn new(option: AppOption, description: &'static str, action: OptionAction) -> Self {
        AppOptionDef {
            option,
            description,
            action,
            enabled: false,
        }
    }

    pub fn new_toggle(option: AppOption, description: &'static str) -> Self {
        AppOptionDef {
            option,
            description,
            action: OptionAction::Toggle,
            enabled: false,
        }
    }

    pub fn get_description(&self) -> &'static str {
        self.description
    }
}

/// Manages app options.
#[derive(Debug)]
pub struct AppOptions {
    /// Vector of option definitions.
    options: Vec<AppOptionDef>,
    /// View state for the options list.
    view: ListViewState,
}

impl Default for AppOptions {
    fn default() -> Self {
        let options = vec![
            AppOptionDef::new(AppOption::HideTimestamp, "Hide Timestamp & Hostname", OptionAction::LineTransform(
                    Regex::new(r"^(?:\w{3}\s+\d{2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[+-]\d{4}\s+)").unwrap()
                )),
            AppOptionDef::new_toggle(AppOption::DisableColors, "Disable Colors"),
            AppOptionDef::new_toggle(AppOption::SearchDisableJumping, "Search: Disable jumping to match"),
            AppOptionDef::new_toggle(AppOption::AlwaysShowMarkedLines, "Always show marked lines"),

        ];

        let num_options = options.len();

        AppOptions {
            options,
            view: ListViewState::new_with_count(num_options),
        }
    }
}

impl AppOptions {
    /// Number of options.
    pub fn count(&self) -> usize {
        self.options.len()
    }

    pub fn is_empty(&self) -> bool {
        self.options.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &AppOptionDef> {
        self.options.iter()
    }

    pub fn is_enabled(&self, option: AppOption) -> bool {
        self.options
            .iter()
            .find(|opt| opt.option == option)
            .map(|opt| opt.enabled)
            .unwrap_or(false)
    }

    pub fn is_disabled(&self, option: AppOption) -> bool {
        !self.is_enabled(option)
    }

    pub fn enable(&mut self, option: AppOption) {
        if let Some(opt) = self.options.iter_mut().find(|opt| opt.option == option) {
            opt.enabled = true;
        }
    }

    /// Applies all enabled line transform options to a line.
    pub fn apply_to_line<'a>(&self, line: &'a str) -> &'a str {
        for opt in &self.options {
            if !opt.enabled {
                continue;
            }

            match &opt.action {
                OptionAction::LineTransform(pattern) => {
                    let mut offset = 0;
                    // Find the maximum offset to skip, but only from the start of the line
                    if let Some(m) = pattern.find(line)
                        && m.start() == 0
                    {
                        offset = offset.max(m.end());
                    }
                    return &line[offset..];
                }
                OptionAction::Toggle => {}
            }
        }

        line
    }

    /// Toggles the enabled state of the currently selected option.
    pub fn toggle_selected_option(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.options.len() {
            self.options[selected].enabled = !self.options[selected].enabled;
        }
    }

    /// Enables the currently selected option (sets it to true).
    pub fn enable_selected_option(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.options.len() {
            self.options[selected].enabled = true;
        }
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.view.selected_index()
    }

    /// Get the option at the selected index.
    pub fn get_selected_option(&self) -> Option<&AppOptionDef> {
        self.options.get(self.view.selected_index())
    }

    /// Moves the selection to the previous option, wrapping to the end.
    pub fn move_selection_up(&mut self) {
        self.view.move_up_wrap();
    }

    /// Moves the selection to the next option, wrapping to the beginning.
    pub fn move_selection_down(&mut self) {
        self.view.move_down_wrap();
    }

    /// Restore options from a saved state.
    pub fn restore(&mut self, saved_options: &[(AppOption, bool)]) {
        for (option, enabled) in saved_options {
            if let Some(option_def) = self.options.iter_mut().find(|opt| opt.option == *option) {
                option_def.enabled = *enabled;
            }
        }
    }
}
