use chrono::{DateTime, Datelike, NaiveDateTime, Utc};
use regex::Regex;
use std::sync::LazyLock;

static ISO8601_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:?\d{2})?")
        .unwrap()
});

static COMMON_DATETIME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d{1,9})?").unwrap());

static SYSLOG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})",
    )
    .unwrap()
});

/// Attempts to parse a timestamp from a log line using multiple common formats
pub fn parse_timestamp(line: &str) -> Option<DateTime<Utc>> {
    // ISO 8601 / RFC 3339 formats
    // Examples: 2024-01-15T10:30:45, 2024-01-15T10:30:45.123Z, 2024-01-15T10:30:45+0200
    if let Some(dt) = try_iso8601(line) {
        return Some(dt);
    }

    // Common log format: YYYY-MM-DD HH:MM:SS
    // Example: 2024-01-15 10:30:45
    if let Some(dt) = try_common_datetime(line) {
        return Some(dt);
    }

    // syslog format: MMM DD HH:MM:SS
    // Example: Jan 15 10:30:45
    if let Some(dt) = try_syslog_format(line) {
        return Some(dt);
    }

    None
}

/// Try to parse ISO 8601 / RFC 3339 format
fn try_iso8601(line: &str) -> Option<DateTime<Utc>> {
    let timestamp_str = ISO8601_RE.find(line)?.as_str();

    // Try RFC 3339 first (with colon in timezone like +02:00)
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // Handle timezone offset without colon (e.g., +0200 from journalctl)
    if let Some(tz_pos) = timestamp_str.rfind(['+', '-'])
        && tz_pos > 0 && !timestamp_str[tz_pos..].contains(':') {
            let mut normalized = timestamp_str.to_string();
            if normalized.len() == tz_pos + 5 {
                normalized.insert(tz_pos + 3, ':');
                if let Ok(dt) = DateTime::parse_from_rfc3339(&normalized) {
                    return Some(dt.with_timezone(&Utc));
                }
            }
        }

    let formats = [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ];

    for format in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(timestamp_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    None
}

/// Try to parse common datetime format: YYYY-MM-DD HH:MM:SS
fn try_common_datetime(line: &str) -> Option<DateTime<Utc>> {
    let timestamp_str = COMMON_DATETIME_RE.find(line)?.as_str();

    let formats = ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S"];

    for format in &formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(timestamp_str, format) {
            return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
        }
    }

    None
}

/// Try to parse syslog format: MMM DD HH:MM:SS (assumes current year)
fn try_syslog_format(line: &str) -> Option<DateTime<Utc>> {
    let caps = SYSLOG_RE.captures(line)?;

    let month = caps.get(1)?.as_str();
    let day: u32 = caps.get(2)?.as_str().trim().parse().ok()?;
    let hour: u32 = caps.get(3)?.as_str().parse().ok()?;
    let minute: u32 = caps.get(4)?.as_str().parse().ok()?;
    let second: u32 = caps.get(5)?.as_str().parse().ok()?;

    // add year
    let year = Utc::now().year();

    let timestamp_str = format!(
        "{} {} {} {:02}:{:02}:{:02}",
        year, month, day, hour, minute, second
    );

    if let Ok(naive) = NaiveDateTime::parse_from_str(&timestamp_str, "%Y %b %d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iso8601_basic() {
        let line = "2025-09-12T10:28:19.304534+0200 pipewire[632]: pw.port:";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_syslog_format() {
        let line = "Nov 04 13:04:44 speaker-6-123456 pipewire[632]: pw.port:";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_no_timestamp() {
        let line = "This line has no timestamp";
        let result = parse_timestamp(line);
        assert!(result.is_none());
    }

    #[test]
    fn test_ordering() {
        let line1 = "2025-09-12T10:28:19.304534+0200 First event";
        let line2 = "2025-09-12T12:13:59.131632+0200 Second event";

        let dt1 = parse_timestamp(line1).unwrap();
        let dt2 = parse_timestamp(line2).unwrap();

        assert!(dt1 < dt2);
    }
}
