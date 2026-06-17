# tracing-declarative 路线图

> 最后更新：2026-06-17 · 当前版本：0.1.0（未发布）· 目标版本：v1.0.0

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
| D10 | 包名 `tracing-declarative` 与 crates.io 已有 crate 冲突 | 🔴 待解决 | 重命名为 `tracing-declarative`，见 RELEASE-PLAN.md |
| D11 | Cargo.toml 缺少 crates.io 必需元数据 | 🔴 待解决 | description / license / repository 等，见 RELEASE-PLAN.md |
| D12 | 缺少 LICENSE 文件 | 🔴 待解决 | 需添加 LICENSE-MIT + LICENSE-APACHE |
| D13 | README.md 过时描述 | 🟡 待修复 | log4j "planned" 实际已完成 |
| D14 | hot-reload 作为 default feature | 🟡 待修复 | unstable 功能不应默认开启 |

---

## 后续里程碑规划

### M5: v1.0.0 发布 — 包名变更 + 发版准备 🔴

> 详见 [RELEASE-PLAN.md](./RELEASE-PLAN.md)

- [ ] M5.1 包名重命名 `tracing-declarative` → `tracing-declarative`
- [ ] M5.2 Cargo.toml 元数据补全
- [ ] M5.3 添加 LICENSE 文件
- [ ] M5.4 README.md 更新（移除过时描述，补充 log4j/otel/sampling）
- [ ] M5.5 hot-reload 从 default feature 移出
- [ ] M5.6 dry-run 验证通过
- [ ] M5.7 正式发布到 crates.io

### M6: v1.1.0 — 实用性增强

- [ ] M6.1 Per-appender formatter — 每个 appender 独立格式化
- [ ] M6.2 Size-based rolling file rotation — 接入 max_size / max_files
- [ ] M6.3 Hot Reload 基于 `reload::Layer` 重构

### M7: v1.2.0 — 开发体验

- [ ] M7.1 YAML 配置支持（`serde_yaml` feature）
- [ ] M7.2 环境变量插值（`${ENV_VAR}` 在 TOML 值中展开）
- [ ] M7.3 配置校验增强（更友好的错误信息 + 字段路径提示）

### M8: v1.3.0 — 高级功能

- [ ] M8.1 `%M` 真实函数名（proc-macro `#[tracing_name]` 配合）
- [ ] M8.2 `%rEx` crate 版本注入（扩展 build.rs 到依赖链）
- [ ] M8.3 自定义 appender 注册（用户注册 `Box<dyn MakeWriter>` 工厂）

---

## 与已有 `tracing-declarative` crate 的差异

crates.io 上的 [tracing-declarative v0.2.2](https://github.com/mateiandrei94/tracing-declarative) 功能相似但实现不同：

| 维度 | tracing-declarative (已有) | tracing-declarative (本项目) |
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
