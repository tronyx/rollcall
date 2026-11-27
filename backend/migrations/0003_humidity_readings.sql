-- Humidity readings for a spool, from a hygrometer in its box or dryer, so
-- you can see when it needs drying.

CREATE TABLE humidity_readings (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    spool_id        UUID NOT NULL REFERENCES spools (id) ON DELETE CASCADE,
    measured_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    humidity_pct    NUMERIC(4,1) NOT NULL CHECK (humidity_pct BETWEEN 0 AND 100),
    temperature_c   NUMERIC(4,1) CHECK (temperature_c BETWEEN -40 AND 150),
    note            TEXT,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX humidity_readings_spool_idx ON humidity_readings (spool_id, measured_at DESC);
