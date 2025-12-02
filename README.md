# lazylog

A terminal-based log viewer with search, filtering, and streaming capabilities.

![lazylog screenshot](docs/lazylog.png)

## Features

- **Syntax highlighting** - Configurable color patterns
- **Search and highlight** - Search the entire log file and highlight results
- **Filtering** - Include/exclude patterns for filtering lines
- **Event tracking** - Define event patterns and track these
- **Stream logs from stdin** - Pipe logs directly from any command
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

View multiple log files:
```bash
lazylog myapp_1.log myapp_2.log
```

Stream from stdin:
```bash
journalctl -f | lazylog
```

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
