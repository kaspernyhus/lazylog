use crate::filter::{ActiveFilterMode, FilterPattern};
use crate::highlighter::{HighlightPattern, Highlighter, PatternMatchType, PatternStyle};
use ratatui::style::Color;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    path: Option<String>,
    /// Inline patterns to highlight.
    #[serde(default)]
    pub highlights: Vec<HighlightConfig>,
    /// Event patterns for coloring and tracking.
    #[serde(default)]
    pub events: Vec<EventConfig>,
    /// Predefined filters.
    #[serde(default)]
    pub filters: Vec<FilterConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Filters {
    /// Predefined filters.
    #[serde(default)]
    pub filters: Vec<FilterConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StyleConfig {
    /// Foreground color.
    #[serde(default)]
    pub fg: Option<String>,
    /// Background color.
    #[serde(default)]
    pub bg: Option<String>,
    /// Bold text.
    #[serde(default)]
    pub bold: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HighlightConfig {
    /// Match pattern. Can be a substring or regex.
    pub pattern: String,
    /// Whether the pattern is a regex or a simple substring.
    #[serde(default)]
    pub regex: bool,
    /// Whether the pattern matching is case-sensitive.
    #[serde(default)]
    pub case_sensitive: bool,
    /// Style to use for highlighting. If None, a style will be generated.
    #[serde(default)]
    pub style: Option<StyleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EventConfig {
    /// Name of the event.
    pub name: String,
    /// Match pattern. Can be a substring or regex.
    pub pattern: String,
    /// Whether the pattern is a regex or a simple substring.
    #[serde(default)]
    pub regex: bool,
    /// Style to use for the whole line. If None, default style is applied.
    #[serde(default)]
    pub style: Option<StyleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FilterConfig {
    /// Match pattern.
    pub pattern: String,
    /// Filter mode: "include" or "exclude".
    #[serde(default)]
    pub mode: String,
    /// Whether the pattern matching is case-sensitive.
    #[serde(default)]
    pub case_sensitive: bool,
    /// Whether this filter is enabled by default.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Filters {
    /// Load filters from a specified file path.
    pub fn load(path: &Option<String>) -> Option<Self> {
        path.as_ref().and_then(|p| {
            let filters_path = PathBuf::from(p);
            if filters_path.exists() {
                match std::fs::read_to_string(&filters_path) {
                    Ok(content) => toml::from_str(&content).ok(),
                    Err(_) => None,
                }
            } else {
                None
            }
        })
    }

    /// Convert to FilterPattern vector.
    pub fn parse_filter_patterns(&self) -> Vec<crate::filter::FilterPattern> {
        self.filters
            .iter()
            .map(|filter_config| {
                let mode = match filter_config.mode.to_lowercase().as_str() {
                    "exclude" => ActiveFilterMode::Exclude,
                    _ => ActiveFilterMode::Include,
                };

                FilterPattern {
                    pattern: filter_config.pattern.clone(),
                    mode,
                    case_sensitive: filter_config.case_sensitive,
                    enabled: filter_config.enabled,
                }
            })
            .collect()
    }
}

impl Config {
    /// Load configuration from the specified path, the default config dir (~/.config/lazylog/) or a local .lazylog.toml.
    pub fn load(path: &Option<String>) -> Self {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            Self::default_config_dir()
        };
        Self::load_from_path(&config_path)
    }

    fn load_from_path(config_path: &PathBuf) -> Self {
        if config_path.exists() {
            match std::fs::read_to_string(config_path) {
                Ok(content) => {
                    let mut config: Config = toml::from_str(&content).unwrap_or_default();
                    config.path = config_path.to_str().map(|s| s.to_string());
                    config
                }
                Err(_) => Self::default(),
            }
        } else {
            Self::default()
        }
    }

    /// Get the path of the configuration file if it was loaded from a file.
    pub fn get_path(&self) -> Option<&String> {
        self.path.as_ref()
    }

    fn default_config_dir() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("lazylog").join("config.toml");
            if config_path.exists() {
                return config_path;
            }
        }
        // Fallback to local .lazylog.toml (might not exist)
        PathBuf::from(".lazylog.toml")
    }

    /// Builds a Highlighter from the configuration.
    pub fn build_highlighter(&self) -> Highlighter {
        let patterns = self.parse_highlight_patterns();
        let events = self.parse_event_patterns();
        Highlighter::new(patterns, events)
    }

    /// Parses filter configurations and returns a list of FilterPatterns.
    pub fn parse_filter_patterns(&self) -> Vec<crate::filter::FilterPattern> {
        use crate::filter::{ActiveFilterMode, FilterPattern};

        self.filters
            .iter()
            .map(|filter_config| {
                let mode = match filter_config.mode.to_lowercase().as_str() {
                    "exclude" => ActiveFilterMode::Exclude,
                    _ => ActiveFilterMode::Include, // Default to Include
                };

                FilterPattern {
                    pattern: filter_config.pattern.clone(),
                    mode,
                    case_sensitive: filter_config.case_sensitive,
                    enabled: filter_config.enabled,
                }
            })
            .collect()
    }

    fn parse_highlight_patterns(&self) -> Vec<HighlightPattern> {
        self.highlights
            .iter()
            .filter_map(|hl_config| {
                let style = if let Some(style_config) = &hl_config.style {
                    Self::parse_style_config(style_config)
                } else {
                    PatternStyle {
                        fg_color: Some(Self::hash_to_color(&hl_config.pattern)),
                        bg_color: None,
                        bold: false,
                    }
                };

                let match_type = if hl_config.regex {
                    PatternMatchType::Regex
                } else {
                    PatternMatchType::Plain(hl_config.case_sensitive)
                };

                HighlightPattern::new(&hl_config.pattern, match_type, style, None)
            })
            .collect()
    }

    fn parse_event_patterns(&self) -> Vec<HighlightPattern> {
        self.events
            .iter()
            .filter_map(|ev_config| {
                let style = ev_config
                    .style
                    .as_ref()
                    .map(Self::parse_style_config)
                    .unwrap_or_else(PatternStyle::default_colors);

                let match_type = if ev_config.regex {
                    PatternMatchType::Regex
                } else {
                    PatternMatchType::Plain(true)
                };

                HighlightPattern::new(&ev_config.pattern, match_type, style, Some(ev_config.name.clone()))
            })
            .collect()
    }

    fn parse_style_config(style_config: &StyleConfig) -> PatternStyle {
        PatternStyle {
            fg_color: style_config.fg.as_ref().and_then(|c| Self::parse_color(c)),
            bg_color: style_config.bg.as_ref().and_then(|c| Self::parse_color(c)),
            bold: style_config.bold,
        }
    }

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
}
