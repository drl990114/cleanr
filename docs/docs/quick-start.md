---
sidebar_position: 2
description: Install Cleanr and safely complete your first scan, review, and cleanup.
---

# Quick start

## 1. Install Cleanr

Choose one installation method.

### npm

Requires Node.js 18 or later:

```bash
npm install --global cleanr-cli
```

The npm package installs a small launcher and the native binary for your
operating system and CPU.

### Cargo

Requires Rust 1.94 or later:

```bash
cargo install cleanr-cli
```

This build includes the local command interpreter. Remote OpenAI and Ollama
providers are optional features; they are included in official release
binaries.

### Prebuilt binary

Download the binary for your platform from
[GitHub Releases](https://github.com/drl990114/cleanr/releases). On macOS or
Linux, make the downloaded file executable and place it somewhere on your
`PATH`.

### Build from source

```bash
git clone https://github.com/drl990114/cleanr.git
cd cleanr
cargo build --release --all-features
```

The binary is written to `target/release/cleanr`.

## 2. Launch the TUI

Run Cleanr in the directory you want to inspect:

```bash
cleanr
```

You can also choose one or more scan roots when launching:

```bash
cleanr ~/projects ~/Downloads
```

Cleanr opens on its home screen. It does **not** scan or clean anything until
you start an action.

## 3. Complete your first cleanup

Inside the TUI:

1. Press `s` to scan the configured roots.
2. When the scan finishes, press `r` to review cleanup candidates.
3. Move with `j`/`k` or the arrow keys.
4. Press `space` to select or deselect an item.
5. Press `c` to review the selected total and open confirmation.
6. Choose **Yes**, then press `Enter` to move the selected items to trash.

Press `?` at any time to see the keyboard shortcuts. Press `Esc` or `x` during
a scan to cancel it.

:::tip Start conservatively

For a first run, scan one project rather than your whole home directory.
Review each candidate's reason and risk note before confirming.

:::

## Scan known developer caches

Press `/`, type the following command, and press `Enter`:

```text
/scan --global
```

This searches known cache locations for the current platform. It does not mean
"scan the entire disk."

## Use Simplified Chinese

Initialize the built-in Chinese language file:

```bash
cleanr init --locale zh-CN
```

You can later open `/languages` in the TUI, select a language, and press
`Enter`. The selection is saved to the default configuration file.

## Next steps

- Learn all shortcuts and slash commands in [Using Cleanr](./using-cleanr).
- Understand the rollback boundary in
  [Safety and recovery](./safety-and-recovery).
- Exclude directories or change the theme in
  [Configuration](./configuration).
