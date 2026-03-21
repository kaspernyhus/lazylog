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
    /// Loads log lines from one or more files and parse timestamps if not disabled.
    pub fn load_files(&mut self, paths: &[&str], parse_timestamps: bool) -> color_eyre::Result<usize> {
        if paths.is_empty() {
            return Err(color_eyre::eyre::eyre!("No files provided"));
        }

        self.streaming = false;
        let multi_file = paths.len() > 1;
        let mut total_lines_skipped = 0;

        for (file_id, path) in paths.iter().enumerate() {
            let bytes = std::fs::read(path)?;
            let content = String::from_utf8_lossy(&bytes);
            let mut file_lines: Vec<LogLine> = content
                .lines()
                .enumerate()
                .map(|(index, line)| LogLine {
                    content: sanitize_line(line),
                    index,
                    timestamp: if parse_timestamps { parse_timestamp(line) } else { None },
                    log_file_id: Some(file_id),
                })
                .collect();

            if parse_timestamps {
                // Lines without a timestamp inherit from the line above.
                let mut last_timestamp: Option<DateTime<Utc>> = None;
                for line in file_lines.iter_mut() {
                    if line.timestamp.is_some() {
                        last_timestamp = line.timestamp;
                    } else {
                        line.timestamp = last_timestamp;
                    }
                }

                if multi_file {
                    total_lines_skipped += file_lines.iter().filter(|l| l.timestamp.is_none()).count();
                }
            }

            self.lines.append(&mut file_lines);
        }

        if multi_file {
            if parse_timestamps {
                self.lines.sort_by(|a, b| match (&a.timestamp, &b.timestamp) {
                    (Some(ts_a), Some(ts_b)) => ts_a.cmp(ts_b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.index.cmp(&b.index),
                });
            }

            for (new_index, line) in self.lines.iter_mut().enumerate() {
                line.index = new_index;
            }
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
