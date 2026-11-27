-- Your own photos of a spool, a filament (shared by its spools), or a piece
-- of equipment or an accessory. The files live in UPLOAD_DIR, named by the
-- photo's id; deleting the owner deletes the rows, and the server removes
-- files without a row when it starts.

CREATE TABLE photos (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    spool_id        UUID REFERENCES spools (id) ON DELETE CASCADE,
    filament_id     UUID REFERENCES filaments (id) ON DELETE CASCADE,
    equipment_id    UUID REFERENCES equipment (id) ON DELETE CASCADE,
    file_name       TEXT NOT NULL UNIQUE,      -- '<id>.jpg' in UPLOAD_DIR
    content_type    TEXT NOT NULL,
    size_bytes      INTEGER NOT NULL CHECK (size_bytes > 0),
    original_name   TEXT,
    caption         TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT photos_one_owner_ck CHECK (num_nonnulls(spool_id, filament_id, equipment_id) = 1)
);
CREATE INDEX photos_spool_idx ON photos (spool_id) WHERE spool_id IS NOT NULL;
CREATE INDEX photos_filament_idx ON photos (filament_id) WHERE filament_id IS NOT NULL;
CREATE INDEX photos_equipment_idx ON photos (equipment_id) WHERE equipment_id IS NOT NULL;
