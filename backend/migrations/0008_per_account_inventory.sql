-- Each account has its own inventory. A location can be shared with other
-- accounts to view, or to edit and add spools to.
--
-- Existing rows go to the first admin. On an install with no account yet,
-- owner_id stays NULL and the first account adopts them when it's created
-- (repo::accounts::create_user). Rows with no owner are visible to no one.

ALTER TABLE locations       ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE printers        ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE filaments       ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE spools          ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE equipment       ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE prints          ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;
ALTER TABLE slicer_profiles ADD COLUMN owner_id UUID REFERENCES users (id) ON DELETE CASCADE;

CREATE FUNCTION adopt_unowned(new_owner UUID) RETURNS void LANGUAGE sql AS $$
    UPDATE locations       SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE printers        SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE filaments       SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE spools          SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE equipment       SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE prints          SET owner_id = new_owner WHERE owner_id IS NULL;
    UPDATE slicer_profiles SET owner_id = new_owner WHERE owner_id IS NULL;
$$;

SELECT adopt_unowned(id) FROM (
    SELECT id FROM users ORDER BY is_admin DESC, created_at LIMIT 1
) first_account;

CREATE INDEX locations_owner_idx  ON locations (owner_id);
CREATE INDEX printers_owner_idx   ON printers (owner_id);
CREATE INDEX filaments_owner_idx  ON filaments (owner_id);
CREATE INDEX spools_owner_idx     ON spools (owner_id);
CREATE INDEX equipment_owner_idx  ON equipment (owner_id);
CREATE INDEX prints_owner_idx     ON prints (owner_id, created_at DESC);

-- Names and codes are unique per account now.
DROP INDEX locations_name_ci_uq;
CREATE UNIQUE INDEX locations_name_ci_uq ON locations (owner_id, lower(name));
DROP INDEX printers_name_ci_uq;
CREATE UNIQUE INDEX printers_name_ci_uq ON printers (owner_id, lower(name));
DROP INDEX printers_one_default_uq;
CREATE UNIQUE INDEX printers_one_default_uq ON printers (owner_id) WHERE is_default;
DROP INDEX filaments_source_url_uq;
CREATE UNIQUE INDEX filaments_source_url_uq ON filaments (owner_id, source_url)
    WHERE source_url IS NOT NULL;
DROP INDEX filaments_identity_uq;
CREATE UNIQUE INDEX filaments_identity_uq ON filaments
    (owner_id, brand_id, material_id, lower(coalesce(product_name, '')), lower(color_name), diameter_mm);
DROP INDEX spools_label_code_uq;
CREATE UNIQUE INDEX spools_label_code_uq ON spools (owner_id, lower(label_code))
    WHERE label_code IS NOT NULL;
ALTER TABLE slicer_profiles DROP CONSTRAINT slicer_profiles_pkey;
CREATE UNIQUE INDEX slicer_profiles_owner_name_uq ON slicer_profiles (owner_id, name);

-- Each account's own settings. app_settings keeps the defaults for new accounts.
CREATE TABLE user_settings (
    user_id                 UUID PRIMARY KEY REFERENCES users (id) ON DELETE CASCADE,
    low_stock_threshold_g   NUMERIC(8,1) NOT NULL DEFAULT 200 CHECK (low_stock_threshold_g >= 0),
    default_currency        CHAR(3) NOT NULL DEFAULT 'USD' CHECK (default_currency ~ '^[A-Z]{3}$'),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TRIGGER user_settings_set_updated_at BEFORE UPDATE ON user_settings
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
INSERT INTO user_settings (user_id, low_stock_threshold_g, default_currency)
SELECT u.id, a.low_stock_threshold_g, a.default_currency FROM users u CROSS JOIN app_settings a;

-- Sharing a location with another account.
CREATE TYPE share_access AS ENUM ('view', 'edit');
CREATE TABLE location_shares (
    location_id UUID NOT NULL REFERENCES locations (id) ON DELETE CASCADE,
    user_id     UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    access      share_access NOT NULL DEFAULT 'view',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (location_id, user_id)
);
CREATE INDEX location_shares_user_idx ON location_shares (user_id);

-- What an account may do with a spool: 'owner', 'edit', 'view', or NULL for nothing.
CREATE FUNCTION spool_access(spool_owner UUID, spool_location UUID, me UUID)
RETURNS TEXT LANGUAGE sql STABLE AS $$
    SELECT CASE
        WHEN spool_owner = me THEN 'owner'
        ELSE (SELECT access::text FROM location_shares
              WHERE location_id = spool_location AND user_id = me)
    END
$$;

-- These views ignored ownership; the queries that used them now filter by account.
DROP VIEW inventory_stats;
DROP VIEW spool_inventory;
DROP VIEW effective_print_settings;
