# tracing-declarative Benchmark 规划

> 最后更新：2026-06-17

## 目标

对比 `tracing-subscriber` 默认配置与 `tracing-declarative` 各种配置下的端到端日志格式化性能，量化声明式配置的额外开销。

---

## 现有 Benchmark

当前 `benches/formatter.rs` 只覆盖内部组件：

| Benchmark | 耗时 | 说明 |
|-----------|------|------|
| `logback_lexer_scan` | ~5.3 µs | 一次 logback pattern 词法扫描 |
| `log4j_lexer_scan` | ~5.7 µs | 一次 log4j pattern 词法扫描 |
| `preprocessed_chrono` | ~590 ns | 预处理后的 chrono 日期格式化 |
| `convert_pattern_per_call` | ~1.5 µs | 每次调用 convert_pattern（未优化路径） |
| `logback_abbreviate_20` | ~250 ns | logback 缩写算法（n=20） |
| `log4j_abbreviate_2` | ~355 ns | log4j 缩写算法（depth=2） |

**缺失**：端到端格式化（FormatEvent 调用）、与 tracing-subscriber 默认 fmt 的对比、多 appender 开销、sampling 开销。

---

## 新增 Benchmark 设计

### B1: 端到端格式化对比（核心）

对比四种配置下单条 event 的格式化耗时：

| 编号 | 名称 | 配置 | 测量重点 |
|------|------|------|----------|
| B1.1 | `tracing_default_compact` | `fmt().compact()` | 基线：tracing-subscriber 内置 compact |
| B1.2 | `tracing_default_full` | `fmt()` | 基线：tracing-subscriber 内置 full format |
| B1.3 | `declarative_default` | `type = "default"`, `format = "compact"` | 声明式默认 formatter 的额外开销 |
| B1.4 | `declarative_logback_simple` | `pattern = "%d [%t] %-5level %logger{36} - %msg%n"` | logback 简单 pattern |
| B1.5 | `declarative_logback_full` | `pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%thread] %-5level %logger{36} - %msg%ex%n"` | logback 含日期+异常 |
| B1.6 | `declarative_log4j_simple` | `pattern = "%d [%t] %-5p %c{1.} - %m%n"` | log4j 简单 pattern |
| B1.7 | `declarative_log4j_full` | `pattern = "%d{yyyy-MM-dd HH:mm:ss.SSS} [%t] %-5p %c{1.} - %m%ex%n"` | log4j 含日期+异常 |

**实现方式**：构造 `tracing::Event`，直接调用 `FormatEvent::format_event()`，写入 `Vec<u8>`。不经过 subscriber 分发，纯测格式化。

**预期**：
- B1.1 / B1.2 是纯基线，无任何额外开销
- B1.3 应接近 B1.1（委托给 fmt::Layer）
- B1.4 / B1.6 简单 pattern 应在基线 2-5x 范围内
- B1.5 / B1.7 含日期格式化，开销主要来自 chrono

### B2: 日期格式化开销

| 编号 | 名称 | 说明 |
|------|------|------|
| B2.1 | `chrono_local_now_format` | `chrono::Local::now().format(preprocessed)` |
| B2.2 | `chrono_utc_now_format` | `chrono::Utc::now().format(preprocessed)` |
| B2.3 | `chrono_offset_now_format` | `chrono::Local::now().format_with_offset()` |
| B2.4 | `std_system_time` | `std::time::SystemTime::now()` 格式化（不含 chrono） |

**目的**：量化日期格式化在总耗时中的占比，评估用 `time` crate 替代 `chrono` 的潜在收益。

### B3: 颜色开销

| 编号 | 名称 | 说明 |
|------|------|------|
| B3.1 | `logback_no_color` | 无颜色 pattern |
| B3.2 | `logback_highlight` | 含 `%highlight(...)` |
| B3.3 | `logback_clr` | 含 `%clr(...){red}` |
| B3.4 | `logback_color_word` | 含 `%red(...)` |

**目的**：量化 ANSI 颜色包裹的额外开销（预期极小，仅为字符串拼接）。

### B4: 多 Appender 开销

| 编号 | 名称 | 说明 |
|------|------|------|
| B4.1 | `single_stdout` | 单 stdout appender |
| B4.2 | `dual_stdout_file` | stdout + file 双 appender |
| B4.3 | `triple_stdout_file_stderr` | stdout + file + stderr 三 appender |

**实现方式**：构造 `MultiMakeWriter`，测量 `MakeWriter::make_writer()` + 写入耗时。

**目的**：量化 fan-out 的线性增长是否可接受。

### B5: Sampling 开销

| 编号 | 名称 | 说明 |
|------|------|------|
| B5.1 | `no_sampling` | 无 sampling |
| B5.2 | `sampling_1000` | rate_per_second = 1000 |
| B5.3 | `sampling_100` | rate_per_second = 100 |
| B5.4 | `sampling_0_rejected` | rate = 0，全部被限流 |

**目的**：量化 token-bucket 检查的额外开销（预期 < 10ns）。

### B6: 配置解析开销

| 编号 | 名称 | 说明 |
|------|------|------|
| B6.1 | `parse_default_toml` | 解析最小 toml 配置 |
| B6.2 | `parse_full_toml` | 解析含 5 appender 的完整配置 |
| B6.3 | `init_from_str_default` | `init_from_str()` 完整初始化（含 subscriber 注册） |
| B6.4 | `init_from_str_logback` | `init_from_str()` logback 配置初始化 |

**目的**：量化启动时的一次性开销，与运行时性能无关，但影响冷启动时间。

---

## 实现方案

### 文件结构

```
benches/
├── formatter.rs          # 现有：lexer/date/abbreviator 微观 benchmark（保留）
└── e2e.rs                # 新增：端到端格式化 benchmark（B1-B6）
```

### 关键实现细节

**B1 的 FormatEvent 调用**：

```rust
use tracing_subscriber::fmt::FormatEvent;
use tracing_core::{Event, Field, Metadata};
use std::io::Cursor;

// 构造一个最小 event
fn make_test_event() -> Event {
    // 使用 tracing_core 的 metadata + field set
    // 或更简单：通过 tracing::info! 宏在 bench 函数中触发
}
```

**问题**：`tracing::Event` 的构造需要 `tracing_core::FieldSet` 和 `Metadata`，且 `FormatEvent` 需要 `tracing_subscriber::fmt::FmtContext`。直接构造较复杂。

**替代方案**：使用 `tracing_subscriber::subscribe()` + `tracing::info!()` 宏触发，写入 `Vec<u8>` sink。但 criterion 要求可重复调用，subscriber 注册是全局的。

**推荐方案**：

1. 对 B1.1 / B1.2：直接调用 `fmt::Layer` 的 `on_event()` 方法
2. 对 B1.4-B1.7：直接调用 `LogbackFormatter::format_event()` / `Log4jFormatter::format_event()`
3. 手动构造 `tracing_core::Event` + `FmtContext`

需要验证 `FormatEvent` trait 的调用是否可以在不注册全局 subscriber 的情况下完成。如果不行，使用 `MakeWriter` 返回 `Vec<u8>` 的 writer，每次 bench 迭代创建新的 subscriber（仅含单 layer），通过 `subscriber.enter()` 上下文触发格式化。

### 测量指标

| 指标 | 说明 |
|------|------|
| `time` | 单次操作的 wall-clock 耗时 |
| `throughput` | 每秒可处理 event 数 |

criterion 默认输出两者。

### 环境要求

- Rust stable（criterion 兼容）
- 关闭 CPU 频率调节：`sudo cpupower frequency-set -g performance`（可选，结果更稳定）
- 多次运行取中位数

---

## 输出格式

### 文档输出

benchmark 结果写入 `docs/BENCHMARK.md`，包含：

1. **测试环境**：CPU / OS / Rust 版本 / 编译参数
2. **结果表格**：每个 benchmark 的中位耗时 + 吞吐量
3. **对比图表**：tracing-default vs declarative 各配置的柱状图（criterion HTML 报告）
4. **分析结论**：开销来源、优化建议

### 示例结果表格格式

```
### B1: 端到端格式化

| 配置 | 中位耗时 | vs tracing compact | 吞吐量 |
|------|----------|-------------------|--------|
| tracing default compact | XXX ns | 1.0x (基线) | X M evt/s |
| tracing default full | XXX ns | X.Xx | X M evt/s |
| declarative default | XXX ns | X.Xx | X M evt/s |
| declarative logback simple | XXX ns | X.Xx | X M evt/s |
| declarative logback full | XXX ns | X.Xx | X M evt/s |
| declarative log4j simple | XXX ns | X.Xx | X M evt/s |
| declarative log4j full | XXX ns | X.Xx | X M evt/s |
```

---

## 执行计划

| 步骤 | 任务 | 依赖 |
|------|------|------|
| 1 | 实现 B1（端到端格式化对比） | 需验证 FormatEvent 直接调用可行性 |
| 2 | 实现 B2-B3（日期 + 颜色开销） | 无 |
| 3 | 实现 B4-B5（多 appender + sampling） | 需构造 MultiMakeWriter |
| 4 | 实现 B6（配置解析开销） | 无 |
| 5 | 运行完整 benchmark，采集数据 | 步骤 1-4 |
| 6 | 生成 `docs/BENCHMARK.md` | 步骤 5 |
| 7 | 更新 `docs/ROADMAP.md` M5 里程碑 | 步骤 6 |

---

## 风险

| 风险 | 缓解 |
|------|------|
| `FormatEvent` 无法在无全局 subscriber 下调用 | 使用 `subscriber::enter()` 临时上下文，或改用 `on_event()` 直接调用 |
| criterion 与全局 subscriber 冲突 | 每次迭代创建独立 subscriber + MakeWriter<Vec<u8>> |
| 日期格式化占主导，掩盖其他开销 | 单独报告 B2 日期开销，从 B1 中剥离分析 |
| 环境差异导致结果不可复现 | 报告中注明 CPU/OS/Rust 版本，使用 criterion 统计分析 |
