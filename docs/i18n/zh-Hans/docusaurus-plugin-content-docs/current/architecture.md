---
description: 面向贡献者的 Cleanr crate、数据流和安全边界说明。
---

# 架构

本页面面向需要理解 Cleanr 内部职责的贡献者和插件作者。只想使用应用时，请从
[使用 Cleanr](./using-cleanr)开始。

## Workspace crate

| Crate | 路径 | 职责 |
| --- | --- | --- |
| `cleanr-core` | `crates/core` | 扫描条目、规则命中、清理计划、安全策略和清单模型 |
| `cleanr-cli` | `crates/cli` | 命令行入口、参数解析、配置命令和插件管理 |
| `cleanr-tui` | `crates/tui` | 终端应用、状态机、页面和后台任务编排 |
| `cleanr-agent` | `crates/agent` | 斜杠命令解析、本地路径解释和可选远程 Provider |
| `cleanr-fs` | `crates/fs` | 文件系统扫描、元数据收集、取消和 `ScanReport` 生成 |
| `cleanr-rules` | `crates/rules` | 内置与插件规则加载、校验、匹配和 `RuleRegistry` |
| `cleanr-plugin-api` | `crates/plugin-api` | 带版本 manifest、发现、兼容性、信任、Schema 和诊断 |
| `cleanr-config` | `crates/config` | 配置 Schema、默认值、校验和原子写入 |
| `cleanr-i18n` | `crates/i18n` | 内置与外部语言包、回退和运行时语言切换 |
| `cleanr-tasks` | `crates/tasks` | 清理执行、系统回收站、恢复和清单持久化 |

## 运行时数据流

```text
CLI 参数 + 配置
        │
        ▼
TUI 状态机 ── 启动扫描任务
        │               │
        │               ▼
        │         cleanr-fs 条目
        │               │
        │               ▼
        │         cleanr-rules 命中
        │               │
        ▼               ▼
用户审阅 ◄──────── 清理计划
        │
        ▼
Workflow 服务 / 本地授权
        │
        ▼
pending 清单 → cleanr-tasks 校验 → 系统回收站 → 清单更新
        │
        └────────────────────→ 恢复 → 恢复清单
```

计划生成器会先移除重叠候选项，再计算已选空间和候选项总空间。

## TUI 边界

`cleanr-tui` 将渲染与 I/O 分离：

- `app/` 负责状态变化和用户动作；
- `effects/` 负责后台扫描、持久化、清理、恢复和 Agent 工作；
- `views/` 只根据应用状态渲染；
- `commands/` 将动作请求映射到命令面板；
- `terminal.rs` 负责 raw mode、输入轮询、绘制和终端恢复。

页面不会遍历文件系统。后台任务将结果发送回状态机，因此取消和部分失败都能
明确反映在 UI 中。

## 安全边界

安全性由多个层次共同执行：

- `cleanr-rules` 只允许高置信度可信规则自动选择；
- `cleanr-core` 在生成计划时排除受保护和重叠候选项，并为选中目录记录指纹；
- `cleanr-tasks` 要求本地授权，在移动文件前写入 journal，并在执行时重新校验每个目标；
- 回收站后端在平台支持时记录回滚信息；
- Agent 可以提出动作，但不能创建清理授权。

插件默认保持声明式。manifest、规则和翻译只会作为数据解析；动态 hook 是单独
受信任的外部命令能力。

## 持久化数据

配置使用平台配置目录。清理和恢复清单位于平台状态目录下的 `cleanr/`，
分别存放在 `runs/` 和 `restores/` 中。

`cleanr-tasks` 通过 `ManifestRepository` 统一负责清单持久化，把列表、查找和
原子写入集中成一套供 TUI 与 CLI 共用的 API。

写入使用临时文件和原子替换，避免写入中断时静默破坏有效配置或清单。
