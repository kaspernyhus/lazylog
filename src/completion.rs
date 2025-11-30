use std::collections::HashSet;

use crate::log::LogLine;

/// Manages tab completion.
#[derive(Debug)]
pub struct CompletionEngine {
    words: HashSet<String>,
}

impl CompletionEngine {
    pub fn new() -> Self {
        Self { words: HashSet::new() }
    }

    /// Extracts all unique words from the provided log lines.
    ///
    /// Words are split on whitespace.
    pub fn update<'a>(&mut self, lines: impl Iterator<Item = &'a LogLine>) {
        let log_line_content = lines.map(|log_line| log_line.content());

        for line in log_line_content {
            for word in line.split_whitespace() {
                self.words.insert(word.to_string());
            }
        }
    }

    /// Appends words from a single log line.
    pub fn append_line(&mut self, log_line: &LogLine) {
        for word in log_line.content().split_whitespace() {
            self.words.insert(word.to_string());
        }
    }

    /// Finds the longest common prefix completion for the given prefix.
    pub fn find_completion(&self, prefix: &str) -> Option<String> {
        if prefix.is_empty() {
            return None;
        }

        let mut matches: Vec<&String> = self.words.iter().filter(|word| word.starts_with(prefix)).collect();

        matches.sort();

        match matches.len() {
            0 => None,
            1 => {
                let word = matches[0];
                (word.len() > prefix.len()).then(|| word[prefix.len()..].to_string())
            }
            _ => {
                let common = self.find_common_prefix(&matches);
                Some(common[prefix.len()..].to_string())
            }
        }
    }

    fn find_common_prefix(&self, words: &[&String]) -> String {
        if words.is_empty() {
            return String::new();
        }

        let first = words[0];
        let mut prefix_len = first.chars().count();

        for word in words.iter().skip(1) {
            prefix_len = first
                .chars()
                .zip(word.chars())
                .take(prefix_len)
                .take_while(|(c1, c2)| c1 == c2)
                .count();

            if prefix_len == 0 {
                return String::new();
            }
        }

        first.chars().take(prefix_len).collect()
    }
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_incremental_updates() {
        let mut engine = CompletionEngine::new();

        // Initial batch
        let line1 = LogLine::new("Processing request".to_string(), 0);
        let line2 = LogLine::new("Error occurred".to_string(), 1);
        engine.update([&line1, &line2].into_iter());

        assert_eq!(engine.find_completion("Pro"), Some("cessing".to_string()));

        // Add more lines incrementally (streaming mode)
        let line3 = LogLine::new("Program started".to_string(), 2);
        let line4 = LogLine::new("Profile loaded".to_string(), 3);
        engine.update([&line3, &line4].into_iter());

        // Should now have multiple matches starting with "Pro"
        let completion = engine.find_completion("Pro");
        assert!(completion.is_some());
        // Common prefix of "Processing", "Program", "Profile" is "Pro"
        assert_eq!(completion.unwrap(), "");
    }

    #[test]
    fn test_case_sensitive_completion() {
        let mut engine = CompletionEngine::new();
        let line1 = LogLine::new("ERROR message".to_string(), 0);
        let line2 = LogLine::new("error occurred".to_string(), 1);
        engine.update([&line1, &line2].into_iter());

        // Case sensitive - should only match "ERROR"
        assert_eq!(engine.find_completion("ERR"), Some("OR".to_string()));
        // No match for lowercase
        assert_eq!(engine.find_completion("err"), Some("or".to_string()));
    }
}
