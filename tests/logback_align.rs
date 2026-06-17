//! Logback align tests - boundary cases for format modifier.

use tracing_declarative::formatter::logback::align::FormatModifier;

#[test]
fn test_min_equals_max() {
    let m = FormatModifier {
        left_align: false,
        min_width: Some(5),
        max_width: Some(5),
        max_from_end: false,
    };
    // Text exactly 5 chars should pass through (4 chars + 1 leading space for right align)
    assert_eq!(m.apply("INFO"), " INFO");
    // Text 7 chars should truncate to 5
    assert_eq!(m.apply("WARNING"), "WARNI");
}

#[test]
fn test_min_greater_than_text() {
    let m = FormatModifier {
        left_align: true,
        min_width: Some(20),
        max_width: None,
        max_from_end: false,
    };
    let result = m.apply("INFO");
    // Verify the result is 20 chars and is left-aligned
    assert_eq!(result.len(), 20);
    assert!(result.starts_with("INFO"));
    assert!(result.ends_with("               ")); // 15 spaces
}

#[test]
fn test_max_greater_than_text() {
    let m = FormatModifier {
        left_align: false,
        min_width: None,
        max_width: Some(100),
        max_from_end: false,
    };
    // 4 char text, no truncation
    assert_eq!(m.apply("INFO"), "INFO");
}

#[test]
fn test_left_vs_right_align_same_input() {
    let left = FormatModifier {
        left_align: true,
        min_width: Some(10),
        ..Default::default()
    };
    let right = FormatModifier {
        left_align: false,
        min_width: Some(10),
        ..Default::default()
    };
    let input = "X";
    assert_eq!(left.apply(input), "X         ");
    assert_eq!(right.apply(input), "         X");
}
