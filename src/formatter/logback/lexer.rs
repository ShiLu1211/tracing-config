//! Logback pattern lexer - parses conversion words into tokens.

use crate::error::ConfigError;

use super::align::FormatModifier;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Literal(String),
    Conversion {
        modifier: Option<FormatModifier>,
        keyword: Keyword,
        option: Option<String>,
        /// Sub-pattern for composite converters like %highlight(%level)
        /// Each child token may itself be a Conversion with its own sub-patterns
        sub_pattern: Option<Vec<Token>>,
    },
    Newline, // %n
    Percent, // %%
}

#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    // Time
    Date,
    Relative,

    // Level
    Level,

    // Thread
    Thread,

    // Logger / target
    Logger,
    Class,

    // Message
    Message,

    // Call site
    Method,
    Line,
    File,

    // MDC / span fields
    Mdc,

    // Event fields
    Kvp,

    // Marker
    Marker,

    // Exception
    Exception,
    RootException,
    NopException,

    // Process
    Pid,

    // Color (with sub-pattern)
    Highlight,
    Clr,
    ColorWord(super::color::Color),

    // Escape
    Percent,
}

/// Scan a logback pattern string into tokens.
pub fn scan(pattern: &str) -> Result<Vec<Token>, ConfigError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = pattern.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '%' {
            // Collect literal text
            let start = i;
            while i < chars.len() && chars[i] != '%' {
                i += 1;
            }
            let literal: String = chars[start..i].iter().collect();
            if !literal.is_empty() {
                tokens.push(Token::Literal(literal));
            }
            continue;
        }

        i += 1; // skip %
        if i >= chars.len() {
            // Trailing %, emit as literal
            tokens.push(Token::Literal("%".to_string()));
            break;
        }

        // Parse modifier like %-10.20
        let (align, align_len) = parse_modifier(&chars, i);
        i += align_len;

        if i >= chars.len() {
            tokens.push(Token::Literal("%".to_string()));
            break;
        }

        let spec_char = chars[i];
        i += 1;

        if spec_char == '%' {
            tokens.push(Token::Percent);
            continue;
        }

        if spec_char == 'n' {
            tokens.push(Token::Newline);
            continue;
        }

        // Parse keyword - returns (keyword, option, keyword_chars_consumed)
        let (keyword, option, kw_len, is_composite) = parse_keyword(spec_char, &chars, i)?;
        i += kw_len;

        // Check for composite pattern (sub-pattern in parentheses)
        // e.g., %highlight(%level), %red(%msg)
        // The '(' may have been counted in kw_len, so check the char before current position
        let sub_pattern = if is_composite && i > 0 && chars[i - 1] == '(' {
            // Find matching closing paren
            let start = i;
            let mut depth = 1;
            let mut end = start;
            while end < chars.len() && depth > 0 {
                if chars[end] == '(' {
                    depth += 1;
                } else if chars[end] == ')' {
                    depth -= 1;
                }
                end += 1;
            }
            if depth == 0 && end > start {
                // Recursively parse sub-pattern
                let sub_str: String = chars[start..end - 1].iter().collect();
                let sub_tokens = scan(&sub_str).ok();
                i = end; // Position after closing paren
                sub_tokens
            } else {
                None
            }
        } else {
            None
        };

        // If option was present in braces, it was already counted in kw_len for {..}
        // but for simple keywords we need to handle options separately

        tokens.push(Token::Conversion {
            modifier: align,
            keyword,
            option,
            sub_pattern,
        });
    }

    Ok(tokens)
}

/// Parse format modifier like %-10.20 or %5
/// Returns (FormatModifier, characters consumed)
fn parse_modifier(chars: &[char], start: usize) -> (Option<FormatModifier>, usize) {
    let mut pos = start;
    let mut left_align = false;

    if pos < chars.len() && chars[pos] == '-' {
        left_align = true;
        pos += 1;
    }

    // Check if next char is a digit
    if pos >= chars.len() || !chars[pos].is_ascii_digit() {
        // No modifier
        if left_align {
            // We consumed '-' but no digits followed, that's just literal text
            // Return no modifier and we'll re-parse
        }
        return (None, 0);
    }

    // Parse min width
    let mut num_str = String::new();
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        num_str.push(chars[pos]);
        pos += 1;
    }
    let min_width: usize = num_str.parse().unwrap_or(0);

    // Check for .max_width
    let mut max_width = None;
    if pos < chars.len() && chars[pos] == '.' {
        pos += 1;
        let mut max_str = String::new();
        while pos < chars.len() && chars[pos].is_ascii_digit() {
            max_str.push(chars[pos]);
            pos += 1;
        }
        if !max_str.is_empty() {
            max_width = max_str.parse().ok();
        }
    }

    let chars_consumed = pos - start;
    (
        Some(FormatModifier {
            left_align,
            min_width: Some(min_width),
            max_width,
            max_from_end: false,
        }),
        chars_consumed,
    )
}

/// Parse a keyword from the character stream.
/// Returns (Keyword, Option<String>, keyword_chars_consumed)
/// The chars_consumed accounts for the keyword letters (e.g., "level" = 4, "thread" = 5)
/// but NOT the option in braces (handled separately via position check).
/// Parse a keyword from the character stream.
/// Returns (Keyword, Option<String>, keyword_chars_consumed, is_composite)
/// is_composite = true for keywords that can have sub-patterns (highlight, clr, color words)
fn parse_keyword(
    first_char: char,
    chars: &[char],
    pos: usize,
) -> Result<(Keyword, Option<String>, usize, bool), ConfigError> {
    match first_char {
        'd' => {
            // %d{...} or %d
            if pos < chars.len() && chars[pos] == '{' {
                let end = chars[pos..].iter().position(|&c| c == '}');
                if let Some(end) = end {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Date, Some(opt), end + 2, false));
                }
            }
            Ok((Keyword::Date, None, 0, false))
        }
        'D' => Ok((Keyword::Date, None, 0, false)),
        'r' => Ok((Keyword::Relative, None, 0, false)),
        'l' => {
            // %logger or %level - distinguish by what follows
            if pos + 6 <= chars.len()
                && chars[pos] == 'o'
                && chars[pos + 1] == 'g'
                && chars[pos + 2] == 'g'
                && chars[pos + 3] == 'e'
                && chars[pos + 4] == 'r'
            {
                // %logger - check for {len} option after
                if pos + 5 < chars.len() && chars[pos + 5] == '{' {
                    let end = chars[pos + 6..].iter().position(|&c| c == '}');
                    if let Some(end) = end {
                        let opt: String = chars[pos + 6..pos + 6 + end].iter().collect();
                        return Ok((Keyword::Logger, Some(opt), end + 7, false));
                    }
                }
                Ok((Keyword::Logger, None, 5, false))
            } else if pos + 4 <= chars.len()
                && chars[pos] == 'e'
                && chars[pos + 1] == 'v'
                && chars[pos + 2] == 'e'
                && chars[pos + 3] == 'l'
            {
                Ok((Keyword::Level, None, 4, false))
            } else {
                Ok((Keyword::Level, None, 0, false))
            }
        }
        'L' => Ok((Keyword::Line, None, 0, false)),
        'f' => Ok((Keyword::File, None, 0, false)),
        'F' => Ok((Keyword::File, None, 0, false)),
        'M' => Ok((Keyword::Method, None, 0, false)),
        'p' => Ok((Keyword::Level, None, 0, false)),
        't' => {
            // %thread - check if followed by "hread" (5 more chars)
            if pos + 5 <= chars.len()
                && chars[pos] == 'h'
                && chars[pos + 1] == 'r'
                && chars[pos + 2] == 'e'
                && chars[pos + 3] == 'a'
                && chars[pos + 4] == 'd'
            {
                Ok((Keyword::Thread, None, 5, false))
            } else {
                Ok((Keyword::Thread, None, 0, false))
            }
        }
        'T' => Ok((Keyword::Thread, None, 0, false)),
        'm' => {
            // %msg or %m
            if pos < chars.len()
                && chars[pos] == 's'
                && pos + 1 < chars.len()
                && chars[pos + 1] == 'g'
            {
                Ok((Keyword::Message, None, 2, false))
            } else {
                Ok((Keyword::Message, None, 0, false))
            }
        }
        'c' => {
            // %c{...} = logger
            if pos < chars.len() && chars[pos] == '{' {
                let end = chars[pos..].iter().position(|&c| c == '}');
                if let Some(end) = end {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Logger, Some(opt), end + 2, false));
                }
            }
            Ok((Keyword::Logger, None, 0, false))
        }
        'C' => Ok((Keyword::Class, None, 0, false)),
        'X' => {
            // %X{...} or %X
            if pos < chars.len() && chars[pos] == '{' {
                let end = chars[pos..].iter().position(|&c| c == '}');
                if let Some(end) = end {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Mdc, Some(opt), end + 2, false));
                }
            }
            Ok((Keyword::Mdc, None, 0, false))
        }
        'k' => {
            // %kvp
            if pos + 1 < chars.len() && chars[pos] == 'v' && chars[pos + 1] == 'p' {
                Ok((Keyword::Kvp, None, 2, false))
            } else {
                Ok((Keyword::Kvp, None, 0, false))
            }
        }
        'e' => {
            // %ex or %exception
            if pos < chars.len() && chars[pos] == 'x' {
                if pos + 1 < chars.len() && chars[pos + 1] == '{' {
                    let end = chars[pos + 2..].iter().position(|&c| c == '}');
                    if let Some(end) = end {
                        let opt: String = chars[pos + 2..pos + 2 + end].iter().collect();
                        return Ok((Keyword::Exception, Some(opt), end + 3, false));
                    }
                }
                Ok((Keyword::Exception, None, 1, false))
            } else {
                Ok((Keyword::Exception, None, 0, false))
            }
        }
        'P' => {
            // %pid
            if pos + 1 < chars.len() && chars[pos] == 'i' && chars[pos + 1] == 'd' {
                Ok((Keyword::Pid, None, 2, false))
            } else {
                Ok((Keyword::Pid, None, 0, false))
            }
        }
        'h' => {
            // %highlight - check if followed by "highlight(" (9 chars + paren)
            if pos + 9 <= chars.len()
                && chars[pos] == 'i'
                && chars[pos + 1] == 'g'
                && chars[pos + 2] == 'h'
                && chars[pos + 3] == 'l'
                && chars[pos + 4] == 'i'
                && chars[pos + 5] == 'g'
                && chars[pos + 6] == 'h'
                && chars[pos + 7] == 't'
                && chars[pos + 8] == '('
            {
                // %highlight(...) - sub-pattern in parens, composite keyword
                Ok((Keyword::Highlight, None, 9, true))
            } else {
                Err(ConfigError::PatternParse {
                    message: "unknown placeholder '%h'".to_string(),
                    position: pos,
                })
            }
        }
        's' => Ok((Keyword::Message, None, 0, false)),
        _ => Err(ConfigError::PatternParse {
            message: format!("unknown placeholder '%{}'", first_char),
            position: pos,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal() {
        let tokens = scan("hello world").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Literal("hello world".to_string()));
    }

    #[test]
    fn test_level() {
        let tokens = scan("%level").unwrap();
        eprintln!("DEBUG tokens for '%level': {:?}", tokens);
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            tokens[0],
            Token::Conversion {
                keyword: Keyword::Level,
                ..
            }
        ));
    }

    #[test]
    fn test_date() {
        let tokens = scan("%d{HH:mm:ss}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            tokens[0],
            Token::Conversion {
                keyword: Keyword::Date,
                ..
            }
        ));
        if let Token::Conversion { option, .. } = &tokens[0] {
            assert_eq!(option.as_deref(), Some("HH:mm:ss"));
        }
    }

    #[test]
    fn test_logger() {
        let tokens = scan("%c{36}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            tokens[0],
            Token::Conversion {
                keyword: Keyword::Logger,
                ..
            }
        ));
        if let Token::Conversion { option, .. } = &tokens[0] {
            assert_eq!(option.as_deref(), Some("36"));
        }
    }

    #[test]
    fn test_modifier() {
        let tokens = scan("%-5level").unwrap();
        assert_eq!(tokens.len(), 1);
        if let Token::Conversion {
            modifier,
            keyword: Keyword::Level,
            ..
        } = &tokens[0]
        {
            assert!(modifier.as_ref().map(|m| m.left_align).unwrap_or(false));
            assert_eq!(modifier.as_ref().and_then(|m| m.min_width), Some(5));
        }
    }

    #[test]
    fn test_percent_escape() {
        let tokens = scan("%%test").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], Token::Percent);
        assert_eq!(tokens[1], Token::Literal("test".to_string()));
    }

    #[test]
    fn test_newline() {
        let tokens = scan("%n").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Newline);
    }

    #[test]
    fn test_thread() {
        let tokens = scan("%thread").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            tokens[0],
            Token::Conversion {
                keyword: Keyword::Thread,
                ..
            }
        ));
    }

    #[test]
    fn test_mdc() {
        let tokens = scan("%X{request_id}").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(
            tokens[0],
            Token::Conversion {
                keyword: Keyword::Mdc,
                ..
            }
        ));
        if let Token::Conversion { option, .. } = &tokens[0] {
            assert_eq!(option.as_deref(), Some("request_id"));
        }
    }
}
