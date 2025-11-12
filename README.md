# lazylog

A terminal-based log viewer with search, filtering, and streaming capabilities.

![lazylog screenshot](docs/lazylog.png)

## Features

- **Stream logs from stdin** - Pipe logs directly from any command
- **Search and highlight** - Fast search with case-insensitive option
- **Filtering** - Include/exclude patterns
- **Event tracking** - Define event patterns and track these
- **Syntax highlighting** - Configurable color patterns
- **Save streams** - Export stdin streams to files
- **Multi-file merging** - Merge multiple log files by timestamp

## Installation
Installs to `/usr/local/bin/`
```bash
./install.sh
```

## Usage

View a log file:
```bash
lazylog myapp.log
```

Stream from stdin:
```bash
journalctl -f | lazylog
```

Merge multiple log files by timestamp:
```bash
lazylog app.log server.log worker.log
```

When viewing merged logs:
- Each line is prefixed with a colored symbol (`[A]`, `[B]`, `[C]`) indicating its source file
- Lines are sorted chronologically by timestamp
- Press `Shift+S` to open the source files menu
- Use arrow keys or `j`/`k` to navigate, `Space` or `Enter` to toggle file visibility
- The footer shows how many source files are visible (e.g., "2/3 sources")

**Note:** Lines without parseable timestamps are skipped during merge. Supported timestamp formats include:
- ISO 8601 / RFC 3339: `2024-01-15T10:30:45`, `2024-01-15 10:30:45`
- Syslog: `Jan 15 10:30:45`, `2024 Jan 15 10:30:45`

## Configuration

Color highlighting patterns can be configured by creating a `config.toml` file:
`~/.config/lazylog/config.toml`

Use a custom config file with the `-c` option:
```bash
lazylog -c /path/to/config.toml myapp.log
```

Load predefined filters from a separate file with the `-f` or `--filters` option:
```bash
lazylog --filters /path/to/filters.toml myapp.log
```

### Color Configuration

**Highlights** - Highlight specific patterns within lines:
```toml
highlights = [
    { pattern = "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}", regex = true },  # IP addresses
    { pattern = "TODO", regex = false, style = { fg = "lightmagenta" } },  # Custom style
]
```

If no `style` is specified for highlights, a unique color is auto-assigned.

**Events** - Color entire lines and track events when patterns match:
```toml
events = [
    { name = "Error", pattern = " ERROR ", regex = false, style = { fg = "lightred", bold = true } },
    { name = "Warning", pattern = " WARN", regex = false, style = { fg = "yellow" } },
    { name = "Critical", pattern = " CRITICAL ", regex = false, style = { bg = "red" } },
]
```

**Filters** - Predefined filters.
```toml
filters = [
    { pattern = "DEBUG", mode = "exclude", case_sensitive = false, enabled = true },
    { pattern = "INFO", mode = "include", case_sensitive = true, enabled = false },
]
```

**Multi-file Merge Settings** - Configure timestamp parsing and merge behavior:
```toml
[merge]
# Custom timestamp formats (optional, in addition to auto-detected formats)
# custom_timestamp_formats = [
#     "%Y-%m-%d %H:%M:%S%.f",
#     "%d/%b/%Y:%H:%M:%S",
# ]

# Show merge statistics after loading (default: true)
show_merge_stats = true
```

**Supported colors:** red, green, yellow, blue, magenta, cyan, white, black, gray, lightred, lightgreen, lightyellow, lightblue, lightmagenta, lightcyan, darkgray

See `examples/config.toml` for a complete configuration example and `examples/filters.toml` for a filters file example.

## Development
Clone the repository and build with Cargo:
```bash
cargo build
```
Run the application:
```bash
cargo run -- path/to/logfile.log
```
Run unit tests:
```bash
cargo test -- --skip perf
```
Run performance tests:
```bash
cargo test --release --test perf -- --nocapture --test-threads=1
```
