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

/// Style configuration for text rendering.
#[derive(Debug, Clone)]
pub struct PatternStyle {
    /// Foreground color.
    pub fg_color: Option<Color>,
    /// Background color.
    pub bg_color: Option<Color>,
    /// Bold text.
    pub bold: bool,
}

impl PatternStyle {
    /// Creates a default style for events.
    pub fn default_event_style() -> Self {
        Self {
            fg_color: Some(Color::Rgb(255, 255, 255)),
            bg_color: Some(Color::Blue),
            bold: false,
        }
    }
}

/// Pattern with associated color for text highlighting.
#[derive(Debug, Clone)]
pub struct HighlightPattern {
    /// Matcher to identify text spans to highlight.
    pub matcher: PatternMatcher,
    /// Style to apply to matched text.
    pub style: PatternStyle,
}

impl HighlightPattern {
    /// Creates a new highlight pattern.
    pub fn new(pattern: &str, is_regex: bool, style: PatternStyle) -> Option<Self> {
        let matcher = if is_regex {
            Regex::new(pattern).ok().map(PatternMatcher::Regex)?
        } else {
            PatternMatcher::Plain(pattern.to_string())
        };

        Some(Self { matcher, style })
    }
}

/// Event pattern with associated style for line coloring and tracking.
#[derive(Debug, Clone)]
pub struct EventPattern {
    /// Name of the event.
    pub name: String,
    /// Matcher to identify lines matching this event.
    pub matcher: PatternMatcher,
    /// Style to apply to matched lines.
    pub style: PatternStyle,
}

impl EventPattern {
    /// Creates a new event pattern.
    pub fn new(name: &str, pattern: &str, is_regex: bool, style: PatternStyle) -> Option<Self> {
        let matcher = if is_regex {
            Regex::new(pattern).ok().map(PatternMatcher::Regex)?
        } else {
            PatternMatcher::Plain(pattern.to_string())
        };

        Some(Self {
            name: name.to_string(),
            matcher,
            style,
        })
    }
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

/// Styled range for rendering.
#[derive(Debug, Clone)]
pub struct StyledRange {
    /// Start position in text.
    pub start: usize,
    /// End position in text.
    pub end: usize,
    /// Pattern style
    pub style: PatternStyle,
}

/// Manages text highlighting and line coloring based on configured patterns.
#[derive(Debug)]
pub struct Highlighter {
    /// Patterns for text highlighting.
    patterns: Vec<HighlightPattern>,
    /// Event patterns for line coloring and tracking.
    events: Vec<EventPattern>,
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
    /// Creates a new highlighter with the given patterns.
    pub fn new(patterns: Vec<HighlightPattern>, events: Vec<EventPattern>) -> Self {
        Self {
            patterns,
            events,
            temporary_highlights: Vec::new(),
        }
    }

    /// Returns whether there are no highlight or event patterns.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty() && self.events.is_empty()
    }

    /// Returns all highlight patterns.
    pub fn get_patterns(&self) -> &[HighlightPattern] {
        &self.patterns
    }

    /// Returns all event patterns.
    pub fn get_events(&self) -> &[EventPattern] {
        &self.events
    }

    /// Returns the style for a line if it matches any event pattern.
    ///
    /// Returns the first matching event's style, or `None` if no pattern matches.
    pub fn get_line_style(&self, text: &str) -> Option<&PatternStyle> {
        for event in &self.events {
            if event.matcher.matches(text, true) {
                return Some(&event.style);
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
            if let Some(fg_color) = pattern.style.fg_color {
                for (start, end) in pattern.matcher.find_all(text, true) {
                    ranges.push((start, end, fg_color));
                }
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
            if let Some(fg_color) = pattern.style.fg_color {
                for (start, end) in pattern.matcher.find_all(text, true) {
                    ranges.push((start, end, fg_color, pattern.style.bg_color));
                }
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

    /// Returns styled ranges adjusted for horizontal offset, ready for rendering.
    ///
    /// Returns (styled_ranges, line_style).
    pub fn get_styled_ranges_for_viewport(
        &self,
        full_line: &str,
        horizontal_offset: usize,
        enable_colors: bool,
    ) -> (Vec<StyledRange>, Option<&PatternStyle>) {
        let mut ranges = Vec::new();

        // Always include temporary highlights
        ranges.extend(self.get_temporary_highlight_ranges(full_line));

        let line_style = if enable_colors {
            ranges.extend(self.get_config_highlight_ranges(full_line));
            self.get_line_style(full_line)
        } else {
            None
        };

        // Adjust ranges for horizontal offset
        let mut styled_ranges: Vec<StyledRange> = ranges
            .into_iter()
            .filter_map(|(start, end, fg, bg)| {
                if end <= horizontal_offset {
                    // Range is completely before visible area
                    None
                } else if start >= horizontal_offset {
                    // Range is in visible area, adjust coordinates
                    Some(StyledRange {
                        start: start - horizontal_offset,
                        end: end - horizontal_offset,
                        style: PatternStyle {
                            fg_color: Some(fg),
                            bg_color: bg,
                            bold: bg.is_some(), // Background highlights (search/filter) are bold
                        },
                    })
                } else {
                    // Range starts before offset but ends in visible area
                    Some(StyledRange {
                        start: 0,
                        end: end - horizontal_offset,
                        style: PatternStyle {
                            fg_color: Some(fg),
                            bg_color: bg,
                            bold: bg.is_some(),
                        },
                    })
                }
            })
            .collect();

        // Sort ranges by start position, then by priority (background > no background)
        styled_ranges.sort_by(|a, b| {
            a.start.cmp(&b.start).then_with(|| match (a.style.bg_color, b.style.bg_color) {
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            })
        });

        (styled_ranges, line_style)
    }

    /// Gets only config-based highlight ranges.
    fn get_config_highlight_ranges(&self, text: &str) -> Vec<(usize, usize, Color, Option<Color>)> {
        let mut ranges = Vec::new();

        for pattern in &self.patterns {
            if let Some(fg_color) = pattern.style.fg_color {
                for (start, end) in pattern.matcher.find_all(text, true) {
                    ranges.push((start, end, fg_color, pattern.style.bg_color));
                }
            }
        }

        ranges
    }

    /// Gets only temporary highlight ranges.
    fn get_temporary_highlight_ranges(
        &self,
        text: &str,
    ) -> Vec<(usize, usize, Color, Option<Color>)> {
        let mut ranges = Vec::new();

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
pub fn parse_color(color_str: &str) -> Option<Color> {
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

/// Generates a deterministic color from a pattern using djb2 hash.
pub fn hash_to_color(pattern: &str) -> Color {
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
