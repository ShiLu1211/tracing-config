//! Date/time format conversion from Java SimpleDateFormat to chrono strftime.
//!
//! Mapping table:
//! | Java  | chrono | description |
//! |-------|--------|-------------|
//! | yyyy  | %Y     | 4-digit year |
//! | MM    | %m     | month (01-12) |
//! | dd    | %d     | day (01-31) |
//! | HH    | %H     | hour (00-23) |
//! | mm    | %M     | minute (00-59) |
//! | ss    | %S     | second (00-59) |
//! | SSS   | %.3f   | millisecond (3 digits) |
//! | XXX   | %:z    | timezone offset (+08:00) |
//! | 'T'   | T      | literal T (in single quotes) |

use chrono::Local;

/// Convert Java SimpleDateFormat pattern to chrono strftime format.
pub fn convert_pattern(j_pattern: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = j_pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\'' {
            // Quoted literal - find end quote
            i += 1;
            while i < chars.len() && chars[i] != '\'' {
                result.push(chars[i]);
                i += 1;
            }
            if i < chars.len() {
                i += 1; // consume closing quote
            }
            continue;
        }

        let remaining = chars.len() - i;

        // Handle % as format specifier prefix
        if ch == '%' && i + 1 < chars.len() {
            let next = chars[i + 1];
            // Check if % followed by a valid pattern char
            match next {
                'Y' | 'y' => {
                    result.push_str("%Y");
                    i += 2;
                    continue;
                }
                'm' => {
                    result.push_str("%m");
                    i += 2;
                    continue;
                }
                'd' | 'D' => {
                    result.push_str("%d");
                    i += 2;
                    continue;
                }
                'H' => {
                    result.push_str("%H");
                    i += 2;
                    continue;
                }
                'M' => {
                    result.push_str("%M");
                    i += 2;
                    continue;
                }
                'S' | 's' => {
                    result.push_str("%S");
                    i += 2;
                    continue;
                }
                'f' => {
                    result.push_str("%.3f");
                    i += 2;
                    continue;
                }
                'z' | 'Z' => {
                    result.push_str("%:z");
                    i += 2;
                    continue;
                }
                'T' => {
                    result.push('T');
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        // Handle non-pattern characters (punctuation, spaces)
        // These are literals in date patterns
        if ch == '.' || ch == '-' || ch == ' ' || ch == ':' || ch == '/' || ch == '+' {
            // Check if this dot could be part of ".SSS" milliseconds pattern
            if ch == '.' && remaining >= 4 {
                let four: String = chars[i..i + 4].iter().collect();
                if four == ".SSS" {
                    result.push_str("%.3f");
                    i += 4;
                    continue;
                }
            }
            result.push(ch);
            i += 1;
            continue;
        }

        // Check for 4-char patterns first (longest first)
        if remaining >= 4 {
            let four: String = chars[i..i + 4].iter().collect();
            match four.as_str() {
                "yyyy" | "YYYY" => {
                    result.push_str("%Y");
                    i += 4;
                    continue;
                }
                "XXX" => {
                    result.push_str("%:z");
                    i += 4;
                    continue;
                }
                _ => {}
            }
        }

        // Check for 3-char patterns
        if remaining >= 3 {
            let three: String = chars[i..i + 3].iter().collect();
            if three == "SSS" {
                result.push_str("%.3f");
                i += 3;
                continue;
            }
        }

        // Check for 3-char XXX pattern (redundant check but handles remaining == 3)
        if remaining >= 3 {
            let three: String = chars[i..i + 3].iter().collect();
            if three == "XXX" {
                result.push_str("%:z");
                i += 3;
                continue;
            }
        }

        // Two-char patterns
        if remaining >= 2 {
            let two: String = chars[i..i + 2].iter().collect();
            match two.as_str() {
                "MM" => {
                    result.push_str("%m");
                    i += 2;
                    continue;
                }
                "dd" | "DD" => {
                    result.push_str("%d");
                    i += 2;
                    continue;
                }
                "HH" => {
                    result.push_str("%H");
                    i += 2;
                    continue;
                }
                "mm" => {
                    result.push_str("%M");
                    i += 2;
                    continue;
                }
                "ss" | "SS" => {
                    result.push_str("%S");
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }

        // Single char patterns
        match ch {
            'y' | 'Y' => result.push_str("%Y"),
            'M' => result.push_str("%m"),
            'd' | 'D' => result.push_str("%d"),
            'H' => result.push_str("%H"),
            'm' => result.push_str("%M"),
            's' | 'S' => result.push_str("%S"),
            'f' => result.push_str("%.3f"),
            'z' => result.push_str("%:z"),
            'Z' => result.push_str("%:z"),
            'T' => result.push('T'),
            _ => result.push(ch),
        }
        i += 1;
    }

    result
}

/// Format current time using the given Java SimpleDateFormat pattern.
pub fn format_time(j_pattern: &str) -> String {
    let chrono_pattern = convert_pattern(j_pattern);
    Local::now().format(&chrono_pattern).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_conversion() {
        assert_eq!(convert_pattern("yyyy-MM-dd"), "%Y-%m-%d");
    }

    #[test]
    fn test_time_conversion() {
        assert_eq!(convert_pattern("HH:mm:ss"), "%H:%M:%S");
    }

    #[test]
    fn test_milliseconds() {
        let result = convert_pattern("HH:mm:ss.SSS");
        assert_eq!(result, "%H:%M:%S%.3f", "got: {}", result);
    }

    #[test]
    fn test_timezone() {
        assert_eq!(convert_pattern("HH:mm:ssXXX"), "%H:%M:%S%:z");
    }

    #[test]
    fn test_full_logback() {
        let result = convert_pattern("yyyy-MM-dd'T'HH:mm:ss.SSSXXX");
        assert_eq!(result, "%Y-%m-%dT%H:%M:%S%.3f%:z", "got: {}", result);
    }

    #[test]
    fn test_literal_in_quotes() {
        assert_eq!(
            convert_pattern("yyyy-MM-dd'T'HH:mm:ss"),
            "%Y-%m-%dT%H:%M:%S"
        );
    }

    #[test]
    fn test_format_time() {
        let result = format_time("HH:mm:ss");
        // Should be something like "14:30:00"
        assert!(result.matches(':').count() == 2);
    }

    #[test]
    fn test_convert_empty_pattern() {
        assert_eq!(convert_pattern(""), "");
    }

    #[test]
    fn test_convert_single_char() {
        assert_eq!(convert_pattern("y"), "%Y");
        assert_eq!(convert_pattern("M"), "%m");
        assert_eq!(convert_pattern("d"), "%d");
        assert_eq!(convert_pattern("H"), "%H");
        assert_eq!(convert_pattern("m"), "%M");
        assert_eq!(convert_pattern("s"), "%S");
    }

    #[test]
    fn test_convert_lowercase() {
        // Lowercase should also work
        assert_eq!(convert_pattern("yyyy"), "%Y");
        assert_eq!(convert_pattern("ss"), "%S");
    }

    #[test]
    fn test_date_with_percent_prefix() {
        // Patterns that already have % prefix should be handled
        let result = convert_pattern("%Y-%m-%d");
        assert_eq!(result, "%Y-%m-%d");
    }

    #[test]
    fn test_date_with_escaped_text() {
        // Single quotes for literal text
        assert_eq!(convert_pattern("yyyy'年'MM'月'dd'日'"), "%Y年%m月%d日");
    }

    #[test]
    fn test_date_format_with_dots() {
        // Dots are preserved as literals
        assert_eq!(convert_pattern("yyyy.MM.dd"), "%Y.%m.%d");
    }
}
