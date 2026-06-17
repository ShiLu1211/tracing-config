//! Logback color tests - ANSI color output verification.

use tracing_declarative::formatter::logback::color::{level_color, with_color, Color, RESET};

#[test]
fn test_all_color_codes() {
    let colors = [
        Color::Black,
        Color::Red,
        Color::Green,
        Color::Yellow,
        Color::Blue,
        Color::Magenta,
        Color::Cyan,
        Color::White,
        Color::Faint,
        Color::BoldRed,
        Color::BoldGreen,
    ];
    for color in colors {
        let result = with_color(color, "test");
        assert!(
            result.starts_with(color.code()),
            "color {:?} code mismatch",
            color
        );
        assert!(result.ends_with(RESET), "color {:?} missing RESET", color);
    }
}

#[test]
fn test_level_color_mapping() {
    assert_eq!(level_color(&tracing::Level::ERROR), Color::BoldRed);
    assert_eq!(level_color(&tracing::Level::WARN), Color::Yellow);
    assert_eq!(level_color(&tracing::Level::INFO), Color::Blue);
    assert_eq!(level_color(&tracing::Level::DEBUG), Color::Green);
    assert_eq!(level_color(&tracing::Level::TRACE), Color::Faint);
}

#[test]
fn test_color_parse() {
    assert_eq!(Color::parse("red"), Some(Color::Red));
    assert_eq!(Color::parse("bold_green"), Some(Color::BoldGreen));
    assert_eq!(Color::parse("unknown_color"), None);
}

#[test]
fn test_ansi_escape_sequences() {
    assert_eq!(Color::Red.code(), "\x1b[31m");
    assert_eq!(Color::BoldGreen.code(), "\x1b[1;32m");
}
