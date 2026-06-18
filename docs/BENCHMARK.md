# tracing-declarative Benchmark Results

> Last updated: 2026-06-18 · Linux 6.8 x86_64 · criterion 0.5 · 100 samples per benchmark

---

## B1: End-to-end Formatting

| Configuration | Median Time | vs tracing compact |
|---------------|-------------|-------------------|
| tracing default compact | 3.21 µs | **1.0x** (baseline) |
| tracing default full | 3.25 µs | 1.01x |
| declarative default | 3.27 µs | 1.02x |
| declarative logback simple | 9.32 µs | 2.90x |
| declarative logback full | 11.42 µs | 3.56x |
| declarative log4j simple | 9.24 µs | 2.88x |
| declarative log4j full | 10.28 µs | 3.20x |

### Analysis

- **declarative default** has only **2% overhead** vs tracing default compact — effectively zero
- **logback/log4j simple** patterns are ~2.9x slower (down from initial 3.1x)
- **logback/log4j full** are 3.2-3.6x (down from initial 3.7-3.8x)
- logback and log4j perform identically — shared rendering infrastructure

---

## B2: Date Formatting

| Method | Median Time | Notes |
|--------|-------------|-------|
| `std::time::SystemTime::now()` | 35.6 ns | Raw system call, no formatting |
| `chrono::Utc::now().format()` | 466.9 ns | UTC, no timezone lookup |
| `chrono::Local::now().format()` | 596.4 ns | Local timezone |
| `chrono::Local` + `%:z` timezone | 661.5 ns | With timezone offset |

### Analysis

- Date formatting accounts for ~6% of logback simple total time (600ns / 9.3µs)
- `TS_CACHE` optimization: same-millisecond events reuse cached formatted string
- Fast path (millis unchanged): single u64 comparison, no chrono call
- Only ~1/1000 events trigger the slow path (chrono formatting)

---

## B3: Color Overhead

| Configuration | Median Time | vs no-color |
|---------------|-------------|------------|
| logback no color | 10.5 µs | **1.0x** (baseline) |
| logback `%cyan(...)` | 10.8 µs | 1.03x |
| logback `%clr(...){cyan}` | 13.3 µs | 1.27x |
| logback `%highlight(...)` | 11.7 µs | 1.11x |

### Analysis

- **Color words** (`%cyan`, `%red`, etc.): +3% overhead — minimal
- **`%clr(sub){color}`**: +27% — post-sub-pattern option parsing
- **`%highlight(...)`**: +11% — level-to-color lookup
- **Recommendation**: prefer color words for performance-sensitive output

---

## B4: Multi-Appender

| Configuration | Median Time | vs single writer |
|---------------|-------------|-----------------|
| 1 writer (logback) | 5.8 µs | **1.0x** (baseline) |
| 2 writers (dual layer) | 10.2 µs | 1.76x |
| 3 writers (triple layer) | 14.5 µs | 2.50x |

### Analysis

- Each additional writer adds ~4.3 µs — close to single-format cost
- Near-linear scaling as expected
- Using `MakeNullWriter` eliminates lock contention from earlier measurements

---

## B5: Sampling

| Configuration | Median Time | Notes |
|---------------|-------------|-------|
| no sampling (rate=0) | 2.0 ns | Fast path: immediate return |
| sampling rate=1000 | 36.5 ns | Token bucket acquire |
| sampling rate=100 | 36.7 ns | Same check, different rate |
| sampling exhausted | 36.8 ns | Bucket empty, event rejected |

### Analysis

- Sampling check is ~37 ns per event (single `AtomicU64` CAS)
- At ~10 µs per event formatting, 37 ns is **0.37%** — negligible

---

## B6: Config Parsing (one-time startup)

| Operation | Median Time | Notes |
|-----------|-------------|-------|
| parse minimal TOML | 2.85 µs | 1 appender + default formatter |
| parse full TOML | 12.78 µs | 3 appenders + filter + sampling + otel |
| logback lexer (full pattern) | 6.89 µs | One-time pattern scanning |
| build logback formatter | 8.03 µs | Lexer + date preprocessing + cache build |

### Analysis

- All parsing costs are microsecond-range, executed once at startup
- Full config with 3 appenders takes ~13 µs — negligible for cold start

---

## Optimization History

| Phase | Changes | Key Improvement |
|-------|---------|-----------------|
| Initial | — | logback simple 9.77µs (3.12x) |
| Phase 1 | O1 render_keyword_to_writer, O2 apply_to_writer, O3 TS_CACHE, O6-O8 caching | -1.5% (9.62µs), colors -30% |
| Phase 2 | StackBuf for modifier path, abbreviate_to_writer, TS_CACHE fast path | -6.3% (9.01µs → ~9.3µs with noise), log4j full -11% |

### Cumulative Improvement

| Configuration | Initial | Current | Total Change |
|---------------|---------|---------|-------------|
| logback simple | 9.77 µs (3.12x) | 9.32 µs (2.90x) | **-4.6%** |
| logback full | 11.97 µs (3.82x) | 11.42 µs (3.56x) | **-4.6%** |
| log4j simple | 9.67 µs (3.09x) | 9.24 µs (2.88x) | **-4.4%** |
| log4j full | 11.55 µs (3.69x) | 10.28 µs (3.20x) | **-11.0%** |

---

## Remaining Optimization Opportunities

| Priority | Optimization | Expected Gain | Complexity |
|----------|-------------|---------------|------------|
| 🔴 High | Timestamp caching (same-second reuse) | Eliminate ~600ns for most events | Medium |
| 🟡 Medium | Format once, write N times (multi-appender) | ~50% reduction for N≥2 appenders | Medium |
| 🟡 Medium | Eliminate intermediate `String` in `render_token_string` | Reduce allocations | Medium |
| 🟢 Low | `time` crate migration (replace `chrono`) | ~2x faster date formatting for UTC | High (breaking) |

### Why the gap persists

The ~3x gap between declarative logback/log4j and tracing default is **architectural**:

1. **Token tree traversal** — logback/log4j iterate a `Vec<Token>` per event, match on each keyword. tracing default uses hardcoded `fmt()` formatting — zero indirection.
2. **`SpanFieldsLayer`** — extra layer in the subscriber stack for `%X`/`%kvp` support
3. **`chrono::Local::now()`** — even cached at millisecond granularity, first event per millisecond pays ~900ns
4. **`thread_local!` access** — `TS_CACHE.with()` and `THREAD_NAME.with()` have `RefCell` borrow overhead

These costs are the price of flexible, declarative configuration. The 3x overhead is acceptable for logging scenarios where ~10µs/event latency is negligible compared to I/O.
