-- Equipment (nozzles, AMS units, dryers, enclosures, upgrades) and
-- accessories (build plates, tools, spare parts), each with an optional
-- brand and an optional printer it belongs to or fits.

CREATE TYPE equipment_category AS ENUM ('equipment', 'accessory');

CREATE TABLE equipment (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    category        equipment_category NOT NULL,
    kind            TEXT,           -- 'Nozzle', 'AMS', 'Build plate', ... free text with suggestions
    name            TEXT NOT NULL CHECK (length(btrim(name)) > 0),
    brand_id        UUID REFERENCES brands (id) ON DELETE RESTRICT,
    printer_id      UUID REFERENCES printers (id) ON DELETE SET NULL,
    model           TEXT,
    quantity        INTEGER NOT NULL DEFAULT 1 CHECK (quantity >= 0),
    retired         BOOLEAN NOT NULL DEFAULT false,

    acquisition     acquisition_kind NOT NULL DEFAULT 'bought',
    acquired_on     DATE,
    acquired_from   TEXT,
    price           NUMERIC(10,2) CHECK (price >= 0),
    currency        CHAR(3) NOT NULL DEFAULT 'USD' CHECK (currency ~ '^[A-Z]{3}$'),
    discounted      BOOLEAN NOT NULL DEFAULT false,
    discount_note   TEXT,

    notes           TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT equipment_discount_note_ck CHECK (discounted OR discount_note IS NULL)
);
CREATE INDEX equipment_brand_idx ON equipment (brand_id);
CREATE INDEX equipment_printer_idx ON equipment (printer_id);
CREATE TRIGGER equipment_set_updated_at BEFORE UPDATE ON equipment
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
