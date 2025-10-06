use crate::config::{Config, HighlightConfig, LineColorConfig};
use ratatui::style::Color;
use regex::Regex;

/// Pattern matching strategy for text highlighting.
#[derive(Debug, Clone)]
pub enum PatternMatcher {
    /// Plain string matching.
    Plain(String),
    /// Regular expression matching.
    Regex(Regex),
}

/// Pattern with associated color for text highlighting.
#[derive(Debug, Clone)]
pub struct HighlightPattern {
    /// Matcher to identify text spans to highlight.
    pub matcher: PatternMatcher,
    /// Color to apply to matched text.
    pub color: Color,
}

/// Pattern with associated color for line coloring.
#[derive(Debug, Clone)]
pub struct LineColorPattern {
    /// Matcher to identify lines to color.
    pub matcher: PatternMatcher,
    /// Color to apply to matched lines.
    pub color: Color,
}

/// Temporary highlight.
#[derive(Debug, Clone)]
pub struct TemporaryHighlight {
    /// Pattern to match.
    pub pattern: String,
    /// Foreground color.
    pub fg_color: Color,
    /// Background color.
    pub bg_color: Option<Color>,
    /// Whether matching is case-sensitive.
    pub case_sensitive: bool,
}

/// Manages text highlighting and line coloring based on configured patterns.
#[derive(Debug)]
pub struct Highlighter {
    /// Patterns for text highlighting.
    patterns: Vec<HighlightPattern>,
    /// Patterns for full line coloring.
    line_colors: Vec<LineColorPattern>,
    /// Temporary highlights.
    temporary_highlights: Vec<TemporaryHighlight>,
}

impl PatternMatcher {
    /// Checks if the pattern matches the given text.
    pub fn matches(&self, text: &str, case_sensitive: bool) -> bool {
        match self {
            PatternMatcher::Plain(s) => {
                if case_sensitive {
                    text.contains(s)
                } else {
                    contains_ignore_case(text, s)
                }
            }
            PatternMatcher::Regex(r) => r.is_match(text),
        }
    }

    /// Finds all occurrences of the pattern in the text.
    ///
    /// Returns a list of (start, end) byte positions for each match.
    pub fn find_all(&self, text: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
        match self {
            PatternMatcher::Plain(s) => {
                if case_sensitive {
                    text.match_indices(s)
                        .map(|(start, matched)| (start, start + matched.len()))
                        .collect()
                } else {
                    find_all_ignore_case(text, s)
                }
            }
            PatternMatcher::Regex(r) => r.find_iter(text).map(|m| (m.start(), m.end())).collect(),
        }
    }
}

impl Highlighter {
    /// Creates a new highlighter from configuration.
    pub fn new(config: &Config) -> Self {
        let patterns = Self::parse_highlight_patterns(config.highlight_patterns.clone());
        let line_colors = Self::parse_line_colors(config.line_colors.clone());
        Self {
            patterns,
            line_colors,
            temporary_highlights: Vec::new(),
        }
    }

    /// Converts highlight config into patterns.
    fn parse_highlight_patterns(highlight_patterns: Vec<HighlightConfig>) -> Vec<HighlightPattern> {
        highlight_patterns
            .into_iter()
            .filter_map(|config| {
                let matcher = if config.regex {
                    Regex::new(&config.pattern).ok().map(PatternMatcher::Regex)
                } else {
                    Some(PatternMatcher::Plain(config.pattern.clone()))
                };

                matcher.map(|m| {
                    let color = if let Some(color_str) = &config.color {
                        parse_color(color_str).unwrap_or_else(|| hash_to_color(&config.pattern))
                    } else {
                        hash_to_color(&config.pattern)
                    };
                    HighlightPattern { matcher: m, color }
                })
            })
            .collect()
    }

    /// Converts line color config into patterns.
    fn parse_line_colors(configs: Vec<LineColorConfig>) -> Vec<LineColorPattern> {
        configs
            .into_iter()
            .filter_map(|config| {
                let color = parse_color(&config.color)?;
                let matcher = if config.regex {
                    Regex::new(&config.pattern)
                        .ok()
                        .map(PatternMatcher::Regex)?
                } else {
                    PatternMatcher::Plain(config.pattern)
                };

                Some(LineColorPattern { matcher, color })
            })
            .collect()
    }

    /// Returns whether there are no highlight or line color patterns.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty() && self.line_colors.is_empty()
    }

    /// Returns all highlight patterns.
    pub fn get_patterns(&self) -> &[HighlightPattern] {
        &self.patterns
    }

    /// Returns all line color patterns.
    pub fn get_line_colors(&self) -> &[LineColorPattern] {
        &self.line_colors
    }

    /// Returns the color for a line if it matches any line color pattern.
    ///
    /// Returns the first matching pattern's color, or `None` if no pattern matches.
    pub fn get_line_color(&self, text: &str) -> Option<Color> {
        for line_color in &self.line_colors {
            if line_color.matcher.matches(text, true) {
                return Some(line_color.color);
            }
        }
        None
    }

    /// Finds all highlight ranges in the given text.
    ///
    /// Returns a list of (start, end, color) tuples for each match.
    pub fn get_highlight_ranges(&self, text: &str) -> Vec<(usize, usize, Color)> {
        let mut ranges = Vec::new();
        for pattern in &self.patterns {
            for (start, end) in pattern.matcher.find_all(text, true) {
                ranges.push((start, end, pattern.color));
            }
        }
        ranges
    }

    /// Adds a temporary highlight.
    pub fn add_temporary_highlight(
        &mut self,
        pattern: String,
        fg_color: Color,
        bg_color: Option<Color>,
        case_sensitive: bool,
    ) {
        self.temporary_highlights.push(TemporaryHighlight {
            pattern,
            fg_color,
            bg_color,
            case_sensitive,
        });
    }

    /// Clears all temporary highlights.
    pub fn clear_temporary_highlights(&mut self) {
        self.temporary_highlights.clear();
    }

    /// Finds all highlight ranges.
    ///
    /// Returns a list of (start, end, fg_color, bg_color) tuples.
    pub fn get_all_highlight_ranges(
        &self,
        text: &str,
    ) -> Vec<(usize, usize, Color, Option<Color>)> {
        let mut ranges = Vec::new();

        // Add config highlights (no background color)
        for pattern in &self.patterns {
            for (start, end) in pattern.matcher.find_all(text, true) {
                ranges.push((start, end, pattern.color, None));
            }
        }

        // Add temporary highlights (may have background color)
        for highlight in &self.temporary_highlights {
            if highlight.pattern.is_empty() {
                continue;
            }
            let matcher = PatternMatcher::Plain(highlight.pattern.clone());
            for (start, end) in matcher.find_all(text, highlight.case_sensitive) {
                ranges.push((start, end, highlight.fg_color, highlight.bg_color));
            }
        }

        ranges
    }
}

/// Finds all case-insensitive occurrences of a substring in text.
///
/// Returns a list of (start, end) byte positions for each match.
fn find_all_ignore_case(text: &str, pattern: &str) -> Vec<(usize, usize)> {
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();

    text_lower
        .match_indices(&pattern_lower)
        .map(|(start, matched)| (start, start + matched.len()))
        .collect()
}

/// Returns true if the given needle matches a sub-slice of haystack string slice ignoring the case.
///
/// Returns false if it does not.
fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Parses a color name string into a Color.
///
/// Returns `None` for unrecognized color names.
fn parse_color(color_str: &str) -> Option<Color> {
    match color_str.to_lowercase().as_str() {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        "gray" => Some(Color::Gray),
        "darkgray" => Some(Color::DarkGray),
        "lightred" => Some(Color::LightRed),
        "lightgreen" => Some(Color::LightGreen),
        "lightyellow" => Some(Color::LightYellow),
        "lightblue" => Some(Color::LightBlue),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightcyan" => Some(Color::LightCyan),
        _ => None,
    }
}

/// Generates a deterministic color from a pattern.
fn hash_to_color(pattern: &str) -> Color {
    let mut hash: u32 = 5381;
    for byte in pattern.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
    }
    // Use bright colors from the 256-color palette (82-231)
    let bright_ranges = [82, 118, 154, 190, 196, 202, 208, 214, 220, 226];
    let range_start = bright_ranges[(hash as usize) % bright_ranges.len()];
    let color_index = range_start + (hash % 6) as u8;
    Color::Indexed(color_index)
}
