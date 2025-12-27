use std::collections::{HashMap, hash_map::Entry};
use std::sync::Arc;

/// Manages expansion expansion for filtered log lines.
/// Tracks which LOG LINES (not viewport positions) have been expanded.
#[derive(Debug)]
pub struct Expansions {
    /// Maps log line index -> Vec of log line indices to show below it
    expanded: Arc<HashMap<usize, Vec<usize>>>,
}

impl Default for Expansions {
    fn default() -> Self {
        Self {
            expanded: Arc::new(HashMap::new()),
        }
    }
}

impl Expansions {
    /// Creates a new empty expansion expansion.
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggles expansion for the given log line index with the given hidden line indices.
    pub fn toggle(&mut self, log_idx: usize, hidden_indices: Vec<usize>) {
        let expanded = Arc::make_mut(&mut self.expanded);
        if let Entry::Vacant(e) = expanded.entry(log_idx) {
            if !hidden_indices.is_empty() {
                e.insert(hidden_indices);
            }
        } else {
            expanded.remove(&log_idx);
        }
    }

    /// Returns whether the given log line is expanded.
    pub fn is_expanded(&self, log_idx: usize) -> bool {
        self.expanded.contains_key(&log_idx)
    }

    /// Returns the number of expanded lines below the given log line.
    pub fn get_expanded_count(&self, log_idx: usize) -> usize {
        self.expanded.get(&log_idx).map(|v| v.len()).unwrap_or(0)
    }

    /// Returns the expanded line indices for the given log line.
    pub fn get_expanded_indices(&self, log_idx: usize) -> Option<&Vec<usize>> {
        self.expanded.get(&log_idx)
    }

    /// Returns all expanded mappings.
    pub fn get_all_expanded(&self) -> Arc<HashMap<usize, Vec<usize>>> {
        Arc::clone(&self.expanded)
    }

    /// Finds the parent log index for a given expanded line if there is any.
    pub fn find_parent(&self, log_idx: usize) -> Option<usize> {
        self.expanded
            .iter()
            .find(|(_, children)| children.contains(&log_idx))
            .map(|(parent, _)| *parent)
    }

    /// Clears all expansions.
    pub fn clear(&mut self) {
        self.expanded = Arc::new(HashMap::new());
    }

    /// Returns the total number of expanded lines across all expansions.
    pub fn total_expanded_lines(&self) -> usize {
        self.expanded.values().map(|v| v.len()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_expansion() {
        let mut expansion = Expansions::new();

        // Expand log line 0 to show log lines 10-14
        assert!(!expansion.is_expanded(0));
        expansion.toggle(0, vec![10, 11, 12, 13, 14]);
        assert!(expansion.is_expanded(0));
        assert_eq!(expansion.get_expanded_count(0), 5);

        // Collapse it
        expansion.toggle(0, vec![10, 11, 12, 13, 14]);
        assert!(!expansion.is_expanded(0));
    }

    #[test]
    fn test_clear() {
        let mut expansion = Expansions::new();
        // Expand log lines 0 and 1
        expansion.toggle(0, vec![10, 11, 12, 13, 14]);
        expansion.toggle(1, vec![20, 21, 22]);

        assert_eq!(expansion.total_expanded_lines(), 8);

        expansion.clear();
        assert_eq!(expansion.total_expanded_lines(), 0);
        assert!(!expansion.is_expanded(0));
        assert!(!expansion.is_expanded(1));
    }
}
