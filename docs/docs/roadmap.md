---
description: See the current direction for Cleanr without confusing planned work with released behavior.
---

# Roadmap

This roadmap describes direction, not a compatibility promise. For behavior
you can rely on today, use the user guide and the release notes.

## Current foundation

The project already includes:

- non-overlapping cleanup plans with accurate selected-space totals;
- system-trash cleanup and manifest-based restore on macOS, Windows, and
  Freedesktop-compatible Linux desktops;
- per-item cleanup and restore results;
- cancellable single-pass scanning, glob ignores, and known-cache discovery;
- execution-time path, type, file size, directory fingerprint,
  modification, and protected-path checks;
- a local authorization boundary that model-generated actions cannot bypass;
- non-interactive scan JSON, plan export, dry-run, and restore commands;
- versioned, declarative plugin bundles with compatibility and trust metadata.

## Near-term: clearer control and recovery

Planned work includes:

- clearer retries for partial cleanup and restore failures;
- large-tree performance benchmarks and broader cross-platform restore tests;
- more visible access to manifest and diagnostic details from the TUI.

## Developer-cache intelligence

The project intends to deepen developer-specific guidance:

- broader cache coverage for package managers, build tools, IDEs, mobile
  toolchains, and containers;
- scoring that considers safety, reclaimable space, last use, and rebuild cost;
- conservative, balanced, and maximum-space presets;
- explanations of how each cache is recreated and whether network access is
  required;
- validated and signed distribution for community rule packs.

## Automation

Potential automation surfaces include executing explicitly reviewed plans,
scheduled diagnostics, and richer machine-readable failure reports.

AI remains an explanation and rule-authoring layer. It should not grant itself
cleanup permission or turn a suggestion into an unattended destructive action.
