//! Log4j `PatternLayout` renderer.

use std::fmt;
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use super::super::logback::color;
use super::super::logback::date;
use super::super::logback::renderer::{EventData, THREAD_NAME, TS_CACHE};
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
                if let Some(j_fmt) = option.as_deref() {
                    *option = Some(date::convert_pattern(j_fmt));
                }
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
    pid: String,
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
            pid: std::process::id().to_string(),
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
            writer.write_str("\n")?;
            exception_write(&data, None, false, &mut writer)?;
        }
        Ok(())
    }
}

struct StringAdapter<'a>(&'a mut String);

impl fmt::Write for StringAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

/// Stack-allocated buffer for short keyword outputs (mirrors logback's `StackBuf`).
struct StackBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
}

impl<const N: usize> StackBuf<N> {
    fn new() -> Self {
        Self {
            buf: [0u8; N],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> fmt::Write for StackBuf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
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
                if let Some(sub_tokens) = sub_pattern {
                    // Sub-pattern path: rare, keep String
                    let mut inner = String::new();
                    for sub in sub_tokens {
                        inner.push_str(&self.render_token_string(meta, data, sub, ctx));
                    }
                    if let Some(m) = modifier {
                        let mut output = String::new();
                        self.render_keyword_to_string(
                            meta,
                            data,
                            keyword,
                            option,
                            &inner,
                            ctx,
                            &mut output,
                        );
                        m.apply_to_writer(&output, writer)
                    } else {
                        self.render_keyword_to_writer(
                            meta, data, keyword, option, &inner, ctx, writer,
                        )
                    }
                } else if let Some(m) = modifier {
                    // No sub-pattern + modifier: use stack buffer
                    let mut buf = StackBuf::<128>::new();
                    if self
                        .render_keyword_to_writer(meta, data, keyword, option, "", ctx, &mut buf)
                        .is_ok()
                    {
                        m.apply_to_writer(buf.as_str(), writer)
                    } else {
                        // Stack overflow fallback to heap
                        let mut output = String::new();
                        self.render_keyword_to_string(
                            meta,
                            data,
                            keyword,
                            option,
                            "",
                            ctx,
                            &mut output,
                        );
                        m.apply_to_writer(&output, writer)
                    }
                } else {
                    // No sub-pattern + no modifier: direct write (zero-alloc)
                    self.render_keyword_to_writer(meta, data, keyword, option, "", ctx, writer)
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

                let mut output = String::new();
                self.render_keyword_to_string(
                    meta,
                    data,
                    keyword,
                    option,
                    &inner,
                    ctx,
                    &mut output,
                );
                if let Some(m) = modifier {
                    m.apply(&output)
                } else {
                    output
                }
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_keyword_to_writer<S, N>(
        &self,
        meta: &tracing::Metadata<'_>,
        data: &EventData,
        keyword: &Keyword,
        option: &Option<String>,
        inner: &str,
        ctx: &FmtContext<'_, S, N>,
        writer: &mut dyn fmt::Write,
    ) -> fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        match keyword {
            Keyword::Date => {
                let chrono_fmt = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                TS_CACHE.with(|c| {
                    let mut cache = c.borrow_mut();
                    let cached = cache.get(chrono_fmt);
                    writer.write_str(cached)
                })
            }
            Keyword::Level => writer.write_str(meta.level().as_str()),
            Keyword::Thread => THREAD_NAME.with(|n| writer.write_str(n.as_str())),
            Keyword::Logger | Keyword::Class => {
                let target = meta.target();
                let depth: usize = option
                    .as_deref()
                    .and_then(|s| s.trim_end_matches('.').parse().ok())
                    .unwrap_or(0);
                if depth == 0 {
                    writer.write_str(target)
                } else {
                    super::abbreviator::abbreviate_to_writer(target, depth, writer)
                }
            }
            Keyword::Message => writer.write_str(&data.message),
            Keyword::Line => match meta.line() {
                Some(l) => write!(writer, "{}", l),
                None => Ok(()),
            },
            Keyword::File => writer.write_str(meta.file().unwrap_or("")),
            Keyword::Method => match ctx.lookup_current() {
                Some(span) => writer.write_str(span.name()),
                None => Ok(()),
            },
            Keyword::Ndc => match ctx.lookup_current() {
                Some(span) => writer.write_str(span.name()),
                None => Ok(()),
            },
            Keyword::Mdc => mdc_write(ctx, option.as_deref(), data, writer),
            Keyword::Pid => writer.write_str(&self.pid),
            Keyword::Throwable => exception_write(data, option.as_deref(), false, writer),
            Keyword::RootException => exception_write(data, option.as_deref(), true, writer),
            Keyword::NopException => Ok(()),
            Keyword::Highlight => {
                let level = *meta.level();
                if inner.is_empty() {
                    color::highlight_to_writer(level, meta.level().as_str(), writer)
                } else {
                    color::highlight_to_writer(level, inner, writer)
                }
            }
            Keyword::Clr => {
                if inner.is_empty() {
                    Ok(())
                } else {
                    let c = option
                        .as_deref()
                        .and_then(color::Color::parse)
                        .unwrap_or(color::Color::White);
                    color::with_color_to_writer(c, inner, writer)
                }
            }
            Keyword::ColorWord(c) => {
                if inner.is_empty() {
                    Ok(())
                } else {
                    color::with_color_to_writer(*c, inner, writer)
                }
            }
            Keyword::Enc(mode) => escape_write(inner, *mode, writer),
            Keyword::MaxLen(n) => truncate_write(inner, *n, writer),
            Keyword::Percent => writer.write_str("%"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_keyword_to_string<S, N>(
        &self,
        meta: &tracing::Metadata<'_>,
        data: &EventData,
        keyword: &Keyword,
        option: &Option<String>,
        inner: &str,
        ctx: &FmtContext<'_, S, N>,
        buf: &mut String,
    ) where
        S: Subscriber + for<'a> LookupSpan<'a>,
        N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
    {
        let mut adapter = StringAdapter(buf);
        let _ =
            self.render_keyword_to_writer(meta, data, keyword, option, inner, ctx, &mut adapter);
    }
}

/// Log4j dot-notation abbreviation. `n` is the number of trailing
/// segments to keep in full; everything before is shortened to its
/// first letter. (For `n=0`, only the last segment survives.)
#[allow(dead_code)]
fn abbreviate_log4j(target: &str, n: usize) -> String {
    super::abbreviator::abbreviate(target, n)
}

fn mdc_write<S, N>(
    ctx: &FmtContext<'_, S, N>,
    key: Option<&str>,
    data: &EventData,
    writer: &mut dyn fmt::Write,
) -> fmt::Result
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
                        return writer.write_str(v);
                    }
                }
            }
            if let Some(v) = data.field(k) {
                return writer.write_str(v);
            }
            Ok(())
        }
        None => {
            let span = match ctx.lookup_current() {
                Some(s) => s,
                None => return Ok(()),
            };
            let ext = span.extensions();
            let store = match ext.get::<SpanFieldStore>() {
                Some(s) => s,
                None => return Ok(()),
            };
            writer.write_char('{')?;
            let mut first = true;
            for (k, v) in store.iter() {
                if !first {
                    writer.write_str(", ")?;
                }
                writer.write_str(k)?;
                writer.write_char('=')?;
                writer.write_str(v)?;
                first = false;
            }
            writer.write_char('}')
        }
    }
}

fn exception_write(
    data: &EventData,
    depth_opt: Option<&str>,
    with_pkg: bool,
    writer: &mut dyn fmt::Write,
) -> fmt::Result {
    if data.error_chain.is_empty() {
        return Ok(());
    }
    let max_depth: usize = depth_opt.and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);
    let mut first = true;
    for frame in data.error_chain.iter().take(max_depth) {
        if !first {
            writer.write_str("\nCaused by: ")?;
        }
        if with_pkg {
            write!(writer, "{} [{} {}]", frame, CRATE_NAME, CRATE_VERSION)?;
        } else {
            writer.write_str(frame)?;
        }
        first = false;
    }
    Ok(())
}

fn escape_write(input: &str, mode: EscapeMode, writer: &mut dyn fmt::Write) -> fmt::Result {
    match mode {
        EscapeMode::None => writer.write_str(input),
        EscapeMode::Html => html_escape_write(input, writer),
        EscapeMode::Xml => html_escape_write(input, writer),
        EscapeMode::Json => json_escape_write(input, writer),
        EscapeMode::Crlf => {
            for c in input.chars() {
                match c {
                    '\r' => writer.write_str("\\r")?,
                    '\n' => writer.write_str("\\n")?,
                    _ => writer.write_char(c)?,
                }
            }
            Ok(())
        }
    }
}

fn html_escape_write(s: &str, writer: &mut dyn fmt::Write) -> fmt::Result {
    for c in s.chars() {
        match c {
            '&' => writer.write_str("&amp;")?,
            '<' => writer.write_str("&lt;")?,
            '>' => writer.write_str("&gt;")?,
            '"' => writer.write_str("&quot;")?,
            '\'' => writer.write_str("&#39;")?,
            _ => writer.write_char(c)?,
        }
    }
    Ok(())
}

fn json_escape_write(s: &str, writer: &mut dyn fmt::Write) -> fmt::Result {
    for c in s.chars() {
        match c {
            '"' => writer.write_str("\\\"")?,
            '\\' => writer.write_str("\\\\")?,
            '\n' => writer.write_str("\\n")?,
            '\r' => writer.write_str("\\r")?,
            '\t' => writer.write_str("\\t")?,
            '\x08' => writer.write_str("\\b")?,
            '\x0c' => writer.write_str("\\f")?,
            c if (c as u32) < 0x20 => write!(writer, "\\u{:04x}", c as u32)?,
            _ => writer.write_char(c)?,
        }
    }
    Ok(())
}

fn truncate_write(s: &str, n: usize, writer: &mut dyn fmt::Write) -> fmt::Result {
    let mut len = 0;
    for c in s.chars() {
        if len + c.len_utf8() > n {
            break;
        }
        writer.write_char(c)?;
        len += c.len_utf8();
    }
    Ok(())
}

#[allow(dead_code)]
fn escape(input: &str, mode: EscapeMode) -> String {
    match mode {
        EscapeMode::None => input.to_string(),
        EscapeMode::Html => html_escape(input),
        EscapeMode::Xml => html_escape(input),
        EscapeMode::Json => json_escape(input),
        EscapeMode::Crlf => input.replace('\r', "\\r").replace('\n', "\\n"),
    }
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
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
