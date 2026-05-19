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
}
