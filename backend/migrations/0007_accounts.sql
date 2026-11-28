-- Accounts: everyone who signs in shares the same inventory.

CREATE TABLE users (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    username        TEXT NOT NULL CHECK (username ~ '^[A-Za-z0-9._-]{1,40}$'),
    password_hash   TEXT NOT NULL,
    is_admin        BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_username_key ON users (lower(username));
CREATE TRIGGER users_set_updated_at BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- Browser sign-ins. Only the SHA-256 of the cookie's token is stored.
CREATE TABLE sessions (
    token_hash      BYTEA PRIMARY KEY,
    user_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);

-- Tokens for scripts, such as the slicer's print logger. Shown once, stored hashed.
CREATE TABLE api_tokens (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    user_id         UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    name            TEXT NOT NULL CHECK (btrim(name) <> ''),
    token_hash      BYTEA NOT NULL UNIQUE,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_used_at    TIMESTAMPTZ
);
CREATE INDEX api_tokens_user_id_idx ON api_tokens (user_id);

-- Off by default: the first account (the admin) can always be created, and
-- after that only when the admin turns registration on.
ALTER TABLE app_settings ADD COLUMN registration_enabled BOOLEAN NOT NULL DEFAULT false;
