# Cleanr

Evidence-first, reviewable disk cleanup for rebuildable caches and system cleanup locations.

[English](readme/en/README.md) | [Simplified Chinese](readme/zh-CN/README.md) | [Documentation](docs/) | [Contributing](CONTRIBUTING.md)

Cleanr helps developers find rebuildable generated files and caches without
turning cleanup into a blind delete. It scans paths you choose, explains why
each item matched, lets you review the plan in a keyboard-driven terminal UI,
and moves selected items to the operating system trash.

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

Install the repository's cross-agent `cleanr-review-disk-cleanup` skill directly
from GitHub:

```bash
npx skills add drl990114/cleanr@cleanr-review-disk-cleanup -g
```

It guides local, read-only analysis and has no cleanup authority. See
[Evidence and privacy](docs/docs/evidence-and-privacy.md) for supported-agent
selection and usage.

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
