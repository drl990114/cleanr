---
sidebar_position: 6
description: Understand why Cleanr marks a path, how confidence affects selection, and what built-in rules cover.
---

# Rules and confidence

Cleanr does not decide that a path is removable from its name alone. It matches
scanned entries against versioned **rule packs** that explain what the item is,
why it can be removed, and what rebuilding it may cost.

## What you see for each candidate

Each rule match includes:

| Field | Meaning |
| --- | --- |
| Label | Human-readable name, such as “Rust target directory” |
| Category | Group such as `build-cache`, `package-cache`, or `downloads` |
| Confidence | `High`, `Medium`, or `Low` |
| Reason | Why the path is considered a cleanup candidate |
| Risk note | What may break, slow down, or require a download afterward |
| Default selection | Whether the rule asks to preselect the item |

When multiple rules match one entry, Cleanr chooses the best hit using trust,
default selection, and confidence. The final plan removes overlapping parent
and child candidates so space is not counted twice.

## Confidence is not a guarantee

| Level | How to treat it |
| --- | --- |
| `High` | Usually generated or downloadable data; still review unfamiliar paths |
| `Medium` | Often rebuildable, but may be expensive or contain local-only state |
| `Low` | May be user data and always needs careful manual review |

Only a `High` confidence rule with `default_selected = true` from a built-in or
trusted source can preselect an item.

## Built-in rule packs

### `builtin-dev`

Recognizes common generated developer data, including:

- `node_modules`, Rust `target`, Python tool caches, and Next.js caches;
- Cargo, npm, pnpm, Yarn, pip, uv, Gradle, Maven, and Go caches;
- Xcode `DerivedData`.

Most clearly rebuildable entries are high confidence. More ambiguous entries,
such as tox environments and Maven's local repository, are not preselected.

### `builtin-general`

Finds broader candidates that should be reviewed manually:

- files of at least 100 MiB under a Downloads directory;
- `.log` files of at least 50 MiB;
- `.tmp` files of at least 1 MiB.

These rules are intentionally medium or low confidence and start unselected.

## Enable or disable packs

Only IDs in `cleanup.enabled_rule_packs` are loaded:

```toml
[cleanup]
enabled_rule_packs = ["builtin-dev"]
```

Removing `builtin-general` is useful when you want Cleanr to focus only on
developer caches.

Run `/rules` inside the TUI to inspect the active packs and rules.

## Add custom rules

The recommended format is a declarative plugin bundle. See
[Plugins](./plugins) for a complete minimal example, validation commands, and
the trust model.

Legacy loose TOML rule-pack files are still discovered in plugin directories,
but bundles provide version and compatibility metadata and are preferred.
