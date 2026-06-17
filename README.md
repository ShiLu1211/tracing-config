# tracing-config

> Declarative `tracing` initialization via `tracing.toml`.

`tracing-config` lets you configure [`tracing`]'s subscriber entirely from a
`tracing.toml` file — no hard-coded formatters, no manual `Layer` plumbing,
no copy-pasted `EnvFilter` directives. Drop a file next to your binary,
pick the appenders and logback-style pattern you want, and you're done.

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
    tracing_config::init().expect("tracing init");
    tracing::info!("hello {}", "world");
}
```

## Highlights

- **`logback`-style patterns** — `%d`, `%level`, `%logger{n}`, `%msg`,
  `%X{key}`, `%ex`, `%clr(...)`, `%highlight(...)`, format modifiers, etc.
- **Multiple appenders** — `stdout` / `stderr` / `file` / `rolling_file`
  can be combined arbitrarily, with global or per-appender sampling.
- **Span field capture** — `%X{request_id}` (and friends) read from
  `tracing` spans via a tiny `Layer`, not logback's MDC.
- **Cause-chain rendering** — `%ex`, `%rEx`, `%xEx` walk `std::error::Error::source()`
  and render each frame, with implicit append when the pattern omits `%ex`.
- **Windows ANSI** — virtual terminal processing is enabled
  automatically when initializing from `cfg(windows)`.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
tracing = "0.1"
tracing-config = "0.1"
```

Drop a `tracing.toml` next to your binary, then call `tracing_config::init()`
at the start of `main`. The library looks in:

1. `$TRACING_CONFIG` (if set)
2. `./tracing.toml`
3. `<exe-dir>/tracing.toml`

If none of those exist, `init` falls back to a built-in default
(`info` level, stdout, default formatter) — it never panics on a
missing config file.

## Conversion words

The full set of supported logback conversion words is documented in
`tracing.toml` next to this file. Highlights:

| Word | Description | Example |
| --- | --- | --- |
| `%d{pattern}` | Timestamp, Java SimpleDateFormat syntax | `%d{HH:mm:ss.SSS}` |
| `%level` | Log level (case as-is) | `INFO` |
| `%logger{n}` | Module path; `{n}` triggers abbreviation | `%logger{36}` |
| `%X{key}` | Span field by name (logback MDC) | `%X{request_id}` |
| `%msg` | Event message | |
| `%ex{depth}` | Error chain (walks `source()`) | `%ex{5}` |
| `%highlight(sub)` | Auto-color by level | `%highlight(%5level)` |
| `%clr(sub){color}` | Wrap in fixed ANSI color | `%clr(%msg){red}` |
| `%pid` | Process ID | |

## Limitations

- `hot-reload` is currently gated by the `hot-reload` feature and is
  only a partial implementation; see `docs/ROADMAP.md` for the plan to
  rebase it on `tracing-subscriber::reload::Layer`.
- The `log4j` formatter engine is planned but not yet implemented.
- Per-appender formatters share the first appender's format string today.
  Per-appender formatters are tracked in `docs/ROADMAP.md`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.

[`tracing`]: https://docs.rs/tracing
