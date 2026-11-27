# 🧵 Rollcall

[![Tests](https://img.shields.io/github/actions/workflow/status/tronyx/rollcall/tests.yml?branch=develop&label=tests)](https://github.com/tronyx/rollcall/actions/workflows/tests.yml?query=branch%3Adevelop)
[![PostgreSQL 18](https://img.shields.io/badge/postgresql-18-blue)](docs/installation.md#-requirements)
[![License: MIT](https://img.shields.io/badge/license-MIT-green)](LICENSE.md)

Rollcall is a self-hosted inventory for 3D printer filament. It tracks every spool, how much is left, where it lives, and the print settings that work for you.

Paste an Amazon or manufacturer link, and Rollcall fills in the brand, material, color, and recommended temperatures. You check the form and save it.

> [!WARNING]
> This is beta software. It is new, and the database layout may still change before the first release. Keep [backups](docs/docker.md#-backups) of anything you care about.

## ✨ Features

- 📦 **Inventory**: brand, material, color swatch, weights, price, purchase date, storage location (dry box, AMS slot), and status (sealed, active, drying, empty, archived).
- ⚖️ **Quick weight updates**: `−10g`, `−50g`, and **Log print** buttons on every spool, with undo. Every change is logged.
- 🔗 **Link parser**: reads Shopify stores, schema.org data, Amazon pages, spec tables, and the page text. A blocked page still gives you a partly filled form. An optional LLM step fills the remaining gaps. See [Link parser](docs/link-parser.md).
- 🖨️ **Print profiles**: the manufacturer's temperature ranges stored with each filament, and the HueForge TD value.
- 📊 **Dashboard**: spool count, kilograms left, low-stock alerts (under 200 g by default), and search and filters by material, brand, color, and location.
- 🐳 **One container**: the server and the web interface in one image for `amd64` and `arm64`, next to PostgreSQL 18.

## 🚀 Quick start

### 🐳 With Docker

The image is on GitHub as `ghcr.io/tronyx/rollcall`. [Installation](docs/installation.md#-with-docker-compose) has a complete `docker-compose.yml` with the database. Then:

```bash
docker compose up -d
```

Open <http://localhost:8080>. Rollcall sets up its database tables when it starts. It has no user accounts, so keep it on your own network.

### 🛠️ From source

You need Rust (stable), Node.js 22, and Docker for the database.

```bash
git clone https://github.com/tronyx/rollcall.git
cd rollcall
docker compose up -d db
cp .env.example backend/.env

cd backend && cargo run          # API on :8080
cd frontend && npm install && npm run dev   # web interface on :5173, in a second terminal
```

[Installation](docs/installation.md) has the details and sample data.

## 📖 Documentation

| Page | Contents |
| --- | --- |
| [📦 Installation](docs/installation.md) | Requirements, running Rollcall with Docker Compose, and building it from source |
| [🚀 Usage](docs/usage.md) | Adding spools, the link parser, weight updates, locations, and the dashboard |
| [🐳 Docker](docs/docker.md) | The images, their tags, the database volume, updates, and backups |
| [⚙️ Configuration](docs/configuration.md) | Every environment variable |
| [🔗 Link parser](docs/link-parser.md) | How Rollcall reads a product page, and what it does when a store blocks it |
| [🏗️ Architecture](docs/architecture.md) | The stack, the repository layout, and the data model |
| [🧪 How it is tested](docs/testing.md) | The tests, and the checks that GitHub runs on each change |
| [🩺 Troubleshooting](docs/troubleshooting.md) | Common problems, and what to do |

## 🤝 Contributing

You can send bug reports, ideas, and pull requests. The [contributing guide](.github/CONTRIBUTING.md) tells you how. Branch from `develop` and open your pull request into `develop`. Do not open an issue to report a security vulnerability, because issues are public. Use the procedure in the [security policy](.github/SECURITY.md) instead. The [changelog](CHANGELOG.md) lists the changes in each release.

## 🤖 A note on how this was built

Rollcall was largely written with [Claude](https://claude.ai), Anthropic's AI coding assistant. I described what I needed, directed the design, and asked for changes across many iterations rather than writing most of the code by hand. The backend has API tests against a real PostgreSQL 18 database and parser tests on saved product pages, and every change runs them, plus a build and test of the Docker image, before it lands.

I'd rather say that plainly than let it pass as fully hand-written. There's a lot of AI-generated code floating around that hasn't been reviewed or tested and ends up breaking people's setups, and I don't want this to be mistaken for that. If something looks off, please [open an issue](https://github.com/tronyx/rollcall/issues).

## 📄 License

The license is MIT. See [LICENSE.md](LICENSE.md).
