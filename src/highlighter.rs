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

impl PatternMatcher {
    /// Checks if the pattern matches the given text.
    pub fn matches(&self, text: &str) -> bool {
        match self {
            PatternMatcher::Plain(s) => text.contains(s),
            PatternMatcher::Regex(r) => r.is_match(text),
        }
    }

    /// Finds all occurrences of the pattern in the text.
    ///
    /// Returns a list of (start, end) byte positions for each match.
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        match self {
            PatternMatcher::Plain(s) => text
                .match_indices(s)
                .map(|(start, matched)| (start, start + matched.len()))
                .collect(),
            PatternMatcher::Regex(r) => r.find_iter(text).map(|m| (m.start(), m.end())).collect(),
        }
    }
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

/// Manages text highlighting and line coloring based on configured patterns.
#[derive(Debug)]
pub struct Highlighter {
    /// Patterns for text highlighting.
    patterns: Vec<HighlightPattern>,
    /// Patterns for line coloring.
    line_colors: Vec<LineColorPattern>,
}

impl Highlighter {
    /// Creates a new highlighter from configuration.
    pub fn new(config: &Config) -> Self {
        let patterns = Self::parse_highlight_patterns(config.highlight_patterns.clone());
        let line_colors = Self::parse_line_colors(config.line_colors.clone());
        Self {
            patterns,
            line_colors,
        }
    }

    /// Returns whether there are no highlight or line color patterns.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty() && self.line_colors.is_empty()
    }

    /// Converts highlight configurations into patterns with assigned colors.
    ///
    /// Uses explicit color if provided, otherwise generates a deterministic color from pattern hash.
    fn parse_highlight_patterns(configs: Vec<HighlightConfig>) -> Vec<HighlightPattern> {
        configs
            .into_iter()
            .filter_map(|config| {
                let matcher = if config.regex {
                    Regex::new(&config.pattern).ok().map(PatternMatcher::Regex)
                } else {
                    Some(PatternMatcher::Plain(config.pattern.clone()))
                };

                matcher.map(|m| {
                    let color = if let Some(color_str) = &config.color {
                        Self::parse_color(color_str)
                            .unwrap_or_else(|| Self::hash_to_color(&config.pattern))
                    } else {
                        Self::hash_to_color(&config.pattern)
                    };
                    HighlightPattern { matcher: m, color }
                })
            })
            .collect()
    }

    /// Converts line color configurations into patterns.
    ///
    /// Filters out configurations with invalid colors or regex patterns.
    fn parse_line_colors(configs: Vec<LineColorConfig>) -> Vec<LineColorPattern> {
        configs
            .into_iter()
            .filter_map(|config| {
                let color = Self::parse_color(&config.color)?;
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

    /// Generates a deterministic bright color from a pattern string using djb2 hash.
    fn hash_to_color(pattern: &str) -> Color {
        let mut hash: u32 = 5381;
        for byte in pattern.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }

        let bright_ranges = [82, 118, 154, 190, 196, 202, 208, 214, 220, 226];
        let range_start = bright_ranges[(hash as usize) % bright_ranges.len()];
        let color_index = range_start + (hash % 6) as u8;
        Color::Indexed(color_index)
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
            if line_color.matcher.matches(text) {
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
            for (start, end) in pattern.matcher.find_all(text) {
                ranges.push((start, end, pattern.color));
            }
        }
        ranges
    }
}
