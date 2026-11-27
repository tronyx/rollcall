# 🚀 Usage

[← All documentation](README.md)

## ➕ Adding a spool

Select **+ Add spool**. Paste a product link from Amazon or a manufacturer's store and select **Fetch details**. Rollcall fills in the brand, material, color, weight, diameter, and recommended temperatures. Check the form before you save it:

- Fields that Rollcall guessed are amber.
- Fields that it did not find are red.
- Nothing is saved until you select **Save spool**.

You can also select **Enter manually** and fill in the form by hand. A second spool of the same brand, material, and color reuses the filament's details, so you only enter the weights, price, and location.

[Link parser](link-parser.md) explains how a page is read.

## ⚖️ Updating the weight

Each spool card has these buttons:

| Button | What it does |
| --- | --- |
| `−10g`, `−50g` | Subtract that amount |
| **Log print** | Subtract the grams that a print used |
| **Undo** | Reverse the last change |

Every change is written to the spool's history. A scale weigh-in (the gross weight, from which Rollcall subtracts the empty-spool weight) is available through the API for now: `POST /api/spools/{id}/weight`.

## 📍 Locations and status

A spool can have a storage location, such as a dry box or an AMS slot, and a status: `sealed`, `active`, `drying`, `empty`, or `archived`.

## 📊 Dashboard

The top of the page shows the spool count, the kilograms left, and the spools that are low (under 200 g by default). Search and filter by material, brand, color, and location.

## 🔌 API

The web interface uses a JSON API under `/api`. The main endpoints are:

| Method and path | What it does |
| --- | --- |
| `GET /api/spools`, `POST /api/spools` | List or add spools |
| `GET`, `PUT`, `DELETE /api/spools/{id}` | Read, change, or delete a spool |
| `POST /api/spools/{id}/weight` | Adjust the weight, log a print, or record a weigh-in |
| `GET /api/spools/{id}/weight-events` | A spool's weight history |
| `GET /api/filaments`, `GET /api/filaments/{id}` | Filament products |
| `POST /api/filaments/parse-url` | Read a product page and return a draft |
| `GET /api/brands`, `GET /api/materials`, `GET /api/locations` | Lookup lists |
| `GET /api/stats` | Dashboard numbers |
| `GET /api/health` | Health check |
