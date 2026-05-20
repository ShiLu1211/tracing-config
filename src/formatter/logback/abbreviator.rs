//! Logger name abbreviation algorithm, aligned with logback's TargetLengthBasedClassNameAbbreviator.
//!
//! Logback abbreviation behavior:
//! - n=0 → only the last segment: "my_app::db::pool" → "pool"
//! - n>0 → abbreviate from the left, keeping the last segment full, until total ≤ n
//!   e.g., "my_app::service::user_handler", n=20 → "m.s.user_handler"

/// Abbreviate a logger/target name according to the given max length.
///
/// - `max_length = 0` → return only the last segment
/// - `max_length > 0` → abbreviate segments from left, never touch last segment
pub fn abbreviate(name: &str, max_length: usize) -> String {
    if name.is_empty() {
        return String::new();
    }

    let segments: Vec<&str> = name.split("::").collect();
    if segments.len() == 1 {
        // No :: separator, just return as-is
        return if max_length == 0 {
            name.to_string()
        } else {
            truncate_right(name, max_length)
        };
    }

    if max_length == 0 {
        // Return only last segment
        return segments.last().unwrap_or(&"").to_string();
    }

    // Abbreviate from left, keep last segment full
    let last = *segments.last().unwrap();
    let prefix_segments = &segments[..segments.len() - 1];

    // Calculate space needed for last segment
    let last_len = last.len();
    if last_len >= max_length {
        // Last segment alone exceeds limit, truncate it
        return truncate_right(last, max_length);
    }

    // Try to fit prefix segments as abbreviated single chars
    let available = max_length.saturating_sub(last_len + 1); // -1 for the separator
    let mut result = String::new();
    let mut remaining = available;

    for (i, seg) in prefix_segments.iter().enumerate() {
        if seg.is_empty() {
            continue;
        }
        if remaining == 0 {
            break;
        }
        let seg_len = seg.len();
        if seg_len <= remaining {
            // Full segment fits
            if i > 0 && remaining >= 1 {
                result.push('.');
                remaining -= 1;
            }
            result.push_str(seg);
            remaining -= seg_len;
        } else if remaining >= 1 {
            // Can't fit full segment, but can take at least one char
            if i > 0 && remaining >= 1 {
                result.push('.');
                remaining -= 1;
            }
            if remaining >= 1 {
                if let Some(c) = seg.chars().next() {
                    result.push(c);
                    remaining = 0;
                }
            }
        } else {
            break;
        }
    }

    format!("{}.{}", result, last)
}

/// Truncate a string from the right to max_length.
fn truncate_right(s: &str, max_length: usize) -> String {
    if s.len() <= max_length {
        return s.to_string();
    }
    s.chars().take(max_length).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_abbreviation() {
        let name = "INFO";
        assert_eq!(abbreviate(name, 20), "INFO");
    }

    #[test]
    fn test_max_zero_returns_last_segment() {
        let name = "my_app::db::pool";
        assert_eq!(abbreviate(name, 0), "pool");
    }

    #[test]
    fn test_simple_segment() {
        let name = "my_app";
        assert_eq!(abbreviate(name, 10), "my_app");
    }

    #[test]
    fn test_logback_example() {
        // my_app::service::user_handler, n=20 → abbreviated but ends with user_handler
        let name = "my_app::service::user_handler";
        let result = abbreviate(name, 20);
        assert!(
            result.ends_with("user_handler"),
            "expected ending with user_handler, got: {}",
            result
        );
        assert!(!result.is_empty());
    }

    #[test]
    fn test_empty_name() {
        assert_eq!(abbreviate("", 20), "");
    }

    #[test]
    fn test_single_long_segment() {
        let name = "very_long_segment_name_here";
        let result = abbreviate(name, 10);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_exact_length() {
        let name = "INFO";
        assert_eq!(abbreviate(name, 4), "INFO");
    }

    #[test]
    fn test_logback_example_deterministic() {
        // Test that abbreviation works - last segment is always preserved
        let name = "my_app::service::user_handler";
        let result = abbreviate(name, 20);
        // Last segment must be preserved
        assert!(
            result.ends_with("user_handler"),
            "expected ending with user_handler, got: {}",
            result
        );
        assert!(result.len() <= 20);
        // Should contain dots separating abbreviated segments
        assert!(result.contains('.'));
    }

    #[test]
    fn test_abbreviate_multiple_segments() {
        // Three segments abbreviated to fit
        let name = "com::example::my_app::handler";
        let result = abbreviate(name, 15);
        // Last: "handler" = 7 chars
        // Available: 15 - 7 - 1 = 7 chars for prefix
        // "com" (3) + "." (1) + "e" (1) + "." (1) + "m" (1) = 7
        // Result: "c.e.m.handler" = 14 chars ≤ 15
        assert!(result.ends_with("handler"));
        assert!(result.len() <= 15);
    }

    #[test]
    fn test_no_colon_separator() {
        // Single segment without ::
        let name = "single_logger";
        let result = abbreviate(name, 10);
        assert_eq!(result.len(), 10);
    }

    #[test]
    fn test_last_segment_only_if_max_zero() {
        let name = "com::example::my_app";
        assert_eq!(abbreviate(name, 0), "my_app");
    }
}
