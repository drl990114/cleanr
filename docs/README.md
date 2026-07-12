# Cleanr Documentation

This directory contains the Docusaurus documentation site for Cleanr.

- English source pages: [`docs/`](docs/)
- Simplified Chinese pages: [`i18n/zh-Hans/docusaurus-plugin-content-docs/current/`](i18n/zh-Hans/docusaurus-plugin-content-docs/current/)
- Site configuration: [`docusaurus.config.ts`](docusaurus.config.ts)
- Sidebar configuration: [`sidebars.ts`](sidebars.ts)

## Develop Locally

Install dependencies and start the documentation server:

```bash
pnpm install --frozen-lockfile
pnpm start
```

Run the TypeScript check before submitting documentation changes:

```bash
pnpm typecheck
```

Keep English and Simplified Chinese content synchronized. Update navigation or
translation keys when adding, renaming, or removing pages. See the repository
[`CONTRIBUTING.md`](../CONTRIBUTING.md) for the complete documentation workflow.
