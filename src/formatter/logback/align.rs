//! Format modifier for alignment and truncation.
//! Strictly aligned with logback behavior: %-10.20, %10.20, %-10, etc.

/// Format modifier parsed from conversion word.
/// e.g., %-10.20logger{36} → modifier = FormatModifier { left_align: true, min_width: 10, max_width: Some(20) }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FormatModifier {
    /// True if `-` flag present (left align), false for right align (default)
    pub left_align: bool,
    /// Minimum width - pad with spaces if output shorter
    pub min_width: Option<usize>,
    /// Maximum width - truncate if output longer
    pub max_width: Option<usize>,
    /// If true, truncate from end; if false, truncate from start (default)
    pub max_from_end: bool,
}

impl FormatModifier {
    /// Apply this modifier to a string: truncate then pad.
    pub fn apply(&self, s: &str) -> String {
        let s = if let Some(max) = self.max_width {
            if s.len() > max {
                if self.max_from_end {
                    &s[s.len() - max..]
                } else {
                    &s[..max]
                }
            } else {
                s
            }
        } else {
            s
        };

        if let Some(min) = self.min_width {
            if s.len() < min {
                let padding = min - s.len();
                let pad = " ".repeat(padding);
                if self.left_align {
                    format!("{}{}", s, pad)
                } else {
                    format!("{}{}", pad, s)
                }
            } else {
                s.to_string()
            }
        } else {
            s.to_string()
        }
    }

    /// Apply this modifier by writing directly to a `fmt::Write` target.
    /// Avoids allocating a new `String` when the output already fits
    /// within min/max width constraints.
    pub fn apply_to_writer(&self, s: &str, writer: &mut dyn std::fmt::Write) -> std::fmt::Result {
        let truncated = if let Some(max) = self.max_width {
            if s.len() > max {
                if self.max_from_end {
                    &s[s.len() - max..]
                } else {
                    &s[..max]
                }
            } else {
                s
            }
        } else {
            s
        };

        let padding = self
            .min_width
            .map(|min| min.saturating_sub(truncated.len()))
            .unwrap_or(0);

        if self.left_align {
            writer.write_str(truncated)?;
            for _ in 0..padding {
                writer.write_char(' ')?;
            }
        } else {
            for _ in 0..padding {
                writer.write_char(' ')?;
            }
            writer.write_str(truncated)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_left_align() {
        let m = FormatModifier {
            left_align: true,
            min_width: Some(10),
            max_width: None,
            max_from_end: false,
        };
        assert_eq!(m.apply("INFO"), "INFO      ");
    }

    #[test]
    fn test_right_align() {
        let m = FormatModifier {
            left_align: false,
            min_width: Some(10),
            max_width: None,
            max_from_end: false,
        };
        assert_eq!(m.apply("INFO"), "      INFO");
    }

    #[test]
    fn test_truncate() {
        let m = FormatModifier {
            left_align: false,
            min_width: None,
            max_width: Some(5),
            max_from_end: false,
        };
        assert_eq!(m.apply("INFO"), "INFO");
        assert_eq!(m.apply("WARNING"), "WARNI"); // Truncate first 5 chars
    }

    #[test]
    fn test_min_and_max() {
        let m = FormatModifier {
            left_align: true,
            min_width: Some(10),
            max_width: Some(5),
            max_from_end: false,
        };
        // Apply max first (truncate to 5), then min (pad to 10)
        assert_eq!(m.apply("WARNING"), "WARNI     "); // 5 chars + 5 spaces = 10
        assert_eq!(m.apply("INFO"), "INFO      "); // 4 chars + 6 spaces = 10
    }

    #[test]
    fn test_truncate_from_end() {
        let m = FormatModifier {
            left_align: false,
            min_width: None,
            max_width: Some(5),
            max_from_end: true, // truncate from end
        };
        assert_eq!(m.apply("INFO"), "INFO");
        // "WARNING" has 7 chars, last 5 = "RNING"
        assert_eq!(m.apply("WARNING"), "RNING");
    }

    #[test]
    fn test_max_equals_min() {
        let m = FormatModifier {
            left_align: false,
            min_width: Some(5),
            max_width: Some(5),
            max_from_end: false,
        };
        assert_eq!(m.apply("HELLO"), "HELLO");
        assert_eq!(m.apply("HI"), "   HI");
    }

    #[test]
    fn test_no_modifier() {
        let m = FormatModifier::default();
        assert_eq!(m.apply("UNCHANGED"), "UNCHANGED");
    }

    #[test]
    fn test_empty_string_with_min_width() {
        let m = FormatModifier {
            left_align: true,
            min_width: Some(5),
            max_width: None,
            max_from_end: false,
        };
        assert_eq!(m.apply(""), "     "); // 5 spaces
    }
}
