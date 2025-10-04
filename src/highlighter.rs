use crate::config::{Config, HighlightConfig, LineColorConfig};
use ratatui::style::Color;
use regex::Regex;

#[derive(Debug, Clone)]
pub enum PatternMatcher {
    Plain(String),
    Regex(Regex),
}

impl PatternMatcher {
    pub fn matches(&self, text: &str) -> bool {
        match self {
            PatternMatcher::Plain(s) => text.contains(s),
            PatternMatcher::Regex(r) => r.is_match(text),
        }
    }

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

#[derive(Debug, Clone)]
pub struct HighlightPattern {
    pub matcher: PatternMatcher,
    pub color: Color,
}

#[derive(Debug, Clone)]
pub struct LineColorPattern {
    pub matcher: PatternMatcher,
    pub color: Color,
}

#[derive(Debug)]
pub struct Highlighter {
    patterns: Vec<HighlightPattern>,
    line_colors: Vec<LineColorPattern>,
}

impl Highlighter {
    pub fn new(config: &Config) -> Self {
        let patterns = Self::assign_colors(config.highlight_patterns.clone());
        let line_colors = Self::parse_line_colors(config.line_colors.clone());
        Self {
            patterns,
            line_colors,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty() && self.line_colors.is_empty()
    }

    fn assign_colors(configs: Vec<HighlightConfig>) -> Vec<HighlightPattern> {
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

    fn hash_to_color(pattern: &str) -> Color {
        let mut hash: u32 = 5381;
        for byte in pattern.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }

        // Map to brighter color indices, avoiding dark colors
        // Using ranges: 82-87 (bright greens/cyans), 118-123 (bright greens),
        // 154-159 (greens), 190-195 (yellows), 196-201 (oranges/reds),
        // 202-207 (reds/pinks), 208-213 (pinks), 214-219 (pinks/purples),
        // 220-225 (purples), 226-231 (yellows)
        let bright_ranges = [82, 118, 154, 190, 196, 202, 208, 214, 220, 226];
        let range_start = bright_ranges[(hash as usize) % bright_ranges.len()];
        let color_index = range_start + (hash % 6) as u8;
        Color::Indexed(color_index)
    }

    pub fn get_patterns(&self) -> &[HighlightPattern] {
        &self.patterns
    }

    pub fn get_line_colors(&self) -> &[LineColorPattern] {
        &self.line_colors
    }

    pub fn get_line_color(&self, text: &str) -> Option<Color> {
        for line_color in &self.line_colors {
            if line_color.matcher.matches(text) {
                return Some(line_color.color);
            }
        }
        None
    }

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
