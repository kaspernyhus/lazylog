#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum FilterMode {
    #[default]
    Include,
    Exclude,
}

#[derive(Debug, Clone)]
pub struct FilterPattern {
    pub pattern: String,
    pub mode: FilterMode,
    pub enabled: bool,
    pub case_sensitive: bool,
}

impl FilterPattern {
    pub fn new(pattern: String, mode: FilterMode, case_sensitive: bool) -> Self {
        Self {
            pattern,
            mode,
            enabled: true,
            case_sensitive,
        }
    }
}

#[derive(Debug, Default)]
pub struct FilterList {
    pub patterns: Vec<FilterPattern>,
    pub selected_index: usize,
}

#[derive(Debug, Default)]
pub struct Filter {
    filter_list: FilterList,
    filter_pattern: Option<String>,
    filter_mode: FilterMode,
    case_sensitive: bool,
}

impl Filter {
    pub fn set_filter_pattern(&mut self, pattern: String) {
        self.filter_pattern = Some(pattern);
    }

    pub fn update_filter_pattern(&mut self, input: &str, min_chars: usize) {
        if input.is_empty() {
            self.clear_filter_pattern();
            return;
        }

        if input.len() >= min_chars {
            self.set_filter_pattern(input.to_string());
        }
    }

    pub fn get_filter_pattern(&self) -> Option<&str> {
        self.filter_pattern.as_deref()
    }

    pub fn clear_filter_pattern(&mut self) {
        self.filter_pattern = None;
    }

    pub fn toggle_mode(&mut self) {
        self.filter_mode = match self.filter_mode {
            FilterMode::Include => FilterMode::Exclude,
            FilterMode::Exclude => FilterMode::Include,
        };
    }

    pub fn get_mode(&self) -> &FilterMode {
        &self.filter_mode
    }

    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn add_filter(&mut self, pattern: String) {
        if !self.pattern_exists(&pattern, self.filter_mode) {
            self.filter_list.patterns.push(FilterPattern::new(
                pattern.clone(),
                self.filter_mode,
                self.case_sensitive,
            ));
        }
        self.clear_filter_pattern();
    }

    fn pattern_exists(&self, pattern: &str, mode: FilterMode) -> bool {
        self.filter_list
            .patterns
            .iter()
            .any(|fp| fp.pattern == pattern && fp.mode == mode)
    }

    pub fn get_filter_patterns(&self) -> &[FilterPattern] {
        &self.filter_list.patterns
    }

    pub fn get_selected_pattern_index(&self) -> usize {
        self.filter_list.selected_index
    }

    pub fn move_selection_up(&mut self) {
        if !self.filter_list.patterns.is_empty() {
            self.filter_list.selected_index = if self.filter_list.selected_index == 0 {
                self.filter_list.patterns.len() - 1
            } else {
                self.filter_list.selected_index - 1
            };
        }
    }

    pub fn move_selection_down(&mut self) {
        if !self.filter_list.patterns.is_empty() {
            self.filter_list.selected_index =
                (self.filter_list.selected_index + 1) % self.filter_list.patterns.len();
        }
    }

    pub fn toggle_selected_pattern(&mut self) {
        if self.filter_list.selected_index < self.filter_list.patterns.len() {
            self.filter_list.patterns[self.filter_list.selected_index].enabled =
                !self.filter_list.patterns[self.filter_list.selected_index].enabled;
        }
    }

    pub fn remove_selected_pattern(&mut self) {
        if self.filter_list.selected_index < self.filter_list.patterns.len() {
            self.filter_list
                .patterns
                .remove(self.filter_list.selected_index);
            if self.filter_list.selected_index >= self.filter_list.patterns.len()
                && !self.filter_list.patterns.is_empty()
            {
                self.filter_list.selected_index = self.filter_list.patterns.len() - 1;
            }
        }
    }
}
