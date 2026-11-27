# ⚙️ Configuration

[← All documentation](README.md)

All settings are environment variables. For development, copy [`.env.example`](../.env.example) to `backend/.env`.

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | required | PostgreSQL connection string |
| `BIND_ADDR` | `0.0.0.0:8080` | Listen address |
| `STATIC_DIR` | unset (`/app/static` in the image) | Serve the built frontend from this folder |
| `UPLOAD_DIR` | `uploads` (`/app/uploads` in the image) | Where uploaded photos are saved. Created if it's missing |
| `CORS_ORIGIN` | unset | An extra allowed origin, for calling the API from another site |
| `RUST_LOG` | `rollcall=info,tower_http=info` in the image | Log levels |
| `SCRAPER_TIMEOUT_SECS` | `12` | How long the link parser waits for a page |
| `SCRAPER_USER_AGENT` | a browser-like value | User agent for fetching product pages |
| `SCRAPER_ALLOW_PRIVATE_HOSTS` | `false` | Allow the link parser to fetch pages on private networks |
| `ANTHROPIC_API_KEY` | unset | Turns on the optional LLM step in the link parser |
| `LLM_MODEL` | `claude-opus-5-5` | Model for that step |

## 🤖 The LLM step

When `ANTHROPIC_API_KEY` is set and a page still leaves important fields empty, Rollcall asks Claude to read the page text and fill only those gaps. It never overwrites a value that the parser found. Without a key, this step is skipped. See [Link parser](link-parser.md).
