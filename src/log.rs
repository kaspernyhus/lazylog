use crate::timestamp::parse_timestamp;
use chrono::{DateTime, Utc};

fn needs_sanitization(line: &str) -> bool {
    line.bytes().any(|b| b == b'\t' || b == b'\r' || b < 0x20)
}

fn sanitize_line(line: &str) -> String {
    if !needs_sanitization(line) {
        return line.to_string();
    }
    do_sanitize(line)
}

fn sanitize_line_owned(line: String) -> String {
    if !needs_sanitization(&line) {
        return line;
    }
    do_sanitize(&line)
}

fn do_sanitize(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    for ch in line.chars() {
        match ch {
            '\t' => result.push_str("    "),
            '\r' => {}
            c if c.is_control() => {}
            c => result.push(c),
        }
    }
    result
}

/// A single log line with its content and original index.
#[derive(Debug, Clone)]
pub struct LogLine {
    /// The text content of the log line.
    pub content: String,
    /// The original index of the line in the source.
    pub index: usize,
    /// Parsed timestamp (if applicable).
    pub timestamp: Option<DateTime<Utc>>,
    /// File id
    pub log_file_id: Option<usize>,
}

/// Buffer for storing and managing log lines with filtering support.
#[derive(Debug, Default)]
pub struct LogBuffer {
    /// All log lines (unfiltered).
    lines: Vec<LogLine>,
    /// Whether the buffer is in streaming mode (reading from stdin).
    pub streaming: bool,
}

impl LogLine {
    /// Creates a new log line.
    pub fn new(content: &str, index: usize) -> Self {
        Self {
            content: sanitize_line(content),
            index,
            timestamp: None,
            log_file_id: None,
        }
    }

    /// Returns the log message content of the log line.
    pub fn content(&self) -> &str {
        &self.content
    }
}

impl LogBuffer {
    /// Loads log lines from one of more files.
    pub fn load_files(&mut self, paths: &[&str]) -> color_eyre::Result<usize> {
        if paths.is_empty() {
            return Err(color_eyre::eyre::eyre!("No files provided"));
        }

        self.streaming = false;

        // Single file: skip timestamp parsing and sorting
        if paths.len() == 1 {
            let bytes = std::fs::read(paths[0])?;
            let content = String::from_utf8_lossy(&bytes);
            self.lines = content
                .lines()
                .enumerate()
                .map(|(index, line)| LogLine::new(line, index))
                .collect();
            return Ok(0);
        }

        // Multi-file: parse timestamps and sort
        let mut total_lines_skipped = 0;

        for (file_id, path) in paths.iter().enumerate() {
            let bytes = std::fs::read(path)?;
            let content = String::from_utf8_lossy(&bytes);
            let mut file_lines: Vec<LogLine> = content
                .lines()
                .enumerate()
                .map(|(index, line)| {
                    let content = sanitize_line(line);
                    if let Some(timestamp) = parse_timestamp(line) {
                        LogLine {
                            content,
                            index,
                            timestamp: Some(timestamp),
                            log_file_id: Some(file_id),
                        }
                    } else {
                        total_lines_skipped += 1;
                        LogLine {
                            content,
                            index,
                            timestamp: None,
                            log_file_id: Some(file_id),
                        }
                    }
                })
                .collect();

            self.lines.append(&mut file_lines);
        }

        // Sort lines with timestamps first, then lines without timestamps
        self.lines.sort_by(|a, b| match (&a.timestamp, &b.timestamp) {
            (Some(ts_a), Some(ts_b)) => ts_a.cmp(ts_b),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => a.index.cmp(&b.index),
        });

        for (new_index, line) in self.lines.iter_mut().enumerate() {
            line.index = new_index;
        }

        Ok(total_lines_skipped)
    }

    /// Initializes the buffer for stdin streaming mode.
    pub fn init_stdin_mode(&mut self) {
        self.streaming = true;
        self.lines.clear();
    }

    /// Appends a new line to the buffer (streaming mode).
    ///
    /// Takes ownership of the content to avoid allocation when no sanitization is needed.
    /// Returns the index of the newly created LogLine.
    pub fn append_line(&mut self, content: String) -> usize {
        let index = self.lines.len();
        let log_line = LogLine {
            content: sanitize_line_owned(content),
            index,
            timestamp: None,
            log_file_id: None,
        };
        self.lines.push(log_line);
        index
    }

    /// Remove all lines and filters from the buffer. (Only in streaming mode.)
    pub fn clear_all(&mut self) {
        if self.streaming {
            self.lines.clear();
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

    /// Returns a reference to a log line by its original index.
    pub fn get_line(&self, line_index: usize) -> Option<&LogLine> {
        if line_index >= self.lines.len() {
            return None;
        }
        Some(&self.lines[line_index])
    }

    /// Returns the total count of log lines.
    pub fn get_total_lines_count(&self) -> usize {
        self.lines.len()
    }

    /// Returns an iterator over all log lines without active line filtering.
    pub fn iter(&self) -> impl Iterator<Item = &LogLine> {
        self.lines.iter()
    }

    /// Returns all log lines as a slice
    pub fn all_lines(&self) -> &[LogLine] {
        &self.lines
    }
}
