# Changelog

All notable changes to `tracing-declarative` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] — 2026-06-18

### Added

- **`StackBuf<N>`** — stack-allocated buffer for modifier-bearing tokens,
  eliminating heap `String` allocation on the hot path. Falls back to heap
  when content exceeds 128 bytes.
- **`abbreviate_to_writer()`** — zero-alloc variant of `abbreviate()` for both
  logback and log4j, writing directly to `fmt::Write` without intermediate
  `Vec` or `String` allocation.
- **`SegIter`** — custom dual-delimiter iterator for log4j abbreviator,
  eliminating the `replace("::", ".")` + `split('.').collect()` allocation.
- **End-to-end benchmark suite** — `benches/e2e.rs` with 6 groups (B1-B6)
  covering formatting, date, color, multi-appender, sampling, and config parsing.
- **Benchmark documentation** — `docs/BENCHMARK.md`, `docs/BENCHMARK-PLAN.md`,
  `docs/PERF-OPTIMIZATION.md`.

### Changed

- **`render_token()` modifier path** — uses `StackBuf<128>` instead of
  `String::new()` for keyword output, with heap fallback on overflow.
- **`TimestampCache::get()`** — fast path checks `last_millis` (u64 compare)
  first; `chrono_fmt` stored as `Option<String>`, only allocated on change.
- **`Keyword::Logger/Class`** — both renderers now call `abbreviate_to_writer()`
  instead of `abbreviate()` + `write_str()`.
- **B4 multi-appender benchmark** — uses `MakeNullWriter` to eliminate
  `Arc<Mutex<Vec>>` lock contention from measurements.

### Performance

- logback simple: 9.77µs → 9.32µs (**-4.6%**, from 3.12x to 2.90x vs default)
- log4j full: 11.55µs → 10.28µs (**-11.0%**, from 3.69x to 3.20x vs default)
- Color words (`%cyan`): +0% overhead (unchanged)
- `%highlight`: -30% vs pre-Phase-1

## [1.0.0] — 2026-06-17

### Added

- **`#![deny(missing_docs)]`** — all public items now require doc comments.
- **Log4j PatternLayout engine** — `formatter/log4j/` with full lexer and
  renderer. Supports `%c{1.}` dot-notation abbreviation, `%x` NDC,
  `%enc{...}{html|xml|json|crlf}` escaping, `%maxLen{...}{n}` truncation,
  color words, and throwable chain rendering.
- **OpenTelemetry integration** — `opentelemetry` feature gate adding
  OTLP HTTP/protobuf trace export via `opentelemetry` 0.32 +
  `tracing-opentelemetry` 0.33. Configured via `[opentelemetry]` TOML
  section. `OtelGuard` ensures provider lifetime.
- **`ConfigError::OpenTelemetry(String)`** variant (feature-gated).
- **Performance: `%d` date format preprocessing** — Java SimpleDateFormat
  patterns are converted to chrono strftime at formatter construction time,
  eliminating per-event `convert_pattern()` calls (~2.5x speedup).
- **Performance: cached `has_exception`/`has_nopex` flags** — computed once
  at construction instead of scanning the token tree on every event.
- **Performance: direct writer write** — `Literal`/`Newline`/`Percent`
  tokens write directly to the `Writer` without intermediate `String`
  allocation.
- **Criterion benchmarks** — `benches/formatter.rs` covering lexer scan,
  date format, and abbreviator paths.
- **Comprehensive rustdoc** — 20 doctests covering all public API
  functions, module-level architecture docs, and complete TOML
  configuration reference examples.
- **`try_init_from_str` deprecated** — marked `#[deprecated]` since it
  has identical behavior to `init_from_str`.

### Fixed

- Single-character keywords (`%d`, `%r`, `%p`, `%t`, `%c`, `%C`, `%m`,
  `%M`, `%L`, `%F`, `%X`) had `kw_len = 1` instead of `0`, causing
  off-by-one errors in literal text after these keywords.
- 4 clippy warnings in log4j lexer (`format!` → `.to_string()`) and
  collapsible-if patterns in both renderers.

### Changed

- `LogbackFormatter::new()` and `Log4jFormatter::new()` now preprocess
  date format tokens and cache exception/nopex flags at construction.
- `LogbackFormatter::has_exception_token()` and `has_nopex_token()`
  removed; use `has_exception` / `has_nopex` fields directly.
- `format_time()` import removed from both renderers; `Keyword::Date`
  rendering now uses pre-converted chrono format strings directly.
- `render_token()` in both renderers now writes `Literal`/`Newline`/
  `Percent` directly to the writer, bypassing `render_token_string()`
  for these simple token types.

## [0.2.0] — core feature completion

### Added

- **Exception rendering** (`%ex` / `%rEx` / `%xEx`) now actually
  outputs the error chain instead of an empty string. `%rEx` and `%xEx`
  append `[crate-name version]` to each frame; `%ex{n}` truncates the
  chain to `n` frames. When the pattern contains no exception word,
  the error text is appended implicitly (matching logback).
- **`%clr(sub){color}`** — the post-sub-pattern `{color}` option is
  now consumed and rendered. Defaults to white when omitted.
- **Color words** (`%red(...)`, `%green(...)`, `%boldBlue(...)`, …) are
  parsed as composite color wrappers equivalent to
  `%clr(...){red}` / `%clr(...){green}` / etc.
- **Span field capture** (`%X{key}` / `%X` / `%mdc`) — a new
  `SpanFieldsLayer` is installed automatically and stores span field
  values into the span's extensions, which the renderer reads at
  format time.
- **`%kvp`** now emits the actual recorded field values (e.g.
  `user_id=42 action=login`) instead of the empty Debug of the
  call-site `FieldSet`.
- **`%marker`** — emits a `marker` field if the event records one.
- **Multi-appender fan-out** — `stdout + file`, `stderr + rolling_file`,
  arbitrary combinations all share the same `MultiMakeWriter`. The
  `stdout+file` hard-coding from v0.1 is gone.
- **Per-appender level filter** — `[[appender]] level = "warn"` adds
  the equivalent `EnvFilter` directive automatically.
- **Windows ANSI** — `init()` calls `windows::enable_ansi_escapes()` so
  color escape codes render on Windows consoles.
- **`%M`** — the innermost active span's name is emitted as a
  "caller method" approximation.
- **`SpanFieldsLayer`** and `SpanFieldStore` are exposed as public
  types so other code can read or extend the span field capture.

### Fixed

- Lexer off-by-one in `%X{...}` and `%c{...}` — a trailing literal
  immediately after the closing `}` was silently dropped.
- `format_error_chain` pre-flattened the entire cause chain into a
  single string, which broke `%ex{n}` depth limits; now the chain is
  stored per-frame.
- `Error::source` is no longer walked twice for the same chain.
- `record_debug` for the `error` field now wraps the rendered text in
  a single-frame chain so `error = %err` (Display) still produces
  `%ex` output.

### Changed

- `Config::from_default_file()` no longer returns `Err(NoConfig)` when
  no `tracing.toml` is found — it returns a built-in default
  (`info` level, stdout, default formatter). Malformed files still
  error.
- The search path now also includes `<exe-dir>/tracing.toml`.
- Renderer internals were refactored: `render_token_string` is now
  the single source of truth for token → text, removing ~80% of
  duplication between `render_token` (writer-backed) and
  `render_token_to_buf` (buf-backed).

## [0.1.0] — initial release

- TOML-driven config (`[global]`, `[filter]`, `[[appender]]`,
  `[sampling]`, `[opentelemetry]`).
- `default` and `logback` formatter engines.
- `stdout`, `stderr`, `file`, `rolling_file` appenders (single-appender
  only; `stdout+file` was the only supported multi-appender combo).
- Logback lexer with modifiers, abbreviations, color words, dates.
- Hot-reload feature (gated, partial — see `docs/ROADMAP.md`).
