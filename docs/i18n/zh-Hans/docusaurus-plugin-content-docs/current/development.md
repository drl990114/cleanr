---
description: 作为贡献者构建、测试、检查和维护 Cleanr 文档。
---

# 开发

## 环境要求

- Rust 1.94.1 或兼容的更高版本
- 文档站点需要 Node.js 20 或更高版本
- pnpm 10

## 构建 workspace

构建默认的本地 Provider 版本：

```bash
cargo build
```

构建包含 OpenAI 和 Ollama 可选 Provider 的发布版本：

```bash
cargo build --release --all-features
```

CLI 二进制位于 `target/release/cleanr`。

## 运行与 CI 相同的检查

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --all-targets --all-features --locked
```

校验生成的 JSON Schema：

```bash
cargo run --locked -p cleanr-cli -- plugin schema manifest >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema rules >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema language >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema config >/dev/null
```

## 本地运行文档站点

```bash
cd docs
pnpm install
pnpm start
```

默认开发地址为 `http://localhost:3000/`。

提交文档改动前：

```bash
pnpm typecheck
pnpm build
```

## 保持中英文同步

- 英文源文档位于 `docs/docs/`。
- 简体中文文档位于
  `docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/`。
- 共享 UI 文案位于 `docs/i18n/zh-Hans/` 下的 locale JSON 文件。

修改 React 翻译文本、导航、页脚或侧边栏分类后，重新生成翻译键：

```bash
pnpm docusaurus write-translations --locale zh-Hans
```

翻译新增条目，并构建两个语言版本。

## 贡献检查清单

- 行为变化需要新增或更新测试。
- 命令、默认值、安全行为或平台支持变化时，更新用户文档。
- 同一次改动中更新英文和简体中文。
- 示例应可执行，不要把计划中的行为写成已经实现。
- 运行格式化、Clippy、workspace 测试、类型检查和文档构建。
