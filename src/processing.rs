use crate::filter::FilterMode;
use crate::filter::FilterPattern;
use crate::log_event::LogEvent;
use crate::utils::contains_ignore_case;
use crate::{highlighter::HighlightPattern, log::LogLine};
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

/// Counts event occurrences.
pub fn count_events(
    lines: &[LogLine],
    event_patterns: &[HighlightPattern],
) -> HashMap<String, usize> {
    let event_patterns = Arc::new(event_patterns.to_vec());
    let counts = Arc::new(Mutex::new(HashMap::new()));

    lines.par_iter().for_each(|log_line| {
        for event in event_patterns.iter() {
            if event.matcher.matches(log_line.content()) {
                if let Some(name) = &event.name {
                    let mut counts_guard = counts.lock().unwrap();
                    *counts_guard.entry(name.clone()).or_insert(0) += 1;
                }
                break;
            }
        }
    });

    Arc::try_unwrap(counts).unwrap().into_inner().unwrap()
}

/// Scans log lines in parallel for event pattern matches.
pub fn scan_for_events(
    lines: &[LogLine],
    event_patterns: &[HighlightPattern],
    active_filters: &HashMap<String, bool>,
) -> Vec<LogEvent> {
    let event_patterns = Arc::new(event_patterns.to_vec());
    let active_filters = Arc::new(active_filters.clone());

    let mut events: Vec<LogEvent> = lines
        .par_iter()
        .filter_map(|log_line| {
            for event in event_patterns.iter() {
                if event.matcher.matches(log_line.content()) {
                    if let Some(name) = &event.name
                        && *active_filters.get(name).unwrap_or(&true)
                    {
                        return Some(LogEvent {
                            event_name: name.clone(),
                            line_index: log_line.index,
                        });
                    }
                    break;
                }
            }
            None
        })
        .collect();

    // Sort by line_index to maintain chronological order
    events.sort_by_key(|e| e.line_index);
    events
}

/// Checks if content passes the given filter patterns.
pub fn apply_filters(content: &str, filter_patterns: &[FilterPattern]) -> bool {
    if filter_patterns.is_empty() {
        return true;
    }

    let mut has_include_filters = false;
    let mut include_matched = false;

    for filter in filter_patterns.iter().filter(|f| f.enabled) {
        let matches = if filter.case_sensitive {
            content.contains(&filter.pattern)
        } else {
            contains_ignore_case(content, &filter.pattern)
        };

        match filter.mode {
            FilterMode::Exclude => {
                if matches {
                    return false;
                }
            }
            FilterMode::Include => {
                has_include_filters = true;
                if matches {
                    include_matched = true;
                }
            }
        }
    }

    if has_include_filters {
        include_matched
    } else {
        true
    }
}
