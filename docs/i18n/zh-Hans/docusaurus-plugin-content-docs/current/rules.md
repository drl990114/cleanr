---
sidebar_position: 6
description: 了解 Cleanr 为什么标记某个路径、置信度如何影响选择，以及内置规则覆盖什么。
---

# 规则与置信度

Cleanr 不会只看目录名就判断它可以移除。扫描条目会与带版本的**规则包**匹配，
规则会解释它是什么、为什么可以清理，以及重建它可能付出什么代价。

## 每个候选项包含什么

| 字段 | 含义 |
| --- | --- |
| 名称 | 便于理解的名称，例如“Rust target 目录” |
| 分类 | `build-cache`、`package-cache`、`downloads` 等分组 |
| 置信度 | `High`、`Medium` 或 `Low` |
| 原因 | 为什么该路径被视为清理候选项 |
| 风险说明 | 清理后可能出现的问题、耗时或网络下载 |
| 默认选择 | 规则是否请求预选该条目 |

同一个条目被多条规则匹配时，Cleanr 会结合来源信任级别、默认选择和置信度
保留最佳命中。最终计划还会移除互相重叠的父子候选项，避免重复计算空间。

## 置信度不是绝对保证

| 等级 | 建议 |
| --- | --- |
| `High` | 通常是生成或可下载数据；不熟悉的路径仍需审阅 |
| `Medium` | 往往可以重建，但代价可能较高，或包含仅存在于本机的状态 |
| `Low` | 可能是用户数据，必须谨慎人工确认 |

只有来自内置或可信来源、置信度为 `High` 且
`default_selected = true` 的规则才能预选条目。

## 内置规则包

### `builtin-dev`

识别常见的开发者生成数据，包括：

- `node_modules`、Rust `target`、Python 工具缓存和 Next.js 缓存；
- Cargo、npm、pnpm、Yarn、pip、uv、Gradle、Maven 和 Go 缓存；
- Xcode `DerivedData`。

明确可重建的内容通常为高置信度。tox 环境和 Maven 本地仓库等更模糊的内容
不会被预选。

### `builtin-general`

查找需要人工审阅的通用候选项：

- Downloads 目录中至少 100 MiB 的文件；
- 至少 50 MiB 的 `.log` 文件；
- 至少 1 MiB 的 `.tmp` 文件。

这些规则有意设置为中低置信度，并且默认不选中。

### `builtin-system`

查找已知用户级系统清理候选项：

- 常见浏览器缓存目录；
- 应用缓存目录；
- 大型临时文件、日志和 Downloads 文件。

只有高置信浏览器缓存目录会被预选。应用缓存、临时文件、日志和 Downloads
默认只展示，需人工审阅。

## 启用或禁用规则包

只有 `cleanup.enabled_rule_packs` 中的 ID 会被加载：

```toml
[cleanup]
enabled_rule_packs = ["builtin-dev", "builtin-general", "builtin-system"]
```

如果只想关注开发者缓存，可以移除 `builtin-general` 和 `builtin-system`。

在 TUI 中运行 `/rules` 可以查看当前启用的规则包和规则。

## 添加自定义规则

推荐使用声明式插件 bundle。完整的最小示例、校验命令和信任模型见
[插件](./plugins)。

插件目录中仍可以发现旧版独立 TOML 规则包，但 bundle 能提供版本和兼容性
元数据，因此更推荐使用。
