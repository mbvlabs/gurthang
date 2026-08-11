CREATE TABLE tower_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    data BYTEA NOT NULL,
    expiry_date TIMESTAMPTZ NOT NULL
);

CREATE INDEX tower_sessions_expiry_date_idx ON tower_sessions (expiry_date);
