---
description: A contributor-oriented map of Cleanr's crates, data flow, and safety boundaries.
---

# Architecture

This page is for contributors and plugin authors who need to understand where
Cleanr's behavior lives. If you only want to use the application, start with
[Using Cleanr](./using-cleanr).

## Workspace crates

| Crate | Path | Responsibility |
| --- | --- | --- |
| `cleanr-core` | `crates/core` | Scan entries, rule hits, evidence reports, cleanup plans, safety policy, and manifest models |
| `cleanr-cli` | `crates/cli` | Command-line entry point, argument parsing, read-only analysis, config commands, and plugin management |
| `cleanr-tui` | `crates/tui` | Interactive terminal application, state machine, views, and background task orchestration |
| `cleanr-fs` | `crates/fs` | Filesystem scanning, metadata collection, cancellation, and `ScanReport` generation |
| `cleanr-rules` | `crates/rules` | Built-in and plugin rule loading, validation, matching, and the `RuleRegistry` |
| `cleanr-plugin-api` | `crates/plugin-api` | Versioned manifests, discovery, compatibility, trust, schemas, and diagnostics |
| `cleanr-config` | `crates/config` | Configuration schema, defaults, validation, and atomic persistence |
| `cleanr-i18n` | `crates/i18n` | Built-in and external language packs, fallback, and runtime locale switching |
| `cleanr-tasks` | `crates/tasks` | Cleanup execution, system trash integration, restore, and manifest persistence |

## Runtime data flow

```text
CLI arguments + config
        │
        ▼
TUI state machine ── starts scan worker
        │                    │
        │                    ▼
        │              cleanr-fs entries
        │                    │
        │                    ▼
        │              cleanr-rules hits
        │                    │
        ▼                    ▼
User review ◄──────── cleanup plan
        │
        ▼
Workflow service / local authorization
        │
        ▼
pending manifest → cleanr-tasks validation → system trash → manifest update
        │
        └────────────────────→ restore → restore manifest
```

The plan builder removes overlapping candidates before it computes selected and
total reclaimable space.

## TUI boundaries

`cleanr-tui` keeps rendering separate from I/O:

- `app/` owns state transitions and user actions;
- `effects/` owns background scanning, persistence, cleanup, and restore work;
- `views/` renders immutable application state;
- `commands/` maps action requests to palette entries;
- `terminal.rs` owns raw mode, input polling, drawing, and terminal cleanup.

Views do not walk the filesystem. Background workers report results back to the
state machine, which keeps cancellation and partial failure visible to the UI.

## External local AI boundary

`cleanr analyze` is a CLI-only, read-only boundary for an external agent on the
same machine. It scans, applies the deterministic rule and recommendation
policy, and prints a versioned `AnalysisReport` JSON document. It does not
create a cleanup plan, grant authorization, or move files. An agent may use
that evidence to explain or propose a review, while the user still selects and
confirms cleanup in Cleanr.

The report includes raw local paths, scan roots, rule metadata and explanatory
text, and diagnostics. It is deliberately a local contract rather than a
remote transport object; a future remote-sharing feature would require a
separate redacted DTO and threat model.

## Safety boundaries

Safety is enforced in more than one layer:

- `cleanr-rules` limits automatic selection to high-confidence trusted rules;
- `cleanr-core` excludes protected and overlapping candidates while building
  the plan and records directory fingerprints for selected trees;
- `cleanr-tasks` requires local authorization, journals cleanup before moving
  files, and revalidates each target at execution time;
- the trash backend records rollback information where the platform supports
  it;
- `cleanr analyze` is read-only and cannot mint cleanup authorization or invoke
  cleanup;
- no embedded model or provider receives scan evidence through this interface.

Plugins remain declarative by default. Their manifests, rules, and translations
are parsed as data; dynamic hooks are a separately trusted external-command
capability.

## Persistent data

Configuration uses the platform config directory. Cleanup and restore
manifests use the platform state directory under `cleanr/`, with separate
`runs/` and `restores/` folders.

`cleanr-tasks` owns manifest persistence through `ManifestRepository`, which
keeps listing, lookup, and atomic writes behind one API for the TUI and CLI.

Writes use temporary files and atomic replacement so a partial write does not
silently replace a valid config or manifest.
