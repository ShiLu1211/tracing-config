//! Logback pattern renderer - renders tokens to formatted output.

use std::borrow::Cow;
use std::fmt;
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use super::color;
use super::lexer::{Keyword, Token};

use crate::span_fields::SpanFieldStore;
use crate::{CRATE_NAME, CRATE_VERSION};

/// Collected data from a single `tracing::Event`.
///
/// Populated once at the top of `format_event` and shared with all token
/// renderers, so that `%msg`, `%kvp`, `%ex` etc. can share one pass over the
/// event's field set. Exposed publicly so alternative formatter engines
/// (e.g. `log4j`) can share the same one-pass collection.
pub struct EventData {
    /// The event's message, with the wrapping Debug quotes stripped.
    pub message: String,
    /// All non-message event fields, in the order they were recorded.
    pub fields: Vec<(String, String)>,
    /// Captured error chain for `%ex` / `%rEx` / `%xEx`, one entry per
    /// `source()` frame (root cause last).
    pub error_chain: Vec<String>,
    /// Captured marker (a string field with name "marker") for `%marker`.
    pub marker: Option<String>,
}

impl EventData {
    /// Collect event data via a single `event.record()` pass.
    pub fn collect(event: &tracing::Event<'_>) -> Self {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);
        let EventVisitor {
            message,
            mut fields,
            error_chain,
            marker,
        } = visitor;

        let message = strip_message_quotes(&message).into_owned();

        fields.retain(|(name, _)| name != "message");

        Self {
            message,
            fields,
            error_chain,
            marker,
        }
    }

    /// Find a non-message field by name (first match wins).
    pub fn field(&self, name: &str) -> Option<&str> {
        self.fields
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Render all non-message fields as `key1=value1 key2=value2`.
    pub fn kvp_string(&self) -> String {
        let mut buf = String::new();
        for (name, value) in &self.fields {
            if !buf.is_empty() {
                buf.push(' ');
            }
            buf.push_str(name);
            buf.push('=');
            buf.push_str(value);
        }
        buf
    }
}

fn strip_message_quotes(raw: &str) -> Cow<'_, str> {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map(|s| Cow::Owned(s.to_string()))
        .unwrap_or_else(|| Cow::Borrowed(trimmed))
}

#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: Vec<(String, String)>,
    error_chain: Vec<String>,
    marker: Option<String>,
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let formatted = format!("{:?}", value);
        match field.name() {
            "message" => {
                if self.message.is_empty() {
                    self.message = formatted;
                }
            }
            "marker" => {
                if self.marker.is_none() {
                    self.marker = Some(formatted);
                }
            }
            "error" => {
                // The user wrote `error = ?err` or `error = %err` — the
                // macro calls `record_debug` instead of `record_error`,
                // so we only have a string. Treat it as a single frame.
                if self.error_chain.is_empty() {
                    self.error_chain.push(formatted.clone());
                }
                self.fields.push((field.name().to_string(), formatted));
            }
            name => {
                self.fields.push((name.to_string(), formatted));
            }
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "message" => {
                if self.message.is_empty() {
                    self.message = value.to_string();
                }
            }
            "marker" => {
                if self.marker.is_none() {
                    self.marker = Some(value.to_string());
                }
            }
            name => {
                self.fields.push((name.to_string(), value.to_string()));
            }
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        match field.name() {
            "error" => {
                self.error_chain = collect_error_chain_frames(value);
            }
            "marker" => {
                if self.marker.is_none() {
                    self.marker = Some(value.to_string());
                }
            }
            name => {
                self.fields.push((name.to_string(), value.to_string()));
            }
        }
    }
}

/// Walk an error and its `source()` chain, returning one entry per
/// frame so the renderer can apply depth limits and package-info
/// decoration per-frame.
pub fn collect_error_chain_frames(err: &(dyn std::error::Error + 'static)) -> Vec<String> {
    let mut frames = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = current {
        frames.push(e.to_string());
        current = e.source();
    }
    frames
}

/// Logback pattern formatter implementing `FormatEvent`.
pub struct LogbackFormatter {
    tokens: Vec<Token>,
    start_time: Instant,
    has_exception: bool,
    has_nopex: bool,
    pid: String,
}

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
                    *option = Some(super::date::convert_pattern(j_fmt));
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
                Keyword::Exception
                    | Keyword::RootException
                    | Keyword::ExtendedException
                    | Keyword::NopException
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

thread_local! {
    pub(crate) static THREAD_NAME: String = std::thread::current().name().unwrap_or("unknown").to_string();
}

thread_local! {
    pub(crate) static TS_CACHE: std::cell::RefCell<TimestampCache> = std::cell::RefCell::new(TimestampCache::new());
}

pub(crate) struct TimestampCache {
    last_millis: u64,
    formatted: String,
    chrono_fmt: Option<String>,
}

impl TimestampCache {
    pub(crate) fn new() -> Self {
        Self {
            last_millis: 0,
            formatted: String::new(),
            chrono_fmt: None,
        }
    }

    pub(crate) fn get(&mut self, chrono_fmt: &str) -> &str {
        let now_millis = timestamp_millis();
        // Fast path: millis unchanged → return cached (no string comparison)
        if now_millis == self.last_millis {
            if let Some(ref fmt) = self.chrono_fmt {
                if fmt == chrono_fmt {
                    return &self.formatted;
                }
            }
        }
        // Slow path: millis changed or format changed
        self.formatted = chrono::Local::now().format(chrono_fmt).to_string();
        self.last_millis = now_millis;
        // Only allocate chrono_fmt string when it actually changes
        match self.chrono_fmt {
            Some(ref mut fmt) if fmt == chrono_fmt => {}
            _ => self.chrono_fmt = Some(chrono_fmt.to_string()),
        }
        &self.formatted
    }
}

pub(crate) fn timestamp_millis() -> u64 {
    static BASE: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let base = BASE.get_or_init(Instant::now);
    base.elapsed().as_millis() as u64
}

impl LogbackFormatter {
    /// Create a new formatter from pre-parsed tokens.
    pub fn new(mut tokens: Vec<Token>) -> Self {
        preprocess_date_tokens(&mut tokens);
        let has_exception = scan_has_exception(&tokens);
        let has_nopex = scan_has_nopex(&tokens);
        Self {
            tokens,
            start_time: Instant::now(),
            has_exception,
            has_nopex,
            pid: std::process::id().to_string(),
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

impl LogbackFormatter {
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
                    for sub_token in sub_tokens {
                        buf.push_str(&self.render_token_string(meta, data, sub_token, ctx));
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
                    // Sub-pattern path (highlight/clr/colorword): rare, keep String
                    let mut inner = String::new();
                    for sub_token in sub_tokens {
                        inner.push_str(&self.render_token_string(meta, data, sub_token, ctx));
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
                    // No sub-pattern + modifier: use stack buffer to avoid heap alloc
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
            Keyword::Relative => {
                let elapsed = self.start_time.elapsed().as_millis();
                write!(writer, "{}", elapsed)
            }
            Keyword::Level => writer.write_str(meta.level().as_str()),
            Keyword::Thread => THREAD_NAME.with(|n| writer.write_str(n.as_str())),
            Keyword::Logger | Keyword::Class => {
                let target = meta.target();
                let len = option.as_ref().and_then(|s| s.parse().ok()).unwrap_or(0);
                if len == 0 {
                    writer.write_str(target)
                } else {
                    super::abbreviator::abbreviate_to_writer(target, len, writer)
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
            Keyword::Mdc => mdc_write(ctx, option.as_deref(), data, writer),
            Keyword::Kvp => kvp_write(data, writer),
            Keyword::Marker => match &data.marker {
                Some(m) => writer.write_str(m),
                None => Ok(()),
            },
            Keyword::Exception => exception_write(data, option.as_deref(), false, writer),
            Keyword::RootException | Keyword::ExtendedException => {
                exception_write(data, option.as_deref(), true, writer)
            }
            Keyword::NopException => Ok(()),
            Keyword::Pid => writer.write_str(&self.pid),
            Keyword::Highlight => {
                let level = *meta.level();
                if inner.is_empty() {
                    color::highlight_to_writer(level, meta.level().as_str(), writer)
                } else {
                    color::highlight_to_writer(level, inner, writer)
                }
            }
            Keyword::Clr => clr_write(inner, option.as_deref(), writer),
            Keyword::ColorWord(c) => {
                if inner.is_empty() {
                    Ok(())
                } else {
                    color::with_color_to_writer(*c, inner, writer)
                }
            }
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

/// Stack-allocated buffer for short keyword outputs, avoiding heap allocation
/// on the modifier path. 128 bytes covers Level (≤5), Logger (≤36), Thread
/// (≤15), Date (≤30), etc. Falls back to heap `String` on overflow.
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
        // SAFETY: we only write valid UTF-8 via `write_str`
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

struct StringAdapter<'a>(&'a mut String);

impl fmt::Write for StringAdapter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

fn kvp_write(data: &EventData, writer: &mut dyn fmt::Write) -> fmt::Result {
    let mut first = true;
    for (name, value) in &data.fields {
        if !first {
            writer.write_str(" ")?;
        }
        writer.write_str(name)?;
        writer.write_str("=")?;
        writer.write_str(value)?;
        first = false;
    }
    Ok(())
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
            if let Some(v) = lookup_span_field(ctx, k) {
                return writer.write_str(&v);
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
            let extensions = span.extensions();
            let store = match extensions.get::<SpanFieldStore>() {
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

fn lookup_span_field<S, N>(ctx: &FmtContext<'_, S, N>, key: &str) -> Option<String>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    let span = ctx.lookup_current()?;
    let extensions = span.extensions();
    let store = extensions.get::<SpanFieldStore>()?;
    store.get(key).map(String::from)
}

fn exception_write(
    data: &EventData,
    depth_opt: Option<&str>,
    with_package_info: bool,
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
        if with_package_info {
            write!(writer, "{} [{} {}]", frame, CRATE_NAME, CRATE_VERSION)?;
        } else {
            writer.write_str(frame)?;
        }
        first = false;
    }
    Ok(())
}

fn clr_write(inner: &str, color_opt: Option<&str>, writer: &mut dyn fmt::Write) -> fmt::Result {
    if inner.is_empty() {
        return Ok(());
    }
    let color = color_opt
        .and_then(color::Color::parse)
        .unwrap_or(color::Color::White);
    color::with_color_to_writer(color, inner, writer)
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

    #[test]
    fn test_has_exception_token_detects_ex() {
        let tokens = super::super::lexer::scan("%msg %ex").unwrap();
        let f = LogbackFormatter::new(tokens);
        assert!(f.has_exception);
        assert!(!f.has_nopex);
    }

    #[test]
    fn test_has_exception_token_detects_nopex() {
        let tokens = super::super::lexer::scan("%msg %nopex").unwrap();
        let f = LogbackFormatter::new(tokens);
        assert!(f.has_exception);
        assert!(f.has_nopex);
    }

    #[test]
    fn test_has_exception_token_false() {
        let tokens = super::super::lexer::scan("%msg %level").unwrap();
        let f = LogbackFormatter::new(tokens);
        assert!(!f.has_exception);
        assert!(!f.has_nopex);
    }

    #[test]
    fn test_strip_message_quotes() {
        assert_eq!(strip_message_quotes("\"hello\""), "hello");
        assert_eq!(strip_message_quotes("hello"), "hello");
        assert_eq!(strip_message_quotes("\"\""), "");
        assert_eq!(strip_message_quotes(""), "");
    }

    #[test]
    fn test_event_data_kvp() {
        let data = EventData {
            message: "hi".to_string(),
            fields: vec![("user_id".into(), "42".into())],
            error_chain: vec![],
            marker: None,
        };
        let mut buf = String::new();
        kvp_write(&data, &mut buf).unwrap();
        assert_eq!(buf, "user_id=42");
    }

    #[test]
    fn test_collect_error_chain_frames() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "root");
        let frames = collect_error_chain_frames(&inner);
        assert_eq!(frames, vec!["root".to_string()]);
    }

    #[test]
    fn test_exception_write() {
        let data = EventData {
            message: String::new(),
            fields: vec![],
            error_chain: vec!["err1".to_string(), "err2".to_string()],
            marker: None,
        };
        let mut buf = String::new();
        exception_write(&data, None, false, &mut buf).unwrap();
        assert_eq!(buf, "err1\nCaused by: err2");
    }

    #[test]
    fn test_timestamp_cache() {
        let mut cache = TimestampCache::new();
        let s1 = cache.get("%Y-%m-%d").to_string();
        let s2 = cache.get("%Y-%m-%d").to_string();
        assert_eq!(s1, s2);
    }
}
