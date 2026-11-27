-- =============================================================================
-- Rollcall: initial schema
-- Target: PostgreSQL 18. Uses uuidv7() (new in 18: time-ordered UUIDs, so
-- primary-key indexes stay compact) and UNIQUE NULLS NOT DISTINCT (15+).
--
-- Model overview
--   brands ─┐
--           ├─< filaments (a product/SKU: brand + material + color + manufacturer specs)
--   materials┘        │
--                     ├─< spools (a physical roll you own) >── locations
--                     │      └─< spool_weight_events (append-only usage log)
--                     └─< tuned_settings >── printers (your verified overrides)
--
-- Weights are stored as NET filament grams (spool tare excluded) unless a
-- column name says "gross". NUMERIC(8,1) keeps 0.1 g precision for scale input.
-- =============================================================================

CREATE EXTENSION IF NOT EXISTS pg_trgm;   -- fuzzy search on brand/color/product names

-- -----------------------------------------------------------------------------
-- Shared helpers
-- -----------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- -----------------------------------------------------------------------------
-- Enums
-- Kept to values that drive application logic. Open-ended lists (materials,
-- locations) are tables so they can grow without a migration.
-- -----------------------------------------------------------------------------

CREATE TYPE spool_status AS ENUM (
    'sealed',    -- new, unopened
    'active',    -- opened and in use
    'drying',    -- currently in a dryer
    'empty',     -- used up; kept for history and price stats
    'archived'   -- GUESS: hidden from inventory (gave away, tangled, binned)
);

CREATE TYPE weight_event_kind AS ENUM (
    'initial',     -- first record when a spool is created
    'print',       -- "Log Print Weight" deduction
    'quick_adjust',-- -10g / -50g buttons
    'weigh_in',    -- user put the spool on a scale; absolute reading
    'correction'   -- manual fix of a bad value
);

CREATE TYPE location_kind AS ENUM (
    'dry_box',
    'ams_slot',    -- AMS / MMU / multi-material slot
    'printer',     -- mounted on a printer's spool holder
    'dryer',
    'shelf',
    'other'
);

CREATE TYPE spec_source AS ENUM (
    'manual',      -- typed in by the user
    'scraped',     -- filled by POST /api/filaments/parse-url
    'llm'          -- GUESS: filled by an LLM-assisted extraction fallback
);

-- -----------------------------------------------------------------------------
-- Lookup tables
-- -----------------------------------------------------------------------------

CREATE TABLE brands (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT NOT NULL CHECK (length(btrim(name)) > 0),
    website     TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX brands_name_ci_uq ON brands (lower(name));
CREATE INDEX brands_name_trgm_idx ON brands USING gin (name gin_trgm_ops);
CREATE TRIGGER brands_set_updated_at BEFORE UPDATE ON brands
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE materials (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    code            TEXT NOT NULL CHECK (code ~ '^[A-Z0-9][A-Z0-9+\-]*$'),  -- 'PLA', 'PETG-CF', 'PLA+'
    display_name    TEXT NOT NULL,
    family          TEXT NOT NULL,       -- groups variants: 'PLA', 'PETG', 'ABS', ...
    density_g_cm3   NUMERIC(4,2) CHECK (density_g_cm3 > 0),  -- for grams to meters conversion
    -- Generic fallbacks shown when a filament has no manufacturer specs.
    default_drying_temp_c     SMALLINT CHECK (default_drying_temp_c BETWEEN 30 AND 150),
    default_drying_time_min   INTEGER  CHECK (default_drying_time_min > 0),
    is_abrasive     BOOLEAN NOT NULL DEFAULT false,  -- CF/GF/glow: needs hardened nozzle
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX materials_code_uq ON materials (code);
CREATE INDEX materials_family_idx ON materials (family);
CREATE TRIGGER materials_set_updated_at BEFORE UPDATE ON materials
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Seed common materials. Densities and drying values are typical published
-- figures, not authoritative; users can edit them.
INSERT INTO materials (code, display_name, family, density_g_cm3, default_drying_temp_c, default_drying_time_min, is_abrasive) VALUES
    ('PLA',      'PLA',                    'PLA',   1.24, 50, 480, false),
    ('PLA+',     'PLA+ / Tough PLA',       'PLA',   1.23, 50, 480, false),
    ('PLA-SILK', 'Silk PLA',               'PLA',   1.24, 50, 480, false),
    ('PLA-MATTE','Matte PLA',              'PLA',   1.30, 50, 480, false),
    ('PLA-CF',   'PLA Carbon Fiber',       'PLA',   1.30, 55, 480, true),
    ('PETG',     'PETG',                   'PETG',  1.27, 65, 480, false),
    ('PETG-CF',  'PETG Carbon Fiber',      'PETG',  1.30, 65, 480, true),
    ('ABS',      'ABS',                    'ABS',   1.04, 80, 480, false),
    ('ASA',      'ASA',                    'ASA',   1.07, 80, 480, false),
    ('TPU',      'TPU',                    'TPU',   1.21, 55, 480, false),
    ('PA',       'Nylon (PA)',             'PA',    1.14, 80, 720, false),
    ('PA-CF',    'Nylon Carbon Fiber',     'PA',    1.20, 80, 720, true),
    ('PC',       'Polycarbonate',          'PC',    1.20, 80, 480, false),
    ('HIPS',     'HIPS',                   'HIPS',  1.04, 60, 240, false),
    ('PVA',      'PVA (support)',          'PVA',   1.23, 55, 480, false);

CREATE TABLE locations (
    id          UUID PRIMARY KEY DEFAULT uuidv7(),
    name        TEXT NOT NULL CHECK (length(btrim(name)) > 0),   -- 'Dry Box 1', 'AMS Slot 2'
    kind        location_kind NOT NULL DEFAULT 'other',
    description TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX locations_name_ci_uq ON locations (lower(name));
CREATE TRIGGER locations_set_updated_at BEFORE UPDATE ON locations
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

CREATE TABLE printers (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    name                TEXT NOT NULL CHECK (length(btrim(name)) > 0),  -- 'X1C', 'MK4 (0.6 nozzle)'
    model               TEXT,
    nozzle_diameter_mm  NUMERIC(3,2) NOT NULL DEFAULT 0.40 CHECK (nozzle_diameter_mm > 0),
    is_default          BOOLEAN NOT NULL DEFAULT false,
    notes               TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX printers_name_ci_uq ON printers (lower(name));
-- At most one default printer.
CREATE UNIQUE INDEX printers_one_default_uq ON printers (is_default) WHERE is_default;
CREATE TRIGGER printers_set_updated_at BEFORE UPDATE ON printers
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- -----------------------------------------------------------------------------
-- Filaments: a product definition (one brand + material + color + diameter),
-- with the manufacturer-recommended specs. Many spools can share one filament,
-- so buying a second roll of the same color does not re-enter specs.
-- -----------------------------------------------------------------------------

CREATE TABLE filaments (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    brand_id            UUID NOT NULL REFERENCES brands (id) ON DELETE RESTRICT,
    material_id         UUID NOT NULL REFERENCES materials (id) ON DELETE RESTRICT,
    product_name        TEXT,                         -- 'PLA Basic', 'Prusament PETG'
    color_name          TEXT NOT NULL CHECK (length(btrim(color_name)) > 0),
    color_hex           CHAR(7) CHECK (color_hex ~ '^#[0-9A-Fa-f]{6}$'),
    -- Optional second color for dual/rainbow/silk-gradient swatches.
    color_hex_secondary CHAR(7) CHECK (color_hex_secondary ~ '^#[0-9A-Fa-f]{6}$'),
    diameter_mm         NUMERIC(3,2) NOT NULL DEFAULT 1.75
                        CHECK (diameter_mm IN (1.75, 2.85, 3.00)),
    nominal_weight_g    NUMERIC(8,1) CHECK (nominal_weight_g > 0),   -- net weight on the label, e.g. 1000
    typical_tare_g      NUMERIC(8,1) CHECK (typical_tare_g >= 0),    -- empty spool weight, if known

    -- Manufacturer-recommended specs (all optional; scraper fills what it finds).
    nozzle_temp_min_c   SMALLINT CHECK (nozzle_temp_min_c BETWEEN 150 AND 500),
    nozzle_temp_max_c   SMALLINT CHECK (nozzle_temp_max_c BETWEEN 150 AND 500),
    bed_temp_min_c      SMALLINT CHECK (bed_temp_min_c BETWEEN 0 AND 150),
    bed_temp_max_c      SMALLINT CHECK (bed_temp_max_c BETWEEN 0 AND 150),
    print_speed_min_mm_s SMALLINT CHECK (print_speed_min_mm_s > 0),
    print_speed_max_mm_s SMALLINT CHECK (print_speed_max_mm_s > 0),
    drying_temp_c       SMALLINT CHECK (drying_temp_c BETWEEN 30 AND 150),
    drying_time_min     INTEGER  CHECK (drying_time_min > 0),
    density_g_cm3       NUMERIC(4,2) CHECK (density_g_cm3 > 0),     -- overrides materials.density
    -- HueForge transmission distance: how far light passes through the color (mm).
    td_value            NUMERIC(4,1) CHECK (td_value > 0 AND td_value <= 100),

    -- Provenance from the URL parser.
    source_url          TEXT,
    image_url           TEXT,
    spec_source         spec_source NOT NULL DEFAULT 'manual',
    scraped_at          TIMESTAMPTZ,
    scrape_raw          JSONB,        -- full parser output incl. fields we did not map

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT filaments_nozzle_range_ck CHECK (nozzle_temp_min_c IS NULL OR nozzle_temp_max_c IS NULL OR nozzle_temp_min_c <= nozzle_temp_max_c),
    CONSTRAINT filaments_bed_range_ck    CHECK (bed_temp_min_c    IS NULL OR bed_temp_max_c    IS NULL OR bed_temp_min_c    <= bed_temp_max_c),
    CONSTRAINT filaments_speed_range_ck  CHECK (print_speed_min_mm_s IS NULL OR print_speed_max_mm_s IS NULL OR print_speed_min_mm_s <= print_speed_max_mm_s)
);
CREATE INDEX filaments_brand_idx    ON filaments (brand_id);
CREATE INDEX filaments_material_idx ON filaments (material_id);
CREATE INDEX filaments_color_trgm_idx   ON filaments USING gin (color_name gin_trgm_ops);
CREATE INDEX filaments_product_trgm_idx ON filaments USING gin (product_name gin_trgm_ops);
-- The same product page should map to one filament row (dedupe on re-scrape).
CREATE UNIQUE INDEX filaments_source_url_uq ON filaments (source_url) WHERE source_url IS NOT NULL;
-- Prevent accidental duplicates of the same product/color/diameter.
CREATE UNIQUE INDEX filaments_identity_uq ON filaments
    (brand_id, material_id, lower(coalesce(product_name, '')), lower(color_name), diameter_mm);
CREATE TRIGGER filaments_set_updated_at BEFORE UPDATE ON filaments
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- -----------------------------------------------------------------------------
-- Tuned / verified settings: your overrides for a filament on a given printer.
-- printer_id NULL means "applies to any printer" (general override).
-- NULL columns fall back to the manufacturer value on the filament.
-- -----------------------------------------------------------------------------

CREATE TABLE tuned_settings (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    filament_id         UUID NOT NULL REFERENCES filaments (id) ON DELETE CASCADE,
    printer_id          UUID REFERENCES printers (id) ON DELETE CASCADE,
    nozzle_temp_c       SMALLINT CHECK (nozzle_temp_c BETWEEN 150 AND 500),
    first_layer_nozzle_temp_c SMALLINT CHECK (first_layer_nozzle_temp_c BETWEEN 150 AND 500),
    bed_temp_c          SMALLINT CHECK (bed_temp_c BETWEEN 0 AND 150),
    print_speed_mm_s    SMALLINT CHECK (print_speed_mm_s > 0),
    max_volumetric_speed_mm3_s NUMERIC(5,1) CHECK (max_volumetric_speed_mm3_s > 0),
    flow_ratio          NUMERIC(4,3) CHECK (flow_ratio BETWEEN 0.5 AND 1.5),
    pressure_advance    NUMERIC(5,4) CHECK (pressure_advance BETWEEN 0 AND 2),
    retraction_length_mm NUMERIC(4,2) CHECK (retraction_length_mm >= 0),
    retraction_speed_mm_s SMALLINT CHECK (retraction_speed_mm_s > 0),
    fan_speed_pct       SMALLINT CHECK (fan_speed_pct BETWEEN 0 AND 100),
    drying_temp_c       SMALLINT CHECK (drying_temp_c BETWEEN 30 AND 150),
    drying_time_min     INTEGER  CHECK (drying_time_min > 0),
    notes               TEXT,
    verified_at         TIMESTAMPTZ,   -- set when you mark the profile as tested
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- One override row per (filament, printer), including one "any printer" row.
    CONSTRAINT tuned_settings_filament_printer_uq UNIQUE NULLS NOT DISTINCT (filament_id, printer_id)
);
CREATE INDEX tuned_settings_printer_idx ON tuned_settings (printer_id);
CREATE TRIGGER tuned_settings_set_updated_at BEFORE UPDATE ON tuned_settings
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- -----------------------------------------------------------------------------
-- Spools: the physical rolls you own.
-- -----------------------------------------------------------------------------

CREATE TABLE spools (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    -- Short human code for labels/QR (e.g. 'R-0042'). Generated by the app.
    label_code          TEXT CHECK (label_code ~ '^[A-Za-z0-9\-]{1,16}$'),
    filament_id         UUID NOT NULL REFERENCES filaments (id) ON DELETE RESTRICT,
    location_id         UUID REFERENCES locations (id) ON DELETE SET NULL,
    status              spool_status NOT NULL DEFAULT 'sealed',

    initial_weight_g    NUMERIC(8,1) NOT NULL CHECK (initial_weight_g > 0),   -- net, e.g. 1000
    remaining_weight_g  NUMERIC(8,1) NOT NULL CHECK (remaining_weight_g >= 0),
    tare_weight_g       NUMERIC(8,1) CHECK (tare_weight_g >= 0),              -- empty spool weight

    purchase_price      NUMERIC(10,2) CHECK (purchase_price >= 0),
    currency            CHAR(3) NOT NULL DEFAULT 'USD' CHECK (currency ~ '^[A-Z]{3}$'),  -- GUESS: default USD
    purchase_date       DATE,
    vendor              TEXT,          -- where bought (Amazon, direct), if different from brand
    lot_number          TEXT,

    opened_at           TIMESTAMPTZ,
    last_dried_at       TIMESTAMPTZ,
    emptied_at          TIMESTAMPTZ,
    notes               TEXT,

    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- Allow a small overshoot: labels say 1000 g but rolls often weigh in a bit heavier.
    CONSTRAINT spools_remaining_le_initial_ck CHECK (remaining_weight_g <= initial_weight_g * 1.10),
    CONSTRAINT spools_emptied_consistent_ck   CHECK (status <> 'empty' OR emptied_at IS NOT NULL)
);
CREATE UNIQUE INDEX spools_label_code_uq ON spools (lower(label_code)) WHERE label_code IS NOT NULL;
CREATE INDEX spools_filament_idx ON spools (filament_id);
CREATE INDEX spools_location_idx ON spools (location_id);
CREATE INDEX spools_status_idx   ON spools (status);
-- Low-stock alerts and "in inventory" lists only care about non-empty spools.
CREATE INDEX spools_in_stock_remaining_idx ON spools (remaining_weight_g)
    WHERE status NOT IN ('empty', 'archived');
CREATE TRIGGER spools_set_updated_at BEFORE UPDATE ON spools
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- -----------------------------------------------------------------------------
-- Weight log: append-only history of every change to remaining weight.
-- The backend writes the event and updates spools.remaining_weight_g in one
-- transaction; weight_after_g makes the history readable without replaying.
-- -----------------------------------------------------------------------------

CREATE TABLE spool_weight_events (
    id              BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    spool_id        UUID NOT NULL REFERENCES spools (id) ON DELETE CASCADE,
    kind            weight_event_kind NOT NULL,
    delta_g         NUMERIC(8,1) NOT NULL,                         -- negative = used
    weight_after_g  NUMERIC(8,1) NOT NULL CHECK (weight_after_g >= 0),
    gross_reading_g NUMERIC(8,1) CHECK (gross_reading_g >= 0),     -- raw scale value for weigh_in
    print_name      TEXT,                                          -- for kind = 'print'
    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT weight_events_weigh_in_has_reading_ck
        CHECK (kind <> 'weigh_in' OR gross_reading_g IS NOT NULL)
);
CREATE INDEX spool_weight_events_spool_time_idx ON spool_weight_events (spool_id, created_at DESC);

-- -----------------------------------------------------------------------------
-- App settings: single-row table for user preferences.
-- -----------------------------------------------------------------------------

CREATE TABLE app_settings (
    id                      BOOLEAN PRIMARY KEY DEFAULT true CHECK (id),   -- enforces one row
    low_stock_threshold_g   NUMERIC(8,1) NOT NULL DEFAULT 200 CHECK (low_stock_threshold_g >= 0),
    default_currency        CHAR(3) NOT NULL DEFAULT 'USD' CHECK (default_currency ~ '^[A-Z]{3}$'),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO app_settings DEFAULT VALUES;
CREATE TRIGGER app_settings_set_updated_at BEFORE UPDATE ON app_settings
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- -----------------------------------------------------------------------------
-- Views
-- -----------------------------------------------------------------------------

-- One row per spool with everything the inventory list and filters need.
CREATE VIEW spool_inventory AS
SELECT
    s.id,
    s.label_code,
    s.status,
    s.initial_weight_g,
    s.remaining_weight_g,
    s.tare_weight_g,
    round(100 * s.remaining_weight_g / s.initial_weight_g, 1)    AS remaining_pct,
    (s.status NOT IN ('empty', 'archived')
        AND s.remaining_weight_g < st.low_stock_threshold_g)    AS is_low_stock,
    s.purchase_price,
    s.currency,
    s.purchase_date,
    s.notes,
    s.updated_at,
    f.id            AS filament_id,
    f.product_name,
    f.color_name,
    f.color_hex,
    f.color_hex_secondary,
    f.diameter_mm,
    f.image_url,
    b.id            AS brand_id,
    b.name          AS brand_name,
    m.id            AS material_id,
    m.code          AS material_code,
    m.family        AS material_family,
    l.id            AS location_id,
    l.name          AS location_name,
    l.kind          AS location_kind
FROM spools s
JOIN filaments f      ON f.id = s.filament_id
JOIN brands b         ON b.id = f.brand_id
JOIN materials m      ON m.id = f.material_id
LEFT JOIN locations l ON l.id = s.location_id
CROSS JOIN app_settings st;

-- Dashboard totals in one query.
CREATE VIEW inventory_stats AS
SELECT
    count(*) FILTER (WHERE status NOT IN ('empty', 'archived'))                 AS spools_in_stock,
    count(*) FILTER (WHERE status = 'sealed')                                   AS spools_sealed,
    count(*) FILTER (WHERE status = 'active')                                   AS spools_active,
    count(*) FILTER (WHERE status = 'drying')                                   AS spools_drying,
    count(*) FILTER (WHERE status = 'empty')                                    AS spools_empty,
    coalesce(round(sum(remaining_weight_g) FILTER (WHERE status NOT IN ('empty', 'archived')) / 1000, 2), 0)
                                                                                AS remaining_kg,
    count(*) FILTER (WHERE is_low_stock)                                        AS low_stock_count
FROM spool_inventory;

-- Effective print settings per filament and printer: your tuned value wins,
-- then a printer-agnostic override, then the manufacturer range midpoint.
CREATE VIEW effective_print_settings AS
SELECT
    f.id  AS filament_id,
    p.id  AS printer_id,
    coalesce(tp.nozzle_temp_c, tg.nozzle_temp_c,
             ((f.nozzle_temp_min_c + f.nozzle_temp_max_c) / 2)::smallint,
             f.nozzle_temp_min_c, f.nozzle_temp_max_c)                         AS nozzle_temp_c,
    coalesce(tp.bed_temp_c, tg.bed_temp_c,
             ((f.bed_temp_min_c + f.bed_temp_max_c) / 2)::smallint,
             f.bed_temp_min_c, f.bed_temp_max_c)                               AS bed_temp_c,
    coalesce(tp.print_speed_mm_s, tg.print_speed_mm_s,
             f.print_speed_max_mm_s, f.print_speed_min_mm_s)                   AS print_speed_mm_s,
    coalesce(tp.drying_temp_c, tg.drying_temp_c, f.drying_temp_c, m.default_drying_temp_c)       AS drying_temp_c,
    coalesce(tp.drying_time_min, tg.drying_time_min, f.drying_time_min, m.default_drying_time_min) AS drying_time_min,
    coalesce(tp.flow_ratio, tg.flow_ratio)                                     AS flow_ratio,
    coalesce(tp.pressure_advance, tg.pressure_advance)                         AS pressure_advance,
    (tp.id IS NOT NULL OR tg.id IS NOT NULL)                                   AS has_tuned_override,
    coalesce(tp.verified_at, tg.verified_at)                                   AS verified_at
FROM filaments f
JOIN materials m ON m.id = f.material_id
CROSS JOIN printers p
LEFT JOIN tuned_settings tp ON tp.filament_id = f.id AND tp.printer_id = p.id
LEFT JOIN tuned_settings tg ON tg.filament_id = f.id AND tg.printer_id IS NULL;
