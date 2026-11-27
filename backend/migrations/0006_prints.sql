-- Prints logged from OrcaSlicer or Bambu Studio, and what each one used.
--
-- A print uses one or more filament slots. A slot is logged when its grams
-- were deducted from a spool (weight_event_id is that deduction), pending
-- until you pick the spool, or skipped.

CREATE TYPE print_slot_status AS ENUM ('pending', 'logged', 'skipped');

CREATE TABLE prints (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    name            TEXT NOT NULL CHECK (length(btrim(name)) > 0),
    slicer          TEXT,                    -- 'OrcaSlicer 2.3.1'
    printer_preset  TEXT,                    -- the slicer's printer_settings_id
    -- The same upload twice in a row (a slicer can run its scripts twice) is
    -- recognized by this and not logged again.
    fingerprint     TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX prints_created_idx ON prints (created_at DESC);
CREATE INDEX prints_fingerprint_idx ON prints (fingerprint, created_at DESC);

CREATE TABLE print_slots (
    print_id        UUID NOT NULL REFERENCES prints (id) ON DELETE CASCADE,
    slot            SMALLINT NOT NULL CHECK (slot >= 1),
    grams           NUMERIC(8,2) NOT NULL CHECK (grams > 0),
    profile_name    TEXT,                    -- the slicer's filament preset
    filament_type   TEXT,
    color_hex       TEXT,
    status          print_slot_status NOT NULL DEFAULT 'pending',
    spool_id        UUID REFERENCES spools (id) ON DELETE SET NULL,
    weight_event_id BIGINT REFERENCES spool_weight_events (id) ON DELETE SET NULL,
    PRIMARY KEY (print_id, slot)
);
CREATE INDEX print_slots_pending_idx ON print_slots (print_id) WHERE status = 'pending';

-- Which Rollcall filament a slicer preset is. Filled when you export a preset
-- from Rollcall, or when you pick a spool for a print and keep "remember".
CREATE TABLE slicer_profiles (
    name        TEXT PRIMARY KEY CHECK (length(btrim(name)) > 0),
    filament_id UUID NOT NULL REFERENCES filaments (id) ON DELETE CASCADE,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX slicer_profiles_filament_idx ON slicer_profiles (filament_id);
