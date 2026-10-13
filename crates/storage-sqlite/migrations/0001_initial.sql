-- Instants are INTEGER microseconds since the Unix epoch, UTC (decision #46).

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL CONSTRAINT users_username_unique UNIQUE,
    password_hash TEXT,
    role          TEXT    NOT NULL CHECK (role IN ('resident', 'guest', 'viewer')),
    locale        TEXT    NOT NULL CHECK (locale IN ('en', 'it', 'tr')),
    expires_at    INTEGER,
    created_at    INTEGER NOT NULL,
    CHECK (role <> 'resident' OR expires_at IS NULL),
    CHECK (role <> 'guest' OR expires_at IS NOT NULL)
);

CREATE TABLE invites (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users (id),
    purpose    TEXT    NOT NULL CHECK (purpose IN ('setup', 'password_reset')),
    token_hash TEXT    NOT NULL CONSTRAINT invites_token_hash_unique UNIQUE,
    created_by INTEGER NOT NULL REFERENCES users (id),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at    INTEGER
);

CREATE TABLE rooms (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    i18n_key  TEXT    CONSTRAINT rooms_i18n_key_unique UNIQUE,
    area      TEXT    NOT NULL CHECK (area IN ('inside', 'common')),
    hidden_at INTEGER
);

CREATE TABLE symptoms (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    i18n_key  TEXT    CONSTRAINT symptoms_i18n_key_unique UNIQUE,
    hidden_at INTEGER
);

CREATE TABLE events (
    id                        INTEGER PRIMARY KEY,
    owner_id                  INTEGER NOT NULL REFERENCES users (id),
    opened_at                 INTEGER NOT NULL,
    closed_at                 INTEGER,
    close_reason              TEXT    CHECK (close_reason IN ('manual', 'new_event', 'auto_0400', 'backdated')),
    auto_close_notice_seen_at INTEGER,
    recorded_at               INTEGER NOT NULL,
    CHECK ((closed_at IS NULL) = (close_reason IS NULL)),
    CHECK (closed_at IS NULL OR closed_at >= opened_at)
);

-- At most one open event (decision #49).
CREATE UNIQUE INDEX events_one_open ON events ((closed_at IS NULL)) WHERE closed_at IS NULL;
CREATE INDEX events_opened_at ON events (opened_at);

CREATE TABLE smell_reports (
    id          INTEGER PRIMARY KEY,
    event_id    INTEGER NOT NULL REFERENCES events (id),
    user_id     INTEGER NOT NULL REFERENCES users (id),
    observed_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    intensity   INTEGER NOT NULL CHECK (intensity BETWEEN 1 AND 10),
    note        TEXT,
    deleted_at  INTEGER
);

CREATE INDEX smell_reports_event ON smell_reports (event_id, observed_at);

CREATE TABLE smell_report_rooms (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
    room_id         INTEGER NOT NULL REFERENCES rooms (id),
    PRIMARY KEY (smell_report_id, room_id)
);

CREATE TABLE smell_report_symptoms (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
    symptom_id      INTEGER NOT NULL REFERENCES symptoms (id),
    PRIMARY KEY (smell_report_id, symptom_id)
);

CREATE TABLE pm25_readings (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users (id),
    event_id    INTEGER REFERENCES events (id),
    observed_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    value_ugm3  REAL    NOT NULL CHECK (value_ugm3 >= 0 AND value_ugm3 <= 1000),
    deleted_at  INTEGER
);

CREATE INDEX pm25_readings_event ON pm25_readings (event_id, observed_at);

-- Append-only (decision #48). `id` is assigned by the application: it is part of the hash.
CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY,
    at          INTEGER NOT NULL,
    actor_id    INTEGER REFERENCES users (id),
    entity      TEXT    NOT NULL,
    entity_id   INTEGER NOT NULL,
    action      TEXT    NOT NULL,
    before_json TEXT,
    after_json  TEXT,
    prev_hash   TEXT    NOT NULL CONSTRAINT audit_log_prev_hash_unique UNIQUE,
    hash        TEXT    NOT NULL CONSTRAINT audit_log_hash_unique UNIQUE
);

CREATE INDEX audit_log_entity ON audit_log (entity, entity_id);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only');
END;

CREATE TABLE jobs (
    name        TEXT    NOT NULL PRIMARY KEY,
    last_run_at INTEGER NOT NULL
);
