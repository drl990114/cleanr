---
description: Build, test, lint, and document Cleanr as a contributor.
---

# Development

## Prerequisites

- Rust 1.94.1 or a compatible newer toolchain
- Node.js 20 or later for the documentation site
- pnpm 10

## Build the workspace

Build the workspace:

```bash
cargo build
```

Build the release binary:

```bash
cargo build --release
```

The CLI binary is `target/release/cleanr`.

## Run the same checks as CI

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
cargo build --workspace --all-targets --all-features --locked
```

Validate generated JSON Schemas:

```bash
cargo run --locked -p cleanr-cli -- plugin schema manifest >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema index >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema rules >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema language >/dev/null
cargo run --locked -p cleanr-cli -- plugin schema config >/dev/null
```

## Run the documentation site

```bash
cd docs
pnpm install
pnpm start
```

The development server is available at `http://localhost:3000/` by default.

Before submitting documentation changes:

```bash
pnpm typecheck
pnpm build
```

## Keep English and Chinese in sync

- English source pages live in `docs/docs/`.
- Simplified Chinese pages live in
  `docs/i18n/zh-Hans/docusaurus-plugin-content-docs/current/`.
- Shared UI strings live in the locale JSON files under
  `docs/i18n/zh-Hans/`.

After changing translated React text, navbar labels, footer labels, or sidebar
categories, regenerate translation keys:

```bash
pnpm docusaurus write-translations --locale zh-Hans
```

Then translate new entries and build both locales.

## Contribution checklist

- Add or update tests for behavior changes.
- Update user documentation when commands, defaults, safety behavior, or
  supported platforms change.
- Update both English and Simplified Chinese pages in the same change.
- Keep examples executable and avoid documenting planned behavior as if it
  already exists.
- Run formatting, Clippy, workspace tests, type-checking, and the docs build.
