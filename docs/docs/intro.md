---
sidebar_position: 1
description: Understand what Cleanr cleans, how it keeps cleanup reviewable, and where to begin.
---

# Cleanr overview

Cleanr helps you find and remove **rebuildable caches and reviewable system
cleanup candidates** without turning disk cleanup into a leap of faith. It runs
in your terminal, shows why each item matched, and lets you decide what is
moved to the system trash.

Typical candidates include:

- project dependencies such as `node_modules`;
- build output such as Rust `target` directories and Xcode `DerivedData`;
- package-manager caches for Cargo, npm, pnpm, pip, Gradle, and other tools;
- browser and application caches from known user-level cleanup locations;
- large downloads, logs, and temporary files that require manual review.

## The basic workflow

Cleanr separates discovery from deletion:

1. **Scan** one or more directories.
2. **Review** the matched candidates and their risk notes.
3. **Select** only the items you want to remove.
4. **Confirm** the cleanup.
5. **Restore** a previous cleanup run if needed.

Nothing is cleaned just because it was found. High-confidence, rebuildable
items may be preselected, but the plan remains editable before execution.

## Safety at a glance

- Cleanup moves items to the operating system trash; it does not permanently
  delete them.
- Cleanr removes overlapping parent and child candidates before calculating
  reclaimable space.
- Targets are checked again immediately before cleanup. Changed files,
  symbolic links, paths outside the scan roots, and protected Cleanr data are
  rejected.
- Each cleanup and restore writes a local manifest so the result is auditable.
- Plugins are declarative data files by default; dynamic hooks require explicit trust.

See [Safety and recovery](./safety-and-recovery) for the exact guarantees and
restore limitations.

## Is Cleanr a good fit?

Cleanr is designed for developers who want to inspect generated files and
caches from a keyboard-driven interface. It is not a general-purpose system
optimizer, registry cleaner, or unattended deletion service.

## Start here

- [Quick start](./quick-start): install Cleanr and complete a first cleanup.
- [Using Cleanr](./using-cleanr): learn the workflow, shortcuts, and commands.
- [Evidence and privacy](./evidence-and-privacy): use the local, read-only
  analysis contract safely with another local agent.
- [Configuration](./configuration): change scan, cleanup, language, and theme.
- [Troubleshooting](./troubleshooting): resolve common startup, scan, and
  restore problems.
