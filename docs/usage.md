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

## 🛒 How you got a spool

Every spool records how you got it. You fill this in when you add the spool, or later on its page:

- **Acquired**: **Bought** (the default), **Gift**, or **Other**.
- **On**: the date you got it.
- **From**: the store, website, or person. When you add a spool from a link, this starts out as the link's site, such as `amazon.com`.
- **Price, if known**: leave it empty for gifts or when you don't remember.
- **Bought at a discount**: tick it and describe the deal, such as "Black Friday, 30% off". The spool page shows it next to the price.

## 🧵 The spool page

Select a spool's color name to open its page. It has:

- The same weight buttons, plus **Weigh** and **Set remaining**:
  - **Weigh**: enter what the scale shows for the whole spool. Rollcall subtracts the empty-spool weight, so set that first.
  - **Set remaining**: type the remaining weight directly. Rollcall logs the difference as a correction.
- The full weight history, with **Undo** on the latest change.
- **This spool**: edit the status, location, weights, how you got the spool, label code, lot number, last drying date, and notes. The remaining weight only changes through the weight buttons, so the history stays complete.
- **Filament**: edit the brand, material, color, swatch, diameter, temperatures, speeds, drying settings, TD value, and product page. Spools of the same product share these details, so the page tells you when a change affects more than one spool.
- **Humidity**: hygrometer readings for this spool. See below.
- **Tuned settings**: what you've dialed in for this filament on each printer. See below.
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

## 🎯 Tuned settings

The filament's temperatures and speeds are the maker's ranges. **Tuned settings**, on the spool page, hold what actually works for you: nozzle and first-layer temperature, bed, speed, max volumetric speed, flow ratio, pressure advance, retraction, fan, and drying.

- Select **+ Add** and choose a printer, or **Any printer** for settings that apply everywhere. Each printer gets one set. Add your printers in [Settings](#%EF%B8%8F-settings) first.
- Fill in only what you changed. Empty fields fall back to the filament's values, which the form shows in grey and the summary shows as "maker".
- Tick **I've printed with these settings and they work** to mark them tested. The date you first marked them is kept until you untick it.
- The settings belong to the filament, so every spool of it shows them. Deleting a printer deletes its settings too.

### 📤 Export to your slicer

Select **Export** on a set of tuned settings to download it as a filament preset for **OrcaSlicer** or **Bambu Studio**. Then, in the slicer, choose **File › Import › Import Configs…** and pick the file.

- **Preset name**: the filament and the printer, ending in "(Rollcall)". Change it if you like.
- **Based on**: the slicer's own generic preset for the material, such as `Generic PLA @System` in OrcaSlicer, or `Generic PLA @BBL A1` in Bambu Studio for an A1. The preset keeps everything else from it, such as cooling and start G-code. Rollcall suggests one from the material and the printer's name or model. If your slicer doesn't have that preset, it skips the import, so pick one it has, or leave the field empty for a standalone preset.
- The preset gets the nozzle and first-layer temperatures, the bed temperature for the textured and smooth PEI plates, max volumetric speed, flow ratio, pressure advance, retraction, fan, the maker's temperature range, the brand, type, color, diameter, and density. Empty tuned values use the maker's, as the summary shows. Print speed and drying have no filament setting in the slicers, so they aren't exported.
- Rollcall remembers which slicer you picked last.

## 💧 Humidity

If you keep a hygrometer in a dry box or dryer, log its readings on the spool page under **Humidity**: select **+ Add reading**, and enter the humidity, and optionally the temperature and a note. The time starts out as now.

The latest reading shows a tag: **Dry** under 20%, **OK** up to 40%, and **Humid: dry it soon** above that. A line shows the trend of the last 30 readings.

## 📷 Photos

Add your own photos to a spool, to a filament, or to a piece of equipment or an accessory. Select **+ Add photos** and pick one or more JPEG, PNG, WebP, GIF, or AVIF images, up to 20 MB each. On a phone, you can take the photo right away.

- **Photos of this spool**, on the spool page: the label, the box, a print you made with it.
- **Filament photos**, on the spool page too: shared by every spool of that filament, so add a swatch or a benchy once.
- **Equipment and accessories**: edit an item to add photos. The first one shows on its card.

Select a photo to see it larger, add a caption, go to the next one with the arrow keys, or delete it. Deleting a spool or an item deletes its photos too. Photos are saved as files in the `UPLOAD_DIR` folder (see [Configuration](configuration.md)), so include that folder in your [backups](docker.md#-backups).

## 📍 Locations and status

A spool can have a storage location, such as a dry box or an AMS slot, and a status: `sealed`, `active`, `drying`, `empty`, or `archived`.

## 🧰 Equipment and accessories

Select **🧰** (**Equipment** on wider screens) on the dashboard to keep track of everything besides filament:

- **Equipment**: nozzles, hotends, AMS units, dryers, enclosures, cameras, and upgrades.
- **Accessories**: build plates, tools, spare parts, glue, and other supplies.

Each item has a kind, a name, a brand, a model or part number, the printer it belongs to (or **Any printer**), a quantity, and how you got it, the same way as a spool. Brands are shared with filament, so typing "bambu lab" matches "Bambu Lab".

Filter the list with the **All**, **Equipment**, and **Accessories** tabs, by brand, and by printer. **Not tied to a printer** shows items for any printer. In [Settings](#%EF%B8%8F-settings), **🧰 Equipment** next to a printer opens the list for that printer.

When something wears out or you give it away, edit it and tick **Retired**. Retired items are hidden unless you tick **Show retired**.

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
| `GET`, `PUT /api/filaments/{id}/tuned`; `DELETE /api/tuned/{id}` | Tuned settings for a filament, per printer or for any printer |
| `GET`, `POST /api/spools/{id}/humidity`; `DELETE /api/humidity/{id}` | A spool's humidity readings |
| `POST /api/filaments/parse-url` | Read a product page and return a draft |
| `GET /api/brands`, `GET /api/materials` | Lookup lists |
| `GET`, `POST /api/locations`; `PUT`, `DELETE /api/locations/{id}` | Storage locations |
| `GET`, `POST /api/printers`; `PUT`, `DELETE /api/printers/{id}` | Printers |
| `GET`, `PUT /api/settings` | Low-stock threshold and default currency |
| `GET`, `POST /api/equipment`; `GET`, `PUT`, `DELETE /api/equipment/{id}` | Equipment and accessories. Filters: `category`, `brand_id`, `printer` (an id or `none`), `q`, `include_retired` |
| `GET /api/photos?spool_id=` (or `filament_id`, `equipment_id`); `POST /api/photos` | List photos, or upload one as `multipart/form-data` with an owner id, a `file`, and an optional `caption` |
| `GET`, `PATCH`, `DELETE /api/photos/{id}`; `GET /api/photos/{id}/file` | A photo's details, its caption, deleting it, and the image itself |
| `GET /api/stats` | Dashboard numbers |
| `GET /api/health` | Health check |
