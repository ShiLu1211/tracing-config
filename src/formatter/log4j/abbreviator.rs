//! Log4j-style dot-notation abbreviation for logger / class names.
//!
//! Log4j's `%c{1.}` means "abbreviate all but the last `N` segments
//! to their first letter". The trailing dot is part of the syntax.
//! Examples:
//!
//! | Input                       | `{1.}`    | `{2.}`    | `{0.}` |
//! | --------------------------- | --------- | --------- | ------ |
//! | `com.example.foo.Bar`       | `c.e.f.Bar` | `c.e.Bar` | `Bar` |
//! | `Bar`                       | `Bar`     | `Bar`     | `Bar` |
//!
//! When the requested depth is `0` (or absent), only the last
//! segment is kept — matching log4j default.

use std::fmt;

/// Apply log4j's `{n.}` abbreviation to a dotted / `::`-separated
/// target string.
///
/// `depth` is the number of trailing segments to *keep* in full
/// form; anything before that is collapsed to its first letter.
/// `depth == 0` returns the last segment only (log4j default).
pub fn abbreviate(target: &str, depth: usize) -> String {
    // Log4j splits on `.`; we additionally accept `::` so Rust
    // module paths like `my::app::service` behave intuitively.
    // Pre-normalise `::` to `.` to avoid empty segments.
    let normalized = target.replace("::", ".");
    let segments: Vec<&str> = normalized.split('.').collect();
    if segments.is_empty() {
        return target.to_string();
    }
    if depth == 0 {
        return segments.last().copied().unwrap_or("").to_string();
    }

    let keep_from = segments.len().saturating_sub(depth);
    let mut out = String::new();
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            out.push('.');
        }
        if i < keep_from {
            // Abbreviate to first char.
            if let Some(c) = seg.chars().next() {
                out.push(c);
            } else {
                out.push_str(seg);
            }
        } else {
            out.push_str(seg);
        }
    }
    out
}

/// Zero-alloc variant of [`abbreviate`] that writes directly to a `fmt::Write` target.
pub fn abbreviate_to_writer(
    target: &str,
    depth: usize,
    writer: &mut dyn fmt::Write,
) -> fmt::Result {
    if target.is_empty() {
        return Ok(());
    }

    // Count segments using the dual delimiter iterator
    let segments: Vec<&str> = SegIter::new(target).collect();
    if segments.is_empty() {
        return writer.write_str(target);
    }
    if depth == 0 {
        return writer.write_str(segments.last().copied().unwrap_or(""));
    }

    let keep_from = segments.len().saturating_sub(depth);
    for (i, seg) in segments.iter().enumerate() {
        if i > 0 {
            writer.write_str(".")?;
        }
        if i < keep_from {
            if let Some(c) = seg.chars().next() {
                write!(writer, "{}", c)?;
            } else {
                writer.write_str(seg)?;
            }
        } else {
            writer.write_str(seg)?;
        }
    }
    Ok(())
}

/// Iterator that splits a string on both `.` and `::` delimiters,
/// yielding segments in order. This avoids the `replace("::", ".")`
/// allocation.
struct SegIter<'a> {
    remaining: &'a str,
}

impl<'a> SegIter<'a> {
    fn new(s: &'a str) -> Self {
        Self { remaining: s }
    }
}

impl<'a> Iterator for SegIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining.is_empty() {
            return None;
        }

        // Find the next delimiter: either "::" or "."
        // Look for "::" first (longer match), then "."
        let pos = if let Some(p) = self.remaining.find("::") {
            // Check if "." comes before "::"
            if let Some(dp) = self.remaining.find('.') {
                if dp < p {
                    dp
                } else {
                    p
                }
            } else {
                p
            }
        } else if let Some(p) = self.remaining.find('.') {
            p
        } else {
            // No more delimiters
            let seg = self.remaining;
            self.remaining = "";
            return Some(seg);
        };

        let seg = &self.remaining[..pos];
        // Skip the delimiter: "." or "::"
        if self.remaining[pos..].starts_with("::") {
            self.remaining = &self.remaining[pos + 2..];
        } else {
            self.remaining = &self.remaining[pos + 1..];
        }
        Some(seg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_zero_returns_last_segment() {
        assert_eq!(abbreviate("com.example.foo.Bar", 0), "Bar");
    }

    #[test]
    fn depth_one_abbreviates_all_but_last() {
        assert_eq!(abbreviate("com.example.foo.Bar", 1), "c.e.f.Bar");
    }

    #[test]
    fn depth_two_keeps_last_two_segments() {
        // "com.example.foo.Bar" with depth=2 keeps the last two
        // segments (foo.Bar) intact and abbreviates the leading
        // two (com.example) to their first letter.
        assert_eq!(abbreviate("com.example.foo.Bar", 2), "c.e.foo.Bar");
    }

    #[test]
    fn single_segment_passes_through() {
        assert_eq!(abbreviate("Bar", 1), "Bar");
        assert_eq!(abbreviate("Bar", 0), "Bar");
    }

    #[test]
    fn depth_larger_than_segments_keeps_everything() {
        assert_eq!(abbreviate("a.b.c", 10), "a.b.c");
    }

    #[test]
    fn rust_module_path_uses_dot_output() {
        assert_eq!(abbreviate("my::app::service", 1), "m.a.service");
    }

    #[test]
    fn empty_segment_is_preserved() {
        assert_eq!(abbreviate("a..b", 0), "b");
        assert_eq!(abbreviate("a..b", 1), "a..b");
    }
}
