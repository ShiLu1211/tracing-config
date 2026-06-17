//! Span field capture layer for `%X{key}` / `%mdc` support.
//!
//! Records every span's field values into the span's extensions, so that
//! the logback renderer can look them up by name when emitting `%X{key}`.
//!
//! Without this layer, span fields can't be recovered (the registry's
//! `SpanAttributes` only stores field metadata, not values).
//!
//! # Example
//!
//! ```rust
//! use tracing_subscriber::layer::SubscriberExt;
//! use tracing_subscriber::Registry;
//! use tracing_declarative::span_fields::SpanFieldsLayer;
//!
//! let subscriber = Registry::default()
//!     .with(SpanFieldsLayer)
//!     .with(tracing_subscriber::fmt::layer());
//! tracing::subscriber::with_default(subscriber, || {
//!     let span = tracing::span!(tracing::Level::INFO, "req", user_id = 42);
//!     let _enter = span.enter();
//!     tracing::info!("handling request");
//! });
//! ```

use std::collections::BTreeMap;

use tracing::field::Visit;
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// In-memory snapshot of a span's field values.
///
/// `BTreeMap` so `%X` (no key) renders fields in a stable order.
#[derive(Debug, Default, Clone)]
pub struct SpanFieldStore {
    fields: BTreeMap<String, String>,
}

impl SpanFieldStore {
    /// Look up a single key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }

    /// Iterate `(key, value)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fields.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    fn record(&mut self, name: &str, value: String) {
        self.fields.insert(name.to_string(), value);
    }
}

/// Subscriber layer that copies span field values into the span's
/// extensions. Should be installed before any user spans are created.
///
/// This layer is automatically installed by [`crate::init`] and related
/// functions. You only need to add it manually if you are building a
/// custom subscriber.
///
/// # Example
///
/// ```rust
/// use tracing_subscriber::layer::SubscriberExt;
/// use tracing_subscriber::Registry;
/// use tracing_declarative::span_fields::SpanFieldsLayer;
///
/// let subscriber = Registry::default()
///     .with(SpanFieldsLayer)
///     .with(tracing_subscriber::fmt::layer());
/// ```
#[derive(Debug, Default, Clone, Copy)]
pub struct SpanFieldsLayer;

impl<S> Layer<S> for SpanFieldsLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let span = match ctx.span(id) {
            Some(s) => s,
            None => return,
        };
        let mut extensions = span.extensions_mut();
        let mut store = SpanFieldStore::default();
        let mut visitor = FieldRecorder { store: &mut store };
        attrs.record(&mut visitor);
        extensions.insert(store);
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        let span = match ctx.span(id) {
            Some(s) => s,
            None => return,
        };
        let mut extensions = span.extensions_mut();
        if let Some(store) = extensions.get_mut::<SpanFieldStore>() {
            let mut visitor = FieldRecorder { store };
            values.record(&mut visitor);
        }
    }
}

struct FieldRecorder<'a> {
    store: &'a mut SpanFieldStore,
}

impl<'a> Visit for FieldRecorder<'a> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.store.record(field.name(), format!("{:?}", value));
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.store.record(field.name(), value.to_string());
    }
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.store.record(field.name(), value.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_get_and_iter() {
        let mut s = SpanFieldStore::default();
        s.record("user_id", "42".into());
        s.record("trace", "abc".into());
        assert_eq!(s.get("user_id"), Some("42"));
        assert_eq!(s.get("missing"), None);
        let collected: Vec<_> = s.iter().collect();
        assert_eq!(collected, vec![("trace", "abc"), ("user_id", "42")]);
    }
}
