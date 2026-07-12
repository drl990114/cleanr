<div align="center">
  <h1>Cleanr</h1>
  <p><strong>Let your AI help you safely clean your disk with Cleanr.</strong></p>
  <p>
    <a href="https://drl990114.github.io/cleanr/">Documentation</a>
    ·
    <a href="https://github.com/drl990114/cleanr/releases">Download</a>
    ·
    <a href="https://github.com/drl990114/cleanr/discussions">Discussions</a>
  </p>
  <p>
    <a href="https://github.com/drl990114/cleanr/actions/workflows/ci.yml"><img alt="CI workflow" src="https://img.shields.io/github/actions/workflow/status/drl990114/cleanr/ci.yml?branch=main&label=CI&style=flat-square&logo=githubactions&logoColor=white"></a>
    <a href="https://github.com/drl990114/cleanr/actions/workflows/release.yml"><img alt="Release workflow" src="https://img.shields.io/github/actions/workflow/status/drl990114/cleanr/release.yml?label=release&style=flat-square&logo=githubactions&logoColor=white"></a>
    <a href="https://github.com/drl990114/cleanr/blob/main/LICENSE"><img alt="MIT License" src="https://img.shields.io/github/license/drl990114/cleanr?style=flat-square&color=0f766e"></a>
    <a href="https://www.npmjs.com/package/cleanr-cli"><img alt="npm version" src="https://img.shields.io/npm/v/cleanr-cli?style=flat-square&logo=npm"></a>
  </p>
  <p>
    <img alt="Rust" src="https://img.shields.io/badge/Rust-1.94-000000?style=flat-square&logo=rust&logoColor=white">
    <img alt="Ratatui" src="https://img.shields.io/badge/Ratatui-0.29-2563eb?style=flat-square">
    <img alt="Platforms: macOS, Linux, and Windows" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-475569?style=flat-square">
    <img alt="Open source" src="https://img.shields.io/badge/open%20source-MIT-155eef?style=flat-square">
  </p>
  <p>
    <a href="readme/en/README.md">English</a>
    ·
    <a href="readme/zh-CN/README.md">简体中文</a>
    ·
    <a href="CONTRIBUTING.md">Contributing</a>
  </p>
</div>

Cleanr helps developers find rebuildable generated files and caches without
turning cleanup into a blind delete. It scans paths you choose, explains why
each item matched, lets you review the plan in a keyboard-driven terminal UI,
and moves selected items to the operating system trash.

## AI-Friendly by Design

Cleanr gives local coding agents deterministic, versioned JSON evidence through
`cleanr analyze` while keeping cleanup authority with the user. Agents can
inspect recommendation states, decision codes, risk notes, and scan integrity
without parsing terminal output or deleting files. Raw paths and reports stay
local unless you explicitly choose to share them.

Install the cross-agent `cleanr-review-disk-cleanup` skill directly from GitHub:

```bash
npx skills add drl990114/cleanr@cleanr-review-disk-cleanup -g
```

The skill checks whether the Cleanr CLI is available, installs `cleanr-cli`
globally when needed, and guides a local, read-only analysis workflow. See
[Evidence and privacy](docs/docs/evidence-and-privacy.md) for supported agents,
the report contract, and privacy guidance.

## Features

- Keyboard-driven scan, review, cleanup, and restore workflow.
- Built-in rules for common developer caches, browser caches, application
  caches, build output, package-manager caches, large downloads, logs, and
  temporary files.
- Reviewable cleanup plans with size, confidence, reason, and risk notes for
  every candidate.
- A local-only `cleanr analyze` JSON contract so a user's local coding agent
  can inspect deterministic evidence without receiving cleanup authority.
- Conservative default selection: only high-confidence items from built-in or
  trusted rules can be preselected.
- Safer execution through trash-based cleanup, final pre-clean validation,
  overlap removal, and local cleanup manifests.
- Restore history for macOS Trash, Windows Recycle Bin, and
  Freedesktop-compatible Linux trash implementations.
- Declarative plugin support for custom cleanup rules and translations.
- Native packages for macOS, Linux, and Windows, with npm, Cargo, and GitHub
  Release installation options.
- English and Simplified Chinese UI support.

## Install

Install with npm:

```bash
npm install --global cleanr-cli
```

Install with Cargo:

```bash
cargo install cleanr-cli
```

You can also download a prebuilt binary from
[GitHub Releases](https://github.com/drl990114/cleanr/releases).

## Start

Run Cleanr in the directory you want to inspect:

```bash
cleanr
```

Or pass one or more scan roots:

```bash
cleanr ~/projects ~/Downloads
```

Inside the TUI, press `s` to scan, `r` to review candidates, `space` to select
or deselect an item, and `c` to confirm cleanup. Use `/scan --global` to inspect
known system cleanup locations and `/restore` to restore a previous cleanup run
when the platform supports it.

Press `?` in the TUI for keyboard help.

For a local coding agent, use the read-only analysis command and keep its JSON
on the machine unless you deliberately redact it first:

```bash
cleanr analyze ~/projects > cleanr-analysis.json
```

The report is evidence for review, not a cleanup instruction. Cleanr does not
offer an agent execution command; a person still reviews and confirms cleanup
inside the TUI.

The TUI, `analyze`, `plan`, and `dry-run` share
`[recommendations].preselect_after_days` from `cleanr.toml` (90 days by
default; `0` disables the age gate).

## Safety Model

Cleanr does not clean anything just because it was found. The plan remains
editable before execution, selected paths are validated again immediately
before cleanup, and items are moved to the operating system trash rather than
permanently deleted.

Restore is best-effort and depends on the system trash. Do not empty the trash
until you are confident the cleanup was correct.

## Learn More

- [Quick start](docs/docs/quick-start.md)
- [Using Cleanr](docs/docs/using-cleanr.md)
- [Safety and recovery](docs/docs/safety-and-recovery.md)
- [Configuration](docs/docs/configuration.md)
- [Plugins](docs/docs/plugins.md)

## Contributing

Development setup, checks, documentation workflow, and release notes live in
[CONTRIBUTING.md](CONTRIBUTING.md).

## Acknowledgements

Cleanr includes code adapted from
[Byron/dua-cli](https://github.com/Byron/dua-cli), an MIT-licensed disk usage
analyzer by Sebastian Thiel and contributors.

## License

Cleanr is licensed under the [MIT License](LICENSE).
