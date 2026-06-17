//! Log4j `PatternLayout` renderer.

use std::fmt;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use super::super::logback::color;
use super::super::logback::date;
use super::super::logback::renderer::EventData;
use super::lexer::{EscapeMode, Keyword, Token};

use crate::span_fields::SpanFieldStore;
use crate::{CRATE_NAME, CRATE_VERSION};

fn preprocess_date_tokens(tokens: &mut [Token]) {
    for token in tokens.iter_mut() {
        match token {
            Token::Conversion {
                keyword: Keyword::Date,
                option,
                sub_pattern,
                ..
            } => {
                let j_fmt = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                *option = Some(date::convert_pattern(j_fmt));
                if let Some(sub) = sub_pattern {
                    preprocess_date_tokens(sub);
                }
            }
            Token::Conversion {
                sub_pattern: Some(sub),
                ..
            } => preprocess_date_tokens(sub),
            Token::Conversion { .. } => {}
            _ => {}
        }
    }
}

fn scan_has_exception(tokens: &[Token]) -> bool {
    for t in tokens {
        if let Token::Conversion {
            keyword,
            sub_pattern,
            ..
        } = t
        {
            if matches!(
                keyword,
                Keyword::Throwable | Keyword::RootException | Keyword::NopException
            ) {
                return true;
            }
            if let Some(sub) = sub_pattern {
                if scan_has_exception(sub) {
                    return true;
                }
            }
        }
    }
    false
}

fn scan_has_nopex(tokens: &[Token]) -> bool {
    for t in tokens {
        if let Token::Conversion {
            keyword,
            sub_pattern,
            ..
        } = t
        {
            if matches!(keyword, Keyword::NopException) {
                return true;
            }
            if let Some(sub) = sub_pattern {
                if scan_has_nopex(sub) {
                    return true;
                }
            }
        }
    }
    false
}

/// Log4j pattern formatter implementing `FormatEvent`.
pub struct Log4jFormatter {
    tokens: Vec<Token>,
    has_exception: bool,
    has_nopex: bool,
}

impl Log4jFormatter {
    /// Create a new formatter from pre-parsed tokens.
    pub fn new(mut tokens: Vec<Token>) -> Self {
        preprocess_date_tokens(&mut tokens);
        let has_exception = scan_has_exception(&tokens);
        let has_nopex = scan_has_nopex(&tokens);
        Self {
            tokens,
            has_exception,
            has_nopex,
        }
    }
}

impl<S, N> FormatEvent<S, N> for Log4jFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let meta = event.metadata();
        let data = EventData::collect(event);

        for token in &self.tokens {
            self.render_token(&mut writer, meta, &data, token, ctx)?;
        }

        if !self.has_exception && !self.has_nopex && !data.error_chain.is_empty() {
            let _ = writeln!(writer);
            let _ = writeln!(writer, "{}", data.error_chain.join("\nCaused by: "));
        }
        Ok(())
    }
}

impl Log4jFormatter {
    fn render_token<S, N>(
        &self,
        writer: &mut Writer<'_>,
        meta: &tracing::Metadata<'_>,
        data: &EventData,
        token: &Token,
        ctx: &FmtContext<'_, S, N>,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        match token {
            Token::Literal(s) => writer.write_str(s),
            Token::Newline => writer.write_str("\n"),
            Token::Percent => writer.write_str("%"),
            Token::Conversion {
                modifier,
                keyword,
                option,
                sub_pattern,
            } => {
                let inner = if let Some(sub_tokens) = sub_pattern {
                    let mut buf = String::new();
                    for sub in sub_tokens {
                        buf.push_str(&self.render_token_string(meta, data, sub, ctx));
                    }
                    buf
                } else {
                    String::new()
                };

                let output = self.render_keyword(meta, data, keyword, option, &inner, ctx);
                if let Some(m) = modifier {
                    writer.write_str(&m.apply(&output))
                } else {
                    writer.write_str(&output)
                }
            }
        }
    }

    fn render_token_string<S, N>(
        &self,
        meta: &tracing::Metadata<'_>,
        data: &EventData,
        token: &Token,
        ctx: &FmtContext<'_, S, N>,
    ) -> String
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        match token {
            Token::Literal(s) => s.clone(),
            Token::Newline => "\n".to_string(),
            Token::Percent => "%".to_string(),
            Token::Conversion {
                modifier,
                keyword,
                option,
                sub_pattern,
            } => {
                let inner = if let Some(sub_tokens) = sub_pattern {
                    let mut buf = String::new();
                    for sub in sub_tokens {
                        buf.push_str(&self.render_token_string(meta, data, sub, ctx));
                    }
                    buf
                } else {
                    String::new()
                };

                let output = self.render_keyword(meta, data, keyword, option, &inner, ctx);
                if let Some(m) = modifier {
                    m.apply(&output)
                } else {
                    output
                }
            }
        }
    }

    fn render_keyword<S, N>(
        &self,
        meta: &tracing::Metadata<'_>,
        data: &EventData,
        keyword: &Keyword,
        option: &Option<String>,
        inner: &str,
        ctx: &FmtContext<'_, S, N>,
    ) -> String
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        match keyword {
            Keyword::Date => {
                let chrono_fmt = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                chrono::Local::now().format(chrono_fmt).to_string()
            }
            Keyword::Level => meta.level().as_str().to_string(),
            Keyword::Thread => std::thread::current()
                .name()
                .unwrap_or("unknown")
                .to_string(),
            Keyword::Logger | Keyword::Class => {
                let target = meta.target();
                let depth: usize = option
                    .as_deref()
                    .and_then(|s| s.trim_end_matches('.').parse().ok())
                    .unwrap_or(0);
                // log4j's {n.} semantics match our `abbreviate(n)` —
                // `n` is the number of trailing segments to keep
                // in full form. For `n=0` we get the last segment
                // only; otherwise we keep `n` segments and shorten
                // the rest.
                abbreviate_log4j(target, depth)
            }
            Keyword::Message => data.message.clone(),
            Keyword::Line => meta.line().map(|l| l.to_string()).unwrap_or_default(),
            Keyword::File => meta.file().unwrap_or("").to_string(),
            Keyword::Method => ctx
                .lookup_current()
                .map(|span| span.name().to_string())
                .unwrap_or_default(),
            Keyword::Ndc => ctx
                .lookup_current()
                .map(|span| span.name().to_string())
                .unwrap_or_default(),
            Keyword::Mdc => mdc_render(ctx, option.as_deref(), data),
            Keyword::Pid => std::process::id().to_string(),
            Keyword::Throwable => exception_render(data, option.as_deref(), false),
            Keyword::RootException => exception_render(data, option.as_deref(), true),
            Keyword::NopException => String::new(),
            Keyword::Highlight => {
                if inner.is_empty() {
                    color::highlight(*meta.level(), meta.level().as_str()).to_string()
                } else {
                    color::highlight(*meta.level(), inner).to_string()
                }
            }
            Keyword::Clr => {
                if inner.is_empty() {
                    String::new()
                } else {
                    let c = option
                        .as_deref()
                        .and_then(color::Color::parse)
                        .unwrap_or(color::Color::White);
                    color::with_color(c, inner)
                }
            }
            Keyword::ColorWord(c) => {
                if inner.is_empty() {
                    String::new()
                } else {
                    color::with_color(*c, inner)
                }
            }
            Keyword::Enc(mode) => escape(inner, *mode),
            Keyword::MaxLen(n) => truncate(inner, *n),
            Keyword::Percent => "%".to_string(),
        }
    }
}

/// Log4j dot-notation abbreviation. `n` is the number of trailing
/// segments to keep in full; everything before is shortened to its
/// first letter. (For `n=0`, only the last segment survives.)
fn abbreviate_log4j(target: &str, n: usize) -> String {
    super::abbreviator::abbreviate(target, n)
}

fn mdc_render<S, N>(ctx: &FmtContext<'_, S, N>, key: Option<&str>, data: &EventData) -> String
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    match key {
        Some(k) => {
            if let Some(span) = ctx.lookup_current() {
                let ext = span.extensions();
                if let Some(store) = ext.get::<SpanFieldStore>() {
                    if let Some(v) = store.get(k) {
                        return v.to_string();
                    }
                }
            }
            if let Some(v) = data.field(k) {
                return v.to_string();
            }
            String::new()
        }
        None => {
            let span = match ctx.lookup_current() {
                Some(s) => s,
                None => return String::new(),
            };
            let ext = span.extensions();
            let store = match ext.get::<SpanFieldStore>() {
                Some(s) => s,
                None => return String::new(),
            };
            let mut out = String::new();
            out.push('{');
            let mut first = true;
            for (k, v) in store.iter() {
                if !first {
                    out.push_str(", ");
                }
                out.push_str(k);
                out.push('=');
                out.push_str(v);
                first = false;
            }
            out.push('}');
            out
        }
    }
}

fn exception_render(data: &EventData, depth_opt: Option<&str>, with_pkg: bool) -> String {
    if data.error_chain.is_empty() {
        return String::new();
    }
    let max_depth: usize = depth_opt.and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let frames: Vec<String> = data
        .error_chain
        .iter()
        .take(max_depth)
        .map(|f| {
            if with_pkg {
                format!("{} [{} {}]", f, CRATE_NAME, CRATE_VERSION)
            } else {
                f.clone()
            }
        })
        .collect();
    frames.join("\nCaused by: ")
}

fn escape(input: &str, mode: EscapeMode) -> String {
    match mode {
        EscapeMode::None => input.to_string(),
        EscapeMode::Html => html_escape(input),
        EscapeMode::Xml => html_escape(input), // XML is a subset of HTML for these chars
        EscapeMode::Json => json_escape(input),
        EscapeMode::Crlf => input.replace('\r', "\\r").replace('\n', "\\n"),
    }
}

fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => out.push(c),
        }
    }
    out
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        // Truncate at char boundary to avoid splitting a multi-byte char.
        let mut out = String::new();
        for c in s.chars() {
            if out.len() + c.len_utf8() > n {
                break;
            }
            out.push(c);
        }
        out
    }
}

// Suppress unused-import warnings for items re-exported from
// super::logback; the log4j renderer leans on the same primitives.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_html_replaces_specials() {
        assert_eq!(
            escape("<a href=\"x\">&", EscapeMode::Html),
            "&lt;a href=&quot;x&quot;&gt;&amp;"
        );
    }

    #[test]
    fn escape_json_handles_quotes_and_control_chars() {
        let s = "a\"b\nc\td";
        let out = escape(s, EscapeMode::Json);
        assert_eq!(out, "a\\\"b\\nc\\td");
    }

    #[test]
    fn truncate_respects_char_boundaries() {
        let s = "中文汉字字符";
        assert_eq!(truncate(s, 4), "中");
        assert_eq!(truncate(s, 7), "中文");
    }

    #[test]
    fn truncate_no_op_when_short() {
        assert_eq!(truncate("hi", 10), "hi");
    }
}
