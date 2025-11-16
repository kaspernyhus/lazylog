use crate::{
    filter::{Filter, apply_filters},
    timestamp::parse_timestamp,
};
use chrono::{DateTime, Utc};
use rayon::prelude::*;

/// A single log line with its content and original index.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// The text content of the log line.
    pub content: String,
    /// The original index of the line in the source.
    pub index: usize,
    /// Parsed timestamp (if applicable).
    pub timestamp: Option<DateTime<Utc>>,
}

/// Buffer for storing and managing log lines with filtering support.
#[derive(Debug, Default)]
pub struct LogBuffer {
    /// Optional path to the file being viewed.
    pub file_path: Option<String>,
    /// All log lines (unfiltered).
    lines: Vec<LogLine>,
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
        Self {
            content,
            index,
            timestamp: None,
        }
    }

    pub fn with_timestamp(content: String, index: usize, timestamp: DateTime<Utc>) -> Self {
        Self {
            content,
            index,
            timestamp: Some(timestamp),
        }
    }

    /// Returns the log message content of the log line.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl LogBuffer {
    /// Loads log lines from a file. (Not streaming mode.)
    pub fn load_file(&mut self, path: &str) -> color_eyre::Result<()> {
        let content = std::fs::read_to_string(path)?;
        self.file_path = Some(path.to_string());
        self.streaming = false;
        self.lines = content
            .lines()
            .enumerate()
            .map(|(index, line)| LogLine::new(line.to_string(), index))
            .collect();
        self.reset_active_lines();
        Ok(())
    }

    pub fn load_files(&mut self, paths: &[String]) -> color_eyre::Result<()> {
        if paths.is_empty() {
            return Err(color_eyre::eyre::eyre!("No files provided"));
        }

        let mut total_lines_skipped = 0;
        self.streaming = false;
        self.file_path = Some(paths.join(", "));

        for path in paths.iter() {
            let content = std::fs::read_to_string(path)?;
            let base_index = self.lines.len();

            let mut file_lines: Vec<LogLine> = content
                .lines()
                .enumerate()
                .filter_map(|(line_num, line)| {
                    let index = base_index + line_num;
                    if let Some(timestamp) = parse_timestamp(line) {
                        Some(LogLine::with_timestamp(line.to_string(), index, timestamp))
                    } else {
                        total_lines_skipped += 1;
                        None
                    }
                })
                .collect();

            self.lines.append(&mut file_lines);
        }

        self.reset_active_lines();

        if total_lines_skipped > 0 {
            return Err(color_eyre::eyre::eyre!(
                "Failed to parse timestamp for {} lines",
                total_lines_skipped
            ));
        }

        self.lines.sort_by_key(|line| line.timestamp);

        for (new_index, line) in self.lines.iter_mut().enumerate() {
            line.index = new_index;
        }

        Ok(())
    }

    /// Initializes the buffer for stdin streaming mode.
    pub fn init_stdin_mode(&mut self) {
        self.file_path = Some("<stdin>".to_string());
        self.streaming = true;
        self.lines.clear();
        self.reset_active_lines();
    }

    /// Appends a new line to the buffer.
    ///
    /// Returns the index of the newly created LogLine.
    pub fn append_line(&mut self, content: String) -> usize {
        let index = self.lines.len();
        let log_line = LogLine::new(content, index);
        self.lines.push(log_line);
        index
    }

    /// Adds a line index to the active lines list.
    pub fn add_to_active_lines(&mut self, index: usize) {
        self.active_lines.push(index);
    }

    /// Applies the filters to all lines in the buffer.
    pub fn apply_filtering(
        &mut self,
        filter: &Filter,
        marked_indices: &[usize],
        show_marked_lines_only: bool,
        should_show_marked: bool,
    ) {
        let filter_patterns = filter.get_filter_patterns();
        if filter_patterns.is_empty() {
            self.reset_active_lines();
        } else {
            self.active_lines = self
                .lines
                .par_iter()
                .enumerate()
                .filter_map(|(index, log_line)| {
                    let is_marked_line = marked_indices.binary_search(&index).is_ok();

                    if show_marked_lines_only {
                        if is_marked_line {
                            return Some(index);
                        } else {
                            return None;
                        }
                    }

                    if is_marked_line && should_show_marked {
                        return Some(index);
                    }

                    // par_iter needs to call the stand alone apply_filters function to be thread safe
                    let passes_text_filter = apply_filters(&log_line.content, filter_patterns);

                    if passes_text_filter { Some(index) } else { None }
                })
                .collect();
        }
    }

    /// Clears all filters.
    fn reset_active_lines(&mut self) {
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
    pub fn get_active_lines_iter(&self, interval: Interval) -> impl Iterator<Item = &LogLine> {
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
    pub fn get_active_lines_count(&self) -> usize {
        self.active_lines.len()
    }

    /// Returns the total count of log lines.
    pub fn get_total_lines_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns a reference to the active lines (original indices of visible lines).
    pub fn get_active_lines(&self) -> &[usize] {
        &self.active_lines
    }

    /// Returns an iterator over all log lines without active line filtering.
    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
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
            let distance = log_line_index.abs_diff(target_log_line_index);

            if distance < min_distance {
                min_distance = distance;
                best_match = active_lines_index;
            }
        }

        Some(best_match)
    }
}
