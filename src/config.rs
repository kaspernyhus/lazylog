use crate::highlighter::{hash_to_color, parse_color, HighlightPattern, Highlighter, LineColorPattern};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct HighlightConfig {
    /// Match pattern. Can be a substring or regex.
    pub pattern: String,
    /// Whether the pattern is a regex or a simple substring.
    #[serde(default)]
    pub regex: bool,
    /// Color to use for highlighting. If None, a color will be generated.
    #[serde(default)]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LineColorConfig {
    /// Match pattern. Can be a substring or regex.
    pub pattern: String,
    /// Color to use for the whole line.
    pub color: String,
    /// Whether the pattern is a regex or a simple substring.
    #[serde(default)]
    pub regex: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    path: Option<String>,
    /// Single patterns to color highlight
    #[serde(default)]
    pub highlight_patterns: Vec<HighlightConfig>,
    /// Whole lines to color when pattern matches
    #[serde(default)]
    pub line_colors: Vec<LineColorConfig>,
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
        let line_colors = self.parse_line_colors();
        Highlighter::new(patterns, line_colors)
    }

    fn parse_highlight_patterns(&self) -> Vec<HighlightPattern> {
        self.highlight_patterns
            .iter()
            .filter_map(|config| {
                let color = config
                    .color
                    .as_ref()
                    .and_then(|c| parse_color(c))
                    .unwrap_or_else(|| hash_to_color(&config.pattern));

                HighlightPattern::new(&config.pattern, config.regex, color)
            })
            .collect()
    }

    fn parse_line_colors(&self) -> Vec<LineColorPattern> {
        self.line_colors
            .iter()
            .filter_map(|config| {
                let color = parse_color(&config.color)?;
                LineColorPattern::new(&config.pattern, config.regex, color)
            })
            .collect()
    }
}
