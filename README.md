# tracing-declarative

> Declarative `tracing` initialization via `tracing.toml`.

`tracing-declarative` lets you configure [`tracing`]'s subscriber entirely from a
`tracing.toml` file — no hard-coded formatters, no manual `Layer` plumbing,
no copy-pasted `EnvFilter` directives. Drop a file next to your binary,
pick the appenders and pattern you want, and you're done.

```toml
# tracing.toml
[global]
level = "info"

[[appender]]
name = "stdout"
kind = "stdout"

[appender.formatter]
type = "logback"
pattern = "%d{HH:mm:ss.SSS} %-5level [%t] %logger{36} - %msg%n"
```

```rust
fn main() {
    tracing_declarative::init().expect("tracing init");
    tracing::info!("hello {}", "world");
}
```

## Highlights

- **Three formatter engines** — `default` (tracing-subscriber built-in),
  `logback` (logback conversion words), `log4j` (PatternLayout)
- **Logback-style patterns** — `%d`, `%level`, `%logger{n}`, `%msg`,
  `%X{key}`, `%ex`, `%clr(...)`, `%highlight(...)`, 16 color words,
  format modifiers, abbreviation, etc.
- **Log4j-style patterns** — `%d`, `%p`, `%c{1.}`, `%m`, `%x`, `%X{key}`,
  `%enc{...}{html}`, `%maxLen{...}{n}`, throwable chain, etc.
- **Multiple appenders** — `stdout` / `stderr` / `file` / `rolling_file`
  can be combined arbitrarily, with global or per-appender sampling.
- **Span field capture** — `%X{request_id}` (and friends) read from
  `tracing` spans via a tiny `Layer`, not logback's MDC.
- **Cause-chain rendering** — `%ex`, `%rEx`, `%xEx` walk `std::error::Error::source()`
  and render each frame, with implicit append when the pattern omits `%ex`.
- **OpenTelemetry** — feature-gated OTLP trace export (HTTP/protobuf)
- **Sampling** — token-bucket rate limiting per appender
- **Windows ANSI** — virtual terminal processing is enabled
  automatically when initializing from `cfg(windows)`.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-declarative = "1.0"
```

Drop a `tracing.toml` next to your binary, then call `tracing_declarative::init()`
at the start of `main`. The library looks in:

1. `$TRACING_CONFIG` (if set)
2. `./tracing.toml`
3. `<exe-dir>/tracing.toml`

If none of those exist, `init` falls back to a built-in default
(`info` level, stdout, default formatter) — it never panics on a
missing config file.

## Logback conversion words

| Word | Description | Example |
| --- | --- | --- |
| `%d{pattern}` | Timestamp, Java SimpleDateFormat syntax | `%d{HH:mm:ss.SSS}` |
| `%level` | Log level (case as-is) | `INFO` |
| `%logger{n}` | Module path; `{n}` triggers abbreviation | `%logger{36}` |
| `%X{key}` | Span field by name (logback MDC) | `%X{request_id}` |
| `%msg` | Event message | |
| `%ex{depth}` | Error chain (walks `source()`) | `%ex{5}` |
| `%rEx` | Error chain with crate version info | |
| `%highlight(sub)` | Auto-color by level | `%highlight(%5level)` |
| `%clr(sub){color}` | Wrap in fixed ANSI color | `%clr(%msg){red}` |
| `%kvp` | Structured event key-value pairs | |
| `%pid` | Process ID | |

## Log4j conversion words

| Word | Description | Example |
| --- | --- | --- |
| `%d{pattern}` | Timestamp | `%d{HH:mm:ss.SSS}` |
| `%p` | Log level | `INFO` |
| `%c{precision}` | Logger name with dot-notation abbreviation | `%c{1.}` |
| `%m` | Event message | |
| `%x` | NDC (nearest span name) | |
| `%X{key}` | MDC (span field) | `%X{user_id}` |
| `%enc{sub}{html}` | HTML/XML/JSON/CRLF escaping | `%enc{%m}{html}` |
| `%maxLen{sub}{n}` | Truncate to n characters | `%maxLen{%m}{80}` |
| `%throwable` | Exception chain | |

## OpenTelemetry

Enable the `opentelemetry` feature and add an `[opentelemetry]` section:

```toml
[dependencies]
tracing-declarative = { version = "1.0", features = ["opentelemetry"] }
```

```toml
# tracing.toml
[opentelemetry]
enabled = true
endpoint = "http://localhost:4318/v1/traces"
service_name = "my-service"
service_version = "1.0.0"
```

## Sampling

Limit events per second across all appenders:

```toml
[sampling]
enabled = true
rate_per_second = 100
```

## Feature flags

| Feature | Default | Description |
|---------|---------|-------------|
| `hot-reload` | off | File-watch support (unstable — see docs) |
| `opentelemetry` | off | OTLP trace export |

## Limitations

- `hot-reload` is gated by the `hot-reload` feature and is currently
  unstable; `tracing` does not support re-initializing the global
  dispatcher. See `docs/ROADMAP.md` for the plan to rebase it on
  `tracing-subscriber::reload::Layer`.
- Per-appender formatters share the first appender's format string today.
  Per-appender formatters are tracked in `docs/ROADMAP.md`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.

[`tracing`]: https://docs.rs/tracing
