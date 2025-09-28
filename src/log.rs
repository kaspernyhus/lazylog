use crate::filter::{Filter, FilterMode, FilterPattern};

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub file_path: Option<String>,
    pub lines: Vec<String>,
    active_lines: Vec<usize>, // Indices of lines that pass the current
}

#[derive(Debug)]
pub enum Interval {
    All,
    Range(usize, usize),
}

impl LogBuffer {
    pub fn load_from_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.file_path = Some(path.to_string());
        self.lines = content.lines().map(|line| line.to_string()).collect();
        self.clear_filters();
        Ok(())
    }

    pub fn apply_filters(&mut self, filter: &Filter) {
        self.rebuild_active_lines(filter);
    }

    pub fn clear_filters(&mut self) {
        self.active_lines = (0..self.lines.len()).collect();
    }

    fn rebuild_active_lines(&mut self, filter: &Filter) {
        let filter_patterns = filter.get_filter_patterns();
        if filter_patterns.is_empty() {
            self.clear_filters();
        } else {
            self.active_lines = self
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| self.line_passes_filters(line, filter_patterns))
                .map(|(index, _)| index)
                .collect();
        }
    }

    pub fn get_lines_iter(&self, interval: Interval) -> impl Iterator<Item = &str> {
        let active_indices = match interval {
            Interval::All => &self.active_lines[..],
            Interval::Range(start_index, end) => {
                let end_index = end.min(self.active_lines.len());
                if start_index >= self.active_lines.len() {
                    &[]
                } else {
                    &self.active_lines[start_index..end_index]
                }
            }
        };

        active_indices
            .iter()
            .map(move |&idx| self.lines[idx].as_str())
    }

    pub fn get_lines_max_length(&self, interval: Interval) -> usize {
        let (start_index, end) = match interval {
            Interval::All => (0, self.active_lines.len()),
            Interval::Range(s, e) => (s, e),
        };

        let end_index = end.min(self.active_lines.len());
        if start_index >= self.active_lines.len() {
            return 0;
        }

        self.active_lines[start_index..end_index]
            .iter()
            .map(|&idx| self.lines[idx].len())
            .max()
            .unwrap_or(0)
    }

    pub fn get_lines_count(&self) -> usize {
        self.active_lines.len()
    }

    fn line_passes_filters(&self, line: &str, filters: &[FilterPattern]) -> bool {
        // Exclude filters take precedence
        for filter in filters
            .iter()
            .filter(|f| f.enabled && f.mode == FilterMode::Exclude)
        {
            if self.line_matches_pattern(line, &filter.pattern, filter.case_sensitive) {
                return false;
            }
        }

        // Check for Include filters
        let has_include_filters = filters
            .iter()
            .any(|f| f.enabled && f.mode == FilterMode::Include);

        if has_include_filters {
            filters
                .iter()
                .filter(|f| f.enabled && f.mode == FilterMode::Include)
                .any(|filter| {
                    self.line_matches_pattern(line, &filter.pattern, filter.case_sensitive)
                })
        } else {
            true
        }
    }

    fn line_matches_pattern(&self, line: &str, pattern: &str, case_sensitive: bool) -> bool {
        if case_sensitive {
            line.contains(pattern)
        } else {
            Self::contains_ignore_case(line, pattern)
        }
    }

    fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
        if needle.is_empty() {
            return true;
        }
        if needle.len() > haystack.len() {
            return false;
        }

        haystack
            .as_bytes()
            .windows(needle.len())
            .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
    }
}
