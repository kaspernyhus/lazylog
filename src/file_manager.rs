use std::{collections::HashSet, sync::Arc};

use crate::{log::LogLine, resolver::VisibilityRule};

/// Represents a single file in a multi-file session.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// The file path.
    pub path: String,
    /// The file ID.
    pub file_id: usize,
    /// Whether the view for this file is enabled.
    pub enabled: bool,
}

impl FileEntry {
    pub fn new(path: String, file_id: usize) -> Self {
        Self {
            path,
            file_id,
            enabled: true,
        }
    }

    pub fn get_filename(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }
}

/// Manages the list of opened files in multi-file sessions.
#[derive(Debug)]
pub struct FileManager {
    /// List of file entries.
    files: Vec<FileEntry>,
}

impl FileManager {
    pub fn new(file_paths: &[String]) -> Self {
        Self {
            files: file_paths
                .iter()
                .enumerate()
                .map(|(id, path)| FileEntry::new(path.clone(), id))
                .collect(),
        }
    }

    /// Returns the number of files.
    pub fn count(&self) -> usize {
        self.files.len()
    }

    /// Returns true if there are multiple files loaded.
    pub fn is_multi_file(&self) -> bool {
        self.files.len() > 1
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Returns all file paths.
    pub fn paths(&self) -> Vec<&str> {
        self.files.iter().map(|f| f.path.as_str()).collect()
    }

    /// Returns the first file path (typically used in single-file mode).
    pub fn first_path(&self) -> Option<&str> {
        self.files.first().map(|f| f.path.as_str())
    }

    /// Returns an iterator over the file entries.
    pub fn iter(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter()
    }

    /// Gets a file entry by index.
    pub fn get(&self, index: usize) -> Option<&FileEntry> {
        self.files.get(index)
    }

    /// Toggles the enabled state of a file at the given index.
    pub fn toggle_enabled(&mut self, index: usize) {
        if let Some(file) = self.files.get_mut(index) {
            file.enabled = !file.enabled;
        }
    }

    /// Returns a vec of enabled file IDs (only relevant for multi-file filtering).
    pub fn enabled_file_ids(&self) -> HashSet<usize> {
        self.files.iter().filter(|f| f.enabled).map(|f| f.file_id).collect()
    }
}

/// Rule that filters lines by file ID
pub struct FileFilterRule {
    enabled_file_ids: Arc<HashSet<usize>>,
}

impl FileFilterRule {
    pub fn new(enabled_file_ids: Arc<HashSet<usize>>) -> Self {
        Self { enabled_file_ids }
    }
}

impl VisibilityRule for FileFilterRule {
    fn is_visible(&self, line: &LogLine) -> bool {
        if let Some(file_id) = line.log_file_id {
            self.enabled_file_ids.contains(&file_id)
        } else {
            // Lines without file ID are always shown
            true
        }
    }
}
