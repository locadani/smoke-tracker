-- The photo a PM2.5 reading was read from. The audited whole-log snapshot of the entry
-- includes this row, so a changed, relinked or deleted photo is caught by verify-audit.
CREATE TABLE pm25_photos (
    id              INTEGER PRIMARY KEY,
    pm25_reading_id INTEGER NOT NULL UNIQUE REFERENCES pm25_readings (id),
    object_key      TEXT    NOT NULL UNIQUE,
    bytes           INTEGER NOT NULL CHECK (bytes > 0),
    sha256          TEXT    NOT NULL CHECK (length(sha256) = 64),
    taken_at        INTEGER NOT NULL
);
