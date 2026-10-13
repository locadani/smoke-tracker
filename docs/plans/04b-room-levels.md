# Milestone 4b: Room levels and no-smell checks — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A smell report lists the rooms the person checked, each with its own level 0–10 (0 =
checked and clean); a report's intensity is its highest room level. A no-smell report (every
room at 0, or *No smell* with no rooms, meaning the whole home) never opens an event: it joins the
open event without the "Same smell still going?" question, or is saved as a **check** outside any
event. The Log screen gets a level row under each selected room and a **No smell** button; the
summary card counts clean checks and the Events list shows each day's checks.

**Architecture:** Same layering as milestone 4. `smoke-domain` gets `Level` (replacing
`Intensity`), `RoomLevel`, the derived `ReportValues::intensity()` and `is_no_smell()`, and
`SmellReport.event_id: Option<EventId>`; the logging use cases enforce "a no-smell report never
opens an event". Two migrations per backend: `0003` moves the level onto each room (copying the
old report intensity), `0004` makes a report's event optional (SQLite rebuilds the tables). Old
audit snapshots are never rewritten: `SmellReport::upgrade_snapshot` reads them in today's shape
for `verify-audit` and History. `smoke-web` changes only forms, pages and translations.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, askama 0.16, fluent-templates 0.15,
chrono + chrono-tz, sqlx 0.9 (SQLite + Postgres), tokio. No new crates.

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` as changed by `27463d7` (a level per
room, 0 = clean) and `8c72055` (no-smell checks): §5, §6, §6.1, §7.1, §7.2, §9 screens 1–4,
§13.3. Decisions: `docs/DECISIONS.md` rows #102–#109 (added by Task 4). Previous plan:
`docs/plans/04-logging-flow.md` (this plan ships in the same pull request).

**Provenance:** every code block in this plan was compiled and tested before the plan was written,
as one commit per task on the branch **`m4b-proto`** (from `milestone-4/logging-flow` at
`8c72055`; head `9c303db`). At every task boundary `cargo fmt --all --check`, `cargo clippy
--workspace --all-targets -- -D warnings` and `cargo test --workspace` (SQLite and Postgres)
passed, and `scripts/sqlx-prepare.sh --check` passed after Tasks 1 and 2. At the head: 482 tests
(domain 118, SQLite contract 111, Postgres contract 106, web 127, app 17, other 3). Executors may
consult that branch; this plan is the source of truth.

## Decisions for the user to confirm

Defaults were chosen so nothing blocks; each is a decision row and easy to change.

1. **Reports without rooms block migration 0003** (decision #103). The migration refuses to run,
   changing nothing, while any report has no rooms: such a report carries a smell with no room
   to hold it. No released build can create one (milestones 1–3 have no logging, milestone 4
   ships with this change), so only local trial databases are affected; fix them with the
   previous build (add a room) or start a new local database. A placeholder room was rejected
   because it would be a row with no audit entry that `verify-audit` reports for ever.
2. **Old audit entries are read, not rewritten** (decision #104). `verify-audit` and History read
   a report snapshot from before this change (`"intensity": 6, "rooms": [2]`) as every room at
   that intensity, which is exactly what migration 0003 stores. The hash chain is untouched.
3. **A report inside an event may be corrected to no smell**, even the one that opened the event
   (decision #106). Only a check is held at 0.
4. **No smell with a selected room set above 0 is refused** with a message (decision #107); a
   selected room with no level counts as 0. Rooms are pre-selected from the last report without
   levels, so every level is a fresh choice.
5. **Summary card** (decision #108): *reports* are reports in an event, *clean checks* are checks
   outside any event (no overlap); a bedroom day needs the bedroom at level 1 or more.
6. **About 20 new translation keys** in `it` and `tr` are drafts for native review.

## Global Constraints

- Personal project. Before every commit, `git config user.name` must print `locadani` and
  `git config user.email` must print locadani's GitHub noreply address (the repository's local
  config already holds it). Never put any email address in a tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings: local and CI Postgres use trust authentication.
- Never modify `description.md` or `claude.md`. Never `git add` them.
- **Never edit `docs/specs/2026-10-13-smoke-tracker-design.md` without the user's explicit
  approval.** This change's spec edits are already approved and committed (`27463d7`,
  `8c72055`); no task touches the spec.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI, `cargo fmt`
  enforced.
- `smoke-web` never sees SQL and never depends on a storage crate except as a dev-dependency. It
  writes only through the `smoke_domain::logbook` use cases (plus `mark_auto_close_notice_seen`),
  never by calling `create_event`/`close_event`/… itself.
- Free tier and a small footprint: no new crates; every script, style and icon is served by the
  app. `Cargo.lock` does not change in this plan.
- Content-Security-Policy is `'self'` only. No template may contain inline `<script>` code,
  `<style>`, `style=` attributes, `on*=` handlers, `hx-on` or `javascript:` URLs. The only scripts
  are the vendored htmx and `static/app.js`. Every new page is added to
  `no_page_needs_inline_scripts_styles_or_eval`.
- Every "now" goes through `smoke_domain::clock::Clock`. Local times (display, typed input, days,
  months, 04:00) go through `DailySchedule` and `APP_TIMEZONE`, never the browser's zone.
- Guests see only their own entries; other people's events show only their start time, and only in
  the event question. Someone else's event or report answers **404** to a guest.
- Every new translation key exists in `en`, `it` and `tr` (the existing test fails otherwise); the
  Italian and Turkish texts are drafts for native review.
- Web tests that depend on the time of day pin the fake clock with `app.at("…")` (Rome, 04:00).
- Postgres tests need `TEST_POSTGRES_URL`; locally the container `smoke-pg` listens on
  **port 55432** (`postgres://postgres@localhost:55432/postgres`). CI uses its own service.
- After changing any `query!` text, run `scripts/sqlx-prepare.sh` (with `TEST_POSTGRES_URL` set)
  and commit the regenerated `.sqlx/` files.
- Migrations are append-only: never edit `0001`–`0003` once committed; both backends get a file
  with the same number, and `schema_parity` must keep passing (66 columns).
- `docs/DECISIONS.md` is append-only; this plan adds rows **#102–#109**.
- Disk: all builds share one `CARGO_TARGET_DIR`; never create another target folder, never run
  `cargo clean` or `rm -rf` on build folders (the user does that).

- Disk: before a big build, `df -h /System/Volumes/Data` must show at least 20 GB free;
  otherwise stop and ask the user to run their `cargo clean` commands.

## Review Focus

1. **A no-smell report opening, closing or reopening an event** by any path: logged now with
   nothing open, a stale "No, it had stopped: new event" answer, backdated with "New event",
   "It was still going" on an auto-closed event, or *Move to another event → New event*.
   Expected: it joins the open event (now), the event picked (past), or no event; no event is
   created, closed or reopened. Tests:
   `a_no_smell_report_with_nothing_open_is_a_check_outside_any_event`,
   `a_no_smell_report_joins_the_open_event_without_closing_it`,
   `a_backdated_no_smell_report_joins_the_event_picked_or_none`,
   `a_check_can_move_into_an_event_but_never_opens_one` (Task 2);
   `no_smell_with_an_event_open_joins_it_without_the_question` (Task 3).
2. **Audit data written before the change.** Expected: after the migrations `verify-audit` still
   reports the chain and every record as intact, a direct edit of a migrated report is still
   caught, and old corrections read as per-room changes. Tests:
   `a_report_audited_before_room_levels_still_matches_its_converted_record`,
   `a_correction_audited_before_room_levels_compares_every_room_at_its_intensity`,
   `a_snapshot_from_before_room_levels_reads_as_every_room_at_the_intensity`,
   `a_direct_link_table_edit_is_detected` (Task 1).
3. **Migrating a local database that has data.** Expected: every room keeps its report's
   intensity, events and symptoms survive the SQLite rebuild, the link tables still enforce
   their foreign keys, a report without rooms stops the migration with nothing changed, and both
   backends end with the same columns. Tests:
   `migration_0003_gives_each_room_its_reports_intensity` (extended in Task 2 to cover 0004),
   `migration_0003_refuses_a_report_without_rooms_and_changes_nothing` (Task 1),
   `after_the_rebuild_the_link_tables_still_refer_to_the_reports` (Task 2),
   `sqlite_and_postgres_schemas_have_the_same_columns`.
4. **A guest learning about other people through checks**: the Events list, a check's URL, or
   the question skipped for a no-smell report. Expected: guests list only their own checks;
   someone else's check answers 404 like any report; joining without the question reveals
   nothing. Tests: `the_list_shows_each_days_checks_to_those_who_may_see_them` (Task 3),
   `guests_cannot_see_or_change_other_peoples_reports` (milestone 4, still passing).
5. **A level saved that the person did not mean**: a selected room without a level, a level left
   on a room that was unselected, *No smell* pressed with a room at 5, or a check corrected to a
   smell. Expected: the form comes back (422) saying what to fix; unselected rooms are ignored.
   Tests: `a_smell_needs_offered_rooms_each_with_a_level` (Task 1),
   `invalid_input_shows_the_form_again_with_what_was_typed` (Task 1),
   `no_smell_records_selected_rooms_at_zero_and_refuses_a_higher_level`,
   `a_check_has_no_event_and_cannot_be_corrected_to_a_smell` (Task 3),
   `a_check_can_be_corrected_but_never_to_a_smell` (Task 2).

---

## File Structure

```
smoke-tracker/
├─ docs/DECISIONS.md, plans/ROADMAP.md   + rows #102–#109, link to this plan
└─ crates/
   ├─ domain/src/
   │  ├─ types.rs        Level (0..=10) replaces Intensity
   │  ├─ error.rs        InvalidLevel, RoomListedTwice (InvalidIntensity removed)
   │  ├─ records.rs      RoomLevel; ReportValues by room; intensity(), is_no_smell();
   │  │                  SmellReport.event_id: Option; SmellReport::upgrade_snapshot
   │  ├─ verify.rs       compares old report snapshots in today's shape
   │  ├─ history.rs      LevelChanged / RoomAdded(RoomLevel) / RoomRemoved; Moved with Option
   │  ├─ summary.rs      bedroom days need level ≥ 1; MonthSummary.checks
   │  ├─ events.rs       no_smell_prompt
   │  └─ logbook.rs      no-smell placement; NoSmellCannotOpen, CheckCannotSmell
   ├─ storage-sqlite/, storage-postgres/
   │  ├─ migrations/0003_room_levels.sql   level per room; refuses reports without rooms
   │  ├─ migrations/0004_checks.sql        event_id nullable (SQLite: table rebuild)
   │  ├─ src/reports.rs                    level column; optional event
   │  ├─ tests/contract.rs                 migration tests
   │  └─ .sqlx/                            regenerated
   ├─ storage-contract/src/
   │  ├─ migrations.rs   NEW  rows written before 0003 and what must become of them
   │  ├─ fixture.rs      FIRST_ROOM; report_values(level, rooms, symptoms)
   │  └─ logbook.rs, corrections.rs, …     no-smell and check tests
   └─ web/
      ├─ src/routes/entries.rs   level-<room> fields; Smell { Levels, None, Correction }; prompts
      ├─ src/routes/{log,reports,events,guide,mod}.rs, src/{labels,forms}.rs
      ├─ templates/room_level.html  NEW  a room chip and its level row
      ├─ templates/{report_fields,log,report,correct,events}.html
      ├─ static/app.css          .rooms / .room / level rows shown while selected
      ├─ locales/{en,it,tr}/main.ftl   ~20 new keys, 2 removed
      └─ tests/                  form bodies use room=<id>&level-<id>=<n>; new check tests
```

**Dependency direction** is unchanged: `smoke-tracker → smoke-web → smoke-domain ← storage-*`.

**Why three code tasks and not one per layer:** changing `ReportValues` and `SmellReport.event_id`
breaks every crate at once, and each commit must build and pass on its own. So each task is a
vertical slice: Task 1 per-room levels end to end, Task 2 the no-smell rules and the optional
event (with only the web changes needed to compile), Task 3 the screens for checks.

---

### Task 1: A level 0–10 per room, end to end

Each room of a report gets its own level, and the report's intensity becomes its highest room
level (spec §6, decision #102). The domain type `Level` (0–10) replaces `Intensity` (1–10);
migration 0003 moves the level onto `smell_report_rooms`, copying each report's old intensity,
and refuses reports without rooms (decision #103). Report snapshots audited before the change are
read in today's shape (decision #104). The Log and correction forms show a level row under each
selected room (decision #107); History reads "Bedroom 6 → 7, added Kitchen 3"; the rating guide
gains level 0 "Clean". Until Task 2 a report still needs at least one room
(`DomainError::NoRooms`).

**Files:**
- Create: `crates/storage-contract/src/migrations.rs`, `crates/storage-postgres/migrations/0003_room_levels.sql`, `crates/storage-sqlite/migrations/0003_room_levels.sql`, `crates/web/templates/room_level.html`
- Modify: `crates/domain/src/error.rs`, `crates/domain/src/history.rs`, `crates/domain/src/records.rs`, `crates/domain/src/summary.rs`, `crates/domain/src/types.rs`, `crates/domain/src/verify.rs`, `crates/storage-contract/src/audit.rs`, `crates/storage-contract/src/corrections.rs`, `crates/storage-contract/src/fixture.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/logbook.rs`, `crates/storage-contract/src/reports.rs`, `crates/storage-contract/src/sweep.rs`, `crates/storage-postgres/src/reports.rs`, `crates/storage-postgres/tests/contract.rs`, `crates/storage-sqlite/src/reports.rs`, `crates/storage-sqlite/tests/contract.rs`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/forms.rs`, `crates/web/src/labels.rs`, `crates/web/src/routes/entries.rs`, `crates/web/src/routes/events.rs`, `crates/web/src/routes/guide.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/reports.rs`, `crates/web/static/app.css`, `crates/web/templates/report_fields.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/events.rs`, `crates/web/tests/log.rs`, `crates/web/tests/logging.rs`, `crates/web/tests/reports.rs`, `crates/web/tests/security.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/` (`scripts/sqlx-prepare.sh`)

**Interfaces:**
- Consumes: `ReportValues`, `SmellReport`, `report_history`, `summarise`, `verify_snapshots`
  (milestones 1–4); `Pairs`, `Draft`, `fields`, `offered` (milestone 4).
- Produces (in `smoke_domain`):
  - `types::Level` — `MIN = 0`, `MAX = 10`, `CLEAN`, `new(u8) -> Result<Level, DomainError>`,
    `get(self) -> u8`, `is_clean(self) -> bool`; serde as a bare number.
  - `DomainError::InvalidLevel(u8)`, `DomainError::RoomListedTwice(i64)`,
    `DomainError::NoRooms` (removed again in Task 2); `InvalidIntensity` is gone.
  - `records::RoomLevel { room: RoomId, level: Level }` (serialises as `{"room":2,"level":6}`).
  - `ReportValues::new(rooms: Vec<RoomLevel>, symptoms: Vec<SymptomId>, note: Option<&str>)`;
    `rooms(&self) -> &[RoomLevel]` (sorted by room), `room_ids(&self) -> Vec<RoomId>`,
    `level_in(&self, RoomId) -> Option<Level>`, `intensity(&self) -> Level`.
  - `SmellReport::upgrade_snapshot(&serde_json::Value) -> Cow<'_, Value>`.
  - `history::Change::LevelChanged { room, from: Level, to: Level }`, `RoomAdded(RoomLevel)`,
    `RoomRemoved(RoomId)` (`Intensity { .. }` is gone); level changes come first, then added,
    then removed rooms.
- Produces (tests and web):
  - `smoke_storage_contract::fixture::FIRST_ROOM: RoomId` and
    `report_values(level: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues`.
  - `smoke_storage_contract::migrations::{ROWS_BEFORE_0003, REPORT_WITHOUT_ROOMS, NEEDS_A_ROOM,
    each_room_took_its_reports_intensity}`.
  - Form fields `room=<id>` plus `level-<id>=0..10`; `entries::level_field(&Pairs, RoomId) ->
    Option<u8>`; `read_values(&Pairs, &[RoomId], &[SymptomId])`; `Pairs::names_with_prefix`;
    `log::RoomChoice { room: Choice, levels: Vec<Choice> }`; `Draft.levels: Vec<(RoomId, u8)>`
    (`Draft.intensity` is gone); `guide::GuideRow` (was `guide::Level`);
    `labels::room_by_id`, `labels::room_level` ("Bedroom 6"); `templates/room_level.html`.
  - Translation keys added: `log-level-in`, `log-error-rooms`, `log-error-level`,
    `intensity-0-name`, `intensity-0-desc`; removed: `log-error-intensity`, `history-intensity`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diffs, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/audit.rs b/crates/storage-contract/src/audit.rs
index 9038917..8f6c2f4 100644
--- a/crates/storage-contract/src/audit.rs
+++ b/crates/storage-contract/src/audit.rs
@@ -15,7 +15,7 @@ struct Populated {
     reading: Pm25Reading,
 }
 
-/// Seeding, a user, an event, a report (intensity 6), a reading and a correction.
+/// Seeding, a user, an event, a report (one room at level 6), a reading and a correction.
 async fn populate<S: Store>(fx: &Fixture<S>) -> Populated {
     let scene = scene(fx).await;
     let rooms = [scene.rooms[0].id, scene.rooms[1].id];
@@ -78,7 +78,7 @@ pub async fn a_direct_record_edit_is_detected<S: Store>(fx: &Fixture<S>) {
         .unwrap()
         .pop()
         .unwrap();
-    fx.raw("UPDATE smell_reports SET intensity = 9")
+    fx.raw("UPDATE smell_report_rooms SET level = 9")
         .await
         .unwrap();
 
@@ -157,11 +157,11 @@ pub async fn a_direct_link_table_edit_is_detected<S: Store>(fx: &Fixture<S>) {
     let rooms = fx.store.rooms().await.unwrap();
     let target = rooms.last().unwrap();
     assert_eq!(target.i18n_key.as_deref(), Some("room-stairwell"));
-    assert!(!stored.values.rooms().contains(&target.id));
+    assert!(!stored.values.room_ids().contains(&target.id));
     // `populate` makes the only report, and the stairwell (the last room) is not linked to it.
     fx.raw(
-        "INSERT INTO smell_report_rooms (smell_report_id, room_id) \
-         SELECT (SELECT MIN(id) FROM smell_reports), (SELECT MAX(id) FROM rooms)",
+        "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) \
+         SELECT (SELECT MIN(id) FROM smell_reports), (SELECT MAX(id) FROM rooms), 3",
     )
     .await
     .unwrap();
@@ -203,7 +203,7 @@ pub async fn an_edited_audit_entry_breaks_the_chain<S: Store>(fx: &Fixture<S>) {
         .remove(0);
     fx.drop_audit_guard().await;
     fx.raw(
-        r#"UPDATE audit_log SET after_json = replace(after_json, '"intensity":6', '"intensity":9') WHERE entity = 'smell_report'"#,
+        r#"UPDATE audit_log SET after_json = replace(after_json, '"level":6', '"level":9') WHERE entity = 'smell_report'"#,
     )
     .await
     .unwrap();
diff --git a/crates/storage-contract/src/corrections.rs b/crates/storage-contract/src/corrections.rs
index 05d965c..a6be353 100644
--- a/crates/storage-contract/src/corrections.rs
+++ b/crates/storage-contract/src/corrections.rs
@@ -13,7 +13,7 @@ use smoke_domain::repos::{
 use smoke_domain::types::{CloseReason, EventId, ReportId};
 
 use crate::Fixture;
-use crate::fixture::{add_event, report_values, start};
+use crate::fixture::{FIRST_ROOM, add_event, report_values, start};
 use crate::logbook::{log, people, rome, smell};
 
 async fn close<S>(
@@ -52,7 +52,7 @@ where
     S: Transactional<Tx: WriteTx>,
 {
     let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
-    let values = report_values(intensity, &[], &[]);
+    let values = report_values(intensity, &[FIRST_ROOM], &[]);
     let now = fx.clock.now();
     let report = correct_report(&mut tx, &who.actor(), id, at, values, &rome(), now).await?;
     tx.commit().await.unwrap();
diff --git a/crates/storage-contract/src/fixture.rs b/crates/storage-contract/src/fixture.rs
index a18aa3b..e21f4ca 100644
--- a/crates/storage-contract/src/fixture.rs
+++ b/crates/storage-contract/src/fixture.rs
@@ -9,14 +9,14 @@ use chrono::{DateTime, TimeDelta, Utc};
 use smoke_domain::audit::ChainedEntry;
 use smoke_domain::clock::FakeClock;
 use smoke_domain::records::{
-    Closure, Event, NewEvent, NewReport, NewUser, PasswordHash, ReportValues, Room, SmellReport,
-    Symptom, User,
+    Closure, Event, NewEvent, NewReport, NewUser, PasswordHash, ReportValues, Room, RoomLevel,
+    SmellReport, Symptom, User,
 };
 use smoke_domain::repos::{
     AuditRepo, Commit, EventWrites, ListRepo, ListWrites, ReportWrites, Transactional, UserWrites,
 };
 use smoke_domain::starter::seed_starter_lists;
-use smoke_domain::types::{Intensity, Locale, Role, RoomId, SymptomId, UserId, Username};
+use smoke_domain::types::{Level, Locale, Role, RoomId, SymptomId, UserId, Username};
 
 /// Runs SQL directly against the test database, bypassing the store: how tests tamper.
 pub type RawSql = Box<
@@ -157,14 +157,17 @@ where
     }
 }
 
-pub fn report_values(intensity: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues {
-    ReportValues::new(
-        Intensity::new(intensity).expect("valid intensity"),
-        rooms.to_vec(),
-        symptoms.to_vec(),
-        None,
-    )
-    .expect("valid values")
+/// The first starter room. Suites that log seed the starter lists first.
+pub const FIRST_ROOM: RoomId = RoomId(1);
+
+/// Report values with every room in `rooms` at `level`.
+pub fn report_values(level: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues {
+    let level = Level::new(level).expect("valid level");
+    let rooms = rooms
+        .iter()
+        .map(|&room| RoomLevel { room, level })
+        .collect();
+    ReportValues::new(rooms, symptoms.to_vec(), None).expect("valid values")
 }
 
 pub async fn add_report<S>(
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index 8d8f769..4ff7d86 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -14,6 +14,7 @@ pub mod events;
 pub mod fixture;
 pub mod lists;
 pub mod logbook;
+pub mod migrations;
 pub mod readings;
 pub mod reports;
 pub mod sessions;
diff --git a/crates/storage-contract/src/logbook.rs b/crates/storage-contract/src/logbook.rs
index 42869bc..8472062 100644
--- a/crates/storage-contract/src/logbook.rs
+++ b/crates/storage-contract/src/logbook.rs
@@ -14,7 +14,7 @@ use smoke_domain::starter::seed_starter_lists;
 use smoke_domain::types::{CloseReason, EventId, Pm25, Role, RoomId};
 
 use crate::Fixture;
-use crate::fixture::{add_event, add_user, report_values, start};
+use crate::fixture::{FIRST_ROOM, add_event, add_user, report_values, start};
 
 /// Everyone who might log, and the seeded lists.
 pub(crate) struct People {
@@ -44,7 +44,7 @@ pub(crate) fn rome() -> DailySchedule {
 pub(crate) fn smell(at: DateTime<Utc>, intensity: u8) -> SmellEntry {
     SmellEntry {
         observed_at: at,
-        values: report_values(intensity, &[], &[]),
+        values: report_values(intensity, &[FIRST_ROOM], &[]),
         pm25: None,
     }
 }
diff --git a/crates/storage-contract/src/migrations.rs b/crates/storage-contract/src/migrations.rs
new file mode 100644
index 0000000..b912d8a
--- /dev/null
+++ b/crates/storage-contract/src/migrations.rs
@@ -0,0 +1,43 @@
+//! Data written before a migration, and what each backend must make of it. Backends load the
+//! rows into a database migrated up to the version before, then run the remaining migrations.
+
+use smoke_domain::repos::ReportRepo;
+use smoke_domain::types::{Level, ReportId, RoomId};
+
+/// Rows as the schema of migration 0002 stores them: two rooms; report 1 at intensity 6 in
+/// both, report 2 at intensity 4 in the second.
+pub const ROWS_BEFORE_0003: &str = "\
+    INSERT INTO users (id, username, role, locale, created_at) \
+        VALUES (1, 'daniele', 'resident', 'en', 0); \
+    INSERT INTO rooms (id, name, area) VALUES (1, 'Kitchen', 'inside'), (2, 'Bedroom', 'inside'); \
+    INSERT INTO events (id, owner_id, opened_at, recorded_at) VALUES (1, 1, 0, 0); \
+    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
+        VALUES (1, 1, 1, 0, 0, 6), (2, 1, 1, 60000000, 60000000, 4); \
+    INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES (1, 1), (1, 2), (2, 2);";
+
+/// A report without rooms, which migration 0003 refuses to convert.
+pub const REPORT_WITHOUT_ROOMS: &str = "\
+    INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
+        VALUES (3, 1, 1, 120000000, 120000000, 5);";
+
+/// The name the refusal carries in its error message.
+pub const NEEDS_A_ROOM: &str = "every_smell_report_needs_a_room";
+
+/// After migration 0003, every room of [`ROWS_BEFORE_0003`] has its report's intensity.
+pub async fn each_room_took_its_reports_intensity<S: ReportRepo>(store: &S) {
+    let levels = |id| async move {
+        let report = store.report(ReportId(id)).await.unwrap().unwrap();
+        report
+            .values
+            .rooms()
+            .iter()
+            .map(|r| (r.room, r.level))
+            .collect::<Vec<_>>()
+    };
+    let level = |value| Level::new(value).unwrap();
+    assert_eq!(
+        levels(1).await,
+        [(RoomId(1), level(6)), (RoomId(2), level(6))]
+    );
+    assert_eq!(levels(2).await, [(RoomId(2), level(4))]);
+}
diff --git a/crates/storage-contract/src/reports.rs b/crates/storage-contract/src/reports.rs
index 114bef5..e54c483 100644
--- a/crates/storage-contract/src/reports.rs
+++ b/crates/storage-contract/src/reports.rs
@@ -12,7 +12,7 @@ use smoke_domain::repos::{
 use smoke_domain::types::{CloseReason, EventId, RoomId, SymptomId};
 
 use crate::Fixture;
-use crate::fixture::{add_event, add_report, last_entry, report_values, scene, start};
+use crate::fixture::{FIRST_ROOM, add_event, add_report, last_entry, report_values, scene, start};
 
 pub async fn a_report_keeps_its_rooms_and_symptoms<S>(fx: &Fixture<S>)
 where
@@ -27,11 +27,11 @@ where
         fx,
         &scene,
         observed_at,
-        report_values(6, &[r2, r0, r2], &[scene.symptoms[0].id]),
+        report_values(6, &[r2, r0], &[scene.symptoms[0].id]),
     )
     .await;
 
-    assert_eq!(report.values.rooms(), [r0, r2]);
+    assert_eq!(report.values.room_ids(), [r0, r2]);
     assert_eq!(report.values.symptoms(), [scene.symptoms[0].id]);
     assert_eq!(
         (
@@ -81,9 +81,11 @@ where
     assert_eq!(entry.at_micros, fx.clock.now().timestamp_micros());
     let before = entry.before.unwrap();
     let after = entry.after.unwrap();
-    assert_eq!(before["intensity"], 6);
-    assert_eq!(after["intensity"], 7);
-    assert_eq!(after["rooms"], json!([r0.0, r2.0]));
+    assert_eq!(before["rooms"], json!([{ "room": r0.0, "level": 6 }]));
+    assert_eq!(
+        after["rooms"],
+        json!([{ "room": r0.0, "level": 7 }, { "room": r2.0, "level": 7 }])
+    );
 }
 
 pub async fn moving_a_report_is_audited<S>(fx: &Fixture<S>)
@@ -105,7 +107,7 @@ where
         }),
     )
     .await;
-    let report = add_report(fx, &scene, start(), report_values(4, &[], &[])).await;
+    let report = add_report(fx, &scene, start(), report_values(4, &[FIRST_ROOM], &[])).await;
 
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
     let moved = tx.move_report(report.id, other.id).await.unwrap();
@@ -140,7 +142,7 @@ where
         + AuditRepo,
 {
     let scene = scene(fx).await;
-    let report = add_report(fx, &scene, start(), report_values(5, &[], &[])).await;
+    let report = add_report(fx, &scene, start(), report_values(5, &[FIRST_ROOM], &[])).await;
     fx.clock.advance(TimeDelta::minutes(1));
 
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
@@ -148,7 +150,7 @@ where
     assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
     for error in [
         tx.delete_report(report.id).await.unwrap_err(),
-        tx.correct_report(report.id, start(), report_values(6, &[], &[]))
+        tx.correct_report(report.id, start(), report_values(6, &[FIRST_ROOM], &[]))
             .await
             .unwrap_err(),
         tx.move_report(report.id, scene.event.id).await.unwrap_err(),
@@ -178,8 +180,8 @@ where
     let scene = scene(fx).await;
     let earlier = start() - TimeDelta::minutes(30);
     let later = start() - TimeDelta::minutes(10);
-    add_report(fx, &scene, earlier, report_values(3, &[], &[])).await;
-    let last = add_report(fx, &scene, later, report_values(5, &[], &[])).await;
+    add_report(fx, &scene, earlier, report_values(3, &[FIRST_ROOM], &[])).await;
+    let last = add_report(fx, &scene, later, report_values(5, &[FIRST_ROOM], &[])).await;
 
     let mut tx = fx.store.begin(None).await.unwrap();
     assert_eq!(
@@ -265,10 +267,10 @@ where
     tx.commit().await.unwrap();
 
     let stored = fx.store.report(report.id).await.unwrap().unwrap();
-    assert_eq!(stored.values.rooms(), [r2]);
+    assert_eq!(stored.values.room_ids(), [r2]);
     assert!(stored.values.symptoms().is_empty());
     let after = last_entry(fx).await.record.after.unwrap();
-    assert_eq!(after["rooms"], json!([r2.0]));
+    assert_eq!(after["rooms"], json!([{ "room": r2.0, "level": 6 }]));
     assert_eq!(after["symptoms"], json!([]));
 }
 
@@ -311,7 +313,7 @@ where
         + AuditRepo,
 {
     let scene = scene(fx).await;
-    let report = add_report(fx, &scene, start(), report_values(4, &[], &[])).await;
+    let report = add_report(fx, &scene, start(), report_values(4, &[FIRST_ROOM], &[])).await;
     let entries_before = fx.store.audit_log().await.unwrap().len();
 
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
@@ -342,7 +344,7 @@ where
             event_id: scene.event.id,
             user_id: scene.user.id,
             observed_at: start(),
-            values: report_values(5, &[], &[SymptomId(999)]),
+            values: report_values(5, &[FIRST_ROOM], &[SymptomId(999)]),
         })
         .await
         .unwrap_err();
@@ -373,7 +375,7 @@ where
     let at = |minutes: i64| start() - TimeDelta::minutes(minutes);
     let before = add_report(fx, &scene, at(50), report_values(2, &[r0], &[])).await;
     let first = add_report(fx, &scene, at(40), report_values(5, &[r1, r0], &[s0])).await;
-    let second = add_report(fx, &scene, at(30), report_values(6, &[], &[])).await;
+    let second = add_report(fx, &scene, at(30), report_values(6, &[FIRST_ROOM], &[])).await;
     let deleted = add_report(fx, &scene, at(20), report_values(7, &[r0], &[s0])).await;
     let at_end = add_report(fx, &scene, at(10), report_values(8, &[r1], &[])).await;
     let mut tx = fx.store.begin(None).await.unwrap();
@@ -382,7 +384,7 @@ where
 
     let found = fx.store.reports_between(at(40), at(10)).await.unwrap();
     assert_eq!(found, [first.clone(), second]);
-    assert_eq!(found[0].values.rooms(), [r0, r1]);
+    assert_eq!(found[0].values.room_ids(), [r0, r1]);
     assert_eq!(found[0].values.symptoms(), [s0]);
     assert!(!found.contains(&before) && !found.contains(&at_end));
     assert!(
diff --git a/crates/storage-contract/src/sweep.rs b/crates/storage-contract/src/sweep.rs
index 7439c45..e64fc08 100644
--- a/crates/storage-contract/src/sweep.rs
+++ b/crates/storage-contract/src/sweep.rs
@@ -6,15 +6,16 @@ use smoke_domain::audit::{AuditAction, AuditEntity};
 use smoke_domain::clock::Clock;
 use smoke_domain::records::{Event, NewReport};
 use smoke_domain::repos::{
-    AuditRepo, Commit, EventRepo, EventWrites, JobRepo, JobWrites, ReportWrites, Transactional,
-    UserWrites,
+    AuditRepo, Commit, EventRepo, EventWrites, JobRepo, JobWrites, ListRepo, ListWrites,
+    ReportWrites, Transactional, UserWrites,
 };
 use smoke_domain::schedule::DailySchedule;
+use smoke_domain::starter::seed_starter_lists;
 use smoke_domain::sweep::{AUTO_CLOSE_JOB, SweepOutcome, auto_close};
 use smoke_domain::types::{CloseReason, Role, UserId};
 
 use crate::Fixture;
-use crate::fixture::{add_event, add_user, last_entry, report_values};
+use crate::fixture::{FIRST_ROOM, add_event, add_user, last_entry, report_values};
 
 fn t(s: &str) -> DateTime<Utc> {
     s.parse().expect("valid instant")
@@ -33,7 +34,7 @@ where
         event_id: event.id,
         user_id: user,
         observed_at: at,
-        values: report_values(5, &[], &[]),
+        values: report_values(5, &[FIRST_ROOM], &[]),
     })
     .await
     .unwrap();
@@ -53,8 +54,9 @@ where
 /// An evening event (20:00–21:30 UTC) on the night before `date`, with no sweep since.
 async fn evening_event<S>(fx: &Fixture<S>, user: UserId, date: &str) -> Event
 where
-    S: Transactional<Tx: EventWrites + ReportWrites + JobWrites> + JobRepo,
+    S: Transactional<Tx: EventWrites + ReportWrites + JobWrites + ListWrites> + JobRepo + ListRepo,
 {
+    seed_starter_lists(&fx.store).await.unwrap();
     let opened = t(&format!("{date}T20:00:00Z")) - TimeDelta::days(1);
     let mut tx = fx.store.begin(None).await.unwrap();
     tx.record_job_run(AUTO_CLOSE_JOB, opened).await.unwrap();
@@ -83,8 +85,9 @@ where
 
 pub async fn a_sweep_at_four_closes_the_event_at_its_last_report<S>(fx: &Fixture<S>)
 where
-    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites>
+    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
         + JobRepo
+        + ListRepo
         + EventRepo
         + AuditRepo,
 {
@@ -120,7 +123,10 @@ where
 
 pub async fn a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four<S>(fx: &Fixture<S>)
 where
-    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites> + JobRepo + EventRepo,
+    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
+        + JobRepo
+        + ListRepo
+        + EventRepo,
 {
     let user = add_user(fx, "daniele", Role::Resident).await;
     let first = evening_event(fx, user.id, "2026-01-16").await;
@@ -182,7 +188,10 @@ where
 
 pub async fn on_dst_days_the_sweep_still_runs_at_four_local<S>(fx: &Fixture<S>)
 where
-    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites> + JobRepo + EventRepo,
+    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites + ListWrites>
+        + JobRepo
+        + ListRepo
+        + EventRepo,
 {
     let user = add_user(fx, "daniele", Role::Resident).await;
     // Spring forward: 04:00 CEST is 02:00 UTC. Fall back: 04:00 CET is 03:00 UTC.
diff --git a/crates/storage-postgres/tests/contract.rs b/crates/storage-postgres/tests/contract.rs
index 24c8fa9..4893d5e 100644
--- a/crates/storage-postgres/tests/contract.rs
+++ b/crates/storage-postgres/tests/contract.rs
@@ -2,8 +2,9 @@ mod common;
 
 use std::sync::Arc;
 
-use smoke_domain::clock::FakeClock;
+use smoke_domain::clock::{FakeClock, SystemClock};
 use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
+use smoke_storage_contract::migrations;
 use smoke_storage_postgres::PostgresStore;
 use sqlx::PgPool;
 
@@ -37,3 +38,52 @@ fn the_postgres_store_is_a_complete_store() {
     fn assert_store<S: smoke_domain::repos::Store>() {}
     assert_store::<PostgresStore>();
 }
+
+/// A fresh database migrated up to `version`, with `rows` written by hand.
+async fn database_at(version: i64, rows: &'static str) -> Option<(String, PgPool)> {
+    let url = common::fresh_database().await?;
+    let pool = PgPool::connect(&url).await.unwrap();
+    sqlx::migrate!("./migrations")
+        .run_to(version, &pool)
+        .await
+        .unwrap();
+    sqlx::raw_sql(rows).execute(&pool).await.unwrap();
+    Some((url, pool))
+}
+
+#[tokio::test]
+async fn migration_0003_gives_each_room_its_reports_intensity() {
+    let Some((url, _pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
+        return;
+    };
+    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
+        .await
+        .unwrap();
+    store.migrate().await.unwrap();
+    migrations::each_room_took_its_reports_intensity(&store).await;
+}
+
+#[tokio::test]
+async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
+    let Some((url, pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
+        return;
+    };
+    sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
+        .execute(&pool)
+        .await
+        .unwrap();
+    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
+        .await
+        .unwrap();
+    let error = store.migrate().await.unwrap_err();
+    assert!(
+        format!("{error:?}").contains(migrations::NEEDS_A_ROOM),
+        "{error:?}"
+    );
+    let intensities: Vec<i64> =
+        sqlx::query_scalar("SELECT intensity FROM smell_reports ORDER BY id")
+            .fetch_all(&pool)
+            .await
+            .unwrap();
+    assert_eq!(intensities, [6, 4, 5]);
+}
diff --git a/crates/storage-sqlite/tests/contract.rs b/crates/storage-sqlite/tests/contract.rs
index 67dfd6c..977e4b7 100644
--- a/crates/storage-sqlite/tests/contract.rs
+++ b/crates/storage-sqlite/tests/contract.rs
@@ -2,6 +2,7 @@ use std::sync::Arc;
 
 use smoke_domain::clock::{FakeClock, SystemClock};
 use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
+use smoke_storage_contract::migrations;
 use smoke_storage_sqlite::SqliteStore;
 use sqlx::SqlitePool;
 use tempfile::TempDir;
@@ -138,3 +139,52 @@ async fn a_second_writer_waits_at_begin_instead_of_failing_at_commit() {
         .await
         .expect("the lock is free again once the first transaction ends");
 }
+
+/// A database migrated up to `version`, with `rows` written by hand.
+async fn database_at(version: i64, rows: &'static str) -> (TempDir, String) {
+    let (dir, url) = database();
+    let pool = SqlitePool::connect(&format!("{url}?mode=rwc"))
+        .await
+        .unwrap();
+    sqlx::migrate!("./migrations")
+        .run_to(version, &pool)
+        .await
+        .unwrap();
+    sqlx::raw_sql(rows).execute(&pool).await.unwrap();
+    pool.close().await;
+    (dir, url)
+}
+
+#[tokio::test]
+async fn migration_0003_gives_each_room_its_reports_intensity() {
+    let (_dir, url) = database_at(2, migrations::ROWS_BEFORE_0003).await;
+    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
+        .await
+        .unwrap();
+    store.migrate().await.unwrap();
+    migrations::each_room_took_its_reports_intensity(&store).await;
+}
+
+#[tokio::test]
+async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
+    let (_dir, url) = database_at(2, migrations::ROWS_BEFORE_0003).await;
+    let pool = SqlitePool::connect(&url).await.unwrap();
+    sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
+        .execute(&pool)
+        .await
+        .unwrap();
+    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
+        .await
+        .unwrap();
+    let error = store.migrate().await.unwrap_err();
+    assert!(
+        format!("{error:?}").contains(migrations::NEEDS_A_ROOM),
+        "{error:?}"
+    );
+    let intensities: Vec<i64> =
+        sqlx::query_scalar("SELECT intensity FROM smell_reports ORDER BY id")
+            .fetch_all(&pool)
+            .await
+            .unwrap();
+    assert_eq!(intensities, [6, 4, 5]);
+}
diff --git a/crates/web/tests/common/mod.rs b/crates/web/tests/common/mod.rs
index 020579e..362be7d 100644
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -14,13 +14,13 @@ use chrono::{DateTime, TimeDelta, Utc};
 use http_body_util::BodyExt;
 use smoke_domain::clock::{Clock, FakeClock, SystemClock};
 use smoke_domain::logbook::{EventChoice, Logged, SmellEntry, log_reading, log_smell};
-use smoke_domain::records::{NewUser, User};
+use smoke_domain::records::{NewUser, RoomLevel, User};
 use smoke_domain::records::{Pm25Reading, ReportValues};
 use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
 use smoke_domain::schedule::DailySchedule;
 use smoke_domain::starter::seed_starter_lists;
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{Intensity, Pm25, RoomId};
+use smoke_domain::types::{Level, Pm25, RoomId};
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
 use smoke_web::config::{BaseUrl, WebConfig};
@@ -103,20 +103,28 @@ impl TestApp {
             .id
     }
 
-    /// Logs a report through the domain, as the Log form would, at `at` (RFC 3339).
+    /// Logs a report through the domain, as the Log form would, at `at` (RFC 3339): every room
+    /// in `rooms` at `level`, or the living room alone when `rooms` is empty.
     pub async fn log(
         &self,
         user: &User,
         at: &str,
-        intensity: u8,
+        level: u8,
         rooms: &[RoomId],
         choice: EventChoice,
     ) -> Logged {
+        let level = Level::new(level).unwrap();
+        let rooms = match rooms {
+            [] => vec![self.room("room-living-room").await],
+            rooms => rooms.to_vec(),
+        };
         let entry = SmellEntry {
             observed_at: t(at),
             values: ReportValues::new(
-                Intensity::new(intensity).unwrap(),
-                rooms.to_vec(),
+                rooms
+                    .into_iter()
+                    .map(|room| RoomLevel { room, level })
+                    .collect(),
                 vec![],
                 None,
             )
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index 4e772de..493749d 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -151,7 +151,7 @@ async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
     assert!(html.contains("Started by daniele"), "{html}");
     assert!(
         html.contains(&format!(
-            r#"<a href="/reports/{}">intensity 3 · Bedroom</a>"#,
+            r#"<a href="/reports/{}">intensity 3 · Bedroom 3</a>"#,
             first.report.id
         )),
         "{html}"
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index 022b40c..a3376df 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -27,7 +27,10 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
         html.contains(r#"<form method="post" action="/log""#),
         "{html}"
     );
-    assert_eq!(html.matches(r#"name="intensity""#).count(), 10);
+    let bedroom_id = app.room("room-bedroom").await;
+    let level_row = format!(r#"name="level-{bedroom_id}""#);
+    assert_eq!(html.matches(&level_row).count(), 11, "levels 0 to 10");
+    assert!(html.contains("<legend>Level in Bedroom</legend>"), "{html}");
     let inside = html.find("Inside").unwrap();
     let common = html.find("Common areas").unwrap();
     let bedroom = html.find("<span>Bedroom</span>").unwrap();
@@ -250,6 +253,10 @@ async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
     );
     assert!(html.contains(r#"<dialog id="rating-guide""#), "{html}");
     assert!(html.contains(r#"<form method="dialog""#), "{html}");
+    assert!(
+        html.contains(r#"<tr><th scope="row">0</th><td>Pulito</td></tr>"#),
+        "{html}"
+    );
     assert!(html.contains("<td>Appena percepibile</td>"), "{html}");
     assert!(
         html.contains("Tra due numeri, scegli il più basso."),
@@ -261,6 +268,10 @@ async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
     assert_eq!(guide.status(), StatusCode::OK);
     let guide = text(guide).await;
     assert!(guide.contains("<h1>Come valutare</h1>"), "{guide}");
+    assert!(
+        guide.contains("Hai controllato: nessun odore di fumo."),
+        "{guide}"
+    );
     assert!(
         guide.contains("Come una stanza piena di fumo. Devi uscire."),
         "{guide}"
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index a28e34d..6b7384e 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -23,7 +23,7 @@ async fn logging_with_nothing_open_saves_at_once_and_says_so() {
     let cookie = app.sign_in("daniele").await;
 
     let body = format!(
-        "intensity=6&room={}&note=from+the+balcony&action=smell",
+        "room={0}&level-{0}=6&note=from+the+balcony&action=smell",
         bedroom.0
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -32,7 +32,8 @@ async fn logging_with_nothing_open_saves_at_once_and_says_so() {
 
     let open = app.store.open_event().await.unwrap().unwrap();
     let reports = app.store.reports_for_event(open.id).await.unwrap();
-    assert_eq!(reports[0].values.rooms(), [bedroom]);
+    assert_eq!(reports[0].values.room_ids(), [bedroom]);
+    assert_eq!(reports[0].values.intensity().get(), 6);
     assert_eq!(reports[0].values.note(), Some("from the balcony"));
     assert_eq!(reports[0].observed_at, t("2026-01-15T20:00:00Z"));
     let html = text(app.send(get_with("/?saved=report", &cookie)).await).await;
@@ -56,19 +57,23 @@ async fn with_an_event_open_the_same_smell_question_comes_first() {
     let cookie = app.sign_in("daniele").await;
 
     let question = app
-        .send(post_form("/log", "intensity=7&action=smell", Some(&cookie)))
+        .send(post_form(
+            "/log",
+            "room=1&level-1=7&action=smell",
+            Some(&cookie),
+        ))
         .await;
     assert_eq!(question.status(), StatusCode::OK);
     let html = text(question).await;
     assert!(html.contains("<h1>Same smell still going?</h1>"), "{html}");
     assert!(html.contains("(started 19:05)"), "{html}");
-    assert!(html.contains(r#"<input type="hidden" name="intensity" value="7">"#));
+    assert!(html.contains(r#"<input type="hidden" name="level-1" value="7">"#));
     let id = first.report.event_id;
     assert!(html.contains(&format!(r#"value="join-{id}">Yes, update</button>"#)));
     assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));
 
     let body = format!(
-        "intensity=7&action=smell&asked={}&choice=join-{id}",
+        "room=1&level-1=7&action=smell&asked={}&choice=join-{id}",
         asked(&html)
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -92,7 +97,7 @@ async fn answering_no_closes_the_open_event_at_its_last_report() {
         .await;
     let cookie = app.sign_in("daniele").await;
     let body = format!(
-        "intensity=3&action=smell&asked=same-smell-{}&choice=new",
+        "room=1&level-1=3&action=smell&asked=same-smell-{}&choice=new",
         first.report.event_id
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -143,8 +148,12 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     let cookie = app.sign_in("guest").await;
 
     let html = text(
-        app.send(post_form("/log", "intensity=2&action=smell", Some(&cookie)))
-            .await,
+        app.send(post_form(
+            "/log",
+            "room=1&level-1=2&action=smell",
+            Some(&cookie),
+        ))
+        .await,
     )
     .await;
     assert!(
@@ -158,7 +167,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     let past = text(
         app.send(post_form(
             "/log",
-            "intensity=2&action=smell&observed_at=2026-01-15T11:30",
+            "room=1&level-1=2&action=smell&observed_at=2026-01-15T11:30",
             Some(&cookie),
         ))
         .await,
@@ -198,8 +207,12 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
         .await;
     let cookie = app.sign_in("daniele").await;
     let question = text(
-        app.send(post_form("/log", "intensity=7&action=smell", Some(&cookie)))
-            .await,
+        app.send(post_form(
+            "/log",
+            "room=1&level-1=7&action=smell",
+            Some(&cookie),
+        ))
+        .await,
     )
     .await;
     // Meanwhile the partner's phone answers "No, it had stopped".
@@ -216,7 +229,7 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
         .await;
 
     let body = format!(
-        "intensity=7&action=smell&asked={}&choice=join-{}",
+        "room=1&level-1=7&action=smell&asked={}&choice=join-{}",
         asked(&question),
         first.report.event_id
     );
@@ -268,7 +281,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
     let html = text(
         app.send(post_form(
             "/log",
-            "intensity=5&action=smell&observed_at=2026-01-15T12:00",
+            "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00",
             Some(&cookie),
         ))
         .await,
@@ -282,7 +295,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
     assert!(html.contains(r#"value="new" class="secondary">New event</button>"#));
 
     let body = format!(
-        "intensity=5&action=smell&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
+        "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=report"));
@@ -310,8 +323,12 @@ async fn after_an_auto_close_the_next_log_offers_a_continuation() {
     app.at("2026-01-16T06:00:00Z");
 
     let html = text(
-        app.send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
-            .await,
+        app.send(post_form(
+            "/log",
+            "room=1&level-1=5&action=smell",
+            Some(&cookie),
+        ))
+        .await,
     )
     .await;
     assert!(
@@ -319,7 +336,7 @@ async fn after_an_auto_close_the_next_log_offers_a_continuation() {
         "{html}"
     );
     assert_eq!(asked(&html), format!("auto-closed-{id}"));
-    let body = format!("intensity=5&action=smell&asked=auto-closed-{id}&choice=continue-{id}");
+    let body = format!("room=1&level-1=5&action=smell&asked=auto-closed-{id}&choice=continue-{id}");
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=report"));
     let event = app.store.event(id).await.unwrap().unwrap();
@@ -374,7 +391,7 @@ async fn while_another_event_is_open_the_notice_is_a_dismissable_banner() {
 }
 
 #[tokio::test]
-async fn pm25_alone_needs_no_intensity_and_joins_the_open_event() {
+async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
     let app = TestApp::new().await;
     app.at("2026-01-15T20:00:00Z");
     let me = app.add_user("daniele", Role::Resident, Locale::En).await;
@@ -401,7 +418,7 @@ async fn pm25_alone_needs_no_intensity_and_joins_the_open_event() {
     assert_eq!(readings[0].value.ugm3(), 35.5);
 
     let with_smell = format!(
-        "intensity=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
+        "room=1&level-1=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
         open.report.event_id
     );
     app.send(post_form("/log", &with_smell, Some(&cookie)))
@@ -426,7 +443,10 @@ async fn invalid_input_shows_the_form_again_with_what_was_typed() {
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
     let html = text(response).await;
-    assert!(html.contains("Pick an intensity from 1 to 10."), "{html}");
+    assert!(
+        html.contains("Pick a level from 0 to 10 for each selected room."),
+        "{html}"
+    );
     assert!(html.contains(">kept as typed</textarea>"), "{html}");
     assert!(
         html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
@@ -449,7 +469,11 @@ async fn viewers_cannot_log() {
     app.add_user("lawyer", Role::Viewer, Locale::En).await;
     let cookie = app.sign_in("lawyer").await;
     let response = app
-        .send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
+        .send(post_form(
+            "/log",
+            "room=1&level-1=5&action=smell",
+            Some(&cookie),
+        ))
         .await;
     assert_eq!(response.status(), StatusCode::FORBIDDEN);
     assert!(text(response).await.contains("<h1>Not allowed</h1>"));
diff --git a/crates/web/tests/reports.rs b/crates/web/tests/reports.rs
index 6e55adf..4eb361e 100644
--- a/crates/web/tests/reports.rs
+++ b/crates/web/tests/reports.rs
@@ -40,7 +40,7 @@ async fn a_report_shows_both_times_its_values_and_its_history() {
         "recorded: {html}"
     );
     assert!(html.contains("<dd>6 · Strong</dd>"), "{html}");
-    assert!(html.contains("<dd>Bedroom</dd>"), "{html}");
+    assert!(html.contains("<dd>Bedroom 6</dd>"), "{html}");
     assert!(html.contains("<dd>daniele</dd>"), "{html}");
     assert!(
         html.contains("<li>15/01/2026 21:00 · daniele · logged</li>"),
@@ -76,7 +76,10 @@ async fn correcting_a_mistake_is_recorded_and_badged() {
             .await,
     )
     .await;
-    assert!(form.contains(r#"value="6" required checked>"#), "{form}");
+    assert!(
+        form.contains(&format!(r#"name="level-{}" value="6" checked>"#, bedroom.0)),
+        "{form}"
+    );
     assert!(
         form.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
         "{form}"
@@ -89,7 +92,7 @@ async fn correcting_a_mistake_is_recorded_and_badged() {
 
     app.at("2026-01-16T08:15:00Z");
     let body = format!(
-        "intensity=7&room={}&room={}&observed_at=2026-01-15T19:05&note=",
+        "room={0}&level-{0}=7&room={1}&level-{1}=3&observed_at=2026-01-15T19:05&note=",
         bedroom.0, kitchen.0
     );
     let response = app
@@ -107,7 +110,7 @@ async fn correcting_a_mistake_is_recorded_and_badged() {
     );
     assert!(
         html.contains(
-            "<li>16/01/2026 09:15 · daniele · corrected: intensity 6 → 7, added Kitchen</li>"
+            "<li>16/01/2026 09:15 · daniele · corrected: Bedroom 6 → 7, added Kitchen 3</li>"
         ),
         "{html}"
     );
@@ -132,7 +135,7 @@ async fn a_correction_must_stay_inside_the_event() {
     let response = app
         .send(post_form(
             &format!("/reports/{id}/correct"),
-            "intensity=6&observed_at=2026-01-15T18:00",
+            "room=1&level-1=6&observed_at=2026-01-15T18:00",
             Some(&cookie),
         ))
         .await;
@@ -262,7 +265,7 @@ async fn guests_cannot_see_or_change_other_peoples_reports() {
                 .await,
             app.send(post_form(
                 &format!("/reports/{id}/correct"),
-                "intensity=1",
+                "room=1&level-1=1",
                 Some(&cookie),
             ))
             .await,
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index 6a59ba8..de3a1db 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -183,10 +183,18 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
     let app = TestApp::new().await;
     app.add_user("daniele", Role::Resident, Locale::En).await;
     let cookie = app.sign_in("daniele").await;
-    app.send(post_form("/log", "intensity=4&action=smell", Some(&cookie)))
-        .await;
+    app.send(post_form(
+        "/log",
+        "room=1&level-1=4&action=smell",
+        Some(&cookie),
+    ))
+    .await;
     let question = app
-        .send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
+        .send(post_form(
+            "/log",
+            "room=1&level-1=5&action=smell",
+            Some(&cookie),
+        ))
         .await;
     let pages = [
         text(question).await,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace`
Expected: compile errors — `Level`, `RoomLevel`, `FIRST_ROOM` and `smoke_storage_contract::migrations` do not exist, and `ReportValues::new` takes four arguments.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/domain/src/error.rs b/crates/domain/src/error.rs
index c6588e4..23dc31f 100644
--- a/crates/domain/src/error.rs
+++ b/crates/domain/src/error.rs
@@ -3,8 +3,14 @@ use thiserror::Error;
 /// Every way a domain rule can reject input.
 #[derive(Debug, Clone, PartialEq, Error)]
 pub enum DomainError {
-    #[error("intensity must be between 1 and 10, got {0}")]
-    InvalidIntensity(u8),
+    #[error("a level must be between 0 and 10, got {0}")]
+    InvalidLevel(u8),
+
+    #[error("a room can be listed only once in a report, got room {0} twice")]
+    RoomListedTwice(i64),
+
+    #[error("a report needs at least one room")]
+    NoRooms,
 
     #[error("PM2.5 must be a number between 0 and 1000 µg/m³, got {0}")]
     InvalidPm25(f64),
diff --git a/crates/domain/src/history.rs b/crates/domain/src/history.rs
index 5874b66..ac3e14b 100644
--- a/crates/domain/src/history.rs
+++ b/crates/domain/src/history.rs
@@ -1,11 +1,12 @@
 //! A smell report's History view (spec §6.1): what each audit entry changed, e.g.
-//! "intensity 6 → 7, added bedroom".
+//! "living room 6 → 7, added bedroom 3".
 
 use chrono::{DateTime, Utc};
 use serde::Deserialize;
 
 use crate::audit::{AuditAction, ChainedEntry};
-use crate::types::{EventId, RoomId, SymptomId, UserId};
+use crate::records::{RoomLevel, SmellReport};
+use crate::types::{EventId, Level, RoomId, SymptomId, UserId};
 
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub enum Change {
@@ -13,11 +14,13 @@ pub enum Change {
         from: DateTime<Utc>,
         to: DateTime<Utc>,
     },
-    Intensity {
-        from: u8,
-        to: u8,
+    /// A room listed before and after, at another level.
+    LevelChanged {
+        room: RoomId,
+        from: Level,
+        to: Level,
     },
-    RoomAdded(RoomId),
+    RoomAdded(RoomLevel),
     RoomRemoved(RoomId),
     SymptomAdded(SymptomId),
     SymptomRemoved(SymptomId),
@@ -40,13 +43,13 @@ pub struct HistoryEntry {
     pub changes: Vec<Change>,
 }
 
-/// The audited fields of a smell report, as `SmellReport` serialises them.
+/// The audited fields of a smell report, as `SmellReport` serialises them; older snapshots are
+/// read through [`SmellReport::upgrade_snapshot`].
 #[derive(Deserialize)]
 struct Snapshot {
     event_id: EventId,
     observed_at: DateTime<Utc>,
-    intensity: u8,
-    rooms: Vec<RoomId>,
+    rooms: Vec<RoomLevel>,
     symptoms: Vec<SymptomId>,
     note: Option<String>,
 }
@@ -58,7 +61,8 @@ pub fn report_history(entries: &[ChainedEntry]) -> Vec<HistoryEntry> {
         .map(|entry| {
             let record = &entry.record;
             let snapshot = |value: &Option<serde_json::Value>| -> Option<Snapshot> {
-                serde_json::from_value(value.clone()?).ok()
+                let value = SmellReport::upgrade_snapshot(value.as_ref()?).into_owned();
+                serde_json::from_value(value).ok()
             };
             let changes = match (snapshot(&record.before), snapshot(&record.after)) {
                 (Some(before), Some(after)) => changes(&before, &after),
@@ -99,21 +103,32 @@ fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
             to: after.observed_at,
         });
     }
-    if before.intensity != after.intensity {
-        changes.push(Change::Intensity {
-            from: before.intensity,
-            to: after.intensity,
-        });
+    let level_in =
+        |rooms: &[RoomLevel], room: RoomId| rooms.iter().find(|r| r.room == room).map(|r| r.level);
+    for now in &after.rooms {
+        if let Some(was) = level_in(&before.rooms, now.room)
+            && was != now.level
+        {
+            changes.push(Change::LevelChanged {
+                room: now.room,
+                from: was,
+                to: now.level,
+            });
+        }
     }
     changes.extend(
-        added(&before.rooms, &after.rooms)
-            .into_iter()
-            .map(Change::RoomAdded),
+        after
+            .rooms
+            .iter()
+            .filter(|now| level_in(&before.rooms, now.room).is_none())
+            .map(|now| Change::RoomAdded(*now)),
     );
     changes.extend(
-        added(&after.rooms, &before.rooms)
-            .into_iter()
-            .map(Change::RoomRemoved),
+        before
+            .rooms
+            .iter()
+            .filter(|was| level_in(&after.rooms, was.room).is_none())
+            .map(|was| Change::RoomRemoved(was.room)),
     );
     changes.extend(
         added(&before.symptoms, &after.symptoms)
@@ -138,14 +153,21 @@ fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
 mod tests {
     use super::*;
     use crate::audit::{AuditEntity, AuditRecord, GENESIS_HASH};
-    use crate::records::{Audited, ReportValues, SmellReport};
-    use crate::types::{Intensity, ReportId};
+    use crate::records::{Audited, ReportValues};
+    use crate::types::ReportId;
 
     fn t(s: &str) -> DateTime<Utc> {
         s.parse().unwrap()
     }
 
-    fn report(intensity: u8, rooms: &[i64]) -> SmellReport {
+    fn at(room: i64, level: u8) -> RoomLevel {
+        RoomLevel {
+            room: RoomId(room),
+            level: Level::new(level).unwrap(),
+        }
+    }
+
+    fn report(rooms: &[(i64, u8)]) -> SmellReport {
         SmellReport {
             id: ReportId(9),
             event_id: EventId(4),
@@ -153,8 +175,7 @@ mod tests {
             observed_at: t("2026-01-15T19:00:00Z"),
             recorded_at: t("2026-01-15T19:00:01Z"),
             values: ReportValues::new(
-                Intensity::new(intensity).unwrap(),
-                rooms.iter().copied().map(RoomId).collect(),
+                rooms.iter().map(|&(room, level)| at(room, level)).collect(),
                 vec![SymptomId(1)],
                 None,
             )
@@ -185,15 +206,10 @@ mod tests {
 
     #[test]
     fn a_correction_lists_each_changed_value() {
-        let before = report(6, &[1]);
+        let before = report(&[(2, 4), (3, 6)]);
         let mut after = before.clone();
-        after.values = ReportValues::new(
-            Intensity::new(7).unwrap(),
-            vec![RoomId(1), RoomId(3)],
-            vec![],
-            Some("from the balcony"),
-        )
-        .unwrap();
+        after.values =
+            ReportValues::new(vec![at(1, 3), at(3, 7)], vec![], Some("from the balcony")).unwrap();
         after.observed_at = t("2026-01-15T19:30:00Z");
         let history = report_history(&[
             entry(AuditAction::Create, None, &before),
@@ -209,8 +225,13 @@ mod tests {
                     from: t("2026-01-15T19:00:00Z"),
                     to: t("2026-01-15T19:30:00Z"),
                 },
-                Change::Intensity { from: 6, to: 7 },
-                Change::RoomAdded(RoomId(3)),
+                Change::LevelChanged {
+                    room: RoomId(3),
+                    from: Level::new(6).unwrap(),
+                    to: Level::new(7).unwrap(),
+                },
+                Change::RoomAdded(at(1, 3)),
+                Change::RoomRemoved(RoomId(2)),
                 Change::SymptomRemoved(SymptomId(1)),
                 Change::Note {
                     from: None,
@@ -223,7 +244,7 @@ mod tests {
 
     #[test]
     fn a_move_names_both_events_and_is_not_a_correction() {
-        let before = report(6, &[1]);
+        let before = report(&[(1, 6)]);
         let mut after = before.clone();
         after.event_id = EventId(5);
         let history = report_history(&[
@@ -240,9 +261,37 @@ mod tests {
         assert!(!was_corrected(&history));
     }
 
+    #[test]
+    fn a_correction_audited_before_room_levels_compares_every_room_at_its_intensity() {
+        let mut old = entry(
+            AuditAction::Update,
+            Some(&report(&[(1, 6)])),
+            &report(&[(1, 6)]),
+        );
+        old.record.before = Some(serde_json::json!({
+            "event_id": 4, "observed_at": "2026-01-15T19:00:00Z",
+            "intensity": 6, "rooms": [1], "symptoms": [1], "note": null,
+        }));
+        old.record.after = Some(serde_json::json!({
+            "event_id": 4, "observed_at": "2026-01-15T19:00:00Z",
+            "intensity": 7, "rooms": [1, 3], "symptoms": [1], "note": null,
+        }));
+        assert_eq!(
+            report_history(&[old])[0].changes,
+            [
+                Change::LevelChanged {
+                    room: RoomId(1),
+                    from: Level::new(6).unwrap(),
+                    to: Level::new(7).unwrap(),
+                },
+                Change::RoomAdded(at(3, 7)),
+            ]
+        );
+    }
+
     #[test]
     fn a_deletion_has_no_value_changes() {
-        let before = report(6, &[1]);
+        let before = report(&[(1, 6)]);
         let mut after = before.clone();
         after.deleted_at = Some(t("2026-01-16T08:15:00Z"));
         let history = report_history(&[entry(AuditAction::Delete, Some(&before), &after)]);
diff --git a/crates/domain/src/records.rs b/crates/domain/src/records.rs
index 42b4636..2d1d194 100644
--- a/crates/domain/src/records.rs
+++ b/crates/domain/src/records.rs
@@ -1,10 +1,11 @@
 //! Stored records and the inputs that create them. Each record serialises to the snapshot the
 //! audit log keeps of it.
 
+use std::borrow::Cow;
 use std::fmt;
 
 use chrono::{DateTime, Utc};
-use serde::{Serialize, Serializer};
+use serde::{Deserialize, Serialize, Serializer};
 use serde_json::Value;
 
 use crate::DomainError;
@@ -12,7 +13,7 @@ use crate::audit::AuditEntity;
 use crate::events::EventSummary;
 use crate::permissions::Actor;
 use crate::types::{
-    CloseReason, EventId, Intensity, ListName, Locale, Pm25, ReadingId, ReportId, Role, RoomArea,
+    CloseReason, EventId, Level, ListName, Locale, Pm25, ReadingId, ReportId, Role, RoomArea,
     RoomId, SymptomId, UserId, Username,
 };
 
@@ -157,12 +158,18 @@ impl Audited for Event {
     }
 }
 
-/// What a smell report says. Room and symptom ids are kept sorted and unique, and a blank note
-/// is no note.
+/// A room the person checked, and how strong the smell was there.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
+pub struct RoomLevel {
+    pub room: RoomId,
+    pub level: Level,
+}
+
+/// What a smell report says: the rooms checked, each with its level, sorted by room; the
+/// symptoms, sorted and unique; a note, where a blank note is no note.
 #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
 pub struct ReportValues {
-    intensity: Intensity,
-    rooms: Vec<RoomId>,
+    rooms: Vec<RoomLevel>,
     symptoms: Vec<SymptomId>,
     note: Option<String>,
 }
@@ -171,13 +178,17 @@ impl ReportValues {
     pub const NOTE_MAX_CHARS: usize = 2000;
 
     pub fn new(
-        intensity: Intensity,
-        mut rooms: Vec<RoomId>,
+        mut rooms: Vec<RoomLevel>,
         mut symptoms: Vec<SymptomId>,
         note: Option<&str>,
     ) -> Result<Self, DomainError> {
-        rooms.sort_unstable();
-        rooms.dedup();
+        if rooms.is_empty() {
+            return Err(DomainError::NoRooms);
+        }
+        rooms.sort_unstable_by_key(|r| r.room);
+        if let Some(pair) = rooms.windows(2).find(|pair| pair[0].room == pair[1].room) {
+            return Err(DomainError::RoomListedTwice(pair[0].room.0));
+        }
         symptoms.sort_unstable();
         symptoms.dedup();
         let note = note.map(str::trim).filter(|n| !n.is_empty());
@@ -188,19 +199,32 @@ impl ReportValues {
             }
         }
         Ok(Self {
-            intensity,
             rooms,
             symptoms,
             note: note.map(str::to_owned),
         })
     }
 
-    pub fn intensity(&self) -> Intensity {
-        self.intensity
+    pub fn rooms(&self) -> &[RoomLevel] {
+        &self.rooms
+    }
+
+    pub fn room_ids(&self) -> Vec<RoomId> {
+        self.rooms.iter().map(|r| r.room).collect()
     }
 
-    pub fn rooms(&self) -> &[RoomId] {
-        &self.rooms
+    /// The level reported for `room`; `None` when the room was not checked.
+    pub fn level_in(&self, room: RoomId) -> Option<Level> {
+        self.rooms.iter().find(|r| r.room == room).map(|r| r.level)
+    }
+
+    /// The report's intensity: its highest room level (spec §6).
+    pub fn intensity(&self) -> Level {
+        self.rooms
+            .iter()
+            .map(|r| r.level)
+            .max()
+            .unwrap_or(Level::CLEAN)
     }
 
     pub fn symptoms(&self) -> &[SymptomId] {
@@ -228,6 +252,28 @@ impl SmellReport {
     pub fn is_deleted(&self) -> bool {
         self.deleted_at.is_some()
     }
+
+    /// A snapshot audited before rooms had their own levels (`"intensity": 6, "rooms": [2]`),
+    /// in today's shape: every room at the report's intensity, as migration 0003 converted the
+    /// stored report. Any other snapshot is returned unchanged.
+    pub fn upgrade_snapshot(snapshot: &Value) -> Cow<'_, Value> {
+        let Value::Object(fields) = snapshot else {
+            return Cow::Borrowed(snapshot);
+        };
+        let (Some(intensity), Some(Value::Array(rooms))) =
+            (fields.get("intensity"), fields.get("rooms"))
+        else {
+            return Cow::Borrowed(snapshot);
+        };
+        let mut upgraded = fields.clone();
+        upgraded.remove("intensity");
+        let rooms = rooms
+            .iter()
+            .map(|room| serde_json::json!({ "room": room, "level": intensity }))
+            .collect();
+        upgraded.insert("rooms".to_owned(), Value::Array(rooms));
+        Cow::Owned(Value::Object(upgraded))
+    }
 }
 
 impl Audited for SmellReport {
@@ -346,10 +392,16 @@ mod tests {
         }
     }
 
-    fn values(rooms: Vec<i64>, note: Option<&str>) -> ReportValues {
+    fn at(room: i64, level: u8) -> RoomLevel {
+        RoomLevel {
+            room: RoomId(room),
+            level: Level::new(level).unwrap(),
+        }
+    }
+
+    fn values(rooms: &[(i64, u8)], note: Option<&str>) -> ReportValues {
         ReportValues::new(
-            Intensity::new(6).unwrap(),
-            rooms.into_iter().map(RoomId).collect(),
+            rooms.iter().map(|&(room, level)| at(room, level)).collect(),
             vec![SymptomId(2), SymptomId(1), SymptomId(2)],
             note,
         )
@@ -388,28 +440,48 @@ mod tests {
     }
 
     #[test]
-    fn report_values_sort_and_dedup_ids_and_trim_the_note() {
-        let v = values(vec![3, 1, 3], Some("  smell from the balcony "));
-        assert_eq!(v.rooms(), [RoomId(1), RoomId(3)]);
+    fn report_values_sort_rooms_and_symptoms_and_trim_the_note() {
+        let v = values(&[(3, 0), (1, 6)], Some("  smell from the balcony "));
+        assert_eq!(v.rooms(), [at(1, 6), at(3, 0)]);
+        assert_eq!(v.room_ids(), [RoomId(1), RoomId(3)]);
         assert_eq!(v.symptoms(), [SymptomId(1), SymptomId(2)]);
         assert_eq!(v.note(), Some("smell from the balcony"));
     }
 
+    #[test]
+    fn a_report_needs_a_room_and_lists_each_room_once() {
+        assert_eq!(
+            ReportValues::new(vec![], vec![], None),
+            Err(DomainError::NoRooms)
+        );
+        assert_eq!(
+            ReportValues::new(vec![at(3, 2), at(1, 1), at(3, 2)], vec![], None),
+            Err(DomainError::RoomListedTwice(3))
+        );
+    }
+
+    #[test]
+    fn the_intensity_is_the_highest_room_level() {
+        let v = values(&[(1, 2), (2, 7), (3, 0)], None);
+        assert_eq!(v.intensity(), Level::new(7).unwrap());
+        assert_eq!(v.level_in(RoomId(3)), Some(Level::CLEAN));
+        assert_eq!(v.level_in(RoomId(4)), None);
+        assert_eq!(values(&[(1, 0)], None).intensity(), Level::CLEAN);
+    }
+
     #[test]
     fn a_blank_note_becomes_none() {
-        assert_eq!(values(vec![], Some("   ")).note(), None);
-        assert_eq!(values(vec![], None).note(), None);
+        assert_eq!(values(&[(1, 1)], Some("   ")).note(), None);
+        assert_eq!(values(&[(1, 1)], None).note(), None);
     }
 
     #[test]
     fn notes_are_limited_in_characters_not_bytes() {
         let at_limit = "ç".repeat(ReportValues::NOTE_MAX_CHARS);
-        assert!(
-            ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&at_limit)).is_ok()
-        );
+        assert!(ReportValues::new(vec![at(1, 1)], vec![], Some(&at_limit)).is_ok());
         let over = "x".repeat(ReportValues::NOTE_MAX_CHARS + 1);
         assert_eq!(
-            ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&over)),
+            ReportValues::new(vec![at(1, 1)], vec![], Some(&over)),
             Err(DomainError::NoteTooLong(2001))
         );
     }
@@ -422,7 +494,7 @@ mod tests {
             user_id: UserId(1),
             observed_at: t("2026-01-15T19:00:00Z"),
             recorded_at: t("2026-01-15T19:00:00.123456Z"),
-            values: values(vec![2], None),
+            values: values(&[(2, 6), (5, 0)], None),
             deleted_at: None,
         };
         assert_eq!(
@@ -433,8 +505,7 @@ mod tests {
                 "user_id": 1,
                 "observed_at": "2026-01-15T19:00:00Z",
                 "recorded_at": "2026-01-15T19:00:00.123456Z",
-                "intensity": 6,
-                "rooms": [2],
+                "rooms": [{ "room": 2, "level": 6 }, { "room": 5, "level": 0 }],
                 "symptoms": [1, 2],
                 "note": null,
                 "deleted_at": null,
@@ -443,6 +514,31 @@ mod tests {
         assert_eq!(report.audit_id(), 9);
     }
 
+    #[test]
+    fn a_snapshot_from_before_room_levels_reads_as_every_room_at_the_intensity() {
+        let old = json!({
+            "id": 9,
+            "event_id": 4,
+            "intensity": 6,
+            "rooms": [2, 5],
+            "symptoms": [1],
+        });
+        assert_eq!(
+            SmellReport::upgrade_snapshot(&old).into_owned(),
+            json!({
+                "id": 9,
+                "event_id": 4,
+                "rooms": [{ "room": 2, "level": 6 }, { "room": 5, "level": 6 }],
+                "symptoms": [1],
+            })
+        );
+        let current = json!({ "id": 9, "rooms": [{ "room": 2, "level": 6 }] });
+        assert!(matches!(
+            SmellReport::upgrade_snapshot(&current),
+            Cow::Borrowed(_)
+        ));
+    }
+
     #[test]
     fn room_snapshot_has_exactly_the_audited_fields() {
         let room = Room {
diff --git a/crates/domain/src/summary.rs b/crates/domain/src/summary.rs
index 0b0fd52..edaddb4 100644
--- a/crates/domain/src/summary.rs
+++ b/crates/domain/src/summary.rs
@@ -12,7 +12,8 @@ pub struct MonthSummary {
     /// Events with at least one of the reports.
     pub events: usize,
     pub reports: usize,
-    /// Local days with a report naming the bedroom; `None` when there is no bedroom room.
+    /// Local days with a report giving the bedroom a level of 1 or more; `None` when there is no
+    /// bedroom room.
     pub bedroom_days: Option<usize>,
 }
 
@@ -27,7 +28,11 @@ pub fn summarise(
     let bedroom_days = bedroom.map(|room| {
         reports
             .iter()
-            .filter(|r| r.values.rooms().contains(&room))
+            .filter(|r| {
+                r.values
+                    .level_in(room)
+                    .is_some_and(|level| !level.is_clean())
+            })
             .map(|r| schedule.local_date(r.observed_at))
             .collect::<BTreeSet<_>>()
             .len()
@@ -44,10 +49,10 @@ mod tests {
     use chrono::{DateTime, Utc};
 
     use super::*;
-    use crate::records::ReportValues;
-    use crate::types::{EventId, Intensity, ReportId, UserId};
+    use crate::records::{ReportValues, RoomLevel};
+    use crate::types::{EventId, Level, ReportId, UserId};
 
-    fn report(id: i64, event: i64, at: &str, rooms: &[i64]) -> SmellReport {
+    fn report(id: i64, event: i64, at: &str, rooms: &[(i64, u8)]) -> SmellReport {
         let at: DateTime<Utc> = at.parse().unwrap();
         SmellReport {
             id: ReportId(id),
@@ -56,8 +61,13 @@ mod tests {
             observed_at: at,
             recorded_at: at,
             values: ReportValues::new(
-                Intensity::new(5).unwrap(),
-                rooms.iter().copied().map(RoomId).collect(),
+                rooms
+                    .iter()
+                    .map(|&(room, level)| RoomLevel {
+                        room: RoomId(room),
+                        level: Level::new(level).unwrap(),
+                    })
+                    .collect(),
                 vec![],
                 None,
             )
@@ -74,17 +84,19 @@ mod tests {
     fn counts_events_reports_and_local_days_with_bedroom_smoke() {
         let bedroom = RoomId(3);
         let reports = [
-            report(1, 1, "2026-01-10T20:00:00Z", &[3]),
-            report(2, 1, "2026-01-10T22:30:00Z", &[1, 3]),
+            report(1, 1, "2026-01-10T20:00:00Z", &[(3, 4)]),
+            report(2, 1, "2026-01-10T22:30:00Z", &[(1, 2), (3, 5)]),
             // 00:30 on the 11th in Rome: a second bedroom day.
-            report(3, 1, "2026-01-10T23:30:00Z", &[3]),
-            report(4, 2, "2026-01-12T08:00:00Z", &[1]),
+            report(3, 1, "2026-01-10T23:30:00Z", &[(3, 1)]),
+            report(4, 2, "2026-01-12T08:00:00Z", &[(1, 6)]),
+            // The bedroom checked and clean: not a day with bedroom smoke.
+            report(5, 2, "2026-01-12T09:00:00Z", &[(1, 6), (3, 0)]),
         ];
         assert_eq!(
             summarise(&reports, Some(bedroom), &rome()),
             MonthSummary {
                 events: 2,
-                reports: 4,
+                reports: 5,
                 bedroom_days: Some(2),
             }
         );
@@ -93,7 +105,12 @@ mod tests {
     #[test]
     fn without_a_bedroom_or_reports_the_counts_are_empty() {
         assert_eq!(
-            summarise(&[report(1, 1, "2026-01-10T20:00:00Z", &[3])], None, &rome()).bedroom_days,
+            summarise(
+                &[report(1, 1, "2026-01-10T20:00:00Z", &[(3, 4)])],
+                None,
+                &rome()
+            )
+            .bedroom_days,
             None
         );
         assert_eq!(
diff --git a/crates/domain/src/types.rs b/crates/domain/src/types.rs
index dedf2b5..13aeae6 100644
--- a/crates/domain/src/types.rs
+++ b/crates/domain/src/types.rs
@@ -224,29 +224,35 @@ impl fmt::Debug for NewPassword {
     }
 }
 
-/// How strong a smell is, on the personal 1–10 scale.
+/// How strong a smell is in one room, on the personal 0–10 scale: 0 means the room was checked
+/// and smelled clean.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
 #[serde(try_from = "u8", into = "u8")]
-pub struct Intensity(u8);
+pub struct Level(u8);
 
-impl Intensity {
-    pub const MIN: u8 = 1;
+impl Level {
+    pub const MIN: u8 = 0;
     pub const MAX: u8 = 10;
+    pub const CLEAN: Level = Level(0);
 
     pub fn new(value: u8) -> Result<Self, DomainError> {
-        if (Self::MIN..=Self::MAX).contains(&value) {
+        if value <= Self::MAX {
             Ok(Self(value))
         } else {
-            Err(DomainError::InvalidIntensity(value))
+            Err(DomainError::InvalidLevel(value))
         }
     }
 
     pub fn get(self) -> u8 {
         self.0
     }
+
+    pub fn is_clean(self) -> bool {
+        self == Self::CLEAN
+    }
 }
 
-impl TryFrom<u8> for Intensity {
+impl TryFrom<u8> for Level {
     type Error = DomainError;
 
     fn try_from(value: u8) -> Result<Self, Self::Error> {
@@ -254,9 +260,9 @@ impl TryFrom<u8> for Intensity {
     }
 }
 
-impl From<Intensity> for u8 {
-    fn from(intensity: Intensity) -> Self {
-        intensity.0
+impl From<Level> for u8 {
+    fn from(level: Level) -> Self {
+        level.0
     }
 }
 
@@ -300,26 +306,24 @@ mod tests {
     use super::*;
 
     #[test]
-    fn intensity_accepts_the_full_range() {
-        for value in 1..=10 {
-            assert_eq!(Intensity::new(value).unwrap().get(), value);
+    fn level_accepts_zero_to_ten() {
+        for value in 0..=10 {
+            assert_eq!(Level::new(value).unwrap().get(), value);
         }
+        assert!(Level::new(0).unwrap().is_clean());
+        assert!(!Level::new(1).unwrap().is_clean());
     }
 
     #[test]
-    fn intensity_rejects_values_outside_one_to_ten() {
-        assert_eq!(Intensity::new(0), Err(DomainError::InvalidIntensity(0)));
-        assert_eq!(Intensity::new(11), Err(DomainError::InvalidIntensity(11)));
+    fn level_rejects_values_above_ten() {
+        assert_eq!(Level::new(11), Err(DomainError::InvalidLevel(11)));
     }
 
     #[test]
-    fn intensity_cannot_be_deserialised_out_of_range() {
-        assert!(serde_json::from_str::<Intensity>("11").is_err());
-        assert_eq!(serde_json::from_str::<Intensity>("7").unwrap().get(), 7);
-        assert_eq!(
-            serde_json::to_string(&Intensity::new(7).unwrap()).unwrap(),
-            "7"
-        );
+    fn level_cannot_be_deserialised_out_of_range() {
+        assert!(serde_json::from_str::<Level>("11").is_err());
+        assert_eq!(serde_json::from_str::<Level>("0").unwrap(), Level::CLEAN);
+        assert_eq!(serde_json::to_string(&Level::new(7).unwrap()).unwrap(), "7");
     }
 
     #[test]
diff --git a/crates/domain/src/verify.rs b/crates/domain/src/verify.rs
index de6452f..0f501b1 100644
--- a/crates/domain/src/verify.rs
+++ b/crates/domain/src/verify.rs
@@ -6,7 +6,7 @@ use std::collections::{BTreeMap, BTreeSet};
 use serde_json::Value;
 
 use crate::audit::{AuditEntity, ChainError, ChainedEntry, GENESIS_HASH, verify_chain};
-use crate::records::Audited;
+use crate::records::{Audited, SmellReport};
 use crate::repos::{SnapshotRepo, StoreResult};
 
 /// The audit log and every audited record, read at one moment.
@@ -96,7 +96,7 @@ pub fn verify_snapshots(entries: &[ChainedEntry], current: &[Snapshot]) -> Vec<S
                 entity: snapshot.entity,
                 entity_id: snapshot.entity_id,
             }),
-            Some(entry) if entry.record.after.as_ref() != Some(&snapshot.value) => {
+            Some(entry) if !matches_audited(entry, &snapshot.value) => {
                 problems.push(SnapshotProblem::Differs {
                     entity: snapshot.entity,
                     entity_id: snapshot.entity_id,
@@ -118,6 +118,18 @@ pub fn verify_snapshots(entries: &[ChainedEntry], current: &[Snapshot]) -> Vec<S
     problems
 }
 
+/// Whether `current` equals the entry's `after` snapshot. Report snapshots audited before rooms
+/// had their own levels are compared in today's shape (decision #104).
+fn matches_audited(entry: &ChainedEntry, current: &Value) -> bool {
+    match &entry.record.after {
+        Some(after) if entry.record.entity == AuditEntity::SmellReport => {
+            *SmellReport::upgrade_snapshot(after) == *current
+        }
+        Some(after) => after == current,
+        None => false,
+    }
+}
+
 /// Runs both checks against a store.
 pub async fn verify_store<S: SnapshotRepo>(store: &S) -> StoreResult<AuditReport> {
     let AuditState {
@@ -199,9 +211,13 @@ mod tests {
             AuditEntity::SmellReport,
             7,
             AuditAction::Create,
-            json!({ "intensity": 6 }),
+            json!({ "rooms": [{ "room": 1, "level": 6 }] }),
         )]);
-        let current = [snap(AuditEntity::SmellReport, 7, json!({ "intensity": 9 }))];
+        let current = [snap(
+            AuditEntity::SmellReport,
+            7,
+            json!({ "rooms": [{ "room": 1, "level": 9 }] }),
+        )];
         assert_eq!(
             verify_snapshots(&entries, &current),
             [SnapshotProblem::Differs {
@@ -212,6 +228,23 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_report_audited_before_room_levels_still_matches_its_converted_record() {
+        let entries = chain(&[(
+            AuditEntity::SmellReport,
+            7,
+            AuditAction::Create,
+            json!({ "intensity": 6, "rooms": [1, 3] }),
+        )]);
+        let converted = json!({ "rooms": [{ "room": 1, "level": 6 }, { "room": 3, "level": 6 }] });
+        let current = [snap(AuditEntity::SmellReport, 7, converted)];
+        assert_eq!(verify_snapshots(&entries, &current), []);
+
+        let edited = json!({ "rooms": [{ "room": 1, "level": 6 }, { "room": 3, "level": 9 }] });
+        let current = [snap(AuditEntity::SmellReport, 7, edited)];
+        assert_eq!(verify_snapshots(&entries, &current).len(), 1);
+    }
+
     #[test]
     fn a_record_without_any_entry_is_unaudited() {
         let current = [snap(AuditEntity::Symptom, 3, json!({}))];
diff --git a/crates/storage-postgres/migrations/0003_room_levels.sql b/crates/storage-postgres/migrations/0003_room_levels.sql
new file mode 100644
index 0000000..b4c9724
--- /dev/null
+++ b/crates/storage-postgres/migrations/0003_room_levels.sql
@@ -0,0 +1,27 @@
+-- Each room in a smell report gets its own level 0–10, and a report's intensity becomes its
+-- highest room level (spec §6). Every existing room takes its report's intensity. A report
+-- without rooms has no room to carry its intensity, so the migration refuses to run while one
+-- exists (decision #103): correct those reports first.
+-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.
+
+DO $$
+BEGIN
+    IF EXISTS (
+        SELECT 1 FROM smell_reports r
+        WHERE NOT EXISTS (SELECT 1 FROM smell_report_rooms l WHERE l.smell_report_id = r.id)
+    ) THEN
+        RAISE EXCEPTION 'every_smell_report_needs_a_room: give each smell report a room before migrating';
+    END IF;
+END
+$$;
+
+ALTER TABLE smell_report_rooms ADD COLUMN level BIGINT;
+
+UPDATE smell_report_rooms l SET level = r.intensity
+FROM smell_reports r WHERE r.id = l.smell_report_id;
+
+ALTER TABLE smell_report_rooms
+    ALTER COLUMN level SET NOT NULL,
+    ADD CONSTRAINT smell_report_rooms_level_check CHECK (level BETWEEN 0 AND 10);
+
+ALTER TABLE smell_reports DROP COLUMN intensity;
diff --git a/crates/storage-postgres/src/reports.rs b/crates/storage-postgres/src/reports.rs
index 0c3fa87..27d6038 100644
--- a/crates/storage-postgres/src/reports.rs
+++ b/crates/storage-postgres/src/reports.rs
@@ -4,9 +4,9 @@ use std::collections::HashMap;
 
 use chrono::{DateTime, Utc};
 use smoke_domain::audit::{AuditAction, AuditEntity};
-use smoke_domain::records::{NewReport, ReportValues, SmellReport};
+use smoke_domain::records::{NewReport, ReportValues, RoomLevel, SmellReport};
 use smoke_domain::repos::{Conflict, ReportRepo, ReportWrites, StoreError, StoreResult};
-use smoke_domain::types::{EventId, Intensity, ReportId, RoomId, SymptomId, UserId};
+use smoke_domain::types::{EventId, Level, ReportId, RoomId, SymptomId, UserId};
 use sqlx::PgConnection;
 
 use crate::convert::{corrupt, instant, micros, opt_instant};
@@ -19,19 +19,21 @@ struct ReportRow {
     user_id: i64,
     observed_at: i64,
     recorded_at: i64,
-    intensity: i64,
     note: Option<String>,
     deleted_at: Option<i64>,
 }
 
 async fn load(conn: &mut PgConnection, row: ReportRow) -> StoreResult<SmellReport> {
-    let rooms = sqlx::query_scalar!(
-        "SELECT room_id FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
+    let rooms = sqlx::query!(
+        "SELECT room_id, level FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
         row.id,
     )
     .fetch_all(&mut *conn)
     .await
-    .db()?;
+    .db()?
+    .into_iter()
+    .map(|l| (l.room_id, l.level))
+    .collect();
     let symptoms = sqlx::query_scalar!(
         "SELECT symptom_id FROM smell_report_symptoms WHERE smell_report_id = $1 ORDER BY symptom_id",
         row.id,
@@ -42,14 +44,24 @@ async fn load(conn: &mut PgConnection, row: ReportRow) -> StoreResult<SmellRepor
     assemble(row, rooms, symptoms)
 }
 
-/// A report from its row and the ids in its link tables.
-fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<SmellReport> {
-    let intensity = u8::try_from(row.intensity)
-        .map_err(corrupt)
-        .and_then(|value| Intensity::new(value).map_err(corrupt))?;
+/// A report from its row, its `(room id, level)` pairs and its symptom ids.
+fn assemble(
+    row: ReportRow,
+    rooms: Vec<(i64, i64)>,
+    symptoms: Vec<i64>,
+) -> StoreResult<SmellReport> {
+    let rooms = rooms
+        .into_iter()
+        .map(|(room, level)| {
+            let level = u8::try_from(level).map_err(corrupt)?;
+            Ok(RoomLevel {
+                room: RoomId(room),
+                level: Level::new(level).map_err(corrupt)?,
+            })
+        })
+        .collect::<StoreResult<_>>()?;
     let values = ReportValues::new(
-        intensity,
-        rooms.into_iter().map(RoomId).collect(),
+        rooms,
         symptoms.into_iter().map(SymptomId).collect(),
         row.note.as_deref(),
     )
@@ -68,9 +80,9 @@ fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<
     })
 }
 
-/// Groups `(report id, linked id)` pairs by report.
-fn group(links: impl IntoIterator<Item = (i64, i64)>) -> HashMap<i64, Vec<i64>> {
-    let mut grouped: HashMap<i64, Vec<i64>> = HashMap::new();
+/// Groups `(report id, link)` pairs by report.
+fn group<T>(links: impl IntoIterator<Item = (i64, T)>) -> HashMap<i64, Vec<T>> {
+    let mut grouped: HashMap<i64, Vec<T>> = HashMap::new();
     for (report, linked) in links {
         grouped.entry(report).or_default().push(linked);
     }
@@ -88,7 +100,7 @@ async fn load_all(conn: &mut PgConnection, rows: Vec<ReportRow>) -> StoreResult<
 async fn fetch(conn: &mut PgConnection, id: ReportId) -> StoreResult<Option<SmellReport>> {
     let row = sqlx::query_as!(
         ReportRow,
-        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+        "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
          FROM smell_reports WHERE id = $1",
         id.0,
     )
@@ -121,9 +133,10 @@ async fn require_live(conn: &mut PgConnection, id: ReportId) -> StoreResult<Smel
 async fn insert_links(conn: &mut PgConnection, id: i64, values: &ReportValues) -> StoreResult<()> {
     for room in values.rooms() {
         sqlx::query!(
-            "INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES ($1, $2)",
+            "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) VALUES ($1, $2, $3)",
             id,
-            room.0,
+            room.room.0,
+            i64::from(room.level.get()),
         )
         .execute(&mut *conn)
         .await
@@ -179,7 +192,7 @@ impl ReportRepo for PostgresStore {
         let mut conn = self.read().await?;
         let rows = sqlx::query_as!(
             ReportRow,
-            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+            "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
              FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL \
              ORDER BY observed_at, id",
             event.0,
@@ -201,7 +214,7 @@ impl ReportRepo for PostgresStore {
         let mut conn = self.read().await?;
         let rows = sqlx::query_as!(
             ReportRow,
-            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+            "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
              FROM smell_reports WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
              ORDER BY observed_at, id",
             from,
@@ -211,7 +224,7 @@ impl ReportRepo for PostgresStore {
         .await
         .db()?;
         let rooms = sqlx::query!(
-            "SELECT l.smell_report_id, l.room_id FROM smell_report_rooms l \
+            "SELECT l.smell_report_id, l.room_id, l.level FROM smell_report_rooms l \
              JOIN smell_reports r ON r.id = l.smell_report_id \
              WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
              ORDER BY l.smell_report_id, l.room_id",
@@ -232,7 +245,11 @@ impl ReportRepo for PostgresStore {
         .fetch_all(&mut *conn)
         .await
         .db()?;
-        let mut rooms = group(rooms.into_iter().map(|l| (l.smell_report_id, l.room_id)));
+        let mut rooms = group(
+            rooms
+                .into_iter()
+                .map(|l| (l.smell_report_id, (l.room_id, l.level))),
+        );
         let mut symptoms = group(
             symptoms
                 .into_iter()
@@ -259,13 +276,12 @@ impl ReportWrites for PostgresTx {
     async fn create_report(&mut self, new: NewReport) -> StoreResult<SmellReport> {
         let now = micros(self.now());
         let id = sqlx::query_scalar!(
-            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, intensity, note) \
-             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
+            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, note) \
+             VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
             new.event_id.0,
             new.user_id.0,
             micros(new.observed_at),
             now,
-            i64::from(new.values.intensity().get()),
             new.values.note(),
         )
         .fetch_one(self.conn())
@@ -285,9 +301,8 @@ impl ReportWrites for PostgresTx {
     ) -> StoreResult<SmellReport> {
         let before = require_live(self.conn(), id).await?;
         sqlx::query!(
-            "UPDATE smell_reports SET observed_at = $1, intensity = $2, note = $3 WHERE id = $4",
+            "UPDATE smell_reports SET observed_at = $1, note = $2 WHERE id = $3",
             micros(observed_at),
-            i64::from(values.intensity().get()),
             values.note(),
             id.0,
         )
@@ -333,7 +348,7 @@ impl ReportWrites for PostgresTx {
 pub(crate) async fn fetch_all(conn: &mut PgConnection) -> StoreResult<Vec<SmellReport>> {
     let rows = sqlx::query_as!(
         ReportRow,
-        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+        "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
          FROM smell_reports ORDER BY id"
     )
     .fetch_all(&mut *conn)
diff --git a/crates/storage-sqlite/migrations/0003_room_levels.sql b/crates/storage-sqlite/migrations/0003_room_levels.sql
new file mode 100644
index 0000000..1357c89
--- /dev/null
+++ b/crates/storage-sqlite/migrations/0003_room_levels.sql
@@ -0,0 +1,30 @@
+-- Each room in a smell report gets its own level 0–10, and a report's intensity becomes its
+-- highest room level (spec §6). Every existing room takes its report's intensity. A report
+-- without rooms has no room to carry its intensity, so the migration refuses to run while one
+-- exists (decision #103): correct those reports first.
+
+CREATE TEMP TABLE migration_0003_check (
+    reports_without_rooms INTEGER NOT NULL
+        CONSTRAINT every_smell_report_needs_a_room CHECK (reports_without_rooms = 0)
+);
+INSERT INTO migration_0003_check
+SELECT count(*) FROM smell_reports r
+WHERE NOT EXISTS (SELECT 1 FROM smell_report_rooms l WHERE l.smell_report_id = r.id);
+DROP TABLE migration_0003_check;
+
+ALTER TABLE smell_report_rooms RENAME TO smell_report_rooms_0002;
+
+CREATE TABLE smell_report_rooms (
+    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
+    room_id         INTEGER NOT NULL REFERENCES rooms (id),
+    level           INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
+    PRIMARY KEY (smell_report_id, room_id)
+);
+
+INSERT INTO smell_report_rooms (smell_report_id, room_id, level)
+SELECT l.smell_report_id, l.room_id, r.intensity
+FROM smell_report_rooms_0002 l JOIN smell_reports r ON r.id = l.smell_report_id;
+
+DROP TABLE smell_report_rooms_0002;
+
+ALTER TABLE smell_reports DROP COLUMN intensity;
diff --git a/crates/storage-sqlite/src/reports.rs b/crates/storage-sqlite/src/reports.rs
index 5674fe0..74bf0e9 100644
--- a/crates/storage-sqlite/src/reports.rs
+++ b/crates/storage-sqlite/src/reports.rs
@@ -4,9 +4,9 @@ use std::collections::HashMap;
 
 use chrono::{DateTime, Utc};
 use smoke_domain::audit::{AuditAction, AuditEntity};
-use smoke_domain::records::{NewReport, ReportValues, SmellReport};
+use smoke_domain::records::{NewReport, ReportValues, RoomLevel, SmellReport};
 use smoke_domain::repos::{Conflict, ReportRepo, ReportWrites, StoreError, StoreResult};
-use smoke_domain::types::{EventId, Intensity, ReportId, RoomId, SymptomId, UserId};
+use smoke_domain::types::{EventId, Level, ReportId, RoomId, SymptomId, UserId};
 use sqlx::SqliteConnection;
 
 use crate::convert::{corrupt, instant, micros, opt_instant};
@@ -19,19 +19,21 @@ struct ReportRow {
     user_id: i64,
     observed_at: i64,
     recorded_at: i64,
-    intensity: i64,
     note: Option<String>,
     deleted_at: Option<i64>,
 }
 
 async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellReport> {
-    let rooms = sqlx::query_scalar!(
-        "SELECT room_id FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
+    let rooms = sqlx::query!(
+        "SELECT room_id, level FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
         row.id,
     )
     .fetch_all(&mut *conn)
     .await
-    .db()?;
+    .db()?
+    .into_iter()
+    .map(|l| (l.room_id, l.level))
+    .collect();
     let symptoms = sqlx::query_scalar!(
         "SELECT symptom_id FROM smell_report_symptoms WHERE smell_report_id = $1 ORDER BY symptom_id",
         row.id,
@@ -42,14 +44,24 @@ async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellR
     assemble(row, rooms, symptoms)
 }
 
-/// A report from its row and the ids in its link tables.
-fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<SmellReport> {
-    let intensity = u8::try_from(row.intensity)
-        .map_err(corrupt)
-        .and_then(|value| Intensity::new(value).map_err(corrupt))?;
+/// A report from its row, its `(room id, level)` pairs and its symptom ids.
+fn assemble(
+    row: ReportRow,
+    rooms: Vec<(i64, i64)>,
+    symptoms: Vec<i64>,
+) -> StoreResult<SmellReport> {
+    let rooms = rooms
+        .into_iter()
+        .map(|(room, level)| {
+            let level = u8::try_from(level).map_err(corrupt)?;
+            Ok(RoomLevel {
+                room: RoomId(room),
+                level: Level::new(level).map_err(corrupt)?,
+            })
+        })
+        .collect::<StoreResult<_>>()?;
     let values = ReportValues::new(
-        intensity,
-        rooms.into_iter().map(RoomId).collect(),
+        rooms,
         symptoms.into_iter().map(SymptomId).collect(),
         row.note.as_deref(),
     )
@@ -68,9 +80,9 @@ fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<
     })
 }
 
-/// Groups `(report id, linked id)` pairs by report.
-fn group(links: impl IntoIterator<Item = (i64, i64)>) -> HashMap<i64, Vec<i64>> {
-    let mut grouped: HashMap<i64, Vec<i64>> = HashMap::new();
+/// Groups `(report id, link)` pairs by report.
+fn group<T>(links: impl IntoIterator<Item = (i64, T)>) -> HashMap<i64, Vec<T>> {
+    let mut grouped: HashMap<i64, Vec<T>> = HashMap::new();
     for (report, linked) in links {
         grouped.entry(report).or_default().push(linked);
     }
@@ -91,7 +103,7 @@ async fn load_all(
 async fn fetch(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<Option<SmellReport>> {
     let row = sqlx::query_as!(
         ReportRow,
-        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+        "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
          FROM smell_reports WHERE id = $1",
         id.0,
     )
@@ -128,9 +140,10 @@ async fn insert_links(
 ) -> StoreResult<()> {
     for room in values.rooms() {
         sqlx::query!(
-            "INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES ($1, $2)",
+            "INSERT INTO smell_report_rooms (smell_report_id, room_id, level) VALUES ($1, $2, $3)",
             id,
-            room.0,
+            room.room.0,
+            i64::from(room.level.get()),
         )
         .execute(&mut *conn)
         .await
@@ -186,7 +199,7 @@ impl ReportRepo for SqliteStore {
         let mut conn = self.read().await?;
         let rows = sqlx::query_as!(
             ReportRow,
-            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+            "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
              FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL \
              ORDER BY observed_at, id",
             event.0,
@@ -208,7 +221,7 @@ impl ReportRepo for SqliteStore {
         let mut conn = self.read().await?;
         let rows = sqlx::query_as!(
             ReportRow,
-            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+            "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
              FROM smell_reports WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
              ORDER BY observed_at, id",
             from,
@@ -218,7 +231,7 @@ impl ReportRepo for SqliteStore {
         .await
         .db()?;
         let rooms = sqlx::query!(
-            "SELECT l.smell_report_id, l.room_id FROM smell_report_rooms l \
+            "SELECT l.smell_report_id, l.room_id, l.level FROM smell_report_rooms l \
              JOIN smell_reports r ON r.id = l.smell_report_id \
              WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
              ORDER BY l.smell_report_id, l.room_id",
@@ -239,7 +252,11 @@ impl ReportRepo for SqliteStore {
         .fetch_all(&mut *conn)
         .await
         .db()?;
-        let mut rooms = group(rooms.into_iter().map(|l| (l.smell_report_id, l.room_id)));
+        let mut rooms = group(
+            rooms
+                .into_iter()
+                .map(|l| (l.smell_report_id, (l.room_id, l.level))),
+        );
         let mut symptoms = group(
             symptoms
                 .into_iter()
@@ -266,13 +283,12 @@ impl ReportWrites for SqliteTx {
     async fn create_report(&mut self, new: NewReport) -> StoreResult<SmellReport> {
         let now = micros(self.now());
         let id = sqlx::query_scalar!(
-            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, intensity, note) \
-             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
+            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, note) \
+             VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
             new.event_id.0,
             new.user_id.0,
             micros(new.observed_at),
             now,
-            i64::from(new.values.intensity().get()),
             new.values.note(),
         )
         .fetch_one(self.conn())
@@ -292,9 +308,8 @@ impl ReportWrites for SqliteTx {
     ) -> StoreResult<SmellReport> {
         let before = require_live(self.conn(), id).await?;
         sqlx::query!(
-            "UPDATE smell_reports SET observed_at = $1, intensity = $2, note = $3 WHERE id = $4",
+            "UPDATE smell_reports SET observed_at = $1, note = $2 WHERE id = $3",
             micros(observed_at),
-            i64::from(values.intensity().get()),
             values.note(),
             id.0,
         )
@@ -340,7 +355,7 @@ impl ReportWrites for SqliteTx {
 pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<SmellReport>> {
     let rows = sqlx::query_as!(
         ReportRow,
-        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+        "SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at \
          FROM smell_reports ORDER BY id"
     )
     .fetch_all(&mut *conn)
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 831449b..2650a62 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -18,6 +18,7 @@ event-open-since = Smell event in progress since { $time }.
 log-intensity = Intensity
 log-how-to-rate = How to rate
 log-rooms = Rooms
+log-level-in = Level in { $room }
 log-inside = Inside
 log-common = Common areas
 log-symptoms = Symptoms
@@ -33,7 +34,8 @@ log-read-only = You can look at the data but not log.
 
 log-saved-report = Report saved.
 log-saved-pm25 = PM2.5 reading saved.
-log-error-intensity = Pick an intensity from 1 to 10.
+log-error-rooms = Pick at least one room.
+log-error-level = Pick a level from 0 to 10 for each selected room.
 log-error-pm25 = PM2.5 must be a number from 0 to 1000.
 log-error-pm25-missing = Enter a PM2.5 value to log it on its own.
 log-error-time = Enter a valid date and time.
@@ -81,6 +83,8 @@ rating-description = What it's like
 rating-tip = Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.
 rating-full-guide = Full guide →
 
+intensity-0-name = Clean
+intensity-0-desc = You checked: no smoke smell at all.
 intensity-1-name = Barely there
 intensity-1-desc = Only if you focus. Unsure it's smoke.
 intensity-2-name = Faint
@@ -138,7 +142,6 @@ history-update = corrected
 history-move = moved
 history-delete = deleted
 history-time = time { $change }
-history-intensity = intensity { $change }
 history-added = added { $name }
 history-removed = removed { $name }
 history-note = note changed
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index d19ce6a..7c1bb7d 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -18,6 +18,7 @@ event-open-since = Evento di fumo in corso dalle { $time }.
 log-intensity = Intensità
 log-how-to-rate = Come valutare
 log-rooms = Stanze
+log-level-in = Livello in { $room }
 log-inside = In casa
 log-common = Parti comuni
 log-symptoms = Sintomi
@@ -33,7 +34,8 @@ log-read-only = Puoi consultare i dati ma non registrare.
 
 log-saved-report = Segnalazione salvata.
 log-saved-pm25 = Lettura PM2.5 salvata.
-log-error-intensity = Scegli un'intensità da 1 a 10.
+log-error-rooms = Scegli almeno una stanza.
+log-error-level = Scegli un livello da 0 a 10 per ogni stanza selezionata.
 log-error-pm25 = Il PM2.5 deve essere un numero da 0 a 1000.
 log-error-pm25-missing = Inserisci un valore di PM2.5 per registrarlo da solo.
 log-error-time = Inserisci una data e un'ora valide.
@@ -81,6 +83,8 @@ rating-description = Come si presenta
 rating-tip = Valuta quello che senti adesso, non quanto sei infastidito. Tra due numeri, scegli il più basso.
 rating-full-guide = Guida completa →
 
+intensity-0-name = Pulito
+intensity-0-desc = Hai controllato: nessun odore di fumo.
 intensity-1-name = Appena percepibile
 intensity-1-desc = Solo se ci fai caso. Non sei sicuro che sia fumo.
 intensity-2-name = Debole
@@ -138,7 +142,6 @@ history-update = corretta
 history-move = spostata
 history-delete = eliminata
 history-time = ora { $change }
-history-intensity = intensità { $change }
 history-added = aggiunto { $name }
 history-removed = tolto { $name }
 history-note = nota modificata
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index d4864d2..00ced4f 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -18,6 +18,7 @@ event-open-since = { $time } saatinden beri süren bir koku olayı var.
 log-intensity = Yoğunluk
 log-how-to-rate = Nasıl puanlanır
 log-rooms = Odalar
+log-level-in = { $room } seviyesi
 log-inside = Ev içi
 log-common = Ortak alanlar
 log-symptoms = Belirtiler
@@ -33,7 +34,8 @@ log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.
 
 log-saved-report = Bildirim kaydedildi.
 log-saved-pm25 = PM2.5 ölçümü kaydedildi.
-log-error-intensity = 1 ile 10 arasında bir yoğunluk seç.
+log-error-rooms = En az bir oda seç.
+log-error-level = Seçtiğin her oda için 0 ile 10 arasında bir seviye seç.
 log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
 log-error-pm25-missing = Tek başına kaydetmek için bir PM2.5 değeri gir.
 log-error-time = Geçerli bir tarih ve saat gir.
@@ -81,6 +83,8 @@ rating-description = Nasıl bir şey
 rating-tip = Şu an aldığın kokuyu puanla, ne kadar rahatsız olduğunu değil. İki sayı arasında kalırsan küçük olanı seç.
 rating-full-guide = Tüm rehber →
 
+intensity-0-name = Temiz
+intensity-0-desc = Kontrol ettin: hiç duman kokusu yok.
 intensity-1-name = Zar zor
 intensity-1-desc = Ancak dikkat edersen. Duman olduğundan emin değilsin.
 intensity-2-name = Hafif
@@ -138,7 +142,6 @@ history-update = düzeltildi
 history-move = taşındı
 history-delete = silindi
 history-time = saat { $change }
-history-intensity = yoğunluk { $change }
 history-added = eklendi: { $name }
 history-removed = çıkarıldı: { $name }
 history-note = not değişti
diff --git a/crates/web/src/forms.rs b/crates/web/src/forms.rs
index 1ff9a22..9d41818 100644
--- a/crates/web/src/forms.rs
+++ b/crates/web/src/forms.rs
@@ -38,6 +38,17 @@ impl Pairs {
             .collect()
     }
 
+    /// What follows `prefix` in each field name that starts with it, e.g. the room ids of
+    /// `level-<id>` fields.
+    pub(crate) fn names_with_prefix<'a>(
+        &'a self,
+        prefix: &'a str,
+    ) -> impl Iterator<Item = &'a str> + 'a {
+        self.0
+            .iter()
+            .filter_map(move |(key, _)| key.strip_prefix(prefix))
+    }
+
     /// All pairs except those named in `skip`: what a follow-up form re-posts as hidden fields.
     pub(crate) fn without(&self, skip: &[&str]) -> Vec<(String, String)> {
         self.0
diff --git a/crates/web/src/labels.rs b/crates/web/src/labels.rs
index 6bdac9f..9c629dd 100644
--- a/crates/web/src/labels.rs
+++ b/crates/web/src/labels.rs
@@ -1,8 +1,9 @@
 //! Names shown for stored values: starter rooms and symptoms through their translation key,
 //! custom ones as typed, and the translation keys of enum values.
 
-use smoke_domain::records::{Room, Symptom};
+use smoke_domain::records::{Room, RoomLevel, Symptom};
 use smoke_domain::types::CloseReason;
+use smoke_domain::types::RoomId;
 
 use crate::i18n::Tr;
 
@@ -13,6 +14,20 @@ pub(crate) fn room(tr: Tr, room: &Room) -> String {
     }
 }
 
+/// The name of the room with this id among `rooms`; empty if there is none.
+pub(crate) fn room_by_id(tr: Tr, rooms: &[Room], id: RoomId) -> String {
+    rooms
+        .iter()
+        .find(|r| r.id == id)
+        .map(|r| room(tr, r))
+        .unwrap_or_default()
+}
+
+/// A room of a report with its level: "Living room 6".
+pub(crate) fn room_level(tr: Tr, rooms: &[Room], at: RoomLevel) -> String {
+    format!("{} {}", room_by_id(tr, rooms, at.room), at.level.get())
+}
+
 pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
     match &symptom.i18n_key {
         Some(key) => tr.t(key),
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index 875fd97..5958c16 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -12,10 +12,10 @@ use chrono_tz::Tz;
 use smoke_domain::events::{LogPrompt, can_join_backdated, classify_observed, log_prompt};
 use smoke_domain::logbook::{EventChoice, LogError, SmellEntry, log_reading, log_smell};
 use smoke_domain::permissions::{Action, can};
-use smoke_domain::records::{Event, ReportValues, Room, Symptom, User};
+use smoke_domain::records::{Event, ReportValues, Room, RoomLevel, Symptom, User};
 use smoke_domain::repos::{Commit, Store};
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{EventId, Intensity, Pm25, Role, RoomId, SymptomId};
+use smoke_domain::types::{EventId, Level, Pm25, Role, RoomId, SymptomId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
@@ -28,12 +28,13 @@ use crate::time::{self, TimeInputError};
 
 /// The translation keys [`read`] can return, for the translation tests.
 #[cfg(test)]
-pub(crate) const FORM_ERRORS: [&str; 7] = [
+pub(crate) const FORM_ERRORS: [&str; 8] = [
     "log-error-time",
     "log-error-time-skipped",
     "log-error-pm25",
     "log-error-pm25-missing",
-    "log-error-intensity",
+    "log-error-rooms",
+    "log-error-level",
     "log-error-form",
     "log-error-note",
 ];
@@ -58,23 +59,27 @@ pub(crate) fn read_time(
         .transpose()
 }
 
-/// Reads intensity, rooms, symptoms and note. Only the offered rooms and symptoms are accepted.
+/// The level picked for `room` (`level-<id>`), if any.
+pub(crate) fn level_field(form: &Pairs, room: RoomId) -> Option<u8> {
+    form.get(&format!("level-{room}"))
+        .and_then(|value| value.parse().ok())
+}
+
+/// Reads the rooms with their levels, the symptoms and the note. Only the offered rooms and
+/// symptoms are accepted; every selected room needs a level.
 pub(crate) fn read_values(
     form: &Pairs,
     rooms: &[RoomId],
     symptoms: &[SymptomId],
 ) -> Result<ReportValues, &'static str> {
-    let intensity = form
-        .get("intensity")
-        .and_then(|value| value.parse().ok())
-        .and_then(|value| Intensity::new(value).ok())
-        .ok_or("log-error-intensity")?;
-    let room_ids: Vec<RoomId> = form
+    let mut room_ids: Vec<RoomId> = form
         .ids("room")
         .ok_or("log-error-form")?
         .into_iter()
         .map(RoomId)
         .collect();
+    room_ids.sort_unstable();
+    room_ids.dedup();
     let symptom_ids: Vec<SymptomId> = form
         .ids("symptom")
         .ok_or("log-error-form")?
@@ -86,8 +91,20 @@ pub(crate) fn read_values(
     {
         return Err("log-error-form");
     }
-    ReportValues::new(intensity, room_ids, symptom_ids, form.get("note"))
-        .map_err(|_| "log-error-note")
+    if room_ids.is_empty() {
+        return Err("log-error-rooms");
+    }
+    let rooms = room_ids
+        .into_iter()
+        .map(|room| {
+            let level = level_field(form, room).and_then(|value| Level::new(value).ok());
+            Ok(RoomLevel {
+                room,
+                level: level.ok_or("log-error-level")?,
+            })
+        })
+        .collect::<Result<_, &'static str>>()?;
+    ReportValues::new(rooms, symptom_ids, form.get("note")).map_err(|_| "log-error-note")
 }
 
 /// Reads and validates the Log form. `Err` is the translation key of what to fix. Only rooms
@@ -414,25 +431,48 @@ mod tests {
     }
 
     #[test]
-    fn a_smell_needs_an_intensity_and_offered_rooms_only() {
-        let Ok(Entry::Smell(entry)) = read_form(&[("intensity", "6"), ("room", "1")]) else {
+    fn a_smell_needs_offered_rooms_each_with_a_level() {
+        let Ok(Entry::Smell(entry)) = read_form(&[("room", "1"), ("level-1", "6")]) else {
             panic!("a valid smell");
         };
         assert_eq!(entry.observed_at, now());
-        assert_eq!(entry.values.rooms(), [RoomId(1)]);
+        assert_eq!(
+            entry.values.rooms(),
+            [RoomLevel {
+                room: RoomId(1),
+                level: Level::new(6).unwrap(),
+            }]
+        );
         assert_eq!(entry.pm25, None);
+        let Ok(Entry::Smell(clean)) =
+            read_form(&[("room", "1"), ("level-1", "0"), ("level-3", "9")])
+        else {
+            panic!("a valid clean report");
+        };
+        assert!(
+            clean.values.intensity().is_clean(),
+            "unselected rooms are ignored"
+        );
         for (pairs, key) in [
-            (&[("room", "1")][..], "log-error-intensity"),
-            (&[("intensity", "11")][..], "log-error-intensity"),
-            (&[("intensity", "6"), ("room", "2")][..], "log-error-form"),
-            (&[("intensity", "6"), ("room", "x")][..], "log-error-form"),
+            (&[("level-1", "6")][..], "log-error-rooms"),
+            (&[("room", "1")][..], "log-error-level"),
+            (&[("room", "1"), ("level-1", "11")][..], "log-error-level"),
+            (&[("room", "2"), ("level-2", "6")][..], "log-error-form"),
+            (&[("room", "x")][..], "log-error-form"),
             (
-                &[("intensity", "6"), ("symptom", "1")][..],
+                &[("room", "1"), ("level-1", "6"), ("symptom", "1")][..],
                 "log-error-form",
             ),
-            (&[("intensity", "6"), ("pm25", "-1")][..], "log-error-pm25"),
             (
-                &[("intensity", "6"), ("observed_at", "2026-01-15T22:00")][..],
+                &[("room", "1"), ("level-1", "6"), ("pm25", "-1")][..],
+                "log-error-pm25",
+            ),
+            (
+                &[
+                    ("room", "1"),
+                    ("level-1", "6"),
+                    ("observed_at", "2026-01-15T22:00"),
+                ][..],
                 "refused-future",
             ),
         ] {
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index c4d44cf..4f3d1c4 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -271,10 +271,11 @@ async fn detail<S: Store>(
             "level",
             &report.values.intensity().get().to_string(),
         )];
-        let room_names: Vec<String> = rooms
+        let room_names: Vec<String> = report
+            .values
+            .rooms()
             .iter()
-            .filter(|room| report.values.rooms().contains(&room.id))
-            .map(|room| labels::room(tr, room))
+            .map(|&at| labels::room_level(tr, &rooms, at))
             .collect();
         let symptom_names: Vec<String> = symptoms
             .iter()
diff --git a/crates/web/src/routes/guide.rs b/crates/web/src/routes/guide.rs
index f0f5d98..0d5ec7a 100644
--- a/crates/web/src/routes/guide.rs
+++ b/crates/web/src/routes/guide.rs
@@ -1,25 +1,25 @@
-//! The intensity guide (spec §5): a compact version in a popup on the Log screen, and this full
+//! The rating guide (spec §5): a compact version in a popup on the Log screen, and this full
 //! page. The texts live in the translation files so they are easy to edit.
 
 use askama::Template;
 use axum::http::StatusCode;
 use axum::response::Response;
-use smoke_domain::types::Intensity;
+use smoke_domain::types::Level;
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::i18n::Tr;
 
 /// One row of the guide.
-pub(crate) struct Level {
+pub(crate) struct GuideRow {
     pub(crate) value: u8,
     pub(crate) name: String,
     pub(crate) description: String,
 }
 
-pub(crate) fn levels(tr: Tr) -> Vec<Level> {
-    (Intensity::MIN..=Intensity::MAX)
-        .map(|value| Level {
+pub(crate) fn levels(tr: Tr) -> Vec<GuideRow> {
+    (Level::MIN..=Level::MAX)
+        .map(|value| GuideRow {
             value,
             name: tr.t(&format!("intensity-{value}-name")),
             description: tr.t(&format!("intensity-{value}-desc")),
@@ -30,7 +30,7 @@ pub(crate) fn levels(tr: Tr) -> Vec<Level> {
 /// Every key [`levels`] looks up, for the translation tests.
 #[cfg(test)]
 pub(crate) fn keys() -> Vec<String> {
-    (Intensity::MIN..=Intensity::MAX)
+    (Level::MIN..=Level::MAX)
         .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")])
         .collect()
 }
@@ -40,7 +40,7 @@ pub(crate) fn keys() -> Vec<String> {
 struct GuidePage {
     tr: Tr,
     signed_in: Option<String>,
-    levels: Vec<Level>,
+    levels: Vec<GuideRow>,
 }
 
 pub(crate) async fn rating(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index ee21aa7..0742270 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -12,13 +12,14 @@ use smoke_domain::repos::Store;
 use smoke_domain::starter::BEDROOM_KEY;
 use smoke_domain::summary::{MonthSummary, summarise};
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{Intensity, RoomArea, RoomId, SymptomId};
+use smoke_domain::types::{Level, RoomArea, RoomId, SymptomId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
 use crate::i18n::Tr;
-use crate::routes::guide::{Level, levels};
+use crate::routes::entries::level_field;
+use crate::routes::guide::{GuideRow, levels};
 use crate::routes::{can_open, notices};
 use crate::state::AppState;
 use crate::{labels, time};
@@ -26,8 +27,9 @@ use crate::{labels, time};
 /// What the form holds: blank, pre-filled, or as typed before a validation error.
 #[derive(Debug, Clone, Default, PartialEq)]
 pub(crate) struct Draft {
-    pub(crate) intensity: Option<u8>,
     pub(crate) rooms: Vec<RoomId>,
+    /// The level picked for each room, selected or not.
+    pub(crate) levels: Vec<(RoomId, u8)>,
     pub(crate) symptoms: Vec<SymptomId>,
     pub(crate) pm25: String,
     pub(crate) observed_at: String,
@@ -39,13 +41,17 @@ impl Draft {
     pub(crate) fn from_form(form: &Pairs) -> Self {
         let text = |name| form.get(name).unwrap_or_default().to_owned();
         Self {
-            intensity: form.get("intensity").and_then(|v| v.parse().ok()),
             rooms: form
                 .ids("room")
                 .unwrap_or_default()
                 .into_iter()
                 .map(RoomId)
                 .collect(),
+            levels: form
+                .names_with_prefix("level-")
+                .filter_map(|id| id.parse().ok().map(RoomId))
+                .filter_map(|room| level_field(form, room).map(|level| (room, level)))
+                .collect(),
             symptoms: form
                 .ids("symptom")
                 .unwrap_or_default()
@@ -66,12 +72,17 @@ pub(crate) struct Choice {
     pub(crate) checked: bool,
 }
 
-/// The intensity, room and symptom fields shared by the Log form and the correction form
+/// A room chip and the row of levels it shows while selected.
+pub(crate) struct RoomChoice {
+    pub(crate) room: Choice,
+    pub(crate) levels: Vec<Choice>,
+}
+
+/// The room, level and symptom fields shared by the Log form and the correction form
 /// (`templates/report_fields.html`).
 pub(crate) struct Fields {
-    pub(crate) intensities: Vec<Choice>,
-    pub(crate) inside: Vec<Choice>,
-    pub(crate) common: Vec<Choice>,
+    pub(crate) inside: Vec<RoomChoice>,
+    pub(crate) common: Vec<RoomChoice>,
     pub(crate) symptoms: Vec<Choice>,
 }
 
@@ -88,21 +99,30 @@ pub(crate) fn fields(
         rooms
             .iter()
             .filter(|r| r.area == area && (r.hidden_at.is_none() || keep.0.contains(&r.id)))
-            .map(|r| Choice {
-                value: r.id.0,
-                label: labels::room(tr, r),
-                checked: draft.rooms.contains(&r.id),
+            .map(|r| {
+                let picked = draft
+                    .levels
+                    .iter()
+                    .find(|(room, _)| *room == r.id)
+                    .map(|&(_, level)| level);
+                RoomChoice {
+                    room: Choice {
+                        value: r.id.0,
+                        label: labels::room(tr, r),
+                        checked: draft.rooms.contains(&r.id),
+                    },
+                    levels: (Level::MIN..=Level::MAX)
+                        .map(|value| Choice {
+                            value: value.into(),
+                            label: value.to_string(),
+                            checked: picked == Some(value),
+                        })
+                        .collect(),
+                }
             })
             .collect()
     };
     Fields {
-        intensities: (Intensity::MIN..=Intensity::MAX)
-            .map(|value| Choice {
-                value: value.into(),
-                label: value.to_string(),
-                checked: draft.intensity == Some(value),
-            })
-            .collect(),
         inside: room_choices(RoomArea::Inside),
         common: room_choices(RoomArea::Common),
         symptoms: symptoms
@@ -171,7 +191,7 @@ pub(crate) struct LogPage {
     /// A translation key confirming what was just saved.
     saved: Option<&'static str>,
     summary: MonthSummary,
-    levels: Vec<Level>,
+    levels: Vec<GuideRow>,
     /// `None` for people who may not log (viewers, expired guests).
     form: Option<FormView>,
 }
@@ -251,7 +271,7 @@ pub(crate) async fn log_page<S: Store>(
             None => {
                 let mut blank = Draft::default();
                 if let Some(last) = open_reports.iter().rev().find(|r| r.user_id == user.id) {
-                    blank.rooms = last.values.rooms().to_vec();
+                    blank.rooms = last.values.room_ids();
                 }
                 blank
             }
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index cc9c657..5d2cd7a 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -9,17 +9,17 @@ use smoke_domain::audit::{AuditAction, AuditEntity};
 use smoke_domain::history::{Change, report_history, was_corrected};
 use smoke_domain::logbook::{MoveTarget, correct_report, move_report};
 use smoke_domain::permissions::{Action, can, can_see};
-use smoke_domain::records::{Room, SmellReport, Symptom, User};
+use smoke_domain::records::{SmellReport, Symptom, User};
 use smoke_domain::repos::{Commit, Store};
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{EventId, ReportId, RoomId, SymptomId, UserId};
+use smoke_domain::types::{EventId, ReportId, SymptomId, UserId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
 use crate::i18n::Tr;
 use crate::routes::entries::{joinable_events, read_time, read_values};
-use crate::routes::guide::{Level, levels};
+use crate::routes::guide::{GuideRow, levels};
 use crate::routes::log::{Draft, Fields, fields, offered};
 use crate::routes::refusal;
 use crate::state::AppState;
@@ -41,14 +41,6 @@ async fn visible_report<S: Store>(
         .ok_or(AppError::NotFound)
 }
 
-fn room_names(tr: Tr, rooms: &[Room], ids: &[RoomId]) -> Vec<String> {
-    rooms
-        .iter()
-        .filter(|r| ids.contains(&r.id))
-        .map(|r| labels::room(tr, r))
-        .collect()
-}
-
 fn symptom_names(tr: Tr, symptoms: &[Symptom], ids: &[SymptomId]) -> Vec<String> {
     symptoms
         .iter()
@@ -177,18 +169,21 @@ async fn report_page<S: Store>(
                             time::date_time(tr.0, *to, tz)
                         ),
                     ),
-                    Change::Intensity { from, to } => {
-                        tr.t1("history-intensity", "change", &format!("{from} → {to}"))
-                    }
-                    Change::RoomAdded(room) => tr.t1(
+                    Change::LevelChanged { room, from, to } => format!(
+                        "{} {} → {}",
+                        labels::room_by_id(tr, &rooms, *room),
+                        from.get(),
+                        to.get()
+                    ),
+                    Change::RoomAdded(at) => tr.t1(
                         "history-added",
                         "name",
-                        &room_names(tr, &rooms, &[*room]).concat(),
+                        &labels::room_level(tr, &rooms, *at),
                     ),
                     Change::RoomRemoved(room) => tr.t1(
                         "history-removed",
                         "name",
-                        &room_names(tr, &rooms, &[*room]).concat(),
+                        &labels::room_by_id(tr, &rooms, *room),
                     ),
                     Change::SymptomAdded(symptom) => tr.t1(
                         "history-added",
@@ -253,7 +248,14 @@ async fn report_page<S: Store>(
             "{intensity} · {}",
             tr.t(&format!("intensity-{intensity}-name"))
         ),
-        rooms: join(room_names(tr, &rooms, report.values.rooms())),
+        rooms: join(
+            report
+                .values
+                .rooms()
+                .iter()
+                .map(|&at| labels::room_level(tr, &rooms, at))
+                .collect(),
+        ),
         symptoms: join(symptom_names(tr, &symptoms, report.values.symptoms())),
         note: report.values.note().map(str::to_owned),
         by: name(Some(report.user_id), &users),
@@ -279,7 +281,7 @@ struct CorrectPage {
     tr: Tr,
     signed_in: Option<String>,
     id: i64,
-    levels: Vec<Level>,
+    levels: Vec<GuideRow>,
     form: CorrectForm,
 }
 
@@ -316,7 +318,8 @@ async fn correct_page<S: Store>(
 ) -> Result<CorrectPage, AppError> {
     let rooms = state.store.rooms().await?;
     let symptoms = state.store.symptoms().await?;
-    let keep = (report.values.rooms(), report.values.symptoms());
+    let room_ids = report.values.room_ids();
+    let keep = (room_ids.as_slice(), report.values.symptoms());
     Ok(CorrectPage {
         tr,
         signed_in: Some(user.username.to_string()),
@@ -340,8 +343,13 @@ pub(crate) async fn correct_form<S: Store + Clone + 'static>(
 ) -> Result<Response, AppError> {
     let report = editable_report(&state, &user, ReportId(id)).await?;
     let draft = Draft {
-        intensity: Some(report.values.intensity().get()),
-        rooms: report.values.rooms().to_vec(),
+        rooms: report.values.room_ids(),
+        levels: report
+            .values
+            .rooms()
+            .iter()
+            .map(|r| (r.room, r.level.get()))
+            .collect(),
         symptoms: report.values.symptoms().to_vec(),
         pm25: String::new(),
         observed_at: time::input_value(report.observed_at, state.config.schedule.tz()),
@@ -364,7 +372,8 @@ pub(crate) async fn correct<S: Store + Clone + 'static>(
     let report = editable_report(&state, &user, ReportId(id)).await?;
     let rooms = state.store.rooms().await?;
     let symptoms = state.store.symptoms().await?;
-    let keep = (report.values.rooms(), report.values.symptoms());
+    let room_ids = report.values.room_ids();
+    let keep = (room_ids.as_slice(), report.values.symptoms());
     let (offered_rooms, offered_symptoms) = offered(&rooms, &symptoms, keep);
     let parsed = read_time(&form, "observed_at", schedule.tz()).and_then(|at| {
         let values = read_values(&form, &offered_rooms, &offered_symptoms)?;
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index d125066..8cee9b4 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -92,28 +92,36 @@ legend {
   margin-bottom: 0.5rem;
 }
 
-.levels {
-  display: grid;
-  grid-template-columns: repeat(5, 1fr);
+.rooms {
+  display: flex;
+  flex-wrap: wrap;
   gap: 0.5rem;
 }
 
-.levels legend {
-  grid-column: 1 / -1;
+.rooms legend,
+.rooms .group,
+.room:has(input[name="room"]:checked) {
+  flex-basis: 100%;
+  margin: 0;
 }
 
-.chips {
-  display: flex;
-  flex-wrap: wrap;
+.levels {
+  display: grid;
+  grid-template-columns: repeat(6, 1fr);
   gap: 0.5rem;
+  margin-top: 0.5rem;
 }
 
-.chips legend,
-.chips .group {
-  flex-basis: 100%;
+.levels legend {
+  grid-column: 1 / -1;
+  font-weight: 400;
   margin: 0;
 }
 
+.room:not(:has(input[name="room"]:checked)) .levels {
+  display: none;
+}
+
 .chip {
   position: relative;
 }
diff --git a/crates/web/templates/report_fields.html b/crates/web/templates/report_fields.html
index 46ff3d7..da20452 100644
--- a/crates/web/templates/report_fields.html
+++ b/crates/web/templates/report_fields.html
@@ -1,22 +1,16 @@
-{# The intensity, room and symptom fields of the Log and correction forms. #}
-<fieldset class="levels">
-  <legend>{{ tr.t("log-intensity") }}
+{# The room, level and symptom fields of the Log and correction forms. #}
+<fieldset class="rooms">
+  <legend>{{ tr.t("log-rooms") }}
     <a href="/guide/rating" data-dialog="rating-guide" class="info">ⓘ {{ tr.t("log-how-to-rate") }}</a>
   </legend>
-  {% for level in form.fields.intensities %}
-  <label class="chip"><input type="radio" name="intensity" value="{{ level.value }}" required{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
-  {% endfor %}
-</fieldset>
-<fieldset class="chips">
-  <legend>{{ tr.t("log-rooms") }}</legend>
   <p class="group">{{ tr.t("log-inside") }}</p>
-  {% for room in form.fields.inside %}
-  <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
+  {% for choice in form.fields.inside %}
+  {% include "room_level.html" %}
   {% endfor %}
   {% if !form.fields.common.is_empty() %}
   <p class="group">{{ tr.t("log-common") }}</p>
-  {% for room in form.fields.common %}
-  <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
+  {% for choice in form.fields.common %}
+  {% include "room_level.html" %}
   {% endfor %}
   {% endif %}
 </fieldset>
diff --git a/crates/web/templates/room_level.html b/crates/web/templates/room_level.html
new file mode 100644
index 0000000..a4174d3
--- /dev/null
+++ b/crates/web/templates/room_level.html
@@ -0,0 +1,10 @@
+{# One room chip and its level row 0–10, shown while the room is selected (spec §9, screen 1). #}
+<div class="room">
+  <label class="chip"><input type="checkbox" name="room" value="{{ choice.room.value }}"{% if choice.room.checked %} checked{% endif %}><span>{{ choice.room.label }}</span></label>
+  <fieldset class="levels">
+    <legend>{{ tr.t1("log-level-in", "room", choice.room.label) }}</legend>
+    {% for level in choice.levels %}
+    <label class="chip"><input type="radio" name="level-{{ choice.room.value }}" value="{{ level.value }}"{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
+    {% endfor %}
+  </fieldset>
+</div>
```

Then regenerate the offline query data (the `query!` texts changed):

```sh
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres scripts/sqlx-prepare.sh
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres scripts/sqlx-prepare.sh --check
```
Expected: no formatting changes, no clippy warnings; every test passes, including `migration_0003_gives_each_room_its_reports_intensity`, `migration_0003_refuses_a_report_without_rooms_and_changes_nothing` (both backends), `a_report_audited_before_room_levels_still_matches_its_converted_record`, `a_correction_lists_each_changed_value`, `a_smell_needs_offered_rooms_each_with_a_level` and `sqlite_and_postgres_schemas_have_the_same_columns` (still 66 columns). With `TEST_POSTGRES_URL` set: 462 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/error.rs \
  crates/domain/src/history.rs \
  crates/domain/src/records.rs \
  crates/domain/src/summary.rs \
  crates/domain/src/types.rs \
  crates/domain/src/verify.rs \
  crates/storage-contract/src/audit.rs \
  crates/storage-contract/src/corrections.rs \
  crates/storage-contract/src/fixture.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/logbook.rs \
  crates/storage-contract/src/migrations.rs \
  crates/storage-contract/src/reports.rs \
  crates/storage-contract/src/sweep.rs \
  crates/storage-postgres/migrations/0003_room_levels.sql \
  crates/storage-postgres/src/reports.rs \
  crates/storage-postgres/tests/contract.rs \
  crates/storage-sqlite/migrations/0003_room_levels.sql \
  crates/storage-sqlite/src/reports.rs \
  crates/storage-sqlite/tests/contract.rs \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/forms.rs \
  crates/web/src/labels.rs \
  crates/web/src/routes/entries.rs \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/guide.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/reports.rs \
  crates/web/static/app.css \
  crates/web/templates/report_fields.html \
  crates/web/templates/room_level.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/events.rs \
  crates/web/tests/log.rs \
  crates/web/tests/logging.rs \
  crates/web/tests/reports.rs \
  crates/web/tests/security.rs \
  crates/storage-postgres/.sqlx \
  crates/storage-sqlite/.sqlx
git commit -m "feat: a level 0–10 per room; a report's intensity is its highest room level

Level replaces Intensity (0 = checked and clean); ReportValues holds a
RoomLevel per room. Migration 0003 copies each report's intensity to its
rooms and refuses reports without rooms. Report snapshots audited before
the change are read in today's shape by verify-audit and History. The Log
and correction forms show a level row under each selected room.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: No-smell reports and checks outside any event

A no-smell report (every room at 0, or no rooms at all, meaning the whole home) never opens,
closes or reopens an event (spec §7.1–§7.2, decisions #105–#106). Logged now it joins the open
event whatever the answer, or stands alone as a check (`event_id` NULL); backdated it joins the
event picked, or none for "New event". A check can be corrected but never above 0, and moved into
an event but never into a new one. Migration 0004 makes `smell_reports.event_id` nullable (SQLite
rebuilds the reports table and both link tables, decision #109). The web layer changes only what
the optional event needs to compile and to show a check's page; the screens come in Task 3.

**Files:**
- Create: `crates/storage-postgres/migrations/0004_checks.sql`, `crates/storage-sqlite/migrations/0004_checks.sql`
- Modify: `crates/domain/src/error.rs`, `crates/domain/src/events.rs`, `crates/domain/src/history.rs`, `crates/domain/src/logbook.rs`, `crates/domain/src/records.rs`, `crates/domain/src/summary.rs`, `crates/storage-contract/src/audit.rs`, `crates/storage-contract/src/corrections.rs`, `crates/storage-contract/src/fixture.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/logbook.rs`, `crates/storage-contract/src/migrations.rs`, `crates/storage-contract/src/reports.rs`, `crates/storage-contract/src/sweep.rs`, `crates/storage-postgres/src/reports.rs`, `crates/storage-sqlite/src/reports.rs`, `crates/storage-sqlite/tests/contract.rs`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/routes/events.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/routes/reports.rs`, `crates/web/templates/report.html`, `crates/web/tests/events.rs`, `crates/web/tests/log.rs`, `crates/web/tests/logging.rs`, `crates/web/tests/reports.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/` (`scripts/sqlx-prepare.sh`)

**Interfaces:**
- Consumes: Task 1's `Level`, `RoomLevel`, `ReportValues`, `report_values`, `FIRST_ROOM`,
  `migrations::*`.
- Produces (in `smoke_domain`):
  - `ReportValues::new` accepts no rooms (`DomainError::NoRooms` is removed);
    `ReportValues::is_no_smell(&self) -> bool` (true without rooms); `intensity()` is
    `Level::CLEAN` without rooms.
  - `SmellReport.event_id: Option<EventId>` and `NewReport.event_id: Option<EventId>`; `None` is a
    check.
  - `events::no_smell_prompt(ObservedTime) -> LogPrompt` (`Direct` now, `WhichEvent` backdated).
  - `LogError::NoSmellCannotOpen` (move to a new event) and `LogError::CheckCannotSmell`
    (correcting a check above 0).
  - `history::Change::Moved { from: Option<EventId>, to: Option<EventId> }`;
    `summary::MonthSummary.checks: usize` (reports outside any event; `reports` now counts only
    reports in an event).
- Produces (tests and web): `logbook::no_smell(at) -> SmellEntry` in the contract suite;
  refusal keys `refused-no-smell-new-event`, `refused-check-smell`; key `report-no-event`;
  `ReportPage.event: Option<EventLink { id, date }>`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diffs, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/audit.rs b/crates/storage-contract/src/audit.rs
index 8f6c2f4..d4b8fd7 100644
--- a/crates/storage-contract/src/audit.rs
+++ b/crates/storage-contract/src/audit.rs
@@ -152,7 +152,11 @@ pub async fn a_direct_link_table_edit_is_detected<S: Store>(fx: &Fixture<S>) {
         .pop()
         .unwrap();
     let stored = fx.store.report(p.report.id).await.unwrap().unwrap();
-    let in_event = fx.store.reports_for_event(stored.event_id).await.unwrap();
+    let in_event = fx
+        .store
+        .reports_for_event(stored.event_id.unwrap())
+        .await
+        .unwrap();
     assert_eq!(in_event.len(), 1);
     let rooms = fx.store.rooms().await.unwrap();
     let target = rooms.last().unwrap();
diff --git a/crates/storage-contract/src/corrections.rs b/crates/storage-contract/src/corrections.rs
index a6be353..dff9a1d 100644
--- a/crates/storage-contract/src/corrections.rs
+++ b/crates/storage-contract/src/corrections.rs
@@ -6,7 +6,7 @@ use smoke_domain::clock::Clock;
 use smoke_domain::logbook::{
     EventChoice, LogError, MoveTarget, close_event, correct_report, move_report, reopen_event,
 };
-use smoke_domain::records::{Closure, Event, SmellReport, User};
+use smoke_domain::records::{Closure, Event, ReportValues, SmellReport, User};
 use smoke_domain::repos::{
     AuditRepo, Commit, EventRepo, ListRepo, ReportWrites, Transactional, WriteTx,
 };
@@ -14,7 +14,7 @@ use smoke_domain::types::{CloseReason, EventId, ReportId};
 
 use crate::Fixture;
 use crate::fixture::{FIRST_ROOM, add_event, report_values, start};
-use crate::logbook::{log, people, rome, smell};
+use crate::logbook::{log, no_smell, people, rome, smell};
 
 async fn close<S>(
     fx: &Fixture<S>,
@@ -46,13 +46,26 @@ async fn correct<S>(
     who: &User,
     id: ReportId,
     at: chrono::DateTime<chrono::Utc>,
-    intensity: u8,
+    level: u8,
+) -> Result<SmellReport, LogError>
+where
+    S: Transactional<Tx: WriteTx>,
+{
+    let values = report_values(level, &[FIRST_ROOM], &[]);
+    correct_values(fx, who, id, at, values).await
+}
+
+async fn correct_values<S>(
+    fx: &Fixture<S>,
+    who: &User,
+    id: ReportId,
+    at: chrono::DateTime<chrono::Utc>,
+    values: ReportValues,
 ) -> Result<SmellReport, LogError>
 where
     S: Transactional<Tx: WriteTx>,
 {
     let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
-    let values = report_values(intensity, &[FIRST_ROOM], &[]);
     let now = fx.clock.now();
     let report = correct_report(&mut tx, &who.actor(), id, at, values, &rome(), now).await?;
     tx.commit().await.unwrap();
@@ -91,7 +104,7 @@ where
     )
     .await
     .unwrap();
-    let id = logged.report.event_id;
+    let id = logged.report.event_id.unwrap();
     fx.clock.advance(TimeDelta::hours(1));
 
     let early = close(fx, &p.resident, id, start() - TimeDelta::minutes(1)).await;
@@ -134,7 +147,8 @@ where
     .await
     .unwrap()
     .report
-    .event_id;
+    .event_id
+    .unwrap();
     for who in [&p.guest, &p.viewer] {
         let refused = close(fx, who, residents, start()).await;
         assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
@@ -153,7 +167,8 @@ where
     .await
     .unwrap()
     .report
-    .event_id;
+    .event_id
+    .unwrap();
     close(fx, &p.guest, guests, fx.clock.now()).await.unwrap();
     reopen(fx, &p.guest, guests).await.unwrap();
     assert!(event(fx, guests).await.is_open());
@@ -210,7 +225,7 @@ pub async fn correcting_after_the_end_moves_the_end_and_before_the_start_is_refu
     )
     .await
     .unwrap();
-    let id = logged.report.event_id;
+    let id = logged.report.event_id.unwrap();
     close(fx, &p.resident, id, opened + TimeDelta::minutes(30))
         .await
         .unwrap();
@@ -251,7 +266,7 @@ where
     )
     .await
     .unwrap();
-    let event_id = residents.report.event_id;
+    let event_id = residents.report.event_id.unwrap();
     let guests = log(fx, &p.guest, smell(start(), 5), EventChoice::Join(event_id))
         .await
         .unwrap();
@@ -293,7 +308,7 @@ where
         fx,
         &p.guest,
         smell(start(), 5),
-        EventChoice::Join(first.report.event_id),
+        EventChoice::Join(first.report.event_id.unwrap()),
     )
     .await
     .unwrap();
@@ -302,7 +317,7 @@ where
         .await
         .unwrap();
     assert_ne!(moved.event_id, first.report.event_id);
-    let new = event(fx, moved.event_id).await;
+    let new = event(fx, moved.event_id.unwrap()).await;
     assert_eq!((new.owner_id, new.opened_at), (p.guest.id, start()));
     assert_eq!(
         new.closed,
@@ -316,7 +331,7 @@ where
         fx,
         &p.partner,
         joined.report.id,
-        MoveTarget::Event(moved.event_id),
+        MoveTarget::Event(moved.event_id.unwrap()),
     )
     .await
     .unwrap();
@@ -357,7 +372,7 @@ where
     )
     .await
     .unwrap();
-    assert_eq!(moved.event_id, earlier.id);
+    assert_eq!(moved.event_id, Some(earlier.id));
     assert_eq!(
         event(fx, earlier.id).await.closed.map(|c| c.at),
         Some(later_at)
@@ -368,7 +383,7 @@ where
         &p.resident,
         smell(opened - TimeDelta::hours(1), 4),
         EventChoice::New {
-            replacing: Some(later.report.event_id),
+            replacing: later.report.event_id,
         },
     )
     .await
@@ -452,3 +467,68 @@ where
     let deleted = relocate(fx, &p.resident, logged.report.id, MoveTarget::New).await;
     assert!(matches!(deleted, Err(LogError::NotFound)), "{deleted:?}");
 }
+
+pub async fn a_check_can_be_corrected_but_never_to_a_smell<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
+{
+    let p = people(fx).await;
+    let check = log(
+        fx,
+        &p.guest,
+        no_smell(start()),
+        EventChoice::New { replacing: None },
+    )
+    .await
+    .unwrap()
+    .report;
+    let earlier = start() - TimeDelta::hours(5);
+    let corrected = correct_values(
+        fx,
+        &p.guest,
+        check.id,
+        earlier,
+        report_values(0, &[FIRST_ROOM], &[]),
+    )
+    .await
+    .unwrap();
+    assert_eq!((corrected.event_id, corrected.observed_at), (None, earlier));
+    let refused = correct_values(
+        fx,
+        &p.guest,
+        check.id,
+        earlier,
+        report_values(3, &[FIRST_ROOM], &[]),
+    )
+    .await;
+    assert!(
+        matches!(refused, Err(LogError::CheckCannotSmell)),
+        "{refused:?}"
+    );
+}
+
+pub async fn a_check_can_move_into_an_event_but_never_opens_one<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
+{
+    let p = people(fx).await;
+    let check = log(
+        fx,
+        &p.resident,
+        no_smell(start()),
+        EventChoice::New { replacing: None },
+    )
+    .await
+    .unwrap()
+    .report;
+    let refused = relocate(fx, &p.resident, check.id, MoveTarget::New).await;
+    assert!(
+        matches!(refused, Err(LogError::NoSmellCannotOpen)),
+        "{refused:?}"
+    );
+    let earlier = add_event(fx, p.resident.id, start() - TimeDelta::hours(1), None).await;
+    let moved = relocate(fx, &p.resident, check.id, MoveTarget::Event(earlier.id))
+        .await
+        .unwrap();
+    assert_eq!(moved.event_id, Some(earlier.id));
+}
diff --git a/crates/storage-contract/src/fixture.rs b/crates/storage-contract/src/fixture.rs
index e21f4ca..20b41a3 100644
--- a/crates/storage-contract/src/fixture.rs
+++ b/crates/storage-contract/src/fixture.rs
@@ -182,7 +182,7 @@ where
     let mut tx = fx.store.begin(Some(scene.user.id)).await.expect("begin");
     let report = tx
         .create_report(NewReport {
-            event_id: scene.event.id,
+            event_id: Some(scene.event.id),
             user_id: scene.user.id,
             observed_at,
             values,
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index 4ff7d86..0496215 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -112,6 +112,9 @@ macro_rules! contract_tests {
             logbook::a_failed_report_rolls_back_the_new_event_close,
             logbook::only_people_who_may_log_can_log,
             logbook::a_time_in_the_future_is_refused,
+            logbook::a_no_smell_report_with_nothing_open_is_a_check_outside_any_event,
+            logbook::a_no_smell_report_joins_the_open_event_without_closing_it,
+            logbook::a_backdated_no_smell_report_joins_the_event_picked_or_none,
             corrections::a_manual_close_needs_a_time_between_the_last_report_and_now,
             corrections::only_residents_and_owning_guests_can_close_or_reopen,
             corrections::reopening_waits_until_no_other_event_is_open,
@@ -121,6 +124,8 @@ macro_rules! contract_tests {
             corrections::moving_after_the_end_of_the_target_moves_its_end,
             corrections::a_report_cannot_move_into_another_days_event,
             corrections::a_deleted_report_or_a_missing_target_event_is_not_found,
+            corrections::a_check_can_be_corrected_but_never_to_a_smell,
+            corrections::a_check_can_move_into_an_event_but_never_opens_one,
             sweep::the_first_sweep_runs_and_is_remembered,
             sweep::a_sweep_at_four_closes_the_event_at_its_last_report,
             sweep::a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four,
diff --git a/crates/storage-contract/src/logbook.rs b/crates/storage-contract/src/logbook.rs
index 8472062..1f86c85 100644
--- a/crates/storage-contract/src/logbook.rs
+++ b/crates/storage-contract/src/logbook.rs
@@ -4,7 +4,7 @@ use chrono::{DateTime, TimeDelta, Utc};
 use smoke_domain::DomainError;
 use smoke_domain::clock::Clock;
 use smoke_domain::logbook::{EventChoice, LogError, Logged, SmellEntry, log_reading, log_smell};
-use smoke_domain::records::{Closure, Event, Pm25Reading, User};
+use smoke_domain::records::{Closure, Event, Pm25Reading, ReportValues, User};
 use smoke_domain::repos::{
     AuditRepo, Commit, EventRepo, ListRepo, ListWrites, ReadingRepo, ReportRepo, Transactional,
     UserWrites, WriteTx,
@@ -49,6 +49,15 @@ pub(crate) fn smell(at: DateTime<Utc>, intensity: u8) -> SmellEntry {
     }
 }
 
+/// "No smell" with no rooms: nothing anywhere in the home.
+pub(crate) fn no_smell(at: DateTime<Utc>) -> SmellEntry {
+    SmellEntry {
+        observed_at: at,
+        values: ReportValues::new(vec![], vec![], None).unwrap(),
+        pm25: None,
+    }
+}
+
 /// Runs [`log_smell`] in its own transaction, committed only on success.
 pub(crate) async fn log<S>(
     fx: &Fixture<S>,
@@ -104,7 +113,7 @@ where
     .await
     .unwrap();
     let open = fx.store.open_event().await.unwrap().unwrap();
-    assert_eq!(logged.report.event_id, open.id);
+    assert_eq!(logged.report.event_id, Some(open.id));
     assert_eq!((open.owner_id, open.opened_at), (p.guest.id, start()));
     assert_eq!(logged.reading, None);
 }
@@ -123,7 +132,7 @@ where
     .await
     .unwrap();
     fx.clock.advance(TimeDelta::minutes(20));
-    let event_id = first.report.event_id;
+    let event_id = first.report.event_id.unwrap();
     let second = log(
         fx,
         &p.guest,
@@ -132,7 +141,7 @@ where
     )
     .await
     .unwrap();
-    assert_eq!(second.report.event_id, event_id);
+    assert_eq!(second.report.event_id, Some(event_id));
     assert_eq!(event(fx, event_id).await.owner_id, p.resident.id);
     assert_eq!(fx.store.reports_for_event(event_id).await.unwrap().len(), 2);
 }
@@ -155,7 +164,7 @@ where
         fx,
         &p.resident,
         smell(fx.clock.now(), 2),
-        EventChoice::Join(first.report.event_id),
+        EventChoice::Join(first.report.event_id.unwrap()),
     )
     .await
     .unwrap();
@@ -165,13 +174,13 @@ where
         &p.guest,
         smell(fx.clock.now(), 5),
         EventChoice::New {
-            replacing: Some(first.report.event_id),
+            replacing: first.report.event_id,
         },
     )
     .await
     .unwrap();
 
-    let old = event(fx, first.report.event_id).await;
+    let old = event(fx, first.report.event_id.unwrap()).await;
     assert_eq!(
         old.closed,
         Some(Closure {
@@ -180,7 +189,7 @@ where
         })
     );
     let open = fx.store.open_event().await.unwrap().unwrap();
-    assert_eq!(open.id, next.report.event_id);
+    assert_eq!(Some(open.id), next.report.event_id);
     assert_eq!(open.owner_id, p.guest.id);
 }
 
@@ -204,7 +213,7 @@ where
         &p.partner,
         smell(fx.clock.now(), 5),
         EventChoice::New {
-            replacing: Some(first.report.event_id),
+            replacing: first.report.event_id,
         },
     )
     .await
@@ -215,7 +224,7 @@ where
         fx,
         &p.guest,
         smell(fx.clock.now(), 6),
-        EventChoice::Join(first.report.event_id),
+        EventChoice::Join(first.report.event_id.unwrap()),
     )
     .await;
     assert!(matches!(late_yes, Err(LogError::Stale)), "{late_yes:?}");
@@ -223,7 +232,7 @@ where
         fx,
         &p.guest,
         smell(fx.clock.now(), 6),
-        EventChoice::Continue(first.report.event_id),
+        EventChoice::Continue(first.report.event_id.unwrap()),
     )
     .await;
     assert!(
@@ -254,7 +263,7 @@ where
     )
     .await
     .unwrap();
-    let first_id = first.report.event_id;
+    let first_id = first.report.event_id.unwrap();
     fx.clock.advance(TimeDelta::minutes(5));
     let second = log(
         fx,
@@ -288,7 +297,7 @@ where
     assert!(matches!(unaware, Err(LogError::Stale)), "{unaware:?}");
     assert_eq!(
         fx.store.open_event().await.unwrap().map(|e| e.id),
-        Some(second.report.event_id)
+        second.report.event_id
     );
     assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
 }
@@ -308,7 +317,11 @@ pub async fn a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_pas
     )
     .await
     .unwrap();
-    assert!(event(fx, earlier_tonight.report.event_id).await.is_open());
+    assert!(
+        event(fx, earlier_tonight.report.event_id.unwrap())
+            .await
+            .is_open()
+    );
 
     let earlier_still = start() - TimeDelta::hours(2);
     let while_open = log(
@@ -316,13 +329,13 @@ pub async fn a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_pas
         &p.resident,
         smell(earlier_still, 3),
         EventChoice::New {
-            replacing: Some(earlier_tonight.report.event_id),
+            replacing: earlier_tonight.report.event_id,
         },
     )
     .await
     .unwrap();
     assert_eq!(
-        event(fx, while_open.report.event_id).await.closed,
+        event(fx, while_open.report.event_id.unwrap()).await.closed,
         Some(Closure {
             at: earlier_still,
             reason: CloseReason::Backdated
@@ -336,20 +349,22 @@ pub async fn a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_pas
         &p.resident,
         smell(yesterday, 3),
         EventChoice::New {
-            replacing: Some(earlier_tonight.report.event_id),
+            replacing: earlier_tonight.report.event_id,
         },
     )
     .await
     .unwrap();
     assert_eq!(
-        event(fx, past_four.report.event_id)
+        event(fx, past_four.report.event_id.unwrap())
             .await
             .closed
             .map(|c| c.reason),
         Some(CloseReason::Backdated)
     );
     assert!(
-        event(fx, earlier_tonight.report.event_id).await.is_open(),
+        event(fx, earlier_tonight.report.event_id.unwrap())
+            .await
+            .is_open(),
         "a past report never closes the open event"
     );
 }
@@ -374,7 +389,7 @@ where
     )
     .await
     .unwrap();
-    assert_eq!(inside.report.event_id, past.id);
+    assert_eq!(inside.report.event_id, Some(past.id));
     assert_eq!(event(fx, past.id).await.closed, Some(closed));
 
     let after = opened + TimeDelta::hours(2);
@@ -449,7 +464,7 @@ where
     )
     .await
     .unwrap();
-    assert_eq!(logged.report.event_id, guests.id);
+    assert_eq!(logged.report.event_id, Some(guests.id));
     assert!(event(fx, guests.id).await.is_open());
 
     let manual = add_event(
@@ -510,14 +525,14 @@ where
         .await
         .unwrap();
     let reading = logged.reading.unwrap();
-    assert_eq!(reading.event_id, Some(logged.report.event_id));
+    assert_eq!(reading.event_id, logged.report.event_id);
     assert_eq!(
         (reading.observed_at, reading.user_id),
         (start(), p.resident.id)
     );
     assert_eq!(
         fx.store
-            .readings_for_event(logged.report.event_id)
+            .readings_for_event(logged.report.event_id.unwrap())
             .await
             .unwrap(),
         [reading]
@@ -584,7 +599,7 @@ where
         &p.guest.actor(),
         entry,
         EventChoice::New {
-            replacing: Some(first.report.event_id),
+            replacing: first.report.event_id,
         },
         &rome(),
         fx.clock.now(),
@@ -595,7 +610,7 @@ where
 
     assert_eq!(
         fx.store.open_event().await.unwrap().map(|e| e.id),
-        Some(first.report.event_id),
+        first.report.event_id,
         "the close and the new event were rolled back with the report"
     );
     assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
@@ -661,3 +676,89 @@ where
         "{future:?}"
     );
 }
+
+pub async fn a_no_smell_report_with_nothing_open_is_a_check_outside_any_event<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + ReportRepo,
+{
+    let p = people(fx).await;
+    let logged = log(
+        fx,
+        &p.guest,
+        no_smell(start()),
+        EventChoice::New { replacing: None },
+    )
+    .await
+    .unwrap();
+    assert_eq!(logged.report.event_id, None);
+    assert!(logged.report.values.rooms().is_empty(), "the whole home");
+    assert_eq!(fx.store.open_event().await.unwrap(), None);
+    assert_eq!(fx.store.event(EventId(1)).await.unwrap(), None);
+    let listed = fx
+        .store
+        .reports_between(start(), start() + TimeDelta::minutes(1))
+        .await
+        .unwrap();
+    assert_eq!(listed, [logged.report]);
+}
+
+pub async fn a_no_smell_report_joins_the_open_event_without_closing_it<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
+{
+    let p = people(fx).await;
+    let first = log(
+        fx,
+        &p.resident,
+        smell(start(), 6),
+        EventChoice::New { replacing: None },
+    )
+    .await
+    .unwrap();
+    fx.clock.advance(TimeDelta::minutes(30));
+    let clean = SmellEntry {
+        values: report_values(0, &[FIRST_ROOM], &[]),
+        ..smell(fx.clock.now(), 0)
+    };
+    let answer = EventChoice::New {
+        replacing: first.report.event_id,
+    };
+    let joined = log(fx, &p.guest, clean, answer).await.unwrap();
+    assert_eq!(joined.report.event_id, first.report.event_id);
+    let open = fx.store.open_event().await.unwrap().unwrap();
+    assert_eq!(Some(open.id), first.report.event_id);
+}
+
+pub async fn a_backdated_no_smell_report_joins_the_event_picked_or_none<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
+{
+    let p = people(fx).await;
+    let past = add_event(fx, p.resident.id, start() - TimeDelta::hours(3), None).await;
+    let earlier = start() - TimeDelta::hours(2);
+    let joined = log(
+        fx,
+        &p.resident,
+        no_smell(earlier),
+        EventChoice::Join(past.id),
+    )
+    .await
+    .unwrap();
+    assert_eq!(joined.report.event_id, Some(past.id));
+
+    let before = start() - TimeDelta::hours(4);
+    let check = log(
+        fx,
+        &p.resident,
+        no_smell(before),
+        EventChoice::New { replacing: None },
+    )
+    .await
+    .unwrap();
+    assert_eq!(check.report.event_id, None);
+    assert_eq!(
+        fx.store.events_between(before, start()).await.unwrap(),
+        [event(fx, past.id).await],
+        "no event was created"
+    );
+}
diff --git a/crates/storage-contract/src/migrations.rs b/crates/storage-contract/src/migrations.rs
index b912d8a..b21f5ba 100644
--- a/crates/storage-contract/src/migrations.rs
+++ b/crates/storage-contract/src/migrations.rs
@@ -2,18 +2,20 @@
 //! rows into a database migrated up to the version before, then run the remaining migrations.
 
 use smoke_domain::repos::ReportRepo;
-use smoke_domain::types::{Level, ReportId, RoomId};
+use smoke_domain::types::{EventId, Level, ReportId, RoomId, SymptomId};
 
-/// Rows as the schema of migration 0002 stores them: two rooms; report 1 at intensity 6 in
-/// both, report 2 at intensity 4 in the second.
+/// Rows as the schema of migration 0002 stores them: two rooms and a symptom; report 1 at
+/// intensity 6 in both rooms, with the symptom; report 2 at intensity 4 in the second room.
 pub const ROWS_BEFORE_0003: &str = "\
     INSERT INTO users (id, username, role, locale, created_at) \
         VALUES (1, 'daniele', 'resident', 'en', 0); \
     INSERT INTO rooms (id, name, area) VALUES (1, 'Kitchen', 'inside'), (2, 'Bedroom', 'inside'); \
+    INSERT INTO symptoms (id, name) VALUES (1, 'Cough'); \
     INSERT INTO events (id, owner_id, opened_at, recorded_at) VALUES (1, 1, 0, 0); \
     INSERT INTO smell_reports (id, event_id, user_id, observed_at, recorded_at, intensity) \
         VALUES (1, 1, 1, 0, 0, 6), (2, 1, 1, 60000000, 60000000, 4); \
-    INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES (1, 1), (1, 2), (2, 2);";
+    INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES (1, 1), (1, 2), (2, 2); \
+    INSERT INTO smell_report_symptoms (smell_report_id, symptom_id) VALUES (1, 1);";
 
 /// A report without rooms, which migration 0003 refuses to convert.
 pub const REPORT_WITHOUT_ROOMS: &str = "\
@@ -23,8 +25,12 @@ pub const REPORT_WITHOUT_ROOMS: &str = "\
 /// The name the refusal carries in its error message.
 pub const NEEDS_A_ROOM: &str = "every_smell_report_needs_a_room";
 
-/// After migration 0003, every room of [`ROWS_BEFORE_0003`] has its report's intensity.
+/// After the migrations, every room of [`ROWS_BEFORE_0003`] has its report's intensity, and the
+/// reports keep their event and symptoms.
 pub async fn each_room_took_its_reports_intensity<S: ReportRepo>(store: &S) {
+    let first = store.report(ReportId(1)).await.unwrap().unwrap();
+    assert_eq!(first.event_id, Some(EventId(1)));
+    assert_eq!(first.values.symptoms(), [SymptomId(1)]);
     let levels = |id| async move {
         let report = store.report(ReportId(id)).await.unwrap().unwrap();
         report
diff --git a/crates/storage-contract/src/reports.rs b/crates/storage-contract/src/reports.rs
index e54c483..0a1a33d 100644
--- a/crates/storage-contract/src/reports.rs
+++ b/crates/storage-contract/src/reports.rs
@@ -40,7 +40,7 @@ where
             report.event_id,
             report.user_id
         ),
-        (observed_at, start(), scene.event.id, scene.user.id)
+        (observed_at, start(), Some(scene.event.id), scene.user.id)
     );
     assert_eq!(
         fx.store.report(report.id).await.unwrap(),
@@ -111,7 +111,7 @@ where
 
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
     let moved = tx.move_report(report.id, other.id).await.unwrap();
-    assert_eq!(moved.event_id, other.id);
+    assert_eq!(moved.event_id, Some(other.id));
     tx.move_report(report.id, other.id).await.unwrap();
     tx.commit().await.unwrap();
 
@@ -220,7 +220,7 @@ where
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
     let error = tx
         .create_report(NewReport {
-            event_id: scene.event.id,
+            event_id: Some(scene.event.id),
             user_id: scene.user.id,
             observed_at: start(),
             values: report_values(5, &[RoomId(999)], &[]),
@@ -341,7 +341,7 @@ where
     let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
     let error = tx
         .create_report(NewReport {
-            event_id: scene.event.id,
+            event_id: Some(scene.event.id),
             user_id: scene.user.id,
             observed_at: start(),
             values: report_values(5, &[FIRST_ROOM], &[SymptomId(999)]),
diff --git a/crates/storage-contract/src/sweep.rs b/crates/storage-contract/src/sweep.rs
index e64fc08..ae017cd 100644
--- a/crates/storage-contract/src/sweep.rs
+++ b/crates/storage-contract/src/sweep.rs
@@ -31,7 +31,7 @@ where
 {
     let mut tx = fx.store.begin(Some(user)).await.unwrap();
     tx.create_report(NewReport {
-        event_id: event.id,
+        event_id: Some(event.id),
         user_id: user,
         observed_at: at,
         values: report_values(5, &[FIRST_ROOM], &[]),
diff --git a/crates/storage-sqlite/tests/contract.rs b/crates/storage-sqlite/tests/contract.rs
index 977e4b7..a998366 100644
--- a/crates/storage-sqlite/tests/contract.rs
+++ b/crates/storage-sqlite/tests/contract.rs
@@ -188,3 +188,25 @@ async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
             .unwrap();
     assert_eq!(intensities, [6, 4, 5]);
 }
+
+#[tokio::test]
+async fn after_the_rebuild_the_link_tables_still_refer_to_the_reports() {
+    let (_dir, url) = database();
+    let store = SqliteStore::connect(&url, Arc::new(SystemClock))
+        .await
+        .unwrap();
+    store.migrate().await.unwrap();
+    let pool = SqlitePool::connect(&url).await.unwrap();
+    for table in ["smell_report_rooms", "smell_report_symptoms"] {
+        let targets: Vec<String> =
+            sqlx::query_scalar("SELECT \"table\" FROM pragma_foreign_key_list($1)")
+                .bind(table)
+                .fetch_all(&pool)
+                .await
+                .unwrap();
+        assert!(
+            targets.contains(&"smell_reports".to_owned()),
+            "{table}: {targets:?}"
+        );
+    }
+}
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index 493749d..92f93ec 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -24,7 +24,7 @@ async fn the_list_groups_a_months_events_by_day_newest_first() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let join = EventChoice::Join(first.report.event_id);
+    let join = EventChoice::Join(first.report.event_id.unwrap());
     app.log(&me, "2026-01-10T19:00:00Z", 7, &[], join).await;
     let second = app
         .log(
@@ -49,14 +49,14 @@ async fn the_list_groups_a_months_events_by_day_newest_first() {
     assert!(
         html.contains(&format!(
             r#"<a href="/events/{}">19:00–20:00</a> · 2 reports · max 7"#,
-            first.report.event_id
+            first.report.event_id.unwrap()
         )),
         "{html}"
     );
     assert!(
         html.contains(&format!(
             r#"<a href="/events/{}">08:30–"#,
-            second.report.event_id
+            second.report.event_id.unwrap()
         )),
         "{html}"
     );
@@ -97,19 +97,19 @@ async fn guests_list_only_events_they_own_or_reported_in() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let join = EventChoice::Join(shared.report.event_id);
+    let join = EventChoice::Join(shared.report.event_id.unwrap());
     app.log(&guest, "2026-01-11T18:30:00Z", 4, &[], join).await;
     let cookie = app.sign_in("guest").await;
 
     let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
     assert!(
-        !html.contains(&format!("/events/{}", private.report.event_id)),
+        !html.contains(&format!("/events/{}", private.report.event_id.unwrap())),
         "{html}"
     );
     assert!(
         html.contains(&format!(
             r#"<a href="/events/{}">19:00–19:30</a> · 1 report · max 4"#,
-            shared.report.event_id
+            shared.report.event_id.unwrap()
         )),
         "only the guest's own report counts: {html}"
     );
@@ -131,7 +131,7 @@ async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = first.report.event_id;
+    let id = first.report.event_id.unwrap();
     app.log(
         &guest,
         "2026-01-15T10:00:00Z",
@@ -184,7 +184,7 @@ async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = shared.report.event_id;
+    let id = shared.report.event_id.unwrap();
     app.log(
         &guest,
         "2026-01-15T09:30:00Z",
@@ -213,7 +213,7 @@ async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
     }
     let other = app
         .send(get_with(
-            &format!("/events/{}", private.report.event_id),
+            &format!("/events/{}", private.report.event_id.unwrap()),
             &cookie,
         ))
         .await;
@@ -236,7 +236,7 @@ async fn closing_by_hand_needs_an_end_between_the_last_report_and_now() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = open.report.event_id;
+    let id = open.report.event_id.unwrap();
     let cookie = app.sign_in("daniele").await;
     let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
     assert!(
@@ -303,7 +303,7 @@ async fn the_log_screen_closes_the_open_event_now() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = open.report.event_id;
+    let id = open.report.event_id.unwrap();
     let cookie = app.sign_in("daniele").await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
     assert!(
@@ -339,7 +339,7 @@ async fn only_residents_and_owning_guests_can_close_or_reopen() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = open.report.event_id;
+    let id = open.report.event_id.unwrap();
     app.log(
         &guest,
         "2026-01-15T19:30:00Z",
@@ -375,7 +375,7 @@ async fn reopening_is_offered_only_while_no_other_event_is_open() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = earlier.report.event_id;
+    let id = earlier.report.event_id.unwrap();
     let current = app
         .log(
             &me,
@@ -404,7 +404,7 @@ async fn reopening_is_offered_only_while_no_other_event_is_open() {
             .contains("Another event is open. Close it first.")
     );
 
-    let close = format!("/events/{}/close", current.report.event_id);
+    let close = format!("/events/{}/close", current.report.event_id.unwrap());
     app.send(post_form(&close, "", Some(&cookie))).await;
     let response = app
         .send(post_form(
@@ -432,7 +432,7 @@ async fn closing_or_reopening_an_event_a_guest_cannot_see_answers_not_found() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = open.report.event_id;
+    let id = open.report.event_id.unwrap();
     let guest = app.sign_in("guest").await;
 
     let close = app
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index a3376df..6744155 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -61,7 +61,7 @@ async fn with_an_event_open_the_form_offers_an_update_with_my_last_rooms() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let event = EventChoice::Join(first.report.event_id);
+    let event = EventChoice::Join(first.report.event_id.unwrap());
     app.log(&partner, "2026-01-15T19:55:00Z", 6, &[kitchen], event)
         .await;
     let cookie = app.sign_in("daniele").await;
@@ -119,7 +119,7 @@ async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = first.report.event_id;
+    let id = first.report.event_id.unwrap();
     let close = format!(r#"action="/events/{}/close""#, id.0);
 
     let cookie = app.sign_in("guest").await;
@@ -190,7 +190,7 @@ async fn the_summary_counts_this_month_and_guests_count_only_their_own() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let join = EventChoice::Join(first.report.event_id);
+    let join = EventChoice::Join(first.report.event_id.unwrap());
     app.log(&guest, "2026-01-10T21:00:00Z", 3, &[], join).await;
     app.log(
         &guest,
@@ -234,7 +234,11 @@ async fn opening_the_log_screen_runs_the_auto_close_sweep() {
 
     let html = text(app.send(get_with("/", &cookie)).await).await;
     assert!(html.contains("No smell event in progress."), "{html}");
-    let event = app.store.event(logged.report.event_id).await.unwrap();
+    let event = app
+        .store
+        .event(logged.report.event_id.unwrap())
+        .await
+        .unwrap();
     assert_eq!(
         event.unwrap().closed.map(|c| c.reason),
         Some(CloseReason::Auto0400)
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index 6b7384e..05407f6 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -68,7 +68,7 @@ async fn with_an_event_open_the_same_smell_question_comes_first() {
     assert!(html.contains("<h1>Same smell still going?</h1>"), "{html}");
     assert!(html.contains("(started 19:05)"), "{html}");
     assert!(html.contains(r#"<input type="hidden" name="level-1" value="7">"#));
-    let id = first.report.event_id;
+    let id = first.report.event_id.unwrap();
     assert!(html.contains(&format!(r#"value="join-{id}">Yes, update</button>"#)));
     assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));
 
@@ -98,13 +98,13 @@ async fn answering_no_closes_the_open_event_at_its_last_report() {
     let cookie = app.sign_in("daniele").await;
     let body = format!(
         "room=1&level-1=3&action=smell&asked=same-smell-{}&choice=new",
-        first.report.event_id
+        first.report.event_id.unwrap()
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=report"));
     let old = app
         .store
-        .event(first.report.event_id)
+        .event(first.report.event_id.unwrap())
         .await
         .unwrap()
         .unwrap();
@@ -114,7 +114,7 @@ async fn answering_no_closes_the_open_event_at_its_last_report() {
     );
     assert_ne!(
         app.store.open_event().await.unwrap().unwrap().id,
-        first.report.event_id
+        first.report.event_id.unwrap()
     );
 }
 
@@ -141,7 +141,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
             8,
             &[kitchen],
             EventChoice::New {
-                replacing: Some(first.report.event_id),
+                replacing: first.report.event_id,
             },
         )
         .await;
@@ -174,7 +174,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     )
     .await;
     assert!(past.contains("<h1>Which event was this?</h1>"), "{past}");
-    let earlier = closed_earlier.report.event_id;
+    let earlier = closed_earlier.report.event_id.unwrap();
     assert!(
         past.contains(&format!(
             r#"value="join-{earlier}" class="secondary">Started 11:00</button>"#
@@ -182,7 +182,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
         "{past}"
     );
     assert!(
-        !past.contains(&format!("join-{}", first.report.event_id)),
+        !past.contains(&format!("join-{}", first.report.event_id.unwrap())),
         "not started yet"
     );
     for private in ["daniele", "Kitchen", "ended"] {
@@ -223,7 +223,7 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
             5,
             &[],
             EventChoice::New {
-                replacing: Some(first.report.event_id),
+                replacing: first.report.event_id,
             },
         )
         .await;
@@ -231,19 +231,19 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
     let body = format!(
         "room=1&level-1=7&action=smell&asked={}&choice=join-{}",
         asked(&question),
-        first.report.event_id
+        first.report.event_id.unwrap()
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(response.status(), StatusCode::OK);
     let html = text(response).await;
     assert_eq!(
         asked(&html),
-        format!("same-smell-{}", second.report.event_id)
+        format!("same-smell-{}", second.report.event_id.unwrap())
     );
     assert!(html.contains("(started 20:58)"), "{html}");
     assert_eq!(
         app.store
-            .reports_for_event(first.report.event_id)
+            .reports_for_event(first.report.event_id.unwrap())
             .await
             .unwrap()
             .len(),
@@ -266,7 +266,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = past.report.event_id;
+    let id = past.report.event_id.unwrap();
     app.log(
         &me,
         "2026-01-15T20:00:00Z",
@@ -318,7 +318,7 @@ async fn after_an_auto_close_the_next_log_offers_a_continuation() {
             EventChoice::New { replacing: None },
         )
         .await;
-    let id = evening.report.event_id;
+    let id = evening.report.event_id.unwrap();
     let cookie = app.sign_in("daniele").await;
     app.at("2026-01-16T06:00:00Z");
 
@@ -411,7 +411,7 @@ async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
     assert_eq!(location(&response), Some("/?saved=pm25"));
     let readings = app
         .store
-        .readings_for_event(open.report.event_id)
+        .readings_for_event(open.report.event_id.unwrap())
         .await
         .unwrap();
     assert_eq!(readings.len(), 1);
@@ -419,13 +419,13 @@ async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
 
     let with_smell = format!(
         "room=1&level-1=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
-        open.report.event_id
+        open.report.event_id.unwrap()
     );
     app.send(post_form("/log", &with_smell, Some(&cookie)))
         .await;
     assert_eq!(
         app.store
-            .readings_for_event(open.report.event_id)
+            .readings_for_event(open.report.event_id.unwrap())
             .await
             .unwrap()
             .len(),
diff --git a/crates/web/tests/reports.rs b/crates/web/tests/reports.rs
index 4eb361e..68398ab 100644
--- a/crates/web/tests/reports.rs
+++ b/crates/web/tests/reports.rs
@@ -167,7 +167,7 @@ async fn moving_offers_that_days_events_and_a_new_one() {
             5,
             &[],
             EventChoice::New {
-                replacing: Some(morning.report.event_id),
+                replacing: morning.report.event_id,
             },
         )
         .await;
@@ -177,7 +177,7 @@ async fn moving_offers_that_days_events_and_a_new_one() {
             "2026-01-15T11:00:00Z",
             5,
             &[],
-            EventChoice::Join(morning.report.event_id),
+            EventChoice::Join(morning.report.event_id.unwrap()),
         )
         .await;
     let stray = app
@@ -186,7 +186,7 @@ async fn moving_offers_that_days_events_and_a_new_one() {
             "2026-01-15T20:00:00Z",
             3,
             &[],
-            EventChoice::Join(evening.report.event_id),
+            EventChoice::Join(evening.report.event_id.unwrap()),
         )
         .await;
     let cookie = app.sign_in("daniele").await;
@@ -203,7 +203,7 @@ async fn moving_offers_that_days_events_and_a_new_one() {
 
     let path = format!("/reports/{}", stray.report.id);
     let html = text(app.send(get_with(&path, &cookie)).await).await;
-    let morning_id = morning.report.event_id;
+    let morning_id = morning.report.event_id.unwrap();
     assert!(
         html.contains(&format!(
             r#"<option value="event-{morning_id}">Started 11:00, ended 12:00</option>"#
@@ -219,14 +219,14 @@ async fn moving_offers_that_days_events_and_a_new_one() {
         .await;
     assert_eq!(location(&response), Some(&*path));
     let moved = app.store.report(stray.report.id).await.unwrap().unwrap();
-    assert_eq!(moved.event_id, morning_id);
+    assert_eq!(moved.event_id, Some(morning_id));
     let event = app.store.event(morning_id).await.unwrap().unwrap();
     assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T20:00:00Z")));
     let html = text(app.send(get_with(&path, &cookie)).await).await;
     assert!(
         html.contains(&format!(
             "moved: event #{} → #{morning_id}",
-            evening.report.event_id
+            evening.report.event_id.unwrap()
         )),
         "{html}"
     );
@@ -253,7 +253,7 @@ async fn guests_cannot_see_or_change_other_peoples_reports() {
             "2026-01-15T19:30:00Z",
             4,
             &[],
-            EventChoice::Join(mine.report.event_id),
+            EventChoice::Join(mine.report.event_id.unwrap()),
         )
         .await;
     let cookie = app.sign_in("guest").await;
@@ -350,7 +350,7 @@ async fn guests_see_other_events_in_the_move_options_without_their_end() {
             3,
             &[],
             EventChoice::New {
-                replacing: Some(morning.report.event_id),
+                replacing: morning.report.event_id,
             },
         )
         .await;
@@ -358,7 +358,7 @@ async fn guests_see_other_events_in_the_move_options_without_their_end() {
     app.sweep().await;
     let morning_event = app
         .store
-        .event(morning.report.event_id)
+        .event(morning.report.event_id.unwrap())
         .await
         .unwrap()
         .unwrap();
@@ -373,7 +373,7 @@ async fn guests_see_other_events_in_the_move_options_without_their_end() {
     assert!(
         html.contains(&format!(
             r#"<option value="event-{}">Started 11:00</option>"#,
-            morning.report.event_id
+            morning.report.event_id.unwrap()
         )),
         "{html}"
     );
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace`
Expected: compile errors — the tests compare `SmellReport.event_id` with `Option<EventId>`, build values without rooms, and match `LogError::CheckCannotSmell` / `LogError::NoSmellCannotOpen`, none of which exist yet.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/domain/src/error.rs b/crates/domain/src/error.rs
index 23dc31f..5708388 100644
--- a/crates/domain/src/error.rs
+++ b/crates/domain/src/error.rs
@@ -9,9 +9,6 @@ pub enum DomainError {
     #[error("a room can be listed only once in a report, got room {0} twice")]
     RoomListedTwice(i64),
 
-    #[error("a report needs at least one room")]
-    NoRooms,
-
     #[error("PM2.5 must be a number between 0 and 1000 µg/m³, got {0}")]
     InvalidPm25(f64),
 
diff --git a/crates/domain/src/events.rs b/crates/domain/src/events.rs
index 0641cf0..322edab 100644
--- a/crates/domain/src/events.rs
+++ b/crates/domain/src/events.rs
@@ -47,7 +47,8 @@ pub struct EventSummary {
 /// The question to ask before saving a report.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum LogPrompt {
-    /// Nothing to ask: the report starts a new event.
+    /// Nothing to ask: the report starts a new event or, if it is a no-smell report, joins the
+    /// open event or stands alone as a check.
     Direct,
     /// "Same smell still going?" about the open event.
     SameSmell(EventSummary),
@@ -72,6 +73,16 @@ pub fn log_prompt(
     }
 }
 
+/// The prompt for a no-smell report (spec §7.1, §7.2): none when logged now, since it joins the
+/// open event or stands alone; "Which event was this?" when backdated, with "no event" in place
+/// of a new one.
+pub fn no_smell_prompt(observed: ObservedTime) -> LogPrompt {
+    match observed {
+        ObservedTime::Now => LogPrompt::Direct,
+        ObservedTime::Backdated => LogPrompt::WhichEvent,
+    }
+}
+
 /// Whether a newly created event starts open or already closed.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum NewEventState {
@@ -262,6 +273,15 @@ mod tests {
         assert_eq!(log_prompt(ObservedTime::Now, None, None), LogPrompt::Direct);
     }
 
+    #[test]
+    fn a_no_smell_report_is_asked_only_which_event_when_backdated() {
+        assert_eq!(no_smell_prompt(ObservedTime::Now), LogPrompt::Direct);
+        assert_eq!(
+            no_smell_prompt(ObservedTime::Backdated),
+            LogPrompt::WhichEvent
+        );
+    }
+
     // --- new_event_state ---
 
     #[test]
diff --git a/crates/domain/src/history.rs b/crates/domain/src/history.rs
index ac3e14b..0824b36 100644
--- a/crates/domain/src/history.rs
+++ b/crates/domain/src/history.rs
@@ -28,9 +28,10 @@ pub enum Change {
         from: Option<String>,
         to: Option<String>,
     },
+    /// Into another event; `None` is no event (a no-smell check).
     Moved {
-        from: EventId,
-        to: EventId,
+        from: Option<EventId>,
+        to: Option<EventId>,
     },
 }
 
@@ -47,7 +48,7 @@ pub struct HistoryEntry {
 /// read through [`SmellReport::upgrade_snapshot`].
 #[derive(Deserialize)]
 struct Snapshot {
-    event_id: EventId,
+    event_id: Option<EventId>,
     observed_at: DateTime<Utc>,
     rooms: Vec<RoomLevel>,
     symptoms: Vec<SymptomId>,
@@ -170,7 +171,7 @@ mod tests {
     fn report(rooms: &[(i64, u8)]) -> SmellReport {
         SmellReport {
             id: ReportId(9),
-            event_id: EventId(4),
+            event_id: Some(EventId(4)),
             user_id: UserId(1),
             observed_at: t("2026-01-15T19:00:00Z"),
             recorded_at: t("2026-01-15T19:00:01Z"),
@@ -246,7 +247,7 @@ mod tests {
     fn a_move_names_both_events_and_is_not_a_correction() {
         let before = report(&[(1, 6)]);
         let mut after = before.clone();
-        after.event_id = EventId(5);
+        after.event_id = Some(EventId(5));
         let history = report_history(&[
             entry(AuditAction::Create, None, &before),
             entry(AuditAction::Move, Some(&before), &after),
@@ -254,8 +255,8 @@ mod tests {
         assert_eq!(
             history[1].changes,
             [Change::Moved {
-                from: EventId(4),
-                to: EventId(5),
+                from: Some(EventId(4)),
+                to: Some(EventId(5)),
             }]
         );
         assert!(!was_corrected(&history));
diff --git a/crates/domain/src/logbook.rs b/crates/domain/src/logbook.rs
index 04cb68e..e42965e 100644
--- a/crates/domain/src/logbook.rs
+++ b/crates/domain/src/logbook.rs
@@ -36,6 +36,12 @@ pub enum LogError {
     /// The time lies outside the event: before it started, or on another day.
     #[error("the time is outside the event")]
     OutsideEvent,
+    /// A no-smell report never opens an event (spec §7.1).
+    #[error("a no-smell report cannot start an event")]
+    NoSmellCannotOpen,
+    /// A check stays a no-smell report: a smell is logged as a new report (spec §7.1).
+    #[error("a check cannot be corrected to a smell")]
+    CheckCannotSmell,
     #[error(transparent)]
     Invalid(#[from] DomainError),
     #[error(transparent)]
@@ -43,11 +49,13 @@ pub enum LogError {
 }
 
 /// The answer to "Same smell still going?", "Which event was this?" or the auto-close notice.
+/// A no-smell report logged now ignores it: it joins the open event or stands alone.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum EventChoice {
     /// Start a new event: nothing was asked, "No, it had stopped", or "New event". `replacing`
     /// is the event that was open when the answer was given (`None` if none was), so an event
-    /// opened meanwhile by another phone is never closed by a stale answer.
+    /// opened meanwhile by another phone is never closed by a stale answer. For a backdated
+    /// no-smell report: no event, a check.
     New { replacing: Option<EventId> },
     /// Join this event: "Yes, update", or the event picked for a past report.
     Join(EventId),
@@ -108,7 +116,8 @@ async fn open_new_event<T: EventWrites>(
 }
 
 /// Logs a smell report, and its PM2.5 reading if there is one, into the event `choice` names
-/// (spec §7.1, §7.2, §7.4, §7.5).
+/// (spec §7.1, §7.2, §7.4, §7.5). A no-smell report never opens or closes an event: logged now
+/// it joins the open event, if any; backdated it joins the event picked, or none.
 pub async fn log_smell<T>(
     tx: &mut T,
     actor: &Actor,
@@ -126,8 +135,12 @@ where
     let observed_at = entry.observed_at;
     let observed = classify_observed(observed_at, now)?;
     let open = tx.open_event().await?;
-    let event_id = match choice {
-        EventChoice::New { replacing } => {
+    let event_id = match (entry.values.is_no_smell(), observed, choice) {
+        (true, ObservedTime::Now, _) => open.map(|event| event.id),
+        (true, ObservedTime::Backdated, EventChoice::New { .. }) => None,
+        (true, ObservedTime::Backdated, EventChoice::Continue(_)) => return Err(LogError::Stale),
+        (_, _, EventChoice::Join(id)) => Some(join(tx, id, observed, observed_at, schedule).await?),
+        (false, _, EventChoice::New { replacing }) => {
             if open.as_ref().map(|e| e.id) != replacing {
                 return Err(LogError::Stale);
             }
@@ -138,23 +151,9 @@ where
                 }
                 (_, open) => open.is_some(),
             };
-            open_new_event(tx, actor.id, observed_at, schedule, now, another_open).await?
+            Some(open_new_event(tx, actor.id, observed_at, schedule, now, another_open).await?)
         }
-        EventChoice::Join(id) => {
-            let event = tx.event(id).await?.ok_or(LogError::Stale)?;
-            match observed {
-                ObservedTime::Now if !event.is_open() => return Err(LogError::Stale),
-                ObservedTime::Now => {}
-                ObservedTime::Backdated => {
-                    if !can_join_backdated(&event, observed_at, schedule) {
-                        return Err(LogError::OutsideEvent);
-                    }
-                    extend_to(tx, &event, observed_at).await?;
-                }
-            }
-            id
-        }
-        EventChoice::Continue(id) => {
+        (false, _, EventChoice::Continue(id)) => {
             let event = tx.event(id).await?.ok_or(LogError::Stale)?;
             let auto_closed = event.closed.map(|c| c.reason) == Some(CloseReason::Auto0400);
             if observed != ObservedTime::Now || open.is_some() || !auto_closed {
@@ -170,7 +169,7 @@ where
                 return Err(LogError::Forbidden);
             }
             tx.reopen_event(id).await?;
-            id
+            Some(id)
         }
     };
     let report = tx
@@ -185,7 +184,7 @@ where
         Some(value) => Some(
             tx.create_reading(NewReading {
                 user_id: actor.id,
-                event_id: Some(event_id),
+                event_id,
                 observed_at,
                 value,
             })
@@ -196,6 +195,29 @@ where
     Ok(Logged { report, reading })
 }
 
+/// Joins the event `id` names: running now, or one the past report can belong to (its end moves
+/// if the report is later).
+async fn join<T: EventWrites>(
+    tx: &mut T,
+    id: EventId,
+    observed: ObservedTime,
+    observed_at: DateTime<Utc>,
+    schedule: &DailySchedule,
+) -> Result<EventId, LogError> {
+    let event = tx.event(id).await?.ok_or(LogError::Stale)?;
+    match observed {
+        ObservedTime::Now if !event.is_open() => return Err(LogError::Stale),
+        ObservedTime::Now => {}
+        ObservedTime::Backdated => {
+            if !can_join_backdated(&event, observed_at, schedule) {
+                return Err(LogError::OutsideEvent);
+            }
+            extend_to(tx, &event, observed_at).await?;
+        }
+    }
+    Ok(id)
+}
+
 /// Logs a PM2.5 reading on its own ("Log PM2.5 only", spec §7.6). It joins the event whose span
 /// contains its time, if any, and never opens, extends or closes an event.
 pub async fn log_reading<T>(
@@ -285,7 +307,8 @@ pub async fn reopen_event<T: EventWrites>(
 }
 
 /// "Correct a mistake" (spec §7.7): new values and observed time for a report. The time must
-/// still belong to the report's event; after a closed event's end, the end moves with it.
+/// still belong to the report's event; after a closed event's end, the end moves with it. A
+/// check (no event) keeps every level at 0 (spec §7.1).
 pub async fn correct_report<T: EventWrites + ReportWrites>(
     tx: &mut T,
     actor: &Actor,
@@ -297,16 +320,23 @@ pub async fn correct_report<T: EventWrites + ReportWrites>(
 ) -> Result<SmellReport, LogError> {
     let report = editable_report(tx, actor, id, now).await?;
     classify_observed(observed_at, now)?;
-    let event = tx.event(report.event_id).await?.ok_or(LogError::NotFound)?;
-    if !can_join_backdated(&event, observed_at, schedule) {
-        return Err(LogError::OutsideEvent);
+    match report.event_id {
+        Some(event_id) => {
+            let event = tx.event(event_id).await?.ok_or(LogError::NotFound)?;
+            if !can_join_backdated(&event, observed_at, schedule) {
+                return Err(LogError::OutsideEvent);
+            }
+            extend_to(tx, &event, observed_at).await?;
+        }
+        None if !values.is_no_smell() => return Err(LogError::CheckCannotSmell),
+        None => {}
     }
-    extend_to(tx, &event, observed_at).await?;
     Ok(tx.correct_report(id, observed_at, values).await?)
 }
 
 /// "Move to another event" (spec §7.7): into an event that was running when the report was
-/// observed (its end moves if needed), or into a new event owned by the report's author.
+/// observed (its end moves if needed), or into a new event owned by the report's author. A
+/// no-smell report never opens one (spec §7.1).
 pub async fn move_report<T: EventWrites + ReportWrites>(
     tx: &mut T,
     actor: &Actor,
@@ -317,7 +347,7 @@ pub async fn move_report<T: EventWrites + ReportWrites>(
 ) -> Result<SmellReport, LogError> {
     let report = editable_report(tx, actor, id, now).await?;
     let target = match to {
-        MoveTarget::Event(target) if target == report.event_id => return Ok(report),
+        MoveTarget::Event(target) if Some(target) == report.event_id => return Ok(report),
         MoveTarget::Event(target) => {
             let event = tx.event(target).await?.ok_or(LogError::NotFound)?;
             if !can_join_backdated(&event, report.observed_at, schedule) {
@@ -326,6 +356,7 @@ pub async fn move_report<T: EventWrites + ReportWrites>(
             extend_to(tx, &event, report.observed_at).await?;
             target
         }
+        MoveTarget::New if report.values.is_no_smell() => return Err(LogError::NoSmellCannotOpen),
         MoveTarget::New => {
             let another_open = tx.open_event().await?.is_some();
             let (owner, at) = (report.user_id, report.observed_at);
diff --git a/crates/domain/src/records.rs b/crates/domain/src/records.rs
index 2d1d194..d1d6c49 100644
--- a/crates/domain/src/records.rs
+++ b/crates/domain/src/records.rs
@@ -166,7 +166,8 @@ pub struct RoomLevel {
 }
 
 /// What a smell report says: the rooms checked, each with its level, sorted by room; the
-/// symptoms, sorted and unique; a note, where a blank note is no note.
+/// symptoms, sorted and unique; a note, where a blank note is no note. No rooms at all means
+/// "no smell anywhere in the home" (spec §7.1).
 #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
 pub struct ReportValues {
     rooms: Vec<RoomLevel>,
@@ -182,9 +183,6 @@ impl ReportValues {
         mut symptoms: Vec<SymptomId>,
         note: Option<&str>,
     ) -> Result<Self, DomainError> {
-        if rooms.is_empty() {
-            return Err(DomainError::NoRooms);
-        }
         rooms.sort_unstable_by_key(|r| r.room);
         if let Some(pair) = rooms.windows(2).find(|pair| pair[0].room == pair[1].room) {
             return Err(DomainError::RoomListedTwice(pair[0].room.0));
@@ -218,7 +216,13 @@ impl ReportValues {
         self.rooms.iter().find(|r| r.room == room).map(|r| r.level)
     }
 
-    /// The report's intensity: its highest room level (spec §6).
+    /// Whether this is a no-smell report: every room listed is at 0, or none is listed (spec
+    /// §7.1). Such a report never opens an event.
+    pub fn is_no_smell(&self) -> bool {
+        self.intensity().is_clean()
+    }
+
+    /// The report's intensity: its highest room level (spec §6); 0 without rooms.
     pub fn intensity(&self) -> Level {
         self.rooms
             .iter()
@@ -239,7 +243,8 @@ impl ReportValues {
 #[derive(Debug, Clone, PartialEq, Serialize)]
 pub struct SmellReport {
     pub id: ReportId,
-    pub event_id: EventId,
+    /// `None` for a no-smell check made while no event was running (spec §7.1).
+    pub event_id: Option<EventId>,
     pub user_id: UserId,
     pub observed_at: DateTime<Utc>,
     pub recorded_at: DateTime<Utc>,
@@ -342,7 +347,8 @@ pub struct NewEvent {
 
 #[derive(Debug, Clone, PartialEq)]
 pub struct NewReport {
-    pub event_id: EventId,
+    /// `None` for a no-smell check outside any event.
+    pub event_id: Option<EventId>,
     pub user_id: UserId,
     pub observed_at: DateTime<Utc>,
     pub values: ReportValues,
@@ -449,11 +455,7 @@ mod tests {
     }
 
     #[test]
-    fn a_report_needs_a_room_and_lists_each_room_once() {
-        assert_eq!(
-            ReportValues::new(vec![], vec![], None),
-            Err(DomainError::NoRooms)
-        );
+    fn a_report_lists_each_room_once() {
         assert_eq!(
             ReportValues::new(vec![at(3, 2), at(1, 1), at(3, 2)], vec![], None),
             Err(DomainError::RoomListedTwice(3))
@@ -469,6 +471,13 @@ mod tests {
         assert_eq!(values(&[(1, 0)], None).intensity(), Level::CLEAN);
     }
 
+    #[test]
+    fn only_clean_rooms_or_no_rooms_make_a_no_smell_report() {
+        assert!(values(&[(1, 0), (2, 0)], None).is_no_smell());
+        assert!(values(&[], None).is_no_smell(), "no rooms: the whole home");
+        assert!(!values(&[(1, 0), (2, 1)], None).is_no_smell());
+    }
+
     #[test]
     fn a_blank_note_becomes_none() {
         assert_eq!(values(&[(1, 1)], Some("   ")).note(), None);
@@ -490,7 +499,7 @@ mod tests {
     fn report_snapshot_is_flat() {
         let report = SmellReport {
             id: ReportId(9),
-            event_id: EventId(4),
+            event_id: Some(EventId(4)),
             user_id: UserId(1),
             observed_at: t("2026-01-15T19:00:00Z"),
             recorded_at: t("2026-01-15T19:00:00.123456Z"),
@@ -512,6 +521,13 @@ mod tests {
             })
         );
         assert_eq!(report.audit_id(), 9);
+        let check = SmellReport {
+            event_id: None,
+            values: values(&[], None),
+            ..report
+        };
+        assert_eq!(check.snapshot()["event_id"], json!(null));
+        assert_eq!(check.snapshot()["rooms"], json!([]));
     }
 
     #[test]
diff --git a/crates/domain/src/summary.rs b/crates/domain/src/summary.rs
index edaddb4..2093512 100644
--- a/crates/domain/src/summary.rs
+++ b/crates/domain/src/summary.rs
@@ -1,5 +1,5 @@
 //! The summary card on the Log screen (spec §9, screen 2): "This month: 14 events · 41 reports
-//! · 9 days with bedroom smoke."
+//! · 9 days with bedroom smoke · 6 clean checks."
 
 use std::collections::BTreeSet;
 
@@ -11,7 +11,10 @@ use crate::types::RoomId;
 pub struct MonthSummary {
     /// Events with at least one of the reports.
     pub events: usize,
+    /// Reports in an event.
     pub reports: usize,
+    /// No-smell checks made while no event was running.
+    pub checks: usize,
     /// Local days with a report giving the bedroom a level of 1 or more; `None` when there is no
     /// bedroom room.
     pub bedroom_days: Option<usize>,
@@ -24,7 +27,8 @@ pub fn summarise(
     bedroom: Option<RoomId>,
     schedule: &DailySchedule,
 ) -> MonthSummary {
-    let events: BTreeSet<_> = reports.iter().map(|r| r.event_id).collect();
+    let events: BTreeSet<_> = reports.iter().filter_map(|r| r.event_id).collect();
+    let checks = reports.iter().filter(|r| r.event_id.is_none()).count();
     let bedroom_days = bedroom.map(|room| {
         reports
             .iter()
@@ -39,7 +43,8 @@ pub fn summarise(
     });
     MonthSummary {
         events: events.len(),
-        reports: reports.len(),
+        reports: reports.len() - checks,
+        checks,
         bedroom_days,
     }
 }
@@ -52,11 +57,12 @@ mod tests {
     use crate::records::{ReportValues, RoomLevel};
     use crate::types::{EventId, Level, ReportId, UserId};
 
+    /// Event 0 stands for no event: a no-smell check.
     fn report(id: i64, event: i64, at: &str, rooms: &[(i64, u8)]) -> SmellReport {
         let at: DateTime<Utc> = at.parse().unwrap();
         SmellReport {
             id: ReportId(id),
-            event_id: EventId(event),
+            event_id: (event != 0).then_some(EventId(event)),
             user_id: UserId(1),
             observed_at: at,
             recorded_at: at,
@@ -81,7 +87,7 @@ mod tests {
     }
 
     #[test]
-    fn counts_events_reports_and_local_days_with_bedroom_smoke() {
+    fn counts_events_reports_checks_and_local_days_with_bedroom_smoke() {
         let bedroom = RoomId(3);
         let reports = [
             report(1, 1, "2026-01-10T20:00:00Z", &[(3, 4)]),
@@ -91,12 +97,15 @@ mod tests {
             report(4, 2, "2026-01-12T08:00:00Z", &[(1, 6)]),
             // The bedroom checked and clean: not a day with bedroom smoke.
             report(5, 2, "2026-01-12T09:00:00Z", &[(1, 6), (3, 0)]),
+            report(6, 0, "2026-01-13T09:00:00Z", &[(3, 0)]),
+            report(7, 0, "2026-01-14T09:00:00Z", &[]),
         ];
         assert_eq!(
             summarise(&reports, Some(bedroom), &rome()),
             MonthSummary {
                 events: 2,
                 reports: 5,
+                checks: 2,
                 bedroom_days: Some(2),
             }
         );
@@ -118,6 +127,7 @@ mod tests {
             MonthSummary {
                 events: 0,
                 reports: 0,
+                checks: 0,
                 bedroom_days: Some(0),
             }
         );
diff --git a/crates/storage-postgres/migrations/0004_checks.sql b/crates/storage-postgres/migrations/0004_checks.sql
new file mode 100644
index 0000000..92c1d12
--- /dev/null
+++ b/crates/storage-postgres/migrations/0004_checks.sql
@@ -0,0 +1,5 @@
+-- A no-smell report made while no event is running is kept as a check outside any event
+-- (spec §7.1), so a smell report's event becomes optional.
+-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.
+
+ALTER TABLE smell_reports ALTER COLUMN event_id DROP NOT NULL;
diff --git a/crates/storage-postgres/src/reports.rs b/crates/storage-postgres/src/reports.rs
index 27d6038..006334f 100644
--- a/crates/storage-postgres/src/reports.rs
+++ b/crates/storage-postgres/src/reports.rs
@@ -15,7 +15,7 @@ use crate::{PostgresStore, PostgresTx};
 
 struct ReportRow {
     id: i64,
-    event_id: i64,
+    event_id: Option<i64>,
     user_id: i64,
     observed_at: i64,
     recorded_at: i64,
@@ -71,7 +71,7 @@ fn assemble(
     }
     Ok(SmellReport {
         id: ReportId(row.id),
-        event_id: EventId(row.event_id),
+        event_id: row.event_id.map(EventId),
         user_id: UserId(row.user_id),
         observed_at: instant(row.observed_at)?,
         recorded_at: instant(row.recorded_at)?,
@@ -278,7 +278,7 @@ impl ReportWrites for PostgresTx {
         let id = sqlx::query_scalar!(
             "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, note) \
              VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
-            new.event_id.0,
+            new.event_id.map(|event| event.0),
             new.user_id.0,
             micros(new.observed_at),
             now,
@@ -316,7 +316,7 @@ impl ReportWrites for PostgresTx {
 
     async fn move_report(&mut self, id: ReportId, to: EventId) -> StoreResult<SmellReport> {
         let before = require_live(self.conn(), id).await?;
-        if before.event_id == to {
+        if before.event_id == Some(to) {
             return Ok(before);
         }
         sqlx::query!(
diff --git a/crates/storage-sqlite/migrations/0004_checks.sql b/crates/storage-sqlite/migrations/0004_checks.sql
new file mode 100644
index 0000000..f902b5d
--- /dev/null
+++ b/crates/storage-sqlite/migrations/0004_checks.sql
@@ -0,0 +1,46 @@
+-- A no-smell report made while no event is running is kept as a check outside any event
+-- (spec §7.1), so a smell report's event becomes optional. SQLite cannot drop NOT NULL in
+-- place: the reports table is rebuilt, and its link tables with it, since they refer to it.
+
+CREATE TABLE smell_reports_0004 (
+    id          INTEGER PRIMARY KEY,
+    event_id    INTEGER REFERENCES events (id),
+    user_id     INTEGER NOT NULL REFERENCES users (id),
+    observed_at INTEGER NOT NULL,
+    recorded_at INTEGER NOT NULL,
+    note        TEXT,
+    deleted_at  INTEGER
+);
+
+INSERT INTO smell_reports_0004 (id, event_id, user_id, observed_at, recorded_at, note, deleted_at)
+SELECT id, event_id, user_id, observed_at, recorded_at, note, deleted_at FROM smell_reports;
+
+CREATE TABLE smell_report_rooms_0004 (
+    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0004 (id),
+    room_id         INTEGER NOT NULL REFERENCES rooms (id),
+    level           INTEGER NOT NULL CHECK (level BETWEEN 0 AND 10),
+    PRIMARY KEY (smell_report_id, room_id)
+);
+
+INSERT INTO smell_report_rooms_0004 (smell_report_id, room_id, level)
+SELECT smell_report_id, room_id, level FROM smell_report_rooms;
+
+CREATE TABLE smell_report_symptoms_0004 (
+    smell_report_id INTEGER NOT NULL REFERENCES smell_reports_0004 (id),
+    symptom_id      INTEGER NOT NULL REFERENCES symptoms (id),
+    PRIMARY KEY (smell_report_id, symptom_id)
+);
+
+INSERT INTO smell_report_symptoms_0004 (smell_report_id, symptom_id)
+SELECT smell_report_id, symptom_id FROM smell_report_symptoms;
+
+DROP TABLE smell_report_rooms;
+DROP TABLE smell_report_symptoms;
+DROP TABLE smell_reports;
+
+-- Renaming also rewrites the link tables' references to the new name.
+ALTER TABLE smell_reports_0004 RENAME TO smell_reports;
+ALTER TABLE smell_report_rooms_0004 RENAME TO smell_report_rooms;
+ALTER TABLE smell_report_symptoms_0004 RENAME TO smell_report_symptoms;
+
+CREATE INDEX smell_reports_event ON smell_reports (event_id, observed_at);
diff --git a/crates/storage-sqlite/src/reports.rs b/crates/storage-sqlite/src/reports.rs
index 74bf0e9..25bd9b2 100644
--- a/crates/storage-sqlite/src/reports.rs
+++ b/crates/storage-sqlite/src/reports.rs
@@ -15,7 +15,7 @@ use crate::{SqliteStore, SqliteTx};
 
 struct ReportRow {
     id: i64,
-    event_id: i64,
+    event_id: Option<i64>,
     user_id: i64,
     observed_at: i64,
     recorded_at: i64,
@@ -71,7 +71,7 @@ fn assemble(
     }
     Ok(SmellReport {
         id: ReportId(row.id),
-        event_id: EventId(row.event_id),
+        event_id: row.event_id.map(EventId),
         user_id: UserId(row.user_id),
         observed_at: instant(row.observed_at)?,
         recorded_at: instant(row.recorded_at)?,
@@ -285,7 +285,7 @@ impl ReportWrites for SqliteTx {
         let id = sqlx::query_scalar!(
             "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, note) \
              VALUES ($1, $2, $3, $4, $5) RETURNING id AS \"id!\"",
-            new.event_id.0,
+            new.event_id.map(|event| event.0),
             new.user_id.0,
             micros(new.observed_at),
             now,
@@ -323,7 +323,7 @@ impl ReportWrites for SqliteTx {
 
     async fn move_report(&mut self, id: ReportId, to: EventId) -> StoreResult<SmellReport> {
         let before = require_live(self.conn(), id).await?;
-        if before.event_id == to {
+        if before.event_id == Some(to) {
             return Ok(before);
         }
         sqlx::query!(
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 2650a62..e594932 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -59,6 +59,8 @@ prompt-cancel = Cancel
 refused-stale = The events changed in the meantime. Check and try again.
 refused-another-open = Another event is open. Close it first.
 refused-outside-event = That time is outside the event: before it started, or on another day.
+refused-no-smell-new-event = A report with no smell cannot start an event.
+refused-check-smell = This is a no-smell check, so its levels stay at 0. To record a smell, log a new report.
 refused-future = The time cannot be in the future.
 refused-close-before-last-report = The end cannot be before the last report.
 refused-close-in-future = The end cannot be in the future.
@@ -126,6 +128,7 @@ event-reopen = Reopen
 
 report-title = Smell report
 report-event = Event
+report-no-event = None: a no-smell check
 report-observed = Observed
 report-recorded = Recorded
 report-note = Note
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 7c1bb7d..f1beb0c 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -59,6 +59,8 @@ prompt-cancel = Annulla
 refused-stale = Nel frattempo gli eventi sono cambiati. Controlla e riprova.
 refused-another-open = C'è un altro evento aperto. Chiudilo prima.
 refused-outside-event = Quell'ora è fuori dall'evento: prima che iniziasse o in un altro giorno.
+refused-no-smell-new-event = Un rapporto senza odore non può iniziare un evento.
+refused-check-smell = Questo è un controllo senza odore: i livelli restano a 0. Per registrare un odore, registra un nuovo rapporto.
 refused-future = L'ora non può essere nel futuro.
 refused-close-before-last-report = La fine non può essere prima dell'ultima segnalazione.
 refused-close-in-future = La fine non può essere nel futuro.
@@ -126,6 +128,7 @@ event-reopen = Riapri
 
 report-title = Segnalazione
 report-event = Evento
+report-no-event = Nessuno: un controllo senza odore
 report-observed = Osservata
 report-recorded = Registrata
 report-note = Nota
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 00ced4f..679717d 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -59,6 +59,8 @@ prompt-cancel = Vazgeç
 refused-stale = Bu arada olaylar değişti. Kontrol edip tekrar dene.
 refused-another-open = Açık başka bir olay var. Önce onu kapat.
 refused-outside-event = Bu saat olayın dışında: başlamadan önce ya da başka bir günde.
+refused-no-smell-new-event = Kokusuz bir rapor olay başlatamaz.
+refused-check-smell = Bu kokusuz bir kontrol, seviyeleri 0 kalır. Bir koku kaydetmek için yeni bir rapor ekle.
 refused-future = Saat gelecekte olamaz.
 refused-close-before-last-report = Bitiş, son bildirimden önce olamaz.
 refused-close-in-future = Bitiş gelecekte olamaz.
@@ -126,6 +128,7 @@ event-reopen = Yeniden aç
 
 report-title = Koku bildirimi
 report-event = Olay
+report-no-event = Yok: kokusuz bir kontrol
 report-observed = Gözlem
 report-recorded = Kayıt
 report-note = Not
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index 4f3d1c4..43ec682 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -116,7 +116,9 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
         .reports_between(window_start, window_end + TimeDelta::microseconds(1))
         .await?
     {
-        by_event.entry(report.event_id).or_default().push(report);
+        if let Some(event) = report.event_id {
+            by_event.entry(event).or_default().push(report);
+        }
     }
 
     let mut days: BTreeMap<NaiveDate, Day> = BTreeMap::new();
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index 050ff33..cfb3d6d 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -41,6 +41,8 @@ pub(crate) fn refusal(error: LogError) -> Result<&'static str, AppError> {
         LogError::Stale => Ok("refused-stale"),
         LogError::AnotherEventOpen => Ok("refused-another-open"),
         LogError::OutsideEvent => Ok("refused-outside-event"),
+        LogError::NoSmellCannotOpen => Ok("refused-no-smell-new-event"),
+        LogError::CheckCannotSmell => Ok("refused-check-smell"),
         LogError::Invalid(DomainError::ObservedInFuture) => Ok("refused-future"),
         LogError::Invalid(DomainError::CloseBeforeLastReport) => {
             Ok("refused-close-before-last-report")
@@ -53,10 +55,12 @@ pub(crate) fn refusal(error: LogError) -> Result<&'static str, AppError> {
 
 /// Every key [`refusal`] can return, for the translation tests.
 #[cfg(test)]
-pub(crate) const REFUSALS: [&str; 6] = [
+pub(crate) const REFUSALS: [&str; 8] = [
     "refused-stale",
     "refused-another-open",
     "refused-outside-event",
+    "refused-no-smell-new-event",
+    "refused-check-smell",
     "refused-future",
     "refused-close-before-last-report",
     "refused-close-in-future",
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index 5d2cd7a..3ae3163 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -56,6 +56,11 @@ struct HistoryLine {
     changes: Vec<String>,
 }
 
+struct EventLink {
+    id: i64,
+    date: String,
+}
+
 struct MoveOption {
     value: String,
     label: String,
@@ -67,8 +72,8 @@ struct ReportPage {
     tr: Tr,
     signed_in: Option<String>,
     id: i64,
-    event_id: i64,
-    event_date: String,
+    /// `None` for a no-smell check outside any event.
+    event: Option<EventLink>,
     observed: String,
     recorded: String,
     intensity: String,
@@ -125,11 +130,10 @@ async fn report_page<S: Store>(
     let tz = schedule.tz();
     let actor = user.actor();
     let report = visible_report(state, user, id).await?;
-    let event = state
-        .store
-        .event(report.event_id)
-        .await?
-        .ok_or(AppError::NotFound)?;
+    let event = match report.event_id {
+        Some(id) => Some(state.store.event(id).await?.ok_or(AppError::NotFound)?),
+        None => None,
+    };
     let rooms = state.store.rooms().await?;
     let symptoms = state.store.symptoms().await?;
     let sees_all = can(&actor, Action::ViewAll, now);
@@ -197,7 +201,14 @@ async fn report_page<S: Store>(
                     ),
                     Change::Note { .. } => tr.t("history-note"),
                     Change::Moved { from, to } => {
-                        tr.t1("history-moved", "change", &format!("#{from} → #{to}"))
+                        let event = |id: &Option<EventId>| {
+                            id.map_or("—".to_owned(), |id| format!("#{id}"))
+                        };
+                        tr.t1(
+                            "history-moved",
+                            "change",
+                            &format!("{} → {}", event(from), event(to)),
+                        )
                     }
                 })
                 .collect(),
@@ -214,7 +225,7 @@ async fn report_page<S: Store>(
         );
     let mut move_options = Vec::new();
     if editable {
-        let joinable = joinable_events(state, user, tr, report.observed_at, Some(report.event_id));
+        let joinable = joinable_events(state, user, tr, report.observed_at, report.event_id);
         for (id, label) in joinable.await? {
             move_options.push(MoveOption {
                 value: format!("event-{id}"),
@@ -240,8 +251,10 @@ async fn report_page<S: Store>(
         tr,
         signed_in: Some(user.username.to_string()),
         id: id.0,
-        event_id: report.event_id.0,
-        event_date: time::date(tr.0, schedule.local_date(event.opened_at)),
+        event: event.map(|event| EventLink {
+            id: event.id.0,
+            date: time::date(tr.0, schedule.local_date(event.opened_at)),
+        }),
         observed: time::date_time(tr.0, report.observed_at, tz),
         recorded: time::date_time(tr.0, report.recorded_at, tz),
         intensity: format!(
diff --git a/crates/web/templates/report.html b/crates/web/templates/report.html
index b2b1127..3da73cb 100644
--- a/crates/web/templates/report.html
+++ b/crates/web/templates/report.html
@@ -7,7 +7,7 @@
 {% endif %}
 <dl class="facts">
   <dt>{{ tr.t("report-event") }}</dt>
-  <dd><a href="/events/{{ event_id }}">{{ tr.t1("event-title", "date", event_date) }}</a></dd>
+  <dd>{% if let Some(event) = event %}<a href="/events/{{ event.id }}">{{ tr.t1("event-title", "date", event.date) }}</a>{% else %}{{ tr.t("report-no-event") }}{% endif %}</dd>
   <dt>{{ tr.t("report-observed") }}</dt>
   <dd>{{ observed }}</dd>
   <dt>{{ tr.t("report-recorded") }}</dt>
```

Then regenerate the offline query data (the `query!` texts changed):

```sh
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres scripts/sqlx-prepare.sh
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres scripts/sqlx-prepare.sh --check
```
Expected: no formatting changes, no clippy warnings; every test passes, including the five new contract tests on both backends, `after_the_rebuild_the_link_tables_still_refer_to_the_reports` and `only_clean_rooms_or_no_rooms_make_a_no_smell_report`. With `TEST_POSTGRES_URL` set: 475 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/error.rs \
  crates/domain/src/events.rs \
  crates/domain/src/history.rs \
  crates/domain/src/logbook.rs \
  crates/domain/src/records.rs \
  crates/domain/src/summary.rs \
  crates/storage-contract/src/audit.rs \
  crates/storage-contract/src/corrections.rs \
  crates/storage-contract/src/fixture.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/logbook.rs \
  crates/storage-contract/src/migrations.rs \
  crates/storage-contract/src/reports.rs \
  crates/storage-contract/src/sweep.rs \
  crates/storage-postgres/migrations/0004_checks.sql \
  crates/storage-postgres/src/reports.rs \
  crates/storage-sqlite/migrations/0004_checks.sql \
  crates/storage-sqlite/src/reports.rs \
  crates/storage-sqlite/tests/contract.rs \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/reports.rs \
  crates/web/templates/report.html \
  crates/web/tests/events.rs \
  crates/web/tests/log.rs \
  crates/web/tests/logging.rs \
  crates/web/tests/reports.rs \
  crates/storage-postgres/.sqlx \
  crates/storage-sqlite/.sqlx
git commit -m "feat: a no-smell report never opens an event; with none running it is a check

A report whose rooms are all at 0, or that lists no rooms (the whole
home), joins the open event when logged now and otherwise stands alone:
SmellReport.event_id becomes Option. Migration 0004 makes the column
nullable (SQLite rebuilds the reports and link tables). A check can be
corrected but never to a smell, moved into an event, never into a new one.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Web — No smell, checks in the Events list and the summary card

The screens for checks (spec §9 screens 1–4, decisions #105, #107, #108): a **No smell** button
for everyone who may log (guests too) that records the selected rooms at 0 or, with none
selected, the whole home; no question when a no-smell report is logged now; "No event: just a
check" in place of "New event" when one is backdated; a saved message; "N clean checks" on the
summary card; each day's checks in the Events list (guests see their own); "whole home" for a
report without rooms; a check's page offers no new event to move to and its correction form
explains that it stays at 0.

**Files:**
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/labels.rs`, `crates/web/src/routes/entries.rs`, `crates/web/src/routes/events.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/reports.rs`, `crates/web/templates/correct.html`, `crates/web/templates/events.html`, `crates/web/templates/log.html`, `crates/web/templates/report.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/events.rs`, `crates/web/tests/log.rs`, `crates/web/tests/logging.rs`, `crates/web/tests/reports.rs`

**Interfaces:**
- Consumes: Task 2's `is_no_smell`, `no_smell_prompt`, `MonthSummary.checks`, optional
  `event_id`, `LogError::CheckCannotSmell`.
- Produces: `entries::Smell { Levels, None, Correction }` and
  `read_values(&Pairs, &[RoomId], &[SymptomId], Smell)` (`Correction` lets a correction leave no
  room selected: the whole home, clean); the form action `no-smell`; the redirect
  `/?saved=no-smell`; `labels::report_rooms(Tr, &[Room], &ReportValues) -> String`;
  `CorrectPage.check: bool`; `TestApp::no_smell(&User, &str) -> Logged` in the web tests.
  Translation keys added: `log-no-smell`, `log-saved-no-smell`, `log-error-no-smell-level`,
  `prompt-no-event`, `summary-checks`, `events-check`, `report-whole-home`,
  `correct-check-hint`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diffs, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/web/tests/common/mod.rs b/crates/web/tests/common/mod.rs
index 362be7d..442059c 100644
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -118,17 +118,29 @@ impl TestApp {
             [] => vec![self.room("room-living-room").await],
             rooms => rooms.to_vec(),
         };
+        let rooms = rooms
+            .into_iter()
+            .map(|room| RoomLevel { room, level })
+            .collect();
+        self.log_values(user, at, rooms, choice).await
+    }
+
+    /// Logs "No smell" for the whole home through the domain, at `at` (RFC 3339).
+    pub async fn no_smell(&self, user: &User, at: &str) -> Logged {
+        let choice = EventChoice::New { replacing: None };
+        self.log_values(user, at, vec![], choice).await
+    }
+
+    async fn log_values(
+        &self,
+        user: &User,
+        at: &str,
+        rooms: Vec<RoomLevel>,
+        choice: EventChoice,
+    ) -> Logged {
         let entry = SmellEntry {
             observed_at: t(at),
-            values: ReportValues::new(
-                rooms
-                    .into_iter()
-                    .map(|room| RoomLevel { room, level })
-                    .collect(),
-                vec![],
-                None,
-            )
-            .unwrap(),
+            values: ReportValues::new(rooms, vec![], None).unwrap(),
             pm25: None,
         };
         let mut tx = self.store.begin(Some(user.id)).await.unwrap();
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index 92f93ec..225b2db 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -455,3 +455,35 @@ async fn closing_or_reopening_an_event_a_guest_cannot_see_answers_not_found() {
     assert_eq!(reopen.status(), StatusCode::NOT_FOUND);
     assert!(!app.store.event(id).await.unwrap().unwrap().is_open());
 }
+
+#[tokio::test]
+async fn the_list_shows_each_days_checks_to_those_who_may_see_them() {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    let mine = app.no_smell(&me, "2026-01-12T08:00:00Z").await;
+    let theirs = app.no_smell(&guest, "2026-01-13T08:00:00Z").await;
+
+    let resident = app.sign_in("daniele").await;
+    let html = text(app.send(get_with("/events?month=2026-01", &resident)).await).await;
+    for (check, at) in [(&mine, "09:00"), (&theirs, "09:00")] {
+        let row = format!(
+            r#"<a href="/reports/{}">{at} · no smell (check)</a>"#,
+            check.report.id
+        );
+        assert!(html.contains(&row), "{html}");
+    }
+    assert!(!html.contains("No events this month."), "{html}");
+
+    let cookie = app.sign_in("guest").await;
+    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
+    assert!(
+        html.contains(&format!("/reports/{}", theirs.report.id)),
+        "{html}"
+    );
+    assert!(
+        !html.contains(&format!("/reports/{}", mine.report.id)),
+        "{html}"
+    );
+}
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index 6744155..e2f30b5 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -40,6 +40,11 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
     assert!(html.contains(r#"name="symptom""#) && html.contains("Eye irritation"));
     assert!(html.contains(">Log smell</button>"), "{html}");
     assert!(html.contains(">Log PM2.5 only</button>"), "{html}");
+    assert!(
+        html.contains(r#"value="no-smell" formnovalidate class="secondary">No smell</button>"#),
+        "{html}"
+    );
+    assert!(html.contains(" · 0 clean checks"), "{html}");
 }
 
 #[tokio::test]
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index 05407f6..ac94b55 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -6,7 +6,7 @@ use axum::http::StatusCode;
 use common::*;
 use smoke_domain::logbook::EventChoice;
 use smoke_domain::repos::{EventRepo, ReadingRepo, ReportRepo};
-use smoke_domain::types::{CloseReason, EventId, Locale, Role};
+use smoke_domain::types::{CloseReason, EventId, Locale, ReportId, Role};
 
 /// The value of the hidden `asked` field on a question page.
 fn asked(html: &str) -> String {
@@ -479,3 +479,101 @@ async fn viewers_cannot_log() {
     assert!(text(response).await.contains("<h1>Not allowed</h1>"));
     assert_eq!(app.store.event(EventId(1)).await.unwrap(), None);
 }
+
+#[tokio::test]
+async fn no_smell_with_nothing_open_saves_a_check_without_asking() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let cookie = app.sign_in("guest").await;
+
+    let response = app
+        .send(post_form("/log", "action=no-smell", Some(&cookie)))
+        .await;
+    assert_eq!(location(&response), Some("/?saved=no-smell"));
+    assert_eq!(app.store.open_event().await.unwrap(), None);
+    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
+    assert_eq!(check.event_id, None);
+    assert!(check.values.rooms().is_empty(), "the whole home");
+    let html = text(app.send(get_with("/?saved=no-smell", &cookie)).await).await;
+    assert!(html.contains("Saved: no smell."), "{html}");
+    assert!(html.contains(" · 1 clean check"), "{html}");
+}
+
+#[tokio::test]
+async fn no_smell_with_an_event_open_joins_it_without_the_question() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let first = app
+        .log(
+            &me,
+            "2026-01-15T19:00:00Z",
+            6,
+            &[],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let cookie = app.sign_in("guest").await;
+    let bedroom = app.room("room-bedroom").await;
+
+    let body = format!("room={0}&level-{0}=0&action=smell", bedroom.0);
+    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
+    assert_eq!(location(&response), Some("/?saved=no-smell"));
+    let open = app.store.open_event().await.unwrap().unwrap();
+    assert_eq!(Some(open.id), first.report.event_id, "still open");
+    let reports = app.store.reports_for_event(open.id).await.unwrap();
+    assert_eq!(reports.len(), 2);
+    assert!(reports[1].values.is_no_smell());
+}
+
+#[tokio::test]
+async fn a_past_no_smell_report_is_offered_no_event_instead_of_a_new_one() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+
+    let body = "action=no-smell&observed_at=2026-01-15T12:00";
+    let question = text(app.send(post_form("/log", body, Some(&cookie))).await).await;
+    assert!(
+        question.contains("<h1>Which event was this?</h1>"),
+        "{question}"
+    );
+    assert!(
+        question.contains(">No event: just a check</button>"),
+        "{question}"
+    );
+    assert!(!question.contains(">New event</button>"), "{question}");
+
+    let answer = format!("{body}&asked={}&choice=new", asked(&question));
+    let response = app.send(post_form("/log", &answer, Some(&cookie))).await;
+    assert_eq!(location(&response), Some("/?saved=no-smell"));
+    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
+    assert_eq!(check.event_id, None);
+    assert_eq!(app.store.event(EventId(1)).await.unwrap(), None);
+}
+
+#[tokio::test]
+async fn no_smell_records_selected_rooms_at_zero_and_refuses_a_higher_level() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let bedroom = app.room("room-bedroom").await;
+
+    let body = format!("room={0}&level-{0}=4&action=no-smell", bedroom.0);
+    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
+    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+    assert!(
+        text(response)
+            .await
+            .contains("No smell records the selected rooms at 0.")
+    );
+
+    let body = format!("room={}&action=no-smell", bedroom.0);
+    app.send(post_form("/log", &body, Some(&cookie))).await;
+    let check = app.store.report(ReportId(1)).await.unwrap().unwrap();
+    assert_eq!(check.values.level_in(bedroom).map(|l| l.get()), Some(0));
+}
diff --git a/crates/web/tests/reports.rs b/crates/web/tests/reports.rs
index 68398ab..a85ad4a 100644
--- a/crates/web/tests/reports.rs
+++ b/crates/web/tests/reports.rs
@@ -382,3 +382,56 @@ async fn guests_see_other_events_in_the_move_options_without_their_end() {
         "a guest never learns when another event ended: {html}"
     );
 }
+
+#[tokio::test]
+async fn a_check_has_no_event_and_cannot_be_corrected_to_a_smell() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let check = app.no_smell(&me, "2026-01-15T19:00:00Z").await.report;
+    let cookie = app.sign_in("daniele").await;
+    let id = check.id;
+
+    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
+    assert!(html.contains("<dd>None: a no-smell check</dd>"), "{html}");
+    assert!(html.contains("<dd>whole home</dd>"), "{html}");
+    assert!(html.contains("<dd>0 · Clean</dd>"), "{html}");
+    assert!(
+        !html.contains(r#"value="new""#),
+        "a check never opens an event: {html}"
+    );
+
+    let form = text(
+        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
+            .await,
+    )
+    .await;
+    assert!(form.contains("This is a no-smell check"), "{form}");
+    let bedroom = app.room("room-bedroom").await;
+    let body = format!(
+        "room={0}&level-{0}=3&observed_at=2026-01-15T20:00",
+        bedroom.0
+    );
+    let refused = app
+        .send(post_form(
+            &format!("/reports/{id}/correct"),
+            &body,
+            Some(&cookie),
+        ))
+        .await;
+    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
+    assert!(text(refused).await.contains("its levels stay at 0"));
+
+    let body = format!(
+        "room={0}&level-{0}=0&observed_at=2026-01-15T20:00",
+        bedroom.0
+    );
+    let response = app
+        .send(post_form(
+            &format!("/reports/{id}/correct"),
+            &body,
+            Some(&cookie),
+        ))
+        .await;
+    assert_eq!(location(&response), Some(&*format!("/reports/{id}")));
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --workspace`
Expected: the new web tests compile but fail: `action=no-smell` gets the form back asking for a room, there is no "clean check" text and no check in the Events list, and a check's page still offers "New event".

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index e594932..0154e94 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -30,12 +30,15 @@ log-note = Note (optional)
 log-submit = Log smell
 log-update = Update: situation changed
 log-pm25-only = Log PM2.5 only
+log-no-smell = No smell
 log-read-only = You can look at the data but not log.
 
 log-saved-report = Report saved.
 log-saved-pm25 = PM2.5 reading saved.
+log-saved-no-smell = Saved: no smell.
 log-error-rooms = Pick at least one room.
 log-error-level = Pick a level from 0 to 10 for each selected room.
+log-error-no-smell-level = No smell records the selected rooms at 0. Remove the levels above 0, or use Log smell.
 log-error-pm25 = PM2.5 must be a number from 0 to 1000.
 log-error-pm25-missing = Enter a PM2.5 value to log it on its own.
 log-error-time = Enter a valid date and time.
@@ -52,6 +55,7 @@ prompt-which-event = Which event was this?
 prompt-started-at = Started { $time }
 prompt-ended-at = ended { $time }
 prompt-new-event = New event
+prompt-no-event = No event: just a check
 prompt-auto-closed = The event from { $when } was auto-closed.
 prompt-continue = It was still going: continuation
 prompt-cancel = Cancel
@@ -78,6 +82,10 @@ summary-bedroom-days = { $count ->
     [one] 1 day with bedroom smoke
    *[other] { $count } days with bedroom smoke
 }
+summary-checks = { $count ->
+    [one] 1 clean check
+   *[other] { $count } clean checks
+}
 
 rating-title = How to rate
 rating-level = Level
@@ -113,6 +121,7 @@ nav-events = Events
 
 events-title = Events
 events-none = No events this month.
+events-check = no smell (check)
 events-previous = ← Previous
 events-next = Next →
 events-in-progress = in progress
@@ -134,6 +143,7 @@ report-recorded = Recorded
 report-note = Note
 report-by = Logged by
 report-none = none
+report-whole-home = whole home
 report-edited = edited
 report-deleted = deleted
 report-correct = Correct a mistake
@@ -150,6 +160,7 @@ history-removed = removed { $name }
 history-note = note changed
 history-moved = event { $change }
 correct-hint = Correct only mistakes. If the situation changed, log a new report instead.
+correct-check-hint = This is a no-smell check: every level stays at 0. If you smelled something, log a new report instead.
 correct-time = Observed at
 correct-submit = Save the correction
 
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index f1beb0c..f3272dd 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -30,12 +30,15 @@ log-note = Nota (facoltativa)
 log-submit = Registra odore
 log-update = Aggiorna: la situazione è cambiata
 log-pm25-only = Registra solo PM2.5
+log-no-smell = Nessun odore
 log-read-only = Puoi consultare i dati ma non registrare.
 
 log-saved-report = Segnalazione salvata.
 log-saved-pm25 = Lettura PM2.5 salvata.
+log-saved-no-smell = Salvato: nessun odore.
 log-error-rooms = Scegli almeno una stanza.
 log-error-level = Scegli un livello da 0 a 10 per ogni stanza selezionata.
+log-error-no-smell-level = Nessun odore registra le stanze selezionate a 0. Togli i livelli sopra 0, oppure usa Registra odore.
 log-error-pm25 = Il PM2.5 deve essere un numero da 0 a 1000.
 log-error-pm25-missing = Inserisci un valore di PM2.5 per registrarlo da solo.
 log-error-time = Inserisci una data e un'ora valide.
@@ -52,6 +55,7 @@ prompt-which-event = Di quale evento si trattava?
 prompt-started-at = Iniziato alle { $time }
 prompt-ended-at = finito alle { $time }
 prompt-new-event = Nuovo evento
+prompt-no-event = Nessun evento: solo un controllo
 prompt-auto-closed = L'evento del { $when } è stato chiuso automaticamente.
 prompt-continue = Continuava ancora: è lo stesso evento
 prompt-cancel = Annulla
@@ -59,8 +63,8 @@ prompt-cancel = Annulla
 refused-stale = Nel frattempo gli eventi sono cambiati. Controlla e riprova.
 refused-another-open = C'è un altro evento aperto. Chiudilo prima.
 refused-outside-event = Quell'ora è fuori dall'evento: prima che iniziasse o in un altro giorno.
-refused-no-smell-new-event = Un rapporto senza odore non può iniziare un evento.
-refused-check-smell = Questo è un controllo senza odore: i livelli restano a 0. Per registrare un odore, registra un nuovo rapporto.
+refused-no-smell-new-event = Una segnalazione senza odore non può iniziare un evento.
+refused-check-smell = Questo è un controllo senza odore: i livelli restano a 0. Per registrare un odore, registra una nuova segnalazione.
 refused-future = L'ora non può essere nel futuro.
 refused-close-before-last-report = La fine non può essere prima dell'ultima segnalazione.
 refused-close-in-future = La fine non può essere nel futuro.
@@ -78,6 +82,10 @@ summary-bedroom-days = { $count ->
     [one] 1 giorno con fumo in camera da letto
    *[other] { $count } giorni con fumo in camera da letto
 }
+summary-checks = { $count ->
+    [one] 1 controllo pulito
+   *[other] { $count } controlli puliti
+}
 
 rating-title = Come valutare
 rating-level = Livello
@@ -113,6 +121,7 @@ nav-events = Eventi
 
 events-title = Eventi
 events-none = Nessun evento questo mese.
+events-check = nessun odore (controllo)
 events-previous = ← Precedente
 events-next = Successivo →
 events-in-progress = in corso
@@ -134,6 +143,7 @@ report-recorded = Registrata
 report-note = Nota
 report-by = Registrata da
 report-none = nessuno
+report-whole-home = tutta la casa
 report-edited = modificata
 report-deleted = eliminata
 report-correct = Correggi un errore
@@ -150,6 +160,7 @@ history-removed = tolto { $name }
 history-note = nota modificata
 history-moved = evento { $change }
 correct-hint = Correggi solo gli errori. Se la situazione è cambiata, registra invece una nuova segnalazione.
+correct-check-hint = Questo è un controllo senza odore: ogni livello resta a 0. Se hai sentito un odore, registra invece una nuova segnalazione.
 correct-time = Osservata alle
 correct-submit = Salva la correzione
 
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 679717d..9d02206 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -30,12 +30,15 @@ log-note = Not (isteğe bağlı)
 log-submit = Kokuyu kaydet
 log-update = Güncelle: durum değişti
 log-pm25-only = Yalnızca PM2.5 kaydet
+log-no-smell = Koku yok
 log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.
 
 log-saved-report = Bildirim kaydedildi.
 log-saved-pm25 = PM2.5 ölçümü kaydedildi.
+log-saved-no-smell = Kaydedildi: koku yok.
 log-error-rooms = En az bir oda seç.
 log-error-level = Seçtiğin her oda için 0 ile 10 arasında bir seviye seç.
+log-error-no-smell-level = Koku yok, seçili odaları 0 olarak kaydeder. 0'dan büyük seviyeleri kaldır ya da Kokuyu kaydet'i kullan.
 log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
 log-error-pm25-missing = Tek başına kaydetmek için bir PM2.5 değeri gir.
 log-error-time = Geçerli bir tarih ve saat gir.
@@ -52,6 +55,7 @@ prompt-which-event = Bu hangi olaydı?
 prompt-started-at = { $time } saatinde başladı
 prompt-ended-at = { $time } saatinde bitti
 prompt-new-event = Yeni olay
+prompt-no-event = Olay yok: yalnızca kontrol
 prompt-auto-closed = { $when } tarihli olay otomatik olarak kapatıldı.
 prompt-continue = Hâlâ sürüyordu: devamı
 prompt-cancel = Vazgeç
@@ -59,8 +63,8 @@ prompt-cancel = Vazgeç
 refused-stale = Bu arada olaylar değişti. Kontrol edip tekrar dene.
 refused-another-open = Açık başka bir olay var. Önce onu kapat.
 refused-outside-event = Bu saat olayın dışında: başlamadan önce ya da başka bir günde.
-refused-no-smell-new-event = Kokusuz bir rapor olay başlatamaz.
-refused-check-smell = Bu kokusuz bir kontrol, seviyeleri 0 kalır. Bir koku kaydetmek için yeni bir rapor ekle.
+refused-no-smell-new-event = Kokusuz bir bildirim olay başlatamaz.
+refused-check-smell = Bu kokusuz bir kontrol, seviyeleri 0 kalır. Bir koku kaydetmek için yeni bir bildirim kaydet.
 refused-future = Saat gelecekte olamaz.
 refused-close-before-last-report = Bitiş, son bildirimden önce olamaz.
 refused-close-in-future = Bitiş gelecekte olamaz.
@@ -78,6 +82,10 @@ summary-bedroom-days = { $count ->
     [one] yatak odasında duman olan 1 gün
    *[other] yatak odasında duman olan { $count } gün
 }
+summary-checks = { $count ->
+    [one] 1 temiz kontrol
+   *[other] { $count } temiz kontrol
+}
 
 rating-title = Nasıl puanlanır
 rating-level = Seviye
@@ -113,6 +121,7 @@ nav-events = Olaylar
 
 events-title = Olaylar
 events-none = Bu ay olay yok.
+events-check = koku yok (kontrol)
 events-previous = ← Önceki
 events-next = Sonraki →
 events-in-progress = sürüyor
@@ -134,6 +143,7 @@ report-recorded = Kayıt
 report-note = Not
 report-by = Kaydeden
 report-none = yok
+report-whole-home = tüm ev
 report-edited = düzenlendi
 report-deleted = silindi
 report-correct = Bir hatayı düzelt
@@ -150,6 +160,7 @@ history-removed = çıkarıldı: { $name }
 history-note = not değişti
 history-moved = olay { $change }
 correct-hint = Yalnızca hataları düzelt. Durum değiştiyse bunun yerine yeni bir bildirim kaydet.
+correct-check-hint = Bu kokusuz bir kontrol: tüm seviyeler 0 kalır. Bir koku aldıysan bunun yerine yeni bir bildirim kaydet.
 correct-time = Gözlem zamanı
 correct-submit = Düzeltmeyi kaydet
 
diff --git a/crates/web/src/labels.rs b/crates/web/src/labels.rs
index 9c629dd..8667428 100644
--- a/crates/web/src/labels.rs
+++ b/crates/web/src/labels.rs
@@ -1,7 +1,7 @@
 //! Names shown for stored values: starter rooms and symptoms through their translation key,
 //! custom ones as typed, and the translation keys of enum values.
 
-use smoke_domain::records::{Room, RoomLevel, Symptom};
+use smoke_domain::records::{ReportValues, Room, RoomLevel, Symptom};
 use smoke_domain::types::CloseReason;
 use smoke_domain::types::RoomId;
 
@@ -28,6 +28,20 @@ pub(crate) fn room_level(tr: Tr, rooms: &[Room], at: RoomLevel) -> String {
     format!("{} {}", room_by_id(tr, rooms, at.room), at.level.get())
 }
 
+/// A report's rooms with their levels, "Living room 6, Bedroom 0"; "whole home" for a no-smell
+/// report that lists none.
+pub(crate) fn report_rooms(tr: Tr, rooms: &[Room], values: &ReportValues) -> String {
+    if values.rooms().is_empty() {
+        return tr.t("report-whole-home");
+    }
+    values
+        .rooms()
+        .iter()
+        .map(|&at| room_level(tr, rooms, at))
+        .collect::<Vec<_>>()
+        .join(", ")
+}
+
 pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
     match &symptom.i18n_key {
         Some(key) => tr.t(key),
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index 5958c16..da99f1a 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -9,7 +9,9 @@ use axum::http::StatusCode;
 use axum::response::{IntoResponse, Redirect, Response};
 use chrono::{DateTime, Utc};
 use chrono_tz::Tz;
-use smoke_domain::events::{LogPrompt, can_join_backdated, classify_observed, log_prompt};
+use smoke_domain::events::{
+    LogPrompt, can_join_backdated, classify_observed, log_prompt, no_smell_prompt,
+};
 use smoke_domain::logbook::{EventChoice, LogError, SmellEntry, log_reading, log_smell};
 use smoke_domain::permissions::{Action, can};
 use smoke_domain::records::{Event, ReportValues, Room, RoomLevel, Symptom, User};
@@ -28,13 +30,14 @@ use crate::time::{self, TimeInputError};
 
 /// The translation keys [`read`] can return, for the translation tests.
 #[cfg(test)]
-pub(crate) const FORM_ERRORS: [&str; 8] = [
+pub(crate) const FORM_ERRORS: [&str; 9] = [
     "log-error-time",
     "log-error-time-skipped",
     "log-error-pm25",
     "log-error-pm25-missing",
     "log-error-rooms",
     "log-error-level",
+    "log-error-no-smell-level",
     "log-error-form",
     "log-error-note",
 ];
@@ -65,12 +68,25 @@ pub(crate) fn level_field(form: &Pairs, room: RoomId) -> Option<u8> {
         .and_then(|value| value.parse().ok())
 }
 
+/// How a form's rooms and levels are read.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum Smell {
+    /// "Log smell": at least one room, each with a level.
+    Levels,
+    /// "No smell": the selected rooms at 0, or the whole home when none is selected.
+    None,
+    /// "Correct a mistake": each selected room with a level; none selected is the whole home,
+    /// clean.
+    Correction,
+}
+
 /// Reads the rooms with their levels, the symptoms and the note. Only the offered rooms and
-/// symptoms are accepted; every selected room needs a level.
+/// symptoms are accepted.
 pub(crate) fn read_values(
     form: &Pairs,
     rooms: &[RoomId],
     symptoms: &[SymptomId],
+    smell: Smell,
 ) -> Result<ReportValues, &'static str> {
     let mut room_ids: Vec<RoomId> = form
         .ids("room")
@@ -91,13 +107,20 @@ pub(crate) fn read_values(
     {
         return Err("log-error-form");
     }
-    if room_ids.is_empty() {
+    if room_ids.is_empty() && smell == Smell::Levels {
         return Err("log-error-rooms");
     }
     let rooms = room_ids
         .into_iter()
         .map(|room| {
-            let level = level_field(form, room).and_then(|value| Level::new(value).ok());
+            let picked = level_field(form, room);
+            let level = match (smell, picked) {
+                (Smell::None, None | Some(0)) => Some(Level::CLEAN),
+                (Smell::None, Some(_)) => return Err("log-error-no-smell-level"),
+                (Smell::Levels | Smell::Correction, picked) => {
+                    picked.and_then(|value| Level::new(value).ok())
+                }
+            };
             Ok(RoomLevel {
                 room,
                 level: level.ok_or("log-error-level")?,
@@ -135,8 +158,12 @@ fn read(
         let value = pm25.ok_or("log-error-pm25-missing")?;
         return Ok(Entry::Pm25Only { observed_at, value });
     }
+    let smell = match form.get("action") {
+        Some("no-smell") => Smell::None,
+        _ => Smell::Levels,
+    };
     let (offered_rooms, offered_symptoms) = offered(rooms, symptoms, (&[], &[]));
-    let values = read_values(form, &offered_rooms, &offered_symptoms)?;
+    let values = read_values(form, &offered_rooms, &offered_symptoms, smell)?;
     Ok(Entry::Smell(SmellEntry {
         observed_at,
         values,
@@ -174,17 +201,21 @@ fn question_key(prompt: &LogPrompt) -> Option<String> {
 async fn current_prompt<S: Store>(
     state: &AppState<S>,
     user: &User,
-    observed_at: DateTime<Utc>,
+    entry: &SmellEntry,
     now: DateTime<Utc>,
 ) -> Result<(LogPrompt, Vec<Event>, Option<EventId>), AppError> {
-    let observed = classify_observed(observed_at, now).map_err(AppError::internal)?;
+    let observed = classify_observed(entry.observed_at, now).map_err(AppError::internal)?;
     let open = state.store.open_event().await?;
     let notices = notices::pending(state, user, now).await?;
-    let prompt = log_prompt(
-        observed,
-        open.as_ref().map(Event::summary),
-        notices.last().map(Event::summary),
-    );
+    let prompt = if entry.values.is_no_smell() {
+        no_smell_prompt(observed)
+    } else {
+        log_prompt(
+            observed,
+            open.as_ref().map(Event::summary),
+            notices.last().map(Event::summary),
+        )
+    };
     Ok((prompt, notices, open.map(|e| e.id)))
 }
 
@@ -220,7 +251,7 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
         Entry::Smell(entry) => entry,
     };
 
-    let (prompt, notices, open) = current_prompt(&state, &user, entry.observed_at, now).await?;
+    let (prompt, notices, open) = current_prompt(&state, &user, &entry, now).await?;
     let asked = question_key(&prompt);
     let answer = form.get("choice").and_then(parse_choice);
     let choice = match (&asked, form.get("asked"), answer) {
@@ -230,20 +261,24 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
             other => other,
         },
         (Some(_), _, _) => {
-            return ask(&state, &user, tr, now, &form, prompt, entry.observed_at).await;
+            return ask(&state, &user, tr, now, &form, prompt, &entry).await;
         }
     };
-    let observed_at = entry.observed_at;
+    let saved = if entry.values.is_no_smell() {
+        "/?saved=no-smell"
+    } else {
+        "/?saved=report"
+    };
     let mut tx = state.store.begin(Some(user.id)).await?;
-    match log_smell(&mut tx, &actor, entry, choice, &schedule, now).await {
+    match log_smell(&mut tx, &actor, entry.clone(), choice, &schedule, now).await {
         Ok(_) => {}
         Err(LogError::Stale) => {
             drop(tx);
-            let (fresh, ..) = current_prompt(&state, &user, observed_at, now).await?;
+            let (fresh, ..) = current_prompt(&state, &user, &entry, now).await?;
             if fresh == LogPrompt::Direct {
                 return back_to_form(&state, &user, tr, now, &form, "refused-stale").await;
             }
-            return ask(&state, &user, tr, now, &form, fresh, observed_at).await;
+            return ask(&state, &user, tr, now, &form, fresh, &entry).await;
         }
         Err(error) => {
             drop(tx);
@@ -254,7 +289,7 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
         notices::mark_seen(&mut tx, &notices).await?;
     }
     tx.commit().await?;
-    Ok(Redirect::to("/?saved=report").into_response())
+    Ok(Redirect::to(saved).into_response())
 }
 
 /// The Log screen again, with what was typed and what to fix.
@@ -326,7 +361,8 @@ pub(crate) async fn joinable_events<S: Store>(
         .collect())
 }
 
-/// Asks the event question. Guests see only start times of events (spec §7.2, §7.5).
+/// Asks the event question. Guests see only start times of events (spec §7.2, §7.5). A
+/// no-smell report is offered "no event" instead of a new one (spec §7.1).
 async fn ask<S: Store>(
     state: &AppState<S>,
     user: &User,
@@ -334,8 +370,9 @@ async fn ask<S: Store>(
     now: DateTime<Utc>,
     form: &Pairs,
     prompt: LogPrompt,
-    observed_at: DateTime<Utc>,
+    entry: &SmellEntry,
 ) -> Result<Response, AppError> {
+    let observed_at = entry.observed_at;
     let schedule = &state.config.schedule;
     let tz = schedule.tz();
     let answer = |choice: String, key: &str, primary: bool| Answer {
@@ -368,7 +405,11 @@ async fn ask<S: Store>(
                     primary: false,
                 })
                 .collect();
-            answers.push(new_event("prompt-new-event"));
+            answers.push(new_event(if entry.values.is_no_smell() {
+                "prompt-no-event"
+            } else {
+                "prompt-new-event"
+            }));
             (tr.t("prompt-which-event"), None, answers)
         }
         LogPrompt::AutoClosed(closed) => (
@@ -480,6 +521,22 @@ mod tests {
         }
     }
 
+    #[test]
+    fn no_smell_needs_no_room_and_puts_selected_rooms_at_zero() {
+        let Ok(Entry::Smell(home)) = read_form(&[("action", "no-smell")]) else {
+            panic!("the whole home");
+        };
+        assert!(home.values.rooms().is_empty() && home.values.is_no_smell());
+        let Ok(Entry::Smell(room)) = read_form(&[("action", "no-smell"), ("room", "1")]) else {
+            panic!("one room at 0");
+        };
+        assert_eq!(room.values.level_in(RoomId(1)), Some(Level::CLEAN));
+        assert_eq!(
+            read_form(&[("action", "no-smell"), ("room", "1"), ("level-1", "2")]).err(),
+            Some("log-error-no-smell-level")
+        );
+    }
+
     #[test]
     fn pm25_only_ignores_the_smell_fields_but_needs_a_value() {
         let Ok(Entry::Pm25Only { observed_at, value }) = read_form(&[
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index 43ec682..14f451d 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -46,9 +46,16 @@ struct ReadingRow {
     value: String,
 }
 
+/// A no-smell check made while no event was running.
+struct CheckRow {
+    id: i64,
+    at: String,
+}
+
 struct Day {
     date: String,
     events: Vec<EventRow>,
+    checks: Vec<CheckRow>,
     readings: Vec<ReadingRow>,
 }
 
@@ -77,6 +84,7 @@ fn day(days: &mut BTreeMap<NaiveDate, Day>, tr: Tr, date: NaiveDate) -> &mut Day
     days.entry(date).or_insert_with(|| Day {
         date: time::date(tr.0, date),
         events: Vec::new(),
+        checks: Vec::new(),
         readings: Vec::new(),
     })
 }
@@ -111,13 +119,15 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
         .unwrap_or(to)
         .max(to);
     let mut by_event: HashMap<EventId, Vec<SmellReport>> = HashMap::new();
+    let mut checks = Vec::new();
     for report in state
         .store
         .reports_between(window_start, window_end + TimeDelta::microseconds(1))
         .await?
     {
-        if let Some(event) = report.event_id {
-            by_event.entry(event).or_default().push(report);
+        match report.event_id {
+            Some(event) => by_event.entry(event).or_default().push(report),
+            None => checks.push(report),
         }
     }
 
@@ -146,6 +156,17 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
                 .map(|max| max.to_string()),
         });
     }
+    for check in checks.iter().rev() {
+        let in_month = (from..to).contains(&check.observed_at);
+        if in_month && can_see(&actor, check.user_id, now) {
+            day(&mut days, tr, schedule.local_date(check.observed_at))
+                .checks
+                .push(CheckRow {
+                    id: check.id.0,
+                    at: time::clock(check.observed_at, tz),
+                });
+        }
+    }
     for reading in state.store.readings_between(from, to).await?.iter().rev() {
         if reading.event_id.is_none() && can_see(&actor, reading.user_id, now) {
             day(&mut days, tr, schedule.local_date(reading.observed_at))
@@ -273,18 +294,13 @@ async fn detail<S: Store>(
             "level",
             &report.values.intensity().get().to_string(),
         )];
-        let room_names: Vec<String> = report
-            .values
-            .rooms()
-            .iter()
-            .map(|&at| labels::room_level(tr, &rooms, at))
-            .collect();
+        let room_names = labels::report_rooms(tr, &rooms, &report.values);
         let symptom_names: Vec<String> = symptoms
             .iter()
             .filter(|s| report.values.symptoms().contains(&s.id))
             .map(|s| labels::symptom(tr, s))
             .collect();
-        parts.extend([room_names.join(", "), symptom_names.join(", ")]);
+        parts.extend([room_names, symptom_names.join(", ")]);
         timeline.push((
             report.observed_at,
             Entry {
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index 0742270..ba1f73f 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -212,6 +212,7 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
     let saved = match query.saved.as_deref() {
         Some("report") => Some("log-saved-report"),
         Some("pm25") => Some("log-saved-pm25"),
+        Some("no-smell") => Some("log-saved-no-smell"),
         _ => None,
     };
     let log_page = log_page(&state, &user, tr, now, None, None, saved).await?;
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index 3ae3163..08a69dc 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -18,7 +18,7 @@ use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
 use crate::i18n::Tr;
-use crate::routes::entries::{joinable_events, read_time, read_values};
+use crate::routes::entries::{Smell, joinable_events, read_time, read_values};
 use crate::routes::guide::{GuideRow, levels};
 use crate::routes::log::{Draft, Fields, fields, offered};
 use crate::routes::refusal;
@@ -232,10 +232,12 @@ async fn report_page<S: Store>(
                 label,
             });
         }
-        move_options.push(MoveOption {
-            value: "new".to_owned(),
-            label: tr.t("prompt-new-event"),
-        });
+        if !report.values.is_no_smell() {
+            move_options.push(MoveOption {
+                value: "new".to_owned(),
+                label: tr.t("prompt-new-event"),
+            });
+        }
     }
 
     let none = || tr.t("report-none");
@@ -261,14 +263,7 @@ async fn report_page<S: Store>(
             "{intensity} · {}",
             tr.t(&format!("intensity-{intensity}-name"))
         ),
-        rooms: join(
-            report
-                .values
-                .rooms()
-                .iter()
-                .map(|&at| labels::room_level(tr, &rooms, at))
-                .collect(),
-        ),
+        rooms: labels::report_rooms(tr, &rooms, &report.values),
         symptoms: join(symptom_names(tr, &symptoms, report.values.symptoms())),
         note: report.values.note().map(str::to_owned),
         by: name(Some(report.user_id), &users),
@@ -295,6 +290,8 @@ struct CorrectPage {
     signed_in: Option<String>,
     id: i64,
     levels: Vec<GuideRow>,
+    /// A no-smell check: its levels stay at 0.
+    check: bool,
     form: CorrectForm,
 }
 
@@ -338,6 +335,7 @@ async fn correct_page<S: Store>(
         signed_in: Some(user.username.to_string()),
         id: report.id.0,
         levels: levels(tr),
+        check: report.event_id.is_none(),
         form: CorrectForm {
             fields: fields(tr, &rooms, &symptoms, &draft, keep),
             observed_at: draft.observed_at,
@@ -389,7 +387,7 @@ pub(crate) async fn correct<S: Store + Clone + 'static>(
     let keep = (room_ids.as_slice(), report.values.symptoms());
     let (offered_rooms, offered_symptoms) = offered(&rooms, &symptoms, keep);
     let parsed = read_time(&form, "observed_at", schedule.tz()).and_then(|at| {
-        let values = read_values(&form, &offered_rooms, &offered_symptoms)?;
+        let values = read_values(&form, &offered_rooms, &offered_symptoms, Smell::Correction)?;
         Ok((at.unwrap_or(report.observed_at), values))
     });
     let error = match parsed {
diff --git a/crates/web/templates/correct.html b/crates/web/templates/correct.html
index e0252ad..02a0ed7 100644
--- a/crates/web/templates/correct.html
+++ b/crates/web/templates/correct.html
@@ -3,6 +3,9 @@
 {% block content %}
 <h1>{{ tr.t("report-correct") }}</h1>
 <p>{{ tr.t("correct-hint") }}</p>
+{% if check %}
+<p>{{ tr.t("correct-check-hint") }}</p>
+{% endif %}
 {% if let Some(error) = form.error %}
 <p class="notice" role="alert">{{ tr.t(error) }}</p>
 {% endif %}
diff --git a/crates/web/templates/events.html b/crates/web/templates/events.html
index 0ddb9cc..e0fc607 100644
--- a/crates/web/templates/events.html
+++ b/crates/web/templates/events.html
@@ -14,6 +14,9 @@
     {% for event in day.events %}
     <li><a href="/events/{{ event.id }}">{{ event.span }}</a> · {{ tr.n("summary-reports", event.reports) }}{% if let Some(max) = event.max_intensity %} · {{ tr.t1("events-max-intensity", "level", max) }}{% endif %}</li>
     {% endfor %}
+    {% for check in day.checks %}
+    <li><a href="/reports/{{ check.id }}">{{ check.at }} · {{ tr.t("events-check") }}</a></li>
+    {% endfor %}
     {% for reading in day.readings %}
     <li>{{ reading.at }} · {{ tr.t1("event-pm25", "value", reading.value) }}</li>
     {% endfor %}
diff --git a/crates/web/templates/log.html b/crates/web/templates/log.html
index 3fd96f4..11bac56 100644
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -25,7 +25,7 @@
   {% endif %}
 </div>
 <p class="summary">
-  {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %}
+  {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %} · {{ tr.n("summary-checks", summary.checks) }}
 </p>
 {% if let Some(form) = form %}
 {% if let Some(error) = form.error %}
@@ -44,6 +44,7 @@
     <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
   </label>
   <button type="submit" name="action" value="smell">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
+  <button type="submit" name="action" value="no-smell" formnovalidate class="secondary">{{ tr.t("log-no-smell") }}</button>
   <button type="submit" name="action" value="pm25" formnovalidate class="secondary">{{ tr.t("log-pm25-only") }}</button>
 </form>
 {% include "rating_dialog.html" %}
diff --git a/crates/web/templates/report.html b/crates/web/templates/report.html
index 3da73cb..d4f6388 100644
--- a/crates/web/templates/report.html
+++ b/crates/web/templates/report.html
@@ -29,6 +29,7 @@
 </dl>
 {% if editable %}
 <p><a href="/reports/{{ id }}/correct">{{ tr.t("report-correct") }}</a></p>
+{% if !move_options.is_empty() %}
 <form method="post" action="/reports/{{ id }}/move" class="stack">
   <label>{{ tr.t("report-move") }}
     <select name="target">
@@ -40,6 +41,7 @@
   <button type="submit" class="secondary">{{ tr.t("report-move-submit") }}</button>
 </form>
 {% endif %}
+{% endif %}
 <h2>{{ tr.t("report-history") }}</h2>
 <ol class="history">
   {% for line in history %}
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: no formatting changes, no clippy warnings; every test passes, including `no_smell_with_nothing_open_saves_a_check_without_asking`, `no_smell_with_an_event_open_joins_it_without_the_question`, `a_past_no_smell_report_is_offered_no_event_instead_of_a_new_one`, `the_list_shows_each_days_checks_to_those_who_may_see_them` and `no_page_needs_inline_scripts_styles_or_eval`. With `TEST_POSTGRES_URL` set: 482 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/labels.rs \
  crates/web/src/routes/entries.rs \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/reports.rs \
  crates/web/templates/correct.html \
  crates/web/templates/events.html \
  crates/web/templates/log.html \
  crates/web/templates/report.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/events.rs \
  crates/web/tests/log.rs \
  crates/web/tests/logging.rs \
  crates/web/tests/reports.rs
git commit -m "feat(web): a No smell button, checks in the Events list and the summary card

No smell records the selected rooms at 0, or the whole home; logged now
it asks nothing, and backdated it offers \"No event: just a check\" in
place of a new event. The Events list shows each day's checks, the
summary card counts clean checks, and a check's page says it has no
event, offers no new event to move to and explains it stays at 0.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Decisions and roadmap

**Files:**
- Modify: `docs/DECISIONS.md` (append rows #102–#109; the file is append-only)
- Modify: `docs/plans/ROADMAP.md`

**Interfaces:** none (documentation). The code comments of Tasks 1–2 already cite decisions #103
and #104.

- [ ] **Step 1: Append the decisions and link this plan from the roadmap**

```diff
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index a3622dc..d0c01f8 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -104,3 +104,11 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 95 | 2026-10-13 | Storage | `reports_between` loads the room and symptom links of all its reports with **two joined queries** (three round trips per window, not two per report); no new indexes | One query per report; indexes on `observed_at` | Avoids N+1 round trips on a remote Postgres such as Neon; a few thousand rows scan in microseconds |
 | 96 | 2026-10-13 | UI | The Events list shows **one local month** (`/events?month=YYYY-MM`), grouped by the day an event opened, newest first, with standalone PM2.5 readings; an event that started in the previous month is listed there only | Infinite scroll; list by every day an event touches | Bounded queries; each event appears once |
 | 97 | 2026-10-13 | Errors | A role refusal shows a translated **"Not allowed" (403)** page, distinct from the cross-site **"Request blocked"** page | One 403 page | The two causes need different advice |
+| 98 | 2026-10-13 | Model | A report stores **a level 0–10 per room** (`smell_report_rooms.level`; `Level` replaces `Intensity`, 0 = checked and clean); a report's **intensity is derived** as its highest room level and is not stored | Keep a stored report intensity next to the room levels; a separate list of clean rooms | One source of truth (spec §6); a room left out stays "not checked" |
+| 99 | 2026-10-13 | Storage | Migration 0003 gives **every existing room its report's intensity**, and **refuses to run, changing nothing, while a report has no rooms** (`every_smell_report_needs_a_room`) | A hidden placeholder room; the intensity on every room; deleting such reports; turning them into no-smell reports | No released build can store such a report (milestones 1–3 have no logging; milestone 4 ships with this change), so only local trial data can; a placeholder room would be a row with no audit entry that `verify-audit` flags for ever, and the other options invent or drop evidence. Fix: correct those reports with the previous build, or start a new local database |
+| 100 | 2026-10-13 | Audit | Audit entries are **never rewritten**: `verify-audit` and History read a report snapshot in the old shape (`"intensity"` and room ids) as **every room at that intensity**, exactly what migration 0003 did (`SmellReport::upgrade_snapshot`) | Append "migrated" audit entries; rewrite old entries; skip reports in `verify-audit` | The chain hashes the stored JSON and SQL cannot extend it; one pure function keeps old chains valid, the record check meaningful and old corrections readable ("Bedroom 6 → 7") |
+| 101 | 2026-10-13 | Events | A **no-smell report** (rooms all at 0, or no rooms = the whole home) **never opens an event**: logged now it joins the open event **without the question** and does not close it, otherwise it is a **check** with no event (`event_id` NULL); backdated, "Which event was this?" offers **"No event: just a check"** in place of a new event; it cannot continue an auto-closed event | The form refusing it (the first wording of this change); a zero-intensity event | Spec §7.1–§7.2: a check records that someone looked; an event means smoke |
+| 102 | 2026-10-13 | Events | A check can be **corrected** (rooms, symptoms, note, time) but **never above 0**, and **moved into an event**, never into a new one; a report inside an event **may be corrected to no smell**, even the one that opened it, and the event stays | Refuse correcting an event's first report to 0 | Spec §7.1; a correction records what was true, and refusing would leave a known mistake with no fix |
+| 103 | 2026-10-13 | UI | Each room chip has a **level row 0–10, shown only while the room is selected** (CSS `:has`, no script); one ⓘ *How to rate* beside "Rooms"; **No smell** stores the selected rooms at 0 (a room without a level counts as 0, a level above 0 is refused) or, with none selected, the whole home; **Log smell** needs a room and a level for each; rooms are pre-selected from the last report **without levels** | Level rows always visible; ⓘ on every row; pre-filled levels | Spec §9 screen 1; a stale level is never submitted by accident; without `:has` the rows simply stay visible |
+| 104 | 2026-10-13 | UI | Summary card: **reports** = reports in an event, **clean checks** = checks outside any event, **days with bedroom smoke** = days with the bedroom at level 1 or more; the Events list shows each day's checks, which guests see only for themselves | Count every no-smell report as a check | The two counts never overlap; a clean bedroom is not a bedroom-smoke day |
+| 105 | 2026-10-13 | Storage | Migration 0004 makes `smell_reports.event_id` **nullable**: SQLite **rebuilds** `smell_reports` and both link tables inside the migration's transaction (copy, drop, rename) with foreign keys on; Postgres drops `NOT NULL` | `PRAGMA foreign_keys = OFF` in a no-transaction migration; a sentinel "no event" event | Atomic, no switched-off checks, no sentinel rows to filter out everywhere |
diff --git a/docs/plans/ROADMAP.md b/docs/plans/ROADMAP.md
index 22445a3..4d40006 100644
--- a/docs/plans/ROADMAP.md
+++ b/docs/plans/ROADMAP.md
@@ -10,7 +10,7 @@ guesses.
 | 1 | **Foundation & domain core** | Cargo workspace, secret protection, CI + security workflows, and the pure `domain` crate: types, clock, permissions, 04:00 schedule, event rules, hash-chained audit | §2, §4, §6.1, §7, §10, §11, §12 (secrets), §13.1 | `01-foundation-and-domain.md` ✅ |
 | 2 | **Storage** | Repository traits in domain; storage-sqlite and storage-postgres crates with migrations; an audit entry inside every write transaction; one storage-contract test suite run on both backends; schema-parity check; verify-audit logic | §6, §6.1, §10.1, §13.3 | `02-storage.md` ✅ |
 | 3 | **Web foundation & auth** | Axum server, config, CLI (`serve`, `migrate`, `create-resident`, `verify-audit`), sessions, login + rate limit, security headers, Origin check, i18n (en/it/tr), base layout, embedded assets, Dockerfile, release workflow, Trivy | §8.1, §8.2, §8.4, §9.2, §10.3–10.5, §12, §13.1 | `03-web-and-auth.md` ✅ |
-| 4 | **Logging flow** | Log screen, every prompt (same smell / which event / auto-close notice), close/reopen, PM2.5, events list and detail, report detail (correct, move, history), large-gap warning, auto-close sweep, summary card, "How to rate" popup | §5, §7, §9 (1–4), §11 | `04-logging-flow.md` |
+| 4 | **Logging flow** | Log screen, every prompt (same smell / which event / auto-close notice), close/reopen, PM2.5, events list and detail, report detail (correct, move, history), large-gap warning, auto-close sweep, summary card, "How to rate" popup | §5, §7, §9 (1–4), §11 | `04-logging-flow.md`, `04b-room-levels.md` |
 | 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | — |
 | 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | — |
 | 7 | **Neon readiness** | `migrate-to-postgres` command and its CI test | §10.2 | — |
```

- [ ] **Step 2: Commit**

```bash
git add docs/DECISIONS.md docs/plans/ROADMAP.md
git commit -m "docs: decisions #102–#109 for room levels and no-smell checks; roadmap links the plan

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Development

Local loop (one shared build folder for every worktree):

```sh
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
export TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/sqlx-prepare.sh --check
```

A local database created by the milestone 4 branch before this change migrates on the next
`migrate` or `serve`, unless it holds a report without rooms (decision #103): then the migration
stops with `every_smell_report_needs_a_room` and changes nothing.

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §3 | A smell report: the rooms checked, each with a level 0–10 (0 = no smell) | 1 |
| §5 | Rule 2: a room's level, a room starting or clearing (logged at 0); a room left out is "not checked" | 1, 3 |
| §5 | Rule 5: *No smell* keeps a record that you checked | 3 |
| §5 | Rating table with level 0 "Clean" | 1 |
| §6 | `smell_report_rooms.level 0–10`; no stored report intensity; at least one room except a no-smell report; `event_id` optional | 1, 2 |
| §6 | A report's intensity is its highest room level; an event's is the highest among its reports | 1 (Events list "max", timeline, report page) |
| §6.1 | History reads "living room 6 → 7, added bedroom 3"; the audit chain and `verify-audit` keep working | 1 |
| §7.1 | A no-smell report never opens an event, now or backdated; with nothing open it is a check | 2, 3 |
| §7.1 | A check can be corrected but never above 0 (the form says to log a smell); it can be moved into an event | 2, 3 |
| §7.2 | A no-smell report joins the open event without the prompt and does not close it | 2, 3 |
| §9 screen 1 | A level row 0–10 under each selected room with ⓘ How to rate; a level for every selected room; a room required except for *No smell*; *No smell* button, guests too | 1, 3 |
| §9 screen 2 | Summary card "… · N clean checks"; bedroom days count only bedroom levels ≥ 1 | 1, 2, 3 |
| §9 screen 3 | Events list with each day's no-smell checks | 3 |
| §9 screen 4 | Report detail: room levels, no event for a check | 1, 2 |
| §9 screen 5 | Graphs of room levels | Milestone 6 (nothing to build here) |
| §13.3 | `Level` bounds; migrations on both backends | 1, 2 |
