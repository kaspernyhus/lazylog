/// History manager for storing and navigating through previous entries.
#[derive(Debug, Default, Clone)]
pub struct History<T> {
    /// List of history entries.
    history: Vec<T>,
    /// Current position when navigating history (None when not navigating).
    index: Option<usize>,
}

impl<T: Clone + PartialEq> History<T> {
    /// Creates a new empty history.
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
            index: None,
        }
    }

    /// Adds an entry to the history if the entry doesn't already exist.
    pub fn add(&mut self, entry: T) {
        if !self.history.contains(&entry) {
            self.history.push(entry);
        }
        self.index = None;
    }

    /// Navigates to the previous (older) entry in history.
    pub fn previous_record(&mut self) -> Option<&T> {
        if self.history.is_empty() {
            return None;
        }

        match self.index {
            None => {
                // start at the end (most recent)
                self.index = Some(self.history.len() - 1);
                Some(&self.history[self.history.len() - 1])
            }
            Some(0) => {
                // Already at oldest entry
                None
            }
            Some(i) => {
                self.index = Some(i - 1);
                Some(&self.history[i - 1])
            }
        }
    }

    /// Navigates to the next (newer) entry in history.
    pub fn next_record(&mut self) -> Option<&T> {
        match self.index {
            None => None,
            Some(i) if i + 1 >= self.history.len() => {
                // At newest entry - exit history navigation mode
                self.index = None;
                None
            }
            Some(i) => {
                self.index = Some(i + 1);
                Some(&self.history[i + 1])
            }
        }
    }

    /// Resets the navigation index to None (exits history navigation mode).
    pub fn reset(&mut self) {
        self.index = None;
    }

    /// Returns a slice of all history entries.
    pub fn get_history(&self) -> &[T] {
        &self.history
    }

    /// Restores history from a vector of entries.
    pub fn restore(&mut self, history: Vec<T>) {
        self.history = history;
        self.index = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entry() {
        let mut history = History::new();
        history.add("test1".to_string());
        history.add("test2".to_string());
        assert_eq!(history.get_history(), &["test1", "test2"]);
    }

    #[test]
    fn test_add_duplicate_ignored() {
        let mut history = History::new();
        history.add("test1".to_string());
        history.add("test1".to_string());
        assert_eq!(history.get_history(), &["test1"]);
    }

    #[test]
    fn test_previous_from_empty() {
        let mut history: History<String> = History::new();
        assert_eq!(history.previous_record(), None);
    }

    #[test]
    fn test_previous_navigation() {
        let mut history = History::new();
        history.add("test1".to_string());
        history.add("test2".to_string());
        history.add("test3".to_string());

        // First previous should return most recent (test3)
        assert_eq!(history.previous_record(), Some(&"test3".to_string()));
        // Second previous should return test2
        assert_eq!(history.previous_record(), Some(&"test2".to_string()));
        // Third previous should return test1
        assert_eq!(history.previous_record(), Some(&"test1".to_string()));
        // Fourth previous should return None (at oldest)
        assert_eq!(history.previous_record(), None);
    }

    #[test]
    fn test_next_navigation() {
        let mut history = History::new();
        history.add("test1".to_string());
        history.add("test2".to_string());

        // Navigate back first
        history.previous_record();
        history.previous_record();

        // Now navigate forward
        assert_eq!(history.next_record(), Some(&"test2".to_string()));
        // At newest, next should return None and reset index
        assert_eq!(history.next_record(), None);
    }

    #[test]
    fn test_reset() {
        let mut history = History::new();
        history.add("test1".to_string());
        history.previous_record();
        history.reset();

        // After reset, previous should start from the end again
        assert_eq!(history.previous_record(), Some(&"test1".to_string()));
    }
}
