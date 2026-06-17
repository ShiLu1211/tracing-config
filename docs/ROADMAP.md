# tracing-config 路线图

> 最后更新：2026-06-17 · 版本：0.1.0 → v1.0.0 完成

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
| Appender: stdout/stderr/file/rolling | ✅ 完成 | 四种 appender 均可用 |
| 多 Appender | ✅ 完成 | MultiMakeWriter + MultiWriter 扇出，支持任意组合 |
| Sampling | ✅ 完成 | SamplingWriter + RateLimiter，已接入 multi-appender |
| Hot Reload | ⚠️ Unstable | 标记为 unstable；tracing 全局 dispatcher 不可重新初始化 |
| %ex / %rEx / %xEx 异常渲染 | ✅ 完成 | error_chain 逐帧存储，深度限制，隐式追加逻辑 |
| %clr(sub){color} | ✅ 完成 | 后置 option 解析，16 个 color words |
| %X{key} / %mdc | ✅ 完成 | SpanFieldsLayer + SpanFieldStore |
| %kvp | ✅ 完成 | EventVisitor 一次性收集非 message 字段 |
| %marker | ✅ 完成 | 从 event `marker` 字段提取 |
| Log4j 引擎 | ✅ 完成 | formatter/log4j/ 完整实现 |
| OpenTelemetry 集成 | ✅ 完成 | OTLP HTTP/protobuf exporter，feature-gated |
| Windows ANSI | ✅ 完成 | init_with_config 入口自动调用 |
| 文档 / README | ✅ 完成 | README.md + CHANGELOG.md + 全面 rustdoc |
| 性能优化 | ✅ 完成 | %d 预处理(2.5x)、has_exception 缓存、直接 writer 写入 |
| API 稳定化 | ✅ 完成 | #![deny(missing_docs)]、deprecated 标记、clippy 零警告 |
| Criterion 基准测试 | ✅ 完成 | benches/formatter.rs |

---

## 里程碑完成记录

### M1: v0.2.0 — 核心功能补全 ✅

- [x] M1.1 异常渲染：`%ex` / `%rEx` / `%xEx`，含深度限制和隐式追加
- [x] M1.2 `%clr(sub){color}` 后置 option 解析 + 16 个 color words
- [x] M1.3 `%X{key}` / `%mdc` — SpanFieldsLayer + SpanFieldStore
- [x] M1.4 `%kvp` — EventVisitor 一次性收集非 message 字段
- [x] M1.5 多 Appender 泛化 — MultiMakeWriter + MultiWriter + Box::leak 闭包模式
- [x] M1.6 Windows ANSI — init_with_config 入口自动调用

### M2: v0.3.0 — 质量与稳定性 ✅

- [x] M2.1 Renderer 去重 — render_token_string 单一来源
- [x] M2.2 配置文件未找到回退 — Config::builtin_default()
- [x] M2.3 搜索路径补全 — 含 `<exe-dir>/tracing.toml`
- [x] M2.4 补齐测试 — logback_render(14) / init_multi(3) / end_to_end(5) / config_fallback(3) / log4j_render(6) + invalid.toml fixture
- [x] M2.5 README.md + CHANGELOG.md
- [x] M2.6 Hot Reload 标记为 unstable + 文档说明

### M3: v0.4.0 — 扩展引擎 ✅

- [x] M3.1 Log4j PatternLayout 引擎 — %c{1.} dot-notation、%x NDC、%enc{}、%maxLen{}、颜色词、throwable chain
- [x] M3.2 OpenTelemetry 集成 — feature-gated，HTTP/protobuf OTLP exporter，OtelGuard

### M4: v1.0.0 — 生产就绪 ✅

- [x] M4.1 API 审查与稳定化 — #![deny(missing_docs)]、try_init_from_str deprecated、ConfigError 变体审查、clippy 零警告
- [x] M4.2 性能优化 — %d 预处理(2.5x)、has_exception/has_nopex 缓存、Literal 直接写入 writer、criterion benchmarks
- [x] M4.3 完整文档 — crate-level docs、所有 pub API 代码示例(20 doctests)、配置参考 TOML 示例、模块级架构说明

---

## 技术债清单

| 编号 | 问题 | 状态 | 备注 |
|------|------|------|------|
| D1 | Renderer 双份代码 | ✅ 已解决 | M2.1 render_token_string 单一来源 |
| D2 | hot_reload.rs 复制 lib.rs 逻辑 | ⚠️ Unstable | 标记为 unstable，待基于 reload::Layer 重构 |
| D3 | appender/mod.rs 为空 | ℹ️ 低优先级 | 逻辑在 lib.rs，不影响功能 |
| D4 | SamplingWriter rate=0 封装开销 | ℹ️ 低优先级 | 热路径上开销极小 |
| D5 | `%d` 每次事件重新格式化 | ✅ 已解决 | 构造时预处理 Java→chrono，2.5x 提速 |
| D6 | message 提取用 trim + strip_prefix hack | ✅ 已解决 | M1.4 EventVisitor 改进 |
| D7 | multi-appender 只取第一个 formatter | ✅ 已解决 | M1.5 MultiMakeWriter 泛化 |
| D8 | `tracing-test` / `strip-ansi-escapes` 未在 Cargo.toml | ✅ 已解决 | 已添加 |
| D9 | `try_init_from_str` 与 `init_from_str` 行为一致 | ✅ 已解决 | deprecated 标记 |

---

## 后续可能的方向（v1.x）

| 方向 | 优先级 | 说明 |
|------|--------|------|
| Hot Reload 基于 `reload::Layer` | 中 | 当前标记 unstable，需重新设计 |
| `%M` / `%method` 取真实函数名 | 低 | 需宏配合，当前取最近 span 名 |
| `%rEx` crate 版本信息构建时注入 | 低 | 需 build.rs 改进 |
| Windows 彩色输出 `ENABLE_VIRTUAL_TERMINAL_PROCESSING` | 低 | 已有基本实现 |
| 预编译 pattern 为渲染闭包 | 低 | 当前 token 遍历已足够快 |
| `hot-reload` feature 基于 notify | 中 | 需 reload::Layer 先完成 |
| `[sampling]` 限流逻辑完善 | 低 | 当前已可用 |
| `formatter/log4j/` 更多转换符 | 低 | 核心已实现 |
| size-based rolling file rotation | 中 | max_size 字段已预留 |
| per-appender formatter（非共享第一个） | 中 | 当前架构限制 |
