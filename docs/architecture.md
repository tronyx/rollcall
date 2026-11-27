# 🏗️ Architecture

[← All documentation](README.md)

Self-hosted 3D printer filament inventory. One user per instance (no accounts in v1).

## 🧱 Stack decisions

| Area | Choice | Why |
| --- | --- | --- |
| Backend | Rust, `axum` 0.8, `tokio`, `serde`, `tower-http` (CORS, tracing, static files) | Small binary, low memory on a home server or Pi |
| DB access | `sqlx` with `query!`/`query_as!` macros, offline mode (`.sqlx/` committed) | Compile-time checked SQL; CI builds without a live DB via `SQLX_OFFLINE=true` |
| Database | PostgreSQL 18 | `uuidv7()` time-ordered keys (new in 18), `UNIQUE NULLS NOT DISTINCT`, `pg_trgm` search |
| Scraping | `reqwest` (rustls) + `scraper` | Pure Rust, no OpenSSL headaches in Docker |
| Frontend | SvelteKit 3 + Svelte 5 (runes) + TypeScript 6 + Tailwind 4 + Vite 8, `adapter-static` SPA | Built to static files and served by the Rust binary, so deployment is one container plus Postgres |
| Errors | `thiserror` for domain errors, a single `AppError` mapped to JSON responses | Consistent `{ "error": { "code", "message" } }` |
| Config | env vars via `dotenvy` + a typed `Config` struct | `DATABASE_URL`, `BIND_ADDR`, `STATIC_DIR`, scraper settings, optional `ANTHROPIC_API_KEY` |

During development the SvelteKit dev server (port 5173) proxies `/api` to the backend (port 8080).

## 🔗 URL parser strategy

`POST /api/filaments/parse-url` runs extractors from most to least trustworthy and fills each field from the first one that finds it. It never fails because data is missing; it returns partial data, the list of missing fields, the source of every field, and plain-language warnings.

1. **Safety check**: http(s) only; hosts that resolve to private, loopback or link-local addresses are refused (and redirects to private IP literals). A host that can't be resolved skips the fetch and falls back to URL hints.
2. **Fetch**: browser-like user agent, 12 s timeout, 4 MB cap, 5 redirects. Captcha and access-denied pages (Amazon robot check, Cloudflare challenge, 403/429/503) are flagged as `blocked`.
3. **Shopify product JSON**: stores on Shopify (Polymaker, eSUN, HATCHBOX and many more) serve `/products/<handle>.js`, which gives vendor, title, the selected `?variant=` color, price and description.
4. **schema.org JSON-LD** `Product` (including `@graph`, `additionalProperty`).
5. **Site adapters**: Amazon (title, byline brand, overview and tech-spec tables, bullets, hi-res image, price).
6. **Generic spec tables**: `<table>` rows, `<dl>` lists and "Key: Value" bullets.
7. **OpenGraph and meta tags**.
8. **Title regex**: brand, material, color phrase ("Galaxy Blue"), weight, diameter.
9. **Visible page text regex** (scripts and styles excluded): temperature ranges, speeds, drying conditions.
10. **Optional LLM pass** with structured output, only when `ANTHROPIC_API_KEY` is set and essentials are still missing. It only fills gaps.
11. **URL hints**: brand from the store domain, material and color from the slug.
12. **Derived**: swatch hex from the color name, canonical brand spelling, product name from a short title.

The frontend highlights fields that came from guesses (amber) or were not found (red). Nothing is saved until the user confirms.

## 🗂️ Repository layout

```text
rollcall/
├── .github/
│   ├── workflows/
│   │   ├── tests.yml            # lint, backend, frontend, Docker image (amd64 + arm64), publish
│   │   ├── scan.yml             # midweek Trivy scan of the published images
│   │   ├── sync-develop.yml     # fast-forwards develop after a merge into main
│   │   ├── sync-wiki.yml        # copies docs/ to the wiki on release
│   │   └── dockerhub-description.yml
│   ├── scripts/docs_to_wiki.py
│   ├── ISSUE_TEMPLATE/          # bug report and feature request forms
│   └── CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, FUNDING.yml, dependabot.yml
├── backend/
│   ├── Cargo.toml
│   ├── .sqlx/                   # sqlx offline query metadata (committed)
│   ├── migrations/
│   │   └── 0001_initial_schema.sql
│   ├── src/
│   │   ├── main.rs              # boot: config, tracing, pool, migrate, serve
│   │   ├── config.rs
│   │   ├── db.rs                # PgPool setup, sqlx::migrate!()
│   │   ├── error.rs             # AppError -> IntoResponse
│   │   ├── routes/              # one module per resource
│   │   │   ├── mod.rs           # Router assembly, /api prefix, static frontend fallback
│   │   │   ├── spools.rs        # CRUD, POST /spools/{id}/weight (quick adjust, log print, weigh-in)
│   │   │   ├── filaments.rs     # list/get, POST /filaments/parse-url
│   │   │   ├── lookups.rs       # brands, materials, locations
│   │   │   └── stats.rs         # dashboard
│   │   ├── models/              # FromRow structs + request/response DTOs
│   │   ├── repo/                # SQL lives here, routes stay thin
│   │   └── scraper/
│   │       ├── mod.rs           # pipeline + merge + normalization
│   │       ├── draft.rs         # result type, field-by-field merge with sources
│   │       ├── fetch.rs         # HTTP client, SSRF guard, block detection
│   │       ├── structured.rs    # JSON-LD / OpenGraph
│   │       ├── heuristics.rs    # regex extraction
│   │       ├── brands.rs        # domain → brand, canonical spelling
│   │       ├── colors.rs        # color phrases, swatch hex guesses
│   │       ├── llm.rs           # optional Claude API fallback
│   │       └── sites/           # amazon.rs, shopify.rs, spec_table.rs
│   ├── examples/parse_file.rs   # run the parser on a saved page
│   └── tests/
│       ├── api.rs               # #[sqlx::test] end-to-end API tests
│       └── fixtures/            # saved pages for scraper tests (no live network in CI)
├── frontend/
│   ├── package.json             # #lib/* subpath import for src/lib
│   ├── vite.config.ts           # SvelteKit options (adapter-static), /api proxy in dev
│   └── src/
│       ├── app.css              # Tailwind
│       ├── lib/
│       │   ├── api/             # typed fetch client + types mirroring backend DTOs
│       │   ├── components/      # SpoolCard, ColorSwatch, AddSpoolModal, FilterBar, StatTiles
│       │   └── util/            # formatting helpers (+ vitest tests)
│       └── routes/
│           └── +page.svelte     # dashboard + inventory (detail/settings pages come later)
├── Dockerfile                   # multi-stage: node build -> cargo build -> slim runtime
├── docker-compose.yml           # postgres:18 (+ healthcheck, volume); app service under a "full" profile
├── .env.example
├── scripts/seed-dev.sh          # sample data through the API
├── docs/                        # these pages (also published to the wiki)
├── CHANGELOG.md
└── README.md
```

## 🗃️ Data model

See [`backend/migrations/0001_initial_schema.sql`](../backend/migrations/0001_initial_schema.sql). Key choices:

- **Filament vs spool.** `filaments` is the product (brand, material, color, manufacturer specs, scraped source). `spools` is a physical roll with its own weight, price, location and status. A second roll of the same color reuses the filament row and its specs.
- **Materials and locations are tables, not enums**, so you can add "PLA-WOOD" or "AMS Slot 5" from the UI. Status and event kinds are enums because code branches on them.
- **Weights are net grams** (`NUMERIC(8,1)`). Tare is stored per spool so a scale weigh-in can be converted: `remaining = gross - tare`.
- **Weight log** (`spool_weight_events`) records every deduction or weigh-in, written in the same transaction as the update to `spools.remaining_weight_g`.
- **Tuned overrides** (`tuned_settings`) are per filament per printer, with an optional "any printer" row. The `effective_print_settings` view resolves printer override, then general override, then manufacturer value.
- **Photos** (`photos`) belong to exactly one spool, filament, or piece of equipment (a `CHECK` on `num_nonnulls`) and are deleted with it. The image is a file named `<id>.<ext>` in `UPLOAD_DIR`; the row keeps its type, size, and caption. The type is read from the file's first bytes, and the file is served with `nosniff` and a sandboxing CSP. At startup, files no row names are removed.
- **Keys are UUIDv7** (`uuidv7()`, PostgreSQL 18), so new rows sort by creation time and index inserts stay local.
- **TD value** (HueForge transmission distance) is stored per filament.
- **Dashboard** reads `inventory_stats` and `spool_inventory`; the low-stock threshold lives in `app_settings` (default 200 g).
