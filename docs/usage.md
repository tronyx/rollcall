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

Every change is written to the spool's history.

## 🧵 The spool page

Select a spool's color name to open its page. It has:

- The same weight buttons, plus **Weigh** and **Set remaining**:
  - **Weigh**: enter what the scale shows for the whole spool. Rollcall subtracts the empty-spool weight, so set that first.
  - **Set remaining**: type the remaining weight directly. Rollcall logs the difference as a correction.
- The full weight history, with **Undo** on the latest change.
- **This spool**: edit the status, location, weights, price, purchase date, vendor, label code, lot number, last drying date, and notes. The remaining weight only changes through the weight buttons, so the history stays complete.
- **Filament**: edit the brand, material, color, swatch, diameter, temperatures, speeds, drying settings, TD value, and product page. Spools of the same product share these details, so the page tells you when a change affects more than one spool.
- **Delete spool**: removes the spool and its history. To keep the history, set the status to `archived` instead.

The page has its own address, `/spools/<id>`, so you can bookmark it.

## 🏷️ QR labels

Print a label for each spool, and scan it with your phone's camera to open that spool's page next to the printer.

- **One spool**: select **🏷️ Print label** on its page.
- **Several spools**: select **🏷️ Labels** on the dashboard. Every spool that isn't `empty` or `archived` starts out chosen; select **Choose spools** to change that.

Each label has a QR code, the color, brand, material and product, the print temperatures, a swatch, and the label code (or the diameter, if the spool has no code). Choose:

- **Label size**: 40 × 30, 50 × 30, 62 × 29 (Brother DK), or 70 × 40 mm.
- **Print on**: a label printer, which prints one label per page at the label's size, or paper, which prints a sheet with cut lines.
- **QR codes open**: the address your phone opens Rollcall at, such as `http://192.168.1.20:8080`. It starts out as the address in your browser, so change it if that's `localhost`.

The browser remembers these settings. In the print dialog, choose your label printer and set the margins to none, if it asks.

## 📍 Locations and status

A spool can have a storage location, such as a dry box or an AMS slot, and a status: `sealed`, `active`, `drying`, `empty`, or `archived`.

## ⚙️ Settings

Select **⚙️ Settings** at the top of the list. The settings page has three parts:

- **General**: the weight under which a spool counts as low stock (200 g by default), and the currency for new spools that don't name one.
- **Locations**: add, rename, reorder, and delete the places where your spools live. Lower order numbers come first in lists. When you delete a location, its spools keep everything else and just lose the location.
- **Printers**: add your printers with their nozzle size, and pick one as the default. Tuned print settings per printer will use these.

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
| `PUT /api/filaments/{id}` | Change a filament's details (for every spool that uses it) |
| `POST /api/filaments/parse-url` | Read a product page and return a draft |
| `GET /api/brands`, `GET /api/materials` | Lookup lists |
| `GET`, `POST /api/locations`; `PUT`, `DELETE /api/locations/{id}` | Storage locations |
| `GET`, `POST /api/printers`; `PUT`, `DELETE /api/printers/{id}` | Printers |
| `GET`, `PUT /api/settings` | Low-stock threshold and default currency |
| `GET /api/stats` | Dashboard numbers |
| `GET /api/health` | Health check |
