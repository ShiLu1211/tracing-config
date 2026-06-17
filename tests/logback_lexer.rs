//! Logback lexer tests - boundary cases for token parsing.

use tracing_declarative::formatter::logback::{scan, Keyword, Token};

#[test]
fn test_empty_pattern() {
    let tokens = scan("").unwrap();
    assert!(tokens.is_empty());
}

#[test]
fn test_plain_text_only() {
    let tokens = scan("hello world").unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(matches!(&tokens[0], Token::Literal(s) if s == "hello world"));
}

#[test]
fn test_all_conversion_words() {
    let pattern = "%d{%Y-%m-%d} %level %thread %logger %msg";
    let tokens = scan(pattern).unwrap();
    // Verify we got the conversion words (exact count varies by space handling)
    assert!(tokens.len() >= 5);
    // Verify first is Date with option
    assert!(matches!(
        &tokens[0],
        Token::Conversion {
            keyword: Keyword::Date,
            ..
        }
    ));
    // Verify last is Message
    assert!(matches!(
        &tokens[tokens.len() - 1],
        Token::Conversion {
            keyword: Keyword::Message,
            ..
        }
    ));
}

#[test]
fn test_illegal_pattern_char() {
    let result = scan("%Z");
    assert!(result.is_err());
}
