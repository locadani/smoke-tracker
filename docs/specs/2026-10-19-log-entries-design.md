# Log entries — design

**Status:** draft for review · 2026-10-19
**Issue:** [#25](https://github.com/locadani/smoke-tracker/issues/25) "cannot add notes to sole pm level logging"
**Builds on:** the design spec `2026-10-13-smoke-tracker-design.md` §6–§7 and the look-and-feel spec §6.1.

## 1. Why

One press of **Log** can save a smell report, a PM2.5 reading, or both. Today these are two
unrelated tables that each repeat the author, time and event, and only the report has a note.
So:

- a PM2.5 value with a note is refused ("Pick at least one room."), which is issue #25;
- a combined log is two rows that nothing links: moving the report leaves its reading in the
  old event, and correcting the report's time leaves the reading's time behind.

The fix is in the data model: **one log entry** holds what the person logged once (who, when,
which event, the note) and owns an optional smell report and an optional PM2.5 reading.

## 2. Goals, non-goals, anti-goals

- **Goals:** a PM2.5 value alone can carry a note; a combined log is one record; moving or
  re-timing a log moves both halves; the audit chain still verifies, old entries included.
- **Non-goals:** a correction or delete UI for readings (none exists today); showing notes in
  the event timeline; changing graphs, filters or summaries.
- **Anti-goals:** rewriting or re-hashing existing audit entries (decision #104); losing or
  duplicating a note; changing report URLs (`/reports/{id}` stays).

## 3. Schema (migration 0005, SQLite and Postgres)

```
log_entries     id, user_id → users, event_id → events (nullable), observed_at, recorded_at,
                note (nullable, canonical form), deleted_at
smell_reports   id, log_entry_id → log_entries.id UNIQUE NOT NULL
                (+ smell_report_rooms, smell_report_symptoms as today)
pm25_readings   id, log_entry_id → log_entries.id UNIQUE NOT NULL, value_ugm3
```

- `user_id`, `event_id`, `observed_at`, `recorded_at`, `note` and `deleted_at` leave the two child
  tables. Index `log_entries (event_id, observed_at)` replaces the two per-table indexes.
- Report and reading **ids are kept**, so URLs and audit `entity_id`s stay valid.
- **An entry has at least one child.** The schema cannot say this; the domain enforces it and a
  storage contract test checks it.
- Delete is per entry (no delete UI exists; the storage methods become `delete_entry`).

**Data migration** (pattern of 0004, decision #109, with the 0003 refusal guard):
every report becomes an entry. A reading becomes the second child of a report's entry when both
were written by one `log_smell` call, recognised in the audit log: the reading's `create` entry
is the one right after the report's `create` entry (audit id + 1), by the same actor, with the
same `observed_at` in the two `create` snapshots. The current event, time and `recorded_at` are
**not** compared: a report may have been moved or re-timed since, and the two rows of one call
were stamped tens of milliseconds apart. The shared entry takes the report's current values.
Any other reading gets its own entry with no note. If anything would pair twice, the migration
refuses (guard) instead of guessing.

Checked against production on 2026-10-19 (read-only queries): 21 reports, 20 readings,
17 pairs (all audit id + 1, same actor), 4 reports alone, 3 readings alone, one moved report.
The migration test fixture mirrors these cases with made-up data.

## 4. Domain

- A `LogEntry` record (id, user, event, observed/recorded time, note, deleted) with
  `report: Option<…>` and `reading: Option<…>`. The note moves out of `ReportValues` into the
  entry, with the same rules (trimmed, blank = none, ≤ 2000 chars, decision #51).
- `log_smell` writes one entry plus its children; `log_reading` writes an entry with a reading
  and an optional note.
- `correct_report` corrects the entry's time and note and the report's rooms, levels and
  symptoms; the reading keeps the corrected time because it no longer has its own.
- `move_report` moves the entry, so its reading moves with it.
- `latest_report_at` keeps counting smell reports only (readings never opened or closed events).

## 5. Log button rules (replaces the note row of look-and-feel §6.1 and decision #135)

| Filled in | Result |
|---|---|
| Rooms with levels (± PM2.5, ± note) | one entry: report (+ reading), note on the entry |
| No rooms, PM2.5 (± note) | one entry: reading, note on the entry |
| No rooms, symptoms (± note) | "Pick at least one room." (as today) |
| No rooms, no PM2.5, only a note | "Pick a room, or enter a PM2.5 value." |
| Nothing | "Pick a room, or enter a PM2.5 value." (as today) |

## 6. Audit

- **One audit row per log entry (decided by Daniele).** New activity is audited only as entity
  `log_entry`: one row per Log press, correction, move or delete. Its snapshot is the whole log:
  the entry's fields plus the report (id, rooms with levels, symptoms) and the reading (id,
  value) when present. No new `smell_report` or `pm25_reading` audit rows are written.
- The hash is computed as today (previous hash + the row as canonical JSON); existing rows are
  never rewritten or re-hashed (decision #104).
- Entries created by the migration have no audit row yet. On the first start after 0005, the
  app writes one `create` row per entry with its whole-log snapshot (actor: none), once: only
  while the audit has no `log_entry` rows at all. After that, an unaudited entry is reported as
  today.
- `verify-audit` compares log entries (with their children) against their latest `log_entry`
  snapshot. Old `smell_report` / `pm25_reading` rows stay in the hash chain and are still
  checked as links; their snapshots are no longer compared with current rows, because the
  backfill snapshot covers every report and reading.
- History for a log reads the old `smell_report` rows (shown through an upgrade step like
  `SmellReport::upgrade_snapshot`) followed by the `log_entry` rows, and shows only what
  changed by comparing consecutive snapshots, as today.

## 6a. Calls made by Claude (check these)

- **Entry ids are new** (their own sequence). The link lives only in `log_entry_id` on the
  children, so the audit upgrade step finds an old snapshot's entry through the kept child id.
- **Pairing in the migration** uses the audit log (§3). An earlier draft assumed both rows
  share one `recorded_at`; they do not (see the next point). A read-only preflight query
  (`scripts/neon-preflight-0005.sql`) is run against Neon before deploying.
- **One "now" per transaction.** Today the storage layer reads the clock again for every row
  it writes, so one Log press has several slightly different instants; tests miss it because
  `FakeClock` is frozen. A write transaction now takes its instant once, when it begins, and
  uses it for every row and audit entry it writes. A test with a clock that moves on every
  read pins this.
- **Backfill** runs inside `Store::migrate`, right after the SQL migrations, in one
  transaction that re-checks "no `log_entry` entries" inside it. A second instance racing it
  fails on the audit id (assigned as head + 1) and rolls back. `actor_id` none is already used
  for system entries.
- **Audit per write:** superseded by Daniele's decision in §6 (one `log_entry` row per write).
- **Domain shape:** `LogEntry` with nested children is the model; graphs and summaries keep
  their current inputs through a thin adapter, so their output does not change.
- **No database trigger** for "at least one child": domain plus contract test, keeping the
  two backends alike.

## 7. Where the note shows

**Decided (Daniele): every log gets its own page.** `/entries/{id}` shows the time, author,
PM2.5 value, rooms/levels/symptoms when present, and the note. PM2.5 lines in the event
timeline and the "PM" rows of the Events list become links to it, like report lines are today.
Seeing it follows `can_see` on the entry's author, as for reports.

Claude's call: `/reports/{id}` keeps working and redirects to its entry's page; the report's
correct and move routes stay where they are. Correcting a reading-only entry stays out of
scope (§2), so its page has no Edit button.

## 8. Docs to update (with approval)

Spec §6, §6.1, §7.6 ("two records" → one entry), §7.7; look-and-feel §6.1; a new decision
#147 superseding the note part of #135 and refining #3; learning notes later (M8).

## 9. Left to the builder

Type and module names, query shapes, how `reports_between` / `readings_between` join, the
order of tasks, test names. Back up the Neon database before running 0005 in production.
