---
sidebar_position: 8
description: 解决 Cleanr 安装、终端、扫描、配置、Provider 和恢复中的常见问题。
---

# 故障排查

## Cleanr 已打开，但没有扫描

这是正常行为。启动只会设置扫描根目录。按 `s`、运行 `/scan`，或按 `u`
执行用量扫描。

## 命令面板里没有 `/review` 或 `/clean`

依赖扫描结果的命令会在扫描完成前隐藏。请先运行 `/scan`。如果扫描仍在进行，
等待完成，或按 `Esc` / `x` 取消。

## 扫描没有找到候选项

请检查：

- 对应规则包是否在 `cleanup.enabled_rule_packs` 中；
- 目标是否被 `ignore_dirs` 或 `ignore_patterns` 排除；
- 条目是否满足规则的大小、时间、名称或路径条件；
- 扫描的是包含候选项的目录，而不是把候选目录本身作为根目录。扫描根目录本身
  永远不会成为清理候选项。

使用 `/rules` 查看已加载规则，使用 `/plugins` 确认自定义 bundle 是否被发现。

## `/scan --global` 提示没有发现缓存目录

当前平台没有返回已知的全局开发者缓存根目录。仍然可以显式指定路径：

```text
/scan /home/me/.cargo /home/me/.npm
```

请使用当前操作系统真实存在的绝对路径。TUI 中输入的路径不会展开 `~` 或
环境变量。

## Cleanr 报告配置解析错误

打印默认配置路径：

```bash
cleanr config path
```

如果使用自定义文件，排查时要带上相同的 `--config`。重点检查未知键、拼错的
枚举值、重复 ID 和无效 TOML。

在不覆盖原文件的情况下生成一份新默认配置用于对比：

```bash
cleanr --config /tmp/cleanr-default.toml config init
```

## 终端显示异常

- 确认终端支持 Unicode 和彩色显示。
- Cleanr 默认使用便携的 ANSI 基础色。如果颜色仍变成整片红/绿块，
  请重置终端配置，或换一个终端应用验证。
- 如果只是背景明暗不对，再设置显式主题：

  ```bash
  cleanr config set ui.theme dark
  ```

- 放大过小的终端窗口。
- 如果程序被强制中断，在 Shell 中运行 `reset` 恢复终端状态。

## OpenAI 或 Ollama 显示不受支持

当前二进制没有编译对应可选功能。官方发布版本包含两者。从源码构建时使用：

```bash
cargo build --release --all-features
```

然后配置 Provider、模型、必要时的端点，以及保存 API Key 的环境变量。
`api_key_env` 中应填写环境变量名，而不是密钥本身。

## 已选条目在清理时被跳过

Cleanr 会在执行前重新校验每个目标。如果条目在扫描后变化、变成符号链接、
移出扫描根目录或与受保护路径重叠，就会被跳过。请重新扫描并审阅新状态，不要
强制执行旧计划。

## 恢复失败

常见原因包括：

- 系统回收站已清空；
- 条目被手动移出回收站；
- 原路径已经存在；
- 回收站元数据发生变化或不可用；
- 当前平台不支持程序化恢复。

Cleanr 不会覆盖当前路径。排查期间请手动检查系统回收站，并保留 Cleanr 状态
目录和清单。

## 关闭更新检查

Cleanr 最多每 24 小时检查一次新版本。可以关闭这个非阻塞启动检查：

```bash
cleanr --no-update-check
```

或：

```bash
export CLEANR_NO_UPDATE_CHECK=true
```

## 获取更多帮助

如果问题可以稳定复现，请在
[GitHub](https://github.com/drl990114/cleanr/issues) 提交 issue，并附上：

- Cleanr 版本（`cleanr --version`）；
- 操作系统和终端；
- 安装方式；
- 完整命令或按键步骤；
- 去除密钥和个人路径后的完整错误信息。
