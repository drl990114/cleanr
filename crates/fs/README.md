# cleanr-fs

Filesystem scanning for cleanr.

This crate walks configured paths, collects file metadata, tracks hardlinks, and
produces a `ScanReport` used by the rule engine and the TUI.

Scanning is single-pass, supports cancellation and glob-based ignores, and can
discover known global developer cache roots.
