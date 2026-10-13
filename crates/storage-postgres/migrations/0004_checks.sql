-- A no-smell report made while no event is running is kept as a check outside any event
-- (spec §7.1), so a smell report's event becomes optional.
-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.

ALTER TABLE smell_reports ALTER COLUMN event_id DROP NOT NULL;
