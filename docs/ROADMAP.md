# tracing-declarative 路线图

> 最后更新：2026-06-17 · 当前版本：1.0.0 · 状态：可发布

本文档基于当前代码库的实际完成状态，规划后续开发优先级和里程碑。

---

## 当前状态总览

| 模块 | 状态 | 说明 |
|------|------|------|
| TOML 配置解析 | ✅ 完成 | Config / Global / Filter / Appender / Formatter / Sampling / OpenTelemetry 结构体 |
| 默认 Formatter | ✅ 完成 | 委托 tracing-subscriber fmt::Layer |
| Logback Lexer | ✅ 完成 | 全部 Keyword 解析，含嵌套 sub-pattern |
| Logback 对齐/截断 | ✅ 完成 | FormatModifier，行为对齐 logback |
| Logback 缩写 | ✅ 完成 | abbreviate() 对齐 TargetLengthBasedClassNameAbbreviator |
| Logback 日期 | ✅ 完成 | Java SimpleDateFormat → chrono strftime，构造时预处理 |
| Logback 颜色 | ✅ 完成 | %highlight / %clr / %ColorWord，16 个 color words |
| Log4j 引擎 | ✅ 完成 | formatter/log4j/ 完整实现，%c{1.} / %enc / %maxLen / 颜色词 / throwable chain |
| Appender: stdout/stderr/file/rolling | ✅ 完成 | 四种 appender 均可用 |
| 多 Appender | ✅ 完成 | MultiMakeWriter + MultiWriter 扇出，支持任意组合 |
| Per-appender level filter | ✅ 完成 | 每个 appender 可独立设置日志级别 |
| Sampling | ✅ 完成 | SamplingWriter + RateLimiter 令牌桶限流 |
| OpenTelemetry 集成 | ✅ 完成 | OTLP HTTP/protobuf exporter，feature-gated |
| Hot Reload | ⚠️ Unstable | 文件监控可用，但 tracing 全局 dispatcher 不可重新初始化 |
| %ex / %rEx / %xEx 异常渲染 | ✅ 完成 | error_chain 逐帧存储，深度限制，隐式追加逻辑 |
| %X{key} / %mdc | ✅ 完成 | SpanFieldsLayer + SpanFieldStore |
| %kvp / %marker | ✅ 完成 | EventVisitor 收集 + marker 字段提取 |
| Windows ANSI | ✅ 完成 | init 入口自动调用 |
| 性能优化 | ✅ 完成 | %d 预处理(2.5x)、has_exception 缓存、直接 writer 写入 |
| API 稳定化 | ✅ 完成 | #![deny(missing_docs)]、deprecated 标记、clippy 零警告 |
| Criterion 基准测试 | ✅ 完成 | benches/formatter.rs |
| 发版准备 | ✅ 完成 | 包名 / 元数据 / LICENSE / README / CHANGELOG 均已就绪 |

---

## 里程碑完成记录

### M1: v0.2.0 — 核心功能补全 ✅

- [x] M1.1 异常渲染：`%ex` / `%rEx` / `%xEx`，含深度限制和隐式追加
- [x] M1.2 `%clr(sub){color}` 后置 option 解析 + 16 个 color words
- [x] M1.3 `%X{key}` / `%mdc` — SpanFieldsLayer + SpanFieldStore
- [x] M1.4 `%kvp` — EventVisitor 一次性收集非 message 字段
- [x] M1.5 多 Appender 泛化 — MultiMakeWriter + MultiWriter + Box::leak 闭包模式
- [x] M1.6 Windows ANSI — init 入口自动调用

### M2: v0.3.0 — 质量与稳定性 ✅

- [x] M2.1 Renderer 去重 — render_token_string 单一来源
- [x] M2.2 配置文件未找到回退 — Config::builtin_default()
- [x] M2.3 搜索路径补全 — 含 `<exe-dir>/tracing.toml`
- [x] M2.4 补齐测试 — logback_render / init_multi / end_to_end / config_fallback / log4j_render + fixture
- [x] M2.5 README.md + CHANGELOG.md
- [x] M2.6 Hot Reload 标记为 unstable + 文档说明

### M3: v0.4.0 — 扩展引擎 ✅

- [x] M3.1 Log4j PatternLayout 引擎 — %c{1.} dot-notation、%x NDC、%enc{}、%maxLen{}、颜色词、throwable chain
- [x] M3.2 OpenTelemetry 集成 — feature-gated，HTTP/protobuf OTLP exporter，OtelGuard

### M4: v1.0.0 — 生产就绪 ✅

- [x] M4.1 API 审查与稳定化 — #![deny(missing_docs)]、try_init_from_str deprecated、ConfigError 变体审查、clippy 零警告
- [x] M4.2 性能优化 — %d 预处理(2.5x)、has_exception/has_nopex 缓存、Literal 直接写入 writer、criterion benchmarks
- [x] M4.3 完整文档 — crate-level docs、所有 pub API 代码示例(20 doctests)、配置参考 TOML 示例、模块级架构说明

### M5: v1.0.0 发布 — 发版准备 ✅

- [x] M5.1 包名 `tracing-declarative` 确认（crates.io 未被占用）
- [x] M5.2 Cargo.toml 元数据补全（description/license/repository/documentation/readme/keywords/categories/rust-version）
- [x] M5.3 添加 LICENSE-MIT + LICENSE-APACHE 文件
- [x] M5.4 README.md 更新（log4j 引擎、OTel、Sampling、Feature flags）
- [x] M5.5 hot-reload 从 default feature 移出（`default = []`）
- [x] M5.6 `cargo publish --dry-run` 通过

---

## 技术债清单

| 编号 | 问题 | 状态 | 备注 |
|------|------|------|------|
| D1 | Renderer 双份代码 | ✅ 已解决 | render_token_string 单一来源 |
| D2 | hot_reload.rs 基于 reload::Layer 重构 | ⚠️ Unstable | 当前文件监控可用，但无法重初始化 subscriber |
| D3 | appender/mod.rs 为空 | ℹ️ 低优先级 | 逻辑在 lib.rs，不影响功能 |
| D4 | SamplingWriter rate=0 封装开销 | ℹ️ 低优先级 | 热路径上开销极小 |
| D5 | `%d` 每次事件重新格式化 | ✅ 已解决 | 构造时预处理 Java→chrono |
| D6 | message 提取用 trim + strip_prefix hack | ✅ 已解决 | EventVisitor 改进 |
| D7 | Per-appender formatter（非共享第一个） | 🟡 v1.1 | 当前所有 appender 共享第一个 formatter |
| D8 | `tracing-test` / `strip-ansi-escapes` 未在 Cargo.toml | ✅ 已解决 | 已调整 |
| D9 | `try_init_from_str` 与 `init_from_str` 行为一致 | ✅ 已解决 | deprecated 标记 |
| D10 | 包名与 crates.io 已有 crate 冲突 | ✅ 已解决 | 重命名为 `tracing-declarative` |
| D11 | Cargo.toml 缺少 crates.io 必需元数据 | ✅ 已解决 | 全部补齐 |
| D12 | 缺少 LICENSE 文件 | ✅ 已解决 | 双文件已添加 |
| D13 | README.md 过时描述 | ✅ 已解决 | 已更新 log4j/otel/sampling |
| D14 | hot-reload 作为 default feature | ✅ 已解决 | `default = []` |

---

## 后续里程碑规划

### M6: v1.1.0 — 实用性增强

| 编号 | 功能 | 优先级 | 复杂度 | 说明 |
|------|------|--------|--------|------|
| M6.1 | Per-appender formatter | 🔴 高 | 中 | 每个 appender 独立格式化，需重构 `build_factory` |
| M6.2 | Size-based rolling file rotation | 🟡 中 | 低 | `max_size` / `max_files` 字段已预留 |
| M6.3 | Hot Reload 基于 `reload::Layer` | 🟡 中 | 高 | 需重新设计架构，替换当前 unstable 实现 |

### M7: v1.2.0 — 开发体验

| 编号 | 功能 | 优先级 | 复杂度 | 说明 |
|------|------|--------|--------|------|
| M7.1 | YAML 配置支持 | 🟡 中 | 中 | `serde_yaml` feature，Config 已是 serde 结构体 |
| M7.2 | 环境变量插值 | 🟡 中 | 中 | `${ENV_VAR}` 在 TOML 值中展开 |
| M7.3 | 配置校验增强 | 🟢 低 | 低 | 更友好的错误信息 + 字段路径提示 |

### M8: v1.3.0 — 高级功能

| 编号 | 功能 | 优先级 | 复杂度 | 说明 |
|------|------|--------|--------|------|
| M8.1 | `%M` 真实函数名 | 🟡 中 | 中 | 需 proc-macro `#[tracing_name]` 配合 |
| M8.2 | `%rEx` crate 版本注入 | 🟢 低 | 低 | build.rs 已有基础，需扩展到依赖链 |
| M8.3 | 自定义 appender 注册 | 🟢 低 | 高 | 允许用户注册 `Box<dyn MakeWriter>` 工厂 |

---

## 与已有 `tracing-config` crate 的差异

crates.io 上的 [tracing-config v0.2.2](https://github.com/mateiandrei94/tracing-config) 功能相似但实现不同：

| 维度 | tracing-config (已有) | tracing-declarative (本项目) |
|------|----------------------|------------------------------|
| 配置格式 | TOML | TOML（可扩展 YAML） |
| Formatter 引擎 | 内置简单格式 | logback + log4j 双引擎，完整 conversion word |
| Pattern 语法 | 无 | logback conversion words / log4j PatternLayout |
| 多 Appender | 有限 | 任意组合 + per-appender level filter |
| 颜色 | 无 | %highlight / %clr / 16 color words |
| 异常链 | 无 | %ex / %rEx / %xEx，深度限制，隐式追加 |
| Span 字段 | 无 | %X{key} / %mdc / %kvp |
| OpenTelemetry | 无 | feature-gated OTLP 集成 |
| Sampling | 无 | 令牌桶限流 |
| 性能优化 | — | %d 预处理 2.5x，缓存优化 |
