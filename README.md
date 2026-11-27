# Rollcall

Self-hosted inventory for 3D printer filament. Track every spool, how much is left, where it lives, and the print settings that work for you. Paste an Amazon or manufacturer link and Rollcall fills in the brand, material, color and recommended temperatures.

- **Inventory**: brand, material, color swatch, weights, price, purchase date, storage location (dry box, AMS slot) and status (sealed, active, drying, empty).
- **Quick weight updates**: `−10g`, `−50g` and "Log print" buttons on every spool, with undo, plus scale weigh-ins that subtract the empty-spool weight. Every change is logged.
- **Link parser**: `POST /api/filaments/parse-url` reads product pages (Shopify product JSON, schema.org data, Amazon layout, spec tables, page text, the URL itself) and returns whatever it finds. Blocked pages still give you a partly filled form. An optional LLM step fills remaining gaps.
- **Print profiles**: manufacturer ranges stored with each filament, plus your own tuned overrides per printer.
- **Dashboard**: spool count, kilograms remaining, low-stock alerts (under 200 g by default), and search and filters by material, brand, color and location.

Stack: Rust (axum, sqlx, tokio, reqwest, scraper), SvelteKit + TypeScript + Tailwind CSS, PostgreSQL 18. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Quick start (Docker)

```sh
docker compose --profile full up -d --build
```

Open http://localhost:8080. Migrations run automatically when the server starts.

## Development

Requirements: Rust (stable), Node 22, Docker (for PostgreSQL).

```sh
# 1. Database
docker compose up -d db
cp .env.example backend/.env

# 2. Backend on :8080 (applies migrations on start)
cd backend
cargo run

# 3. Frontend on :5173, proxying /api to :8080
cd frontend
npm install
npm run dev

# Optional: sample data
scripts/seed-dev.sh
```

### Tests and checks

```sh
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test            # needs DATABASE_URL; each DB test gets its own fresh database

cd frontend
npm run lint && npm run check && npm test && npm run build
```

### Database changes

Queries use sqlx's compile-time checked macros. After adding a migration or changing a query, regenerate the offline metadata so CI (and Docker builds) can compile without a database:

```sh
cargo install sqlx-cli --no-default-features --features rustls,postgres
cd backend
sqlx migrate add <name>          # creates migrations/<timestamp>_<name>.sql
cargo sqlx prepare -- --all-targets
```

Commit the `backend/.sqlx/` changes along with your code.

## Branching and CI

`develop` is the default branch.

1. Branch from `develop` (`feature/…`, `fix/…`).
2. Open a pull request into `develop`.
3. GitHub Actions runs on every push to `develop` and every pull request targeting `develop`:
   - **Backend**: rustfmt, clippy against the committed sqlx metadata, and tests against a PostgreSQL 18 service.
   - **Frontend**: prettier, svelte-check, vitest and a production build.

   Each workflow only runs when files in its folder change.
4. Merge once checks pass.

## Configuration

All settings are environment variables. See [.env.example](.env.example).

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | required | PostgreSQL connection string |
| `BIND_ADDR` | `0.0.0.0:8080` | Listen address |
| `STATIC_DIR` | unset | Serve the built frontend from this folder |
| `CORS_ORIGIN` | unset | Extra allowed origin |
| `SCRAPER_TIMEOUT_SECS` | `12` | Page fetch timeout |
| `SCRAPER_ALLOW_PRIVATE_HOSTS` | `false` | Allow parsing pages on private networks |
| `ANTHROPIC_API_KEY` | unset | Turns on the LLM fallback in the link parser |
| `LLM_MODEL` | `claude-opus-5-5` | Model for the LLM fallback |

## Repository layout

```
backend/    Rust API server, migrations, sqlx metadata, tests
frontend/   SvelteKit single-page app
docker/     Production Dockerfile
scripts/    Dev helpers
docs/       Architecture notes
```
