use crate::list_view_state::ListViewState;

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
    /// View state for the file list.
    view: ListViewState,
}

impl FileManager {
    pub fn new(file_paths: &[String]) -> Self {
        let mut mgr = Self {
            files: Vec::new(),
            view: ListViewState::new(),
        };

        mgr.files = file_paths
            .iter()
            .enumerate()
            .map(|(id, path)| FileEntry::new(path.clone(), id))
            .collect();

        mgr.view.set_item_count(mgr.count());

        mgr
    }

    /// Returns the number of files.
    pub fn count(&self) -> usize {
        self.files.len()
    }

    /// Returns true if there are multiple files loaded.
    pub fn is_multi_file(&self) -> bool {
        self.files.len() > 1
    }

    pub fn get_paths(&self) -> Vec<String> {
        self.files.iter().map(|f| f.path.clone()).collect()
    }

    pub fn get_path(&self) -> &str {
        if !self.is_multi_file() { &self.files[0].path } else { "" }
    }

    /// Returns an iterator over the file entries.
    pub fn iter(&self) -> impl Iterator<Item = &FileEntry> {
        self.files.iter()
    }

    /// Gets the currently selected index.
    pub fn selected_index(&self) -> usize {
        self.view.selected_index()
    }

    /// Gets the viewport offset for scrolling.
    pub fn viewport_offset(&self) -> usize {
        self.view.viewport_offset()
    }

    /// Sets the viewport height (called from UI rendering).
    pub fn set_viewport_height(&self, height: usize) {
        self.view.set_viewport_height(height);
    }

    /// Moves selection up.
    pub fn move_selection_up(&mut self) {
        self.view.move_up();
    }

    /// Moves selection down.
    pub fn move_selection_down(&mut self) {
        self.view.move_down();
    }

    /// Moves selection up by half a page.
    pub fn page_up(&mut self) {
        self.view.page_up();
    }

    /// Moves selection down by half a page.
    pub fn page_down(&mut self) {
        self.view.page_down();
    }

    /// Toggles the enabled state of the currently selected file.
    pub fn toggle_selected(&mut self) {
        let selected = self.view.selected_index();
        if selected < self.files.len() {
            self.files[selected].enabled = !self.files[selected].enabled;
        }
    }

    /// Gets the selected file entry.
    pub fn get_selected(&self) -> Option<&FileEntry> {
        self.files.get(self.view.selected_index())
    }

    /// Returns a list of enabled file paths.
    pub fn get_enabled_ids(&self) -> Option<Vec<usize>> {
        if self.is_multi_file() {
            let enabled_ids = self
                .files
                .iter()
                .filter_map(|f| if f.enabled { Some(f.file_id) } else { None })
                .collect();
            Some(enabled_ids)
        } else {
            None
        }
    }
}
