# tracing-declarative 发布计划

> 最后更新：2026-06-17 · 目标版本：v1.0.0

本文档是 `tracing-declarative` 发布到 crates.io 的执行清单。

---

## 包名说明

原项目名 `tracing-config` 在 crates.io 已被占用（v0.2.2，[mateiandrei94/tracing-config](https://github.com/mateiandrei94/tracing-config)），
功能相似但实现不同。选择 `tracing-declarative` 作为包名，理由：

- **突出核心理念** — 声明式（Declarative）配置驱动，与硬编码初始化形成对比
- **避免混淆** — 与已有 crate 无命名冲突
- **语义准确** — TOML 文件声明 → 自动初始化，正是 declarative 的含义
- **可扩展** — 未来支持 YAML / JSON 等格式时，"declarative" 仍然适用

---

## 发版准备：完成状态

### ✅ 已完成项

| # | 项目 | 状态 | 说明 |
|---|------|------|------|
| 0.1 | 包名 `tracing-declarative` | ✅ | Cargo.toml / examples / tests / README / CHANGELOG / CLAUDE.md 全部更新 |
| 0.2 | Cargo.toml 元数据 | ✅ | description / license / repository / documentation / readme / keywords / categories / rust-version |
| 0.3 | LICENSE 文件 | ✅ | LICENSE-MIT + LICENSE-APACHE 双文件 |
| 0.4 | README.md 更新 | ✅ | 移除过时描述，补充 log4j 表格 / OTel / Sampling / Feature flags |
| 0.5 | hot-reload 从 default feature 移出 | ✅ | `default = []` |
| 0.6 | CHANGELOG.md 版本号 | ✅ | `[1.0.0] — 2026-06-17` |
| 0.7 | rust-version | ✅ | `rust-version = "1.75"` |
| 0.8 | `cargo publish --dry-run` | ✅ | 73 files, 84.1 KiB compressed，零错误 |

### 🔴 阻塞发版（1 项）

| # | 项目 | 文件 | 说明 |
|---|------|------|------|
| B1 | crate-level doc 包名 | `src/lib.rs:1` | 首行写 `tracing-config`，应为 `tracing-declarative` |

### 🟡 发版后应修复（不阻塞）

| # | 项目 | 文件 | 说明 |
|---|------|------|------|
| P1 | CLAUDE.md 过时内容 | `CLAUDE.md` | 多处 "log4j/otel/sampling 待实现"、依赖版本过时、目录结构不完整 |
| P2 | tracing.toml 注释 | `tracing.toml:3` | `log4j（待扩展）` → log4j 已完成 |

---

## 正式发布步骤

### Step 1：修复阻塞项

修复 `src/lib.rs` 第 1 行 crate-level doc 中的包名：

```rust
// 修复前
//! tracing-config — Declarative tracing initialization via `tracing.toml`.

// 修复后
//! tracing-declarative — Declarative tracing initialization via `tracing.toml`.
```

验证：

```bash
cargo test && cargo doc
```

### Step 2：最终确认

```bash
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo publish --dry-run --registry crates-io
```

### Step 3：Git tag

```bash
git add -A
git commit -m "release: tracing-declarative v1.0.0"
git tag v1.0.0 -m "Release tracing-declarative v1.0.0"
git push origin master
git push origin v1.0.0
```

### Step 4：发布到 crates.io

```bash
cargo publish --registry crates-io
```

### Step 5：发布后验证

- [ ] `cargo search tracing-declarative --registry crates-io` 可搜到
- [ ] `docs.rs/tracing-declarative` 文档生成成功
- [ ] 新项目 `cargo add tracing-declarative` 可正常使用
- [ ] GitHub Release 页面创建

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| `tracing-declarative` 名称被抢注 | 🔴 阻塞 | ✅ 已确认未被占用（`cargo search` 无结果） |
| `tracing-subscriber` 0.3 API 变更 | 🟡 破坏性 | 锁定 `0.3` 版本，关注 0.4 迁移 |
| `opentelemetry` 0.32 快速迭代 | 🟡 feature-gated | feature-gated 隔离，不影响核心功能 |
| `chrono` → `time` 迁移趋势 | 🟢 未来技术债 | 当前 chrono 稳定，暂不迁移 |

---

## v1.x 迭代规划

> 详见 [ROADMAP.md](./ROADMAP.md) 中的 M6-M8 里程碑。

| 版本 | 主题 | 核心功能 |
|------|------|----------|
| v1.1.0 | 实用性增强 | Per-appender formatter / Size-based rolling / Hot Reload 重构 |
| v1.2.0 | 开发体验 | YAML 配置 / 环境变量插值 / 配置校验增强 |
| v1.3.0 | 高级功能 | `%M` 真实函数名 / `%rEx` crate 版本 / 自定义 appender |
