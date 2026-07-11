# Cleanr

面向开发者缓存、以证据为先的可审阅本地磁盘清理工具。

[English](../en/README.md) | [仓库 README](../../README.md) | [完整文档](../../docs/) | [贡献指南](../../CONTRIBUTING.md)

Cleanr 帮助开发者发现可重建的生成文件和缓存，避免把磁盘清理变成盲删。它会扫描你选择的路径，说明每个候选项的匹配原因，让你在键盘驱动的终端界面里审阅清理计划，并把选中的项目移动到系统废纸篓。

## 特性

- 键盘驱动的扫描、审阅、清理和恢复流程。
- 内置规则覆盖常见开发者缓存、构建产物、包管理器缓存、大文件下载、日志和临时文件。
- 每个候选项都会展示大小、置信度、匹配原因和风险提示。
- 提供仅限本机的 `cleanr analyze` JSON 契约，供用户自己的本地编码 Agent 读取确定性证据，但不授予清理权限。
- 保守的默认选择策略：只有来自内置规则或可信规则的高置信度项目才可能被预选。
- 通过系统废纸篓清理、执行前再次校验、父子候选项去重和本地清理清单降低风险。
- 支持 macOS 废纸篓、Windows 回收站和兼容 Freedesktop 的 Linux 废纸篓恢复历史。
- 支持声明式插件，用于扩展清理规则和翻译。
- 提供 macOS、Linux 和 Windows 原生包，可通过 npm、Cargo 或 GitHub Release 安装。
- 支持英文和简体中文界面。

## 安装

通过 npm 安装：

```bash
npm install --global cleanr-cli
```

通过 Cargo 安装：

```bash
cargo install cleanr-cli
```

也可以从 [GitHub Releases](https://github.com/drl990114/cleanr/releases) 下载预编译二进制文件。

## 开始使用

在需要检查的目录中运行：

```bash
cleanr
```

或者指定一个或多个扫描根目录：

```bash
cleanr ~/projects ~/Downloads
```

进入 TUI 后，按 `s` 扫描，按 `r` 审阅候选项，按 `space` 选择或取消选择，按 `c` 确认清理。使用 `/scan --global` 可以检查已知系统清理位置；平台支持时，可使用 `/restore` 恢复历史清理运行。

在 TUI 中按 `?` 可查看快捷键帮助。

让本地编码 Agent 协助分析时，使用只读命令；除非先主动脱敏，否则不要将 JSON
发送到设备外：

```bash
cleanr analyze ~/projects > cleanr-analysis.json
```

报告只提供审阅证据，不是清理指令。Cleanr 不提供 Agent 执行命令；清理仍需由人在
TUI 中审阅并确认。

TUI、`analyze`、`plan` 和 `dry-run` 共用 `cleanr.toml` 中的
`[recommendations].preselect_after_days`（默认 90 天；设为 `0` 会关闭年龄门槛）。

可直接从 GitHub 安装仓库中的跨 Agent Skill `cleanr-review-disk-cleanup`：

```bash
npx skills add drl990114/cleanr@cleanr-review-disk-cleanup -g
```

它只指导本地只读分析，没有清理权限。支持的 Agent 选择方式和用法请见
[证据与隐私](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/evidence-and-privacy.md)。

## 安全模型

Cleanr 不会因为找到某个路径就直接清理。执行前你仍然可以编辑计划；选中路径会在清理前再次校验；清理动作会移动到系统废纸篓，而不是永久删除。

恢复能力依赖系统废纸篓，是尽力而为的机制。确认清理结果无误前，请不要清空系统废纸篓。

## 了解更多

- [快速开始](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/quick-start.md)
- [使用 Cleanr](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/using-cleanr.md)
- [安全与恢复](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/safety-and-recovery.md)
- [配置](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/configuration.md)
- [插件](../../docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/plugins.md)

## 参与贡献

开发环境、检查命令、文档工作流和发布说明请见 [CONTRIBUTING.md](../../CONTRIBUTING.md)。

## 致谢

Cleanr 的部分代码来源于
[Byron/dua-cli](https://github.com/Byron/dua-cli)。`dua-cli` 是由
Sebastian Thiel 及其贡献者以 MIT License 授权的磁盘使用分析工具。

## 许可证

Cleanr 使用 [MIT License](../../LICENSE) 授权。
