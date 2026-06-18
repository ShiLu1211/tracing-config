# tracing-declarative 性能优化规划

> 最后更新：2026-06-17 · 目标：将 logback/log4j 格式化开销从 ~3x 缩小到 ~1.5x vs tracing default

---

## 现状基线

| 配置 | 中位耗时 | vs tracing compact |
|------|----------|-------------------|
| tracing default compact | 3.13 µs | 1.0x |
| declarative default | 3.23 µs | 1.03x |
| declarative logback simple | 9.77 µs | 3.12x |
| declarative logback full | 11.97 µs | 3.82x |

**目标**：logback simple 从 9.77µs → ~5µs（~1.6x），logback full 从 11.97µs → ~7µs（~2.2x）

---

## 瓶颈分析

基于代码级分析，benchmark pattern `%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%n` 的每 event 开销分布：

| 热点 | 耗时估算 | 占比 | 根因 |
|------|----------|------|------|
| `chrono::Local::now()` + 格式化 | ~900 ns | 9% | 系统调用 + 时区计算 |
| `render_keyword` 返回 String（9-12 次分配） | ~800 ns | 8% | 中间 String 分配 + 拷贝 |
| `FormatModifier::apply()` 分配 | ~150 ns | 2% | padding String + format! 组合 |
| `EventData::collect()` + `strip_message_quotes` | ~300 ns | 3% | EventVisitor 分配 + 无条件 String 构造 |
| `abbreviate()` (Vec + String) | ~200 ns | 2% | split().collect() + String 拼接 |
| tracing 框架本身（subscriber 分发等） | ~7.4 µs | 76% | 不可优化 |
| **总计** | **~9.77 µs** | | |

> 注：tracing 框架的 ~7.4µs 是所有配置共享的基线（subscriber 分发 + span 查找 + writer 同步）。
> 可优化空间主要是我们自己引入的 ~2.4µs 额外开销。

---

## 优化方案

### O1: 消除中间 String — render_keyword 直接写入 Writer 🔴

**问题**：`render_keyword()` 返回 `String`，每次调用分配堆内存。tracing default 直接写 `fmt::Write`，零分配。

**方案**：重构 `render_keyword` 签名为：

```rust
// 之前
fn render_keyword(&self, keyword: &Keyword, ...) -> String

// 之后
fn render_keyword(&self, keyword: &Keyword, ..., writer: &mut dyn fmt::Write) -> fmt::Result
```

所有 keyword 分支直接 `write!` 到 writer，不构造中间 String。

**影响范围**：
- `src/formatter/logback/renderer.rs` — `render_keyword()`, `render_token()`
- `src/formatter/log4j/renderer.rs` — 同上
- 子 pattern 渲染（`%highlight(sub)`, `%clr(sub){color}`）需要写入临时 buf，但仅限含子 pattern 的 token

**预期收益**：消除 9-12 次 String 分配，节省 ~800ns

**复杂度**：中 — 需重构两个 renderer 的核心循环，但逻辑不变

---

### O2: FormatModifier 直接写入 Writer 🔴

**问题**：`apply(&self, s: &str) -> String` 始终分配新 String，即使无需修改（如 `%-5level` 对 "DEBUG" 已满 5 字符）。

**方案**：新增 `apply_to_writer` 方法：

```rust
// 新增
fn apply_to_writer(&self, s: &str, writer: &mut dyn fmt::Write) -> fmt::Result {
    let truncated = self.truncate(s);  // 截断是 &str 切片，零分配
    let padding = self.min_width.map_or(0, |min| min.saturating_sub(truncated.len()));
    if self.left_align {
        writer.write_str(truncated)?;
        for _ in 0..padding { writer.write_char(' ')?; }
    } else {
        for _ in 0..padding { writer.write_char(' ')?; }
        writer.write_str(truncated)?;
    }
    Ok(())
}
```

**预期收益**：消除 2-3 次 String 分配/keyword，节省 ~150ns

**复杂度**：低 — 纯新增方法，不影响现有 `apply()`

---

### O3: 时间戳缓存 🔴

**问题**：每 event 调用 `chrono::Local::now().format(fmt).to_string()`，包含：
1. 系统调用 `gettimeofday` + 时区查找（~500ns）
2. 格式化到 String（~300ns）

**方案**：在 `LogbackFormatter` / `Log4jFormatter` 中缓存秒级时间戳：

```rust
struct TimestampCache {
    last_sec: i64,           // Unix 秒
    formatted_prefix: String, // 已格式化的秒前缀 "2026-06-17 15:30:42"
}

impl TimestampCache {
    fn format(&mut self, chrono_fmt: &str) -> String {
        let now = chrono::Local::now();
        let sec = now.timestamp();
        if sec != self.last_sec {
            self.last_sec = sec;
            self.formatted_prefix = now.format(秒级格式).to_string();
        }
        // 仅格式化毫秒部分并拼接
        format!("{}.{}", self.formatted_prefix, now.format("%.3f"))
    }
}
```

**更简方案**（首选）：使用 `std::time::Instant` 作为快时钟，仅在 Instant 差 > 1ms 时重新格式化：

```rust
struct TimestampCache {
    instant: Instant,
    formatted: String,
}

fn get_timestamp(&mut self, chrono_fmt: &str) -> &str {
    if self.instant.elapsed() >= Duration::from_millis(1) {
        self.instant = Instant::now();
        self.formatted = chrono::Local::now().format(chrono_fmt).to_string();
    }
    &self.formatted
}
```

> **精度权衡**：毫秒级缓存意味着同一毫秒内的所有 event 共享相同时间戳。这对日志场景完全可接受（日志本身就是毫秒精度）。

**预期收益**：连续 event 间节省 ~600-900ns（仅首 event 付费），高频场景收益最大

**复杂度**：低 — 新增 struct + 缓存逻辑

---

### O4: EventData 零拷贝优化 🟡

**问题 A**：`strip_message_quotes()` 无条件分配新 String（即使无引号）。

**方案**：返回 `Cow<str>`：

```rust
fn strip_message_quotes(s: &str) -> Cow<str> {
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        Cow::Owned(s[1..s.len()-1].to_string())
    } else {
        Cow::Borrowed(s)  // 零分配
    }
}
```

**问题 B**：`EventData` 中 `message: String` 在 `Keyword::Message` 时被 `.clone()`。

**方案**：`render_keyword` 直接写入 writer 后，不再需要 clone。配合 O1 实现后自然消除。

**问题 C**：`fields: Vec<(String, String)>` 为每个字段分配 2 个 String。

**方案**（可选，收益较小）：使用 `smallvec::SmallVec<[(String, String); 4]>` 或固定大小栈数组，避免大多数场景下 Vec 的堆分配。

**预期收益**：O4A 节省 ~100ns（消除 strip 分配），O4B 节省 ~80ns（消除 message clone），O4C 节省 ~50ns

**复杂度**：O4A 低，O4B 依赖 O1，O4C 中

---

### O5: abbreviate() 零分配 🟡

**问题**：
- logback: `name.split("::").collect::<Vec<_>>()` + `String::new()` + `format!` — 3 次分配
- log4j: `target.replace("::", ".")` + `split('.').collect::<Vec<_>>()` + `String::new()` — 3 次分配

**方案**：

**logback**：直接迭代 `split("::")`，写入 writer：

```rust
fn abbreviate_to_writer(target: &str, n: usize, writer: &mut dyn fmt::Write) -> fmt::Result {
    let mut segments = target.split("::").peekable();
    let last = segments.next_back().unwrap_or("");
    // 计算需要缩写的段数...
    for seg in segments {
        write!(writer, "{}.", &seg[..1])?;
    }
    write!(writer, "{}", last)?;
    Ok(())
}
```

**log4j**：去掉 `replace("::", ".")`，改为同时按 `::` 和 `.` 分割：

```rust
fn abbreviate_to_writer(target: &str, depth: usize, writer: &mut dyn fmt::Write) -> fmt::Result {
    // 用自定义迭代器同时按 :: 和 . 分割
    for (i, seg) in SplitDelimiters::new(target).enumerate() {
        if i > 0 { write!(writer, ".")?; }
        write!(writer, "{}", seg)?;
    }
    Ok(())
}
```

**预期收益**：节省 ~200ns（消除 Vec + String 分配）

**复杂度**：中 — 需为 log4j 实现自定义分割迭代器

---

### O6: 移除冗余 .to_string() 🟢

**问题**：color 渲染中 `highlight()` 已返回 `String`，renderer 又调用 `.to_string()` 再克隆一次。

**方案**：直接使用 `highlight()` 返回的 String，移除冗余 `.to_string()`。

**影响文件**：
- `src/formatter/logback/renderer.rs` — `Keyword::Highlight`, `Keyword::Clr`, `Keyword::ColorWord` 分支

**预期收益**：仅影响含颜色 pattern，每个颜色词节省 ~50ns

**复杂度**：极低 — 删除 `.to_string()` 调用

---

### O7: thread::current() 缓存 🟢

**问题**：每 event 调用 `std::thread::current().name().unwrap_or("unknown").to_string()`

**方案**：使用 `thread_local!` 缓存线程名：

```rust
thread_local! {
    static THREAD_NAME: String = std::thread::current()
        .name()
        .unwrap_or("unknown")
        .to_string();
}

fn get_thread_name() -> &'static str {
    THREAD_NAME.with(|n| n.as_str())
    // 注意：这返回的是 thread_local 内部 String 的引用，
    // 生命周期安全但不是 'static — 需要调整 API 或使用 with() 闭包模式
}
```

> **注意**：`thread_local!` 的 `with()` 返回的引用不能逃逸闭包。需要配合 O1 在 `with` 闭包内直接写入 writer。

**预期收益**：节省 ~50-80ns/event（避免 thread::current() + to_string()）

**复杂度**：低

---

### O8: process ID 缓存 🟢

**问题**：`%pid` 每 event 调用 `std::process::id().to_string()`

**方案**：在 `LogbackFormatter::new()` 时一次性计算并缓存：

```rust
struct LogbackFormatter {
    // ... 现有字段
    pid: String,  // 新增
}

impl LogbackFormatter {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            // ...
            pid: std::process::id().to_string(),
        }
    }
}
```

**预期收益**：仅影响含 `%pid` 的 pattern，节省 ~30ns/event

**复杂度**：极低

---

## 实施计划

### Phase 1：核心重构（预期收益最大）

> 目标：logback simple 9.77µs → ~6µs（~1.9x）

| 优化 | 预期节省 | 复杂度 | 依赖 |
|------|----------|--------|------|
| O1 render_keyword 直接写入 | ~800 ns | 中 | 无 |
| O2 FormatModifier 直接写入 | ~150 ns | 低 | O1 |
| O3 时间戳缓存 | ~600 ns (连续 event) | 低 | 无 |

**实施顺序**：O1 → O2 → O3

O1 是基础重构，O2 依赖 O1 的新签名，O3 独立可并行。

### Phase 2：细节优化（边际收益）

> 目标：logback simple ~6µs → ~5µs（~1.6x）

| 优化 | 预期节省 | 复杂度 | 依赖 |
|------|----------|--------|------|
| O4A strip_message_quotes → Cow | ~100 ns | 低 | 无 |
| O4B message 消除 clone | ~80 ns | 低 | O1 |
| O5 abbreviate 零分配 | ~200 ns | 中 | O1 |
| O6 移除冗余 to_string | ~50 ns | 极低 | 无 |
| O7 thread name 缓存 | ~80 ns | 低 | O1 |
| O8 PID 缓存 | ~30 ns | 极低 | 无 |

### Phase 3：高级优化（可选）

| 优化 | 说明 | 预期收益 |
|------|------|----------|
| 预编译 pattern 为渲染闭包 | 将 token tree 编译为闭包链，消除 match 开销 | ~200-300ns |
| `time` crate 替代 `chrono` | UTC 格式化快 ~2x | ~200ns（仅 UTC） |
| 多 appender 共享格式化 | format once, write N times | ~50% (N≥2) |

---

## 预期效果

### Phase 1 后

| 配置 | 当前 | Phase 1 后 | vs tracing compact |
|------|------|-----------|-------------------|
| logback simple | 9.77 µs | ~6.2 µs | ~2.0x |
| logback full | 11.97 µs | ~8.2 µs | ~2.6x |
| log4j simple | 9.67 µs | ~6.1 µs | ~2.0x |
| log4j full | 11.55 µs | ~7.8 µs | ~2.5x |

### Phase 1 + 2 后

| 配置 | 当前 | 全部优化后 | vs tracing compact |
|------|------|-----------|-------------------|
| logback simple | 9.77 µs | ~5.2 µs | ~1.7x |
| logback full | 11.97 µs | ~7.2 µs | ~2.3x |
| log4j simple | 9.67 µs | ~5.1 µs | ~1.6x |
| log4j full | 11.55 µs | ~6.8 µs | ~2.2x |

> **剩余差距来源**：tracing 框架基线 ~3.1µs 中的部分差异来自 `SpanFieldsLayer`（额外 layer 开销）和 `%d` 的 chrono 格式化（即使缓存，首 event 仍需完整格式化）。这些是架构性差异，难以完全消除。

---

## 风险

| 风险 | 缓解 |
|------|------|
| O1 重构范围大，可能引入回归 | 先写 benchmark 基线，重构后立即验证 |
| 时间戳缓存引入精度问题 | 默认 1ms 缓存粒度，可配置；日志场景毫秒精度足够 |
| `render_keyword` 写入 writer 需处理 `fmt::Error` | `fmt::Write` 的 `write_str` 返回 `fmt::Result`，需在 token 循环中传播错误 |
| 子 pattern 渲染（highlight/clr）仍需 String | 保留 `render_token_string` 用于子 pattern，主路径用 writer |
| log4j 自定义分割迭代器复杂度 | 可先保留 Vec 收集，后续优化 |
