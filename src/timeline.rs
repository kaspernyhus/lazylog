use crate::log::LogBuffer;
use crate::log_event::LogEventTracker;
use crate::timestamp::parse_timestamp;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TimeSlot {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    /// Counts keyed by row name (group name or event name)
    pub event_counts: HashMap<String, usize>,
}

/// Represents a row in the timeline (either a group or a single event)
#[derive(Debug, Clone)]
pub struct TimelineRow {
    /// Display name (group name or event name)
    pub name: String,
    /// Whether this is a group (true) or individual event (false)
    pub is_group: bool,
    /// Event names that belong to this row
    pub event_names: Vec<String>,
}

#[derive(Debug)]
pub struct TimelineData {
    pub slots: Vec<TimeSlot>,
    /// Timeline rows (groups or individual events)
    pub rows: Vec<TimelineRow>,
    /// For backwards compatibility
    pub event_names: Vec<String>,
    pub max_count: usize,
    pub time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
}

impl TimelineData {
    pub fn compute(log_buffer: &LogBuffer, event_tracker: &LogEventTracker, slot_count: usize) -> Option<Self> {
        if slot_count == 0 {
            return None;
        }

        let mut line_timestamps: HashMap<usize, DateTime<Utc>> = HashMap::new();
        for log_line in log_buffer.iter() {
            if let Some(ts) = log_line.timestamp.or_else(|| parse_timestamp(&log_line.content)) {
                line_timestamps.insert(log_line.index, ts);
            }
        }

        if line_timestamps.is_empty() {
            return None;
        }

        let min_time = *line_timestamps.values().min()?;
        let max_time = *line_timestamps.values().max()?;

        if min_time == max_time {
            return None;
        }

        let time_range_nanos = (max_time - min_time).num_nanoseconds()?;
        let slot_duration_nanos = time_range_nanos / slot_count as i64;

        if slot_duration_nanos == 0 {
            return None;
        }

        let events = event_tracker.get_events();

        // Build timeline rows from groups and ungrouped events
        let timeline_row_data = event_tracker.get_timeline_rows();
        if timeline_row_data.is_empty() {
            return None;
        }

        let rows: Vec<TimelineRow> = timeline_row_data
            .into_iter()
            .map(|(name, is_group, event_names)| TimelineRow {
                name,
                is_group,
                event_names,
            })
            .collect();

        // Build a lookup from event name to row name
        let mut event_to_row: HashMap<String, String> = HashMap::new();
        for row in &rows {
            for event_name in &row.event_names {
                event_to_row.insert(event_name.clone(), row.name.clone());
            }
        }

        // For backwards compatibility
        let event_names: Vec<String> = rows.iter().map(|r| r.name.clone()).collect();

        let mut slots: Vec<TimeSlot> = (0..slot_count)
            .map(|i| {
                let start_nanos = i as i64 * slot_duration_nanos;
                let end_nanos = if i == slot_count - 1 {
                    time_range_nanos
                } else {
                    (i as i64 + 1) * slot_duration_nanos
                };
                let start_time = min_time + chrono::Duration::nanoseconds(start_nanos);
                let end_time = min_time + chrono::Duration::nanoseconds(end_nanos);
                TimeSlot {
                    start_time,
                    end_time,
                    event_counts: HashMap::new(),
                }
            })
            .collect();

        let mut max_count = 0usize;

        for event in events {
            if let Some(&ts) = line_timestamps.get(&event.line_index) {
                let offset_nanos = (ts - min_time).num_nanoseconds().unwrap_or(0);
                let slot_idx = ((offset_nanos / slot_duration_nanos) as usize).min(slot_count - 1);

                // Get the row name for this event (could be a group name or the event name itself)
                let row_name = event_to_row.get(&event.name).cloned().unwrap_or_else(|| event.name.clone());

                let count = slots[slot_idx].event_counts.entry(row_name).or_insert(0);
                *count += 1;
                max_count = max_count.max(*count);
            }
        }

        Some(TimelineData {
            slots,
            rows,
            event_names,
            max_count,
            time_range: Some((min_time, max_time)),
        })
    }

    pub fn intensity_char(count: usize, max_count: usize) -> char {
        if count == 0 || max_count == 0 {
            return '.';
        }
        let ratio = count as f64 / max_count as f64;
        if ratio <= 0.25 {
            '░'
        } else if ratio <= 0.50 {
            '▒'
        } else if ratio <= 0.75 {
            '▓'
        } else {
            '█'
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intensity_char_empty() {
        assert_eq!(TimelineData::intensity_char(0, 10), '.');
        assert_eq!(TimelineData::intensity_char(0, 0), '.');
        assert_eq!(TimelineData::intensity_char(5, 0), '.');
    }

    #[test]
    fn test_intensity_char_levels() {
        assert_eq!(TimelineData::intensity_char(1, 100), '░');
        assert_eq!(TimelineData::intensity_char(25, 100), '░');
        assert_eq!(TimelineData::intensity_char(26, 100), '▒');
        assert_eq!(TimelineData::intensity_char(50, 100), '▒');
        assert_eq!(TimelineData::intensity_char(51, 100), '▓');
        assert_eq!(TimelineData::intensity_char(75, 100), '▓');
        assert_eq!(TimelineData::intensity_char(76, 100), '█');
        assert_eq!(TimelineData::intensity_char(100, 100), '█');
    }
}
