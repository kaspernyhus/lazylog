use crate::highlighter::{
    EventPattern, HighlightPattern, Highlighter, PatternStyle, hash_to_color, parse_color,
};
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

    fn parse_highlight_patterns(&self) -> Vec<HighlightPattern> {
        self.highlights
            .iter()
            .filter_map(|hl_config| {
                let style = if let Some(style_config) = &hl_config.style {
                    Self::parse_style_config(style_config)
                } else {
                    PatternStyle {
                        fg_color: Some(hash_to_color(&hl_config.pattern)),
                        bg_color: None,
                        bold: false,
                    }
                };

                HighlightPattern::new(&hl_config.pattern, hl_config.regex, style)
            })
            .collect()
    }

    fn parse_event_patterns(&self) -> Vec<EventPattern> {
        self.events
            .iter()
            .filter_map(|ev_config| {
                let style = ev_config
                    .style
                    .as_ref()
                    .map(Self::parse_style_config)
                    .unwrap_or_else(PatternStyle::default_event_style);

                EventPattern::new(&ev_config.name, &ev_config.pattern, ev_config.regex, style)
            })
            .collect()
    }

    fn parse_style_config(style_config: &StyleConfig) -> PatternStyle {
        PatternStyle {
            fg_color: style_config.fg.as_ref().and_then(|c| parse_color(c)),
            bg_color: style_config.bg.as_ref().and_then(|c| parse_color(c)),
            bold: style_config.bold,
        }
    }
}
