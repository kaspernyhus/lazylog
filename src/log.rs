use crate::filter::{Filter, FilterMode, FilterPattern};

#[derive(Debug, Clone)]
pub struct LogLine {
    pub content: String,
    pub index: usize,
}

#[derive(Debug, Default)]
pub struct LogBuffer {
    pub file_path: Option<String>,
    pub lines: Vec<LogLine>,
    active_lines: Vec<usize>, // Indices of lines that pass the applied filters
    pub streaming: bool,
}

#[derive(Debug)]
pub enum Interval {
    All,
    Range(usize, usize),
}

impl LogLine {
    pub fn new(content: String, index: usize) -> Self {
        Self { content, index }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

impl LogBuffer {
    pub fn load_from_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.file_path = Some(path.to_string());
        self.streaming = false;
        self.lines = content
            .lines()
            .enumerate()
            .map(|(index, line)| LogLine::new(line.to_string(), index))
            .collect();
        self.clear_filters();
        Ok(())
    }

    pub fn init_stdin_mode(&mut self) {
        self.file_path = Some("<stdin>".to_string());
        self.streaming = true;
        self.lines.clear();
        self.clear_filters();
    }

    pub fn append_line(&mut self, content: String, filter: &Filter) -> bool {
        let index = self.lines.len();
        let log_line = LogLine::new(content, index);
        let passes_filter = self.line_passes_filters(&log_line, filter.get_filter_patterns());
        if passes_filter {
            self.active_lines.push(log_line.index);
        }
        self.lines.push(log_line);
        passes_filter
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
                .filter(|(_, log_line)| self.line_passes_filters(log_line, filter_patterns))
                .map(|(index, _)| index)
                .collect();
        }
    }

    pub fn get_lines_iter(&self, interval: Interval) -> impl Iterator<Item = &LogLine> {
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

        active_indices.iter().map(move |&idx| &self.lines[idx])
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
            .map(|&idx| self.lines[idx].content.len())
            .max()
            .unwrap_or(0)
    }

    pub fn get_lines_count(&self) -> usize {
        self.active_lines.len()
    }

    pub fn get_log_line_index(&self, line_index: usize) -> Option<usize> {
        if line_index >= self.active_lines.len() {
            return None;
        }
        let actual_index = self.active_lines[line_index];
        Some(self.lines[actual_index].index)
    }

    pub fn find_closest_line_by_index(&self, target_log_line_index: usize) -> Option<usize> {
        if self.active_lines.is_empty() {
            return None;
        }

        let mut best_match = 0;
        let mut min_distance = usize::MAX;

        for (active_lines_index, &lines_index) in self.active_lines.iter().enumerate() {
            let log_line_index = self.lines[lines_index].index;
            let distance = if log_line_index >= target_log_line_index {
                log_line_index - target_log_line_index
            } else {
                target_log_line_index - log_line_index
            };

            if distance < min_distance {
                min_distance = distance;
                best_match = active_lines_index;
            }
        }

        Some(best_match)
    }

    fn line_passes_filters(&self, line: &LogLine, filters: &[FilterPattern]) -> bool {
        // Exclude filters take precedence
        for filter in filters
            .iter()
            .filter(|f| f.enabled && f.mode == FilterMode::Exclude)
        {
            if self.line_matches_pattern(&line.content, &filter.pattern, filter.case_sensitive) {
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
                    self.line_matches_pattern(&line.content, &filter.pattern, filter.case_sensitive)
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
