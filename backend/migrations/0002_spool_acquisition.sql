-- How a spool was acquired: bought, received as a gift, or some other way,
-- and whether it was bought at a discount. purchase_date, vendor and
-- purchase_price already hold the date, where it came from, and the price.

CREATE TYPE acquisition_kind AS ENUM ('bought', 'gift', 'other');

ALTER TABLE spools
    ADD COLUMN acquisition   acquisition_kind NOT NULL DEFAULT 'bought',
    ADD COLUMN discounted    BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN discount_note TEXT,
    ADD CONSTRAINT spools_discount_note_ck CHECK (discounted OR discount_note IS NULL);
