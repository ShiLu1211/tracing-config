//! Logback pattern renderer - renders tokens to formatted output.

use std::fmt;
use std::time::Instant;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use super::abbreviator::abbreviate;
use super::align::FormatModifier;
use super::color;
use super::date::format_time;
use super::lexer::{Keyword, Token};

/// Logback pattern formatter.
pub struct LogbackFormatter {
    tokens: Vec<Token>,
    start_time: Instant,
}

impl LogbackFormatter {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            start_time: Instant::now(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for LogbackFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();

        // Extract message
        let mut message_buf = String::new();
        event.record(
            &mut |field: &tracing::field::Field, value: &dyn std::fmt::Debug| {
                if field.name() == "message" {
                    use std::fmt::Write;
                    let _ = write!(&mut message_buf, "{:?}", value);
                }
            },
        );
        // Strip quotes from Debug format
        let message = message_buf.trim();
        let message = message
            .strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(message);

        // Render tokens
        for token in &self.tokens {
            match token {
                Token::Literal(s) => writer.write_str(s)?,
                Token::Newline => writer.write_char('\n')?,
                Token::Percent => writer.write_str("%")?,
                Token::Conversion {
                    modifier,
                    keyword,
                    option,
                    sub_pattern,
                } => {
                    self.render_keyword(
                        &mut writer,
                        meta,
                        message,
                        modifier,
                        keyword,
                        option,
                        sub_pattern.as_ref(),
                    )?;
                }
            }
        }

        Ok(())
    }
}

impl LogbackFormatter {
    #[allow(clippy::too_many_arguments)]
    fn render_keyword(
        &self,
        writer: &mut Writer<'_>,
        meta: &tracing::Metadata<'_>,
        message: &str,
        modifier: &Option<FormatModifier>,
        keyword: &Keyword,
        option: &Option<String>,
        sub_pattern: Option<&Vec<Token>>,
    ) -> fmt::Result {
        // For composite keywords (Highlight, Clr, ColorWord), render sub_pattern first
        let inner = if let Some(sub_tokens) = sub_pattern {
            let mut buf = String::new();
            for sub_token in sub_tokens {
                self.render_token_to_string(sub_token, meta, message, &mut buf)?;
            }
            buf
        } else {
            String::new()
        };

        let output = match keyword {
            Keyword::Date => {
                let fmt_str = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                format_time(fmt_str)
            }
            Keyword::Relative => {
                let elapsed = self.start_time.elapsed().as_millis();
                format!("{}", elapsed)
            }
            Keyword::Level => meta.level().as_str().to_string(),
            Keyword::Thread => std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
            Keyword::Logger | Keyword::Class => {
                let target = meta.target();
                let len = option.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0);
                abbreviate(target, len)
            }
            Keyword::Message => message.to_string(),
            Keyword::Line => meta.line().map(|l| l.to_string()).unwrap_or_default(),
            Keyword::File => meta.file().unwrap_or("").to_string(),
            Keyword::Method => {
                // NOTE: %M currently returns the span name, not the actual Rust function name.
                // Getting the real function name requires macro support (tracing macros inject span).
                // This is marked unstable until proper macro integration is implemented.
                String::new()
            }
            Keyword::Mdc => {
                if let Some(key) = option {
                    format!("%X{{{}}}", key)
                } else {
                    String::new()
                }
            }
            Keyword::Kvp => {
                // %kvp - all event fields as key=value pairs
                let mut buf = String::new();
                let fields = meta.fields();
                for field in fields.iter() {
                    if !buf.is_empty() {
                        buf.push(' ');
                    }
                    use std::fmt::Write;
                    let _ = write!(&mut buf, "{}={:?}", field.name(), field);
                }
                buf
            }
            Keyword::Marker => String::new(),
            Keyword::Exception | Keyword::RootException | Keyword::NopException => String::new(),
            Keyword::Pid => std::process::id().to_string(),
            Keyword::Highlight => {
                // inner contains the rendered sub-pattern (e.g., "ERROR")
                // Apply level-based color to the inner content
                if inner.is_empty() {
                    color::highlight(*meta.level(), meta.level().as_str()).to_string()
                } else {
                    color::highlight(*meta.level(), &inner).to_string()
                }
            }
            Keyword::Clr => String::new(),
            Keyword::ColorWord(c) => {
                if inner.is_empty() {
                    String::new()
                } else {
                    color::with_color(*c, &inner).to_string()
                }
            }
            Keyword::Percent => "%".to_string(),
        };

        // Apply modifier
        if let Some(m) = modifier {
            writer.write_str(&m.apply(&output))?;
        } else {
            writer.write_str(&output)?;
        }

        Ok(())
    }

    /// Render a single token to a string buffer (for sub-patterns in composite converters)
    #[allow(clippy::too_many_arguments)]
    fn render_token_to_string(
        &self,
        token: &Token,
        meta: &tracing::Metadata<'_>,
        message: &str,
        buf: &mut String,
    ) -> fmt::Result {
        match token {
            Token::Literal(s) => {
                buf.push_str(s);
            }
            Token::Newline => {
                buf.push('\n');
            }
            Token::Percent => {
                buf.push('%');
            }
            Token::Conversion {
                modifier,
                keyword,
                option,
                sub_pattern,
            } => {
                // Render sub-pattern if present
                let inner = if let Some(sub_tokens) = sub_pattern {
                    let mut inner_buf = String::new();
                    for st in sub_tokens {
                        self.render_token_to_string(st, meta, message, &mut inner_buf)?;
                    }
                    inner_buf
                } else {
                    String::new()
                };

                let output = match keyword {
                    Keyword::Date => {
                        let fmt_str = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                        format_time(fmt_str)
                    }
                    Keyword::Relative => {
                        let elapsed = self.start_time.elapsed().as_millis();
                        format!("{}", elapsed)
                    }
                    Keyword::Level => meta.level().as_str().to_string(),
                    Keyword::Thread => std::thread::current()
                        .name()
                        .unwrap_or("unknown")
                        .to_string(),
                    Keyword::Logger | Keyword::Class => {
                        let target = meta.target();
                        let len = option.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0);
                        abbreviate(target, len)
                    }
                    Keyword::Message => message.to_string(),
                    Keyword::Line => meta.line().map(|l| l.to_string()).unwrap_or_default(),
                    Keyword::File => meta.file().unwrap_or("").to_string(),
                    Keyword::Method => String::new(),
                    Keyword::Mdc => {
                        if let Some(key) = option {
                            format!("%X{{{}}}", key)
                        } else {
                            String::new()
                        }
                    }
                    Keyword::Kvp => {
                        let mut kv_buf = String::new();
                        let fields = meta.fields();
                        for field in fields.iter() {
                            if !kv_buf.is_empty() {
                                kv_buf.push(' ');
                            }
                            use std::fmt::Write;
                            let _ = write!(&mut kv_buf, "{}={:?}", field.name(), field);
                        }
                        kv_buf
                    }
                    Keyword::Marker => String::new(),
                    Keyword::Exception | Keyword::RootException | Keyword::NopException => {
                        String::new()
                    }
                    Keyword::Pid => std::process::id().to_string(),
                    Keyword::Highlight => {
                        if inner.is_empty() {
                            color::highlight(*meta.level(), meta.level().as_str()).to_string()
                        } else {
                            color::highlight(*meta.level(), &inner).to_string()
                        }
                    }
                    Keyword::Clr => String::new(),
                    Keyword::ColorWord(c) => {
                        if inner.is_empty() {
                            String::new()
                        } else {
                            color::with_color(*c, &inner).to_string()
                        }
                    }
                    Keyword::Percent => "%".to_string(),
                };

                if let Some(m) = modifier {
                    buf.push_str(&m.apply(&output));
                } else {
                    buf.push_str(&output);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_pattern() {
        let tokens = vec![Token::Literal("hello".to_string())];
        let formatter = LogbackFormatter::new(tokens);
        assert!(formatter.start_time.elapsed().as_secs() < 1);
    }
}
