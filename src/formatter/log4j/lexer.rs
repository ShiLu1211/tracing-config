//! Log4j `PatternLayout` pattern lexer.
//!
//! Parses a log4j-style conversion pattern into a flat `Vec<Token>`.
//! Composite keywords (`%enc`, `%maxLen`, `%highlight`, color words)
//! recursively parse their sub-pattern the same way logback does.

use crate::error::ConfigError;

use super::super::logback::align::FormatModifier;
use super::super::logback::color::Color;

/// Single token in a parsed log4j pattern.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Token {
    Literal(String),
    /// A conversion word with optional modifier, keyword, option, and sub-pattern.
    Conversion {
        /// Format modifier (alignment / truncation).
        modifier: Option<FormatModifier>,
        /// The conversion keyword.
        keyword: Keyword,
        /// Optional parameter in `{…}` braces.
        option: Option<String>,
        /// Sub-pattern for composite keywords.
        sub_pattern: Option<Vec<Token>>,
    },
    Newline,
    Percent,
}

/// Conversion-word keywords supported by the log4j engine.
#[derive(Debug, Clone, PartialEq)]
pub enum Keyword {
    /// Time — `%d{pattern}`.
    Date,
    /// `%level` / `%p` / `%le`.
    Level,
    /// Thread name — `%t` / `%T` / `%thread`.
    Thread,
    /// Logger (target) — `%c` / `%logger` / `%lo`.
    Logger,
    /// Class name — `%C`. We use the same value as the logger
    /// (the `tracing` `target`).
    Class,
    /// Message — `%m` / `%msg` / `%message`.
    Message,
    /// Source file — `%F`.
    File,
    /// Source line number — `%L`.
    Line,
    /// Source method — `%M`.
    Method,
    /// NDC — `%x`. We render the active span's name.
    Ndc,
    /// MDC — `%X{key}` / `%X` (all).
    Mdc,
    /// Process ID — `%pid` / `%P`.
    Pid,
    /// Exception / throwable — `%throwable{n}` / `%ex{n}`.
    Throwable,
    /// Cause-chain root — `%rEx`.
    RootException,
    /// Suppress implicit exception — `%nopex`.
    NopException,
    /// Composite: auto-color by level.
    Highlight,
    /// Composite: fixed color.
    Clr,
    /// Composite: HTML/XML/JSON escape — `%enc{sub}{mode}`.
    Enc(EscapeMode),
    /// Composite: max length — `%maxLen{sub}{n}`.
    MaxLen(usize),
    /// Composite: color word (`%red(sub)`, etc.).
    ColorWord(Color),
    /// Escape `%`.
    Percent,
}

/// Output encoding for `%enc{sub}{mode}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum EscapeMode {
    Html,
    Xml,
    Json,
    Crlf,
    None,
}

/// Parse a log4j pattern string.
pub fn scan(pattern: &str) -> Result<Vec<Token>, ConfigError> {
    let chars: Vec<char> = pattern.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] != '%' {
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

        i += 1;
        if i >= chars.len() {
            tokens.push(Token::Literal("%".to_string()));
            break;
        }

        // Modifier: %-5.10
        let (align, align_len) = parse_modifier(&chars, i);
        i += align_len;
        if i >= chars.len() {
            tokens.push(Token::Literal("%".to_string()));
            break;
        }

        let spec_char = chars[i];
        i += 1;

        // %% literal
        if spec_char == '%' {
            tokens.push(Token::Percent);
            continue;
        }
        // %n newline
        if spec_char == 'n' {
            tokens.push(Token::Newline);
            continue;
        }

        let (keyword, option, kw_len, is_composite) = parse_keyword(spec_char, &chars, i)?;
        i += kw_len;

        let sub_pattern = if is_composite && i > 0 {
            // The character just before i tells us which delimiter
            // opens the sub-pattern: logback-style `(` uses `)` to
            // close, while log4j-style `{` (e.g. `%enc{%m}{html}`)
            // uses `}`.
            let (open, close): (char, char) = match chars[i - 1] {
                '(' => ('(', ')'),
                '{' => ('{', '}'),
                _ => ('\0', '\0'),
            };
            if open != '\0' {
                let start = i;
                let mut depth: usize = 1;
                let mut end = start;
                while end < chars.len() && depth > 0 {
                    if chars[end] == open {
                        depth += 1;
                    } else if chars[end] == close {
                        depth -= 1;
                    }
                    end += 1;
                }
                if depth == 0 && end > start {
                    let sub_str: String = chars[start..end - 1].iter().collect();
                    let sub_tokens = scan(&sub_str).ok();
                    i = end;
                    sub_tokens
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // Post-option for composite keywords: %enc(sub){html},
        // %maxLen(sub){100}. Always one `{...}` after the
        // sub-pattern's closing `)`.
        let mut option = option;
        if i < chars.len() && chars[i] == '{' {
            if let Some(end_offset) = chars[i..].iter().position(|&c| c == '}') {
                let opt_str: String = chars[i + 1..i + end_offset].iter().collect();
                option = Some(opt_str);
                i += end_offset + 1;
            }
        }

        // %highlight / %clr take a color name in the post-option
        // (matching logback's %clr semantics).
        if matches!(keyword, Keyword::Highlight | Keyword::Clr) {
            // The color was already captured as `option` above; we
            // keep it so the renderer can pick it up. No additional
            // action needed.
        }

        // The keyword may need to absorb the post-option value
        // (e.g., for `Enc` and `MaxLen`).
        let keyword = match keyword {
            Keyword::Enc(_) => match option.as_deref().and_then(parse_escape_mode) {
                Some(mode) => Keyword::Enc(mode),
                None => Keyword::Enc(EscapeMode::None),
            },
            Keyword::MaxLen(_) => match option.as_deref().and_then(|s| s.parse().ok()) {
                Some(n) => Keyword::MaxLen(n),
                None => Keyword::MaxLen(0),
            },
            other => other,
        };

        tokens.push(Token::Conversion {
            modifier: align,
            keyword,
            option,
            sub_pattern,
        });
    }

    Ok(tokens)
}

fn parse_modifier(chars: &[char], start: usize) -> (Option<FormatModifier>, usize) {
    let mut pos = start;
    let mut left_align = false;
    if pos < chars.len() && chars[pos] == '-' {
        left_align = true;
        pos += 1;
    }
    if pos >= chars.len() || !chars[pos].is_ascii_digit() {
        return (None, 0);
    }
    let mut num_str = String::new();
    while pos < chars.len() && chars[pos].is_ascii_digit() {
        num_str.push(chars[pos]);
        pos += 1;
    }
    let min_width: usize = num_str.parse().unwrap_or(0);
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
    (
        Some(FormatModifier {
            left_align,
            min_width: Some(min_width),
            max_width,
            max_from_end: false,
        }),
        pos - start,
    )
}

fn parse_keyword(
    first_char: char,
    chars: &[char],
    pos: usize,
) -> Result<(Keyword, Option<String>, usize, bool), ConfigError> {
    // Color words first (just like the logback lexer).
    if let Some((color, len)) = try_parse_color_word(chars, pos.saturating_sub(1)) {
        return Ok((Keyword::ColorWord(color), None, len, true));
    }

    match first_char {
        'd' => parse_dated(chars, pos),
        'D' => Ok((Keyword::Date, None, 1, false)),
        'p' | 'P' => {
            // %p / %level. Some log4j variants also accept
            // `%level`, but `%p` is the canonical short form.
            if pos + 4 <= chars.len()
                && chars[pos] == 'i'
                && chars[pos + 1] == 'd'
                && chars[pos + 2] == '('
            {
                // %pid( — legacy log4j syntax; treat as Pid.
                return Ok((Keyword::Pid, None, 3, false));
            }
            Ok((Keyword::Level, None, 1, false))
        }
        't' | 'T' => {
            if pos + 5 <= chars.len()
                && chars[pos] == 'h'
                && chars[pos + 1] == 'r'
                && chars[pos + 2] == 'e'
                && chars[pos + 3] == 'a'
                && chars[pos + 4] == 'd'
            {
                Ok((Keyword::Thread, None, 5, false))
            } else {
                Ok((Keyword::Thread, None, 1, false))
            }
        }
        'c' => {
            // %c{1.} / %c / %class
            if pos + 4 <= chars.len()
                && chars[pos] == 'l'
                && chars[pos + 1] == 'a'
                && chars[pos + 2] == 's'
                && chars[pos + 3] == 's'
            {
                return Ok((Keyword::Class, None, 5, false));
            }
            if pos < chars.len() && chars[pos] == '{' {
                if let Some(end) = chars[pos..].iter().position(|&c| c == '}') {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Logger, Some(opt), end + 1, false));
                }
            }
            Ok((Keyword::Logger, None, 1, false))
        }
        'C' => {
            if pos + 4 <= chars.len()
                && chars[pos] == 'l'
                && chars[pos + 1] == 'a'
                && chars[pos + 2] == 's'
                && chars[pos + 3] == 's'
            {
                Ok((Keyword::Class, None, 5, false))
            } else {
                Ok((Keyword::Class, None, 1, false))
            }
        }
        'm' | 'M' => {
            // %m / %msg / %message / %method / %maxLen
            // %maxLen starts with "axLen(" — check it first so we
            // don't get distracted by the %message / %msg / %method
            // matches. Both lowercase and uppercase 'M' are accepted.
            if pos + 6 <= chars.len()
                && chars[pos] == 'a'
                && chars[pos + 1] == 'x'
                && chars[pos + 2] == 'L'
                && chars[pos + 3] == 'e'
                && chars[pos + 4] == 'n'
                && (chars[pos + 5] == '(' || chars[pos + 5] == '{')
            {
                return Ok((Keyword::MaxLen(0), None, 6, true));
            }
            if pos + 6 <= chars.len()
                && chars[pos] == 'e'
                && chars[pos + 1] == 's'
                && chars[pos + 2] == 's'
                && chars[pos + 3] == 'a'
                && chars[pos + 4] == 'g'
                && chars[pos + 5] == 'e'
            {
                Ok((Keyword::Message, None, 7, false))
            } else if pos + 2 <= chars.len() && chars[pos] == 's' && chars[pos + 1] == 'g' {
                Ok((Keyword::Message, None, 3, false))
            } else if first_char == 'M' {
                // %method
                if pos + 5 <= chars.len()
                    && chars[pos] == 'e'
                    && chars[pos + 1] == 't'
                    && chars[pos + 2] == 'h'
                    && chars[pos + 3] == 'o'
                    && chars[pos + 4] == 'd'
                {
                    Ok((Keyword::Method, None, 6, false))
                } else {
                    Ok((Keyword::Method, None, 1, false))
                }
            } else {
                Ok((Keyword::Message, None, 1, false))
            }
        }
        'L' => Ok((Keyword::Line, None, 1, false)),
        'F' => Ok((Keyword::File, None, 1, false)),
        'X' => {
            // %X{key} / %X
            if pos < chars.len() && chars[pos] == '{' {
                if let Some(end) = chars[pos..].iter().position(|&c| c == '}') {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Mdc, Some(opt), end + 1, false));
                }
            }
            Ok((Keyword::Mdc, None, 0, false))
        }
        'x' => {
            // %xEx or %xThrowable or %x (NDC)
            if pos + 2 <= chars.len() && chars[pos] == 'E' && chars[pos + 1] == 'x' {
                if pos + 2 < chars.len() && chars[pos + 2] == '{' {
                    if let Some(end) = chars[pos + 3..].iter().position(|&c| c == '}') {
                        let opt: String = chars[pos + 3..pos + 3 + end].iter().collect();
                        return Ok((Keyword::RootException, Some(opt), end + 4, false));
                    }
                }
                Ok((Keyword::RootException, None, 3, false))
            } else {
                Ok((Keyword::Ndc, None, 0, false))
            }
        }
        'E' => {
            // %ENC or just %E
            if pos + 2 <= chars.len()
                && chars[pos] == 'N'
                && chars[pos + 1] == 'C'
                && chars[pos + 2] == '('
            {
                Ok((Keyword::Enc(EscapeMode::None), None, 3, true))
            } else {
                Err(ConfigError::PatternParse {
                    message: "unknown placeholder '%E'".to_string(),
                    position: pos,
                })
            }
        }
        'e' => {
            // %enc{sub}{mode} — log4j wraps the sub-pattern in
            // `{}` rather than `()`. We treat both as the
            // sub-pattern delimiter.
            if pos + 2 <= chars.len()
                && chars[pos] == 'n'
                && chars[pos + 1] == 'c'
                && (chars[pos + 2] == '(' || chars[pos + 2] == '{')
            {
                let kw_len = 3; // "nc" + the delimiter
                return Ok((Keyword::Enc(EscapeMode::None), None, kw_len, true));
            }
            // %ex / %exception / %throwable
            if pos < chars.len() && chars[pos] == 'x' {
                if pos + 8 <= chars.len()
                    && chars[pos + 1..pos + 8] == ['x', 'c', 'e', 'p', 't', 'i', 'o', 'n']
                {
                    // %exception
                    if pos + 8 < chars.len() && chars[pos + 8] == '{' {
                        if let Some(end) = chars[pos + 9..].iter().position(|&c| c == '}') {
                            let opt: String = chars[pos + 9..pos + 9 + end].iter().collect();
                            return Ok((Keyword::Throwable, Some(opt), end + 10, false));
                        }
                    }
                    Ok((Keyword::Throwable, None, 9, false))
                } else if pos + 1 < chars.len() && chars[pos + 1] == '{' {
                    if let Some(end) = chars[pos + 2..].iter().position(|&c| c == '}') {
                        let opt: String = chars[pos + 2..pos + 2 + end].iter().collect();
                        return Ok((Keyword::Throwable, Some(opt), end + 3, false));
                    }
                    Ok((Keyword::Throwable, None, 2, false))
                } else {
                    Ok((Keyword::Throwable, None, 2, false))
                }
            } else {
                Err(ConfigError::PatternParse {
                    message: "unknown placeholder '%e'".to_string(),
                    position: pos,
                })
            }
        }
        'n' => {
            if pos + 4 <= chars.len()
                && chars[pos] == 'o'
                && chars[pos + 1] == 'p'
                && chars[pos + 2] == 'e'
                && chars[pos + 3] == 'x'
            {
                Ok((Keyword::NopException, None, 5, false))
            } else {
                Ok((Keyword::Ndc, None, 1, false))
            }
        }
        'h' => {
            if pos + 9 <= chars.len()
                && chars[pos] == 'i'
                && chars[pos + 1] == 'g'
                && chars[pos + 2] == 'h'
                && chars[pos + 3] == 'l'
                && chars[pos + 4] == 'i'
                && chars[pos + 5] == 'g'
                && chars[pos + 6] == 'h'
                && chars[pos + 7] == 't'
                && (chars[pos + 8] == '(' || chars[pos + 8] == '{')
            {
                Ok((Keyword::Highlight, None, 9, true))
            } else {
                Err(ConfigError::PatternParse {
                    message: "unknown placeholder '%h'".to_string(),
                    position: pos,
                })
            }
        }
        'r' => {
            // %rEx or %r (relative). Relative is uncommon in
            // log4j; we keep it for parity with the logback engine.
            if pos + 2 <= chars.len() && chars[pos] == 'E' && chars[pos + 1] == 'x' {
                Ok((Keyword::RootException, None, 3, false))
            } else {
                Err(ConfigError::PatternParse {
                    message: "unsupported placeholder '%r'".to_string(),
                    position: pos,
                })
            }
        }
        'K' => {
            // %K — log4j's "map diagnostic context" (MDC key).
            // Treat it the same as %X{key}.
            if pos < chars.len() && chars[pos] == '{' {
                if let Some(end) = chars[pos..].iter().position(|&c| c == '}') {
                    let opt: String = chars[pos + 1..pos + end].iter().collect();
                    return Ok((Keyword::Mdc, Some(opt), end + 1, false));
                }
            }
            Ok((Keyword::Mdc, None, 1, false))
        }
        'R' => {
            // %rEx (duplicate of `r` branch — kept for clarity
            // when reading the matcher).
            Ok((Keyword::RootException, None, 1, false))
        }
        _ => Err(ConfigError::PatternParse {
            message: format!("unknown placeholder '%{}'", first_char),
            position: pos,
        }),
    }
}

fn parse_dated(
    chars: &[char],
    pos: usize,
) -> Result<(Keyword, Option<String>, usize, bool), ConfigError> {
    if pos < chars.len() && chars[pos] == '{' {
        if let Some(end) = chars[pos..].iter().position(|&c| c == '}') {
            let opt: String = chars[pos + 1..pos + end].iter().collect();
            return Ok((Keyword::Date, Some(opt), end + 1, false));
        }
    }
    Ok((Keyword::Date, None, 1, false))
}

fn try_parse_color_word(chars: &[char], spec_pos: usize) -> Option<(Color, usize)> {
    const WORDS: &[(&str, Color)] = &[
        ("red", Color::Red),
        ("green", Color::Green),
        ("yellow", Color::Yellow),
        ("blue", Color::Blue),
        ("magenta", Color::Magenta),
        ("cyan", Color::Cyan),
        ("white", Color::White),
        ("faint", Color::Faint),
        ("boldRed", Color::BoldRed),
        ("boldGreen", Color::BoldGreen),
        ("boldYellow", Color::BoldYellow),
        ("boldBlue", Color::BoldBlue),
        ("boldMagenta", Color::BoldMagenta),
        ("boldCyan", Color::BoldCyan),
        ("boldWhite", Color::BoldWhite),
    ];
    for (word, color) in WORDS {
        let word_chars: Vec<char> = word.chars().collect();
        if spec_pos + word_chars.len() + 1 > chars.len() {
            continue;
        }
        if &chars[spec_pos..spec_pos + word_chars.len()] != word_chars.as_slice() {
            continue;
        }
        if chars[spec_pos + word_chars.len()] != '(' {
            continue;
        }
        return Some((*color, word_chars.len()));
    }
    None
}

fn parse_escape_mode(s: &str) -> Option<EscapeMode> {
    match s.to_lowercase().as_str() {
        "html" => Some(EscapeMode::Html),
        "xml" => Some(EscapeMode::Xml),
        "json" => Some(EscapeMode::Json),
        "crlf" => Some(EscapeMode::Crlf),
        "none" | "" => Some(EscapeMode::None),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_only() {
        let tokens = scan("hello world").unwrap();
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Token::Literal(s) if s == "hello world"));
    }

    #[test]
    fn test_basic_keywords() {
        let tokens = scan("%d %p %t %m %n").unwrap();
        assert_eq!(tokens.len(), 5);
    }

    #[test]
    fn test_logger_dot_notation() {
        let tokens = scan("%c{1.}").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::Logger,
                option: Some(opt),
                ..
            } if opt == "1."
        ));
    }

    #[test]
    fn test_logger_plain() {
        let tokens = scan("%c{2}").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::Logger,
                option: Some(opt),
                ..
            } if opt == "2"
        ));
    }

    #[test]
    fn test_enc_with_html() {
        let tokens = scan("%enc{%m}{html}").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::Enc(EscapeMode::Html),
                sub_pattern: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn test_maxlen() {
        let tokens = scan("%maxLen{%msg}{50}").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::MaxLen(50),
                sub_pattern: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn test_throwable_short() {
        let tokens = scan("%ex{3}").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::Throwable,
                option: Some(opt),
                ..
            } if opt == "3"
        ));
    }

    #[test]
    fn test_x_is_ndc() {
        let tokens = scan("%x").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::Ndc,
                ..
            }
        ));
    }

    #[test]
    fn test_color_word() {
        let tokens = scan("%red(%p)").unwrap();
        assert!(matches!(
            &tokens[0],
            Token::Conversion {
                keyword: Keyword::ColorWord(Color::Red),
                sub_pattern: Some(_),
                ..
            }
        ));
    }
}
