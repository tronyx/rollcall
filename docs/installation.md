# 📦 Installation

[← All documentation](README.md)

Rollcall is one container (the API server with the web interface built in) plus a PostgreSQL 18 database.

## 📋 Requirements

- To run it: Docker with Docker Compose.
- To build it from source: Rust (stable, 1.85 or newer), Node.js 26, and Docker for the database.

The image runs on `amd64` and `arm64`, so a Raspberry Pi 4 or 5, or a NAS with an ARM processor, works too.

## 🐳 With Docker Compose

Save this as `docker-compose.yml` and change the password:

```yaml
services:
  db:
    image: postgres:18
    restart: unless-stopped
    environment:
      POSTGRES_USER: rollcall
      POSTGRES_PASSWORD: change-me
      POSTGRES_DB: rollcall
    volumes:
      - pgdata:/var/lib/postgresql
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U rollcall -d rollcall"]
      interval: 5s
      retries: 10

  rollcall:
    image: ghcr.io/tronyx/rollcall:develop
    restart: unless-stopped
    depends_on:
      db:
        condition: service_healthy
    environment:
      DATABASE_URL: postgres://rollcall:change-me@db:5432/rollcall
    ports:
      - "8080:8080"
    volumes:
      - uploads:/app/uploads

volumes:
  pgdata:
  uploads:
```

Then start it:

```bash
docker compose up -d
```

Open <http://localhost:8080>. Rollcall creates and updates its tables when it starts. There are no user accounts, so do not expose port 8080 to the internet. See the [security policy](../.github/SECURITY.md).

> [!NOTE]
> PostgreSQL 18 images keep their data under `/var/lib/postgresql/18/docker`, so the volume is mounted at `/var/lib/postgresql`, not at `/var/lib/postgresql/data` as with older versions.
>
> The `uploads` volume holds the photos you upload. Without it, they are lost when the container is replaced.

## 🛠️ From source

```bash
git clone https://github.com/tronyx/rollcall.git
cd rollcall

# 1. Database
docker compose up -d db
cp .env.example backend/.env

# 2. Backend on :8080 (applies migrations on start)
cd backend
cargo run

# 3. Frontend on :5173, proxying /api to :8080
cd ../frontend
npm install
npm run dev

# Optional: sample data
../scripts/seed-dev.sh
```

To build the whole image yourself instead, run `docker compose --profile full up -d --build` from the repository root.
