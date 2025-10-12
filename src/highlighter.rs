use ratatui::style::{Color, Modifier, Style};
use regex::Regex;

/// Style configuration for text rendering.
#[derive(Debug, Clone, Copy, Default)]
pub struct PatternStyle {
    /// Foreground color.
    pub fg_color: Option<Color>,
    /// Background color.
    pub bg_color: Option<Color>,
    /// Bold text.
    pub bold: bool,
}

impl PatternStyle {
    /// Creates a new pattern style.
    pub fn new(fg_color: Option<Color>, bg_color: Option<Color>, bold: bool) -> Self {
        Self {
            fg_color,
            bg_color,
            bold,
        }
    }

    /// Creates a PatternStyle with blue bg and white fg.
    pub fn white_on_blue() -> Self {
        Self {
            fg_color: Some(Color::Rgb(255, 255, 255)),
            bg_color: Some(Color::Blue),
            bold: false,
        }
    }

    /// Convert to ratatui Style.
    pub fn to_ratatui(&self) -> Style {
        let mut ratatui_style = Style::default();
        if let Some(fg) = self.fg_color {
            ratatui_style = ratatui_style.fg(fg);
        }
        if let Some(bg) = self.bg_color {
            ratatui_style = ratatui_style.bg(bg);
        }
        if self.bold {
            ratatui_style = ratatui_style.add_modifier(Modifier::BOLD);
        }
        ratatui_style
    }
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
            self.contains_ignore_case(haystack)
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

    /// Returns true if the given needle matches a sub-slice of haystack string slice ignoring the case.
    ///
    /// Returns false if it does not.
    fn contains_ignore_case(&self, haystack: &str) -> bool {
        if self.pattern.is_empty() {
            return true;
        }
        if self.pattern.len() > haystack.len() {
            return false;
        }
        haystack
            .as_bytes()
            .windows(self.pattern.len())
            .any(|window| window.eq_ignore_ascii_case(self.pattern.as_bytes()))
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

/// Pattern with associated color for text highlighting.
#[derive(Debug, Clone)]
pub struct HighlightPattern {
    /// Optional name to display
    pub name: Option<String>,
    /// Matcher to identify text spans to highlight.
    pub matcher: PatternMatcher,
    /// Style to apply to matched text.
    pub style: PatternStyle,
}

impl HighlightPattern {
    /// Creates a new highlight pattern.
    pub fn new(
        pattern: &str,
        is_regex: bool,
        style: PatternStyle,
        name: Option<String>,
    ) -> Option<Self> {
        let matcher = if is_regex {
            Regex::new(pattern).ok().map(PatternMatcher::Regex)?
        } else {
            PatternMatcher::Plain(PlainMatch {
                pattern: pattern.to_string(),
                case_sensitive: false,
            })
        };
        Some(Self {
            name,
            matcher,
            style,
        })
    }
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

/// Complete highlighting information for a single line, ready to render.
#[derive(Debug)]
pub struct HighlightedLine {
    /// Non-overlapping segments with styles, in order.
    pub segments: Vec<StyledRange>,
}

/// Manages text highlighting and line coloring based on configured patterns.
#[derive(Debug)]
pub struct Highlighter {
    /// Patterns for text highlighting.
    patterns: Vec<HighlightPattern>,
    /// Event patterns for line coloring and tracking.
    events: Vec<HighlightPattern>,
    /// Temporary highlights.
    temporary_highlights: Vec<HighlightPattern>,
}

impl Highlighter {
    /// Creates a new highlighter with the given patterns.
    pub fn new(patterns: Vec<HighlightPattern>, events: Vec<HighlightPattern>) -> Self {
        Self {
            patterns,
            events,
            temporary_highlights: Vec::new(),
        }
    }

    /// Returns the style for the whole line if it matches any event pattern.
    ///
    /// Returns the first matching event's style, or `None` if no pattern matches.
    pub fn get_line_style(&self, text: &str) -> Option<PatternStyle> {
        for event in &self.events {
            if event.matcher.matches(text) {
                return Some(event.style);
            }
        }
        None
    }

    /// Adds a temporary highlight pattern to be applied on top of any other highlighting.
    pub fn add_temporary_highlight(
        &mut self,
        pattern: String,
        style: PatternStyle,
        case_sensitive: bool,
    ) {
        self.temporary_highlights.push(HighlightPattern {
            name: None,
            matcher: PatternMatcher::Plain(PlainMatch {
                pattern,
                case_sensitive,
            }),
            style,
        });
    }

    /// Clears all temporary highlights.
    pub fn clear_temporary_highlights(&mut self) {
        self.temporary_highlights.clear();
    }

    /// Returns a HighlightedLine with all styling information ready to render.
    pub fn highlight_line(
        &self,
        line: &str,
        horizontal_offset: usize,
        enable_colors: bool,
    ) -> HighlightedLine {
        let mut ranges = Vec::new();

        // Step 1: Add event pattern as base style (full line background)
        if enable_colors {
            if let Some(line_style) = self.get_line_style(line) {
                ranges.push(StyledRange {
                    start: 0,
                    end: line.len(),
                    style: line_style,
                });
            }

            // Step 2: Add configured highlight patterns
            for pattern in &self.patterns {
                for (start, end) in pattern.matcher.find_all(line) {
                    ranges.push(StyledRange {
                        start,
                        end,
                        style: pattern.style,
                    });
                }
            }
        }

        // Step 3: Add temporary highlights (always shown, even if colors disabled)
        for highlight in &self.temporary_highlights {
            for (start, end) in highlight.matcher.find_all(line) {
                ranges.push(StyledRange {
                    start,
                    end,
                    style: highlight.style,
                });
            }
        }

        // Step 4: Adjust for horizontal scrolling
        let styled_ranges = self.adjust_for_viewport_offset(ranges, horizontal_offset);

        // Step 5: Resolve overlaps into non-overlapping segments
        let segments = self.split_into_segments(styled_ranges);

        HighlightedLine { segments }
    }

    /// Adjusts ranges for horizontal scrolling offset.
    fn adjust_for_viewport_offset(
        &self,
        ranges: Vec<StyledRange>,
        offset: usize,
    ) -> Vec<StyledRange> {
        ranges
            .into_iter()
            .filter_map(|styled_range| {
                if styled_range.end <= offset {
                    // Range ends before viewport - not visible, discard
                    None
                } else if styled_range.start >= offset {
                    // Range entirely within viewport - shift coordinates
                    Some(StyledRange {
                        start: styled_range.start - offset,
                        end: styled_range.end - offset,
                        style: styled_range.style,
                    })
                } else {
                    // Range starts before viewport but extends into it - clip at viewport start
                    Some(StyledRange {
                        start: 0,
                        end: styled_range.end - offset,
                        style: styled_range.style,
                    })
                }
            })
            .collect()
    }

    /// Splits overlapping ranges into non-overlapping segments, merging styles as needed.
    fn split_into_segments(&self, ranges: Vec<StyledRange>) -> Vec<StyledRange> {
        if ranges.is_empty() {
            return Vec::new();
        }

        let mut result: Vec<StyledRange> = Vec::new();

        for range in ranges {
            // Temp storage for split segments created during overlap resolution
            let mut splits = Vec::new();

            // Check if this range should inherit background color
            let should_inherit_bg =
                range.style.bg_color.is_none() && range.style.fg_color.is_some();

            // Find background to preserve BEFORE modifying result
            let bg_to_preserve = if should_inherit_bg {
                result
                    .iter()
                    .find(|r| {
                        // Find any overlapping range that has a background
                        r.style.bg_color.is_some()
                            && !(r.end <= range.start || r.start >= range.end)
                    })
                    .and_then(|r| r.style.bg_color)
            } else {
                None
            };

            // Handle overlaps: remove/trim/split existing ranges as needed
            result.retain_mut(|existing| {
                // Case 1: No overlap - keep existing range
                if range.start >= existing.end || range.end <= existing.start {
                    return true;
                }

                // Case 2: New range completely covers existing - remove existing
                if range.start <= existing.start && range.end >= existing.end {
                    return false;
                }

                // Case 3: New range is inside existing - split existing into left and right parts
                //   existing: [--------]
                //   new:          [--]
                //   result:   [--]    [--]
                if range.start > existing.start && range.end < existing.end {
                    splits.push(StyledRange {
                        start: range.end,
                        end: existing.end,
                        style: existing.style,
                    });
                    existing.end = range.start;
                    return true;
                }

                // Case 4: New range overlaps right side - trim existing on right
                //   existing: [--------]
                //   new:            [------]
                //   result:   [----]
                if range.start > existing.start {
                    existing.end = range.start;
                    return true;
                }

                // Case 5: New range overlaps left side - trim existing on left
                //   existing:       [--------]
                //   new:      [------]
                //   result:            [----]
                if range.end < existing.end {
                    existing.start = range.end;
                    return true;
                }

                // Unreachable: all overlap cases are handled above
                true
            });

            // Create the final range, possibly with inherited background
            let merged_range = if let Some(bg_color) = bg_to_preserve {
                // Prevent invisible text: if fg == bg, use white instead
                let fg_color = if range.style.fg_color == Some(bg_color) {
                    Some(Color::Rgb(255, 255, 255))
                } else {
                    range.style.fg_color
                };

                StyledRange {
                    start: range.start,
                    end: range.end,
                    style: PatternStyle {
                        fg_color,
                        bg_color: Some(bg_color),
                        bold: range.style.bold,
                    },
                }
            } else {
                range
            };

            // Add the new range and any split fragments
            result.push(merged_range);
            result.extend(splits);
        }

        // Sort by position for correct rendering order
        result.sort_by_key(|r| r.start);
        result
    }
}
