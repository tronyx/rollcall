# 🧪 How it is tested

[← All documentation](README.md)

## 🦀 Backend

```bash
cd backend
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`cargo test` needs `DATABASE_URL` pointing at a PostgreSQL 18 server (`docker compose up -d db` gives you one). Each database test (`#[sqlx::test]`) gets its own fresh database with the migrations applied, so tests never see each other's data.

- **API tests** (`backend/tests/api.rs`) call the real router: a spool's whole life (adding it, weight changes and undo, the dashboard numbers), search and filters, and validation errors.
- **Parser tests** run the link parser on saved product pages in `backend/tests/fixtures/`. They never use the network, so a store changing its page can't break the build.
- **Unit tests** next to the code cover the color and brand tables, the title and text patterns, and the safety check for private addresses.

Queries are checked against the database schema when the code compiles. After you change a query or add a migration, run `cargo sqlx prepare -- --all-targets` and commit `backend/.sqlx/`. CI compiles against those files, without a database.

## 🖥️ Frontend

```bash
cd frontend
npm run lint && npm run check && npm test && npm run build
```

Prettier, `svelte-check` (TypeScript), Vitest unit tests for the formatting helpers, and a production build.

## ⚙️ On GitHub

`.github/workflows/tests.yml` runs on every push and pull request:

| Job | What it checks |
| --- | --- |
| Lint | actionlint (with shellcheck) on the workflows, hadolint on the Dockerfile, and markdownlint on the documentation |
| Backend | rustfmt, clippy, and the tests against a PostgreSQL 18 service |
| Frontend | The checks above |
| Docker image | Builds the image on `amd64` and `arm64`, starts it next to PostgreSQL 18, checks the API, the web interface, and saving a spool, and scans it with Trivy |
| Publish | On `develop` and `main` only: tags the tested images (see [Docker](docker.md)) |

It also runs each Monday, so a new base image or dependency that breaks the build shows up without a push.
