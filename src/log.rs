use crate::{
    filter::{Filter, FilterMode, FilterPattern},
    utils::contains_ignore_case,
};

/// A single log line with its content and original index.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// The text content of the log line.
    pub content: String,
    /// The original index of the line in the source.
    pub index: usize,
}

/// Buffer for storing and managing log lines with filtering support.
#[derive(Debug, Default)]
pub struct LogBuffer {
    /// Optional path to the file being viewed.
    pub file_path: Option<String>,
    /// All log lines (unfiltered).
    pub lines: Vec<LogLine>,
    /// Indices of lines that pass the applied filters.
    active_lines: Vec<usize>,
    /// Whether the buffer is in streaming mode (reading from stdin).
    pub streaming: bool,
}

/// Specifies which interval range of lines (potentially with filters) to retrieve from the buffer.
#[derive(Debug)]
pub enum Interval {
    /// All active lines.
    All,
    /// Range of active lines by index (start, end).
    Range(usize, usize),
}

impl LogLine {
    /// Creates a new log line.
    pub fn new(content: String, index: usize) -> Self {
        Self { content, index }
    }

    /// Returns the log message content of the log line.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl LogBuffer {
    /// Loads log lines from a file. (Not streaming mode.)
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

    /// Initializes the buffer for stdin streaming mode.
    pub fn init_stdin_mode(&mut self) {
        self.file_path = Some("<stdin>".to_string());
        self.streaming = true;
        self.lines.clear();
        self.clear_filters();
    }

    /// Appends a new line to the buffer.
    ///
    /// Returns a reference to the newly created LogLine.
    pub fn append_line(&mut self, content: String) -> &LogLine {
        let index = self.lines.len();
        let log_line = LogLine::new(content, index);
        self.lines.push(log_line);
        &self.lines[index]
    }

    /// Checks if a log line passes the given filters.
    pub fn check_line_passes_filters(&self, content: &str, filter: &Filter) -> bool {
        let temp_line = LogLine {
            content: content.to_string(),
            index: 0,
        };
        self.line_passes_filters(&temp_line, filter.get_filter_patterns())
    }

    /// Adds a line index to the active lines list.
    pub fn add_to_active_lines(&mut self, index: usize) {
        self.active_lines.push(index);
    }

    /// Applies the given filter to all lines in the buffer.
    ///
    /// Rebuilds the active_lines list based on the filter criteria.
    pub fn apply_filters(&mut self, filter: &Filter) {
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

    /// Clears all filters.
    pub fn clear_filters(&mut self) {
        self.active_lines = (0..self.lines.len()).collect();
    }

    /// Remove all lines and filters from the buffer. (Only in streaming mode.)
    pub fn clear_all(&mut self) {
        if self.streaming {
            self.lines.clear();
            self.active_lines.clear();
        }
    }

    /// Saves all log lines to a file.
    pub fn save_to_file(&self, path: &str) -> color_eyre::Result<()> {
        use std::io::Write;
        let mut file = std::fs::File::create(path)?;
        for line in &self.lines {
            writeln!(file, "{}", line.content)?;
        }
        Ok(())
    }

    /// Returns an iterator over active log lines in the specified interval.
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

    /// Returns a reference to a log line by its original index.
    pub fn get_line(&self, line_index: usize) -> Option<&LogLine> {
        if line_index >= self.lines.len() {
            return None;
        }
        Some(&self.lines[line_index])
    }

    /// Returns the maximum line length in the specified interval.
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

    /// Returns the count of active (filtered) lines.
    pub fn get_lines_count(&self) -> usize {
        self.active_lines.len()
    }

    /// Returns a reference to the active lines (original indices of visible lines).
    pub fn get_active_lines(&self) -> &[usize] {
        &self.active_lines
    }

    /// Returns the original log line index for an active line index.
    ///
    /// Returns `None` if the line_index is out of bounds.
    pub fn viewport_to_log_index(&self, line_index: usize) -> Option<usize> {
        if line_index >= self.active_lines.len() {
            return None;
        }
        let actual_index = self.active_lines[line_index];
        Some(self.lines[actual_index].index)
    }

    /// Finds the active line index for a given original log line index.
    ///
    /// Returns `None` if the line is not active.
    pub fn find_line(&self, log_index: usize) -> Option<usize> {
        self.active_lines
            .iter()
            .position(|&active_line_index| self.lines[active_line_index].index == log_index)
    }

    /// Finds the active line index closest to a target original log line index.
    ///
    /// Useful for maintaining cursor position when filters change.
    /// Returns `None` if there are no active lines.
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
            contains_ignore_case(line, pattern)
        }
    }
}
