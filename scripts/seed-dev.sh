#!/usr/bin/env bash
# Adds sample locations and spools through the API so the dashboard has data.
# Usage: scripts/seed-dev.sh [http://localhost:8080]
set -euo pipefail
API="${1:-http://localhost:8080}/api"
post() { curl -fsS -X POST "$API/$1" -H 'content-type: application/json' -d "$2" >/dev/null; }

post locations '{"name":"AMS Slot 1","kind":"ams_slot","sort_order":1}'
post locations '{"name":"AMS Slot 2","kind":"ams_slot","sort_order":2}'
post locations '{"name":"Dry Box 1","kind":"dry_box","sort_order":10}'
AMS1=$(curl -fsS "$API/locations" | sed -E 's/.*"id":"([^"]+)","name":"AMS Slot 1".*/\1/')

post spools '{"filament":{"brand_name":"Bambu Lab","material_code":"PLA","product_name":"PLA Basic","color_name":"Jade White","color_hex":"#F4F4F0","nozzle_temp_min_c":190,"nozzle_temp_max_c":230,"bed_temp_min_c":35,"bed_temp_max_c":65},"initial_weight_g":1000,"remaining_weight_g":640,"tare_weight_g":250,"status":"active","location_id":"'"$AMS1"'","label_code":"R-0001"}'
post spools '{"filament":{"brand_name":"Polymaker","material_code":"PETG","product_name":"PolyLite","color_name":"Galaxy Blue","color_hex":"#1E3A8A"},"initial_weight_g":1000,"remaining_weight_g":150,"status":"active"}'
post spools '{"filament":{"brand_name":"eSUN","material_code":"PLA+","color_name":"Fire Engine Red","color_hex":"#D32F2F"},"initial_weight_g":1000,"purchase_price":18.99}'
post spools '{"filament":{"brand_name":"Prusament","material_code":"ASA","color_name":"Prusa Orange","color_hex":"#FA6831"},"initial_weight_g":850,"remaining_weight_g":420,"status":"drying"}'
post spools '{"filament":{"brand_name":"SUNLU","material_code":"TPU","color_name":"Black","color_hex":"#000000"},"initial_weight_g":1000,"remaining_weight_g":905,"status":"active"}'
echo "Seeded sample data at $API"
