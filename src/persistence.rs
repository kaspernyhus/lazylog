use crate::app::App;
use crate::filter::{ActiveFilterMode, FilterHistoryEntry};
use crate::options::AppOption;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use tracing::info;

#[derive(Serialize, Deserialize)]
pub struct PersistedState {
    version: u8,
    log_file_paths: Vec<String>,
    viewport: ViewportState,
    search_history: Vec<String>,
    filter_history: Vec<FilterHistoryEntry>,
    filters: Vec<FilterPatternState>,
    marks: Vec<MarkState>,
    event_filters: Vec<EventFilterState>,
    #[serde(default)]
    custom_events: Vec<CustomEventState>,
    options: Vec<OptionState>,
}

#[derive(Serialize, Deserialize)]
struct OptionState {
    option: AppOption,
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
struct ViewportState {
    selected_line: usize,
    top_line: usize,
    horizontal_offset: usize,
    center_cursor_mode: bool,
}

#[derive(Serialize, Deserialize)]
pub struct FilterPatternState {
    pattern: String,
    mode: ActiveFilterMode,
    case_sensitive: bool,
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct MarkState {
    line_index: usize,
    name: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct EventFilterState {
    name: String,
    enabled: bool,
}

#[derive(Serialize, Deserialize)]
pub struct CustomEventState {
    pattern: String,
}

impl PersistedState {
    pub fn from_app(file_paths: &[&str], app: &App) -> Self {
        Self {
            version: 1,
            log_file_paths: file_paths.iter().map(|s| s.to_string()).collect(),
            viewport: ViewportState {
                selected_line: app.viewport.selected_line,
                top_line: app.viewport.top_line,
                horizontal_offset: app.viewport.horizontal_offset,
                center_cursor_mode: app.viewport.center_cursor_mode,
            },
            search_history: app.search.history.get_history().to_vec(),
            filter_history: app.filter.history.get_history().to_vec(),
            filters: app
                .filter
                .get_filter_patterns()
                .iter()
                .map(|fp| FilterPatternState {
                    pattern: fp.pattern.clone(),
                    mode: fp.mode,
                    case_sensitive: fp.case_sensitive,
                    enabled: fp.enabled,
                })
                .collect(),
            marks: app
                .marking
                .get_marks()
                .iter()
                .map(|m| MarkState {
                    line_index: m.line_index,
                    name: m.name.clone(),
                })
                .collect(),
            event_filters: app
                .event_tracker
                .get_event_stats()
                .iter()
                .map(|es| EventFilterState {
                    name: es.name.clone(),
                    enabled: es.enabled,
                })
                .collect(),
            custom_events: app
                .event_tracker
                .get_custom_event_patterns()
                .iter()
                .map(|pattern| CustomEventState {
                    pattern: pattern.to_string(),
                })
                .collect(),
            options: app
                .options
                .iter()
                .map(|opt| OptionState {
                    option: opt.option,
                    enabled: opt.enabled,
                })
                .collect(),
        }
    }
}

/// Saves the current application state to disk.
pub fn save_state(file_paths: &[&str], app: &App) {
    if !ensure_state_dir() {
        return;
    }

    let state_file_path = match get_state_file_path(file_paths) {
        Some(path) => path,
        None => return,
    };

    let state = PersistedState::from_app(file_paths, app);
    let json = match serde_json::to_string_pretty(&state) {
        Ok(j) => j,
        Err(_) => return,
    };

    let _ = fs::write(state_file_path, json);
}

/// Loads the application state from disk if it exists.
pub fn load_state(file_paths: &[&str]) -> Option<PersistedState> {
    let state_path = get_state_file_path(file_paths)?;

    if !state_path.exists() {
        return None;
    }

    match fs::read_to_string(&state_path) {
        Ok(json) => match serde_json::from_str::<PersistedState>(&json) {
            Ok(state) => {
                if paths_match(&state.log_file_paths, file_paths) {
                    Some(state)
                } else {
                    None
                }
            }
            Err(e) => {
                info!("Failed to deserialize state file {:?}: {}", state_path, e);
                // Corrupted state file, ignore it
                None
            }
        },
        Err(e) => {
            info!("Failed to read state file {:?}: {}", state_path, e);
            // Can't read file, ignore it
            None
        }
    }
}

/// Checks if two file path lists contain the same files, regardless of order.
fn paths_match(paths1: &[String], paths2: &[&str]) -> bool {
    if paths1.len() != paths2.len() {
        return false;
    }

    let mut sorted1: Vec<_> = paths1.iter().filter_map(|p| std::fs::canonicalize(p).ok()).collect();
    let mut sorted2: Vec<_> = paths2.iter().filter_map(|p| std::fs::canonicalize(p).ok()).collect();

    if sorted1.len() != sorted2.len() {
        return false;
    }

    sorted1.sort();
    sorted2.sort();

    sorted1 == sorted2
}

/// Calculates the state file path based on the log file paths.
fn get_state_file_path(file_paths: &[&str]) -> Option<PathBuf> {
    let mut hasher = DefaultHasher::new();

    let mut absolute_paths: Vec<PathBuf> = file_paths
        .iter()
        .filter_map(|path| std::fs::canonicalize(path).ok())
        .collect();
    absolute_paths.sort();

    for absolute_path in absolute_paths {
        let path_str = absolute_path.to_string_lossy();
        path_str.hash(&mut hasher);
    }

    let hash = hasher.finish();

    let home = dirs::home_dir()?;
    let state_dir = home.join(".lazylog");

    Some(state_dir.join(format!("{:x}.json", hash)))
}

/// Ensures the ~/.lazylog directory exists.
fn ensure_state_dir() -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let state_dir = home.join(".lazylog");

    if !state_dir.exists() {
        fs::create_dir_all(&state_dir).is_ok()
    } else {
        true
    }
}

/// Clears all persisted state files from the ~/.lazylog directory.
/// Returns Ok(message) on success or Err(error_message) on failure.
pub fn clear_all_state() -> Result<String, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not find home directory".to_string())?;
    let state_dir = home.join(".lazylog");

    if !state_dir.exists() {
        return Ok("No state directory found.".to_string());
    }

    let mut count = 0;
    for entry in fs::read_dir(&state_dir).map_err(|e| format!("Failed to read state directory: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            fs::remove_file(&path).map_err(|e| format!("Failed to remove file {:?}: {}", path, e))?;
            count += 1;
        }
    }

    if count > 0 {
        Ok(format!("Cleared state file(s) from {:?}", state_dir))
    } else {
        Ok(format!("No state files found in {:?}", state_dir))
    }
}

impl PersistedState {
    pub fn viewport_selected_line(&self) -> usize {
        self.viewport.selected_line
    }

    pub fn viewport_top_line(&self) -> usize {
        self.viewport.top_line
    }

    pub fn viewport_horizontal_offset(&self) -> usize {
        self.viewport.horizontal_offset
    }

    pub fn viewport_center_cursor_mode(&self) -> bool {
        self.viewport.center_cursor_mode
    }

    pub fn search_history(&self) -> &[String] {
        &self.search_history
    }

    pub fn filter_history(&self) -> &[FilterHistoryEntry] {
        &self.filter_history
    }

    pub fn filters(&self) -> &[FilterPatternState] {
        &self.filters
    }

    pub fn marks(&self) -> &[MarkState] {
        &self.marks
    }

    pub fn event_filters(&self) -> &[EventFilterState] {
        &self.event_filters
    }

    pub fn custom_events(&self) -> &[CustomEventState] {
        &self.custom_events
    }

    pub fn options(&self) -> Vec<(AppOption, bool)> {
        self.options
            .iter()
            .map(|opt_state| (opt_state.option, opt_state.enabled))
            .collect()
    }
}

impl FilterPatternState {
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn mode(&self) -> ActiveFilterMode {
        self.mode
    }

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

impl MarkState {
    pub fn line_index(&self) -> usize {
        self.line_index
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }
}

impl EventFilterState {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

impl CustomEventState {
    pub fn pattern(&self) -> &str {
        &self.pattern
    }
}
