//! ANSI color support for logback-style color conversion words.
//!
//! Supported color words:
//! - %highlight(sub) → auto-color based on log level (ERROR=red bold, WARN=yellow, INFO=blue, DEBUG=green, TRACE=default)
//! - %clr(sub){color} → fixed color
//! - %red, %green, %yellow, %blue, %magenta, %cyan, %white, %faint
//! - %boldRed, %boldGreen, etc.

/// ANSI color codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Faint,
    BoldRed,
    BoldGreen,
    BoldYellow,
    BoldBlue,
    BoldMagenta,
    BoldCyan,
    BoldWhite,
}

impl Color {
    /// Returns the ANSI escape sequence for this color.
    pub fn code(&self) -> &'static str {
        match self {
            Color::Black => "\x1b[30m",
            Color::Red => "\x1b[31m",
            Color::Green => "\x1b[32m",
            Color::Yellow => "\x1b[33m",
            Color::Blue => "\x1b[34m",
            Color::Magenta => "\x1b[35m",
            Color::Cyan => "\x1b[36m",
            Color::White => "\x1b[37m",
            Color::Faint => "\x1b[2m",
            Color::BoldRed => "\x1b[1;31m",
            Color::BoldGreen => "\x1b[1;32m",
            Color::BoldYellow => "\x1b[1;33m",
            Color::BoldBlue => "\x1b[1;34m",
            Color::BoldMagenta => "\x1b[1;35m",
            Color::BoldCyan => "\x1b[1;36m",
            Color::BoldWhite => "\x1b[1;37m",
        }
    }

    /// Parse a color name from string (for %clr{sub}{color} syntax).
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "black" => Some(Color::Black),
            "red" => Some(Color::Red),
            "green" => Some(Color::Green),
            "yellow" => Some(Color::Yellow),
            "blue" => Some(Color::Blue),
            "magenta" => Some(Color::Magenta),
            "cyan" => Some(Color::Cyan),
            "white" => Some(Color::White),
            "faint" => Some(Color::Faint),
            "boldred" | "bold_red" => Some(Color::BoldRed),
            "boldgreen" | "bold_green" => Some(Color::BoldGreen),
            "boldyellow" | "bold_yellow" => Some(Color::BoldYellow),
            "boldblue" | "bold_blue" => Some(Color::BoldBlue),
            "boldmagenta" | "bold_magenta" => Some(Color::BoldMagenta),
            "boldcyan" | "bold_cyan" => Some(Color::BoldCyan),
            "boldwhite" | "bold_white" => Some(Color::BoldWhite),
            _ => None,
        }
    }
}

/// ANSI reset escape sequence.
pub const RESET: &str = "\x1b[0m";

/// Map a log level to its default highlight color.
pub fn level_color(level: &tracing::Level) -> Color {
    match *level {
        tracing::Level::ERROR => Color::BoldRed,
        tracing::Level::WARN => Color::Yellow,
        tracing::Level::INFO => Color::Blue,
        tracing::Level::DEBUG => Color::Green,
        tracing::Level::TRACE => Color::Faint,
    }
}

/// Wrap a string with ANSI color codes.
pub fn with_color(color: Color, s: &str) -> String {
    format!("{}{}{}", color.code(), s, RESET)
}

/// Write ANSI color codes + content directly to a `fmt::Write` target.
pub fn with_color_to_writer(
    color: Color,
    s: &str,
    writer: &mut dyn std::fmt::Write,
) -> std::fmt::Result {
    writer.write_str(color.code())?;
    writer.write_str(s)?;
    writer.write_str(RESET)
}

/// Wrap a string with level-based highlighting.
pub fn highlight(level: tracing::Level, s: &str) -> String {
    with_color(level_color(&level), s)
}

/// Write level-based highlighting directly to a `fmt::Write` target.
pub fn highlight_to_writer(
    level: tracing::Level,
    s: &str,
    writer: &mut dyn std::fmt::Write,
) -> std::fmt::Result {
    with_color_to_writer(level_color(&level), s, writer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_codes() {
        assert_eq!(Color::Red.code(), "\x1b[31m");
        assert_eq!(Color::BoldGreen.code(), "\x1b[1;32m");
    }

    #[test]
    fn test_level_color() {
        assert_eq!(level_color(&tracing::Level::ERROR), Color::BoldRed);
        assert_eq!(level_color(&tracing::Level::WARN), Color::Yellow);
        assert_eq!(level_color(&tracing::Level::INFO), Color::Blue);
    }

    #[test]
    fn test_with_color() {
        let colored = with_color(Color::Red, "test");
        assert!(colored.starts_with("\x1b[31m"));
        assert!(colored.ends_with("\x1b[0m"));
    }

    #[test]
    fn test_color_from_str() {
        assert_eq!(Color::parse("red"), Some(Color::Red));
        assert_eq!(Color::parse("bold_green"), Some(Color::BoldGreen));
        assert_eq!(Color::parse("unknown"), None);
    }
}
