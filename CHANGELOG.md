# 📜 Changelog

The notable changes to Rollcall, newest first.

The project doesn't use version numbers. Each entry is one merge into `main`, which always has the latest release, and links to its pull request for the full details. Changes waiting on `develop` are listed under **Unreleased**.

## Unreleased

### Added

- The first version of Rollcall: a spool inventory with brands, materials, colors, weights, prices, locations, and statuses, backed by PostgreSQL 18.
- Quick weight updates (`−10g`, `−50g`, **Log print**) with undo, scale weigh-ins through the API, and a history of every change.
- The link parser, which fills in the add-spool form from Shopify stores, schema.org data, Amazon pages, spec tables, page text, the link itself, and an optional LLM step.
- A dashboard with the spool count, kilograms left, low-stock alerts, and search and filters.
- A Docker image for `amd64` and `arm64`, published to `ghcr.io/tronyx/rollcall` as `develop`.

### Project

- The frontend now uses SvelteKit 3, Vite 8, vite-plugin-svelte 7, TypeScript 6, Vitest 5 and Node.js 26. The SvelteKit settings moved from `svelte.config.js` into `vite.config.ts`, and `$lib` imports became `#lib` (a `package.json` subpath import), as SvelteKit 3 requires. Dependabot skips TypeScript 7 for now, since svelte-check can't use it yet.

- The license is now MIT, like the author's other projects.
- Added a code of conduct, a contributing guide, a security policy, issue forms, a pull request template, and GitHub Sponsors.
- One `tests.yml` workflow lints the workflows, the Dockerfile, and the documentation, tests the backend and the frontend, builds and smoke-tests the image on both architectures, scans it with Trivy, and publishes it. A midweek scan checks the published images again.
- Dependabot keeps the actions, the base images, the crates, and the npm packages up to date, with pull requests into `develop`.
- `docs/` is copied to the wiki on each release, and `develop` is fast-forwarded after each merge into `main`.
