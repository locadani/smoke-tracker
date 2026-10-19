# Log entries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One press of **Log** saves one `log_entries` row (who, when, event, note) that owns an optional smell report and an optional PM2.5 reading, so a PM2.5 value alone can carry a note (issue #25) and a combined log moves and re-times as one record.

**Architecture:** Same layering as before. `smoke-domain` gets `LogEntry` (nested `report`/`reading`), `Note`, and flat `ReportView`/`ReadingView` adapters so graphs and summaries keep their inputs. Migration `0005` (both backends) creates `log_entries`, moves `user_id`, `event_id`, `observed_at`, `recorded_at`, `note`, `deleted_at` out of `smell_reports` and `pm25_readings`, and keeps report and reading ids. Old audit snapshots are never rewritten: an upgrade step reads them in today's shape (as `SmellReport::upgrade_snapshot` did for 0003), and `Store::migrate` writes one `create` audit entry per migrated log entry, once. `smoke-web` gets the new Log rules and a page per entry at `/entries/{id}`.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, askama 0.16, fluent-templates 0.15, chrono, sqlx 0.9 (SQLite + Postgres), tokio. No new crates.

**Spec:** `docs/specs/2026-10-19-log-entries-design.md` (approved; §6a are Claude's calls the user was asked to check). Builds on `docs/specs/2026-10-13-smoke-tracker-design.md` §6–§7 and the look-and-feel spec §6.1. Previous comparable plan: `docs/plans/04b-room-levels.md` (migrations 0003/0004).

## Global Constraints

- Personal project. Before every commit `git config user.name` must print `locadani` (local config holds the noreply email). Never put an email address in a tracked file. No push, no merge, no PR by agents.
- Every commit message ends with a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Conventional Commits; name the Rust concept where there is one.
- Never commit secrets; the gitleaks hook (`git config core.hooksPath .githooks`) must be active.
- Never modify `description.md`, `claude.md` or `CLAUDE.md`; never `git add` them. **Never edit `docs/specs/*` or `docs/DECISIONS.md` without the user's explicit approval** (Task 8 only shows draft text).
- At every task boundary: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (SQLite and Postgres) pass.
- Postgres tests need `TEST_POSTGRES_URL`; locally `postgres://postgres@localhost:55432/postgres` (container `smoke-pg`). CI has its own service. No passwords in URLs.
- After changing any `query!` text run `scripts/sqlx-prepare.sh` (with `TEST_POSTGRES_URL`) and commit the regenerated `.sqlx/` of both storage crates; `scripts/sqlx-prepare.sh --check` must pass. `SQLX_OFFLINE=true` is set in `.cargo/config.toml`.
- Migrations are append-only: never edit `0001`–`0004`; both backends get `0005` with the same number; `crates/storage-postgres/tests/schema_parity.rs` must pass (its column count assertion, now 66, is updated in Task 3 to the new total).
- Audit entries are never rewritten or re-hashed (decision #104). `actor_id` none is the existing marker for system entries.
- `smoke-web` never sees SQL; it writes only through `smoke_domain::logbook` use cases. Content-Security-Policy `'self'` only: no inline script, `<style>`, `style=`, `on*=`. Every new page is added to `no_page_needs_inline_scripts_styles_or_eval`.
- Guests see only their own entries (`can_see` on the entry's author); someone else's entry answers **404** to a guest.
- Every new translation key exists in `en`, `it`, `tr` (existing test); it/tr texts are drafts for native review. No tutorial comments in code.
- Every "now" goes through `smoke_domain::clock::Clock`; web tests pin time with `app.at("…")`.
- Disk: all builds share ONE `CARGO_TARGET_DIR`; never create another target dir; never `rm -rf` or `cargo clean` build folders. Before a big build check `df -h /System/Volumes/Data` (>= 20 GB free, else stop and ask the user to run their `cargo clean`). At the end, give the user the `cargo clean` commands.
- `/exercises` is a separate workspace: leave it alone. No new crates; `Cargo.lock` does not change.

## Review Focus

1. **Wrong pairing in the migration** (a reading glued to the wrong report). Expected (spec §3): a reading pairs with a report only when its audit `create` is the one right after the report's, by the same actor, with the same `observed_at` in both `create` snapshots; current event/time/`recorded_at` are not compared; two people logging in the same event never pair across people; unpaired readings get their own note-less entry. Tests: `migration_0005_pairs_by_adjacent_create_entries`, `migration_0005_pairs_a_report_moved_or_retimed_since`, `migration_0005_never_pairs_across_people`, `migration_0005_keeps_a_reading_after_a_reading_alone` (Task 3).
2. **Audit after the upgrade.** Expected: `verify-audit` is clean after migration plus backfill, a direct edit of an entry, report, reading or a child's `log_entry_id` is caught by comparing with the latest whole-log `log_entry` snapshot, the backfill runs once and never twice, an old report correction (time/note) still shows in History. Tests: `a_direct_edit_of_any_part_of_a_log_is_caught` (Task 5), `backfill_writes_one_create_entry_per_log_entry_once`, `a_second_backfill_racing_the_first_changes_nothing` (Task 5), `history_includes_changes_audited_on_the_report_before_the_migration` (Task 4).
3. **A lost, duplicated or oversized note.** Expected: every old report note ends on its entry exactly once; a PM2.5-only note is stored; blank is none; > 2000 chars is refused with the existing message. Tests: `migration_0005_moves_each_report_note_to_its_entry`, `a_reading_only_entry_keeps_its_note`, `note_rules_are_unchanged` (Tasks 1, 3, 4).
4. **A guest seeing someone else's entry** via `/entries/{id}`, the PM2.5 timeline link, or the `/reports/{id}` redirect. Expected: 404, and the redirect target also 404s. Tests: `guests_cannot_see_other_peoples_entries`, `reports_redirect_does_not_leak_other_peoples_entries` (Task 7).
5. **Half-moved logs.** Expected: moving or re-timing a report moves/re-times its reading; an entry never has zero children; the large-gap warning and `latest_report_at` still count smell reports only. Tests: `moving_a_report_moves_its_reading`, `correcting_a_report_time_retimes_its_reading`, `an_entry_cannot_be_created_without_a_child`, `a_reading_does_not_close_or_extend_an_event_timeline_gap` (Task 4).

---

## File Structure

```
crates/
├─ domain/src/
│  ├─ types.rs       LogEntryId
│  ├─ records.rs     Note; LogEntry; SmellReport/Pm25Reading lose moved fields; ReportView, ReadingView;
│  │                 NewEntry (replaces NewReport/NewReading); ReportValues loses the note
│  ├─ audit.rs       AuditEntity::LogEntry
│  ├─ verify.rs      compares old report/reading snapshots through the upgrade
│  ├─ history.rs     entry_history: old report rows, then log_entry rows
│  ├─ repos.rs       EntryRepo / EntryWrites replace ReportRepo/ReadingRepo/ReportWrites/ReadingWrites
│  └─ logbook.rs, summary.rs, graphs.rs
├─ storage-{sqlite,postgres}/
│  ├─ migrations/0005_log_entries.sql
│  └─ src/entries.rs (replaces reports.rs, readings.rs), audit.rs, events.rs, lib.rs (backfill)
├─ storage-contract/src/   entries.rs (replaces reports.rs, readings.rs), migrations.rs, fixture.rs, logbook.rs, corrections.rs, audit.rs
└─ web/  routes/{entries,log,reports,events}.rs, templates/{entry,events,event,report}.html, locales/{en,it,tr}
scripts/neon-preflight-0005.sql      read-only pairing check
docs/runbooks/0005-neon.md           backup and deploy steps (user runs them)
```

**Two pull requests (decided by Daniele):** PR 1 = Tasks 1–5 on branch `feat/log-entries-model` (new model, migration, backfill; the app looks the same apart from a moved report taking its reading along), deployed and lived with for a few days. PR 2 = Tasks 6–8 on `feat/log-entries-ui`, branched from PR 1 after it merges (PM2.5 note in the Log form, entry pages, docs).

**Why the order:** the record types change in every crate at once, so Task 3 (types + migration + both backends + contract) is one atomic task that keeps behaviour identical; each commit builds. Tasks 1 and 2 are additive. Task 4 adds the new behaviour on the new model. Task 5 (backfill) comes after 3 and 4 so `verify-audit` is only expected clean from Task 5.

---

### Task 1: Domain building blocks (additive, nothing else changes)

**Files:**
- Modify: `crates/domain/src/types.rs` (add `LogEntryId`, like `ReportId`), `crates/domain/src/audit.rs:34-50` (add `LogEntry => "log_entry"` to `AuditEntity`; `has_snapshot` stays true for it), `crates/domain/src/records.rs` (add `Note`)
- Test: same files, `#[cfg(test)]`

**Interfaces:**
- Produces:
  - `pub struct LogEntryId(pub i64)` (same derives as `ReportId`).
  - `pub struct Note(String)`; `Note::MAX_CHARS: usize = 2000`; `Note::parse(raw: Option<&str>) -> Result<Option<Note>, DomainError>` (trim, blank = `None`, > MAX chars = `DomainError::NoteTooLong(n)`); `Note::as_str(&self) -> &str`; `Serialize` as a plain string.

- [ ] **Step 1: Tests** `note_rules_are_unchanged` (blank/whitespace -> `None`; trims; 2000 `ç` accepted; 2001 -> `NoteTooLong(2001)`; copy the cases of the existing `ReportValues` note test at `records.rs:529`), `log_entry_is_a_snapshot_entity` (`AuditEntity::LogEntry` parses from and prints `"log_entry"`, `has_snapshot()` is true).
- [ ] **Step 2:** Run `cargo test -p smoke-domain` and expect compile errors/FAIL.
- [ ] **Step 3:** Implement the items above. `AuditEntity::LogEntry` must keep the exhaustive matches compiling (grep `AuditEntity::` in `crates/`; storage `FromStr` already goes through `str_enum!`).
- [ ] **Step 4:** Run `cargo test -p smoke-domain`, then workspace fmt/clippy/test. Expect PASS.
- [ ] **Step 5:** Commit `feat(domain): Note, LogEntryId and the log_entry audit entity`.

---

### Task 2: Neon preflight query and deploy runbook

**Files:**
- Create: `scripts/neon-preflight-0005.sql`, `docs/runbooks/0005-neon.md`

**Interfaces:** none (deliverables for the user, who runs them).

- [ ] **Step 1:** `scripts/neon-preflight-0005.sql` already exists (written and run with the user on 2026-10-19). Check it uses exactly the pairing rule of migration 0005 (Task 3); if Task 3 changes the rule, change both together.
- [ ] **Step 2:** Write `docs/runbooks/0005-neon.md` (short, numbered): run the preflight in the Neon SQL editor; **back up before deploy** (Neon keeps only 6 hours of history, decision hosting): a Neon branch created from the console (GUI), named `backup-before-0005`, kept until the deploy is verified; deploy (user triggers, no push by agents); after start: `smoke-tracker verify-audit` must report clean (the backfill ran), check `SELECT count(*) FROM log_entries`; rollback = restore the backup and redeploy the previous image (0005 is not reversible). GUI-only machine: console steps, no CLI assumed.
- [ ] **Step 3:** Run the query against a local Postgres fixture only if one is at hand (`smoke-pg`); otherwise state "not run" in the commit body. Commit `docs: Neon preflight and runbook for migration 0005`.

---

### Task 3: The log-entry model: migration 0005, types, both backends (behaviour unchanged)

Atomic by necessity: removing columns and changing `SmellReport`/`Pm25Reading` breaks every crate, so this lands as one commit. Observable behaviour of the app stays what it was; `verify-audit` on a migrated database is not expected clean until Task 5.

**Files:**
- Create: `crates/storage-{sqlite,postgres}/migrations/0005_log_entries.sql`, `crates/storage-{sqlite,postgres}/src/entries.rs`, `crates/storage-contract/src/entries.rs`
- Delete: `crates/storage-*/src/reports.rs`, `readings.rs`, `crates/storage-contract/src/{reports,readings}.rs` (their tests move to `entries.rs`)
- Modify: `domain/src/{records,repos,logbook,summary,graphs,verify,history,lib}.rs`; storage `audit.rs` (`audit_state` adds entries), `events.rs` (`latest_report_at` joins `log_entries`, still smell reports only; `events_between` etc.), `tx.rs` (one instant per transaction, taken in `begin`; audit id = head + 1 stays); `storage-contract/src/{migrations,fixture,corrections,logbook,audit,events}.rs` (raw SQL at `audit.rs:123`, `:168`, `:278` follows the new columns); `storage-postgres/tests/schema_parity.rs:80` (new column total, recount); minimal web edits to compile (`routes/{events,reports,entries}.rs`, `labels.rs`: read times/author/note from the entry)
- Regenerate: `.sqlx/` of both crates

**Interfaces:**
- Produces (domain, `records.rs` / `repos.rs`):
  - `pub struct LogEntry { pub id: LogEntryId, pub user_id: UserId, pub event_id: Option<EventId>, pub observed_at, pub recorded_at: DateTime<Utc>, pub note: Option<Note>, pub deleted_at: Option<DateTime<Utc>>, pub report: Option<SmellReport>, pub reading: Option<Pm25Reading> }`, `impl Audited` (entity `LogEntry`; the snapshot is the whole log: entry fields plus `report` {id, rooms, symptoms} and `reading` {id, value}, `null` when absent — spec §6). `SmellReport` and `Pm25Reading` no longer implement `Audited` for new writes. `fn is_deleted(&self) -> bool`, `fn report_view(&self) -> Option<ReportView>`, `fn reading_view(&self) -> Option<ReadingView>`.
  - `pub struct SmellReport { pub id: ReportId, pub log_entry_id: LogEntryId, #[serde(flatten)] pub values: ReportValues }`; `pub struct Pm25Reading { pub id: ReadingId, pub log_entry_id: LogEntryId, pub value: Pm25 }`. `ReportValues` loses `note` (`ReportValues::new(rooms, symptoms)`; `NOTE_MAX_CHARS` is removed, `Note::MAX_CHARS` replaces it everywhere).
  - `pub struct ReportView` / `ReadingView`: flat copies of the old `SmellReport` / `Pm25Reading` shapes (report id, entry id, event, user, times, values, note, deleted_at / reading id, …, value) used by `summary.rs`, `graphs.rs` and web lists, so their output does not change. Graph/summary function signatures swap `&[SmellReport]` for `&[ReportView]` and nothing else.
  - `pub struct NewEntry { pub user_id: UserId, pub event_id: Option<EventId>, pub observed_at: DateTime<Utc>, pub note: Option<Note>, pub report: Option<ReportValues>, pub reading: Option<Pm25> }` — `NewEntry::new` returns `DomainError::EntryWithoutChildren` when both are `None`.
  - `trait EntryRepo { entry(LogEntryId); entry_of_report(ReportId); entry_of_reading(ReadingId); entries_for_event(EventId); entries_between(from, to) }` each returning `StoreResult<Option<LogEntry>>` / `Vec<LogEntry>` with children loaded (replaces `ReportRepo` + `ReadingRepo`; `Store` bound updated).
  - `trait EntryWrites { entry(&mut self, LogEntryId); entry_of_report(&mut self, ReportId); create_entry(&mut self, NewEntry) -> LogEntry; correct_entry(&mut self, LogEntryId, observed_at, Option<Note>, Option<ReportValues>) -> LogEntry; move_entry(&mut self, LogEntryId, EventId) -> LogEntry; delete_entry(&mut self, LogEntryId) -> LogEntry }`. Every write records exactly one `log_entry` audit row with the whole-log before/after (spec §6); `record` already skips no-op updates. `WriteTx` bound updated; `EventWrites::latest_report_at` signature unchanged.
  - `verify.rs`: `AuditState.snapshots` holds log entries (with children) instead of reports and readings; `verify_snapshots` compares `log_entry` only. `smell_report`/`pm25_reading` rows are still verified as chain links but no longer compared with current rows (`AuditEntity::has_snapshot` false for them once a `log_entry` row exists; simplest: false always, since the backfill covers every report and reading — spec §6). Until the backfill (Task 5) runs, a migrated database reports entries as `Unaudited`.
- Migration `0005` (both backends), in this order: a temporary `pairs (report_id, reading_id)` table from the audit log — `audit_log ar` (`smell_report`, `create`) joined to `audit_log ap` on `ap.id = ar.id + 1`, `ap` = (`pm25_reading`, `create`), same `actor_id` (null-safe), equal `observed_at` in the two `after_json` snapshots (Postgres `::jsonb -> 'observed_at'`, SQLite `json_extract(…, '$.observed_at')`) — exactly the CTE in `scripts/neon-preflight-0005.sql`; refusal guard: every report and every reading has a `create` audit entry (SQLite: temp-table CHECK like `0003_room_levels.sql`, constraint name `every_report_and_reading_has_a_create_entry`; Postgres: a `DO` block raising with that name) — adjacency makes each pairing one-to-one, so no "pairs twice" check is needed; then `CREATE TABLE log_entries` (id identity/`INTEGER PRIMARY KEY`, user_id, event_id nullable, observed_at, recorded_at, note, deleted_at, plus temporary `from_report`/`from_reading` columns dropped at the end), then one entry per report from its current row, then one entry per reading not in `pairs` (note null), then children get `log_entry_id` (a paired reading takes its report's entry) (SQLite rebuilds `smell_reports`, `pm25_readings` and the two link tables like `0004_checks.sql`; Postgres `ALTER TABLE … ADD COLUMN/DROP COLUMN` in place, `log_entry_id` `UNIQUE NOT NULL REFERENCES log_entries (id)`), finally `CREATE INDEX log_entries_event ON log_entries (event_id, observed_at)` and drop the two old per-table indexes. Report and reading ids unchanged.

- [ ] **Step 1: Write the failing contract tests** (backend-neutral in `storage-contract`, run by both backends): `entry_with_report_and_reading_round_trips`, `an_entry_cannot_be_created_without_a_child` (the domain error and a raw-SQL check that no childless entry exists after any contract flow), `creating_a_combined_log_writes_one_log_entry_audit_row` (exactly one new audit row, entity `log_entry`, snapshot holds report and reading), `delete_entry_hides_both_children`, and the migration fixture in `migrations.rs`: `ROWS_BEFORE_0005` (database_at version 4, rows **and their audit entries** written through the version-4 store so the chain is real; made-up data mirroring production on 2026-10-19): two users; a combined log by user 1 (pair); a combined log by user 2 directly followed by a report alone by user 2; a report alone by user 2 directly followed by a combined log by user 1 (no cross-person pair); two readings alone in a row; a combined log whose report was then moved to another event and back, and one whose report was moved and stays moved (reading follows the report's current event); a combined log whose report time was later corrected; a report with a note; a no-smell check without rooms. Tests: `migration_0005_pairs_by_adjacent_create_entries`, `migration_0005_never_pairs_across_people`, `migration_0005_keeps_a_reading_after_a_reading_alone`, `migration_0005_pairs_a_report_moved_or_retimed_since`, `migration_0005_moves_each_report_note_to_its_entry`, `migration_0005_keeps_report_and_reading_ids`, `migration_0005_refuses_a_row_without_a_create_entry_and_changes_nothing`, and `preflight_counts_match_what_0005_creates` (Postgres only: run `scripts/neon-preflight-0005.sql` on the fixture before migrating and compare its `info_*` columns with the rows created).
  Also in this task, **one instant per write transaction** (spec §6a): `SqliteTx`/`PostgresTx` take `clock.now()` once in `begin` and `now()` returns that stored instant for every row and audit entry. Test `one_write_transaction_uses_one_instant` (contract, with a test clock that advances 1 ms on every read: report, reading and their audit `at` share one instant). In `verify.rs`: `old_report_and_reading_rows_stay_valid_chain_links` (a chain holding old `smell_report`/`pm25_reading` rows followed by `log_entry` rows verifies; the old rows' snapshots are not compared with current rows) and `a_log_entry_snapshot_differing_from_its_row_is_reported` (unit test on `verify_snapshots`). Convert existing contract tests in `reports.rs`/`readings.rs`/`corrections.rs`/`logbook.rs` to the new API in the same step (they pin current behaviour; do not weaken them).
- [ ] **Step 2:** `cargo test --workspace` fails to compile / FAIL (expected).
- [ ] **Step 3:** Domain: add the types and traits above; keep `logbook.rs` behaviour identical by building a `NewEntry` in `log_smell` (note comes from the form's current report note via `Note::parse` in the web layer, passed in `SmellEntry.note`) and `log_reading` (note `None` for now).
- [ ] **Step 4:** Both backends: write `0005`, `entries.rs` (one query per shape; load children with the existing group-by-id helpers; `entries_between` returns every entry in the window, readings and reports alike), update `audit_state`, `latest_report_at` (join `log_entries ON id = smell_reports.log_entry_id`, `deleted_at IS NULL`, still only reports), `lib.rs` untouched here. Both backends keep the same SQL shape; Postgres uses `$n` placeholders as today.
- [ ] **Step 5:** Web compile fixes only (no new behaviour): list/timeline/report pages read time, author, event, note from the entry; `read()` in `routes/entries.rs` passes the note as `SmellEntry.note`. Adapt web tests that built `ReportValues` with a note.
- [ ] **Step 6:** Update `schema_parity.rs` count; run `scripts/sqlx-prepare.sh` then `--check`; run fmt, clippy and the whole workspace tests with `TEST_POSTGRES_URL` set. Expect PASS on both backends.
- [ ] **Step 7:** Commit `feat(storage): log entries own a report and a reading (migration 0005)`. Include `.sqlx/` changes.

---

### Task 4: New behaviour: reading-only notes, linked move/re-time, History

**Files:**
- Modify: `domain/src/logbook.rs` (`log_reading`, `correct_report`, `move_report`, `editable_report`), `domain/src/history.rs`, `crates/storage-contract/src/{logbook,corrections,audit}.rs`
- Test: `crates/domain` unit tests; `storage-contract` tests via both backends

**Interfaces:**
- Consumes: `EntryWrites`, `NewEntry`, `Note` (Task 3).
- Produces:
  - `pub struct ReadingEntry { observed_at, value: Pm25, note: Option<Note> }`; `log_reading(tx, actor, entry: ReadingEntry, now) -> Result<LogEntry, LogError>` (event placement exactly as today: `event_at(observed_at)`, never opens, extends or closes an event; no `EventChoice`).
  - `SmellEntry.note: Option<Note>` (added in Task 3); `Logged { entry: LogEntry }` (children via `entry.report` / `entry.reading`).
  - `correct_report(tx, actor, id: ReportId, observed_at, values: ReportValues, note: Option<Note>, schedule, now) -> Result<LogEntry, LogError>` (same checks as today; calls `correct_entry`, so the reading re-times). `move_report(...) -> Result<LogEntry, LogError>` moves the entry (reading included). Both resolve `ReportId` through `entry_of_report`; `editable_report` returns the `LogEntry`.
  - `pub fn entry_history(old_report_rows: &[ChainedEntry], entry_rows: &[ChainedEntry]) -> Vec<HistoryEntry>`: old `smell_report` rows (upgraded with `SmellReport::upgrade_snapshot`) then `log_entry` rows, in audit order; changes come from comparing consecutive snapshots as `history.rs` does today (the backfill `create` row is not shown as a change). `was_corrected` unchanged. `report_history` is removed; web history route calls `entry_history(audit_history(SmellReport, id), audit_history(LogEntry, entry_id))`.

- [ ] **Step 1: Failing tests:** `a_reading_only_entry_keeps_its_note` (contract), `log_reading_without_note_stores_none`, `moving_a_report_moves_its_reading`, `correcting_a_report_time_retimes_its_reading`, `correcting_only_rooms_shows_only_room_changes_in_history`, `a_reading_does_not_close_or_extend_an_event_timeline_gap` (a reading after the last report leaves `latest_report_at` and the close time unchanged), `history_includes_changes_audited_on_the_report_before_the_migration` (feed an old-shape report row with a note change, the backfill `create`, then a `log_entry` update changing the note; expect both `Note` changes in audit order and nothing for the backfill), `history_shows_a_move_audited_on_the_entry`.
- [ ] **Step 2:** Run `cargo test -p smoke-domain -p smoke-storage-contract` (and the two backend crates); expect FAIL.
- [ ] **Step 3:** Implement per the Interfaces block; web call sites compile with the new signatures (the note still comes from the existing field; reading-only notes arrive in Task 6).
- [ ] **Step 4:** Full fmt/clippy/workspace tests, both backends; `scripts/sqlx-prepare.sh --check` if any `query!` changed. Expect PASS.
- [ ] **Step 5:** Commit `feat(domain): notes on PM2.5 entries; moving and re-timing act on the entry`.

---

### Task 5: Backfill the audit for migrated entries (at `migrate`, race-guarded)

**Files:**
- Modify: `crates/storage-sqlite/src/lib.rs:78` (`migrate`), `crates/storage-postgres/src/lib.rs:50` (`migrate`), each backend's `entries.rs` (a `pub(crate) async fn backfill_log_entry_audit`), `crates/storage-contract/src/audit.rs` (shared test fns), `crates/storage-{sqlite,postgres}/tests/contract.rs`

**Interfaces:**
- Produces: `Store::migrate` = SQL migrations, then backfill. Backfill = one write transaction, actor none: (1) re-check inside the transaction that `audit_log` has no `entity = 'log_entry'` row at all, else commit nothing and return; (2) write one `create` entry per row of `log_entries` (ordered by id) through the normal `record` path (audit id = head + 1). Safe to run on every start. Race: a second instance fails on the audit id (`prev_hash`/id unique) and rolls back; `migrate` treats that rollback as "already done" after re-reading that `log_entry` audit entries now exist, and returns `Ok`.
- A new log entry never triggers the backfill (the guard is "no `log_entry` audit entries at all"); an entry that is unaudited afterwards is reported by `verify-audit` as today.

- [ ] **Step 1: Failing tests** (shared in contract, run on both): `backfill_writes_one_create_entry_per_log_entry_once` (load `ROWS_BEFORE_0005`, migrate: N `log_entry` creates with whole-log snapshots, actor none, `verify_store` clean), `a_direct_edit_of_any_part_of_a_log_is_caught` (after the backfill, raw SQL changing an entry's note, a report's room level, a reading's value, or a reading's `log_entry_id` each makes `verify_store` report `Differs`), `migrate_twice_adds_no_second_set_of_entries`, `a_second_backfill_racing_the_first_changes_nothing` (run two `migrate()` calls concurrently on Postgres; on SQLite the writer lock serialises them; either way exactly one set, chain verifies), `an_entry_logged_after_the_backfill_is_not_backfilled`, `an_empty_database_migrates_without_backfill_entries`.
- [ ] **Step 2:** Run the backend contract tests; expect FAIL.
- [ ] **Step 3:** Implement as above; `crates/app` `prepare`/`serve` already call `migrate`, so no app change; confirm with `grep -n "migrate" crates/app/src`.
- [ ] **Step 4:** Full fmt/clippy/workspace tests with Postgres; `verify-audit` contract test over the migrated fixture is clean. Expect PASS.
- [ ] **Step 5:** Commit `feat(storage): audit migrated log entries once at migrate`.

---

### Task 6: Web — the Log button rules and a note on PM2.5-only logs

**Files:**
- Modify: `crates/web/src/routes/entries.rs` (`read()`, decision table test at ~:562 `the_log_button_decides_from_what_is_filled_in`), `routes/log.rs`, `templates/log.html` if a message needs a slot, `locales/{en,it,tr}/main.ftl`, `crates/web/tests/{log,logging}.rs`
- Test: same

**Interfaces:**
- Consumes: `ReadingEntry`, `log_reading`, `Note::parse` (Task 4).
- Produces: the five rows of spec §5. Messages: keep `log-error-rooms` ("Pick at least one room.") for symptoms without rooms; "only a note" and "nothing" both use the existing `log-error-empty` ("Pick a room, or enter a PM2.5 value."). No new translation key.

- [ ] **Step 1: Failing tests** (extend the decision table with these rows): `pm25_with_a_note_and_no_rooms_saves_a_reading_entry_with_the_note` (issue #25), `a_note_alone_asks_for_a_room_or_a_pm25_value`, `symptoms_without_rooms_or_pm25_still_ask_for_a_room` (unchanged text), `nothing_filled_in_asks_for_a_room_or_a_pm25_value`, `rooms_with_pm25_and_note_make_one_entry` (web test with `scraper`: one entry, report + reading, note on it), `a_note_over_2000_characters_shows_the_existing_error`.
- [ ] **Step 2:** `cargo test -p smoke-web`; expect FAIL.
- [ ] **Step 3:** In `read()` route "no rooms" cases per the table; PM2.5-only goes to `log_reading` with the note. Add the key to the three locale files.
- [ ] **Step 4:** Full fmt/clippy/workspace tests. Expect PASS.
- [ ] **Step 5:** Commit `feat(web): the Log button saves a PM2.5 value with a note (#25)`.

---

### Task 7: Web — a page per entry, links, `/reports/{id}` redirect

**Files:**
- Create: `crates/web/templates/entry.html`, `crates/web/tests/entries.rs`
- Modify: `crates/web/src/lib.rs:52-62` (route `GET /entries/{id}`), `routes/reports.rs` (`show` becomes a redirect; `correct`/`relocate`/history stay on `/reports/{id}/…`), new `routes/entry_page.rs` (or `routes/entries.rs` if small) with `show`, `routes/events.rs` (timeline PM2.5 lines and Events-list "PM" rows link to `/entries/{id}`, `ReadingRow` gets `entry: i64`), `templates/{event,events,report}.html`, `locales/{en,it,tr}/main.ftl`, `no_page_needs_inline_scripts_styles_or_eval` list

**Interfaces:**
- Consumes: `EntryRepo::entry`, `entry_of_report`, `entry_history` (Tasks 3–4), `can_see(actor, entry.user_id, now)`.
- Produces: `GET /entries/{id}` shows time, author, PM2.5 value, rooms/levels/symptoms when present, the note, the "edited" badge and History when a report exists; **Edit and Move buttons only when the entry has a report** (they link to the existing `/reports/{rid}/correct|move` routes); a reading-only entry has no Edit button (spec §2). `GET /reports/{id}` answers `303` to `/entries/{entry_id}`; a missing, deleted or not-visible report answers 404 exactly as `show` did. Report lines in the timeline may keep `/reports/{id}` (redirects) or link directly to `/entries/{id}`; link directly. New keys (en/it/tr drafts): `entry-title`, `entry-pm25-label`, `entry-note-label`, `entry-no-edit-for-reading`.

- [ ] **Step 1: Failing tests** (`scraper` on structure): `an_entry_page_shows_value_rooms_symptoms_and_note`, `a_pm25_only_entry_page_has_no_edit_button`, `pm25_lines_in_the_timeline_link_to_the_entry_page`, `pm_rows_in_the_events_list_link_to_the_entry_page`, `reports_url_redirects_to_the_entry_page`, `guests_cannot_see_other_peoples_entries`, `reports_redirect_does_not_leak_other_peoples_entries`, `a_deleted_entry_answers_404`, plus the new page in `no_page_needs_inline_scripts_styles_or_eval`.
- [ ] **Step 2:** `cargo test -p smoke-web`; expect FAIL.
- [ ] **Step 3:** Implement the route and template (reuse `report_fields` partials for rooms/symptoms); `show` in `reports.rs` becomes the redirect; update `lib.rs`.
- [ ] **Step 4:** Full fmt/clippy/workspace tests (both backends). Expect PASS. Then, if Chrome is available, one manual pass on `http://localhost` with the local SQLite dev DB: log PM2.5 + note, open its page, open a combined log, move the report and see the reading follow (state in the commit body whether done).
- [ ] **Step 5:** Commit `feat(web): a page for every log entry at /entries/{id}`.

---

### Task 8: Docs (draft text shown to the user for approval before editing)

**Files (do NOT edit before approval):** `docs/specs/2026-10-13-smoke-tracker-design.md` §6, §6.1, §7.6 ("two records" -> one entry), §7.7; the look-and-feel spec §6.1; `docs/DECISIONS.md` new row **#147** (supersedes the note part of #135, refines #3; date 2026-10-19; one line per the file's table format); `docs/plans/ROADMAP.md` link to this plan; `docs/specs/2026-10-19-log-entries-design.md` status line -> implemented.

- [ ] **Step 1:** Draft the exact replacement text for each section above (diffs, not prose about them) and show it to the user in one short summary. Stop. Do not edit any of these files until the user approves.
- [ ] **Step 2:** After approval, apply exactly the approved text; `cargo fmt --all --check` unaffected; commit `docs: log entries in the spec, decisions and roadmap`.
- [ ] **Step 3:** Final reminders to the user: run the Neon preflight and backup (`docs/runbooks/0005-neon.md`), deploy themselves, then `verify-audit`; `df -h /System/Volumes/Data` and the `cargo clean --target-dir <path>` commands for each build folder used.

---

## Self-review

- **Spec coverage:** §3 schema and migration (Task 3), §4 domain (3, 4), §5 rules (6), §6 audit incl. upgrade (1, 3), backfill (5), history (4), §6a pairing/backfill/audit per write/adapter/no trigger (2, 3, 5), §7 page, redirect, no Edit for readings (7), §8 docs (8), §9 Neon backup (2, 8). User clarifications: columns only on `log_entries` (3), shared entry only on exact match (3), page per log with links (7).
- **Type consistency:** `LogEntryId`, `Note`, `LogEntry`, `NewEntry`, `ReadingEntry`, `ReportView`/`ReadingView`, `EntryRepo`/`EntryWrites`, `entry_history` are defined in Tasks 1/3/4 and used with those names after.
- **Calls made while planning:** the `log_entry` audit snapshot holds the entry's own fields only (children are audited as their own entities); migration pairing follows audit adjacency (spec §3), checked against production counts; repo traits become `EntryRepo`/`EntryWrites`; the "only a note" message reuses `log-error-empty`.
