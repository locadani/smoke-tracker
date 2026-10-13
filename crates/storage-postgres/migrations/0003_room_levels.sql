-- Each room in a smell report gets its own level 0–10, and a report's intensity becomes its
-- highest room level (spec §6). Every existing room takes its report's intensity. A report
-- without rooms has no room to carry its intensity, so the migration refuses to run while one
-- exists (decision #103): correct those reports first.
-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM smell_reports r
        WHERE NOT EXISTS (SELECT 1 FROM smell_report_rooms l WHERE l.smell_report_id = r.id)
    ) THEN
        RAISE EXCEPTION 'every_smell_report_needs_a_room: give each smell report a room before migrating';
    END IF;
END
$$;

ALTER TABLE smell_report_rooms ADD COLUMN level BIGINT;

UPDATE smell_report_rooms l SET level = r.intensity
FROM smell_reports r WHERE r.id = l.smell_report_id;

ALTER TABLE smell_report_rooms
    ALTER COLUMN level SET NOT NULL,
    ADD CONSTRAINT smell_report_rooms_level_check CHECK (level BETWEEN 0 AND 10);

ALTER TABLE smell_reports DROP COLUMN intensity;
