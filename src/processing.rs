use crate::filter::ActiveFilterMode;
use crate::filter::FilterPattern;
use crate::utils::contains_ignore_case;

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
            ActiveFilterMode::Exclude => {
                if matches {
                    return false;
                }
            }
            ActiveFilterMode::Include => {
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
