use ratatui::style::Color;

/// Symbol used to indicate the selected line.
pub const RIGHT_ARROW: &str = "▶";
/// Three-quarters block for mark indicator.
pub const MARK_INDICATOR: &str = "▊";

/// Common colors
pub const GRAY_COLOR: Color = Color::Indexed(237);
pub const BLACK_COLOR: Color = Color::Indexed(234);
pub const BRIGHT_WHITE_COLOR: Color = Color::Rgb(255, 255, 255);
pub const WHITE_COLOR: Color = Color::White;

// Footer
pub const FOOTER_BG: Color = GRAY_COLOR;

// Scrollbar
pub const SCROLLBAR_FG: Color = GRAY_COLOR;

// Search colors
pub const SEARCH_MODE_FG: Color = BLACK_COLOR;
pub const SEARCH_MODE_BG: Color = Color::Yellow;

// Filter mode colors
pub const FILTER_MODE_FG: Color = BLACK_COLOR;
pub const FILTER_MODE_BG: Color = Color::Cyan;
pub const FILTER_LIST_HIGHLIGHT_BG: Color = GRAY_COLOR;
pub const FILTER_ENABLED_FG: Color = Color::Green;
pub const FILTER_DISABLED_FG: Color = Color::DarkGray;

// Events
pub const DEFAULT_EVENT_FG: Color = WHITE_COLOR;
pub const DEFAULT_EVENT_BG: Color = Color::Blue;
pub const EVENT_LIST_BG: Color = Color::Blue;
pub const EVENT_LIST_HIGHLIGHT_BG: Color = GRAY_COLOR;
pub const EVENT_NAME_FG: Color = Color::Yellow;
pub const EVENT_LINE_PREVIEW: Color = Color::Gray;

// Marks
pub const MARK_MODE_FG: Color = Color::White;
pub const MARK_MODE_BG: Color = Color::Indexed(29);
pub const MARK_LIST_HIGHLIGHT_BG: Color = GRAY_COLOR;
pub const MARK_INDICATOR_COLOR: Color = Color::Indexed(29);
pub const MARK_NAME_FG: Color = Color::Yellow;
pub const MARK_LINE_PREVIEW: Color = Color::Gray;

// Help
pub const HELP_BG: Color = Color::Blue;
pub const HELP_BORDER_FG: Color = Color::White;
pub const HELP_HEADER_FG: Color = Color::Yellow;
pub const HELP_HIGHLIGHT_FG: Color = Color::LightBlue;

// Option
pub const OPTION_ENABLED_FG: Color = Color::Green;
pub const OPTION_DISABLED_FG: Color = Color::White;

// Files
pub const FILE_BORDER: Color = Color::Indexed(108);
pub const FILE_ENABLED_FG: Color = Color::Green;
pub const FILE_DISABLED_FG: Color = Color::White;

// Messages
pub const MESSAGE_INFO_FG: Color = WHITE_COLOR;
pub const MESSAGE_BORDER: Color = Color::Blue;
pub const MESSAGE_ERROR_FG: Color = Color::Red;

// Selection colors
pub const SELECTION_BG: Color = Color::LightBlue;

// File ID colors
pub const FILE_ID_COLORS: &[Color] = &[Color::Indexed(24), Color::Indexed(108), Color::Indexed(168)];
