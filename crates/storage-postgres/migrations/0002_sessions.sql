-- Login sessions (spec §8.4), written by the web layer's session store. Not audited
-- (decision #50). `id_hash` is the SHA-256 of the cookie's session id, never the id itself.
-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.

CREATE TABLE sessions (
    id_hash    TEXT   NOT NULL PRIMARY KEY,
    user_id    BIGINT REFERENCES users (id),
    data       TEXT   NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE INDEX sessions_user ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);
