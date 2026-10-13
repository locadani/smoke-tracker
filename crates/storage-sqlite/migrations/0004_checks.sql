-- A no-smell report made while no event is running is kept as a check outside any event
-- (spec §7.1), so a smell report's event becomes optional. SQLite cannot drop NOT NULL in
-- place: the reports table is rebuilt, and its link tables with it, since they refer to it.

CREATE TABLE smell_reports_0004 (
    id          INTEGER PRIMARY KEY,
    event_id    INTEGER REFERENCES events (id),
    user_id     INTEGER NOT NULL REFERENCES users (id),
    observed_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    note        TEXT,
    deleted_at  INTEGER
);

INSERT INTO smell_reports_0004 (id, event_id, user_id, observed_at, recorded_at, note, deleted_at)
SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at FROM smell_reports;

CREATE TABLE smell_report_rooms_0004 (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0004 (id),
    room_id         INTEGER NOT NULL REFERENCES rooms (id),
    level           INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
    PRIMARY KEY (smell_report_id, room_id)
);

INSERT INTO smell_report_rooms_0004 (smell_report_id, room_id, level)
SELECT smell_report_id, room_id, level FROM smell_report_rooms;

CREATE TABLE smell_report_symptoms_0004 (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0004 (id),
    symptom_id      INTEGER NOT NULL REFERENCES symptoms (id),
    PRIMARY KEY (smell_report_id, symptom_id)
);

INSERT INTO smell_report_symptoms_0004 (smell_report_id, symptom_id)
SELECT smell_report_id, symptom_id FROM smell_report_symptoms;

DROP TABLE smell_report_rooms;
DROP TABLE smell_report_symptoms;
DROP TABLE smell_reports;

-- Renaming also rewrites the link tables' references to the new name.
ALTER TABLE smell_reports_0004 RENAME TO smell_reports;
ALTER TABLE smell_report_rooms_0004 RENAME TO smell_report_rooms;
ALTER TABLE smell_report_symptoms_0004 RENAME TO smell_report_symptoms;

CREATE INDEX smell_reports_event ON smell_reports (event_id, observed_at);
