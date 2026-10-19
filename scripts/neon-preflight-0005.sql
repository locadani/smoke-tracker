-- Read-only preflight for migration 0005 (log entries). Changes nothing: one SELECT, one row.
-- Run in the Neon SQL editor on the production database before deploying.
-- The "must_be_*" columns must show the value in their name; the others are for information.
-- Pairing rule (design §3): the reading's audit `create` is the one right after the report's,
-- by the same actor, with the same observed_at in both `create` snapshots.
WITH pairs AS (
    SELECT ar.entity_id AS report_id, ap.entity_id AS reading_id
    FROM audit_log ar
    JOIN audit_log ap
      ON ap.id = ar.id + 1
     AND ap.entity = 'pm25_reading' AND ap.action = 'create'
     AND ap.actor_id IS NOT DISTINCT FROM ar.actor_id
     AND (ap.after_json::jsonb -> 'observed_at') = (ar.after_json::jsonb -> 'observed_at')
    WHERE ar.entity = 'smell_report' AND ar.action = 'create'
)
SELECT
    (SELECT max(version) FROM _sqlx_migrations)                       AS must_be_4_last_migration,
    (SELECT count(*) FROM _sqlx_migrations WHERE NOT success)         AS must_be_0_failed_migrations,
    (to_regclass('public.log_entries') IS NULL)                       AS must_be_true_no_log_entries_table,
    (SELECT count(*) FROM audit_log WHERE entity = 'log_entry')       AS must_be_0_log_entry_audit_rows,
    (SELECT count(*) FROM smell_reports r
      WHERE NOT EXISTS (SELECT 1 FROM audit_log a WHERE a.entity = 'smell_report'
                        AND a.entity_id = r.id AND a.action = 'create')) AS must_be_0_reports_without_create,
    (SELECT count(*) FROM pm25_readings p
      WHERE NOT EXISTS (SELECT 1 FROM audit_log a WHERE a.entity = 'pm25_reading'
                        AND a.entity_id = p.id AND a.action = 'create')) AS must_be_0_readings_without_create,
    (SELECT count(*) FROM smell_reports
      WHERE note IS NOT NULL
        AND (note <> btrim(note) OR note = '' OR char_length(note) > 2000)) AS must_be_0_bad_notes,
    (SELECT count(*) FROM pairs)                                      AS info_pairs_one_entry_each,
    (SELECT count(*) FROM smell_reports
      WHERE id NOT IN (SELECT report_id FROM pairs))                  AS info_reports_alone,
    (SELECT count(*) FROM pm25_readings
      WHERE id NOT IN (SELECT reading_id FROM pairs))                 AS info_readings_alone,
    (SELECT count(*) FROM smell_reports WHERE note IS NOT NULL)       AS info_reports_with_note,
    (SELECT count(*) FROM audit_log)                                  AS info_audit_rows;
