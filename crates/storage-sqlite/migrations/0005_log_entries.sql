-- One log entry holds what a person logged once (who, when, which event, the note) and owns
-- an optional smell report and an optional PM2.5 reading (design: log entries, §3). Every report
-- becomes an entry. A reading becomes the second child of its report's entry when both were
-- written by one `log_smell` call, recognised in the audit log: the reading's `create` entry is
-- the one right after the report's (audit id + 1), by the same actor, with the same
-- `observed_at` in the two `create` snapshots. The current event, time and `recorded_at` are not
-- compared: the report may have been moved or retimed since. The shared entry takes the
-- report's current values. Any other reading gets its own entry, without a note.
-- Report and reading ids are kept. The audit log is not touched.
-- Kept in step with crates/storage-postgres/migrations; scripts/neon-preflight-0005.sql runs the
-- same pairing rule read-only.

CREATE TEMP TABLE pairs (
    report_id  INTEGER NOT NULL PRIMARY KEY,
    reading_id INTEGER NOT NULL UNIQUE
);

INSERT INTO pairs (report_id, reading_id)
SELECT ar.entity_id, ap.entity_id
FROM audit_log ar
JOIN audit_log ap
  ON ap.id = ar.id + 1
 AND ap.entity = 'pm25_reading' AND ap.action = 'create'
 AND ap.actor_id IS ar.actor_id
 AND json_extract(ap.after_json, '$.observed_at') = json_extract(ar.after_json, '$.observed_at')
WHERE ar.entity = 'smell_report' AND ar.action = 'create';

-- Without a `create` entry a row cannot be paired: the migration refuses to run while one
-- exists, instead of guessing.
CREATE TEMP TABLE migration_0005_check (
    rows_without_create_entry INTEGER NOT NULL
        CONSTRAINT every_report_and_reading_has_a_create_entry CHECK (rows_without_create_entry = 0)
);
INSERT INTO migration_0005_check
SELECT
    (SELECT count(*) FROM smell_reports r
      WHERE NOT EXISTS (SELECT 1 FROM audit_log a WHERE a.entity = 'smell_report'
                        AND a.entity_id = r.id AND a.action = 'create'))
  + (SELECT count(*) FROM pm25_readings p
      WHERE NOT EXISTS (SELECT 1 FROM audit_log a WHERE a.entity = 'pm25_reading'
                        AND a.entity_id = p.id AND a.action = 'create'));
DROP TABLE migration_0005_check;

CREATE TABLE log_entries (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL REFERENCES users (id),
    event_id     INTEGER REFERENCES events (id),
    observed_at  INTEGER NOT NULL,
    recorded_at  INTEGER NOT NULL,
    note         TEXT,
    deleted_at   INTEGER,
    from_report  INTEGER,
    from_reading INTEGER
);

INSERT INTO log_entries (user_id, event_id, observed_at, recorded_at, note, deleted_at, from_report)
SELECT user_id, event_id, observed_at, recorded_at, note, deleted_at, id
FROM smell_reports ORDER BY id;

INSERT INTO log_entries (user_id, event_id, observed_at, recorded_at, note, deleted_at, from_reading)
SELECT user_id, event_id, observed_at, recorded_at, NULL, deleted_at, id
FROM pm25_readings
WHERE id NOT IN (SELECT reading_id FROM pairs)
ORDER BY id;

-- SQLite cannot drop columns that other things refer to in place: both child tables are
-- rebuilt, and the report's link tables with the reports table.
CREATE TABLE smell_reports_0005 (
    id           INTEGER PRIMARY KEY,
    log_entry_id INTEGER NOT NULL UNIQUE REFERENCES log_entries (id)
);

INSERT INTO smell_reports_0005 (id, log_entry_id)
SELECT r.id, e.id FROM smell_reports r JOIN log_entries e ON e.from_report = r.id;

CREATE TABLE smell_report_rooms_0005 (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0005 (id),
    room_id         INTEGER NOT NULL REFERENCES rooms (id),
    level           INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
    PRIMARY KEY (smell_report_id, room_id)
);

INSERT INTO smell_report_rooms_0005 (smell_report_id, room_id, level)
SELECT smell_report_id, room_id, level FROM smell_report_rooms;

CREATE TABLE smell_report_symptoms_0005 (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0005 (id),
    symptom_id      INTEGER NOT NULL REFERENCES symptoms (id),
    PRIMARY KEY (smell_report_id, symptom_id)
);

INSERT INTO smell_report_symptoms_0005 (smell_report_id, symptom_id)
SELECT smell_report_id, symptom_id FROM smell_report_symptoms;

CREATE TABLE pm25_readings_0005 (
    id           INTEGER PRIMARY KEY,
    log_entry_id INTEGER NOT NULL UNIQUE REFERENCES log_entries (id),
    value_ugm3   REAL    NOT NULL CHECK (value_ugm3 >= 0 AND value_ugm3 <= 1000)
);

-- A paired reading takes its report's entry; any other reading has its own.
INSERT INTO pm25_readings_0005 (id, log_entry_id, value_ugm3)
SELECT p.id,
       COALESCE(
           (SELECT e.id FROM log_entries e WHERE e.from_reading = p.id),
           (SELECT e.id FROM pairs JOIN log_entries e ON e.from_report = pairs.report_id
             WHERE pairs.reading_id = p.id)
       ),
       p.value_ugm3
FROM pm25_readings p;

DROP TABLE smell_report_rooms;
DROP TABLE smell_report_symptoms;
DROP TABLE smell_reports;
DROP TABLE pm25_readings;

-- Dropping the tables dropped their per-table indexes; renaming also rewrites the link tables'
-- references to the new name.
ALTER TABLE smell_reports_0005 RENAME TO smell_reports;
ALTER TABLE smell_report_rooms_0005 RENAME TO smell_report_rooms;
ALTER TABLE smell_report_symptoms_0005 RENAME TO smell_report_symptoms;
ALTER TABLE pm25_readings_0005 RENAME TO pm25_readings;

ALTER TABLE log_entries DROP COLUMN from_report;
ALTER TABLE log_entries DROP COLUMN from_reading;

CREATE INDEX log_entries_event ON log_entries (event_id, observed_at);

DROP TABLE pairs;
