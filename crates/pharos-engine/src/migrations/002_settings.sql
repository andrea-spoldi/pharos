-- Keyed by the Platform `metadata.name`, not the source, so reconnecting the same platform
-- (e.g. with a corrected URL) keeps them.
CREATE TABLE user_inputs (
    platform TEXT NOT NULL,
    key      TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (platform, key)
);

-- Names only; values live in the OS keyring.
CREATE TABLE secrets_meta (
    platform TEXT NOT NULL,
    name     TEXT NOT NULL,
    set_at   INTEGER NOT NULL,
    PRIMARY KEY (platform, name)
);
