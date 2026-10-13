-- Each room in a smell report gets its own level 0–10, and a report's intensity becomes its
-- highest room level (spec §6). Every existing room takes its report's intensity. A report
-- without rooms has no room to carry its intensity, so the migration refuses to run while one
-- exists (decision #103): correct those reports first.

CREATE TEMP TABLE migration_0003_check (
    reports_without_rooms INTEGER NOT NULL
        CONSTRAINT every_smell_report_needs_a_room CHECK (reports_without_rooms = 0)
);
INSERT INTO migration_0003_check
SELECT count(*) FROM smell_reports r
WHERE NOT EXISTS (SELECT 1 FROM smell_report_rooms l WHERE l.smell_report_id = r.id);
DROP TABLE migration_0003_check;

ALTER TABLE smell_report_rooms RENAME TO smell_report_rooms_0002;

CREATE TABLE smell_report_rooms (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
    room_id         INTEGER NOT NULL REFERENCES rooms (id),
    level           INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
    PRIMARY KEY (smell_report_id, room_id)
);

INSERT INTO smell_report_rooms (smell_report_id, room_id, level)
SELECT l.smell_report_id, l.room_id, r.intensity
FROM smell_report_rooms_0002 l JOIN smell_reports r ON r.id = l.smell_report_id;

DROP TABLE smell_report_rooms_0002;

ALTER TABLE smell_reports DROP COLUMN intensity;
