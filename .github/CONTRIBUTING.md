# 🤝 Contributing

Thanks for wanting to help! Bug reports, ideas and pull requests are all welcome.

## 🐛 Reporting a bug

Found a security vulnerability? Please don't open an issue: see the [security policy](SECURITY.md).

[Open a bug report](https://github.com/tronyx/rollcall/issues/new?template=bug_report.yml). The form asks for:

- What you did and what happened, with a screenshot for anything in the web interface.
- The product link, if a link was parsed wrongly.
- How you run Rollcall (Docker image or built from source) and which version.
- The server log around the problem (`docker compose logs app --tail 100`).

## 💡 Suggesting a feature

[Open a feature request](https://github.com/tronyx/rollcall/issues/new?template=feature_request.yml) describing what you'd like to do and why. For anything bigger than a small fix, please do this before writing code, so we can agree on the approach and your time isn't wasted.

## 🛠️ Making a change

1. Fork the repository, then clone your fork:

   ```bash
   git clone https://github.com/<your_name>/rollcall.git
   cd rollcall
   ```

2. Create a branch from `develop`:

   ```bash
   git checkout -b my-new-feature develop
   ```

3. Make your change. Keep each pull request to one change.

4. Start PostgreSQL and run the checks. You need Rust (stable), Node 26 and Docker:

   ```bash
   docker compose up -d db
   cp .env.example backend/.env

   cd backend
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test

   cd ../frontend
   npm ci
   npm run lint
   npm run check
   npm test
   npm run build
   ```

   `cargo test` needs `DATABASE_URL` (from `backend/.env`): each database test gets a fresh database of its own and runs the migrations in it. The parser tests use saved pages in `backend/tests/fixtures/` and never go online.

   If you add a migration or change a query, update the saved query metadata so CI and the Docker build can compile without a database, and commit it:

   ```bash
   cargo install sqlx-cli --no-default-features --features rustls,postgres
   cd backend
   cargo sqlx prepare -- --all-targets
   ```

   GitHub also lints the workflows ([actionlint](https://github.com/rhysd/actionlint)), the `Dockerfile` ([hadolint](https://github.com/hadolint/hadolint), set up in `.hadolint.yaml`) and the Markdown files ([markdownlint](https://github.com/DavidAnson/markdownlint-cli2), set up in `.markdownlint.json`). If you change any of those, you can run the same checks locally:

   ```bash
   docker run --rm -v "$PWD:/repo" -w /repo rhysd/actionlint:1.7.12
   docker run --rm -v "$PWD:/repo" -w /repo hadolint/hadolint hadolint --config .hadolint.yaml Dockerfile
   docker run --rm -v "$PWD:/workdir" davidanson/markdownlint-cli2 "**/*.md"
   ```

   If you change the `Dockerfile`, build the image and try it. GitHub also builds and tests it on amd64 and arm64:

   ```bash
   docker compose --profile full up -d --build
   ```

5. Commit, push, and open a pull request against `develop`:

   ```bash
   git add -A
   git commit -m "Add some feature"
   git push origin my-new-feature
   ```

## 📏 Code style

- **Formatting is automatic.** `cargo fmt` for Rust and Prettier (`npm run format`) for the frontend. CI checks both.
- **No warnings.** Clippy runs with `-D warnings` and `svelte-check` with `--fail-on-warnings`. Fix the code rather than silencing a lint.
- **SQL lives in `backend/src/repo/`**, written with sqlx's checked macros (`query!`, `query_as!`), never built from strings. Routes stay thin.
- **Schema changes are new migrations.** Never edit a migration that has been released; add a new file in `backend/migrations/`.
- **The parser never fails because data is missing.** A new extractor fills only the fields it is sure of, and gets a saved fixture page and a test in `backend/tests/fixtures/`.
- **Mobile first.** The interface is used next to a printer, on a phone. Check new screens at phone width, with large enough tap targets, and in dark mode.
- **Doc comments, briefly.** Say what something does and, where it isn't obvious, why.
- **Add or update tests** for any change in behavior: unit tests next to the code, API tests in `backend/tests/api.rs`, frontend helpers with Vitest.
- **Update the docs** if you add a setting or change what Rollcall does. The details are in `docs/`; the README only has what a newcomer needs. The wiki is a copy of `docs/`, made with each release, so don't edit it: changes there are overwritten.
- **Add a line to the changelog** under **Unreleased** in `CHANGELOG.md`, saying what changes for someone using Rollcall.
