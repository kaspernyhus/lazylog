# lazylog

A terminal-based log viewer with search, filtering, and streaming capabilities.

![lazylog screenshot](docs/lazylog.png)

## Features

- **Stream logs from stdin** - Pipe logs directly from any command
- **Search and highlight** - Fast search with case-insensitive option
- **Filtering** - Include/exclude patterns
- **Syntax highlighting** - Configurable color patterns
- **Save streams** - Export stdin streams to files

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

## Configuration

Color highlighting patterns can be configured by creating a `config.toml` file:
`~/.config/lazylog/config.toml`

Or use a custom config file with the `-c` option:
```bash
lazylog -c /path/to/config.toml myapp.log
```

### Color Configuration

**Line Colors** - Color entire lines when patterns match:
```toml
line_colors = [
    { pattern = " ERROR ", color = "lightred", regex = false },
    { pattern = " WARN", color = "yellow", regex = false },
]
```

**Highlight Patterns** - Highlight specific patterns within lines:
```toml
highlight_patterns = [
    { pattern = "\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}\\.\\d{1,3}", regex = true },  # IP addresses
    { pattern = "TODO", color = "lightmagenta", regex = false },  # Custom color
]
```

If no `color` is specified for highlight patterns, a unique color is auto-assigned.

**Supported colors:** red, green, yellow, blue, magenta, cyan, white, black, gray, lightred, lightgreen, lightyellow, lightblue, lightmagenta, lightcyan, darkgray

See `examples/config.toml` for a complete example.
