use chrono::{DateTime, Datelike, NaiveDateTime, Utc};

/// Attempts to parse a timestamp from a log line using multiple common formats
pub fn parse_timestamp(line: &str) -> Option<DateTime<Utc>> {
    // Try different timestamp formats in order of likelihood

    // ISO 8601 / RFC 3339 formats (most common in modern logs)
    // Examples: 2024-01-15T10:30:45, 2024-01-15T10:30:45.123Z, 2024-01-15T10:30:45+00:00
    if let Some(dt) = try_iso8601(line) {
        return Some(dt);
    }

    // Common log format: YYYY-MM-DD HH:MM:SS
    // Example: 2024-01-15 10:30:45
    if let Some(dt) = try_common_datetime(line) {
        return Some(dt);
    }

    // Syslog format: MMM DD HH:MM:SS (without year)
    // Example: Jan 15 10:30:45
    if let Some(dt) = try_syslog_format(line) {
        return Some(dt);
    }

    // Syslog with year: YYYY MMM DD HH:MM:SS
    // Example: 2024 Jan 15 10:30:45
    if let Some(dt) = try_syslog_with_year(line) {
        return Some(dt);
    }

    None
}

/// Try to parse ISO 8601 / RFC 3339 formats
fn try_iso8601(line: &str) -> Option<DateTime<Utc>> {
    // Look for patterns like: 2024-01-15T10:30:45
    use regex::Regex;

    let re = Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:\.\d{1,9})?(?:Z|[+-]\d{2}:?\d{2})?").ok()?;
    let timestamp_str = re.find(line)?.as_str();

    // Try parsing with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try parsing with 'Z' suffix
    if timestamp_str.ends_with('Z') {
        if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp_str) {
            return Some(dt.with_timezone(&Utc));
        }
    }

    // Try parsing without timezone (assume UTC)
    // Handle both 'T' and space separators
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
    use regex::Regex;

    let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d{1,9})?").ok()?;
    let timestamp_str = re.find(line)?.as_str();

    let formats = [
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

/// Try to parse syslog format: MMM DD HH:MM:SS (assumes current year)
fn try_syslog_format(line: &str) -> Option<DateTime<Utc>> {
    use regex::Regex;

    // Match: Jan 15 10:30:45 or Jan  5 10:30:45 (note: day can be single digit with leading space)
    let re = Regex::new(r"(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})").ok()?;
    let caps = re.captures(line)?;

    let month = caps.get(1)?.as_str();
    let day: u32 = caps.get(2)?.as_str().trim().parse().ok()?;
    let hour: u32 = caps.get(3)?.as_str().parse().ok()?;
    let minute: u32 = caps.get(4)?.as_str().parse().ok()?;
    let second: u32 = caps.get(5)?.as_str().parse().ok()?;

    // Use current year (syslog typically doesn't include year)
    let year = Utc::now().year();

    let timestamp_str = format!("{} {} {} {:02}:{:02}:{:02}", year, month, day, hour, minute, second);

    if let Ok(naive) = NaiveDateTime::parse_from_str(&timestamp_str, "%Y %b %d %H:%M:%S") {
        return Some(DateTime::from_naive_utc_and_offset(naive, Utc));
    }

    None
}

/// Try to parse syslog with year: YYYY MMM DD HH:MM:SS
fn try_syslog_with_year(line: &str) -> Option<DateTime<Utc>> {
    use regex::Regex;

    let re = Regex::new(r"(\d{4})\s+(Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+(\d{1,2})\s+(\d{2}):(\d{2}):(\d{2})").ok()?;
    let caps = re.captures(line)?;

    let year = caps.get(1)?.as_str();
    let month = caps.get(2)?.as_str();
    let day = caps.get(3)?.as_str().trim();
    let hour = caps.get(4)?.as_str();
    let minute = caps.get(5)?.as_str();
    let second = caps.get(6)?.as_str();

    let timestamp_str = format!("{} {} {} {}:{}:{}", year, month, day, hour, minute, second);

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
        let line = "[INFO] 2024-01-15T10:30:45 Something happened";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_iso8601_with_millis() {
        let line = "2024-01-15T10:30:45.123 Something happened";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_iso8601_with_timezone() {
        let line = "2024-01-15T10:30:45Z Something happened";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_common_datetime() {
        let line = "[ERROR] 2024-01-15 10:30:45 Error occurred";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_syslog_format() {
        let line = "Jan 15 10:30:45 server kernel: message";
        let result = parse_timestamp(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_syslog_with_year() {
        let line = "2024 Jan 15 10:30:45 server kernel: message";
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
        let line1 = "2024-01-15T10:30:45 First event";
        let line2 = "2024-01-15T10:30:46 Second event";

        let dt1 = parse_timestamp(line1).unwrap();
        let dt2 = parse_timestamp(line2).unwrap();

        assert!(dt1 < dt2);
    }
}
