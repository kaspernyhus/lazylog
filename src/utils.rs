/// Returns true if the haystack contains the needle, ignoring ASCII case.
///
/// Uses a sliding window approach for efficient matching.
pub fn contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }

    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_ignore_case_finds_different_cases() {
        assert!(contains_ignore_case("ERROR: foo", "error"));
        assert!(contains_ignore_case("error: foo", "ERROR"));
        assert!(contains_ignore_case("Error: foo", "eRrOr"));
    }

    #[test]
    fn test_contains_ignore_case_returns_false_for_no_match() {
        assert!(!contains_ignore_case("INFO: foo", "error"));
    }

    #[test]
    fn test_contains_ignore_case_handles_empty_needle() {
        assert!(contains_ignore_case("foo", ""));
    }

    #[test]
    fn test_contains_ignore_case_handles_needle_longer_than_haystack() {
        assert!(!contains_ignore_case("foo", "foobar"));
    }
}
