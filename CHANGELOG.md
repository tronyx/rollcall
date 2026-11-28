# 📜 Changelog

The notable changes to Rollcall, newest first.

The project doesn't use version numbers. Each entry is one merge into `main`, which always has the latest release, and links to its pull request for the full details. Changes waiting on `develop` are listed under **Unreleased**.

## Unreleased

### Added

- Each account has its own inventory: spools, filaments, locations, printers, equipment, prints, slicer presets, and settings. Everything from before goes to the first admin account. Share a location from **Settings › Locations** with another account by username, so they can view its spools, or edit them and add spools there (the spools stay yours). Brands and materials are still one shared list. New endpoints: `/api/locations/{id}/shares`. See [Usage](docs/usage.md#-your-inventory-and-sharing).
- Sign-in. Every page and API call now needs an account, except `/api/health`. The first account you create is the admin. Registration is off until the admin turns it on in **Settings › Accounts**, and the admin can add accounts, make other admins, reset passwords, and remove accounts. Each account can create API tokens for scripts. Forgot the password? Run `rollcall reset-password USERNAME` in the container. See [Usage](docs/usage.md#-accounts-and-signing-in).
- **Upgrading:** the first person to open Rollcall after this update creates the admin account, so open it right away. Existing data stays as it is. The slicer print script now needs `--token` with an API token from **Settings**.
- A Rollcall logo: the full logo (`logo.svg`), an icon-only mark (`logo-mark.svg`) in the app header, and a new favicon.
- Print logging from OrcaSlicer and Bambu Studio: a post-processing script (`rollcall-print.py`) sends the grams of each filament, and Rollcall takes them from the spool of the preset's filament, or asks on the new **Prints** page. Exported presets are recognized, and so is a preset after you pick its spool once. Undo, skip, and delete put the filament back. New endpoints: `/api/prints` and `/api/slicer-profiles`. See [Usage](docs/usage.md#-prints-from-your-slicer).
- Export tuned settings as an OrcaSlicer or Bambu Studio filament preset, based on the slicer's generic preset for the material and printer, to import with **Import Configs**. See [Usage](docs/usage.md#-export-to-your-slicer).
- Your own photos for spools, filaments, and equipment and accessories, with captions and a viewer. The first photo of an item shows on its card. Photos are files in the new `UPLOAD_DIR` folder (`/app/uploads`, a volume, in the image). New endpoints: `/api/photos` and `/api/photos/{id}`. See [Usage](docs/usage.md#-photos).
- Equipment and accessories at `/equipment`: kind, name, brand, model, printer, quantity, how you got it, and notes, with tabs and brand and printer filters, and a link from each printer in Settings. Retired items are hidden by default. New endpoints: `/api/equipment` and `/api/equipment/{id}`. See [Usage](docs/usage.md#-equipment-and-accessories).
- Humidity readings for each spool (time, humidity, optional temperature and note), with a dry, OK, or humid tag and a trend line on the spool page. New endpoints: `/api/spools/{id}/humidity` and `DELETE /api/humidity/{id}`. See [Usage](docs/usage.md#-humidity).
- How you got each spool: bought, a gift, or other; the date; where or who it came from; the price, if known; and whether it was discounted, with a note about the deal. Adding a spool from a link fills in the store. See [Usage](docs/usage.md#-how-you-got-a-spool).
- Tuned settings on the spool page: the temperatures, speeds, flow ratio, pressure advance, retraction, fan, and drying that work for a filament, per printer or for any printer, marked as tested when you've printed with them. New endpoints: `/api/filaments/{id}/tuned` and `DELETE /api/tuned/{id}`. See [Usage](docs/usage.md#-tuned-settings).
- QR labels for spools, from the spool page or for many spools at once at `/labels`, in four sizes for label printers or as a sheet on paper. Scanning a label opens the spool's page. See [Usage](docs/usage.md#%EF%B8%8F-qr-labels).
- A settings page: the low-stock threshold, the default currency, storage locations (add, rename, reorder, delete), and printers with a default. New endpoints: `PUT /api/locations/{id}`, `/api/printers`, and `/api/settings`. See [Usage](docs/usage.md#%EF%B8%8F-settings).
- A page for each spool, opened from its card. It shows the full weight history and can weigh the spool, set the remaining weight, edit the spool and its filament, and delete the spool. `PUT /api/filaments/{id}` changes a filament's details. See [Usage](docs/usage.md#-the-spool-page).
- The first version of Rollcall: a spool inventory with brands, materials, colors, weights, prices, locations, and statuses, backed by PostgreSQL 18.
- Quick weight updates (`−10g`, `−50g`, **Log print**) with undo, scale weigh-ins, and a history of every change.
- The link parser, which fills in the add-spool form from Shopify stores, schema.org data, Amazon pages, spec tables, page text, the link itself, and an optional LLM step.
- A dashboard with the spool count, kilograms left, low-stock alerts, and search and filters.
- A Docker image for `amd64` and `arm64`, published to `ghcr.io/tronyx/rollcall` as `develop`.

### Security

- The Docker image is now built on Debian 13 (trixie) instead of Debian 12, which clears the 8 PAM vulnerabilities (CVE-2025-6020, CVE-2024-22365) that Trivy reported in the Debian 12 base.
- The Docker image installs Debian's latest security updates when it's built, so fixes released after the base image (such as those for zstd and shadow) ship in the next build.

### Project

- The frontend now uses SvelteKit 3, Vite 8, vite-plugin-svelte 7, TypeScript 6, Vitest 5 and Node.js 26. The SvelteKit settings moved from `svelte.config.js` into `vite.config.ts`, and `$lib` imports became `#lib` (a `package.json` subpath import), as SvelteKit 3 requires. Dependabot skips TypeScript 7 for now, since svelte-check can't use it yet.

- The license is now MIT, like the author's other projects.
- Added a code of conduct, a contributing guide, a security policy, issue forms, a pull request template, and GitHub Sponsors.
- One `tests.yml` workflow lints the workflows, the Dockerfile, and the documentation, tests the backend and the frontend, builds and smoke-tests the image on both architectures, scans it with Trivy, and publishes it. A midweek scan checks the published images again.
- Dependabot keeps the actions, the base images, the crates, and the npm packages up to date, with pull requests into `develop`.
- `docs/` is copied to the wiki on each release, and `develop` is fast-forwarded after each merge into `main`.
