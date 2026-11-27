# 🐳 Docker

[← All documentation](README.md)

The image contains the Rollcall server and the built web interface. It runs on `amd64` and `arm64`, and needs a PostgreSQL 18 database next to it. [Installation](installation.md#-with-docker-compose) has a complete `docker-compose.yml`.

| Registry | Image |
| --- | --- |
| GitHub | `ghcr.io/tronyx/rollcall` |
| Docker Hub | `tronyx/rollcall` |

The `develop` tag comes from the `develop` branch. The `latest` tag will come from `main` once there is a release. Each week, a new build picks up Debian's security fixes. Before an image is published, it is started next to PostgreSQL 18 and tested, and [Trivy](https://trivy.dev) scans it for known vulnerabilities. An image with a critical vulnerability that has a fix is not published.

## 🔄 Updating

```bash
docker compose pull
docker compose up -d
```

Rollcall applies any new database migrations when it starts.

## 💾 Backups

Everything is in the database. To make a backup:

```bash
docker compose exec db pg_dump -U rollcall rollcall > rollcall-backup.sql
```

To restore it into an empty database:

```bash
docker compose exec -T db psql -U rollcall rollcall < rollcall-backup.sql
```

## 🔒 The container

The server runs as the unprivileged user `rollcall` (UID 10001) and listens on port 8080. Settings are environment variables; see [Configuration](configuration.md).
