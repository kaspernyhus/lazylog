use crate::log::LogLine;
use crate::resolver::VisibilityRule;
use chrono::{DateTime, Utc};
use std::collections::HashSet;

/// Time-based filter for showing only lines within a specific time range.
#[derive(Debug, Clone)]
pub struct TimeFilter {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeFilter {
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }
}

impl VisibilityRule for TimeFilter {
    fn is_visible(&self, line: &LogLine) -> bool {
        match &line.timestamp {
            Some(ts) => *ts >= self.start && *ts <= self.end,
            None => true,
        }
    }
}

/// Focus state for time filter input fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFilterFocus {
    Start,
    End,
}

impl TimeFilterFocus {
    pub fn next(&self) -> Self {
        match self {
            TimeFilterFocus::Start => TimeFilterFocus::End,
            TimeFilterFocus::End => TimeFilterFocus::Start,
        }
    }
}

/// Computes which log indices should have a time gap separator before them.
/// If `skip_date_rollovers` is true, skips gaps where the date changed (to avoid redundancy with date rollover line).
pub fn compute_gap_separator_indices(
    lines: &[LogLine],
    threshold_minutes: i64,
    skip_date_rollovers: bool,
) -> HashSet<usize> {
    let mut result = HashSet::new();
    let mut prev_ts: Option<DateTime<Utc>> = None;

    for line in lines {
        if line.timestamp_inherited {
            continue;
        }

        if let Some(current_ts) = line.timestamp {
            if let Some(prev) = prev_ts {
                let gap_minutes = (current_ts - prev).num_minutes().abs();
                if gap_minutes >= threshold_minutes {
                    let is_date_rollover = current_ts.date_naive() != prev.date_naive();
                    if !skip_date_rollovers || !is_date_rollover {
                        result.insert(line.index);
                    }
                }
            }
            prev_ts = Some(current_ts);
        }
    }

    result
}

/// Computes which log indices should have a date rollover separator before them.
pub fn compute_date_rollover_separator_indices(lines: &[LogLine]) -> HashSet<usize> {
    let mut result = HashSet::new();
    let mut prev_ts: Option<DateTime<Utc>> = None;

    for line in lines {
        if line.timestamp_inherited {
            continue;
        }

        if let Some(current_ts) = line.timestamp {
            if let Some(prev) = prev_ts
                && current_ts.date_naive() != prev.date_naive()
            {
                result.insert(line.index);
            }

            prev_ts = Some(current_ts);
        }
    }

    result
}
