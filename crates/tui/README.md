# cleanr-tui

Terminal user interface for cleanr.

This crate runs the interactive ratatui-based application: event handling,
screen rendering, command palette, and background scan coordination.

The source is split by responsibility:

- `app/` owns application state, input handling, navigation, task events, and
  action orchestration.
- `effects/` is the boundary for threads, plugin/runtime loading, cleanup and
  restore execution, configuration persistence, and manifest I/O.
- `views/` contains rendering only, split by screen and overlay.
- `commands/` contains command-palette filtering and command metadata helpers.
- `terminal.rs` owns raw-mode setup, the event loop, and terminal restoration.
