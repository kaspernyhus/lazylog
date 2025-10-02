use ratatui::style::Color;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct HighlightPattern {
    pub pattern: String,
    pub color: Color,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    highlight_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct Highlighter {
    patterns: Vec<HighlightPattern>,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    pub fn new() -> Self {
        let mut patterns = Vec::new();

        patterns.extend(Self::load_user_patterns());
        patterns.extend(Self::hardcoded_patterns());

        Self { patterns }
    }

    fn hardcoded_patterns() -> Vec<HighlightPattern> {
        vec![]
    }

    fn load_user_patterns() -> Vec<HighlightPattern> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            return Vec::new();
        }

        match std::fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(config) => Self::assign_colors(config.highlight_patterns),
                Err(_) => Vec::new(),
            },
            Err(_) => Vec::new(),
        }
    }

    fn config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("lazylog").join("config.toml")
        } else {
            PathBuf::from(".lazylog.toml")
        }
    }

    fn assign_colors(patterns: Vec<String>) -> Vec<HighlightPattern> {
        patterns
            .into_iter()
            .map(|pattern| {
                let color = Self::hash_to_color(&pattern);
                HighlightPattern { pattern, color }
            })
            .collect()
    }

    fn hash_to_color(pattern: &str) -> Color {
        let mut hash: u32 = 5381;
        for byte in pattern.bytes() {
            hash = hash.wrapping_mul(33).wrapping_add(byte as u32);
        }

        // Map to color indices 16-255 (240 colors)
        // Skipping 0-15 which are the basic ANSI colors for better variety
        let color_index = 16 + (hash % 240) as u8;
        Color::Indexed(color_index)
    }

    pub fn get_patterns(&self) -> &[HighlightPattern] {
        &self.patterns
    }
}
