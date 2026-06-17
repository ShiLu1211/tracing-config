# tracing-declarative 发布计划

> 最后更新：2026-06-17 · 目标版本：v1.0.0

本文档是 `tracing-declarative`（原 `tracing-declarative`）发布到 crates.io 的执行清单。

---

## 包名变更说明

原包名 `tracing-declarative` 在 crates.io 已被占用（v0.2.2，[mateiandrei94/tracing-declarative](https://github.com/mateiandrei94/tracing-declarative)），
功能类似但实现不同。经评估，选择 `tracing-declarative` 作为新包名，理由：

- **突出核心理念** — 声明式（Declarative）配置驱动，与硬编码初始化形成对比
- **避免混淆** — 与已有 crate 无命名冲突
- **语义准确** — TOML 文件声明 → 自动初始化，正是 declarative 的含义
- **可扩展** — 未来支持 YAML / JSON 等格式时，"declarative" 仍然适用

---

## Phase 0：发版准备（阻塞项）

> 以下所有项目必须完成后才能执行 `cargo publish`。

### 0.1 包名重命名 🔴

**范围**：全局替换 `tracing-declarative` → `tracing-declarative`

| 文件 | 变更内容 |
|------|----------|
| `Cargo.toml` | `name = "tracing-declarative"` |
| `README.md` | 所有 `tracing-declarative` → `tracing-declarative`，`tracing_declarative::` → `tracing_declarative::` |
| `CHANGELOG.md` | 包名引用更新 |
| `tracing.toml` | 注释中的包名引用 |
| `examples/*.rs` | `use tracing_declarative` → `use tracing_declarative` |
| `tests/*.rs` | 同上 |
| `src/lib.rs` | crate-level doc 中的包名 |
| `build.rs` | `CRATE_NAME` 常量值 |
| `CLAUDE.md` | 项目上下文中的包名引用 |
| `docs/ROADMAP.md` | 包名引用 |

**验证**：`cargo build && cargo test && cargo doc`

### 0.2 Cargo.toml 元数据补全 🔴

当前缺失字段（crates.io 发布必需/强烈建议）：

```toml
[package]
name = "tracing-declarative"
version = "1.0.0"
edition = "2021"
description = "Declarative tracing initialization via tracing.toml — logback/log4j patterns, multi-appender, OpenTelemetry"
license = "MIT OR Apache-2.0"
repository = "https://github.com/<用户名>/tracing-declarative"
documentation = "https://docs.rs/tracing-declarative"
readme = "README.md"
keywords = ["tracing", "logging", "logback", "toml", "declarative"]
categories = ["development-tools::debugging"]
# rust-version = "1.70"  # 建议补充，需确认实际 MSRV
```

**注意**：`repository` URL 需要确认 GitHub 仓库地址。

### 0.3 LICENSE 文件 🔴

添加双许可文件到项目根目录：

- `LICENSE-MIT` — MIT 许可证全文
- `LICENSE-APACHE` — Apache-2.0 许可证全文

`Cargo.toml` 中 `license = "MIT OR Apache-2.0"` 会自动关联这两个文件。

### 0.4 dry-run 验证 🔴

```bash
cargo publish --dry-run --registry crates-io
```

必须零错误通过。

---

## Phase 1：发版前修复（建议项）

> 不阻塞发布，但建议在 v1.0.0 之前完成。

### 1.1 README.md 更新 🟡

当前 README 存在过时描述：

- ❌ "The log4j formatter engine is planned but not yet implemented" → log4j 已完成
- ❌ "Per-appender formatters share the first appender's format string" → 需更新状态
- ❌ 缺少 log4j 引擎的 conversion word 表格
- ❌ 缺少 OpenTelemetry 配置示例
- ❌ 缺少 Sampling 配置示例
- ❌ 包名需全部替换

建议补充：
- log4j 引擎简介 + conversion word 表
- OpenTelemetry feature 使用示例
- Sampling feature 使用示例
- 更完整的 `tracing.toml` 示例（含所有 section）

### 1.2 hot-reload feature 调整 🟡

当前 `default = ["hot-reload"]`，但 hot-reload 标记为 unstable。
建议改为：

```toml
default = []
hot-reload = ["dep:notify"]
```

理由：默认 feature 应该是稳定的、生产可用的。unstable 功能不应默认开启。

### 1.3 CHANGELOG.md 版本号确认 🟡

当前 CHANGELOG 中 `[Unreleased] — v1.0.0` 需要改为正式版本号：

```markdown
## [1.0.0] — 2026-06-XX
```

### 1.4 toml 依赖版本放宽 🟢

```toml
# 当前
toml = "1.1.2"
# 建议
toml = "1"
```

精确版本锁定不必要，`1.x` 兼容性由 semver 保证。

### 1.5 MSRV 确认 🟢

建议确定并声明 Minimum Supported Rust Version：

```bash
# 使用 cargo-msrv 工具检查
cargo msrv find
```

然后在 `Cargo.toml` 中添加 `rust-version = "1.xx"`。

---

## Phase 2：正式发布

### 2.1 版本号更新

```toml
version = "1.0.0"
```

### 2.2 Git tag

```bash
git tag v1.0.0 -m "Release tracing-declarative v1.0.0"
git push origin v1.0.0
```

### 2.3 发布

```bash
cargo publish --registry crates-io
```

### 2.4 发布后验证

- [ ] `cargo install tracing-declarative` 可安装
- [ ] `docs.rs/tracing-declarative` 文档生成成功
- [ ] 新项目 `cargo add tracing-declarative` 可正常使用
- [ ] GitHub Release 页面创建

---

## Phase 3：v1.x 迭代规划

> 按优先级排序，根据实际需求选择实施。

### v1.1.0 — 实用性增强

| 功能 | 优先级 | 复杂度 | 说明 |
|------|--------|--------|------|
| Per-appender formatter | 🔴 高 | 中 | 当前所有 appender 共享第一个 formatter，需重构 `build_factory` |
| Size-based rolling file | 🟡 中 | 低 | `max_size` / `max_files` 字段已预留，需接入 `tracing-appender::rolling` |
| Hot Reload 基于 `reload::Layer` | 🟡 中 | 高 | 需重新设计架构，当前 unstable 标记 |
| `hot-reload` feature 默认关闭 | 🟢 低 | 低 | Phase 1.2 的延续 |

### v1.2.0 — 开发体验

| 功能 | 优先级 | 复杂度 | 说明 |
|------|--------|--------|------|
| YAML 配置支持 | 🟡 中 | 中 | 新增 `serde_yaml` feature，`Config` 已是 serde 结构体 |
| 环境变量插值 | 🟡 中 | 中 | `${ENV_VAR}` 在 TOML 值中展开 |
| 配置校验增强 | 🟢 低 | 低 | 更友好的错误信息，字段路径提示 |
| `#[derive(TracingInit)]` 宏 | 🟢 低 | 高 | proc-macro 一行初始化 |

### v1.3.0 — 高级功能

| 功能 | 优先级 | 复杂度 | 说明 |
|------|--------|--------|------|
| `%M` 真实函数名 | 🟡 中 | 中 | 需 proc-macro `#[tracing_name]` 配合 |
| `%rEx` crate 版本注入 | 🟢 低 | 低 | `build.rs` 已有 `CRATE_NAME`/`CRATE_VERSION`，需扩展到依赖链 |
| 预编译 pattern 为渲染闭包 | 🟢 低 | 中 | 当前 token 遍历已够快，性能提升有限 |
| 自定义 appender 注册 | 🟢 低 | 高 | 允许用户注册 `Box<dyn MakeWriter>` 工厂 |

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| `tracing-declarative` 名称也被占用 | 🔴 阻塞发布 | 发布前先 `cargo search` 确认；备选名：`tracing-toml-config`、`tracing-logback` |
| `tracing-subscriber` 0.3 API 变更 | 🟡 破坏性 | 锁定 `0.3` 版本，关注 0.4 迁移 |
| `opentelemetry` 0.32 快速迭代 | 🟡 feature-gated 破坏 | feature-gated 隔离，不影响核心功能 |
| `chrono` → `time` 迁移趋势 | 🟢 未来技术债 | 当前 chrono 稳定，暂不迁移 |
| MSRV 不确定 | 🟢 用户兼容性 | Phase 1.5 确认后声明 |

---

## 执行检查清单

发布前逐项确认：

- [ ] 包名 `tracing-declarative` 在 crates.io 未被占用
- [ ] `Cargo.toml` 元数据完整（name/version/description/license/repository/readme/keywords/categories）
- [ ] `LICENSE-MIT` + `LICENSE-APACHE` 文件存在
- [ ] `README.md` 内容准确、无过时描述
- [ ] `CHANGELOG.md` 版本号已确认
- [ ] `cargo test --all-features` 通过
- [ ] `cargo clippy --all-features -- -D warnings` 零警告
- [ ] `cargo doc` 无断链
- [ ] `cargo publish --dry-run --registry crates-io` 通过
- [ ] Git 仓库干净（无未提交变更）
- [ ] GitHub 仓库已创建/重命名
