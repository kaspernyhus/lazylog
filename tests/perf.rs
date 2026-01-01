use lazylog::highlighter::{HighlightPattern, Highlighter, PatternMatchType, PatternStyle, PlainMatch};
use lazylog::options::{AppOption, AppOptions};
use ratatui::style::Color;
use regex::Regex;
use std::time::{Duration, Instant};

/// Measures execution time of a function in nanoseconds
fn measure_time<F, R>(iterations: usize, mut f: F) -> Duration
where
    F: FnMut() -> R,
{
    // Prewarm cache
    for _ in 0..iterations {
        f();
    }

    let start = Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(f());
    }
    start.elapsed()
}

/// Sample log line for testing (typical production log line)
const SAMPLE_LOG_LINE: &str = "okt 18 21:20:22 archlinux INFO [thread-pool-1] com.example.service.UserService - Processing user request id=12345 user=john.doe@example.com status=active duration=42ms";

#[test]
fn perf_display_options_none_enabled() {
    let options = AppOptions::default();

    let iterations = 100000;

    let time = measure_time(iterations, || options.apply_to_line(SAMPLE_LOG_LINE));

    println!(
        "options (no options): total={:?} ({}), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_display_options_hide_pattern_enabled() {
    let mut app_options = AppOptions::default();
    app_options.enable(AppOption::HideTimestamp);

    let iterations = 100000;

    let time = measure_time(iterations, || app_options.apply_to_line(SAMPLE_LOG_LINE));

    println!(
        "options (hide pattern): total={:?} ({}), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_highlight_line_cache_hit() {
    let patterns = vec![
        HighlightPattern::new(
            "ERROR",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Red), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            "INFO",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Green), None, false),
        )
        .unwrap(),
    ];
    let highlighter = Highlighter::new(patterns, vec![]);

    let iterations = 10000;

    let time = measure_time(iterations, || highlighter.highlight_line(0, SAMPLE_LOG_LINE));

    println!(
        "Highlight line (cache hit):  total={:?} ({}), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_highlight_line_cache_miss() {
    let patterns = vec![
        HighlightPattern::new(
            "ERROR",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Red), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            "INFO",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Green), None, false),
        )
        .unwrap(),
    ];
    let highlighter = Highlighter::new(patterns, vec![]);

    let iterations = 10000;

    // Use different log_index values to force cache misses
    let mut counter = 0;
    let time = measure_time(iterations, || {
        counter += 1;
        highlighter.highlight_line(counter, SAMPLE_LOG_LINE)
    });

    println!(
        "Highlight line (cache miss): total={:?} ({}), {:.2}µs/iteration",
        time,
        iterations,
        time.as_micros() as f64 / iterations as f64
    );
}

#[test]
fn perf_plain_match_case_sensitive() {
    let matcher = PlainMatch {
        pattern: "INFO".to_string(),
        case_sensitive: true,
    };

    let iterations = 10000;

    let time = measure_time(iterations, || matcher.is_match(SAMPLE_LOG_LINE));

    println!(
        "Plain match (case sensitive): total={:?} ({}), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_plain_match_case_insensitive() {
    let matcher = PlainMatch {
        pattern: "info".to_string(),
        case_sensitive: false,
    };

    let iterations = 10000;

    let time = measure_time(iterations, || matcher.is_match(SAMPLE_LOG_LINE));

    println!(
        "Plain match (case insensitive): total={:?} ({}), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_regex_match() {
    let matcher = Regex::new(r"\d{4}-\d{2}-\d{2}").unwrap();

    let iterations = 10000;

    let time = measure_time(iterations, || matcher.is_match(SAMPLE_LOG_LINE));

    println!(
        "Regex match: total={:?} ({} iterations), {:.2}ns/iteration",
        time,
        iterations,
        time.as_nanos() as f64 / iterations as f64
    );
}

#[test]
fn perf_highlight_multiple_patterns() {
    let patterns = vec![
        HighlightPattern::new(
            "ERROR",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Red), None, true),
        )
        .unwrap(),
        HighlightPattern::new(
            "WARN",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Yellow), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            "INFO",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Green), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            "DEBUG",
            PatternMatchType::Plain(false),
            PatternStyle::new(Some(Color::Blue), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            r"\d+ms",
            PatternMatchType::Regex,
            PatternStyle::new(Some(Color::Cyan), None, false),
        )
        .unwrap(),
        HighlightPattern::new(
            r"id=\d+",
            PatternMatchType::Regex,
            PatternStyle::new(Some(Color::Magenta), None, false),
        )
        .unwrap(),
    ];
    let highlighter = Highlighter::new(patterns, vec![]);

    let iterations = 10000;

    let mut counter = 0;
    let time = measure_time(iterations, || {
        counter += 1;
        highlighter.highlight_line(counter, SAMPLE_LOG_LINE)
    });

    println!(
        "Highlight with 6 patterns: total={:?} ({} iterations), {:.2}µs/iteration",
        time,
        iterations,
        time.as_micros() as f64 / iterations as f64
    );
}
