# 🩺 Troubleshooting

[← All documentation](README.md)

## 🐘 The server can't connect to the database

Check `DATABASE_URL`. In Docker Compose the host is the service name (`db`), not `localhost`. The server tries to connect once when it starts and exits if the database isn't ready, so start the database first, or use `restart: unless-stopped` and the `depends_on` health check from [Installation](installation.md#-with-docker-compose).

## 📂 PostgreSQL 18 won't start with an old volume

A volume made by PostgreSQL 17 or older, mounted at `/var/lib/postgresql/data`, does not work with PostgreSQL 18. Mount the volume at `/var/lib/postgresql`. To keep old data, dump it with the old version (`pg_dump`) and restore it into the new one.

## 🔗 The link parser fills in almost nothing

- The store probably blocked the request. The form shows a warning when it did. Try the manufacturer's own product page, or fill in the form by hand.
- A link to a server on your own network is refused unless you set `SCRAPER_ALLOW_PRIVATE_HOSTS=true`.
- Set `ANTHROPIC_API_KEY` to let the LLM step fill the gaps on pages that the parser can reach but can't fully read.

## 🧱 `cargo build` fails with "relation does not exist" or a query error

sqlx checks queries against a live database when `DATABASE_URL` is set. Either run the migrations on that database first (`cargo run` does it), or build against the committed metadata with `SQLX_OFFLINE=true`.

## 🔐 Locked out

Another admin can reset your password in **Settings › Accounts**. If there's no one else, run `docker compose exec rollcall rollcall reset-password USERNAME`. It prints a new random password and signs the account out everywhere.

## 🧾 The slicer script says HTTP 401

The script needs an API token. Create one under **Settings › Your account** and add `--token` with it to the script's line in the slicer, or set `ROLLCALL_TOKEN`.

## 🪲 Still stuck?

[Open an issue](https://github.com/tronyx/rollcall/issues/new/choose) with the server log (`docker compose logs rollcall`).
