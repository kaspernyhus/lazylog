use crate::{filter::Filter, processing};
use chrono::{DateTime, Utc};
use rayon::prelude::*;

/// A single log line with its content and original index.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// The text content of the log line.
    pub content: String,
    /// The original index of the line in the source.
    pub index: usize,
    /// The source file ID (for merged log views).
    pub source_file_id: Option<usize>,
    /// The parsed timestamp (for merged log views).
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
    /// Source file paths (for merged views).
    pub source_files: Vec<String>,
    /// Visibility of each source file (for merged views).
    pub source_file_visibility: Vec<bool>,
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
            source_file_id: None,
            timestamp: None,
        }
    }

    /// Creates a new log line with source file and timestamp information.
    pub fn new_with_metadata(
        content: String,
        index: usize,
        source_file_id: Option<usize>,
        timestamp: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            content,
            index,
            source_file_id,
            timestamp,
        }
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

    /// Applies the given filter to all lines in the buffer.
    ///
    /// Rebuilds the active_lines list based on the filter criteria.
    pub fn apply_filters(&mut self, filter: &Filter, marked_indices: &[usize]) {
        let filter_patterns = filter.get_filter_patterns();
        let show_marked_only = filter.is_show_marked_only();
        let source_file_visibility = self.source_file_visibility.clone();

        if filter_patterns.is_empty() && !show_marked_only && !self.is_merged_view() {
            self.clear_filters();
        } else {
            self.active_lines = self
                .lines
                .par_iter()
                .enumerate()
                .filter_map(|(index, log_line)| {
                    let passes_text_filter = if filter_patterns.is_empty() {
                        true
                    } else {
                        processing::apply_filters(&log_line.content, filter_patterns)
                    };

                    let passes_marked_filter = if show_marked_only {
                        marked_indices.binary_search(&index).is_ok()
                    } else {
                        true
                    };

                    let passes_source_visibility = if let Some(source_id) = log_line.source_file_id {
                        source_file_visibility.get(source_id).copied().unwrap_or(true)
                    } else {
                        true
                    };

                    if passes_text_filter && passes_marked_filter && passes_source_visibility {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect();
        }
    }

    /// Clears all filters (but still respects source file visibility).
    pub fn clear_filters(&mut self) {
        if self.is_merged_view() {
            // In merged view, still need to filter by source visibility
            self.active_lines = self
                .lines
                .iter()
                .enumerate()
                .filter_map(|(index, log_line)| {
                    if let Some(source_id) = log_line.source_file_id {
                        if self.source_file_visibility.get(source_id).copied().unwrap_or(true) {
                            Some(index)
                        } else {
                            None
                        }
                    } else {
                        Some(index)
                    }
                })
                .collect();
        } else {
            self.active_lines = (0..self.lines.len()).collect();
        }
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

    /// Loads and merges multiple log files, sorting by timestamp.
    ///
    /// Lines without parseable timestamps are skipped.
    /// Returns an error if no files could be loaded or if no timestamps could be parsed.
    /// Returns (lines_merged, lines_skipped) on success.
    pub fn load_and_merge_files(&mut self, paths: &[String]) -> color_eyre::Result<(usize, usize)> {
        use crate::timestamp::parse_timestamp;

        if paths.is_empty() {
            return Err(color_eyre::eyre::eyre!("No files provided"));
        }

        self.streaming = false;
        self.lines.clear();
        self.source_files.clear();
        self.source_file_visibility.clear();

        let mut all_lines_with_metadata = Vec::new();
        let mut lines_skipped = 0;
        let mut total_lines_read = 0;

        // Load each file
        for (file_id, path) in paths.iter().enumerate() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| color_eyre::eyre::eyre!("Failed to read file '{}': {}", path, e))?;

            self.source_files.push(path.clone());
            self.source_file_visibility.push(true);

            // Parse each line
            for (line_index, line_content) in content.lines().enumerate() {
                total_lines_read += 1;

                if let Some(timestamp) = parse_timestamp(line_content) {
                    all_lines_with_metadata.push((
                        line_content.to_string(),
                        line_index,
                        file_id,
                        timestamp,
                    ));
                } else {
                    lines_skipped += 1;
                }
            }
        }

        // Check if we parsed any timestamps
        if all_lines_with_metadata.is_empty() {
            return Err(color_eyre::eyre::eyre!(
                "Failed to parse timestamps from any log lines. {} lines read, {} lines skipped.",
                total_lines_read,
                lines_skipped
            ));
        }

        // Sort by timestamp
        all_lines_with_metadata.sort_by_key(|(_, _, _, timestamp)| *timestamp);

        // Create LogLines with proper global indexing
        for (_global_index, (content, original_index, file_id, timestamp)) in
            all_lines_with_metadata.into_iter().enumerate()
        {
            self.lines.push(LogLine::new_with_metadata(
                content,
                original_index,
                Some(file_id),
                Some(timestamp),
            ));
        }

        // Set merged file_path indicator
        if self.source_files.len() == 1 {
            self.file_path = Some(self.source_files[0].clone());
        } else {
            self.file_path = Some(format!("<merged: {} files>", self.source_files.len()));
        }

        self.clear_filters();

        // Log statistics
        tracing::info!(
            "Merged {} files: {} lines total, {} lines skipped (no timestamp)",
            self.source_files.len(),
            self.lines.len(),
            lines_skipped
        );

        Ok((self.lines.len(), lines_skipped))
    }

    /// Checks if this buffer is displaying merged files.
    pub fn is_merged_view(&self) -> bool {
        self.source_files.len() > 1
    }

    /// Toggles visibility of a source file by ID.
    pub fn toggle_source_file_visibility(&mut self, file_id: usize) {
        if file_id < self.source_file_visibility.len() {
            self.source_file_visibility[file_id] = !self.source_file_visibility[file_id];
        }
    }

    /// Gets the count of visible source files.
    pub fn get_visible_source_count(&self) -> usize {
        self.source_file_visibility.iter().filter(|&&v| v).count()
    }
}
