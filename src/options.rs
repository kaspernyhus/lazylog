use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppOption {
    HideTimestamp,
    DisableColors,
    SearchDisableJumping,
    AlwaysShowMarkedLines,
    AlwaysShowCriticalEvents,
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
}

impl Default for AppOptions {
    fn default() -> Self {
        AppOptions {
            options: vec![
                AppOptionDef::new(AppOption::HideTimestamp, "Hide Timestamp & Hostname", OptionAction::LineTransform(
                        Regex::new(r"^(?:\w{3}\s+\d{2}\s+\d{2}:\d{2}:\d{2}\s+\S+\s+|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+[+-]\d{4}\s+)").unwrap()
                    )),
                AppOptionDef::new_toggle(AppOption::DisableColors, "Disable Colors"),
                AppOptionDef::new_toggle(AppOption::SearchDisableJumping, "Search: Disable jumping to match"),
                AppOptionDef::new_toggle(AppOption::AlwaysShowMarkedLines, "Always show marked lines"),
                AppOptionDef::new_toggle(AppOption::AlwaysShowCriticalEvents, "Always show critical events"),
            ],
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

    /// Toggles the enabled state of an option at the given index.
    pub fn toggle_option(&mut self, index: usize) {
        if let Some(option) = self.options.get_mut(index) {
            option.enabled = !option.enabled;
        }
    }

    /// Enables an option at the given index (sets it to true).
    pub fn enable_option(&mut self, index: usize) {
        if let Some(option) = self.options.get_mut(index) {
            option.enabled = true;
        }
    }

    /// Get the option at the given index.
    pub fn get(&self, index: usize) -> Option<&AppOptionDef> {
        self.options.get(index)
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
