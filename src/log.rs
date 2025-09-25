use crate::filter::{Filter, FilterMode, FilterPattern};

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub file_path: Option<String>,
    pub lines: Vec<String>,
    filtered_lines: Vec<usize>,
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
        self.rebuild_filtered_view(filter);
    }

    pub fn clear_filters(&mut self) {
        self.filtered_lines = (0..self.lines.len()).collect();
    }

    fn rebuild_filtered_view(&mut self, filter: &Filter) {
        let filter_patterns = filter.get_filter_patterns();
        if filter_patterns.is_empty() {
            self.clear_filters();
        } else {
            self.filtered_lines = self
                .lines
                .iter()
                .enumerate()
                .filter(|(_, line)| self.line_passes_filters(line, filter_patterns))
                .map(|(index, _)| index)
                .collect();
        }
    }

    pub fn get_lines_iter(&self, interval: Option<(usize, usize)>) -> impl Iterator<Item = &str> {
        let filtered_indices = match interval {
            Some((start, end)) => {
                let end_idx = end.min(self.filtered_lines.len());
                if start >= self.filtered_lines.len() {
                    &[]
                } else {
                    &self.filtered_lines[start..end_idx]
                }
            }
            None => &self.filtered_lines[..],
        };

        filtered_indices
            .iter()
            .map(move |&idx| self.lines[idx].as_str())
    }

    pub fn get_lines_max_length(&self, start: usize, end: usize) -> usize {
        let end_idx = end.min(self.filtered_lines.len());
        if start >= self.filtered_lines.len() {
            return 0;
        }

        self.filtered_lines[start..end_idx]
            .iter()
            .map(|&idx| self.lines[idx].len())
            .max()
            .unwrap_or(0)
    }

    pub fn get_lines_count(&self) -> usize {
        self.filtered_lines.len()
    }

    pub fn debug_filter_state(&self) -> (usize, usize) {
        (self.lines.len(), self.filtered_lines.len())
    }

    fn line_passes_filters(&self, line: &str, filters: &[FilterPattern]) -> bool {
        let active_filters: Vec<&FilterPattern> = filters.iter().filter(|f| f.enabled).collect();

        if active_filters.is_empty() {
            return true;
        }

        let include_filters: Vec<&FilterPattern> = active_filters
            .iter()
            .filter(|f| f.mode == FilterMode::Include)
            .copied()
            .collect();

        let exclude_filters: Vec<&FilterPattern> = active_filters
            .iter()
            .filter(|f| f.mode == FilterMode::Exclude)
            .copied()
            .collect();

        for filter in &exclude_filters {
            if self.line_matches_pattern(line, &filter.pattern, filter.case_sensitive) {
                return false;
            }
        }

        if !include_filters.is_empty() {
            for filter in &include_filters {
                if self.line_matches_pattern(line, &filter.pattern, filter.case_sensitive) {
                    return true;
                }
            }
            return false;
        }

        true
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
