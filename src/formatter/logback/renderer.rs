//! Logback pattern renderer - renders tokens to formatted output.

use std::fmt;
use std::time::Instant;
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::FmtContext;
use tracing_subscriber::fmt::FormatEvent;
use tracing_subscriber::registry::LookupSpan;

use super::abbreviator::abbreviate;
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
            mut message,
            mut fields,
            error_chain,
            marker,
        } = visitor;

        // Strip surrounding quotes from Debug-formatted message.
        message = strip_message_quotes(&message);

        // Drop the "message" pseudo-field if it slipped in via the
        // default `record_debug` arm.
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

fn strip_message_quotes(raw: &str) -> String {
    let trimmed = raw.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map(|s| s.to_string())
        .unwrap_or_else(|| trimmed.to_string())
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
                let j_fmt = option.as_deref().unwrap_or("%Y-%m-%d %H:%M:%S%.3f");
                *option = Some(super::date::convert_pattern(j_fmt));
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
            let _ = writeln!(writer);
            let _ = writeln!(writer, "{}", data.error_chain.join("\nCaused by: "));
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

                let output = self.render_keyword(meta, data, keyword, option, &inner, ctx);
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
                let inner = if let Some(sub_tokens) = sub_pattern {
                    let mut buf = String::new();
                    for sub_token in sub_tokens {
                        buf.push_str(&self.render_token_string(meta, data, sub_token, ctx));
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
            Keyword::Relative => {
                let elapsed = self.start_time.elapsed().as_millis();
                elapsed.to_string()
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
            Keyword::Message => data.message.clone(),
            Keyword::Line => meta.line().map(|l| l.to_string()).unwrap_or_default(),
            Keyword::File => meta.file().unwrap_or("").to_string(),
            Keyword::Method => ctx
                .lookup_current()
                .map(|span| span.name().to_string())
                .unwrap_or_default(),
            Keyword::Mdc => mdc_render(ctx, option.as_deref(), data),
            Keyword::Kvp => data.kvp_string(),
            Keyword::Marker => data.marker.clone().unwrap_or_default(),
            Keyword::Exception => exception_render(data, option.as_deref(), false),
            Keyword::RootException => exception_render(data, option.as_deref(), true),
            Keyword::ExtendedException => exception_render(data, option.as_deref(), true),
            Keyword::NopException => String::new(),
            Keyword::Pid => std::process::id().to_string(),
            Keyword::Highlight => {
                if inner.is_empty() {
                    color::highlight(*meta.level(), meta.level().as_str()).to_string()
                } else {
                    color::highlight(*meta.level(), inner).to_string()
                }
            }
            Keyword::Clr => clr_render(inner, option.as_deref()),
            Keyword::ColorWord(c) => {
                if inner.is_empty() {
                    String::new()
                } else {
                    color::with_color(*c, inner).to_string()
                }
            }
            Keyword::Percent => "%".to_string(),
        }
    }
}

/// Render the `%X{key}` / `%X` / `%mdc` conversion word.
///
/// Looks up the requested key in the active span chain (using
/// `SpanFieldStore` populated by `SpanFieldsLayer`), falling back to a
/// value in the event fields. With no key, renders every key/value in
/// the current span.
fn mdc_render<S, N>(ctx: &FmtContext<'_, S, N>, key: Option<&str>, data: &EventData) -> String
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> tracing_subscriber::fmt::FormatFields<'a> + 'static,
{
    match key {
        Some(k) => {
            if let Some(v) = lookup_span_field(ctx, k) {
                return v;
            }
            if let Some(v) = data.field(k) {
                return v.to_string();
            }
            String::new()
        }
        None => {
            // All fields from innermost span (logback semantics).
            let span = match ctx.lookup_current() {
                Some(s) => s,
                None => return String::new(),
            };
            let extensions = span.extensions();
            let store = match extensions.get::<SpanFieldStore>() {
                Some(s) => s,
                None => return String::new(),
            };
            let mut s = String::new();
            s.push('{');
            let mut first = true;
            for (k, v) in store.iter() {
                if !first {
                    s.push_str(", ");
                }
                s.push_str(k);
                s.push('=');
                s.push_str(v);
                first = false;
            }
            s.push('}');
            s
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

/// Render `%clr(sub){color}` — wraps the pre-rendered sub-pattern in
/// the given ANSI color.
fn clr_render(inner: &str, color_opt: Option<&str>) -> String {
    if inner.is_empty() {
        return String::new();
    }
    let color = color_opt
        .and_then(color::Color::parse)
        .unwrap_or(color::Color::White);
    color::with_color(color, inner)
}

/// Render `%ex{depth}` / `%exception` / `%throwable`.
///
/// `with_package_info` (used by `%rEx` / `%xEx`) appends
/// `[crate_name version]` to each cause-chain frame, matching
/// logback's root-cause converter behaviour.
fn exception_render(data: &EventData, depth_opt: Option<&str>, with_package_info: bool) -> String {
    if data.error_chain.is_empty() {
        return String::new();
    }
    let max_depth: usize = depth_opt.and_then(|s| s.parse().ok()).unwrap_or(usize::MAX);

    let frames: Vec<String> = data
        .error_chain
        .iter()
        .take(max_depth)
        .map(|f| {
            if with_package_info {
                format!("{} [{} {}]", f, CRATE_NAME, CRATE_VERSION)
            } else {
                f.clone()
            }
        })
        .collect();
    frames.join("\nCaused by: ")
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
        assert_eq!(data.kvp_string(), "user_id=42");
    }

    #[test]
    fn test_collect_error_chain_frames() {
        let inner = std::io::Error::new(std::io::ErrorKind::Other, "root");
        let frames = collect_error_chain_frames(&inner);
        assert_eq!(frames, vec!["root".to_string()]);
    }
}
