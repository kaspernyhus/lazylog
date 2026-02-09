use crate::utils::contains_ignore_case;
use regex::Regex;

/// Type of pattern matching to use.
#[derive(Debug)]
pub enum PatternMatchType {
    Plain(bool),
    Regex,
}

/// Plain text pattern matcher with optional case sensitivity.
#[derive(Debug, Clone)]
pub struct PlainMatch {
    /// The plain text pattern to match
    pub pattern: String,
    /// Whether matching should be case-sensitive
    pub case_sensitive: bool,
}

impl PlainMatch {
    /// Returns true if there is a match for the plain match pattern anywhere in the haystack given.
    pub fn is_match(&self, haystack: &str) -> bool {
        if self.case_sensitive {
            haystack.contains(&self.pattern)
        } else {
            contains_ignore_case(haystack, &self.pattern)
        }
    }

    /// Find all occurrences of a substring in the haystack
    pub fn find(&self, haystack: &str) -> Vec<(usize, usize)> {
        if self.case_sensitive {
            haystack
                .match_indices(&self.pattern)
                .map(|(start, matched)| (start, start + matched.len()))
                .collect()
        } else {
            self.find_all_ignore_case(haystack)
        }
    }

    /// Finds all case-insensitive occurrences of a substring in text.
    fn find_all_ignore_case(&self, text: &str) -> Vec<(usize, usize)> {
        let pattern_bytes = self.pattern.as_bytes();
        let text_bytes = text.as_bytes();
        let pattern_len = pattern_bytes.len();

        if pattern_len == 0 || pattern_len > text_bytes.len() {
            return Vec::new();
        }

        text_bytes
            .windows(pattern_len)
            .enumerate()
            .filter(|(_, window)| window.eq_ignore_ascii_case(pattern_bytes))
            .map(|(idx, _)| (idx, idx + pattern_len))
            .collect()
    }
}

/// Pattern matching strategy for text highlighting.
#[derive(Debug, Clone)]
pub enum PatternMatcher {
    /// Plain string matching with runtime case sensitivity
    Plain(PlainMatch),
    /// Regular expression matching (case sensitivity determined at compile time)
    Regex(Regex),
}

impl PatternMatcher {
    /// Checks if the pattern matches the given text.
    pub fn matches(&self, text: &str) -> bool {
        match self {
            PatternMatcher::Plain(s) => s.is_match(text),
            PatternMatcher::Regex(r) => r.is_match(text),
        }
    }

    /// Finds all occurrences of the pattern in the text.
    ///
    /// Returns a list of (start, end) byte positions for each match.
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        match self {
            PatternMatcher::Plain(plain_match) => plain_match.find(text),
            PatternMatcher::Regex(r) => r.find_iter(text).map(|m| (m.start(), m.end())).collect(),
        }
    }
}
