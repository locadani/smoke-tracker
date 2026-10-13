# Milestone 4: Logging flow — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The app becomes usable for its purpose: a phone logs a smell in a few taps from the Log
screen, answers "Same smell still going?", "Which event was this?" or the auto-close notice when
one applies, logs PM2.5 with a report or alone, closes and reopens events, browses a month of
events and each event's timeline (with the large-gap warning), and opens a report to see both
times and its History, correct a mistake or move it. The 04:00 auto-close sweep runs at startup,
every five minutes and before every logging request. The Log screen shows the month's summary
card and the "How to rate" popup.

**Architecture:** The rules live in `smoke-domain`. A new `logbook` module holds the use cases
(log a smell, log a reading, close, reopen, correct, move) as generic functions over the
write-transaction traits; each reads the data it decides on inside the transaction and checks the
spec §4 permission there, so a stale answer from a second phone is refused instead of applied. A
`sweep` module is the idempotent 04:00 catch-up job; `summary` and `history` are pure. The storage
crates gain four reads (a time window of reports, of readings, the event at an instant, unseen
auto-close notices); the storage contract runs every use case on SQLite and Postgres. `smoke-web`
adds one route module per screen (`log`, `entries`, `notices`, `events`, `reports`, `guide`): plain
HTML forms with Post/Redirect/Get, the event question re-posting the form as hidden fields with an
`asked` key, and a native `<dialog>` opened by a tiny self-hosted script. `smoke-tracker` runs the
sweep on a timer.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8 (+ `query` feature), askama 0.16
(templates with `include`), fluent-templates 0.15 (plural selectors), chrono + chrono-tz, sqlx
0.9, tokio. No new crates.

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` (§5, §7.1–§7.7, §9 screens 1–4, §11;
also §4, §8.1, §9.2, §12). Decisions: `docs/DECISIONS.md` (rows #86–#101 are added by Task 12).
Previous plan: `docs/plans/03-web-and-auth.md`.

**Provenance:** every code block in this plan was compiled and tested before the plan was written,
as one commit per task on the branch **`m4-proto`** (from `main` at `d56e5d3`; head `4556e4c`).
At every task boundary `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`
and `cargo test --workspace` (SQLite and Postgres) passed; `scripts/sqlx-prepare.sh --check`,
`cargo deny check` and a live run of `smoke-tracker serve` (startup sweep, `/healthz`, `/login`,
`/static/app.js`) passed at the end. Executors may consult that branch; this plan is the source
of truth.

## Open questions for the user

Defaults were chosen so nothing blocks; each is a decision row and easy to change.

1. **One milestone** (12 tasks + an optional spec-wording task), not split.
2. **"Full guide →"** in the popup opens a new page, `/guide/rating`, with the whole intensity
   table and the tip (decision #97). It moves under Settings → Guides in milestone 6.
3. **Italian and Turkish strings** (about 120 new keys in Tasks 8–11) were drafted by Claude and
   need a native speaker's review, like the milestone 3 ones (spec §9.2).
4. **Report notes stay at most 2000 characters.** The 500 suggested while planning conflicts
   with decision #51 and `ReportValues::NOTE_MAX_CHARS`, already enforced by the domain since
   milestone 1, so it was kept. Lowering it is a one-line change plus a decision row that
   supersedes #51.
5. **Summary card** (decision #94): "This month: N events · N reports · N days with bedroom
   smoke", where *bedroom* is the starter room `room-bedroom` (a custom room is never guessed at;
   the third part disappears if that room is gone), *events* are events with a visible report this
   local month, and guests count only their own reports.
6. **Plain forms, not htmx partials** (decision #96): validation errors render the same page again
   with status 422. Spec §12 says "(htmx partial)"; Task 13 proposes the wording change and
   **needs your approval** before anyone runs it.
7. **Corrections may change the observed time** within the report's event (decision #90). A wrong
   start time of an event's *first* report cannot be moved earlier; "Move to another event → New
   event" is the way out.
8. **Guests may answer "No, it had stopped: new event"** even when the open event is a resident's,
   which closes it at its last report (spec §7.2; decision #92). Closing by hand stays limited to
   their own events.
9. **Sweep timer: every 5 minutes** (decision #88), plus startup, the Log screen and every logging
   request.

## Global Constraints

- Personal project. Before every commit, `git config user.email` must print
  `61902944+locadani@users.noreply.github.com` and `git config user.name` must print `locadani`.
  Never put any email address in a tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings: local and CI Postgres use trust authentication.
- Never modify `description.md` or `claude.md`. Never `git add` them.
- **Never edit `docs/specs/2026-10-13-smoke-tracker-design.md` without the user's explicit
  approval.** Task 13 holds every proposed spec edit and must not start until the user approves.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI, `cargo fmt`
  enforced.
- `smoke-web` never sees SQL and never depends on a storage crate except as a dev-dependency. It
  writes only through the `smoke_domain::logbook` use cases (plus `mark_auto_close_notice_seen`),
  never by calling `create_event`/`close_event`/… itself.
- Free tier and a small footprint: no new crates in this milestone; every script, style and icon is
  served by the app.
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
- `Cargo.lock` is committed; commit the one `cargo build` produces with the task that changed it.
- Disk: all builds share one `CARGO_TARGET_DIR`; never create another target folder, never run
  `cargo clean` or `rm -rf` on build folders (the user does that).

## Review Focus

1. **A guest learning about other people's smells** (intensity, rooms, symptoms, who) through the
   event question, the Events screens or a report URL. Expected: the question shows only start
   times, lists and timelines show only the guest's own entries and no names, and another person's
   event or report answers 404 to every read and write. Tests:
   `a_guest_sees_only_the_start_time_of_someone_elses_event` (Task 9),
   `guests_see_only_their_own_entries_and_not_other_peoples_events` (Task 10),
   `guests_cannot_see_or_change_other_peoples_reports` (Task 11).
2. **Two phones answering at once.** Phone A is asked "Same smell?" about event 1; meanwhile
   phone B answers "No, new event". Expected: A's "Yes" never lands in the closed event 1; A gets
   the current question (about event 2) and nothing is written. Tests:
   `a_stale_answer_is_refused_and_changes_nothing` (Task 5),
   `a_stale_answer_gets_the_current_question_instead_of_being_applied` (Task 9).
3. **The sweep on a host that slept, and on DST days.** Expected: a catch-up at 08:00 closes
   exactly as a run at 04:00 would (at the last report, `auto_0400`), an event opened after the
   boundary stays open, and 04:00 local is honoured on both DST days. Tests:
   `a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four`,
   `an_event_opened_after_the_boundary_stays_open`,
   `on_dst_days_the_sweep_still_runs_at_four_local` (Task 2).
4. **A half-applied "new event"** (the open event closed, but the report failed). Expected: the
   close, the new event, the report and its reading commit together or not at all. Test:
   `a_failed_report_rolls_back_the_new_event_close` (Task 5).
5. **The popup breaking the strict CSP** (inline `onclick`, inline styles). Expected: the
   `<dialog>` opens through `static/app.js`, closes with a `method="dialog"` form, and no page has
   inline code; without JavaScript the link opens the full guide. Tests:
   `how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide` (Task 8) and
   `no_page_needs_inline_scripts_styles_or_eval` (extended in Tasks 8–11).

---

## File Structure

```
smoke-tracker/
├─ Cargo.toml                          axum + "query" feature
├─ docs/DECISIONS.md, plans/ROADMAP.md + rows #86–#101, M4 plan link
└─ crates/
   ├─ domain/src/
   │  ├─ repos.rs                      + reports_between, readings_between, event_at,
   │  │                                  unseen_auto_closed
   │  ├─ schedule.rs                   + local_date, day_span, month_span
   │  ├─ permissions.rs                + can_see
   │  ├─ events.rs                     + can_join_backdated, auto_close_notice_is_for
   │  ├─ sweep.rs                      NEW  the 04:00 catch-up job
   │  ├─ logbook.rs                    NEW  log_smell, log_reading, close_event, reopen_event,
   │  │                                     correct_report, move_report
   │  ├─ summary.rs                    NEW  the month summary card
   │  ├─ history.rs                    NEW  a report's History from audit snapshots
   │  └─ starter.rs                    + BEDROOM_KEY
   ├─ storage-sqlite/, storage-postgres/
   │  ├─ src/{events,reports,readings}.rs  the four new reads (reports: batched link loading)
   │  └─ .sqlx/                        regenerated
   ├─ storage-contract/src/            + sweep.rs, logbook.rs, corrections.rs; new read tests
   ├─ web/
   │  ├─ src/config.rs                 WebConfig.schedule
   │  ├─ src/error.rs                  AppError::NotAllowed (403 "Not allowed")
   │  ├─ src/i18n.rs                   Tr::n (plurals); the key test scans templates and sources
   │  ├─ src/{time,labels,forms}.rs    NEW  local times, display names, repeated-name forms
   │  ├─ src/routes/
   │  │  ├─ log.rs                     NEW  GET / — the Log screen (replaces home.rs)
   │  │  ├─ entries.rs                 NEW  POST /log — the event question and saving
   │  │  ├─ notices.rs                 NEW  pending auto-close notices, POST /notices/dismiss
   │  │  ├─ events.rs                  NEW  /events, /events/{id}, close, reopen
   │  │  ├─ reports.rs                 NEW  /reports/{id}, correct, move
   │  │  ├─ guide.rs                   NEW  /guide/rating and the intensity levels
   │  │  └─ mod.rs                     + refusal(LogError)
   │  ├─ templates/                    log, prompt, events, event, report, correct, guide,
   │  │                                report_fields (include), rating_dialog (include);
   │  │                                home.html removed
   │  ├─ static/app.js                 NEW  opens <dialog>s (no inline code)
   │  ├─ locales/{en,it,tr}/main.ftl   ~120 new keys
   │  └─ tests/                        + log.rs, logging.rs, events.rs, reports.rs
   └─ app/src/
      ├─ config.rs                     the schedule moves into WebConfig
      └─ commands/serve.rs             the sweep timer
```

**Dependency direction** is unchanged: `smoke-tracker → smoke-web → smoke-domain ← storage-*`.

**Why the use cases live in the domain:** the rules of spec §7 need several writes in one
transaction (close the open event, open a new one, the report, its reading) and decisions made on
data read inside that transaction. As generic functions over the write traits they stay free of
I/O details, the storage contract exercises them on both backends, and the web layer cannot
forget a rule or a permission check (decision #86).

---

### Task 1: Storage — time-window reads, the event at an instant, unseen notices

Four reads the screens need, on both backends: a month's reports (with their rooms and symptoms
loaded in two joined queries, not two per report), a month's readings, the event whose span
contains an instant (to link a PM2.5 reading, spec §7.6), and the auto-closed events whose notice
nobody has seen (spec §7.4).

**Files:**
- Modify: `crates/domain/src/repos.rs`, `crates/storage-contract/src/events.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/readings.rs`, `crates/storage-contract/src/reports.rs`, `crates/storage-postgres/src/events.rs`, `crates/storage-postgres/src/readings.rs`, `crates/storage-postgres/src/reports.rs`, `crates/storage-sqlite/src/events.rs`, `crates/storage-sqlite/src/readings.rs`, `crates/storage-sqlite/src/reports.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/`

**Interfaces:**
- Consumes: `EventRow`, `ReportRow`, `ReadingRow`, `micros`, `require`, `load` (milestone 2).
- Produces (in `smoke_domain::repos`):
  - `EventRepo::unseen_auto_closed(&self) -> StoreResult<Vec<Event>>` (ordered by `opened_at`).
  - `EventWrites::event_at(&mut self, at) -> StoreResult<Option<Event>>` — latest-opened event
    with `opened_at <= at` and (`closed_at >= at` or open).
  - `ReportRepo::reports_between(&self, from, to) -> StoreResult<Vec<SmellReport>>` and
    `ReadingRepo::readings_between(&self, from, to) -> StoreResult<Vec<Pm25Reading>>` — not
    deleted, observed in `[from, to)`, ordered by observed time.

- [ ] **Step 1: Write the failing tests**

In `crates/storage-contract/src/events.rs`:

```diff
--- a/crates/storage-contract/src/events.rs
+++ b/crates/storage-contract/src/events.rs
@@ -329,3 +329,58 @@ where
     assert!(between(at(4), at(5)).await.is_empty());
     assert_eq!(between(at(10), at(11)).await, [open.id]);
 }
+
+pub async fn unseen_auto_closed_events_are_listed_until_seen<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo,
+{
+    let owner = add_user(fx, "daniele", Role::Resident).await;
+    let at = |hours: i64| start() - TimeDelta::hours(48) + TimeDelta::hours(hours);
+    let auto = |hours: i64| closure(at(hours), CloseReason::Auto0400);
+    let later = add_event(fx, owner.id, at(24), auto(25)).await;
+    let earlier = add_event(fx, owner.id, at(0), auto(1)).await;
+    add_event(fx, owner.id, at(4), closure(at(5), CloseReason::Manual)).await;
+    add_event(fx, owner.id, at(30), None).await;
+
+    let unseen = |events: Vec<Event>| events.into_iter().map(|e| e.id).collect::<Vec<_>>();
+    assert_eq!(
+        unseen(fx.store.unseen_auto_closed().await.unwrap()),
+        [earlier.id, later.id]
+    );
+    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
+    tx.mark_auto_close_notice_seen(earlier.id).await.unwrap();
+    tx.commit().await.unwrap();
+    assert_eq!(
+        unseen(fx.store.unseen_auto_closed().await.unwrap()),
+        [later.id]
+    );
+}
+
+pub async fn event_at_finds_the_event_whose_span_contains_the_time<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: UserWrites + EventWrites>,
+{
+    let owner = add_user(fx, "daniele", Role::Resident).await;
+    let at = |hours: i64| start() - TimeDelta::hours(12) + TimeDelta::hours(hours);
+    let first = add_event(fx, owner.id, at(0), closure(at(2), CloseReason::Manual)).await;
+    let overlapping = add_event(fx, owner.id, at(1), closure(at(3), CloseReason::Manual)).await;
+    let open = add_event(fx, owner.id, at(6), None).await;
+
+    let mut tx = fx.store.begin(None).await.unwrap();
+    let mut found = async |t| tx.event_at(t).await.unwrap().map(|e| e.id);
+    assert_eq!(found(at(0)).await, Some(first.id));
+    assert_eq!(
+        found(at(2)).await,
+        Some(overlapping.id),
+        "the latest start wins"
+    );
+    assert_eq!(
+        found(at(3)).await,
+        Some(overlapping.id),
+        "the end is included"
+    );
+    assert_eq!(found(at(4)).await, None);
+    assert_eq!(found(at(0) - TimeDelta::seconds(1)).await, None);
+    assert_eq!(found(at(6)).await, Some(open.id));
+    assert_eq!(found(start() + TimeDelta::days(1)).await, Some(open.id));
+}
```

In `crates/storage-contract/src/lib.rs`:

```diff
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -51,6 +51,8 @@ macro_rules! contract_tests {
             events::moving_an_event_end_is_audited_and_checked,
             events::the_auto_close_notice_is_marked_seen_once,
             events::events_between_returns_overlapping_events,
+            events::unseen_auto_closed_events_are_listed_until_seen,
+            events::event_at_finds_the_event_whose_span_contains_the_time,
             reports::a_report_keeps_its_rooms_and_symptoms,
             reports::correcting_a_report_records_before_and_after,
             reports::moving_a_report_is_audited,
@@ -61,11 +63,13 @@ macro_rules! contract_tests {
             reports::an_unknown_room_on_correction_keeps_the_original_links,
             reports::moving_a_report_to_an_unknown_event_is_rejected,
             reports::an_unknown_symptom_rolls_back_the_report,
+            reports::reports_between_returns_live_reports_with_their_links,
             readings::a_reading_can_stand_alone_or_join_an_event,
             readings::correcting_and_deleting_a_reading_are_audited,
             readings::a_reading_for_an_unknown_event_is_rejected,
             readings::pm25_values_round_trip_exactly,
             readings::job_runs_are_remembered_but_not_audited,
+            readings::readings_between_returns_live_readings_in_the_window,
             audit::every_write_extends_one_unbroken_chain,
             audit::the_history_of_a_record_lists_its_entries,
             audit::a_direct_record_edit_is_detected,
```

In `crates/storage-contract/src/readings.rs`:

```diff
--- a/crates/storage-contract/src/readings.rs
+++ b/crates/storage-contract/src/readings.rs
@@ -216,3 +216,35 @@ where
     );
     assert!(fx.store.audit_log().await.unwrap().is_empty());
 }
+
+pub async fn readings_between_returns_live_readings_in_the_window<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
+        + ListRepo
+        + ReadingRepo,
+{
+    let scene = scene(fx).await;
+    let at = |minutes: i64| start() - TimeDelta::minutes(minutes);
+    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
+    let mut add = async |minutes: i64, event_id: Option<EventId>| {
+        tx.create_reading(NewReading {
+            user_id: scene.user.id,
+            event_id,
+            observed_at: at(minutes),
+            value: pm25(10.0),
+        })
+        .await
+        .unwrap()
+    };
+    let before = add(50, None).await;
+    let standalone = add(40, None).await;
+    let linked = add(30, Some(scene.event.id)).await;
+    let deleted = add(20, None).await;
+    let at_end = add(10, None).await;
+    tx.delete_reading(deleted.id).await.unwrap();
+    tx.commit().await.unwrap();
+
+    let found = fx.store.readings_between(at(40), at(10)).await.unwrap();
+    assert_eq!(found, [standalone, linked]);
+    assert!(!found.contains(&before) && !found.contains(&at_end));
+}
```

In `crates/storage-contract/src/reports.rs`:

```diff
--- a/crates/storage-contract/src/reports.rs
+++ b/crates/storage-contract/src/reports.rs
@@ -361,3 +361,35 @@ where
     );
     assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
 }
+
+pub async fn reports_between_returns_live_reports_with_their_links<S>(fx: &Fixture<S>)
+where
+    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
+        + ListRepo
+        + ReportRepo,
+{
+    let scene = scene(fx).await;
+    let (r0, r1, s0) = (scene.rooms[0].id, scene.rooms[1].id, scene.symptoms[0].id);
+    let at = |minutes: i64| start() - TimeDelta::minutes(minutes);
+    let before = add_report(fx, &scene, at(50), report_values(2, &[r0], &[])).await;
+    let first = add_report(fx, &scene, at(40), report_values(5, &[r1, r0], &[s0])).await;
+    let second = add_report(fx, &scene, at(30), report_values(6, &[], &[])).await;
+    let deleted = add_report(fx, &scene, at(20), report_values(7, &[r0], &[s0])).await;
+    let at_end = add_report(fx, &scene, at(10), report_values(8, &[r1], &[])).await;
+    let mut tx = fx.store.begin(None).await.unwrap();
+    tx.delete_report(deleted.id).await.unwrap();
+    tx.commit().await.unwrap();
+
+    let found = fx.store.reports_between(at(40), at(10)).await.unwrap();
+    assert_eq!(found, [first.clone(), second]);
+    assert_eq!(found[0].values.rooms(), [r0, r1]);
+    assert_eq!(found[0].values.symptoms(), [s0]);
+    assert!(!found.contains(&before) && !found.contains(&at_end));
+    assert!(
+        fx.store
+            .reports_between(at(5), start())
+            .await
+            .unwrap()
+            .is_empty()
+    );
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres`
Expected: compile error — `unseen_auto_closed`, `event_at`, `reports_between` and `readings_between` are not defined.

- [ ] **Step 3: Implement**

In `crates/domain/src/repos.rs`:

```diff
--- a/crates/domain/src/repos.rs
+++ b/crates/domain/src/repos.rs
@@ -105,6 +105,9 @@ pub trait EventRepo: Send + Sync {
         from: DateTime<Utc>,
         to: DateTime<Utc>,
     ) -> impl Future<Output = StoreResult<Vec<Event>>> + Send;
+    /// Events closed by the 04:00 sweep whose notice nobody has seen yet, ordered by
+    /// `opened_at`.
+    fn unseen_auto_closed(&self) -> impl Future<Output = StoreResult<Vec<Event>>> + Send;
 }
 
 pub trait ReportRepo: Send + Sync {
@@ -116,6 +119,12 @@ pub trait ReportRepo: Send + Sync {
         &self,
         event: EventId,
     ) -> impl Future<Output = StoreResult<Vec<SmellReport>>> + Send;
+    /// Reports that are not deleted, observed in `[from, to)`, ordered by observed time.
+    fn reports_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<Vec<SmellReport>>> + Send;
 }
 
 pub trait ReadingRepo: Send + Sync {
@@ -129,6 +138,12 @@ pub trait ReadingRepo: Send + Sync {
         &self,
         event: EventId,
     ) -> impl Future<Output = StoreResult<Vec<Pm25Reading>>> + Send;
+    /// Readings that are not deleted, observed in `[from, to)`, ordered by observed time.
+    fn readings_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<Vec<Pm25Reading>>> + Send;
 }
 
 pub trait JobRepo: Send + Sync {
@@ -197,6 +212,11 @@ pub trait EventWrites: Send {
     /// Reads inside the transaction, so a decision and the write that follows it see the same data.
     fn open_event(&mut self) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
     fn event(&mut self, id: EventId) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
+    /// The most recently opened event whose span contains `at`; an open event has no end yet.
+    fn event_at(
+        &mut self,
+        at: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
     /// The observed time of the event's latest report that is not deleted.
     fn latest_report_at(
         &mut self,
```

In `crates/storage-postgres/src/events.rs`:

```diff
--- a/crates/storage-postgres/src/events.rs
+++ b/crates/storage-postgres/src/events.rs
@@ -116,6 +116,21 @@ impl EventRepo for PostgresStore {
         .map(EventRow::into_event)
         .collect()
     }
+
+    async fn unseen_auto_closed(&self) -> StoreResult<Vec<Event>> {
+        sqlx::query_as!(
+            EventRow,
+            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
+             FROM events WHERE close_reason = 'auto_0400' AND auto_close_notice_seen_at IS NULL \
+             ORDER BY opened_at, id"
+        )
+        .fetch_all(&mut *self.read().await?)
+        .await
+        .db()?
+        .into_iter()
+        .map(EventRow::into_event)
+        .collect()
+    }
 }
 
 impl EventWrites for PostgresTx {
@@ -127,6 +142,21 @@ impl EventWrites for PostgresTx {
         fetch(self.conn(), id).await
     }
 
+    async fn event_at(&mut self, at: DateTime<Utc>) -> StoreResult<Option<Event>> {
+        sqlx::query_as!(
+            EventRow,
+            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
+             FROM events WHERE opened_at <= $1 AND (closed_at IS NULL OR closed_at >= $1) \
+             ORDER BY opened_at DESC, id DESC LIMIT 1",
+            micros(at),
+        )
+        .fetch_optional(self.conn())
+        .await
+        .db()?
+        .map(EventRow::into_event)
+        .transpose()
+    }
+
     async fn latest_report_at(&mut self, event: EventId) -> StoreResult<Option<DateTime<Utc>>> {
         let latest = sqlx::query_scalar!(
             "SELECT MAX(observed_at) FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL",
```

In `crates/storage-postgres/src/readings.rs`:

```diff
--- a/crates/storage-postgres/src/readings.rs
+++ b/crates/storage-postgres/src/readings.rs
@@ -95,6 +95,27 @@ impl ReadingRepo for PostgresStore {
         .map(ReadingRow::into_reading)
         .collect()
     }
+
+    async fn readings_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> StoreResult<Vec<Pm25Reading>> {
+        sqlx::query_as!(
+            ReadingRow,
+            "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
+             FROM pm25_readings WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
+             ORDER BY observed_at, id",
+            micros(from),
+            micros(to),
+        )
+        .fetch_all(&mut *self.read().await?)
+        .await
+        .db()?
+        .into_iter()
+        .map(ReadingRow::into_reading)
+        .collect()
+    }
 }
 
 impl ReadingWrites for PostgresTx {
```

In `crates/storage-postgres/src/reports.rs`:

```diff
--- a/crates/storage-postgres/src/reports.rs
+++ b/crates/storage-postgres/src/reports.rs
@@ -1,5 +1,7 @@
 //! Smell reports, with their rooms and symptoms in link tables. Deleted softly.
 
+use std::collections::HashMap;
+
 use chrono::{DateTime, Utc};
 use smoke_domain::audit::{AuditAction, AuditEntity};
 use smoke_domain::records::{NewReport, ReportValues, SmellReport};
@@ -37,6 +39,11 @@ async fn load(conn: &mut PgConnection, row: ReportRow) -> StoreResult<SmellRepor
     .fetch_all(&mut *conn)
     .await
     .db()?;
+    assemble(row, rooms, symptoms)
+}
+
+/// A report from its row and the ids in its link tables.
+fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<SmellReport> {
     let intensity = u8::try_from(row.intensity)
         .map_err(corrupt)
         .and_then(|value| Intensity::new(value).map_err(corrupt))?;
@@ -61,6 +68,15 @@ async fn load(conn: &mut PgConnection, row: ReportRow) -> StoreResult<SmellRepor
     })
 }
 
+/// Groups `(report id, linked id)` pairs by report.
+fn group(links: impl IntoIterator<Item = (i64, i64)>) -> HashMap<i64, Vec<i64>> {
+    let mut grouped: HashMap<i64, Vec<i64>> = HashMap::new();
+    for (report, linked) in links {
+        grouped.entry(report).or_default().push(linked);
+    }
+    grouped
+}
+
 async fn load_all(conn: &mut PgConnection, rows: Vec<ReportRow>) -> StoreResult<Vec<SmellReport>> {
     let mut reports = Vec::with_capacity(rows.len());
     for row in rows {
@@ -173,6 +189,66 @@ impl ReportRepo for PostgresStore {
         .db()?;
         load_all(&mut conn, rows).await
     }
+
+    /// Three queries whatever the number of reports: the rows, then all their room links and
+    /// all their symptom links.
+    async fn reports_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> StoreResult<Vec<SmellReport>> {
+        let (from, to) = (micros(from), micros(to));
+        let mut conn = self.read().await?;
+        let rows = sqlx::query_as!(
+            ReportRow,
+            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+             FROM smell_reports WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
+             ORDER BY observed_at, id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let rooms = sqlx::query!(
+            "SELECT l.smell_report_id, l.room_id FROM smell_report_rooms l \
+             JOIN smell_reports r ON r.id = l.smell_report_id \
+             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
+             ORDER BY l.smell_report_id, l.room_id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let symptoms = sqlx::query!(
+            "SELECT l.smell_report_id, l.symptom_id FROM smell_report_symptoms l \
+             JOIN smell_reports r ON r.id = l.smell_report_id \
+             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
+             ORDER BY l.smell_report_id, l.symptom_id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let mut rooms = group(rooms.into_iter().map(|l| (l.smell_report_id, l.room_id)));
+        let mut symptoms = group(
+            symptoms
+                .into_iter()
+                .map(|l| (l.smell_report_id, l.symptom_id)),
+        );
+        rows.into_iter()
+            .map(|row| {
+                let id = row.id;
+                assemble(
+                    row,
+                    rooms.remove(&id).unwrap_or_default(),
+                    symptoms.remove(&id).unwrap_or_default(),
+                )
+            })
+            .collect()
+    }
 }
 
 impl ReportWrites for PostgresTx {
```

In `crates/storage-sqlite/src/events.rs`:

```diff
--- a/crates/storage-sqlite/src/events.rs
+++ b/crates/storage-sqlite/src/events.rs
@@ -116,6 +116,21 @@ impl EventRepo for SqliteStore {
         .map(EventRow::into_event)
         .collect()
     }
+
+    async fn unseen_auto_closed(&self) -> StoreResult<Vec<Event>> {
+        sqlx::query_as!(
+            EventRow,
+            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
+             FROM events WHERE close_reason = 'auto_0400' AND auto_close_notice_seen_at IS NULL \
+             ORDER BY opened_at, id"
+        )
+        .fetch_all(&mut *self.read().await?)
+        .await
+        .db()?
+        .into_iter()
+        .map(EventRow::into_event)
+        .collect()
+    }
 }
 
 impl EventWrites for SqliteTx {
@@ -127,6 +142,21 @@ impl EventWrites for SqliteTx {
         fetch(self.conn(), id).await
     }
 
+    async fn event_at(&mut self, at: DateTime<Utc>) -> StoreResult<Option<Event>> {
+        sqlx::query_as!(
+            EventRow,
+            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
+             FROM events WHERE opened_at <= $1 AND (closed_at IS NULL OR closed_at >= $1) \
+             ORDER BY opened_at DESC, id DESC LIMIT 1",
+            micros(at),
+        )
+        .fetch_optional(self.conn())
+        .await
+        .db()?
+        .map(EventRow::into_event)
+        .transpose()
+    }
+
     async fn latest_report_at(&mut self, event: EventId) -> StoreResult<Option<DateTime<Utc>>> {
         let latest = sqlx::query_scalar!(
             "SELECT MAX(observed_at) FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL",
```

In `crates/storage-sqlite/src/readings.rs`:

```diff
--- a/crates/storage-sqlite/src/readings.rs
+++ b/crates/storage-sqlite/src/readings.rs
@@ -95,6 +95,27 @@ impl ReadingRepo for SqliteStore {
         .map(ReadingRow::into_reading)
         .collect()
     }
+
+    async fn readings_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> StoreResult<Vec<Pm25Reading>> {
+        sqlx::query_as!(
+            ReadingRow,
+            "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
+             FROM pm25_readings WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
+             ORDER BY observed_at, id",
+            micros(from),
+            micros(to),
+        )
+        .fetch_all(&mut *self.read().await?)
+        .await
+        .db()?
+        .into_iter()
+        .map(ReadingRow::into_reading)
+        .collect()
+    }
 }
 
 impl ReadingWrites for SqliteTx {
```

In `crates/storage-sqlite/src/reports.rs`:

```diff
--- a/crates/storage-sqlite/src/reports.rs
+++ b/crates/storage-sqlite/src/reports.rs
@@ -1,5 +1,7 @@
 //! Smell reports, with their rooms and symptoms in link tables. Deleted softly.
 
+use std::collections::HashMap;
+
 use chrono::{DateTime, Utc};
 use smoke_domain::audit::{AuditAction, AuditEntity};
 use smoke_domain::records::{NewReport, ReportValues, SmellReport};
@@ -37,6 +39,11 @@ async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellR
     .fetch_all(&mut *conn)
     .await
     .db()?;
+    assemble(row, rooms, symptoms)
+}
+
+/// A report from its row and the ids in its link tables.
+fn assemble(row: ReportRow, rooms: Vec<i64>, symptoms: Vec<i64>) -> StoreResult<SmellReport> {
     let intensity = u8::try_from(row.intensity)
         .map_err(corrupt)
         .and_then(|value| Intensity::new(value).map_err(corrupt))?;
@@ -61,6 +68,15 @@ async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellR
     })
 }
 
+/// Groups `(report id, linked id)` pairs by report.
+fn group(links: impl IntoIterator<Item = (i64, i64)>) -> HashMap<i64, Vec<i64>> {
+    let mut grouped: HashMap<i64, Vec<i64>> = HashMap::new();
+    for (report, linked) in links {
+        grouped.entry(report).or_default().push(linked);
+    }
+    grouped
+}
+
 async fn load_all(
     conn: &mut SqliteConnection,
     rows: Vec<ReportRow>,
@@ -180,6 +196,66 @@ impl ReportRepo for SqliteStore {
         .db()?;
         load_all(&mut conn, rows).await
     }
+
+    /// Three queries whatever the number of reports: the rows, then all their room links and
+    /// all their symptom links.
+    async fn reports_between(
+        &self,
+        from: DateTime<Utc>,
+        to: DateTime<Utc>,
+    ) -> StoreResult<Vec<SmellReport>> {
+        let (from, to) = (micros(from), micros(to));
+        let mut conn = self.read().await?;
+        let rows = sqlx::query_as!(
+            ReportRow,
+            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
+             FROM smell_reports WHERE deleted_at IS NULL AND observed_at >= $1 AND observed_at < $2 \
+             ORDER BY observed_at, id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let rooms = sqlx::query!(
+            "SELECT l.smell_report_id, l.room_id FROM smell_report_rooms l \
+             JOIN smell_reports r ON r.id = l.smell_report_id \
+             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
+             ORDER BY l.smell_report_id, l.room_id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let symptoms = sqlx::query!(
+            "SELECT l.smell_report_id, l.symptom_id FROM smell_report_symptoms l \
+             JOIN smell_reports r ON r.id = l.smell_report_id \
+             WHERE r.deleted_at IS NULL AND r.observed_at >= $1 AND r.observed_at < $2 \
+             ORDER BY l.smell_report_id, l.symptom_id",
+            from,
+            to,
+        )
+        .fetch_all(&mut *conn)
+        .await
+        .db()?;
+        let mut rooms = group(rooms.into_iter().map(|l| (l.smell_report_id, l.room_id)));
+        let mut symptoms = group(
+            symptoms
+                .into_iter()
+                .map(|l| (l.smell_report_id, l.symptom_id)),
+        );
+        rows.into_iter()
+            .map(|row| {
+                let id = row.id;
+                assemble(
+                    row,
+                    rooms.remove(&id).unwrap_or_default(),
+                    symptoms.remove(&id).unwrap_or_default(),
+                )
+            })
+            .collect()
+    }
 }
 
 impl ReportWrites for SqliteTx {
```

Regenerate the offline query metadata (the new `query!` calls do not compile offline until this has run):

```bash
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres scripts/sqlx-prepare.sh
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass on both backends, including `unseen_auto_closed_events_are_listed_until_seen`, `event_at_finds_the_event_whose_span_contains_the_time`, `reports_between_returns_live_reports_with_their_links` and `readings_between_returns_live_readings_in_the_window`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/repos.rs \
  crates/storage-contract/src/events.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/readings.rs \
  crates/storage-contract/src/reports.rs \
  crates/storage-postgres/src/events.rs \
  crates/storage-postgres/src/readings.rs \
  crates/storage-postgres/src/reports.rs \
  crates/storage-sqlite/src/events.rs \
  crates/storage-sqlite/src/readings.rs \
  crates/storage-sqlite/src/reports.rs \
  crates/storage-sqlite/.sqlx \
  crates/storage-postgres/.sqlx
git commit -m "feat(storage): time-window reads, the event at an instant and unseen auto-close notices

reports_between loads the room and symptom links of all its reports with two
joined queries instead of two per report, so a month of reports costs three
round trips on a remote Postgres, not hundreds (the N+1 query problem).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Domain — the 04:00 auto-close sweep

The sweep of spec §11 as a catch-up job: due when a 04:00 (local) boundary has passed since the
last run; it closes the open event only if a boundary has passed since the event opened, at its
last report, and records the run in the same transaction. The result is identical at 04:00 or at
08:00 after the host slept. Tested on both backends, including both DST days of 2026 in Rome.

**Files:**
- Create: `crates/domain/src/sweep.rs`, `crates/storage-contract/src/sweep.rs`
- Modify: `crates/domain/src/lib.rs`, `crates/storage-contract/Cargo.toml`, `crates/storage-contract/src/lib.rs`

**Interfaces:**
- Consumes: `DailySchedule::boundary_passed_since`, `JobRepo::job_last_run`,
  `JobWrites::record_job_run`, `EventWrites::{open_event, latest_report_at, close_event}`.
- Produces: `smoke_domain::sweep::{AUTO_CLOSE_JOB, SweepOutcome::{NotDue, Ran { closed }},
  auto_close(store, &DailySchedule, now) -> StoreResult<SweepOutcome>}`.

- [ ] **Step 1: Write the failing tests**

In `crates/domain/src/lib.rs`:

```diff
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -12,6 +12,7 @@ pub mod records;
 pub mod repos;
 pub mod schedule;
 pub mod starter;
+pub mod sweep;
 pub mod types;
 pub mod verify;
 
```

In `crates/storage-contract/Cargo.toml`:

```diff
--- a/crates/storage-contract/Cargo.toml
+++ b/crates/storage-contract/Cargo.toml
@@ -10,6 +10,7 @@ repository.workspace = true
 
 [dependencies]
 chrono.workspace = true
+chrono-tz.workspace = true
 serde_json.workspace = true
 smoke-domain.workspace = true
 tokio.workspace = true
```

In `crates/storage-contract/src/lib.rs`:

```diff
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -15,6 +15,7 @@ pub mod lists;
 pub mod readings;
 pub mod reports;
 pub mod sessions;
+pub mod sweep;
 pub mod transactions;
 pub mod users;
 
@@ -93,6 +94,12 @@ macro_rules! contract_tests {
             sessions::expiring_a_user_ends_their_sessions,
             sessions::a_rolled_back_password_change_keeps_the_sessions,
             sessions::sessions_are_not_audited,
+            sweep::the_first_sweep_runs_and_is_remembered,
+            sweep::a_sweep_at_four_closes_the_event_at_its_last_report,
+            sweep::a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four,
+            sweep::an_event_opened_after_the_boundary_stays_open,
+            sweep::an_event_without_reports_closes_where_it_opened,
+            sweep::on_dst_days_the_sweep_still_runs_at_four_local,
         );
     };
     (@each $fixture:path; $($module:ident::$test:ident),+ $(,)?) => {
```

Create `crates/storage-contract/src/sweep.rs`:

```rust
//! The 04:00 auto-close sweep on a real store (spec §11, §13.3: "sweep at 04:00 vs catch-up at
//! 08:00; DST days").

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Event, NewReport};
use smoke_domain::repos::{
    AuditRepo, Commit, EventRepo, EventWrites, JobRepo, JobWrites, ReportWrites, Transactional,
    UserWrites,
};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::sweep::{AUTO_CLOSE_JOB, SweepOutcome, auto_close};
use smoke_domain::types::{CloseReason, Role, UserId};

use crate::Fixture;
use crate::fixture::{add_event, add_user, last_entry, report_values};

fn t(s: &str) -> DateTime<Utc> {
    s.parse().expect("valid instant")
}

fn rome() -> DailySchedule {
    DailySchedule::new(chrono_tz::Europe::Rome, 4).expect("valid schedule")
}

async fn report_at<S>(fx: &Fixture<S>, user: UserId, event: &Event, at: DateTime<Utc>)
where
    S: Transactional<Tx: ReportWrites>,
{
    let mut tx = fx.store.begin(Some(user)).await.unwrap();
    tx.create_report(NewReport {
        event_id: event.id,
        user_id: user,
        observed_at: at,
        values: report_values(5, &[], &[]),
    })
    .await
    .unwrap();
    tx.commit().await.unwrap();
}

async fn sweep_at<S>(fx: &Fixture<S>, now: DateTime<Utc>) -> SweepOutcome
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    fx.clock.set(now);
    auto_close(&fx.store, &rome(), fx.clock.now())
        .await
        .unwrap()
}

/// An evening event (21:00–22:30 Rome) on the night before `date`, with no sweep since.
async fn evening_event<S>(fx: &Fixture<S>, user: UserId, date: &str) -> Event
where
    S: Transactional<Tx: EventWrites + ReportWrites + JobWrites> + JobRepo,
{
    let opened = t(&format!("{date}T20:00:00Z")) - TimeDelta::days(1);
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(AUTO_CLOSE_JOB, opened).await.unwrap();
    tx.commit().await.unwrap();
    let event = add_event(fx, user, opened, None).await;
    report_at(fx, user, &event, opened).await;
    report_at(fx, user, &event, opened + TimeDelta::minutes(90)).await;
    event
}

pub async fn the_first_sweep_runs_and_is_remembered<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    let now = t("2026-01-15T12:00:00Z");
    assert_eq!(sweep_at(fx, now).await, SweepOutcome::Ran { closed: None });
    assert_eq!(
        fx.store.job_last_run(AUTO_CLOSE_JOB).await.unwrap(),
        Some(now)
    );
    assert_eq!(
        sweep_at(fx, now + TimeDelta::hours(10)).await,
        SweepOutcome::NotDue
    );
}

pub async fn a_sweep_at_four_closes_the_event_at_its_last_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites>
        + JobRepo
        + EventRepo
        + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let event = evening_event(fx, user.id, "2026-01-16").await;
    assert_eq!(
        sweep_at(fx, t("2026-01-16T02:59:59Z")).await,
        SweepOutcome::NotDue,
        "03:59:59 in Rome"
    );
    assert_eq!(
        sweep_at(fx, t("2026-01-16T03:00:00Z")).await,
        SweepOutcome::Ran {
            closed: Some(event.id)
        }
    );
    let closed = fx
        .store
        .event(event.id)
        .await
        .unwrap()
        .unwrap()
        .closed
        .unwrap();
    assert_eq!(closed.at, event.opened_at + TimeDelta::minutes(90));
    assert_eq!(closed.reason, CloseReason::Auto0400);
    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.action, entry.actor_id),
        (AuditEntity::Event, AuditAction::Close, None)
    );
}

pub async fn a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let first = evening_event(fx, user.id, "2026-01-16").await;
    sweep_at(fx, t("2026-01-16T03:00:00Z")).await;
    let second = evening_event(fx, user.id, "2026-01-18").await;
    assert_eq!(
        sweep_at(fx, t("2026-01-18T07:00:00Z")).await,
        SweepOutcome::Ran {
            closed: Some(second.id)
        },
        "08:00 in Rome, after the host slept through 04:00"
    );
    let span = |event: Event| {
        let closed = event.closed.unwrap();
        (closed.at - event.opened_at, closed.reason)
    };
    let at_four = span(fx.store.event(first.id).await.unwrap().unwrap());
    let at_eight = span(fx.store.event(second.id).await.unwrap().unwrap());
    assert_eq!(at_four, at_eight);
    assert_eq!(at_eight, (TimeDelta::minutes(90), CloseReason::Auto0400));
}

pub async fn an_event_opened_after_the_boundary_stays_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(AUTO_CLOSE_JOB, t("2026-01-15T12:00:00Z"))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let early_morning = add_event(fx, user.id, t("2026-01-16T04:00:00Z"), None).await;
    assert_eq!(
        sweep_at(fx, t("2026-01-16T06:00:00Z")).await,
        SweepOutcome::Ran { closed: None }
    );
    assert!(
        fx.store
            .event(early_morning.id)
            .await
            .unwrap()
            .unwrap()
            .is_open()
    );
}

pub async fn an_event_without_reports_closes_where_it_opened<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let opened = t("2026-01-15T20:00:00Z");
    let event = add_event(fx, user.id, opened, None).await;
    sweep_at(fx, t("2026-01-16T09:00:00Z")).await;
    let closed = fx.store.event(event.id).await.unwrap().unwrap().closed;
    assert_eq!(closed.map(|c| c.at), Some(opened));
}

pub async fn on_dst_days_the_sweep_still_runs_at_four_local<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites + ReportWrites + JobWrites> + JobRepo + EventRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    // Spring forward: 04:00 CEST is 02:00 UTC. Fall back: 04:00 CET is 03:00 UTC.
    for (date, before, at_four) in [
        ("2026-03-29", "2026-03-29T01:59:00Z", "2026-03-29T02:00:00Z"),
        ("2026-10-25", "2026-10-25T02:59:00Z", "2026-10-25T03:00:00Z"),
    ] {
        let event = evening_event(fx, user.id, date).await;
        assert_eq!(
            sweep_at(fx, t(before)).await,
            SweepOutcome::NotDue,
            "{date}"
        );
        assert_eq!(
            sweep_at(fx, t(at_four)).await,
            SweepOutcome::Ran {
                closed: Some(event.id)
            },
            "{date}"
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres`
Expected: compile error — `smoke_domain::sweep` does not exist.

- [ ] **Step 3: Implement**

Create `crates/domain/src/sweep.rs`:

```rust
//! The 04:00 auto-close sweep (spec §7.3, §11): a catch-up job, so the result is the same
//! whether it runs at 04:00 sharp or hours later after the host slept.

use chrono::{DateTime, Utc};

use crate::records::Closure;
use crate::repos::{Commit, EventWrites, JobRepo, JobWrites, StoreResult, Transactional};
use crate::schedule::DailySchedule;
use crate::types::{CloseReason, EventId};

/// The `jobs` row that remembers the last sweep.
pub const AUTO_CLOSE_JOB: &str = "auto_close_sweep";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepOutcome {
    /// No boundary has passed since the last sweep.
    NotDue,
    /// The sweep ran; `closed` is the event it closed, if one was open across a boundary.
    Ran { closed: Option<EventId> },
}

/// Runs the sweep if a boundary has passed since the last run (or it never ran). An open event
/// that started before the latest boundary is closed at its last report (`auto_0400`); one
/// that started after it is left open. The close and the new `last_run_at` commit together.
pub async fn auto_close<S>(
    store: &S,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> StoreResult<SweepOutcome>
where
    S: Transactional<Tx: EventWrites + JobWrites> + JobRepo,
{
    if let Some(last) = store.job_last_run(AUTO_CLOSE_JOB).await?
        && !schedule.boundary_passed_since(last, now)
    {
        return Ok(SweepOutcome::NotDue);
    }
    let mut tx = store.begin(None).await?;
    let mut closed = None;
    if let Some(event) = tx.open_event().await?
        && schedule.boundary_passed_since(event.opened_at, now)
    {
        let last_report = tx.latest_report_at(event.id).await?;
        let at = last_report.map_or(event.opened_at, |at| at.max(event.opened_at));
        let reason = CloseReason::Auto0400;
        tx.close_event(event.id, Closure { at, reason }).await?;
        closed = Some(event.id);
    }
    tx.record_job_run(AUTO_CLOSE_JOB, now).await?;
    tx.commit().await?;
    Ok(SweepOutcome::Ran { closed })
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass on both backends, including `a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four`, `an_event_opened_after_the_boundary_stays_open` and `on_dst_days_the_sweep_still_runs_at_four_local`; no warnings.

- [ ] **Step 5: Commit**

Include the `Cargo.lock` that `cargo build` updates.

```bash
git add crates/domain/src/lib.rs \
  crates/domain/src/sweep.rs \
  crates/storage-contract/Cargo.toml \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/sweep.rs \
  Cargo.lock
git commit -m "feat(domain): the 04:00 auto-close sweep as an idempotent catch-up job

auto_close closes the open event at its last report only when a boundary has
passed since both the last run and the event's start, and records the run in
the same transaction, so a sweep at 04:00 and a catch-up at 08:00 leave
identical data. Contract tests cover both backends and the two DST days.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: App — the sweep at startup and on a timer; the schedule moves into WebConfig

`serve` spawns a task that runs the sweep at once and then every five minutes. The schedule
(`APP_TIMEZONE`, `AUTO_CLOSE_HOUR`) moves from `ServeConfig` into `WebConfig`, because the pages
show and read local times and the logging routes run the sweep too.

**Files:**
- Modify: `crates/app/Cargo.toml`, `crates/app/src/commands/serve.rs`, `crates/app/src/config.rs`, `crates/web/Cargo.toml`, `crates/web/src/config.rs`, `crates/web/tests/common/mod.rs`

**Interfaces:**
- Consumes: `smoke_domain::sweep::auto_close` (Task 2).
- Produces: `smoke_web::config::WebConfig.schedule: DailySchedule`; `ServeConfig` no longer has
  `schedule` (use `config.web.schedule`); private `serve::sweep_events(store, clock, schedule)`.

- [ ] **Step 1: Write the failing tests**

In `crates/app/Cargo.toml`:

```diff
--- a/crates/app/Cargo.toml
+++ b/crates/app/Cargo.toml
@@ -31,6 +31,7 @@ tracing.workspace = true
 tracing-subscriber.workspace = true
 
 [dev-dependencies]
+chrono.workspace = true
 sqlx = { workspace = true, features = ["sqlite-bundled"] }
 tempfile.workspace = true
 
```

In `crates/app/src/commands/serve.rs`, the tests:

```diff
--- a/crates/app/src/commands/serve.rs
+++ b/crates/app/src/commands/serve.rs
@@ -78,3 +104,59 @@ async fn shutdown_signal() {
     }
     info!("shutting down");
 }
+
+#[cfg(test)]
+mod tests {
+    use chrono::{DateTime, TimeDelta, Utc};
+    use smoke_domain::clock::FakeClock;
+    use smoke_domain::records::{NewEvent, NewUser};
+    use smoke_domain::repos::{Commit, EventRepo, EventWrites, Transactional, UserWrites};
+    use smoke_domain::types::{CloseReason, Locale, Role, Username};
+    use smoke_storage_sqlite::SqliteStore;
+
+    use super::*;
+
+    #[tokio::test]
+    async fn the_event_sweep_runs_as_soon_as_the_server_starts() {
+        let dir = tempfile::tempdir().unwrap();
+        let url = format!("sqlite://{}", dir.path().join("serve.db").display());
+        let evening: DateTime<Utc> = "2026-01-15T20:00:00Z".parse().unwrap();
+        let clock = Arc::new(FakeClock::new(evening));
+        let store = SqliteStore::connect(&url, clock.clone()).await.unwrap();
+        store.migrate().await.unwrap();
+        let mut tx = store.begin(None).await.unwrap();
+        let owner = tx
+            .create_user(NewUser {
+                username: Username::new("daniele").unwrap(),
+                password_hash: None,
+                role: Role::Resident,
+                locale: Locale::En,
+                expires_at: None,
+            })
+            .await
+            .unwrap();
+        let event = tx
+            .create_event(NewEvent {
+                owner_id: owner.id,
+                opened_at: evening,
+                closed: None,
+            })
+            .await
+            .unwrap();
+        tx.commit().await.unwrap();
+        clock.advance(TimeDelta::hours(12));
+
+        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
+        let task = tokio::spawn(sweep_events(store.clone(), clock, schedule));
+        let mut closed = None;
+        for _ in 0..100 {
+            closed = store.event(event.id).await.unwrap().unwrap().closed;
+            if closed.is_some() {
+                break;
+            }
+            tokio::time::sleep(Duration::from_millis(20)).await;
+        }
+        task.abort();
+        assert_eq!(closed.map(|c| c.reason), Some(CloseReason::Auto0400));
+    }
+}
```

In `crates/app/src/config.rs`, the tests:

```diff
--- a/crates/app/src/config.rs
+++ b/crates/app/src/config.rs
@@ -119,8 +117,8 @@ mod tests {
             ServeConfig::from_lookup(lookup(&[("APP_BASE_URL", "https://smoke.example.org/")]))
                 .unwrap();
         assert_eq!(config.web.base_url.origin(), "https://smoke.example.org");
-        assert_eq!(config.schedule.tz(), chrono_tz::Europe::Rome);
-        assert_eq!(config.schedule.hour(), 4);
+        assert_eq!(config.web.schedule.tz(), chrono_tz::Europe::Rome);
+        assert_eq!(config.web.schedule.hour(), 4);
         assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
         assert_eq!(config.web.client_ip_header, None);
         assert_eq!(
@@ -139,8 +137,8 @@ mod tests {
             ("CLIENT_IP_HEADER", "Fly-Client-IP"),
         ]))
         .unwrap();
-        assert_eq!(config.schedule.tz(), chrono_tz::Europe::Istanbul);
-        assert_eq!(config.schedule.hour(), 5);
+        assert_eq!(config.web.schedule.tz(), chrono_tz::Europe::Istanbul);
+        assert_eq!(config.web.schedule.hour(), 5);
         assert_eq!(config.bind_addr, "0.0.0.0:3000".parse().unwrap());
         assert_eq!(config.web.client_ip_header.unwrap(), "fly-client-ip");
     }
```

In `crates/web/tests/common/mod.rs`:

```diff
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -15,6 +15,7 @@ use http_body_util::BodyExt;
 use smoke_domain::clock::{Clock, FakeClock, SystemClock};
 use smoke_domain::records::{NewUser, User};
 use smoke_domain::repos::{Commit, Transactional, UserWrites};
+use smoke_domain::schedule::DailySchedule;
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
 use smoke_web::config::{BaseUrl, WebConfig};
@@ -50,6 +51,7 @@ impl TestApp {
         let config = WebConfig {
             base_url: BaseUrl::parse(base_url).unwrap(),
             client_ip_header: client_ip_header.map(HeaderName::from_static),
+            schedule: DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap(),
         };
         let router = smoke_web::router(AppState::new(store.clone(), clock.clone(), config));
         Self {
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-tracker -p smoke-web`
Expected: compile error — `WebConfig` has no field `schedule`; `sweep_events` is not defined.

- [ ] **Step 3: Implement**

In `crates/app/src/commands/serve.rs`:

```diff
--- a/crates/app/src/commands/serve.rs
+++ b/crates/app/src/commands/serve.rs
@@ -6,6 +6,8 @@ use std::time::Duration;
 use anyhow::Context;
 use smoke_domain::clock::Clock;
 use smoke_domain::repos::{SessionRepo, Store};
+use smoke_domain::schedule::DailySchedule;
+use smoke_domain::sweep::{SweepOutcome, auto_close};
 use smoke_web::{AppState, DbSessions};
 use tokio::net::TcpListener;
 use tower_sessions::session_store::ExpiredDeletion;
@@ -16,6 +18,10 @@ use crate::config::ServeConfig;
 /// How often expired sessions are deleted.
 const SESSION_SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);
 
+/// How often the auto-close sweep checks whether a 04:00 boundary has passed. A check that
+/// finds nothing to do is one small read.
+const EVENT_SWEEP_EVERY: Duration = Duration::from_secs(5 * 60);
+
 pub async fn run<S: Store + Clone + 'static>(
     store: S,
     clock: Arc<dyn Clock>,
@@ -27,20 +33,40 @@ pub async fn run<S: Store + Clone + 'static>(
     info!(
         address = %listener.local_addr()?,
         origin = config.web.base_url.origin(),
-        timezone = %config.schedule.tz(),
-        auto_close_hour = config.schedule.hour(),
+        timezone = %config.web.schedule.tz(),
+        auto_close_hour = config.web.schedule.hour(),
         "listening"
     );
     let sweeper = tokio::spawn(sweep_sessions(DbSessions::new(
         store.clone(),
         clock.clone(),
     )));
+    let closer = tokio::spawn(sweep_events(
+        store.clone(),
+        clock.clone(),
+        config.web.schedule,
+    ));
     let app = smoke_web::router(AppState::new(store, clock, config.web));
     let served = smoke_web::serve(listener, app, shutdown_signal()).await;
     sweeper.abort();
+    closer.abort();
     served.context("the server stopped with an error")
 }
 
+/// Runs the auto-close sweep at startup and then every few minutes (spec §11). Logging
+/// requests run it too, so a sleeping host catches up on its first request.
+async fn sweep_events<S: Store>(store: S, clock: Arc<dyn Clock>, schedule: DailySchedule) {
+    let mut ticks = tokio::time::interval(EVENT_SWEEP_EVERY);
+    loop {
+        ticks.tick().await;
+        match auto_close(&store, &schedule, clock.now()).await {
+            Ok(SweepOutcome::Ran { closed: Some(id) }) => info!(event = id.0, "auto-closed"),
+            Ok(_) => {}
+            Err(error) => warn!(%error, "the auto-close sweep failed"),
+        }
+    }
+}
+
 /// Deletes expired sessions at startup and then every hour, through tower-sessions'
 /// [`ExpiredDeletion`]. A failed run is logged and the next one still happens.
 async fn sweep_sessions<S: SessionRepo + Clone + 'static>(sessions: DbSessions<S>) {
```

In `crates/app/src/config.rs`:

```diff
--- a/crates/app/src/config.rs
+++ b/crates/app/src/config.rs
@@ -32,8 +32,6 @@ pub fn database_url(get: impl Fn(&str) -> Option<String>) -> Result<String, Conf
 #[derive(Debug, Clone)]
 pub struct ServeConfig {
     pub web: WebConfig,
-    /// The local time zone and the auto-close hour (used by the sweep in milestone 4).
-    pub schedule: DailySchedule,
     pub bind_addr: SocketAddr,
 }
 
@@ -79,8 +77,8 @@ impl ServeConfig {
             web: WebConfig {
                 base_url,
                 client_ip_header,
+                schedule,
             },
-            schedule,
             bind_addr,
         })
     }
```

In `crates/web/Cargo.toml`:

```diff
--- a/crates/web/Cargo.toml
+++ b/crates/web/Cargo.toml
@@ -14,6 +14,7 @@ askama.workspace = true
 async-trait.workspace = true
 axum.workspace = true
 chrono.workspace = true
+chrono-tz.workspace = true
 fluent-templates.workspace = true
 getrandom.workspace = true
 hex.workspace = true
```

In `crates/web/src/config.rs`:

```diff
--- a/crates/web/src/config.rs
+++ b/crates/web/src/config.rs
@@ -3,6 +3,7 @@
 use std::net::IpAddr;
 
 use axum::http::{HeaderName, Uri};
+use smoke_domain::schedule::DailySchedule;
 use thiserror::Error;
 
 /// The public address of the app (`APP_BASE_URL`). It decides the cookie flags, HSTS, and the
@@ -85,6 +86,9 @@ pub struct WebConfig {
     /// A header set by a trusted reverse proxy that carries the client's address
     /// (`CLIENT_IP_HEADER`). Without it the peer address is used.
     pub client_ip_header: Option<HeaderName>,
+    /// The local time zone pages show and read times in, and the daily auto-close hour
+    /// (`APP_TIMEZONE`, `AUTO_CLOSE_HOUR`).
+    pub schedule: DailySchedule,
 }
 
 #[cfg(test)]
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-tracker -p smoke-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `the_event_sweep_runs_as_soon_as_the_server_starts` and the updated `only_the_base_url_is_required_to_serve`; no warnings.

- [ ] **Step 5: Commit**

Include the `Cargo.lock` that `cargo build` updates.

```bash
git add crates/app/Cargo.toml \
  crates/app/src/commands/serve.rs \
  crates/app/src/config.rs \
  crates/web/Cargo.toml \
  crates/web/src/config.rs \
  crates/web/tests/common/mod.rs \
  Cargo.lock
git commit -m "feat(app): run the auto-close sweep at startup and every five minutes

The schedule moves into WebConfig, since pages show and read local times and
the logging routes run the sweep too. The timer is a tokio interval whose
first tick is immediate, so a restarted server catches up at once.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Domain — local days and months, joinable events, who sees what

Pure helpers the use cases and pages share: the local calendar day and month of an instant (23-
and 25-hour DST days included), which events a report in the past may belong to (spec §7.5:
"the events on that day", decision #89), who gets an auto-close notice (spec §7.4), and the
guests' "own data only" rule (spec §4).

**Files:**
- Modify: `crates/domain/src/events.rs`, `crates/domain/src/permissions.rs`, `crates/domain/src/schedule.rs`

**Interfaces:**
- Consumes: `DailySchedule`, `Event`, `Actor`, `can`.
- Produces:
  - `DailySchedule::{local_date(at) -> NaiveDate, day_span(date) -> (start, end),
    month_span(date) -> (start, end)}`.
  - `smoke_domain::permissions::can_see(&Actor, owner: UserId, now) -> bool`.
  - `smoke_domain::events::{can_join_backdated(&Event, observed_at, &DailySchedule) -> bool,
    auto_close_notice_is_for(viewer: &Actor, owner: &Actor, now) -> bool}`.

- [ ] **Step 1: Write the failing tests**

In `crates/domain/src/events.rs`, the tests:

```diff
--- a/crates/domain/src/events.rs
+++ b/crates/domain/src/events.rs
@@ -125,6 +146,8 @@ mod tests {
     use chrono_tz::Europe::Rome;
 
     use super::*;
+    use crate::records::Closure;
+    use crate::types::{CloseReason, UserId};
 
     fn t(s: &str) -> DateTime<Utc> {
         s.parse().unwrap()
@@ -328,6 +351,74 @@ mod tests {
         );
     }
 
+    // --- can_join_backdated ---
+
+    fn event(opened: &str, closed: Option<&str>) -> Event {
+        Event {
+            id: EventId(1),
+            owner_id: UserId(1),
+            opened_at: t(opened),
+            closed: closed.map(|at| Closure {
+                at: t(at),
+                reason: CloseReason::Manual,
+            }),
+            auto_close_notice_seen_at: None,
+            recorded_at: t(opened),
+        }
+    }
+
+    #[test]
+    fn a_backdated_report_joins_events_that_had_started_and_ran_that_day() {
+        let s = schedule();
+        let evening = event("2026-01-14T18:00:00Z", Some("2026-01-14T19:00:00Z"));
+        assert!(can_join_backdated(&evening, t("2026-01-14T18:30:00Z"), &s));
+        assert!(
+            can_join_backdated(&evening, t("2026-01-14T22:00:00Z"), &s),
+            "later that day: the end will move"
+        );
+        assert!(!can_join_backdated(&evening, t("2026-01-14T17:59:00Z"), &s));
+        assert!(
+            !can_join_backdated(&evening, t("2026-01-15T10:00:00Z"), &s),
+            "the next day"
+        );
+        let open = event("2026-01-14T18:00:00Z", None);
+        assert!(can_join_backdated(&open, t("2026-01-15T10:00:00Z"), &s));
+    }
+
+    #[test]
+    fn an_event_running_past_midnight_belongs_to_both_days() {
+        let s = schedule();
+        let late = event("2026-01-14T22:30:00Z", Some("2026-01-15T00:30:00Z"));
+        assert!(can_join_backdated(&late, t("2026-01-15T00:10:00Z"), &s));
+        assert!(can_join_backdated(&late, t("2026-01-15T09:00:00Z"), &s));
+    }
+
+    // --- auto_close_notice_is_for ---
+
+    fn person(id: i64, role: Role, expires_at: Option<&str>) -> Actor {
+        Actor {
+            id: UserId(id),
+            role,
+            expires_at: expires_at.map(t),
+        }
+    }
+
+    #[test]
+    fn the_notice_goes_to_the_owner_and_to_residents_for_guests() {
+        let resident = person(1, Role::Resident, None);
+        let partner = person(2, Role::Resident, None);
+        let guest = person(3, Role::Guest, Some("2026-02-01T00:00:00Z"));
+        let viewer = person(4, Role::Viewer, None);
+        assert!(auto_close_notice_is_for(&resident, &resident, now()));
+        assert!(!auto_close_notice_is_for(&partner, &resident, now()));
+        assert!(!auto_close_notice_is_for(&guest, &resident, now()));
+        assert!(auto_close_notice_is_for(&guest, &guest, now()));
+        assert!(auto_close_notice_is_for(&partner, &guest, now()));
+        assert!(!auto_close_notice_is_for(&viewer, &guest, now()));
+        let expired = person(3, Role::Guest, Some("2026-01-01T00:00:00Z"));
+        assert!(!auto_close_notice_is_for(&expired, &expired, now()));
+    }
+
     // --- large_gap_indices ---
 
     #[test]
```

In `crates/domain/src/permissions.rs`, the tests:

```diff
--- a/crates/domain/src/permissions.rs
+++ b/crates/domain/src/permissions.rs
@@ -161,6 +167,22 @@ mod tests {
         }
     }
 
+    #[test]
+    fn guests_see_their_own_entries_and_everyone_else_sees_all() {
+        let cases = [
+            (actor(Role::Resident, None), true),
+            (actor(Role::Viewer, None), true),
+            (actor(Role::Guest, active()), false),
+            (actor(Role::Guest, expired()), false),
+            (actor(Role::Viewer, expired()), false),
+        ];
+        for (who, sees_others) in cases {
+            assert_eq!(can_see(&who, OTHER, now()), sees_others, "{who:?}");
+            let sees_own = !(who.role == Role::Viewer && who.is_expired(now()));
+            assert_eq!(can_see(&who, ME, now()), sees_own, "{who:?}");
+        }
+    }
+
     #[test]
     fn expiry_at_the_exact_instant_counts_as_expired() {
         let guest = actor(Role::Guest, Some(now()));
```

In `crates/domain/src/schedule.rs`, the tests:

```diff
--- a/crates/domain/src/schedule.rs
+++ b/crates/domain/src/schedule.rs
@@ -180,6 +203,52 @@ mod tests {
         assert!(!s.boundary_passed_since(t("2026-01-15T04:00:00Z"), t("2026-01-15T05:00:00Z")));
     }
 
+    fn date(s: &str) -> NaiveDate {
+        s.parse().unwrap()
+    }
+
+    #[test]
+    fn instants_fall_on_their_local_date() {
+        let s = rome_at(4);
+        assert_eq!(s.local_date(t("2026-01-15T22:59:59Z")), date("2026-01-15"));
+        assert_eq!(s.local_date(t("2026-01-15T23:00:00Z")), date("2026-01-16"));
+    }
+
+    #[test]
+    fn a_local_day_runs_from_midnight_to_midnight() {
+        let s = rome_at(4);
+        assert_eq!(
+            s.day_span(date("2026-01-15")),
+            (t("2026-01-14T23:00:00Z"), t("2026-01-15T23:00:00Z"))
+        );
+        // Spring forward: 23 hours. Fall back: 25 hours.
+        assert_eq!(
+            s.day_span(date("2026-03-29")),
+            (t("2026-03-28T23:00:00Z"), t("2026-03-29T22:00:00Z"))
+        );
+        assert_eq!(
+            s.day_span(date("2026-10-25")),
+            (t("2026-10-24T22:00:00Z"), t("2026-10-25T23:00:00Z"))
+        );
+    }
+
+    #[test]
+    fn a_local_month_runs_from_its_first_midnight_to_the_next_months() {
+        let s = rome_at(4);
+        assert_eq!(
+            s.month_span(date("2026-01-15")),
+            (t("2025-12-31T23:00:00Z"), t("2026-01-31T23:00:00Z"))
+        );
+        assert_eq!(
+            s.month_span(date("2026-12-31")),
+            (t("2026-11-30T23:00:00Z"), t("2026-12-31T23:00:00Z"))
+        );
+        assert_eq!(
+            s.month_span(date("2026-03-01")),
+            (t("2026-02-28T23:00:00Z"), t("2026-03-31T22:00:00Z"))
+        );
+    }
+
     #[test]
     fn catch_up_after_sleeping_through_several_nights_is_still_one_sweep() {
         let s = rome_at(4);
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compile error — `local_date`, `day_span`, `month_span`, `can_see`, `can_join_backdated` and `auto_close_notice_is_for` are not defined.

- [ ] **Step 3: Implement**

In `crates/domain/src/events.rs`:

```diff
--- a/crates/domain/src/events.rs
+++ b/crates/domain/src/events.rs
@@ -3,8 +3,10 @@
 use chrono::{DateTime, TimeDelta, Utc};
 
 use crate::DomainError;
+use crate::permissions::{Action, Actor, can};
+use crate::records::Event;
 use crate::schedule::DailySchedule;
-use crate::types::EventId;
+use crate::types::{EventId, Role};
 
 /// Reports observed longer ago than this count as backdated.
 pub const BACKDATE_THRESHOLD: TimeDelta = TimeDelta::minutes(10);
@@ -109,6 +111,25 @@ pub fn validate_manual_close(
     }
 }
 
+/// Whether a report observed at `observed_at` (in the past) can belong to `event`: the event had
+/// started by then and was still running on that local day. "Which event was this?" offers
+/// exactly these events, and joining or moving into any other is refused.
+pub fn can_join_backdated(
+    event: &Event,
+    observed_at: DateTime<Utc>,
+    schedule: &DailySchedule,
+) -> bool {
+    let (day_start, _) = schedule.day_span(schedule.local_date(observed_at));
+    event.opened_at <= observed_at && event.closed.is_none_or(|closed| closed.at >= day_start)
+}
+
+/// Who is told that `owner`'s event was auto-closed (spec §7.4): the owner, and residents when
+/// the owner is a guest; only while they can log, since the notice is part of logging.
+pub fn auto_close_notice_is_for(viewer: &Actor, owner: &Actor, now: DateTime<Utc>) -> bool {
+    can(viewer, Action::LogEntry, now)
+        && (viewer.id == owner.id || (viewer.role == Role::Resident && owner.role == Role::Guest))
+}
+
 /// Indices of reports that follow a gap longer than [`LARGE_GAP`]. `observed_times` must be
 /// sorted ascending.
 pub fn large_gap_indices(observed_times: &[DateTime<Utc>]) -> Vec<usize> {
```

In `crates/domain/src/permissions.rs`:

```diff
--- a/crates/domain/src/permissions.rs
+++ b/crates/domain/src/permissions.rs
@@ -74,6 +74,12 @@ pub fn can(actor: &Actor, action: Action, now: DateTime<Utc>) -> bool {
     }
 }
 
+/// Whether `actor` may see an entry logged by (or an event owned by) `owner`: everyone's with
+/// [`Action::ViewAll`], otherwise only their own.
+pub fn can_see(actor: &Actor, owner: UserId, now: DateTime<Utc>) -> bool {
+    can(actor, Action::ViewAll, now) || (owner == actor.id && can(actor, Action::ViewOwn, now))
+}
+
 #[cfg(test)]
 mod tests {
     use chrono::TimeDelta;
```

In `crates/domain/src/schedule.rs`:

```diff
--- a/crates/domain/src/schedule.rs
+++ b/crates/domain/src/schedule.rs
@@ -1,6 +1,6 @@
 //! A time of day that recurs in a local time zone, such as the 04:00 auto-close.
 
-use chrono::{DateTime, LocalResult, NaiveDate, TimeDelta, TimeZone, Utc};
+use chrono::{DateTime, Datelike, LocalResult, Months, NaiveDate, TimeDelta, TimeZone, Utc};
 use chrono_tz::Tz;
 
 use crate::DomainError;
@@ -46,12 +46,35 @@ impl DailySchedule {
         self.last_boundary(now) > since
     }
 
-    /// The boundary instant on a local calendar date. A local time skipped by DST resolves to
-    /// the end of the gap; a repeated one resolves to its first occurrence.
+    /// The local calendar date of an instant.
+    pub fn local_date(&self, at: DateTime<Utc>) -> NaiveDate {
+        at.with_timezone(&self.tz).date_naive()
+    }
+
+    /// `[start, end)` of a local calendar day: 23 or 25 hours long on DST days.
+    pub fn day_span(&self, date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
+        let next = date.succ_opt().expect("date within chrono's range");
+        (self.at_local(date, 0), self.at_local(next, 0))
+    }
+
+    /// `[start, end)` of the local calendar month that contains `date`.
+    pub fn month_span(&self, date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
+        let first = date.with_day(1).expect("every month has a first day");
+        let next = first
+            .checked_add_months(Months::new(1))
+            .expect("date within chrono's range");
+        (self.at_local(first, 0), self.at_local(next, 0))
+    }
+
+    /// The boundary instant on a local calendar date.
     fn boundary_on(&self, date: NaiveDate) -> DateTime<Utc> {
-        let local = date
-            .and_hms_opt(self.hour, 0, 0)
-            .expect("hour validated in new()");
+        self.at_local(date, self.hour)
+    }
+
+    /// `hour`:00 local time on `date`. A local time skipped by DST resolves to the end of the
+    /// gap; a repeated one resolves to its first occurrence.
+    fn at_local(&self, date: NaiveDate, hour: u32) -> DateTime<Utc> {
+        let local = date.and_hms_opt(hour, 0, 0).expect("hour below 24");
         let instant = match self.tz.from_local_datetime(&local) {
             LocalResult::Single(t) => t,
             LocalResult::Ambiguous(earliest, _) => earliest,
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `a_local_day_runs_from_midnight_to_midnight`, `a_backdated_report_joins_events_that_had_started_and_ran_that_day`, `the_notice_goes_to_the_owner_and_to_residents_for_guests` and `guests_see_their_own_entries_and_everyone_else_sees_all`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/events.rs \
  crates/domain/src/permissions.rs \
  crates/domain/src/schedule.rs
git commit -m "feat(domain): local days and months, joinable events and who sees what

DailySchedule gains local_date, day_span and month_span (23- and 25-hour DST
days included), can_join_backdated decides which events a past report may
join, auto_close_notice_is_for encodes spec §7.4 and can_see the 'own data
only' rule for guests.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Domain — logging a smell or a PM2.5 reading

`log_smell` executes the answer to the event question (`EventChoice`) inside the caller's write
transaction: nothing open → a new open event (§7.1); "Yes, update" → join (§7.2); "No, new event"
→ close the open event at its last report, then a new one; a past time → join the chosen event
(moving its end if the report is later, §7.5) or a new event, open only if nothing is open and no
04:00 has passed; "It was still going" → reopen the auto-closed event (§7.4). A reading in the
same form joins the report's event. `log_reading` links a lone reading to the event whose span
contains it and never changes an event (§7.6). The sweep reuses `close_at_last_report`.

**Files:**
- Create: `crates/domain/src/logbook.rs`, `crates/storage-contract/src/logbook.rs`
- Modify: `crates/domain/src/lib.rs`, `crates/domain/src/sweep.rs`, `crates/storage-contract/src/lib.rs`

**Interfaces:**
- Consumes: `classify_observed`, `new_event_state`, `can_join_backdated` (Task 4),
  `EventWrites::event_at` (Task 1), the write traits.
- Produces (in `smoke_domain::logbook`):
  - `LogError::{Forbidden, Stale, OutsideEvent, Invalid(DomainError), Store(StoreError)}`
    (Task 6 adds `NotFound`, `AnotherEventOpen`).
  - `EventChoice::{New, Join(EventId), Continue(EventId)}`;
    `SmellEntry { observed_at, values: ReportValues, pm25: Option<Pm25> }`;
    `Logged { report, reading }`.
  - `log_smell(tx, &Actor, SmellEntry, EventChoice, &DailySchedule, now) -> Result<Logged,
    LogError>`; `log_reading(tx, &Actor, observed_at, Pm25, now) -> Result<Pm25Reading,
    LogError>`.
  - `pub(crate) close_at_last_report`, `pub(crate) extend_to`.
  - Contract helpers `storage-contract::logbook::{People, people, rome, smell, log}`.

- [ ] **Step 1: Write the failing tests**

In `crates/domain/src/lib.rs`:

```diff
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -7,6 +7,7 @@ pub mod audit;
 pub mod clock;
 pub mod error;
 pub mod events;
+pub mod logbook;
 pub mod permissions;
 pub mod records;
 pub mod repos;
```

In `crates/storage-contract/src/lib.rs`:

```diff
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -12,6 +12,7 @@ pub mod audit;
 pub mod events;
 pub mod fixture;
 pub mod lists;
+pub mod logbook;
 pub mod readings;
 pub mod reports;
 pub mod sessions;
@@ -94,6 +95,20 @@ macro_rules! contract_tests {
             sessions::expiring_a_user_ends_their_sessions,
             sessions::a_rolled_back_password_change_keeps_the_sessions,
             sessions::sessions_are_not_audited,
+            logbook::logging_now_with_nothing_open_starts_an_open_event,
+            logbook::same_smell_joins_the_open_event,
+            logbook::a_new_event_closes_the_open_one_at_its_last_report,
+            logbook::a_stale_answer_is_refused_and_changes_nothing,
+            logbook::a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_passed,
+            logbook::a_backdated_report_after_the_end_moves_the_end,
+            logbook::a_backdated_report_cannot_join_an_event_that_had_not_started,
+            logbook::continuing_an_auto_closed_event_reopens_it,
+            logbook::a_guest_cannot_continue_someone_elses_event,
+            logbook::a_reading_with_a_report_joins_the_reports_event,
+            logbook::a_reading_alone_joins_the_event_containing_its_time,
+            logbook::a_failed_report_rolls_back_the_new_event_close,
+            logbook::only_people_who_may_log_can_log,
+            logbook::a_time_in_the_future_is_refused,
             sweep::the_first_sweep_runs_and_is_remembered,
             sweep::a_sweep_at_four_closes_the_event_at_its_last_report,
             sweep::a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four,
```

Create `crates/storage-contract/src/logbook.rs`:

```rust
//! Logging a smell or a PM2.5 reading (spec §7.1–§7.6), on a real store.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::DomainError;
use smoke_domain::clock::Clock;
use smoke_domain::logbook::{EventChoice, LogError, Logged, SmellEntry, log_reading, log_smell};
use smoke_domain::records::{Closure, Event, Pm25Reading, User};
use smoke_domain::repos::{
    AuditRepo, Commit, EventRepo, ListRepo, ListWrites, ReadingRepo, ReportRepo, Transactional,
    UserWrites, WriteTx,
};
use smoke_domain::schedule::DailySchedule;
use smoke_domain::starter::seed_starter_lists;
use smoke_domain::types::{CloseReason, EventId, Pm25, Role, RoomId};

use crate::Fixture;
use crate::fixture::{add_event, add_user, report_values, start};

/// Everyone who might log, and the seeded lists.
pub(crate) struct People {
    pub resident: User,
    pub partner: User,
    pub guest: User,
    pub viewer: User,
}

pub(crate) async fn people<S>(fx: &Fixture<S>) -> People
where
    S: Transactional<Tx: UserWrites + ListWrites> + ListRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    People {
        resident: add_user(fx, "daniele", Role::Resident).await,
        partner: add_user(fx, "partner", Role::Resident).await,
        guest: add_user(fx, "guest", Role::Guest).await,
        viewer: add_user(fx, "lawyer", Role::Viewer).await,
    }
}

pub(crate) fn rome() -> DailySchedule {
    DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
}

pub(crate) fn smell(at: DateTime<Utc>, intensity: u8) -> SmellEntry {
    SmellEntry {
        observed_at: at,
        values: report_values(intensity, &[], &[]),
        pm25: None,
    }
}

/// Runs [`log_smell`] in its own transaction, committed only on success.
pub(crate) async fn log<S>(
    fx: &Fixture<S>,
    who: &User,
    entry: SmellEntry,
    choice: EventChoice,
) -> Result<Logged, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let logged = log_smell(
        &mut tx,
        &who.actor(),
        entry,
        choice,
        &rome(),
        fx.clock.now(),
    )
    .await?;
    tx.commit().await.unwrap();
    Ok(logged)
}

async fn reading<S>(fx: &Fixture<S>, who: &User, at: DateTime<Utc>) -> Pm25Reading
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let value = Pm25::new(42.0).unwrap();
    let reading = log_reading(&mut tx, &who.actor(), at, value, fx.clock.now())
        .await
        .unwrap();
    tx.commit().await.unwrap();
    reading
}

async fn event<S: EventRepo>(fx: &Fixture<S>, id: EventId) -> Event {
    fx.store.event(id).await.unwrap().unwrap()
}

pub async fn logging_now_with_nothing_open_starts_an_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(fx, &p.guest, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    let open = fx.store.open_event().await.unwrap().unwrap();
    assert_eq!(logged.report.event_id, open.id);
    assert_eq!((open.owner_id, open.opened_at), (p.guest.id, start()));
    assert_eq!(logged.reading, None);
}

pub async fn same_smell_joins_the_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + ReportRepo,
{
    let p = people(fx).await;
    let first = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    fx.clock.advance(TimeDelta::minutes(20));
    let event_id = first.report.event_id;
    let second = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(event_id),
    )
    .await
    .unwrap();
    assert_eq!(second.report.event_id, event_id);
    assert_eq!(event(fx, event_id).await.owner_id, p.resident.id);
    assert_eq!(fx.store.reports_for_event(event_id).await.unwrap().len(), 2);
}

pub async fn a_new_event_closes_the_open_one_at_its_last_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let first = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    fx.clock.advance(TimeDelta::minutes(30));
    let last = log(
        fx,
        &p.resident,
        smell(fx.clock.now(), 2),
        EventChoice::Join(first.report.event_id),
    )
    .await
    .unwrap();
    fx.clock.advance(TimeDelta::hours(2));
    let next = log(fx, &p.guest, smell(fx.clock.now(), 5), EventChoice::New)
        .await
        .unwrap();

    let old = event(fx, first.report.event_id).await;
    assert_eq!(
        old.closed,
        Some(Closure {
            at: last.report.observed_at,
            reason: CloseReason::NewEvent
        })
    );
    let open = fx.store.open_event().await.unwrap().unwrap();
    assert_eq!(open.id, next.report.event_id);
    assert_eq!(open.owner_id, p.guest.id);
}

pub async fn a_stale_answer_is_refused_and_changes_nothing<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let first = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    fx.clock.advance(TimeDelta::minutes(5));
    // The partner's phone answered "No, it had stopped" first.
    log(fx, &p.partner, smell(fx.clock.now(), 5), EventChoice::New)
        .await
        .unwrap();
    let entries = fx.store.audit_log().await.unwrap().len();

    let late_yes = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(first.report.event_id),
    )
    .await;
    assert!(matches!(late_yes, Err(LogError::Stale)), "{late_yes:?}");
    let late_continue = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Continue(first.report.event_id),
    )
    .await;
    assert!(
        matches!(late_continue, Err(LogError::Stale)),
        "{late_continue:?}"
    );
    let unknown = log(
        fx,
        &p.guest,
        smell(fx.clock.now(), 6),
        EventChoice::Join(EventId(999)),
    )
    .await;
    assert!(matches!(unknown, Err(LogError::Stale)), "{unknown:?}");
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
}

pub async fn a_backdated_new_event_is_open_only_if_nothing_is_and_no_four_am_passed<S>(
    fx: &Fixture<S>,
) where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    // 21:00 in Rome: an hour ago is the same evening.
    let earlier_tonight = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(1), 3),
        EventChoice::New,
    )
    .await
    .unwrap();
    assert!(event(fx, earlier_tonight.report.event_id).await.is_open());

    let earlier_still = start() - TimeDelta::hours(2);
    let while_open = log(fx, &p.resident, smell(earlier_still, 3), EventChoice::New)
        .await
        .unwrap();
    assert_eq!(
        event(fx, while_open.report.event_id).await.closed,
        Some(Closure {
            at: earlier_still,
            reason: CloseReason::Backdated
        }),
        "another event is open"
    );

    let yesterday = start() - TimeDelta::days(1);
    let past_four = log(fx, &p.resident, smell(yesterday, 3), EventChoice::New)
        .await
        .unwrap();
    assert_eq!(
        event(fx, past_four.report.event_id)
            .await
            .closed
            .map(|c| c.reason),
        Some(CloseReason::Backdated)
    );
    assert!(
        event(fx, earlier_tonight.report.event_id).await.is_open(),
        "a past report never closes the open event"
    );
}

pub async fn a_backdated_report_after_the_end_moves_the_end<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(5);
    let closed = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Manual,
    };
    let past = add_event(fx, p.resident.id, opened, Some(closed)).await;

    let inside = log(
        fx,
        &p.guest,
        smell(opened + TimeDelta::minutes(30), 3),
        EventChoice::Join(past.id),
    )
    .await
    .unwrap();
    assert_eq!(inside.report.event_id, past.id);
    assert_eq!(event(fx, past.id).await.closed, Some(closed));

    let after = opened + TimeDelta::hours(2);
    log(fx, &p.guest, smell(after, 3), EventChoice::Join(past.id))
        .await
        .unwrap();
    assert_eq!(
        event(fx, past.id).await.closed,
        Some(Closure {
            at: after,
            reason: CloseReason::Manual
        })
    );
}

pub async fn a_backdated_report_cannot_join_an_event_that_had_not_started<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let open = add_event(fx, p.resident.id, start() - TimeDelta::hours(1), None).await;
    let before = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(2), 3),
        EventChoice::Join(open.id),
    )
    .await;
    assert!(matches!(before, Err(LogError::OutsideEvent)), "{before:?}");

    let yesterday = start() - TimeDelta::days(1);
    let old = add_event(
        fx,
        p.resident.id,
        yesterday,
        Some(Closure {
            at: yesterday,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let next_day = log(
        fx,
        &p.resident,
        smell(start() - TimeDelta::hours(2), 3),
        EventChoice::Join(old.id),
    )
    .await;
    assert!(
        matches!(next_day, Err(LogError::OutsideEvent)),
        "{next_day:?}"
    );
}

pub async fn continuing_an_auto_closed_event_reopens_it<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let auto = Closure {
        at: opened + TimeDelta::hours(1),
        reason: CloseReason::Auto0400,
    };
    let guests = add_event(fx, p.guest.id, opened, Some(auto)).await;

    let logged = log(
        fx,
        &p.partner,
        smell(start(), 5),
        EventChoice::Continue(guests.id),
    )
    .await
    .unwrap();
    assert_eq!(logged.report.event_id, guests.id);
    assert!(event(fx, guests.id).await.is_open());

    let manual = add_event(
        fx,
        p.resident.id,
        opened - TimeDelta::hours(1),
        Some(Closure {
            at: opened,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let not_auto = log(
        fx,
        &p.resident,
        smell(start(), 5),
        EventChoice::Continue(manual.id),
    )
    .await;
    assert!(matches!(not_auto, Err(LogError::Stale)), "{not_auto:?}");
}

pub async fn a_guest_cannot_continue_someone_elses_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(20);
    let residents = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened,
            reason: CloseReason::Auto0400,
        }),
    )
    .await;
    let refused = log(
        fx,
        &p.guest,
        smell(start(), 5),
        EventChoice::Continue(residents.id),
    )
    .await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    assert!(!event(fx, residents.id).await.is_open());
}

pub async fn a_reading_with_a_report_joins_the_reports_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + ReadingRepo,
{
    let p = people(fx).await;
    let mut entry = smell(start(), 4);
    entry.pm25 = Some(Pm25::new(35.5).unwrap());
    let logged = log(fx, &p.resident, entry, EventChoice::New).await.unwrap();
    let reading = logged.reading.unwrap();
    assert_eq!(reading.event_id, Some(logged.report.event_id));
    assert_eq!(
        (reading.observed_at, reading.user_id),
        (start(), p.resident.id)
    );
    assert_eq!(
        fx.store
            .readings_for_event(logged.report.event_id)
            .await
            .unwrap(),
        [reading]
    );
}

pub async fn a_reading_alone_joins_the_event_containing_its_time<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(5);
    let past = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened + TimeDelta::hours(1),
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let open = add_event(fx, p.resident.id, start() - TimeDelta::minutes(30), None).await;

    let inside = reading(fx, &p.guest, opened + TimeDelta::minutes(10)).await;
    assert_eq!(inside.event_id, Some(past.id));
    let between = reading(fx, &p.guest, opened + TimeDelta::hours(2)).await;
    assert_eq!(between.event_id, None);
    let now = reading(fx, &p.guest, start()).await;
    assert_eq!(now.event_id, Some(open.id));

    assert_eq!(
        event(fx, past.id).await.closed.map(|c| c.at),
        Some(opened + TimeDelta::hours(1)),
        "a reading never moves an end"
    );
    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        Some(open.id)
    );
}

pub async fn a_failed_report_rolls_back_the_new_event_close<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let first = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    let entries = fx.store.audit_log().await.unwrap().len();
    fx.clock.advance(TimeDelta::minutes(30));

    let mut tx = fx.store.begin(Some(p.guest.id)).await.unwrap();
    let mut entry = smell(fx.clock.now(), 5);
    entry.values = report_values(5, &[RoomId(999)], &[]);
    let failed = log_smell(
        &mut tx,
        &p.guest.actor(),
        entry,
        EventChoice::New,
        &rome(),
        fx.clock.now(),
    )
    .await;
    assert!(matches!(failed, Err(LogError::Store(_))), "{failed:?}");
    drop(tx);

    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        Some(first.report.event_id),
        "the close and the new event were rolled back with the report"
    );
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
}

pub async fn only_people_who_may_log_can_log<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx + UserWrites> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let viewer = log(fx, &p.viewer, smell(start(), 4), EventChoice::New).await;
    assert!(matches!(viewer, Err(LogError::Forbidden)), "{viewer:?}");

    let mut tx = fx.store.begin(Some(p.resident.id)).await.unwrap();
    let expired = tx.expire_user(p.guest.id).await.unwrap();
    tx.commit().await.unwrap();
    let guest = log(fx, &expired, smell(start(), 4), EventChoice::New).await;
    assert!(matches!(guest, Err(LogError::Forbidden)), "{guest:?}");

    let mut tx = fx.store.begin(Some(p.viewer.id)).await.unwrap();
    let alone = log_reading(
        &mut tx,
        &p.viewer.actor(),
        start(),
        Pm25::new(1.0).unwrap(),
        start(),
    )
    .await;
    assert!(matches!(alone, Err(LogError::Forbidden)), "{alone:?}");
    drop(tx);
    assert_eq!(fx.store.open_event().await.unwrap(), None);
}

pub async fn a_time_in_the_future_is_refused<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo,
{
    let p = people(fx).await;
    let future = log(
        fx,
        &p.resident,
        smell(start() + TimeDelta::minutes(5), 4),
        EventChoice::New,
    )
    .await;
    assert!(
        matches!(
            future,
            Err(LogError::Invalid(DomainError::ObservedInFuture))
        ),
        "{future:?}"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres`
Expected: compile error — `smoke_domain::logbook` does not exist.

- [ ] **Step 3: Implement**

Create `crates/domain/src/logbook.rs`:

```rust
//! The logging use cases (spec §7). Each one runs inside a write transaction the caller begins
//! and commits, so every write it makes — closing the open event, opening a new one, the report
//! and its PM2.5 reading — is audited and applied together or not at all. Permissions are
//! checked here, against the data read inside the transaction.

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::events::{
    NewEventState, ObservedTime, can_join_backdated, classify_observed, new_event_state,
};
use crate::permissions::{Action, Actor, can};
use crate::records::{
    Closure, Event, NewEvent, NewReading, NewReport, Pm25Reading, ReportValues, SmellReport,
};
use crate::repos::{EventWrites, ReadingWrites, ReportWrites, StoreError, StoreResult};
use crate::schedule::DailySchedule;
use crate::types::{CloseReason, EventId, Pm25};

#[derive(Debug, Error)]
pub enum LogError {
    #[error("not allowed")]
    Forbidden,
    /// The answer to the event question no longer fits the data (another phone closed or
    /// opened an event meanwhile): ask again rather than guess.
    #[error("the events changed since the question was asked")]
    Stale,
    /// The time lies outside the event: before it started, or on another day.
    #[error("the time is outside the event")]
    OutsideEvent,
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// The answer to "Same smell still going?", "Which event was this?" or the auto-close notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventChoice {
    /// Start a new event: nothing was asked, "No, it had stopped", or "New event".
    New,
    /// Join this event: "Yes, update", or the event picked for a past report.
    Join(EventId),
    /// "It was still going": reopen this auto-closed event and join it.
    Continue(EventId),
}

/// What the Log form says about a smell.
#[derive(Debug, Clone, PartialEq)]
pub struct SmellEntry {
    pub observed_at: DateTime<Utc>,
    pub values: ReportValues,
    /// Logged together with the report, in the same event.
    pub pm25: Option<Pm25>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Logged {
    pub report: SmellReport,
    pub reading: Option<Pm25Reading>,
}

/// Closes `event` at its last report (or where it opened, if it has none): the end used by
/// both the "new event" answer and the 04:00 sweep.
pub(crate) async fn close_at_last_report<T: EventWrites>(
    tx: &mut T,
    event: &Event,
    reason: CloseReason,
) -> StoreResult<Event> {
    let last_report = tx.latest_report_at(event.id).await?;
    let at = last_report.map_or(event.opened_at, |at| at.max(event.opened_at));
    tx.close_event(event.id, Closure { at, reason }).await
}

/// Logs a smell report, and its PM2.5 reading if there is one, into the event `choice` names
/// (spec §7.1, §7.2, §7.4, §7.5).
pub async fn log_smell<T>(
    tx: &mut T,
    actor: &Actor,
    entry: SmellEntry,
    choice: EventChoice,
    schedule: &DailySchedule,
    now: DateTime<Utc>,
) -> Result<Logged, LogError>
where
    T: EventWrites + ReportWrites + ReadingWrites,
{
    if !can(actor, Action::LogEntry, now) {
        return Err(LogError::Forbidden);
    }
    let observed_at = entry.observed_at;
    let observed = classify_observed(observed_at, now)?;
    let open = tx.open_event().await?;
    let event_id = match choice {
        EventChoice::New => {
            let another_open = match (observed, &open) {
                (ObservedTime::Now, Some(open)) => {
                    close_at_last_report(tx, open, CloseReason::NewEvent).await?;
                    false
                }
                (_, open) => open.is_some(),
            };
            let closed = match new_event_state(observed_at, now, schedule, another_open) {
                NewEventState::Open => None,
                NewEventState::ClosedBackdated => Some(Closure {
                    at: observed_at,
                    reason: CloseReason::Backdated,
                }),
            };
            let new = NewEvent {
                owner_id: actor.id,
                opened_at: observed_at,
                closed,
            };
            tx.create_event(new).await?.id
        }
        EventChoice::Join(id) => {
            let event = tx.event(id).await?.ok_or(LogError::Stale)?;
            match observed {
                ObservedTime::Now if !event.is_open() => return Err(LogError::Stale),
                ObservedTime::Now => {}
                ObservedTime::Backdated => {
                    if !can_join_backdated(&event, observed_at, schedule) {
                        return Err(LogError::OutsideEvent);
                    }
                    extend_to(tx, &event, observed_at).await?;
                }
            }
            id
        }
        EventChoice::Continue(id) => {
            let event = tx.event(id).await?.ok_or(LogError::Stale)?;
            let auto_closed = event.closed.map(|c| c.reason) == Some(CloseReason::Auto0400);
            if observed != ObservedTime::Now || open.is_some() || !auto_closed {
                return Err(LogError::Stale);
            }
            if !can(
                actor,
                Action::ManageEvent {
                    owner: event.owner_id,
                },
                now,
            ) {
                return Err(LogError::Forbidden);
            }
            tx.reopen_event(id).await?;
            id
        }
    };
    let report = tx
        .create_report(NewReport {
            event_id,
            user_id: actor.id,
            observed_at,
            values: entry.values,
        })
        .await?;
    let reading = match entry.pm25 {
        Some(value) => Some(
            tx.create_reading(NewReading {
                user_id: actor.id,
                event_id: Some(event_id),
                observed_at,
                value,
            })
            .await?,
        ),
        None => None,
    };
    Ok(Logged { report, reading })
}

/// Logs a PM2.5 reading on its own ("Log PM2.5 only", spec §7.6). It joins the event whose span
/// contains its time, if any, and never opens, extends or closes an event.
pub async fn log_reading<T>(
    tx: &mut T,
    actor: &Actor,
    observed_at: DateTime<Utc>,
    value: Pm25,
    now: DateTime<Utc>,
) -> Result<Pm25Reading, LogError>
where
    T: EventWrites + ReadingWrites,
{
    if !can(actor, Action::LogEntry, now) {
        return Err(LogError::Forbidden);
    }
    classify_observed(observed_at, now)?;
    let event_id = tx.event_at(observed_at).await?.map(|event| event.id);
    let reading = tx
        .create_reading(NewReading {
            user_id: actor.id,
            event_id,
            observed_at,
            value,
        })
        .await?;
    Ok(reading)
}

/// A report after the end of a closed event moves the end to the report (spec §7.5).
pub(crate) async fn extend_to<T: EventWrites>(
    tx: &mut T,
    event: &Event,
    at: DateTime<Utc>,
) -> StoreResult<()> {
    if let Some(closed) = event.closed
        && at > closed.at
    {
        tx.move_event_end(event.id, at).await?;
    }
    Ok(())
}
```

In `crates/domain/src/sweep.rs`:

```diff
--- a/crates/domain/src/sweep.rs
+++ b/crates/domain/src/sweep.rs
@@ -3,7 +3,7 @@
 
 use chrono::{DateTime, Utc};
 
-use crate::records::Closure;
+use crate::logbook::close_at_last_report;
 use crate::repos::{Commit, EventWrites, JobRepo, JobWrites, StoreResult, Transactional};
 use crate::schedule::DailySchedule;
 use crate::types::{CloseReason, EventId};
@@ -40,10 +40,7 @@ where
     if let Some(event) = tx.open_event().await?
         && schedule.boundary_passed_since(event.opened_at, now)
     {
-        let last_report = tx.latest_report_at(event.id).await?;
-        let at = last_report.map_or(event.opened_at, |at| at.max(event.opened_at));
-        let reason = CloseReason::Auto0400;
-        tx.close_event(event.id, Closure { at, reason }).await?;
+        close_at_last_report(&mut tx, &event, CloseReason::Auto0400).await?;
         closed = Some(event.id);
     }
     tx.record_job_run(AUTO_CLOSE_JOB, now).await?;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass on both backends, including `a_stale_answer_is_refused_and_changes_nothing`, `a_backdated_report_after_the_end_moves_the_end` and `a_failed_report_rolls_back_the_new_event_close`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/lib.rs \
  crates/domain/src/logbook.rs \
  crates/domain/src/sweep.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/logbook.rs
git commit -m "feat(domain): log a smell or a PM2.5 reading in one audited transaction

log_smell executes the answer to the event question (EventChoice) against
data read inside the write transaction: a stale answer is refused instead of
guessed, and closing the open event, opening a new one, the report and its
reading commit together or not at all. log_reading links a reading to the
event whose span contains it and never changes an event.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Domain — close, reopen, correct and move

The remaining use cases, each checking the §4 permission against the record read inside the
transaction: manual close between the last report and now (§7.3), reopen when no other event is
open, "Correct a mistake" (values and observed time, inside the event; a later time moves a closed
event's end), and "Move to another event" into a joinable event or a new one owned by the report's
author (§7.7).

**Files:**
- Create: `crates/storage-contract/src/corrections.rs`
- Modify: `crates/domain/src/logbook.rs`, `crates/storage-contract/src/lib.rs`

**Interfaces:**
- Consumes: Task 5's `LogError`, `extend_to`; `validate_manual_close`, `can_join_backdated`.
- Produces (in `smoke_domain::logbook`): `LogError::{NotFound, AnotherEventOpen}`;
  `MoveTarget::{Event(EventId), New}`; `close_event(tx, &Actor, EventId, closed_at, now)`,
  `reopen_event(tx, &Actor, EventId, now)` → `Result<Event, LogError>`;
  `correct_report(tx, &Actor, ReportId, observed_at, ReportValues, &DailySchedule, now)`,
  `move_report(tx, &Actor, ReportId, MoveTarget, &DailySchedule, now)` →
  `Result<SmellReport, LogError>`.

- [ ] **Step 1: Write the failing tests**

Create `crates/storage-contract/src/corrections.rs`:

```rust
//! Closing, reopening, correcting and moving (spec §7.3, §7.7), on a real store.

use chrono::TimeDelta;
use smoke_domain::DomainError;
use smoke_domain::clock::Clock;
use smoke_domain::logbook::{
    EventChoice, LogError, MoveTarget, close_event, correct_report, move_report, reopen_event,
};
use smoke_domain::records::{Closure, Event, SmellReport, User};
use smoke_domain::repos::{
    AuditRepo, Commit, EventRepo, ListRepo, ReportWrites, Transactional, WriteTx,
};
use smoke_domain::types::{CloseReason, EventId, ReportId};

use crate::Fixture;
use crate::fixture::{add_event, report_values, start};
use crate::logbook::{log, people, rome, smell};

async fn close<S>(
    fx: &Fixture<S>,
    who: &User,
    id: EventId,
    at: chrono::DateTime<chrono::Utc>,
) -> Result<Event, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let event = close_event(&mut tx, &who.actor(), id, at, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(event)
}

async fn reopen<S>(fx: &Fixture<S>, who: &User, id: EventId) -> Result<Event, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let event = reopen_event(&mut tx, &who.actor(), id, fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(event)
}

async fn correct<S>(
    fx: &Fixture<S>,
    who: &User,
    id: ReportId,
    at: chrono::DateTime<chrono::Utc>,
    intensity: u8,
) -> Result<SmellReport, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let values = report_values(intensity, &[], &[]);
    let now = fx.clock.now();
    let report = correct_report(&mut tx, &who.actor(), id, at, values, &rome(), now).await?;
    tx.commit().await.unwrap();
    Ok(report)
}

async fn relocate<S>(
    fx: &Fixture<S>,
    who: &User,
    id: ReportId,
    to: MoveTarget,
) -> Result<SmellReport, LogError>
where
    S: Transactional<Tx: WriteTx>,
{
    let mut tx = fx.store.begin(Some(who.id)).await.unwrap();
    let report = move_report(&mut tx, &who.actor(), id, to, &rome(), fx.clock.now()).await?;
    tx.commit().await.unwrap();
    Ok(report)
}

async fn event<S: EventRepo>(fx: &Fixture<S>, id: EventId) -> Event {
    fx.store.event(id).await.unwrap().unwrap()
}

pub async fn a_manual_close_needs_a_time_between_the_last_report_and_now<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let logged = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    let id = logged.report.event_id;
    fx.clock.advance(TimeDelta::hours(1));

    let early = close(fx, &p.resident, id, start() - TimeDelta::minutes(1)).await;
    assert!(
        matches!(
            early,
            Err(LogError::Invalid(DomainError::CloseBeforeLastReport))
        ),
        "{early:?}"
    );
    let future = close(fx, &p.resident, id, fx.clock.now() + TimeDelta::hours(1)).await;
    assert!(
        matches!(future, Err(LogError::Invalid(DomainError::CloseInFuture))),
        "{future:?}"
    );
    let at = start() + TimeDelta::minutes(40);
    let closed = close(fx, &p.resident, id, at).await.unwrap();
    assert_eq!(
        closed.closed,
        Some(Closure {
            at,
            reason: CloseReason::Manual
        })
    );
    let again = close(fx, &p.resident, id, at).await;
    assert!(matches!(again, Err(LogError::Stale)), "{again:?}");
}

pub async fn only_residents_and_owning_guests_can_close_or_reopen<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let residents = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap()
        .report
        .event_id;
    for who in [&p.guest, &p.viewer] {
        let refused = close(fx, who, residents, start()).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    }
    close(fx, &p.partner, residents, start()).await.unwrap();
    let refused = reopen(fx, &p.guest, residents).await;
    assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");

    fx.clock.advance(TimeDelta::minutes(5));
    let guests = log(fx, &p.guest, smell(fx.clock.now(), 4), EventChoice::New)
        .await
        .unwrap()
        .report
        .event_id;
    close(fx, &p.guest, guests, fx.clock.now()).await.unwrap();
    reopen(fx, &p.guest, guests).await.unwrap();
    assert!(event(fx, guests).await.is_open());

    let missing = close(fx, &p.resident, EventId(999), start()).await;
    assert!(matches!(missing, Err(LogError::NotFound)), "{missing:?}");
}

pub async fn reopening_waits_until_no_other_event_is_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(3);
    let past = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened,
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let open = add_event(fx, p.resident.id, start(), None).await;

    let blocked = reopen(fx, &p.resident, past.id).await;
    assert!(
        matches!(blocked, Err(LogError::AnotherEventOpen)),
        "{blocked:?}"
    );
    let already = reopen(fx, &p.resident, open.id).await;
    assert!(matches!(already, Err(LogError::Stale)), "{already:?}");
    close(fx, &p.resident, open.id, start()).await.unwrap();
    reopen(fx, &p.resident, past.id).await.unwrap();
    assert_eq!(
        fx.store.open_event().await.unwrap().map(|e| e.id),
        Some(past.id)
    );
}

pub async fn correcting_after_the_end_moves_the_end_and_before_the_start_is_refused<S>(
    fx: &Fixture<S>,
) where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo + AuditRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(4);
    let logged = log(fx, &p.resident, smell(opened, 4), EventChoice::New)
        .await
        .unwrap();
    let id = logged.report.event_id;
    close(fx, &p.resident, id, opened + TimeDelta::minutes(30))
        .await
        .unwrap();

    let corrected = correct(fx, &p.resident, logged.report.id, opened, 7)
        .await
        .unwrap();
    assert_eq!(corrected.values.intensity().get(), 7);

    let later = opened + TimeDelta::hours(1);
    let moved = correct(fx, &p.resident, logged.report.id, later, 7)
        .await
        .unwrap();
    assert_eq!(moved.observed_at, later);
    assert_eq!(event(fx, id).await.closed.map(|c| c.at), Some(later));

    let before = correct(
        fx,
        &p.resident,
        logged.report.id,
        opened - TimeDelta::minutes(1),
        7,
    )
    .await;
    assert!(matches!(before, Err(LogError::OutsideEvent)), "{before:?}");
}

pub async fn guests_can_correct_and_move_only_their_own_reports<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let residents = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    let event_id = residents.report.event_id;
    let guests = log(fx, &p.guest, smell(start(), 5), EventChoice::Join(event_id))
        .await
        .unwrap();

    for who in [&p.guest, &p.viewer] {
        let refused = correct(fx, who, residents.report.id, start(), 9).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
        let refused = relocate(fx, who, residents.report.id, MoveTarget::New).await;
        assert!(matches!(refused, Err(LogError::Forbidden)), "{refused:?}");
    }
    correct(fx, &p.guest, guests.report.id, start(), 6)
        .await
        .unwrap();
    correct(fx, &p.partner, residents.report.id, start(), 3)
        .await
        .unwrap();

    let mut tx = fx.store.begin(Some(p.guest.id)).await.unwrap();
    tx.delete_report(guests.report.id).await.unwrap();
    tx.commit().await.unwrap();
    let deleted = correct(fx, &p.guest, guests.report.id, start(), 6).await;
    assert!(matches!(deleted, Err(LogError::NotFound)), "{deleted:?}");
}

pub async fn moving_to_a_new_event_creates_it_for_the_reports_author<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let first = log(fx, &p.resident, smell(start(), 4), EventChoice::New)
        .await
        .unwrap();
    let joined = log(
        fx,
        &p.guest,
        smell(start(), 5),
        EventChoice::Join(first.report.event_id),
    )
    .await
    .unwrap();

    let moved = relocate(fx, &p.partner, joined.report.id, MoveTarget::New)
        .await
        .unwrap();
    assert_ne!(moved.event_id, first.report.event_id);
    let new = event(fx, moved.event_id).await;
    assert_eq!((new.owner_id, new.opened_at), (p.guest.id, start()));
    assert_eq!(
        new.closed,
        Some(Closure {
            at: start(),
            reason: CloseReason::Backdated
        }),
        "the first event is still open"
    );
    let same = relocate(
        fx,
        &p.partner,
        joined.report.id,
        MoveTarget::Event(moved.event_id),
    )
    .await
    .unwrap();
    assert_eq!(same, moved);
}

pub async fn moving_after_the_end_of_the_target_moves_its_end<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: WriteTx> + ListRepo + EventRepo,
{
    let p = people(fx).await;
    let opened = start() - TimeDelta::hours(4);
    let earlier = add_event(
        fx,
        p.resident.id,
        opened,
        Some(Closure {
            at: opened + TimeDelta::minutes(10),
            reason: CloseReason::Manual,
        }),
    )
    .await;
    let later_at = opened + TimeDelta::hours(1);
    let later = log(fx, &p.resident, smell(later_at, 4), EventChoice::New)
        .await
        .unwrap();

    let moved = relocate(
        fx,
        &p.resident,
        later.report.id,
        MoveTarget::Event(earlier.id),
    )
    .await
    .unwrap();
    assert_eq!(moved.event_id, earlier.id);
    assert_eq!(
        event(fx, earlier.id).await.closed.map(|c| c.at),
        Some(later_at)
    );

    let first = log(
        fx,
        &p.resident,
        smell(opened - TimeDelta::hours(1), 4),
        EventChoice::New,
    )
    .await
    .unwrap();
    let not_started = relocate(
        fx,
        &p.resident,
        first.report.id,
        MoveTarget::Event(earlier.id),
    )
    .await;
    assert!(
        matches!(not_started, Err(LogError::OutsideEvent)),
        "{not_started:?}"
    );
}
```

In `crates/storage-contract/src/lib.rs`:

```diff
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -9,6 +9,7 @@
 //! migrated database, or `None` to skip.
 
 pub mod audit;
+pub mod corrections;
 pub mod events;
 pub mod fixture;
 pub mod lists;
@@ -109,6 +110,13 @@ macro_rules! contract_tests {
             logbook::a_failed_report_rolls_back_the_new_event_close,
             logbook::only_people_who_may_log_can_log,
             logbook::a_time_in_the_future_is_refused,
+            corrections::a_manual_close_needs_a_time_between_the_last_report_and_now,
+            corrections::only_residents_and_owning_guests_can_close_or_reopen,
+            corrections::reopening_waits_until_no_other_event_is_open,
+            corrections::correcting_after_the_end_moves_the_end_and_before_the_start_is_refused,
+            corrections::guests_can_correct_and_move_only_their_own_reports,
+            corrections::moving_to_a_new_event_creates_it_for_the_reports_author,
+            corrections::moving_after_the_end_of_the_target_moves_its_end,
             sweep::the_first_sweep_runs_and_is_remembered,
             sweep::a_sweep_at_four_closes_the_event_at_its_last_report,
             sweep::a_catch_up_at_eight_closes_exactly_like_a_sweep_at_four,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres`
Expected: compile error — `MoveTarget`, `close_event`, `reopen_event`, `correct_report` and `move_report` are not defined in `smoke_domain::logbook`.

- [ ] **Step 3: Implement**

In `crates/domain/src/logbook.rs`:

```diff
--- a/crates/domain/src/logbook.rs
+++ b/crates/domain/src/logbook.rs
@@ -9,6 +9,7 @@ use thiserror::Error;
 use crate::DomainError;
 use crate::events::{
     NewEventState, ObservedTime, can_join_backdated, classify_observed, new_event_state,
+    validate_manual_close,
 };
 use crate::permissions::{Action, Actor, can};
 use crate::records::{
@@ -16,12 +17,18 @@ use crate::records::{
 };
 use crate::repos::{EventWrites, ReadingWrites, ReportWrites, StoreError, StoreResult};
 use crate::schedule::DailySchedule;
-use crate::types::{CloseReason, EventId, Pm25};
+use crate::types::{CloseReason, EventId, Pm25, ReportId};
 
 #[derive(Debug, Error)]
 pub enum LogError {
     #[error("not allowed")]
     Forbidden,
+    /// No such event or report, or the report was deleted.
+    #[error("not found")]
+    NotFound,
+    /// Reopening while another event is open would break the one-open-event rule.
+    #[error("another event is open")]
+    AnotherEventOpen,
     /// The answer to the event question no longer fits the data (another phone closed or
     /// opened an event meanwhile): ask again rather than guess.
     #[error("the events changed since the question was asked")]
@@ -212,3 +219,157 @@ pub(crate) async fn extend_to<T: EventWrites>(
     }
     Ok(())
 }
+
+/// Where "Move to another event" puts a report.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum MoveTarget {
+    Event(EventId),
+    New,
+}
+
+/// Closes an open event by hand at `closed_at`, which must lie between its last report and now
+/// (spec §7.3).
+pub async fn close_event<T: EventWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: EventId,
+    closed_at: DateTime<Utc>,
+    now: DateTime<Utc>,
+) -> Result<Event, LogError> {
+    let event = manageable_event(tx, actor, id, now).await?;
+    if !event.is_open() {
+        return Err(LogError::Stale);
+    }
+    let last_report = tx.latest_report_at(id).await?.unwrap_or(event.opened_at);
+    validate_manual_close(closed_at, last_report.max(event.opened_at), now)?;
+    let closure = Closure {
+        at: closed_at,
+        reason: CloseReason::Manual,
+    };
+    Ok(tx.close_event(id, closure).await?)
+}
+
+/// Reopens a closed event, if no other event is open.
+pub async fn reopen_event<T: EventWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: EventId,
+    now: DateTime<Utc>,
+) -> Result<Event, LogError> {
+    let event = manageable_event(tx, actor, id, now).await?;
+    if event.is_open() {
+        return Err(LogError::Stale);
+    }
+    if tx.open_event().await?.is_some() {
+        return Err(LogError::AnotherEventOpen);
+    }
+    Ok(tx.reopen_event(id).await?)
+}
+
+/// "Correct a mistake" (spec §7.7): new values and observed time for a report. The time must
+/// still belong to the report's event; after a closed event's end, the end moves with it.
+pub async fn correct_report<T: EventWrites + ReportWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: ReportId,
+    observed_at: DateTime<Utc>,
+    values: ReportValues,
+    schedule: &DailySchedule,
+    now: DateTime<Utc>,
+) -> Result<SmellReport, LogError> {
+    let report = editable_report(tx, actor, id, now).await?;
+    classify_observed(observed_at, now)?;
+    let event = tx.event(report.event_id).await?.ok_or(LogError::NotFound)?;
+    if !can_join_backdated(&event, observed_at, schedule) {
+        return Err(LogError::OutsideEvent);
+    }
+    extend_to(tx, &event, observed_at).await?;
+    Ok(tx.correct_report(id, observed_at, values).await?)
+}
+
+/// "Move to another event" (spec §7.7): into an event that was running when the report was
+/// observed (its end moves if needed), or into a new event owned by the report's author.
+pub async fn move_report<T: EventWrites + ReportWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: ReportId,
+    to: MoveTarget,
+    schedule: &DailySchedule,
+    now: DateTime<Utc>,
+) -> Result<SmellReport, LogError> {
+    let report = editable_report(tx, actor, id, now).await?;
+    let target = match to {
+        MoveTarget::Event(target) if target == report.event_id => return Ok(report),
+        MoveTarget::Event(target) => {
+            let event = tx.event(target).await?.ok_or(LogError::NotFound)?;
+            if !can_join_backdated(&event, report.observed_at, schedule) {
+                return Err(LogError::OutsideEvent);
+            }
+            extend_to(tx, &event, report.observed_at).await?;
+            target
+        }
+        MoveTarget::New => {
+            let another_open = tx.open_event().await?.is_some();
+            let closed = match new_event_state(report.observed_at, now, schedule, another_open) {
+                NewEventState::Open => None,
+                NewEventState::ClosedBackdated => Some(Closure {
+                    at: report.observed_at,
+                    reason: CloseReason::Backdated,
+                }),
+            };
+            let new = NewEvent {
+                owner_id: report.user_id,
+                opened_at: report.observed_at,
+                closed,
+            };
+            tx.create_event(new).await?.id
+        }
+    };
+    Ok(tx.move_report(id, target).await?)
+}
+
+/// The event, if it exists and `actor` may close or reopen it.
+async fn manageable_event<T: EventWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: EventId,
+    now: DateTime<Utc>,
+) -> Result<Event, LogError> {
+    let event = tx.event(id).await?.ok_or(LogError::NotFound)?;
+    if can(
+        actor,
+        Action::ManageEvent {
+            owner: event.owner_id,
+        },
+        now,
+    ) {
+        Ok(event)
+    } else {
+        Err(LogError::Forbidden)
+    }
+}
+
+/// The report, if it exists, is not deleted, and `actor` may correct or move it.
+async fn editable_report<T: ReportWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: ReportId,
+    now: DateTime<Utc>,
+) -> Result<SmellReport, LogError> {
+    let report = tx
+        .report(id)
+        .await?
+        .filter(|report| !report.is_deleted())
+        .ok_or(LogError::NotFound)?;
+    if can(
+        actor,
+        Action::EditEntry {
+            owner: report.user_id,
+        },
+        now,
+    ) {
+        Ok(report)
+    } else {
+        Err(LogError::Forbidden)
+    }
+}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-sqlite -p smoke-storage-postgres` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass on both backends, including `only_residents_and_owning_guests_can_close_or_reopen`, `correcting_after_the_end_moves_the_end_and_before_the_start_is_refused` and `guests_can_correct_and_move_only_their_own_reports`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/logbook.rs \
  crates/storage-contract/src/corrections.rs \
  crates/storage-contract/src/lib.rs
git commit -m "feat(domain): close, reopen, correct and move with the permission checks inside

Each use case reads the event or report inside the write transaction and
checks the spec §4 permission against that data. A correction or move after a
closed event's end moves the end (§7.5); one before the event started, or on
another day, is refused with OutsideEvent.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Domain — the month summary and a report's history

Two pure modules. `summary` counts what the summary card shows (decision #94). `history` turns a
report's audit entries into History lines: each correction's changed values ("intensity 6 → 7,
added bedroom", spec §6.1), moves, and whether the report was ever corrected (its "edited"
badge).

**Files:**
- Create: `crates/domain/src/history.rs`, `crates/domain/src/summary.rs`
- Modify: `crates/domain/src/lib.rs`, `crates/domain/src/starter.rs`

**Interfaces:**
- Consumes: `SmellReport`, `ChainedEntry`, `DailySchedule::local_date` (Task 4).
- Produces:
  - `smoke_domain::summary::{MonthSummary { events, reports, bedroom_days: Option<usize> },
    summarise(&[SmellReport], bedroom: Option<RoomId>, &DailySchedule) -> MonthSummary}`.
  - `smoke_domain::history::{Change, HistoryEntry { at, actor, action, changes },
    report_history(&[ChainedEntry]) -> Vec<HistoryEntry>, was_corrected(&[HistoryEntry]) -> bool}`.
  - `smoke_domain::starter::BEDROOM_KEY = "room-bedroom"`.

- [ ] **Step 1: Write the failing tests**

Create `crates/domain/src/history.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditEntity, AuditRecord, GENESIS_HASH};
    use crate::records::{Audited, ReportValues, SmellReport};
    use crate::types::{Intensity, ReportId};

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn report(intensity: u8, rooms: &[i64]) -> SmellReport {
        SmellReport {
            id: ReportId(9),
            event_id: EventId(4),
            user_id: UserId(1),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:01Z"),
            values: ReportValues::new(
                Intensity::new(intensity).unwrap(),
                rooms.iter().copied().map(RoomId).collect(),
                vec![SymptomId(1)],
                None,
            )
            .unwrap(),
            deleted_at: None,
        }
    }

    fn entry(
        action: AuditAction,
        before: Option<&SmellReport>,
        after: &SmellReport,
    ) -> ChainedEntry {
        ChainedEntry::seal(
            AuditRecord {
                id: 1,
                at_micros: t("2026-01-16T08:15:00Z").timestamp_micros(),
                actor_id: Some(UserId(2)),
                entity: AuditEntity::SmellReport,
                entity_id: 9,
                action,
                before: before.map(Audited::snapshot),
                after: Some(after.snapshot()),
            },
            GENESIS_HASH,
        )
    }

    #[test]
    fn a_correction_lists_each_changed_value() {
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.values = ReportValues::new(
            Intensity::new(7).unwrap(),
            vec![RoomId(1), RoomId(3)],
            vec![],
            Some("from the balcony"),
        )
        .unwrap();
        after.observed_at = t("2026-01-15T19:30:00Z");
        let history = report_history(&[
            entry(AuditAction::Create, None, &before),
            entry(AuditAction::Update, Some(&before), &after),
        ]);
        assert_eq!(history[0].changes, []);
        assert_eq!(history[1].at, t("2026-01-16T08:15:00Z"));
        assert_eq!(history[1].actor, Some(UserId(2)));
        assert_eq!(
            history[1].changes,
            [
                Change::ObservedAt {
                    from: t("2026-01-15T19:00:00Z"),
                    to: t("2026-01-15T19:30:00Z"),
                },
                Change::Intensity { from: 6, to: 7 },
                Change::RoomAdded(RoomId(3)),
                Change::SymptomRemoved(SymptomId(1)),
                Change::Note {
                    from: None,
                    to: Some("from the balcony".to_owned()),
                },
            ]
        );
        assert!(was_corrected(&history));
    }

    #[test]
    fn a_move_names_both_events_and_is_not_a_correction() {
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.event_id = EventId(5);
        let history = report_history(&[
            entry(AuditAction::Create, None, &before),
            entry(AuditAction::Move, Some(&before), &after),
        ]);
        assert_eq!(
            history[1].changes,
            [Change::Moved {
                from: EventId(4),
                to: EventId(5),
            }]
        );
        assert!(!was_corrected(&history));
    }

    #[test]
    fn a_deletion_has_no_value_changes() {
        let before = report(6, &[1]);
        let mut after = before.clone();
        after.deleted_at = Some(t("2026-01-16T08:15:00Z"));
        let history = report_history(&[entry(AuditAction::Delete, Some(&before), &after)]);
        assert_eq!(history[0].action, AuditAction::Delete);
        assert!(history[0].changes.is_empty());
    }
}
```

In `crates/domain/src/lib.rs`:

```diff
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -7,12 +7,14 @@ pub mod audit;
 pub mod clock;
 pub mod error;
 pub mod events;
+pub mod history;
 pub mod logbook;
 pub mod permissions;
 pub mod records;
 pub mod repos;
 pub mod schedule;
 pub mod starter;
+pub mod summary;
 pub mod sweep;
 pub mod types;
 pub mod verify;
```

Create `crates/domain/src/summary.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};

    use super::*;
    use crate::records::ReportValues;
    use crate::types::{EventId, Intensity, ReportId, UserId};

    fn report(id: i64, event: i64, at: &str, rooms: &[i64]) -> SmellReport {
        let at: DateTime<Utc> = at.parse().unwrap();
        SmellReport {
            id: ReportId(id),
            event_id: EventId(event),
            user_id: UserId(1),
            observed_at: at,
            recorded_at: at,
            values: ReportValues::new(
                Intensity::new(5).unwrap(),
                rooms.iter().copied().map(RoomId).collect(),
                vec![],
                None,
            )
            .unwrap(),
            deleted_at: None,
        }
    }

    fn rome() -> DailySchedule {
        DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
    }

    #[test]
    fn counts_events_reports_and_local_days_with_bedroom_smoke() {
        let bedroom = RoomId(3);
        let reports = [
            report(1, 1, "2026-01-10T20:00:00Z", &[3]),
            report(2, 1, "2026-01-10T22:30:00Z", &[1, 3]),
            // 00:30 on the 11th in Rome: a second bedroom day.
            report(3, 1, "2026-01-10T23:30:00Z", &[3]),
            report(4, 2, "2026-01-12T08:00:00Z", &[1]),
        ];
        assert_eq!(
            summarise(&reports, Some(bedroom), &rome()),
            MonthSummary {
                events: 2,
                reports: 4,
                bedroom_days: Some(2),
            }
        );
    }

    #[test]
    fn without_a_bedroom_or_reports_the_counts_are_empty() {
        assert_eq!(
            summarise(&[report(1, 1, "2026-01-10T20:00:00Z", &[3])], None, &rome()).bedroom_days,
            None
        );
        assert_eq!(
            summarise(&[], Some(RoomId(3)), &rome()),
            MonthSummary {
                events: 0,
                reports: 0,
                bedroom_days: Some(0),
            }
        );
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compile error — `summarise`, `report_history` and `was_corrected` are not defined.

- [ ] **Step 3: Implement**

In `crates/domain/src/history.rs`, above the test module:

```rust
//! A smell report's History view (spec §6.1): what each audit entry changed, e.g.
//! "intensity 6 → 7, added bedroom".

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::audit::{AuditAction, ChainedEntry};
use crate::types::{EventId, RoomId, SymptomId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    ObservedAt {
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    },
    Intensity {
        from: u8,
        to: u8,
    },
    RoomAdded(RoomId),
    RoomRemoved(RoomId),
    SymptomAdded(SymptomId),
    SymptomRemoved(SymptomId),
    Note {
        from: Option<String>,
        to: Option<String>,
    },
    Moved {
        from: EventId,
        to: EventId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub at: DateTime<Utc>,
    pub actor: Option<UserId>,
    pub action: AuditAction,
    /// Empty for the creation and for a deletion.
    pub changes: Vec<Change>,
}

/// The audited fields of a smell report, as `SmellReport` serialises them.
#[derive(Deserialize)]
struct Snapshot {
    event_id: EventId,
    observed_at: DateTime<Utc>,
    intensity: u8,
    rooms: Vec<RoomId>,
    symptoms: Vec<SymptomId>,
    note: Option<String>,
}

/// One report's audit entries (oldest first, from `AuditRepo::audit_history`) as history.
pub fn report_history(entries: &[ChainedEntry]) -> Vec<HistoryEntry> {
    entries
        .iter()
        .map(|entry| {
            let record = &entry.record;
            let snapshot = |value: &Option<serde_json::Value>| -> Option<Snapshot> {
                serde_json::from_value(value.clone()?).ok()
            };
            let changes = match (snapshot(&record.before), snapshot(&record.after)) {
                (Some(before), Some(after)) => changes(&before, &after),
                _ => Vec::new(),
            };
            HistoryEntry {
                at: DateTime::from_timestamp_micros(record.at_micros)
                    .expect("audit instants are in range"),
                actor: record.actor_id,
                action: record.action,
                changes,
            }
        })
        .collect()
}

/// Whether the report was ever corrected: what its "edited" badge shows.
pub fn was_corrected(history: &[HistoryEntry]) -> bool {
    history.iter().any(|e| e.action == AuditAction::Update)
}

fn changes(before: &Snapshot, after: &Snapshot) -> Vec<Change> {
    let mut changes = Vec::new();
    if before.event_id != after.event_id {
        changes.push(Change::Moved {
            from: before.event_id,
            to: after.event_id,
        });
    }
    if before.observed_at != after.observed_at {
        changes.push(Change::ObservedAt {
            from: before.observed_at,
            to: after.observed_at,
        });
    }
    if before.intensity != after.intensity {
        changes.push(Change::Intensity {
            from: before.intensity,
            to: after.intensity,
        });
    }
    let added = |old: &[RoomId], new: &[RoomId]| -> Vec<RoomId> {
        new.iter().filter(|id| !old.contains(id)).copied().collect()
    };
    changes.extend(
        added(&before.rooms, &after.rooms)
            .into_iter()
            .map(Change::RoomAdded),
    );
    changes.extend(
        added(&after.rooms, &before.rooms)
            .into_iter()
            .map(Change::RoomRemoved),
    );
    let added = |old: &[SymptomId], new: &[SymptomId]| -> Vec<SymptomId> {
        new.iter().filter(|id| !old.contains(id)).copied().collect()
    };
    changes.extend(
        added(&before.symptoms, &after.symptoms)
            .into_iter()
            .map(Change::SymptomAdded),
    );
    changes.extend(
        added(&after.symptoms, &before.symptoms)
            .into_iter()
            .map(Change::SymptomRemoved),
    );
    if before.note != after.note {
        changes.push(Change::Note {
            from: before.note.clone(),
            to: after.note.clone(),
        });
    }
    changes
}
```

In `crates/domain/src/starter.rs`:

```diff
--- a/crates/domain/src/starter.rs
+++ b/crates/domain/src/starter.rs
@@ -28,10 +28,13 @@ const fn symptom(i18n_key: &'static str, name: &'static str) -> StarterSymptom {
     StarterSymptom { i18n_key, name }
 }
 
+/// The starter bedroom, which the summary card counts days for.
+pub const BEDROOM_KEY: &str = "room-bedroom";
+
 pub const STARTER_ROOMS: [StarterRoom; 6] = [
     room("room-living-room", "Living room", RoomArea::Inside),
     room("room-kitchen", "Kitchen", RoomArea::Inside),
-    room("room-bedroom", "Bedroom", RoomArea::Inside),
+    room(BEDROOM_KEY, "Bedroom", RoomArea::Inside),
     room("room-bathroom", "Bathroom", RoomArea::Inside),
     room("room-entrance-hall", "Entrance hall", RoomArea::Inside),
     room("room-stairwell", "Stairwell (shared)", RoomArea::Common),
```

In `crates/domain/src/summary.rs`, above the test module:

```rust
//! The summary card on the Log screen (spec §9, screen 2): "This month: 14 events · 41 reports
//! · 9 days with bedroom smoke."

use std::collections::BTreeSet;

use crate::records::SmellReport;
use crate::schedule::DailySchedule;
use crate::types::RoomId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MonthSummary {
    /// Events with at least one of the reports.
    pub events: usize,
    pub reports: usize,
    /// Local days with a report naming the bedroom; `None` when there is no bedroom room.
    pub bedroom_days: Option<usize>,
}

/// Summarises `reports`, which the caller has already limited to one month and to what the
/// viewer may see. `bedroom` is the starter bedroom room, if it exists.
pub fn summarise(
    reports: &[SmellReport],
    bedroom: Option<RoomId>,
    schedule: &DailySchedule,
) -> MonthSummary {
    let events: BTreeSet<_> = reports.iter().map(|r| r.event_id).collect();
    let bedroom_days = bedroom.map(|room| {
        reports
            .iter()
            .filter(|r| r.values.rooms().contains(&room))
            .map(|r| schedule.local_date(r.observed_at))
            .collect::<BTreeSet<_>>()
            .len()
    });
    MonthSummary {
        events: events.len(),
        reports: reports.len(),
        bedroom_days,
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `counts_events_reports_and_local_days_with_bedroom_smoke` and `a_correction_lists_each_changed_value`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/history.rs \
  crates/domain/src/lib.rs \
  crates/domain/src/starter.rs \
  crates/domain/src/summary.rs
git commit -m "feat(domain): the month summary and a report's history from audit snapshots

summarise counts events, reports and local days with bedroom smoke for the
summary card; report_history deserialises each audit snapshot with serde and
lists what a correction or move changed, for the History view.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Web — the Log screen, the summary card and the rating popup

`GET /` (replacing the milestone 3 placeholder) runs the sweep, then shows the event status, the
summary card and the form: the 1–10 grid, rooms as chips grouped Inside / Common and pre-selected
from the user's last report in the open event, symptoms as checkboxes, PM2.5, the time ("now" with
a *change* disclosure) and a note; the main button reads "Update: situation changed" while an event
is open. Viewers and expired guests get no form. ⓘ *How to rate* is a link to the new
`/guide/rating` page that `static/app.js` turns into `showModal()` on a native `<dialog>`
(decision #97). The form posts to `/log`, which Task 9 adds. The header now shows who is signed
in.

**Files:**
- Create: `crates/web/src/labels.rs`, `crates/web/src/routes/guide.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/time.rs`, `crates/web/static/app.js`, `crates/web/templates/guide.html`, `crates/web/templates/log.html`, `crates/web/tests/log.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/assets.rs`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/mod.rs`, `crates/web/static/app.css`, `crates/web/templates/base.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/i18n.rs`, `crates/web/tests/security.rs`
- Delete: `crates/web/src/routes/home.rs`, `crates/web/templates/home.html`

**Interfaces:**
- Consumes: `auto_close` (Task 2), `summarise`/`BEDROOM_KEY` (Task 7), `can_see` (Task 4),
  `reports_between` (Task 1), `WebConfig.schedule` (Task 3).
- Produces:
  - `crate::time::{date, clock, date_time, since}`; `crate::labels::{room, symptom}`;
    `Tr::n(key, &usize)` (Fluent plurals on `$count`).
  - `crate::routes::log::{Draft, Choice, FormView, LogPage, show, log_page(state, user, tr, now,
    draft, error)}`; `crate::routes::guide::{Level, levels, rating}`.
  - Routes: `GET /` (Log screen), `GET /guide/rating`; asset `/static/app.js`.
  - Test helpers: `TestApp::{at, room, log}`, `t(…)`, `schedule()`.

- [ ] **Step 1: Write the failing tests**

The tests pin the clock with a new `TestApp::at`, log through the domain with `TestApp::log`, and
the test app now seeds the starter lists. The module declarations in `lib.rs` and
`routes/mod.rs` are part of this step; they fail to compile until Step 3.

In `crates/web/src/i18n.rs`, the tests:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -119,6 +127,7 @@ mod tests {
 
     use super::*;
     use crate::error;
+    use crate::routes::guide;
     use crate::routes::login::Notice;
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -138,12 +147,13 @@ mod tests {
             .collect()
     }
 
-    /// Every literal key in `tr.t("…")` and `tr.t1("…", …)` calls in the templates.
+    /// Every literal key in `tr.t("…")`, `tr.t1("…", …)` and `tr.n("…", …)` calls in the
+    /// templates.
     fn template_keys() -> BTreeSet<String> {
         let mut keys = BTreeSet::new();
         for entry in fs::read_dir(format!("{DIR}/templates")).unwrap() {
             let text = fs::read_to_string(entry.unwrap().path()).unwrap();
-            for call in ["t(\"", "t1(\""] {
+            for call in ["t(\"", "t1(\"", "n(\""] {
                 for (start, _) in text.match_indices(call) {
                     let rest = &text[start + call.len()..];
                     keys.insert(rest[..rest.find('"').unwrap()].to_owned());
@@ -185,6 +195,7 @@ mod tests {
         assert!(used.contains("app-name"), "the template scan found nothing");
         used.extend(error::KEYS.map(str::to_owned));
         used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
+        used.extend(guide::keys());
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
@@ -250,8 +261,17 @@ mod tests {
         assert_eq!(Tr(Locale::It).t("login-title"), "Accedi");
         assert_eq!(Tr(Locale::Tr).t("login-title"), "Giriş yap");
         assert_eq!(
-            Tr(Locale::En).t1("home-signed-in", "username", "daniele"),
+            Tr(Locale::En).t1("signed-in-as", "username", "daniele"),
             "Signed in as daniele."
         );
     }
+
+    #[test]
+    fn counts_pick_the_plural_form_of_each_language() {
+        assert_eq!(Tr(Locale::En).n("summary-events", &1), "1 event");
+        assert_eq!(Tr(Locale::En).n("summary-events", &14), "14 events");
+        assert_eq!(Tr(Locale::It).n("summary-reports", &1), "1 segnalazione");
+        assert_eq!(Tr(Locale::It).n("summary-reports", &41), "41 segnalazioni");
+        assert_eq!(Tr(Locale::Tr).n("summary-events", &14), "14 olay");
+    }
 }
```

Create `crates/web/src/labels.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use smoke_domain::types::{ListName, Locale, RoomArea, RoomId, SymptomId};

    use super::*;

    #[test]
    fn starter_items_are_translated_and_custom_ones_shown_as_typed() {
        let starter = Room {
            id: RoomId(3),
            name: ListName::new("Bedroom").unwrap(),
            i18n_key: Some("room-bedroom".to_owned()),
            area: RoomArea::Inside,
            hidden_at: None,
        };
        let custom = Room {
            i18n_key: None,
            name: ListName::new("Balcone").unwrap(),
            ..starter.clone()
        };
        assert_eq!(room(Tr(Locale::It), &starter), "Camera da letto");
        assert_eq!(room(Tr(Locale::Tr), &custom), "Balcone");
        let cough = Symptom {
            id: SymptomId(1),
            name: ListName::new("Cough").unwrap(),
            i18n_key: Some("symptom-cough".to_owned()),
            hidden_at: None,
        };
        assert_eq!(symptom(Tr(Locale::Tr), &cough), "Öksürük");
    }
}
```

In `crates/web/src/lib.rs`:

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -8,11 +8,13 @@ pub mod config;
 mod cookies;
 mod error;
 mod i18n;
+mod labels;
 mod request_id;
 mod routes;
 mod security;
 mod sessions;
 mod state;
+mod time;
 
 use std::future::Future;
 use std::net::SocketAddr;
@@ -39,7 +41,8 @@ const BODY_LIMIT_BYTES: usize = 16 * 1024;
 pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
     auth::password::prepare_dummy_hash();
     let signed_in = Router::new()
-        .route("/", get(routes::home::show))
+        .route("/", get(routes::log::show::<S>))
+        .route("/guide/rating", get(routes::guide::rating))
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
```

In `crates/web/src/routes/mod.rs`:

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,7 +1,8 @@
 //! One module per feature.
 
-pub(crate) mod home;
+pub(crate) mod guide;
 pub(crate) mod lang;
+pub(crate) mod log;
 pub(crate) mod login;
 
 use crate::error::AppError;
```

Create `crates/web/src/time.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use chrono_tz::Europe::Rome;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn times_are_shown_in_the_configured_zone_and_language() {
        let at = t("2026-01-15T18:05:00Z");
        assert_eq!(clock(at, Rome), "19:05");
        assert_eq!(date_time(Locale::En, at, Rome), "15/01/2026 19:05");
        assert_eq!(date_time(Locale::Tr, at, Rome), "15.01.2026 19:05");
        assert_eq!(
            date_time(Locale::It, t("2026-07-01T22:30:00Z"), Rome),
            "02/07/2026 00:30"
        );
    }

    #[test]
    fn today_shows_only_the_time() {
        let at = t("2026-01-15T18:05:00Z");
        assert_eq!(
            since(Locale::En, at, t("2026-01-15T22:59:00Z"), Rome),
            "19:05"
        );
        assert_eq!(
            since(Locale::En, at, t("2026-01-15T23:00:00Z"), Rome),
            "15/01/2026 19:05",
            "midnight in Rome"
        );
    }
}
```

In `crates/web/tests/common/mod.rs`:

```diff
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -10,12 +10,16 @@ use axum::body::Body;
 use axum::extract::ConnectInfo;
 use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
 use axum::http::{HeaderName, Request, Response, StatusCode};
-use chrono::TimeDelta;
+use chrono::{DateTime, TimeDelta, Utc};
 use http_body_util::BodyExt;
 use smoke_domain::clock::{Clock, FakeClock, SystemClock};
+use smoke_domain::logbook::{EventChoice, Logged, SmellEntry, log_smell};
+use smoke_domain::records::ReportValues;
 use smoke_domain::records::{NewUser, User};
-use smoke_domain::repos::{Commit, Transactional, UserWrites};
+use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
 use smoke_domain::schedule::DailySchedule;
+use smoke_domain::starter::seed_starter_lists;
+use smoke_domain::types::{Intensity, RoomId};
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
 use smoke_web::config::{BaseUrl, WebConfig};
@@ -48,10 +52,11 @@ impl TestApp {
         let clock = Arc::new(FakeClock::new(SystemClock.now()));
         let store = SqliteStore::connect(&db_url, clock.clone()).await.unwrap();
         store.migrate().await.unwrap();
+        seed_starter_lists(&store).await.unwrap();
         let config = WebConfig {
             base_url: BaseUrl::parse(base_url).unwrap(),
             client_ip_header: client_ip_header.map(HeaderName::from_static),
-            schedule: DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap(),
+            schedule: schedule(),
         };
         let router = smoke_web::router(AppState::new(store.clone(), clock.clone(), config));
         Self {
@@ -81,6 +86,57 @@ impl TestApp {
         user
     }
 
+    /// Sets the fake clock. Sessions still work: `tower-sessions` stamps their expiry with the
+    /// real time, which is later than any instant the tests use.
+    pub fn at(&self, instant: &str) {
+        self.clock.set(t(instant));
+    }
+
+    /// The id of a starter room, e.g. `room("room-bedroom")`.
+    pub async fn room(&self, key: &str) -> RoomId {
+        let rooms = self.store.rooms().await.unwrap();
+        rooms
+            .iter()
+            .find(|r| r.i18n_key.as_deref() == Some(key))
+            .unwrap()
+            .id
+    }
+
+    /// Logs a report through the domain, as the Log form would, at `at` (RFC 3339).
+    pub async fn log(
+        &self,
+        user: &User,
+        at: &str,
+        intensity: u8,
+        rooms: &[RoomId],
+        choice: EventChoice,
+    ) -> Logged {
+        let entry = SmellEntry {
+            observed_at: t(at),
+            values: ReportValues::new(
+                Intensity::new(intensity).unwrap(),
+                rooms.to_vec(),
+                vec![],
+                None,
+            )
+            .unwrap(),
+            pm25: None,
+        };
+        let mut tx = self.store.begin(Some(user.id)).await.unwrap();
+        let logged = log_smell(
+            &mut tx,
+            &user.actor(),
+            entry,
+            choice,
+            &schedule(),
+            self.clock.now(),
+        )
+        .await
+        .unwrap();
+        tx.commit().await.unwrap();
+        logged
+    }
+
     pub async fn send(&self, request: Request<Body>) -> Response<Body> {
         self.router.clone().oneshot(request).await.unwrap()
     }
@@ -97,6 +153,15 @@ impl TestApp {
     }
 }
 
+pub fn t(instant: &str) -> DateTime<Utc> {
+    instant.parse().unwrap()
+}
+
+/// Rome, auto-close at 04:00: what the test app is configured with.
+pub fn schedule() -> DailySchedule {
+    DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
+}
+
 pub fn get(path: &str) -> Request<Body> {
     Request::get(path).body(Body::empty()).unwrap()
 }
```

In `crates/web/tests/i18n.rs`:

```diff
--- a/crates/web/tests/i18n.rs
+++ b/crates/web/tests/i18n.rs
@@ -45,7 +45,10 @@ async fn once_signed_in_the_account_language_wins_over_the_cookie() {
     );
     let session = session_cookie(&response).unwrap();
     let cookies = format!("{session}; lang=tr");
-    assert_eq!(heading(&app, get_with("/", &cookies)).await, "Home");
+    assert_eq!(
+        heading(&app, get_with("/", &cookies)).await,
+        "Registra un odore"
+    );
     let html = text(app.send(get_with("/", &cookies)).await).await;
     assert!(html.contains(r#"<html lang="it">"#), "{html}");
     assert!(html.contains("Accesso effettuato come partner."), "{html}");
@@ -83,5 +86,5 @@ async fn choosing_a_language_while_signed_in_saves_it_to_the_account() {
     assert_eq!(location(&response), Some("/"));
     let saved = app.store.user(user.id).await.unwrap().unwrap();
     assert_eq!(saved.locale, Locale::Tr);
-    assert_eq!(heading(&app, get_with("/", &session)).await, "Ana sayfa");
+    assert_eq!(heading(&app, get_with("/", &session)).await, "Koku kaydet");
 }
```

Create `crates/web/tests/log.rs`:

```rust
//! The Log screen (spec §9, screen 1) and the "How to rate" guide (spec §5).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{Commit, EventRepo, ListWrites, Transactional, UserWrites};
use smoke_domain::types::{CloseReason, Locale, Role};

#[tokio::test]
async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let mut tx = app.store.begin(None).await.unwrap();
    tx.set_room_hidden(kitchen, true).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("daniele").await;

    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains("<h1>Log a smell</h1>"), "{html}");
    assert!(html.contains("No smell event in progress."), "{html}");
    assert!(
        html.contains(r#"<form method="post" action="/log""#),
        "{html}"
    );
    assert_eq!(html.matches(r#"name="intensity""#).count(), 10);
    let inside = html.find("Inside").unwrap();
    let common = html.find("Common areas").unwrap();
    let bedroom = html.find("<span>Bedroom</span>").unwrap();
    let stairwell = html.find("<span>Stairwell (shared)</span>").unwrap();
    assert!(inside < bedroom && bedroom < common && common < stairwell);
    assert!(!html.contains("Kitchen"), "hidden rooms are not offered");
    assert!(html.contains(r#"name="symptom""#) && html.contains("Eye irritation"));
    assert!(html.contains(">Log smell</button>"), "{html}");
    assert!(html.contains(">Log PM2.5 only</button>"), "{html}");
}

#[tokio::test]
async fn with_an_event_open_the_form_offers_an_update_with_my_last_rooms() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let (bedroom, kitchen) = (
        app.room("room-bedroom").await,
        app.room("room-kitchen").await,
    );
    let first = app
        .log(&me, "2026-01-15T18:05:00Z", 4, &[bedroom], EventChoice::New)
        .await;
    let event = EventChoice::Join(first.report.event_id);
    app.log(&partner, "2026-01-15T19:55:00Z", 6, &[kitchen], event)
        .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("Smell event in progress since 19:05."),
        "{html}"
    );
    assert!(
        html.contains(">Update: situation changed</button>"),
        "{html}"
    );
    assert!(
        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "my last rooms are pre-selected: {html}"
    );
    assert!(
        html.contains(&format!(r#"value="{}"><span>Kitchen"#, kitchen.0)),
        "not the partner's: {html}"
    );
}

#[tokio::test]
async fn viewers_and_expired_guests_see_no_form() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains(r#"action="/log""#), "{html}");
    assert!(html.contains("You can look at the data but not log."));

    let mut tx = app.store.begin(None).await.unwrap();
    tx.expire_user(guest.id).await.unwrap();
    tx.commit().await.unwrap();
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains(r#"action="/log""#), "{html}");
}

#[tokio::test]
async fn the_summary_counts_this_month_and_guests_count_only_their_own() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    app.log(&me, "2025-12-31T20:00:00Z", 5, &[bedroom], EventChoice::New)
        .await;
    let first = app
        .log(&me, "2026-01-10T20:00:00Z", 5, &[bedroom], EventChoice::New)
        .await;
    let join = EventChoice::Join(first.report.event_id);
    app.log(&guest, "2026-01-10T21:00:00Z", 3, &[], join).await;
    app.log(
        &guest,
        "2026-01-12T20:00:00Z",
        6,
        &[bedroom],
        EventChoice::New,
    )
    .await;

    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("This month: 2 events · 3 reports · 2 days with bedroom smoke"),
        "{html}"
    );
    let cookie = app.sign_in("guest").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("This month: 2 events · 2 reports · 1 day with bedroom smoke"),
        "{html}"
    );
}

#[tokio::test]
async fn opening_the_log_screen_runs_the_auto_close_sweep() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let logged = app
        .log(&me, "2026-01-15T20:00:00Z", 5, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    app.at("2026-01-16T07:00:00Z");

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(html.contains("No smell event in progress."), "{html}");
    let event = app.store.event(logged.report.event_id).await.unwrap();
    assert_eq!(
        event.unwrap().closed.map(|c| c.reason),
        Some(CloseReason::Auto0400)
    );
}

#[tokio::test]
async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::It).await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains(r#"<a href="/guide/rating" data-dialog="rating-guide""#),
        "{html}"
    );
    assert!(html.contains(r#"<dialog id="rating-guide""#), "{html}");
    assert!(html.contains(r#"<form method="dialog""#), "{html}");
    assert!(html.contains("<td>Appena percepibile</td>"), "{html}");
    assert!(
        html.contains("Tra due numeri, scegli il più basso."),
        "{html}"
    );
    assert!(html.contains(r#"<script src="/static/app.js" defer></script>"#));

    let guide = app.send(get_with("/guide/rating", &cookie)).await;
    assert_eq!(guide.status(), StatusCode::OK);
    let guide = text(guide).await;
    assert!(guide.contains("<h1>Come valutare</h1>"), "{guide}");
    assert!(
        guide.contains("Come una stanza piena di fumo. Devi uscire."),
        "{guide}"
    );
    let script = app.send(get("/static/app.js")).await;
    assert_eq!(script.status(), StatusCode::OK);
    assert!(text(script).await.contains("showModal()"));
    assert_eq!(
        location(&app.send(get("/guide/rating")).await),
        Some("/login")
    );
}
```

In `crates/web/tests/security.rs`:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -150,6 +150,7 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
     let pages = [
         text(app.send(get("/login")).await).await,
         text(app.send(get_with("/", &cookie)).await).await,
+        text(app.send(get_with("/guide/rating", &cookie)).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile error — `routes::log`, `routes::guide`, `labels`, `time` and `Tr::n` do not exist yet.

- [ ] **Step 3: Implement**

Delete the files this task replaces:

```bash
git rm crates/web/src/routes/home.rs crates/web/templates/home.html
```

In `crates/web/locales/en/main.ftl`:

```diff
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -1,4 +1,5 @@
 app-name = Smoke Tracker
+close = Close
 language = Language
 
 login-title = Sign in
@@ -8,11 +9,68 @@ login-submit = Sign in
 login-failed = Invalid username or password.
 login-locked = Too many attempts. Try again in 15 minutes.
 login-expired = This account has expired. Ask a resident to reactivate it.
+signed-in-as = Signed in as { $username }.
 logout = Sign out
 
-home-title = Home
-home-signed-in = Signed in as { $username }.
-home-coming-soon = Logging smells arrives in the next version.
+log-title = Log a smell
+event-none = No smell event in progress.
+event-open-since = Smell event in progress since { $time }.
+log-intensity = Intensity
+log-how-to-rate = How to rate
+log-rooms = Rooms
+log-inside = Inside
+log-common = Common areas
+log-symptoms = Symptoms
+log-pm25 = PM2.5 (µg/m³, optional)
+log-time = Time
+log-time-now = now
+log-time-change = change
+log-note = Note (optional)
+log-submit = Log smell
+log-update = Update: situation changed
+log-pm25-only = Log PM2.5 only
+log-read-only = You can look at the data but not log.
+
+summary-this-month = This month:
+summary-events = { $count ->
+    [one] 1 event
+   *[other] { $count } events
+}
+summary-reports = { $count ->
+    [one] 1 report
+   *[other] { $count } reports
+}
+summary-bedroom-days = { $count ->
+    [one] 1 day with bedroom smoke
+   *[other] { $count } days with bedroom smoke
+}
+
+rating-title = How to rate
+rating-level = Level
+rating-description = What it's like
+rating-tip = Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.
+rating-full-guide = Full guide →
+
+intensity-1-name = Barely there
+intensity-1-desc = Only if you focus. Unsure it's smoke.
+intensity-2-name = Faint
+intensity-2-desc = Noticeable entering the room, then fades.
+intensity-3-name = Light
+intensity-3-desc = Clearly smoke, but weak. Easy to ignore.
+intensity-4-name = Noticeable
+intensity-4-desc = Constantly present, mildly annoying.
+intensity-5-name = Moderate
+intensity-5-desc = Obvious all the time. You'd mention it.
+intensity-6-name = Strong
+intensity-6-desc = Hard to ignore; want to open a window or leave the room.
+intensity-7-name = Very strong
+intensity-7-desc = Can't stay comfortably; smell builds up.
+intensity-8-name = Heavy
+intensity-8-desc = Gets into clothes/bedding; lingers after you leave.
+intensity-9-name = Very heavy
+intensity-9-desc = Like standing next to someone smoking.
+intensity-10-name = Unbearable
+intensity-10-desc = Like a smoke-filled room. You must leave.
 
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
```

In `crates/web/locales/it/main.ftl`:

```diff
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -1,4 +1,5 @@
 app-name = Smoke Tracker
+close = Chiudi
 language = Lingua
 
 login-title = Accedi
@@ -8,11 +9,68 @@ login-submit = Accedi
 login-failed = Nome utente o password non validi.
 login-locked = Troppi tentativi. Riprova tra 15 minuti.
 login-expired = Questo account è scaduto. Chiedi a un residente di riattivarlo.
+signed-in-as = Accesso effettuato come { $username }.
 logout = Esci
 
-home-title = Home
-home-signed-in = Accesso effettuato come { $username }.
-home-coming-soon = La registrazione degli odori arriva nella prossima versione.
+log-title = Registra un odore
+event-none = Nessun evento di fumo in corso.
+event-open-since = Evento di fumo in corso dalle { $time }.
+log-intensity = Intensità
+log-how-to-rate = Come valutare
+log-rooms = Stanze
+log-inside = In casa
+log-common = Parti comuni
+log-symptoms = Sintomi
+log-pm25 = PM2.5 (µg/m³, facoltativo)
+log-time = Ora
+log-time-now = adesso
+log-time-change = cambia
+log-note = Nota (facoltativa)
+log-submit = Registra odore
+log-update = Aggiorna: la situazione è cambiata
+log-pm25-only = Registra solo PM2.5
+log-read-only = Puoi consultare i dati ma non registrare.
+
+summary-this-month = Questo mese:
+summary-events = { $count ->
+    [one] 1 evento
+   *[other] { $count } eventi
+}
+summary-reports = { $count ->
+    [one] 1 segnalazione
+   *[other] { $count } segnalazioni
+}
+summary-bedroom-days = { $count ->
+    [one] 1 giorno con fumo in camera da letto
+   *[other] { $count } giorni con fumo in camera da letto
+}
+
+rating-title = Come valutare
+rating-level = Livello
+rating-description = Come si presenta
+rating-tip = Valuta quello che senti adesso, non quanto sei infastidito. Tra due numeri, scegli il più basso.
+rating-full-guide = Guida completa →
+
+intensity-1-name = Appena percepibile
+intensity-1-desc = Solo se ci fai caso. Non sei sicuro che sia fumo.
+intensity-2-name = Debole
+intensity-2-desc = Si nota entrando nella stanza, poi svanisce.
+intensity-3-name = Leggero
+intensity-3-desc = Chiaramente fumo, ma debole. Facile da ignorare.
+intensity-4-name = Percepibile
+intensity-4-desc = Sempre presente, un po' fastidioso.
+intensity-5-name = Moderato
+intensity-5-desc = Evidente tutto il tempo. Lo faresti notare.
+intensity-6-name = Forte
+intensity-6-desc = Difficile da ignorare; vorresti aprire una finestra o uscire dalla stanza.
+intensity-7-name = Molto forte
+intensity-7-desc = Non si riesce a stare comodi; l'odore si accumula.
+intensity-8-name = Pesante
+intensity-8-desc = Entra nei vestiti e nelle lenzuola; resta anche dopo che sei uscito.
+intensity-9-name = Molto pesante
+intensity-9-desc = Come stare accanto a qualcuno che fuma.
+intensity-10-name = Insopportabile
+intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
```

In `crates/web/locales/tr/main.ftl`:

```diff
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -1,4 +1,5 @@
 app-name = Smoke Tracker
+close = Kapat
 language = Dil
 
 login-title = Giriş yap
@@ -8,11 +9,68 @@ login-submit = Giriş yap
 login-failed = Kullanıcı adı veya şifre geçersiz.
 login-locked = Çok fazla deneme. 15 dakika sonra tekrar deneyin.
 login-expired = Bu hesabın süresi doldu. Yeniden etkinleştirmesi için bir ev sakinine başvurun.
+signed-in-as = { $username } olarak giriş yapıldı.
 logout = Çıkış yap
 
-home-title = Ana sayfa
-home-signed-in = { $username } olarak giriş yapıldı.
-home-coming-soon = Koku kaydı bir sonraki sürümde geliyor.
+log-title = Koku kaydet
+event-none = Devam eden bir koku olayı yok.
+event-open-since = { $time } saatinden beri süren bir koku olayı var.
+log-intensity = Yoğunluk
+log-how-to-rate = Nasıl puanlanır
+log-rooms = Odalar
+log-inside = Ev içi
+log-common = Ortak alanlar
+log-symptoms = Belirtiler
+log-pm25 = PM2.5 (µg/m³, isteğe bağlı)
+log-time = Saat
+log-time-now = şimdi
+log-time-change = değiştir
+log-note = Not (isteğe bağlı)
+log-submit = Kokuyu kaydet
+log-update = Güncelle: durum değişti
+log-pm25-only = Yalnızca PM2.5 kaydet
+log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.
+
+summary-this-month = Bu ay:
+summary-events = { $count ->
+    [one] 1 olay
+   *[other] { $count } olay
+}
+summary-reports = { $count ->
+    [one] 1 bildirim
+   *[other] { $count } bildirim
+}
+summary-bedroom-days = { $count ->
+    [one] yatak odasında duman olan 1 gün
+   *[other] yatak odasında duman olan { $count } gün
+}
+
+rating-title = Nasıl puanlanır
+rating-level = Seviye
+rating-description = Nasıl bir şey
+rating-tip = Şu an aldığın kokuyu puanla, ne kadar rahatsız olduğunu değil. İki sayı arasında kalırsan küçük olanı seç.
+rating-full-guide = Tüm rehber →
+
+intensity-1-name = Zar zor
+intensity-1-desc = Ancak dikkat edersen. Duman olduğundan emin değilsin.
+intensity-2-name = Hafif
+intensity-2-desc = Odaya girerken fark edilir, sonra kaybolur.
+intensity-3-name = Az
+intensity-3-desc = Açıkça duman, ama zayıf. Görmezden gelmek kolay.
+intensity-4-name = Belirgin
+intensity-4-desc = Sürekli var, biraz rahatsız edici.
+intensity-5-name = Orta
+intensity-5-desc = Her an belli. Birine söylerdin.
+intensity-6-name = Güçlü
+intensity-6-desc = Görmezden gelmek zor; pencereyi açmak ya da odadan çıkmak istersin.
+intensity-7-name = Çok güçlü
+intensity-7-desc = Rahat duramazsın; koku birikir.
+intensity-8-name = Ağır
+intensity-8-desc = Giysilere ve yatak takımına siner; çıktıktan sonra da kalır.
+intensity-9-name = Çok ağır
+intensity-9-desc = Sigara içen birinin yanında durmak gibi.
+intensity-10-name = Dayanılmaz
+intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
```

In `crates/web/src/assets.rs`:

```diff
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -27,6 +27,7 @@ macro_rules! asset {
 
 const ASSETS: &[Asset] = &[
     asset!("app.css", "text/css; charset=utf-8"),
+    asset!("app.js", "text/javascript; charset=utf-8"),
     asset!("favicon.svg", "image/svg+xml"),
     asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
 ];
```

In `crates/web/src/i18n.rs`:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -45,7 +45,7 @@ impl Tr {
         LOCALES.lookup(language(self.0), key)
     }
 
-    /// A message with one argument, e.g. `tr.t1("home-signed-in", "username", name)`.
+    /// A message with one argument, e.g. `tr.t1("signed-in-as", "username", name)`.
     pub(crate) fn t1(&self, key: &str, name: &str, value: &str) -> String {
         let args: HashMap<Cow<'static, str>, FluentValue> = HashMap::from([(
             Cow::Owned(name.to_owned()),
@@ -53,6 +53,14 @@ impl Tr {
         )]);
         LOCALES.lookup_with_args(language(self.0), key, &args)
     }
+
+    /// A message that depends on a number, e.g. `tr.n("summary-events", &3)`: Fluent picks the
+    /// language's plural form from `$count`. Takes a reference because templates pass one.
+    pub(crate) fn n(&self, key: &str, count: &usize) -> String {
+        let args: HashMap<Cow<'static, str>, FluentValue> =
+            HashMap::from([(Cow::Borrowed("count"), FluentValue::from(*count))]);
+        LOCALES.lookup_with_args(language(self.0), key, &args)
+    }
 }
 
 /// The language chosen by [`crate::auth::load_user`]; English if that middleware did not run.
```

In `crates/web/src/labels.rs`, above the test module:

```rust
//! Names shown for stored values: starter rooms and symptoms through their translation key,
//! custom ones as typed.

use smoke_domain::records::{Room, Symptom};

use crate::i18n::Tr;

pub(crate) fn room(tr: Tr, room: &Room) -> String {
    match &room.i18n_key {
        Some(key) => tr.t(key),
        None => room.name.to_string(),
    }
}

pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
    match &symptom.i18n_key {
        Some(key) => tr.t(key),
        None => symptom.name.to_string(),
    }
}
```

Create `crates/web/src/routes/guide.rs`:

```rust
//! The intensity guide (spec §5): a compact version in a popup on the Log screen, and this full
//! page. The texts live in the translation files so they are easy to edit.

use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;
use smoke_domain::types::Intensity;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;

/// One row of the guide.
pub(crate) struct Level {
    pub(crate) value: u8,
    pub(crate) name: String,
    pub(crate) description: String,
}

pub(crate) fn levels(tr: Tr) -> Vec<Level> {
    (Intensity::MIN..=Intensity::MAX)
        .map(|value| Level {
            value,
            name: tr.t(&format!("intensity-{value}-name")),
            description: tr.t(&format!("intensity-{value}-desc")),
        })
        .collect()
}

/// Every key [`levels`] looks up, for the translation tests.
#[cfg(test)]
pub(crate) fn keys() -> Vec<String> {
    (Intensity::MIN..=Intensity::MAX)
        .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")])
        .collect()
}

#[derive(Template)]
#[template(path = "guide.html")]
struct GuidePage {
    tr: Tr,
    signed_in: Option<String>,
    levels: Vec<Level>,
}

pub(crate) async fn rating(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
    page(
        StatusCode::OK,
        &GuidePage {
            tr,
            signed_in: Some(user.username.to_string()),
            levels: levels(tr),
        },
    )
}
```

Create `crates/web/src/routes/log.rs`:

```rust
//! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::User;
use smoke_domain::repos::Store;
use smoke_domain::starter::BEDROOM_KEY;
use smoke_domain::summary::{MonthSummary, summarise};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{Intensity, RoomArea, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;
use crate::routes::guide::{Level, levels};
use crate::state::AppState;
use crate::{labels, time};

/// What the form holds: blank, pre-filled, or as typed before a validation error.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Draft {
    pub(crate) intensity: Option<u8>,
    pub(crate) rooms: Vec<RoomId>,
    pub(crate) symptoms: Vec<SymptomId>,
    pub(crate) pm25: String,
    pub(crate) observed_at: String,
    pub(crate) note: String,
}

/// A radio button, chip or checkbox.
pub(crate) struct Choice {
    value: i64,
    label: String,
    checked: bool,
}

pub(crate) struct FormView {
    intensities: Vec<Choice>,
    inside: Vec<Choice>,
    common: Vec<Choice>,
    symptoms: Vec<Choice>,
    pm25: String,
    observed_at: String,
    note: String,
    /// An event is open, so the main button reads "Update: situation changed".
    update: bool,
    /// A translation key explaining what to fix.
    error: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "log.html")]
pub(crate) struct LogPage {
    tr: Tr,
    signed_in: Option<String>,
    open_since: Option<String>,
    summary: MonthSummary,
    levels: Vec<Level>,
    /// `None` for people who may not log (viewers, expired guests).
    form: Option<FormView>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let log_page = log_page(&state, &user, tr, now, None, None).await?;
    page(StatusCode::OK, &log_page)
}

/// Builds the Log screen. Without a `draft`, the rooms of the user's last report in the open
/// event are pre-selected (spec §9).
pub(crate) async fn log_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    draft: Option<Draft>,
    error: Option<&'static str>,
) -> Result<LogPage, AppError> {
    let schedule = &state.config.schedule;
    let actor = user.actor();
    let open = state.store.open_event().await?;
    let rooms = state.store.rooms().await?;

    let (month_start, month_end) = schedule.month_span(schedule.local_date(now));
    let visible: Vec<_> = state
        .store
        .reports_between(month_start, month_end)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let bedroom = rooms
        .iter()
        .find(|r| r.i18n_key.as_deref() == Some(BEDROOM_KEY))
        .map(|r| r.id);
    let summary = summarise(&visible, bedroom, schedule);

    let form = if can(&actor, Action::LogEntry, now) {
        let draft = match draft {
            Some(draft) => draft,
            None => {
                let mut blank = Draft::default();
                if let Some(open) = &open {
                    let reports = state.store.reports_for_event(open.id).await?;
                    if let Some(last) = reports.iter().rev().find(|r| r.user_id == user.id) {
                        blank.rooms = last.values.rooms().to_vec();
                    }
                }
                blank
            }
        };
        let room_choices = |area: RoomArea| {
            rooms
                .iter()
                .filter(|r| r.area == area && r.hidden_at.is_none())
                .map(|r| Choice {
                    value: r.id.0,
                    label: labels::room(tr, r),
                    checked: draft.rooms.contains(&r.id),
                })
                .collect()
        };
        let symptoms = state
            .store
            .symptoms()
            .await?
            .iter()
            .filter(|s| s.hidden_at.is_none())
            .map(|s| Choice {
                value: s.id.0,
                label: labels::symptom(tr, s),
                checked: draft.symptoms.contains(&s.id),
            })
            .collect();
        Some(FormView {
            intensities: (Intensity::MIN..=Intensity::MAX)
                .map(|value| Choice {
                    value: value.into(),
                    label: value.to_string(),
                    checked: draft.intensity == Some(value),
                })
                .collect(),
            inside: room_choices(RoomArea::Inside),
            common: room_choices(RoomArea::Common),
            symptoms,
            pm25: draft.pm25,
            observed_at: draft.observed_at,
            note: draft.note,
            update: open.is_some(),
            error,
        })
    } else {
        None
    };

    Ok(LogPage {
        tr,
        signed_in: Some(user.username.to_string()),
        open_since: open.map(|e| time::since(tr.0, e.opened_at, now, schedule.tz())),
        summary,
        levels: levels(tr),
        form,
    })
}
```

In `crates/web/src/time.rs`, above the test module:

```rust
//! Times as people read and type them: in the configured time zone (`APP_TIMEZONE`), never the
//! browser's, so every phone shows the same times.

use chrono::{DateTime, NaiveDate, Utc};
use chrono_tz::Tz;
use smoke_domain::types::Locale;

/// A calendar date as the language writes it with digits.
pub(crate) fn date(locale: Locale, date: NaiveDate) -> String {
    let pattern = match locale {
        Locale::En | Locale::It => "%d/%m/%Y",
        Locale::Tr => "%d.%m.%Y",
    };
    date.format(pattern).to_string()
}

/// `19:05`, in the local time zone.
pub(crate) fn clock(at: DateTime<Utc>, tz: Tz) -> String {
    at.with_timezone(&tz).format("%H:%M").to_string()
}

/// `15/01/2026 19:05`, in the local time zone.
pub(crate) fn date_time(locale: Locale, at: DateTime<Utc>, tz: Tz) -> String {
    let local_date = at.with_timezone(&tz).date_naive();
    format!("{} {}", date(locale, local_date), clock(at, tz))
}

/// Just the time when `at` is on the same local day as `now`, else the date and time.
pub(crate) fn since(locale: Locale, at: DateTime<Utc>, now: DateTime<Utc>, tz: Tz) -> String {
    if at.with_timezone(&tz).date_naive() == now.with_timezone(&tz).date_naive() {
        clock(at, tz)
    } else {
        date_time(locale, at, tz)
    }
}
```

In `crates/web/static/app.css`:

```diff
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -64,3 +64,134 @@ footer {
   margin-top: calc(var(--gap) * 3);
   font-size: 0.875rem;
 }
+
+.who {
+  display: flex;
+  align-items: center;
+  gap: 0.5rem;
+  font-size: 0.875rem;
+}
+
+.status,
+.summary {
+  margin: 0.5rem 0;
+}
+
+.summary {
+  font-size: 0.875rem;
+}
+
+fieldset {
+  border: none;
+  margin: 0;
+  padding: 0;
+}
+
+legend {
+  font-weight: 600;
+  margin-bottom: 0.5rem;
+}
+
+.levels {
+  display: grid;
+  grid-template-columns: repeat(5, 1fr);
+  gap: 0.5rem;
+}
+
+.levels legend {
+  grid-column: 1 / -1;
+}
+
+.chips {
+  display: flex;
+  flex-wrap: wrap;
+  gap: 0.5rem;
+}
+
+.chips legend,
+.chips .group {
+  flex-basis: 100%;
+  margin: 0;
+}
+
+.chip {
+  position: relative;
+}
+
+.chip input {
+  position: absolute;
+  opacity: 0;
+}
+
+.chip span {
+  display: block;
+  padding: 0.6rem 0.9rem;
+  border: 1px solid currentColor;
+  border-radius: 999px;
+  text-align: center;
+}
+
+.chip input:checked + span {
+  background: var(--accent);
+  border-color: var(--accent);
+  color: #fff;
+}
+
+.chip input:focus-visible + span {
+  outline: 2px solid var(--accent);
+  outline-offset: 2px;
+}
+
+.checks label {
+  display: block;
+  padding: 0.25rem 0;
+}
+
+.info {
+  font-weight: 400;
+  margin-left: 0.5rem;
+}
+
+textarea {
+  font: inherit;
+  padding: 0.75rem;
+  border-radius: 0.5rem;
+}
+
+button.secondary {
+  background: none;
+  color: inherit;
+  border: 1px solid currentColor;
+}
+
+dialog {
+  padding: 0;
+  border: none;
+  border-radius: 0.75rem;
+  max-width: 28rem;
+}
+
+dialog::backdrop {
+  background: rgb(0 0 0 / 50%);
+}
+
+.dialog-body {
+  padding: var(--gap);
+}
+
+.dialog-close {
+  float: right;
+}
+
+table.guide {
+  border-collapse: collapse;
+  width: 100%;
+}
+
+table.guide th,
+table.guide td {
+  text-align: left;
+  vertical-align: top;
+  padding: 0.35rem 0.5rem;
+  border-bottom: 1px solid rgb(128 128 128 / 30%);
+}
```

Create `crates/web/static/app.js`:

```javascript
// Progressive enhancement, loaded by every page. Links with a data-dialog attribute open that
// <dialog> instead of navigating (without JavaScript they open the full page), and tapping a
// dialog's backdrop closes it. The close button is a method="dialog" form and needs no script.
document.addEventListener("click", (event) => {
  const opener = event.target.closest("[data-dialog]");
  if (opener) {
    const dialog = document.getElementById(opener.dataset.dialog);
    if (dialog instanceof HTMLDialogElement) {
      event.preventDefault();
      dialog.showModal();
    }
    return;
  }
  if (event.target instanceof HTMLDialogElement) {
    event.target.close();
  }
});
```

In `crates/web/templates/base.html`:

```diff
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -8,12 +8,14 @@
   <link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
   <link rel="stylesheet" href="/static/app.css">
   <script src="/static/htmx-2.0.11.min.js" defer></script>
+  <script src="/static/app.js" defer></script>
 </head>
 <body>
   <header class="bar">
     <span class="brand">{{ tr.t("app-name") }}</span>
-    {% if signed_in.is_some() %}
-    <form method="post" action="/logout">
+    {% if let Some(username) = signed_in %}
+    <form method="post" action="/logout" class="who">
+      <span>{{ tr.t1("signed-in-as", "username", username) }}</span>
       <button type="submit" class="link">{{ tr.t("logout") }}</button>
     </form>
     {% endif %}
```

Create `crates/web/templates/guide.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("rating-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("rating-title") }}</h1>
<table class="guide">
  <thead>
    <tr><th scope="col">{{ tr.t("rating-level") }}</th><th scope="col">{{ tr.t("rating-description") }}</th></tr>
  </thead>
  <tbody>
    {% for level in levels %}
    <tr><th scope="row">{{ level.value }} · {{ level.name }}</th><td>{{ level.description }}</td></tr>
    {% endfor %}
  </tbody>
</table>
<p>{{ tr.t("rating-tip") }}</p>
<p><a href="/">{{ tr.t("error-back-home") }}</a></p>
{% endblock %}
```

Create `crates/web/templates/log.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("log-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("log-title") }}</h1>
<p class="status">
  {% if let Some(since) = open_since %}{{ tr.t1("event-open-since", "time", since) }}{% else %}{{ tr.t("event-none") }}{% endif %}
</p>
<p class="summary">
  {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %}
</p>
{% if let Some(form) = form %}
{% if let Some(error) = form.error %}
<p class="notice" role="alert">{{ tr.t(error) }}</p>
{% endif %}
<form method="post" action="/log" class="stack log">
  <fieldset class="levels">
    <legend>{{ tr.t("log-intensity") }}
      <a href="/guide/rating" data-dialog="rating-guide" class="info">ⓘ {{ tr.t("log-how-to-rate") }}</a>
    </legend>
    {% for level in form.intensities %}
    <label class="chip"><input type="radio" name="intensity" value="{{ level.value }}" required{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
    {% endfor %}
  </fieldset>
  <fieldset class="chips">
    <legend>{{ tr.t("log-rooms") }}</legend>
    <p class="group">{{ tr.t("log-inside") }}</p>
    {% for room in form.inside %}
    <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
    {% endfor %}
    {% if !form.common.is_empty() %}
    <p class="group">{{ tr.t("log-common") }}</p>
    {% for room in form.common %}
    <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
    {% endfor %}
    {% endif %}
  </fieldset>
  <fieldset class="checks">
    <legend>{{ tr.t("log-symptoms") }}</legend>
    {% for symptom in form.symptoms %}
    <label><input type="checkbox" name="symptom" value="{{ symptom.value }}"{% if symptom.checked %} checked{% endif %}> {{ symptom.label }}</label>
    {% endfor %}
  </fieldset>
  <label>{{ tr.t("log-pm25") }}
    <input type="number" name="pm25" min="0" max="1000" step="0.1" inputmode="decimal" value="{{ form.pm25 }}">
  </label>
  <details class="when"{% if !form.observed_at.is_empty() %} open{% endif %}>
    <summary>{{ tr.t("log-time") }}: {{ tr.t("log-time-now") }} · {{ tr.t("log-time-change") }}</summary>
    <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" aria-label="{{ tr.t("log-time") }}">
  </details>
  <label>{{ tr.t("log-note") }}
    <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
  </label>
  <button type="submit" name="action" value="smell">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
  <button type="submit" name="action" value="pm25" formnovalidate class="secondary">{{ tr.t("log-pm25-only") }}</button>
</form>
<dialog id="rating-guide" aria-labelledby="rating-guide-title">
  <div class="dialog-body">
    <form method="dialog" class="dialog-close">
      <button type="submit" class="link" aria-label="{{ tr.t("close") }}">✕</button>
    </form>
    <h2 id="rating-guide-title">{{ tr.t("rating-title") }}</h2>
    <table class="guide compact">
      {% for level in levels %}
      <tr><th scope="row">{{ level.value }}</th><td>{{ level.name }}</td></tr>
      {% endfor %}
    </table>
    <p>{{ tr.t("rating-tip") }}</p>
    <p><a href="/guide/rating">{{ tr.t("rating-full-guide") }}</a></p>
  </div>
</dialog>
{% else %}
<p>{{ tr.t("log-read-only") }}</p>
{% endif %}
{% endblock %}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `with_an_event_open_the_form_offers_an_update_with_my_last_rooms`, `the_summary_counts_this_month_and_guests_count_only_their_own`, `opening_the_log_screen_runs_the_auto_close_sweep`, `how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide` and `no_page_needs_inline_scripts_styles_or_eval`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/assets.rs \
  crates/web/src/i18n.rs \
  crates/web/src/labels.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/guide.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/time.rs \
  crates/web/static/app.css \
  crates/web/static/app.js \
  crates/web/templates/base.html \
  crates/web/templates/guide.html \
  crates/web/templates/log.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/i18n.rs \
  crates/web/tests/log.rs \
  crates/web/tests/security.rs
git commit -m "feat(web): the Log screen, the summary card and a native <dialog> rating guide

GET / runs the auto-close sweep, then shows the event status, the month's
counts (guests count only their own reports) and the form: an intensity grid,
rooms as chips grouped by area and pre-selected from the user's last report in
the open event, symptoms, PM2.5, time and note. 'How to rate' is a link that a
small self-hosted script turns into showModal() on a <dialog>, so the strict
CSP holds and the full guide page still opens without JavaScript.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Web — saving the Log form behind the event questions

`POST /log` reads the form as ordered pairs (rooms and symptoms repeat), runs the sweep, and asks
the question `log_prompt` picks: "Same smell still going?", "Which event was this?" (that day's
joinable events; guests see start times only) or "The event from … was auto-closed". The question
page re-posts the form as hidden fields plus `asked`, naming the question; an answer to a question
that no longer applies gets the current one (decision #87). Then `log_smell`/`log_reading` run in
one transaction and the browser is redirected to `/?saved=…`. Validation errors show the form again
as typed with a 422. While an event is open, a pending auto-close notice is a banner with
*Dismiss*. A role refusal now renders "Not allowed" (403) instead of the cross-site page.

**Files:**
- Create: `crates/web/src/forms.rs`, `crates/web/src/routes/entries.rs`, `crates/web/src/routes/notices.rs`, `crates/web/templates/prompt.html`, `crates/web/tests/logging.rs`
- Modify: `Cargo.toml`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/error.rs`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/time.rs`, `crates/web/templates/log.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/security.rs`

**Interfaces:**
- Consumes: `log_smell`, `log_reading`, `LogError` (Tasks 5–6), `log_prompt`,
  `can_join_backdated`, `auto_close_notice_is_for`, `unseen_auto_closed`, `log_page` (Task 8).
- Produces:
  - `crate::forms::Pairs` (extractor) with `get`, `ids`, `without`.
  - `crate::time::{parse_input, TimeInputError}`; `AppError::NotAllowed`.
  - `crate::routes::refusal(LogError) -> Result<&'static str, AppError>` and `REFUSALS`.
  - `crate::routes::entries::submit` (`POST /log`), `crate::routes::notices::{pending,
    mark_seen, dismiss}` (`POST /notices/dismiss`); `log_page(…, saved)` gains a `saved` key;
    `Draft::from_form`.
  - Test helper `TestApp::sweep`.

- [ ] **Step 1: Write the failing tests**

Create `crates/web/src/forms.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_names_are_all_kept() {
        let form = Pairs::of(&[
            ("intensity", " 6 "),
            ("room", "3"),
            ("room", "1"),
            ("note", "  "),
        ]);
        assert_eq!(form.get("intensity"), Some("6"));
        assert_eq!(form.get("note"), None);
        assert_eq!(form.get("missing"), None);
        assert_eq!(form.ids("room"), Some(vec![3, 1]));
        assert_eq!(form.ids("symptom"), Some(vec![]));
        assert_eq!(Pairs::of(&[("room", "x")]).ids("room"), None);
        assert_eq!(
            form.without(&["room", "note"]),
            [("intensity".to_owned(), " 6 ".to_owned())]
        );
    }
}
```

In `crates/web/src/i18n.rs`, the tests:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,8 +127,8 @@ mod tests {
 
     use super::*;
     use crate::error;
-    use crate::routes::guide;
     use crate::routes::login::Notice;
+    use crate::routes::{self, entries, guide};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
 
@@ -147,13 +147,27 @@ mod tests {
             .collect()
     }
 
-    /// Every literal key in `tr.t("…")`, `tr.t1("…", …)` and `tr.n("…", …)` calls in the
-    /// templates.
-    fn template_keys() -> BTreeSet<String> {
+    /// Every string literal passed as the key to `tr.t`, `tr.t1` or `tr.n` in the templates
+    /// and the Rust sources.
+    fn literal_keys() -> BTreeSet<String> {
+        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
+            for entry in fs::read_dir(dir).unwrap() {
+                let path = entry.unwrap().path();
+                if path.is_dir() {
+                    files(&path, out);
+                } else {
+                    out.push(path);
+                }
+            }
+        }
+        let mut paths = Vec::new();
+        for dir in ["templates", "src"] {
+            files(std::path::Path::new(&format!("{DIR}/{dir}")), &mut paths);
+        }
         let mut keys = BTreeSet::new();
-        for entry in fs::read_dir(format!("{DIR}/templates")).unwrap() {
-            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
-            for call in ["t(\"", "t1(\"", "n(\""] {
+        for path in paths {
+            let text = fs::read_to_string(path).unwrap();
+            for call in ["tr.t(\"", "tr.t1(\"", "tr.n(\""] {
                 for (start, _) in text.match_indices(call) {
                     let rest = &text[start + call.len()..];
                     keys.insert(rest[..rest.find('"').unwrap()].to_owned());
@@ -191,11 +205,17 @@ mod tests {
 
     #[test]
     fn every_key_the_pages_use_is_translated() {
-        let mut used = template_keys();
+        let mut used = literal_keys();
         assert!(used.contains("app-name"), "the template scan found nothing");
+        assert!(
+            used.contains("prompt-same-smell"),
+            "the source scan found nothing"
+        );
         used.extend(error::KEYS.map(str::to_owned));
         used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
         used.extend(guide::keys());
+        used.extend(entries::FORM_ERRORS.map(str::to_owned));
+        used.extend(routes::REFUSALS.map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
```

In `crates/web/src/lib.rs`:

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -7,6 +7,7 @@ mod auth;
 pub mod config;
 mod cookies;
 mod error;
+mod forms;
 mod i18n;
 mod labels;
 mod request_id;
@@ -43,6 +44,8 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
     let signed_in = Router::new()
         .route("/", get(routes::log::show::<S>))
         .route("/guide/rating", get(routes::guide::rating))
+        .route("/log", post(routes::entries::submit::<S>))
+        .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
```

Create `crates/web/src/routes/entries.rs` with its test module (Step 3 adds the code above it):

```rust
#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use smoke_domain::types::{ListName, RoomArea};

    use super::*;

    fn now() -> DateTime<Utc> {
        "2026-01-15T20:00:00Z".parse().unwrap()
    }

    fn rooms() -> Vec<Room> {
        let room = |id, hidden: bool| Room {
            id: RoomId(id),
            name: ListName::new("Room").unwrap(),
            i18n_key: None,
            area: RoomArea::Inside,
            hidden_at: hidden.then(now),
        };
        vec![room(1, false), room(2, true)]
    }

    fn read_form(pairs: &[(&str, &str)]) -> Result<Entry, &'static str> {
        read(
            &Pairs::of(pairs),
            &rooms(),
            &[],
            chrono_tz::Europe::Rome,
            now(),
        )
    }

    #[test]
    fn a_smell_needs_an_intensity_and_offered_rooms_only() {
        let Ok(Entry::Smell(entry)) = read_form(&[("intensity", "6"), ("room", "1")]) else {
            panic!("a valid smell");
        };
        assert_eq!(entry.observed_at, now());
        assert_eq!(entry.values.rooms(), [RoomId(1)]);
        assert_eq!(entry.pm25, None);
        for (pairs, key) in [
            (&[("room", "1")][..], "log-error-intensity"),
            (&[("intensity", "11")][..], "log-error-intensity"),
            (&[("intensity", "6"), ("room", "2")][..], "log-error-form"),
            (&[("intensity", "6"), ("room", "x")][..], "log-error-form"),
            (
                &[("intensity", "6"), ("symptom", "1")][..],
                "log-error-form",
            ),
            (&[("intensity", "6"), ("pm25", "-1")][..], "log-error-pm25"),
            (
                &[("intensity", "6"), ("observed_at", "2026-01-15T22:00")][..],
                "refused-future",
            ),
        ] {
            assert_eq!(read_form(pairs).err(), Some(key), "{pairs:?}");
        }
    }

    #[test]
    fn pm25_only_ignores_the_smell_fields_but_needs_a_value() {
        let Ok(Entry::Pm25Only { observed_at, value }) = read_form(&[
            ("action", "pm25"),
            ("pm25", "12,5"),
            ("observed_at", "2026-01-15T20:30"),
        ]) else {
            panic!("a valid reading");
        };
        assert_eq!(observed_at, now() - TimeDelta::minutes(30));
        assert_eq!(value.ugm3(), 12.5);
        assert_eq!(
            read_form(&[("action", "pm25")]).err(),
            Some("log-error-pm25-missing")
        );
    }

    #[test]
    fn answers_name_the_choice_and_the_event() {
        assert_eq!(parse_choice("new"), Some(EventChoice::New));
        assert_eq!(
            parse_choice("join-12"),
            Some(EventChoice::Join(EventId(12)))
        );
        assert_eq!(
            parse_choice("continue-3"),
            Some(EventChoice::Continue(EventId(3)))
        );
        for bad in ["", "join-", "join-x", "continue", "skip-1"] {
            assert_eq!(parse_choice(bad), None, "{bad}");
        }
    }
}
```

In `crates/web/src/routes/mod.rs`:

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,12 +1,47 @@
 //! One module per feature.
 
+pub(crate) mod entries;
 pub(crate) mod guide;
 pub(crate) mod lang;
 pub(crate) mod log;
 pub(crate) mod login;
+pub(crate) mod notices;
+
+use smoke_domain::DomainError;
+use smoke_domain::logbook::LogError;
 
 use crate::error::AppError;
 
 pub(crate) async fn not_found() -> AppError {
     AppError::NotFound
 }
+
+/// What to tell the user when a use case refuses: the translation key of a message shown with
+/// the form, or an error page.
+pub(crate) fn refusal(error: LogError) -> Result<&'static str, AppError> {
+    match error {
+        LogError::Forbidden => Err(AppError::NotAllowed),
+        LogError::NotFound => Err(AppError::NotFound),
+        LogError::Stale => Ok("refused-stale"),
+        LogError::AnotherEventOpen => Ok("refused-another-open"),
+        LogError::OutsideEvent => Ok("refused-outside-event"),
+        LogError::Invalid(DomainError::ObservedInFuture) => Ok("refused-future"),
+        LogError::Invalid(DomainError::CloseBeforeLastReport) => {
+            Ok("refused-close-before-last-report")
+        }
+        LogError::Invalid(DomainError::CloseInFuture) => Ok("refused-close-in-future"),
+        LogError::Invalid(other) => Err(AppError::internal(other)),
+        LogError::Store(error) => Err(error.into()),
+    }
+}
+
+/// Every key [`refusal`] can return, for the translation tests.
+#[cfg(test)]
+pub(crate) const REFUSALS: [&str; 6] = [
+    "refused-stale",
+    "refused-another-open",
+    "refused-outside-event",
+    "refused-future",
+    "refused-close-before-last-report",
+    "refused-close-in-future",
+];
```

In `crates/web/src/time.rs`, the tests:

```diff
--- a/crates/web/src/time.rs
+++ b/crates/web/src/time.rs
@@ -69,4 +94,35 @@ mod tests {
             "midnight in Rome"
         );
     }
+
+    #[test]
+    fn typed_times_are_read_as_local_time() {
+        assert_eq!(
+            parse_input("2026-01-15T19:05", Rome),
+            Ok(t("2026-01-15T18:05:00Z"))
+        );
+        assert_eq!(
+            parse_input(" 2026-07-01T19:05:30 ", Rome),
+            Ok(t("2026-07-01T17:05:30Z"))
+        );
+        for bad in ["", "now", "2026-13-01T10:00", "2026-01-15 19:05"] {
+            assert_eq!(
+                parse_input(bad, Rome),
+                Err(TimeInputError::Malformed),
+                "{bad}"
+            );
+        }
+    }
+
+    #[test]
+    fn dst_gaps_are_refused_and_repeats_take_the_first() {
+        assert_eq!(
+            parse_input("2026-03-29T02:30", Rome),
+            Err(TimeInputError::Skipped)
+        );
+        assert_eq!(
+            parse_input("2026-10-25T02:30", Rome),
+            Ok(t("2026-10-25T00:30:00Z"))
+        );
+    }
 }
```

In `crates/web/tests/common/mod.rs`:

```diff
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -19,6 +19,7 @@ use smoke_domain::records::{NewUser, User};
 use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
 use smoke_domain::schedule::DailySchedule;
 use smoke_domain::starter::seed_starter_lists;
+use smoke_domain::sweep::auto_close;
 use smoke_domain::types::{Intensity, RoomId};
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
@@ -137,6 +138,13 @@ impl TestApp {
         logged
     }
 
+    /// Runs the auto-close sweep at the fake clock's time, as the server's timer would.
+    pub async fn sweep(&self) {
+        auto_close(&self.store, &schedule(), self.clock.now())
+            .await
+            .unwrap();
+    }
+
     pub async fn send(&self, request: Request<Body>) -> Response<Body> {
         self.router.clone().oneshot(request).await.unwrap()
     }
```

Create `crates/web/tests/logging.rs`:

```rust
//! Saving the Log form and the event questions (spec §7.1–§7.6).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{EventRepo, ReadingRepo, ReportRepo};
use smoke_domain::types::{CloseReason, EventId, Locale, Role};

/// The value of the hidden `asked` field on a question page.
fn asked(html: &str) -> String {
    let start = html.find(r#"name="asked" value=""#).expect("a question") + 20;
    html[start..start + html[start..].find('"').unwrap()].to_owned()
}

#[tokio::test]
async fn logging_with_nothing_open_saves_at_once_and_says_so() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let cookie = app.sign_in("daniele").await;

    let body = format!(
        "intensity=6&room={}&note=from+the+balcony&action=smell",
        bedroom.0
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/?saved=report"));

    let open = app.store.open_event().await.unwrap().unwrap();
    let reports = app.store.reports_for_event(open.id).await.unwrap();
    assert_eq!(reports[0].values.rooms(), [bedroom]);
    assert_eq!(reports[0].values.note(), Some("from the balcony"));
    assert_eq!(reports[0].observed_at, t("2026-01-15T20:00:00Z"));
    let html = text(app.send(get_with("/?saved=report", &cookie)).await).await;
    assert!(html.contains("Report saved."), "{html}");
}

#[tokio::test]
async fn with_an_event_open_the_same_smell_question_comes_first() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(&me, "2026-01-15T18:05:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;

    let question = app
        .send(post_form("/log", "intensity=7&action=smell", Some(&cookie)))
        .await;
    assert_eq!(question.status(), StatusCode::OK);
    let html = text(question).await;
    assert!(html.contains("<h1>Same smell still going?</h1>"), "{html}");
    assert!(html.contains("(started 19:05)"), "{html}");
    assert!(html.contains(r#"<input type="hidden" name="intensity" value="7">"#));
    let id = first.report.event_id;
    assert!(html.contains(&format!(r#"value="join-{id}">Yes, update</button>"#)));
    assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));

    let body = format!(
        "intensity=7&action=smell&asked={}&choice=join-{id}",
        asked(&html)
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    assert_eq!(app.store.reports_for_event(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn answering_no_closes_the_open_event_at_its_last_report() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app
        .log(&me, "2026-01-15T18:05:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let body = format!(
        "intensity=3&action=smell&asked=same-smell-{}&choice=new",
        first.report.event_id
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let old = app
        .store
        .event(first.report.event_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        old.closed.map(|c| (c.at, c.reason)),
        Some((t("2026-01-15T18:05:00Z"), CloseReason::NewEvent))
    );
    assert_ne!(
        app.store.open_event().await.unwrap().unwrap().id,
        first.report.event_id
    );
}

#[tokio::test]
async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let first = app
        .log(&me, "2026-01-15T18:05:00Z", 9, &[kitchen], EventChoice::New)
        .await;
    let closed_earlier = app
        .log(&me, "2026-01-15T10:00:00Z", 8, &[kitchen], EventChoice::New)
        .await;
    let cookie = app.sign_in("guest").await;

    let html = text(
        app.send(post_form("/log", "intensity=2&action=smell", Some(&cookie)))
            .await,
    )
    .await;
    assert!(
        html.contains("A smell event is ongoing (started 19:05). Is this the same smell?"),
        "{html}"
    );
    for private in ["daniele", "Kitchen", ">9<", "ended"] {
        assert!(!html.contains(private), "{private} leaked: {html}");
    }

    let past = text(
        app.send(post_form(
            "/log",
            "intensity=2&action=smell&observed_at=2026-01-15T11:30",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(past.contains("<h1>Which event was this?</h1>"), "{past}");
    let earlier = closed_earlier.report.event_id;
    assert!(
        past.contains(&format!(
            r#"value="join-{earlier}" class="secondary">Started 11:00</button>"#
        )),
        "{past}"
    );
    assert!(
        !past.contains(&format!("join-{}", first.report.event_id)),
        "not started yet"
    );
    for private in ["daniele", "Kitchen", "ended"] {
        assert!(!past.contains(private), "{private} leaked: {past}");
    }
}

#[tokio::test]
async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    let first = app
        .log(&me, "2026-01-15T18:05:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let question = text(
        app.send(post_form("/log", "intensity=7&action=smell", Some(&cookie)))
            .await,
    )
    .await;
    // Meanwhile the partner's phone answers "No, it had stopped".
    let second = app
        .log(&partner, "2026-01-15T19:58:00Z", 5, &[], EventChoice::New)
        .await;

    let body = format!(
        "intensity=7&action=smell&asked={}&choice=join-{}",
        asked(&question),
        first.report.event_id
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert_eq!(
        asked(&html),
        format!("same-smell-{}", second.report.event_id)
    );
    assert!(html.contains("(started 20:58)"), "{html}");
    assert_eq!(
        app.store
            .reports_for_event(first.report.event_id)
            .await
            .unwrap()
            .len(),
        1,
        "nothing joined the closed event"
    );
}

#[tokio::test]
async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let past = app
        .log(&me, "2026-01-15T10:00:00Z", 4, &[], EventChoice::New)
        .await;
    let id = past.report.event_id;
    app.log(&me, "2026-01-15T20:00:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let html = text(
        app.send(post_form(
            "/log",
            "intensity=5&action=smell&observed_at=2026-01-15T12:00",
            Some(&cookie),
        ))
        .await,
    )
    .await;
    assert!(html.contains("Started 11:00, ended 11:00"), "{html}");
    assert!(
        !html.contains("Started 21:00"),
        "not started by then: {html}"
    );
    assert!(html.contains(r#"value="new" class="secondary">New event</button>"#));

    let body = format!(
        "intensity=5&action=smell&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
    );
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T11:00:00Z")));
    assert_eq!(app.store.reports_for_event(id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn after_an_auto_close_the_next_log_offers_a_continuation() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let evening = app
        .log(&me, "2026-01-15T20:00:00Z", 4, &[], EventChoice::New)
        .await;
    let id = evening.report.event_id;
    let cookie = app.sign_in("daniele").await;
    app.at("2026-01-16T06:00:00Z");

    let html = text(
        app.send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
            .await,
    )
    .await;
    assert!(
        html.contains("<h1>The event from 15/01/2026 21:00 was auto-closed.</h1>"),
        "{html}"
    );
    assert_eq!(asked(&html), format!("auto-closed-{id}"));
    let body = format!("intensity=5&action=smell&asked=auto-closed-{id}&choice=continue-{id}");
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(location(&response), Some("/?saved=report"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert!(event.is_open());
    assert!(event.auto_close_notice_seen_at.is_some());
    assert!(app.store.unseen_auto_closed().await.unwrap().is_empty());
}

#[tokio::test]
async fn while_another_event_is_open_the_notice_is_a_dismissable_banner() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
    app.log(&me, "2026-01-15T20:00:00Z", 4, &[], EventChoice::New)
        .await;
    app.at("2026-01-16T06:00:00Z");
    app.sweep().await;
    app.log(&partner, "2026-01-16T06:00:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains("<span>The event from 15/01/2026 21:00 was auto-closed.</span>"),
        "{html}"
    );
    let partners = text(app.send(get_with("/", &app.sign_in("partner").await)).await).await;
    assert!(
        !partners.contains("auto-closed"),
        "only the owner: {partners}"
    );

    let response = app
        .send(post_form("/notices/dismiss", "", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/"));
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!html.contains("auto-closed"), "{html}");
}

#[tokio::test]
async fn pm25_alone_needs_no_intensity_and_joins_the_open_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 4, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form("/log", "pm25=35.5&action=pm25", Some(&cookie)))
        .await;
    assert_eq!(location(&response), Some("/?saved=pm25"));
    let readings = app
        .store
        .readings_for_event(open.report.event_id)
        .await
        .unwrap();
    assert_eq!(readings.len(), 1);
    assert_eq!(readings[0].value.ugm3(), 35.5);

    let with_smell = format!(
        "intensity=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
        open.report.event_id
    );
    app.send(post_form("/log", &with_smell, Some(&cookie)))
        .await;
    assert_eq!(
        app.store
            .readings_for_event(open.report.event_id)
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn invalid_input_shows_the_form_again_with_what_was_typed() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let cookie = app.sign_in("daniele").await;
    let body = format!("room={}&note=kept+as+typed&action=smell", bedroom.0);
    let response = app.send(post_form("/log", &body, Some(&cookie))).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(response).await;
    assert!(html.contains("Pick an intensity from 1 to 10."), "{html}");
    assert!(html.contains(">kept as typed</textarea>"), "{html}");
    assert!(
        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "{html}"
    );
    let missing = app
        .send(post_form("/log", "action=pm25", Some(&cookie)))
        .await;
    assert!(
        text(missing)
            .await
            .contains("Enter a PM2.5 value to log it on its own.")
    );
    assert_eq!(app.store.open_event().await.unwrap(), None);
}

#[tokio::test]
async fn viewers_cannot_log() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    let response = app
        .send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(text(response).await.contains("<h1>Not allowed</h1>"));
    assert_eq!(app.store.event(EventId(1)).await.unwrap(), None);
}
```

In `crates/web/tests/security.rs`:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -147,7 +147,13 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
     let app = TestApp::new().await;
     app.add_user("daniele", Role::Resident, Locale::En).await;
     let cookie = app.sign_in("daniele").await;
+    app.send(post_form("/log", "intensity=4&action=smell", Some(&cookie)))
+        .await;
+    let question = app
+        .send(post_form("/log", "intensity=5&action=smell", Some(&cookie)))
+        .await;
     let pages = [
+        text(question).await,
         text(app.send(get("/login")).await).await,
         text(app.send(get_with("/", &cookie)).await).await,
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile error — `forms`, `routes::entries`, `routes::notices`, `refusal` and `AppError::NotAllowed` do not exist yet.

- [ ] **Step 3: Implement**

Enable axum's `query` feature (for `?saved=` and Task 10's `?month=`).

In `Cargo.toml`:

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -14,7 +14,7 @@ anyhow = "1.0.104"
 argon2 = "0.6.0"
 askama = { version = "0.16.1", default-features = false, features = ["derive", "std"] }
 async-trait = "0.1.92"
-axum = { version = "0.8.9", default-features = false, features = ["form", "http1", "tokio", "tracing"] }
+axum = { version = "0.8.9", default-features = false, features = ["form", "http1", "query", "tokio", "tracing"] }
 chrono = { version = "0.4", default-features = false, features = ["clock", "std", "serde"] }
 chrono-tz = "0.10"
 clap = { version = "4.6.7", features = ["derive"] }
```

In `crates/web/locales/en/main.ftl`:

```diff
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -31,6 +31,36 @@ log-update = Update: situation changed
 log-pm25-only = Log PM2.5 only
 log-read-only = You can look at the data but not log.
 
+log-saved-report = Report saved.
+log-saved-pm25 = PM2.5 reading saved.
+log-error-intensity = Pick an intensity from 1 to 10.
+log-error-pm25 = PM2.5 must be a number from 0 to 1000.
+log-error-pm25-missing = Enter a PM2.5 value to log it on its own.
+log-error-time = Enter a valid date and time.
+log-error-time-skipped = That time does not exist: the clocks moved forward.
+log-error-note = The note can be at most 2000 characters.
+log-error-form = Something in the form is not valid. Check it and try again.
+notice-dismiss = Dismiss
+
+prompt-same-smell = Same smell still going?
+prompt-ongoing = A smell event is ongoing (started { $time }). Is this the same smell?
+prompt-yes-update = Yes, update
+prompt-no-new-event = No, it had stopped: new event
+prompt-which-event = Which event was this?
+prompt-started-at = Started { $time }
+prompt-ended-at = ended { $time }
+prompt-new-event = New event
+prompt-auto-closed = The event from { $when } was auto-closed.
+prompt-continue = It was still going: continuation
+prompt-cancel = Cancel
+
+refused-stale = The events changed in the meantime. Check and try again.
+refused-another-open = Another event is open. Close it first.
+refused-outside-event = That time is outside the event: before it started, or on another day.
+refused-future = The time cannot be in the future.
+refused-close-before-last-report = The end cannot be before the last report.
+refused-close-in-future = The end cannot be in the future.
+
 summary-this-month = This month:
 summary-events = { $count ->
     [one] 1 event
@@ -76,6 +106,8 @@ error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
 error-forbidden = This request did not come from this app, so it was blocked. Go back and try again.
+error-not-allowed-title = Not allowed
+error-not-allowed = Your account cannot do this.
 error-internal-title = Something went wrong
 error-internal = Something went wrong (ref: { $ref }).
 error-back-home = Back to the start
```

In `crates/web/locales/it/main.ftl`:

```diff
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -31,6 +31,36 @@ log-update = Aggiorna: la situazione è cambiata
 log-pm25-only = Registra solo PM2.5
 log-read-only = Puoi consultare i dati ma non registrare.
 
+log-saved-report = Segnalazione salvata.
+log-saved-pm25 = Lettura PM2.5 salvata.
+log-error-intensity = Scegli un'intensità da 1 a 10.
+log-error-pm25 = Il PM2.5 deve essere un numero da 0 a 1000.
+log-error-pm25-missing = Inserisci un valore di PM2.5 per registrarlo da solo.
+log-error-time = Inserisci una data e un'ora valide.
+log-error-time-skipped = Quell'ora non esiste: è scattata l'ora legale.
+log-error-note = La nota può avere al massimo 2000 caratteri.
+log-error-form = Qualcosa nel modulo non è valido. Controlla e riprova.
+notice-dismiss = Ignora
+
+prompt-same-smell = È ancora lo stesso odore?
+prompt-ongoing = C'è un evento in corso (iniziato alle { $time }). È lo stesso odore?
+prompt-yes-update = Sì, aggiorna
+prompt-no-new-event = No, era sparito: nuovo evento
+prompt-which-event = Di quale evento si trattava?
+prompt-started-at = Iniziato alle { $time }
+prompt-ended-at = finito alle { $time }
+prompt-new-event = Nuovo evento
+prompt-auto-closed = L'evento del { $when } è stato chiuso automaticamente.
+prompt-continue = Continuava ancora: è lo stesso evento
+prompt-cancel = Annulla
+
+refused-stale = Nel frattempo gli eventi sono cambiati. Controlla e riprova.
+refused-another-open = C'è un altro evento aperto. Chiudilo prima.
+refused-outside-event = Quell'ora è fuori dall'evento: prima che iniziasse o in un altro giorno.
+refused-future = L'ora non può essere nel futuro.
+refused-close-before-last-report = La fine non può essere prima dell'ultima segnalazione.
+refused-close-in-future = La fine non può essere nel futuro.
+
 summary-this-month = Questo mese:
 summary-events = { $count ->
     [one] 1 evento
@@ -76,6 +106,8 @@ error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
 error-forbidden = Questa richiesta non proviene da questa app, quindi è stata bloccata. Torna indietro e riprova.
+error-not-allowed-title = Non consentito
+error-not-allowed = Il tuo account non può farlo.
 error-internal-title = Qualcosa è andato storto
 error-internal = Qualcosa è andato storto (rif: { $ref }).
 error-back-home = Torna all'inizio
```

In `crates/web/locales/tr/main.ftl`:

```diff
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -31,6 +31,36 @@ log-update = Güncelle: durum değişti
 log-pm25-only = Yalnızca PM2.5 kaydet
 log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.
 
+log-saved-report = Bildirim kaydedildi.
+log-saved-pm25 = PM2.5 ölçümü kaydedildi.
+log-error-intensity = 1 ile 10 arasında bir yoğunluk seç.
+log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
+log-error-pm25-missing = Tek başına kaydetmek için bir PM2.5 değeri gir.
+log-error-time = Geçerli bir tarih ve saat gir.
+log-error-time-skipped = Bu saat yok: saatler ileri alındı.
+log-error-note = Not en fazla 2000 karakter olabilir.
+log-error-form = Formda geçersiz bir şey var. Kontrol edip tekrar dene.
+notice-dismiss = Kapat
+
+prompt-same-smell = Aynı koku hâlâ sürüyor mu?
+prompt-ongoing = Devam eden bir koku olayı var ({ $time } saatinde başladı). Bu aynı koku mu?
+prompt-yes-update = Evet, güncelle
+prompt-no-new-event = Hayır, kesilmişti: yeni olay
+prompt-which-event = Bu hangi olaydı?
+prompt-started-at = { $time } saatinde başladı
+prompt-ended-at = { $time } saatinde bitti
+prompt-new-event = Yeni olay
+prompt-auto-closed = { $when } tarihli olay otomatik olarak kapatıldı.
+prompt-continue = Hâlâ sürüyordu: devamı
+prompt-cancel = Vazgeç
+
+refused-stale = Bu arada olaylar değişti. Kontrol edip tekrar dene.
+refused-another-open = Açık başka bir olay var. Önce onu kapat.
+refused-outside-event = Bu saat olayın dışında: başlamadan önce ya da başka bir günde.
+refused-future = Saat gelecekte olamaz.
+refused-close-before-last-report = Bitiş, son bildirimden önce olamaz.
+refused-close-in-future = Bitiş gelecekte olamaz.
+
 summary-this-month = Bu ay:
 summary-events = { $count ->
     [one] 1 olay
@@ -76,6 +106,8 @@ error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
 error-forbidden = Bu istek bu uygulamadan gelmediği için engellendi. Geri dönüp tekrar deneyin.
+error-not-allowed-title = İzin yok
+error-not-allowed = Hesabın bunu yapamaz.
 error-internal-title = Bir şeyler ters gitti
 error-internal = Bir şeyler ters gitti (ref: { $ref }).
 error-back-home = Başa dön
```

In `crates/web/src/error.rs`:

```diff
--- a/crates/web/src/error.rs
+++ b/crates/web/src/error.rs
@@ -19,7 +19,10 @@ use crate::request_id;
 #[derive(Debug)]
 pub(crate) enum AppError {
     NotFound,
+    /// A cross-site request (the Origin check).
     Forbidden,
+    /// The signed-in user's role does not allow this (spec §4).
+    NotAllowed,
     /// Anything unexpected. Logged with the request id; the user sees only the id.
     Internal(Box<dyn StdError + Send + Sync>),
 }
@@ -32,10 +35,20 @@ impl AppError {
     fn status(&self) -> StatusCode {
         match self {
             Self::NotFound => StatusCode::NOT_FOUND,
-            Self::Forbidden => StatusCode::FORBIDDEN,
+            Self::Forbidden | Self::NotAllowed => StatusCode::FORBIDDEN,
             Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
         }
     }
+
+    /// The title and message keys of the page this error becomes.
+    fn keys(&self) -> (&'static str, &'static str) {
+        match self {
+            Self::NotFound => ("error-not-found-title", "error-not-found"),
+            Self::Forbidden => ("error-forbidden-title", "error-forbidden"),
+            Self::NotAllowed => ("error-not-allowed-title", "error-not-allowed"),
+            Self::Internal(_) => ("error-internal-title", "error-internal"),
+        }
+    }
 }
 
 impl From<StoreError> for AppError {
@@ -64,7 +77,10 @@ impl From<tokio::task::JoinError> for AppError {
 
 /// Marks a response whose body [`render_errors`] should replace with a translated page.
 #[derive(Debug, Clone, Copy)]
-struct ErrorPage(StatusCode);
+struct ErrorPage {
+    title_key: &'static str,
+    message_key: &'static str,
+}
 
 impl IntoResponse for AppError {
     fn into_response(self) -> Response {
@@ -72,8 +88,12 @@ impl IntoResponse for AppError {
         if let Self::Internal(error) = &self {
             tracing::error!(error = %chain(error.as_ref()), "request failed");
         }
+        let (title_key, message_key) = self.keys();
         let mut response = status.into_response();
-        response.extensions_mut().insert(ErrorPage(status));
+        response.extensions_mut().insert(ErrorPage {
+            title_key,
+            message_key,
+        });
         response
     }
 }
@@ -121,14 +141,14 @@ pub(crate) async fn render_errors(request: Request, next: Next) -> Response {
     ));
     let request_id = request_id::of(request.headers()).to_owned();
     let response = next.run(request).await;
-    let Some(ErrorPage(status)) = response.extensions().get::<ErrorPage>().copied() else {
+    let Some(ErrorPage {
+        title_key,
+        message_key,
+    }) = response.extensions().get::<ErrorPage>().copied()
+    else {
         return response;
     };
-    let (title_key, message_key) = match status {
-        StatusCode::NOT_FOUND => ("error-not-found-title", "error-not-found"),
-        StatusCode::FORBIDDEN => ("error-forbidden-title", "error-forbidden"),
-        _ => ("error-internal-title", "error-internal"),
-    };
+    let status = response.status();
     let page = ErrorTemplate {
         tr,
         signed_in: None,
@@ -160,11 +180,13 @@ pub(crate) fn page(status: StatusCode, template: &impl Template) -> Result<Respo
 
 /// Every message key the error pages use, for the translation tests.
 #[cfg(test)]
-pub(crate) const KEYS: [&str; 6] = [
+pub(crate) const KEYS: [&str; 8] = [
     "error-not-found-title",
     "error-not-found",
     "error-forbidden-title",
     "error-forbidden",
+    "error-not-allowed-title",
+    "error-not-allowed",
     "error-internal-title",
     "error-internal",
 ];
```

In `crates/web/src/forms.rs`, above the test module:

```rust
//! Urlencoded forms whose field names repeat (`room=1&room=3`): `serde_urlencoded` cannot put
//! those in a struct, so handlers read the pairs instead.

use axum::Form;
use axum::extract::rejection::FormRejection;
use axum::extract::{FromRequest, Request};

/// A posted form as its `(name, value)` pairs, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Pairs(Vec<(String, String)>);

impl Pairs {
    #[cfg(test)]
    pub(crate) fn of(pairs: &[(&str, &str)]) -> Self {
        Self(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect(),
        )
    }

    /// The first value of `name`, trimmed; `None` if absent or blank.
    pub(crate) fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
    }

    /// Every value of `name` parsed as an id; `None` if one is not a number.
    pub(crate) fn ids(&self, name: &str) -> Option<Vec<i64>> {
        self.0
            .iter()
            .filter(|(key, _)| key == name)
            .map(|(_, value)| value.trim().parse().ok())
            .collect()
    }

    /// All pairs except those named in `skip`: what a follow-up form re-posts as hidden fields.
    pub(crate) fn without(&self, skip: &[&str]) -> Vec<(String, String)> {
        self.0
            .iter()
            .filter(|(key, _)| !skip.contains(&key.as_str()))
            .cloned()
            .collect()
    }
}

impl<S: Send + Sync> FromRequest<S> for Pairs {
    type Rejection = FormRejection;

    async fn from_request(request: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Form(pairs) = Form::<Vec<(String, String)>>::from_request(request, state).await?;
        Ok(Self(pairs))
    }
}
```

In `crates/web/src/i18n.rs`:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -31,7 +31,7 @@ fn language(locale: Locale) -> &'static LanguageIdentifier {
     }
 }
 
-/// Translates for one language. Templates call `tr.t("key")`; a key missing from a language
+/// Translates for one language. Templates call `tr.t("app-name")`; a key missing from a language
 /// falls back to English.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub(crate) struct Tr(pub Locale);
```

In `crates/web/src/routes/entries.rs`, above the test module:

```rust
//! Saving the Log form (spec §7): read it, ask the event question if there is one, then log
//! everything in one transaction. The question travels with the answer (`asked`), so an answer
//! to a question that no longer applies — another phone opened or closed an event meanwhile —
//! gets the current question instead of being applied to the wrong event.

use askama::Template;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use smoke_domain::events::{LogPrompt, can_join_backdated, classify_observed, log_prompt};
use smoke_domain::logbook::{EventChoice, LogError, SmellEntry, log_reading, log_smell};
use smoke_domain::permissions::{Action, can};
use smoke_domain::records::{Event, ReportValues, Room, Symptom, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::{EventId, Intensity, Pm25, Role, RoomId, SymptomId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::log::{Draft, log_page};
use crate::routes::{notices, refusal};
use crate::state::AppState;
use crate::time::{self, TimeInputError};

/// The translation keys [`read`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const FORM_ERRORS: [&str; 7] = [
    "log-error-time",
    "log-error-time-skipped",
    "log-error-pm25",
    "log-error-pm25-missing",
    "log-error-intensity",
    "log-error-form",
    "log-error-note",
];

/// What the form asks to log.
enum Entry {
    Smell(SmellEntry),
    Pm25Only {
        observed_at: DateTime<Utc>,
        value: Pm25,
    },
}

/// Reads and validates the form. `Err` is the translation key of what to fix. Only rooms and
/// symptoms the form offers (not hidden) are accepted.
fn read(
    form: &Pairs,
    rooms: &[Room],
    symptoms: &[Symptom],
    tz: Tz,
    now: DateTime<Utc>,
) -> Result<Entry, &'static str> {
    let observed_at = match form.get("observed_at") {
        None => now,
        Some(value) => time::parse_input(value, tz).map_err(|error| match error {
            TimeInputError::Malformed => "log-error-time",
            TimeInputError::Skipped => "log-error-time-skipped",
        })?,
    };
    if classify_observed(observed_at, now).is_err() {
        return Err("refused-future");
    }
    let pm25 = form
        .get("pm25")
        .map(|value| {
            value
                .replace(',', ".")
                .parse()
                .ok()
                .and_then(|value| Pm25::new(value).ok())
                .ok_or("log-error-pm25")
        })
        .transpose()?;
    if form.get("action") == Some("pm25") {
        let value = pm25.ok_or("log-error-pm25-missing")?;
        return Ok(Entry::Pm25Only { observed_at, value });
    }
    let intensity = form
        .get("intensity")
        .and_then(|value| value.parse().ok())
        .and_then(|value| Intensity::new(value).ok())
        .ok_or("log-error-intensity")?;
    let offered_rooms: Vec<RoomId> = rooms
        .iter()
        .filter(|r| r.hidden_at.is_none())
        .map(|r| r.id)
        .collect();
    let offered_symptoms: Vec<SymptomId> = symptoms
        .iter()
        .filter(|s| s.hidden_at.is_none())
        .map(|s| s.id)
        .collect();
    let room_ids: Vec<RoomId> = form
        .ids("room")
        .ok_or("log-error-form")?
        .into_iter()
        .map(RoomId)
        .collect();
    let symptom_ids: Vec<SymptomId> = form
        .ids("symptom")
        .ok_or("log-error-form")?
        .into_iter()
        .map(SymptomId)
        .collect();
    if !room_ids.iter().all(|id| offered_rooms.contains(id))
        || !symptom_ids.iter().all(|id| offered_symptoms.contains(id))
    {
        return Err("log-error-form");
    }
    let values = ReportValues::new(intensity, room_ids, symptom_ids, form.get("note"))
        .map_err(|_| "log-error-note")?;
    Ok(Entry::Smell(SmellEntry {
        observed_at,
        values,
        pm25,
    }))
}

/// The answer to a prompt, as the buttons post it: `new`, `join-<id>` or `continue-<id>`.
fn parse_choice(value: &str) -> Option<EventChoice> {
    let id = |rest: &str| rest.parse().ok().map(EventId);
    if value == "new" {
        Some(EventChoice::New)
    } else if let Some(rest) = value.strip_prefix("join-") {
        id(rest).map(EventChoice::Join)
    } else {
        value
            .strip_prefix("continue-")
            .and_then(id)
            .map(EventChoice::Continue)
    }
}

/// Names a question, so an answer can be checked against the question that applies now.
fn question_key(prompt: &LogPrompt) -> Option<String> {
    match prompt {
        LogPrompt::Direct => None,
        LogPrompt::SameSmell(open) => Some(format!("same-smell-{}", open.id)),
        LogPrompt::WhichEvent => Some("which-event".to_owned()),
        LogPrompt::AutoClosed(closed) => Some(format!("auto-closed-{}", closed.id)),
    }
}

/// The question that applies now, and the auto-close notices pending for the user.
async fn current_prompt<S: Store>(
    state: &AppState<S>,
    user: &User,
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(LogPrompt, Vec<Event>), AppError> {
    let observed = classify_observed(observed_at, now).map_err(AppError::internal)?;
    let open = state.store.open_event().await?;
    let notices = notices::pending(state, user, now).await?;
    let prompt = log_prompt(
        observed,
        open.as_ref().map(Event::summary),
        notices.last().map(Event::summary),
    );
    Ok((prompt, notices))
}

pub(crate) async fn submit<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = state.config.schedule;
    let actor = user.actor();
    if !can(&actor, Action::LogEntry, now) {
        return Err(AppError::NotAllowed);
    }
    auto_close(&state.store, &schedule, now).await?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let entry = match read(&form, &rooms, &symptoms, schedule.tz(), now) {
        Ok(entry) => entry,
        Err(key) => return back_to_form(&state, &user, tr, now, &form, key).await,
    };
    let entry = match entry {
        Entry::Pm25Only { observed_at, value } => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            if let Err(error) = log_reading(&mut tx, &actor, observed_at, value, now).await {
                drop(tx);
                return back_to_form(&state, &user, tr, now, &form, refusal(error)?).await;
            }
            tx.commit().await?;
            return Ok(Redirect::to("/?saved=pm25").into_response());
        }
        Entry::Smell(entry) => entry,
    };

    let (prompt, notices) = current_prompt(&state, &user, entry.observed_at, now).await?;
    let asked = question_key(&prompt);
    let answer = form.get("choice").and_then(parse_choice);
    let choice = match (&asked, form.get("asked"), answer) {
        (None, _, _) => EventChoice::New,
        (Some(asked), Some(answered), Some(choice)) if asked == answered => choice,
        (Some(_), _, _) => {
            return ask(&state, &user, tr, now, &form, prompt, entry.observed_at).await;
        }
    };
    let observed_at = entry.observed_at;
    let mut tx = state.store.begin(Some(user.id)).await?;
    match log_smell(&mut tx, &actor, entry, choice, &schedule, now).await {
        Ok(_) => {}
        Err(LogError::Stale) => {
            drop(tx);
            let (fresh, _) = current_prompt(&state, &user, observed_at, now).await?;
            if fresh == LogPrompt::Direct {
                return back_to_form(&state, &user, tr, now, &form, "refused-stale").await;
            }
            return ask(&state, &user, tr, now, &form, fresh, observed_at).await;
        }
        Err(error) => {
            drop(tx);
            return back_to_form(&state, &user, tr, now, &form, refusal(error)?).await;
        }
    }
    if matches!(prompt, LogPrompt::AutoClosed(_)) {
        notices::mark_seen(&mut tx, &notices).await?;
    }
    tx.commit().await?;
    Ok(Redirect::to("/?saved=report").into_response())
}

/// The Log screen again, with what was typed and what to fix.
async fn back_to_form<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    error: &'static str,
) -> Result<Response, AppError> {
    let draft = Draft::from_form(form);
    let log_page = log_page(state, user, tr, now, Some(draft), Some(error), None).await?;
    page(StatusCode::UNPROCESSABLE_ENTITY, &log_page)
}

struct Answer {
    choice: String,
    label: String,
    primary: bool,
}

#[derive(Template)]
#[template(path = "prompt.html")]
struct PromptPage {
    tr: Tr,
    signed_in: Option<String>,
    title: String,
    detail: Option<String>,
    /// The form as posted, re-sent with the answer.
    hidden: Vec<(String, String)>,
    asked: String,
    answers: Vec<Answer>,
}

/// Asks the event question. Guests see only start times of events (spec §7.2, §7.5).
async fn ask<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    prompt: LogPrompt,
    observed_at: DateTime<Utc>,
) -> Result<Response, AppError> {
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let answer = |choice: String, key: &str, primary: bool| Answer {
        choice,
        label: tr.t(key),
        primary,
    };
    let new_event = |key: &str| answer("new".to_owned(), key, false);
    let (title, detail, answers) = match prompt {
        LogPrompt::Direct => unreachable!("Direct needs no question"),
        LogPrompt::SameSmell(open) => (
            tr.t("prompt-same-smell"),
            Some(tr.t1(
                "prompt-ongoing",
                "time",
                &time::since(tr.0, open.opened_at, now, tz),
            )),
            vec![
                answer(format!("join-{}", open.id), "prompt-yes-update", true),
                new_event("prompt-no-new-event"),
            ],
        ),
        LogPrompt::WhichEvent => {
            let (day_start, day_end) = schedule.day_span(schedule.local_date(observed_at));
            let mut answers: Vec<Answer> = state
                .store
                .events_between(day_start, day_end)
                .await?
                .into_iter()
                .filter(|event| can_join_backdated(event, observed_at, schedule))
                .map(|event| {
                    let mut label = tr.t1(
                        "prompt-started-at",
                        "time",
                        &time::clock(event.opened_at, tz),
                    );
                    if user.role != Role::Guest
                        && let Some(closed) = event.closed
                    {
                        label.push_str(", ");
                        label.push_str(&tr.t1(
                            "prompt-ended-at",
                            "time",
                            &time::clock(closed.at, tz),
                        ));
                    }
                    Answer {
                        choice: format!("join-{}", event.id),
                        label,
                        primary: false,
                    }
                })
                .collect();
            answers.push(new_event("prompt-new-event"));
            (tr.t("prompt-which-event"), None, answers)
        }
        LogPrompt::AutoClosed(closed) => (
            tr.t1(
                "prompt-auto-closed",
                "when",
                &time::date_time(tr.0, closed.opened_at, tz),
            ),
            None,
            vec![
                answer(format!("continue-{}", closed.id), "prompt-continue", true),
                new_event("prompt-new-event"),
            ],
        ),
    };
    page(
        StatusCode::OK,
        &PromptPage {
            tr,
            signed_in: Some(user.username.to_string()),
            title,
            detail,
            hidden: form.without(&["choice", "asked"]),
            asked: question_key(&prompt).unwrap_or_default(),
            answers,
        },
    )
}
```

In `crates/web/src/routes/log.rs`:

```diff
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -1,10 +1,11 @@
 //! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.
 
 use askama::Template;
-use axum::extract::State;
+use axum::extract::{Query, State};
 use axum::http::StatusCode;
 use axum::response::Response;
 use chrono::{DateTime, Utc};
+use serde::Deserialize;
 use smoke_domain::permissions::{Action, can, can_see};
 use smoke_domain::records::User;
 use smoke_domain::repos::Store;
@@ -15,8 +16,10 @@ use smoke_domain::types::{Intensity, RoomArea, RoomId, SymptomId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
+use crate::forms::Pairs;
 use crate::i18n::Tr;
 use crate::routes::guide::{Level, levels};
+use crate::routes::notices;
 use crate::state::AppState;
 use crate::{labels, time};
 
@@ -31,6 +34,31 @@ pub(crate) struct Draft {
     pub(crate) note: String,
 }
 
+impl Draft {
+    /// The form as posted, to show it again after a validation error.
+    pub(crate) fn from_form(form: &Pairs) -> Self {
+        let text = |name| form.get(name).unwrap_or_default().to_owned();
+        Self {
+            intensity: form.get("intensity").and_then(|v| v.parse().ok()),
+            rooms: form
+                .ids("room")
+                .unwrap_or_default()
+                .into_iter()
+                .map(RoomId)
+                .collect(),
+            symptoms: form
+                .ids("symptom")
+                .unwrap_or_default()
+                .into_iter()
+                .map(SymptomId)
+                .collect(),
+            pm25: text("pm25"),
+            observed_at: text("observed_at"),
+            note: text("note"),
+        }
+    }
+}
+
 /// A radio button, chip or checkbox.
 pub(crate) struct Choice {
     value: i64,
@@ -58,20 +86,35 @@ pub(crate) struct LogPage {
     tr: Tr,
     signed_in: Option<String>,
     open_since: Option<String>,
+    /// "The event from … was auto-closed", shown while another event is open (spec §7.4).
+    notice: Option<String>,
+    /// A translation key confirming what was just saved.
+    saved: Option<&'static str>,
     summary: MonthSummary,
     levels: Vec<Level>,
     /// `None` for people who may not log (viewers, expired guests).
     form: Option<FormView>,
 }
 
+#[derive(Deserialize)]
+pub(crate) struct ShowQuery {
+    saved: Option<String>,
+}
+
 pub(crate) async fn show<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
+    Query(query): Query<ShowQuery>,
 ) -> Result<Response, AppError> {
     let now = state.clock.now();
     auto_close(&state.store, &state.config.schedule, now).await?;
-    let log_page = log_page(&state, &user, tr, now, None, None).await?;
+    let saved = match query.saved.as_deref() {
+        Some("report") => Some("log-saved-report"),
+        Some("pm25") => Some("log-saved-pm25"),
+        _ => None,
+    };
+    let log_page = log_page(&state, &user, tr, now, None, None, saved).await?;
     page(StatusCode::OK, &log_page)
 }
 
@@ -84,6 +127,7 @@ pub(crate) async fn log_page<S: Store>(
     now: DateTime<Utc>,
     draft: Option<Draft>,
     error: Option<&'static str>,
+    saved: Option<&'static str>,
 ) -> Result<LogPage, AppError> {
     let schedule = &state.config.schedule;
     let actor = user.actor();
@@ -103,6 +147,16 @@ pub(crate) async fn log_page<S: Store>(
         .find(|r| r.i18n_key.as_deref() == Some(BEDROOM_KEY))
         .map(|r| r.id);
     let summary = summarise(&visible, bedroom, schedule);
+    let notice = match &open {
+        Some(_) => notices::pending(state, user, now)
+            .await?
+            .pop()
+            .map(|closed| {
+                let when = time::date_time(tr.0, closed.opened_at, schedule.tz());
+                tr.t1("prompt-auto-closed", "when", &when)
+            }),
+        None => None,
+    };
 
     let form = if can(&actor, Action::LogEntry, now) {
         let draft = match draft {
@@ -166,6 +220,8 @@ pub(crate) async fn log_page<S: Store>(
         tr,
         signed_in: Some(user.username.to_string()),
         open_since: open.map(|e| time::since(tr.0, e.opened_at, now, schedule.tz())),
+        notice,
+        saved,
         summary,
         levels: levels(tr),
         form,
```

Create `crates/web/src/routes/notices.rs`:

```rust
//! The auto-close notice (spec §7.4): "The event from [date time] was auto-closed."

use axum::extract::State;
use axum::response::Redirect;
use chrono::{DateTime, Utc};
use smoke_domain::events::auto_close_notice_is_for;
use smoke_domain::records::{Event, User};
use smoke_domain::repos::{Commit, EventWrites, Store};

use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::state::AppState;

/// The unseen auto-close notices meant for `user`, oldest first: only the last one is offered,
/// and answering it marks them all seen.
pub(crate) async fn pending<S: Store>(
    state: &AppState<S>,
    user: &User,
    now: DateTime<Utc>,
) -> Result<Vec<Event>, AppError> {
    let viewer = user.actor();
    let mut pending = Vec::new();
    for event in state.store.unseen_auto_closed().await? {
        let owner = if event.owner_id == user.id {
            Some(user.clone())
        } else {
            state.store.user(event.owner_id).await?
        };
        if owner.is_some_and(|owner| auto_close_notice_is_for(&viewer, &owner.actor(), now)) {
            pending.push(event);
        }
    }
    Ok(pending)
}

/// Marks the notices seen, inside the caller's transaction.
pub(crate) async fn mark_seen<T: EventWrites>(
    tx: &mut T,
    notices: &[Event],
) -> Result<(), AppError> {
    for notice in notices {
        tx.mark_auto_close_notice_seen(notice.id).await?;
    }
    Ok(())
}

/// "Dismiss" on the banner shown while another event is open.
pub(crate) async fn dismiss<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
) -> Result<Redirect, AppError> {
    let notices = pending(&state, &user, state.clock.now()).await?;
    if !notices.is_empty() {
        let mut tx = state.store.begin(Some(user.id)).await?;
        mark_seen(&mut tx, &notices).await?;
        tx.commit().await?;
    }
    Ok(Redirect::to("/"))
}
```

In `crates/web/src/time.rs`:

```diff
--- a/crates/web/src/time.rs
+++ b/crates/web/src/time.rs
@@ -1,9 +1,21 @@
 //! Times as people read and type them: in the configured time zone (`APP_TIMEZONE`), never the
 //! browser's, so every phone shows the same times.
 
-use chrono::{DateTime, NaiveDate, Utc};
+use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
 use chrono_tz::Tz;
 use smoke_domain::types::Locale;
+use thiserror::Error;
+
+/// The `value` format of `<input type="datetime-local">`, to the minute.
+const INPUT_FORMAT: &str = "%Y-%m-%dT%H:%M";
+
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
+pub(crate) enum TimeInputError {
+    #[error("not a date and time")]
+    Malformed,
+    #[error("that local time does not exist: the clocks skipped it")]
+    Skipped,
+}
 
 /// A calendar date as the language writes it with digits.
 pub(crate) fn date(locale: Locale, date: NaiveDate) -> String {
@@ -34,6 +46,19 @@ pub(crate) fn since(locale: Locale, at: DateTime<Utc>, now: DateTime<Utc>, tz: T
     }
 }
 
+/// Reads a `datetime-local` value (seconds optional) as local time in `tz`. A time skipped by a
+/// DST change is refused; a repeated one means its first occurrence.
+pub(crate) fn parse_input(value: &str, tz: Tz) -> Result<DateTime<Utc>, TimeInputError> {
+    let value = value.trim();
+    let naive = NaiveDateTime::parse_from_str(value, INPUT_FORMAT)
+        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S"))
+        .map_err(|_| TimeInputError::Malformed)?;
+    match tz.from_local_datetime(&naive) {
+        LocalResult::Single(at) | LocalResult::Ambiguous(at, _) => Ok(at.with_timezone(&Utc)),
+        LocalResult::None => Err(TimeInputError::Skipped),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use chrono_tz::Europe::Rome;
```

In `crates/web/templates/log.html`:

```diff
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -2,6 +2,15 @@
 {% block title %}{{ tr.t("log-title") }}{% endblock %}
 {% block content %}
 <h1>{{ tr.t("log-title") }}</h1>
+{% if let Some(saved) = saved %}
+<p class="notice" role="status">{{ tr.t(saved) }}</p>
+{% endif %}
+{% if let Some(notice) = notice %}
+<form method="post" action="/notices/dismiss" class="notice banner">
+  <span>{{ notice }}</span>
+  <button type="submit" class="link">{{ tr.t("notice-dismiss") }}</button>
+</form>
+{% endif %}
 <p class="status">
   {% if let Some(since) = open_since %}{{ tr.t1("event-open-since", "time", since) }}{% else %}{{ tr.t("event-none") }}{% endif %}
 </p>
```

Create `crates/web/templates/prompt.html`:

```html
{% extends "base.html" %}
{% block title %}{{ title }}{% endblock %}
{% block content %}
<h1>{{ title }}</h1>
{% if let Some(detail) = detail %}
<p>{{ detail }}</p>
{% endif %}
<form method="post" action="/log" class="stack">
  {% for (name, value) in hidden %}
  <input type="hidden" name="{{ name }}" value="{{ value }}">
  {% endfor %}
  <input type="hidden" name="asked" value="{{ asked }}">
  {% for answer in answers %}
  <button type="submit" name="choice" value="{{ answer.choice }}"{% if !answer.primary %} class="secondary"{% endif %}>{{ answer.label }}</button>
  {% endfor %}
</form>
<p><a href="/">{{ tr.t("prompt-cancel") }}</a></p>
{% endblock %}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `a_guest_sees_only_the_start_time_of_someone_elses_event`, `a_stale_answer_gets_the_current_question_instead_of_being_applied`, `after_an_auto_close_the_next_log_offers_a_continuation` and `invalid_input_shows_the_form_again_with_what_was_typed`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/error.rs \
  crates/web/src/forms.rs \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/entries.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/notices.rs \
  crates/web/src/time.rs \
  crates/web/templates/log.html \
  crates/web/templates/prompt.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/logging.rs \
  crates/web/tests/security.rs
git commit -m "feat(web): save the Log form behind the same-smell, which-event and auto-close questions

POST /log reads the form as repeated pairs, runs the sweep, and asks the
question log_prompt picks. The question page re-posts the form as hidden
fields plus an 'asked' key naming the question, so an answer to a question
that no longer applies is met with the current one. Guests see only start
times. 'Log PM2.5 only' skips the question; validation errors show the form
again, as typed, with a 422.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Web — the Events list and an event's timeline, close and reopen

`/events?month=YYYY-MM` lists a local month's events grouped by the day they opened, newest first,
with standalone PM2.5 readings (decision #100); every report of the month comes from one
`reports_between` window. `/events/{id}` merges reports and readings by time, flags gaps over
three hours (§7.7), names who logged what for people who see everything, and offers Close (end
time defaulting to now) and Reopen per the permission table. The Log screen gets a *Close event*
button and the header a menu.

**Files:**
- Create: `crates/web/src/routes/events.rs`, `crates/web/templates/event.html`, `crates/web/templates/events.html`, `crates/web/tests/events.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/i18n.rs`, `crates/web/src/labels.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/time.rs`, `crates/web/static/app.css`, `crates/web/templates/base.html`, `crates/web/templates/log.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/security.rs`

**Interfaces:**
- Consumes: `close_event`, `reopen_event` (Task 6), `refusal`, `Pairs` (Task 9),
  `large_gap_indices`, `events_between`, `reports_between`, `readings_between`.
- Produces: `crate::routes::events::{list, show, close, reopen}`; `crate::labels::close_reason`
  and `labels::keys()`; `crate::time::input_value`; routes `GET /events`, `GET /events/{id}`,
  `POST /events/{id}/close` (`closed_at`, optional `return_to=log`),
  `POST /events/{id}/reopen`; test helper `TestApp::log_pm25`.

- [ ] **Step 1: Write the failing tests**

In `crates/web/src/i18n.rs`, the tests:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -126,9 +126,9 @@ mod tests {
     use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS};
 
     use super::*;
-    use crate::error;
     use crate::routes::login::Notice;
     use crate::routes::{self, entries, guide};
+    use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
 
@@ -214,6 +214,7 @@ mod tests {
         used.extend(error::KEYS.map(str::to_owned));
         used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
         used.extend(guide::keys());
+        used.extend(labels::keys().into_iter().map(str::to_owned));
         used.extend(entries::FORM_ERRORS.map(str::to_owned));
         used.extend(routes::REFUSALS.map(str::to_owned));
         let english = message_ids(Locale::En);
```

In `crates/web/src/lib.rs`:

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -46,6 +46,10 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
         .route("/guide/rating", get(routes::guide::rating))
         .route("/log", post(routes::entries::submit::<S>))
         .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
+        .route("/events", get(routes::events::list::<S>))
+        .route("/events/{id}", get(routes::events::show::<S>))
+        .route("/events/{id}/close", post(routes::events::close::<S>))
+        .route("/events/{id}/reopen", post(routes::events::reopen::<S>))
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
```

In `crates/web/src/routes/mod.rs`:

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,6 +1,7 @@
 //! One module per feature.
 
 pub(crate) mod entries;
+pub(crate) mod events;
 pub(crate) mod guide;
 pub(crate) mod lang;
 pub(crate) mod log;
```

In `crates/web/src/time.rs`, the tests:

```diff
--- a/crates/web/src/time.rs
+++ b/crates/web/src/time.rs
@@ -95,6 +100,14 @@ mod tests {
         );
     }
 
+    #[test]
+    fn inputs_are_prefilled_with_local_time_to_the_minute() {
+        assert_eq!(
+            input_value(t("2026-01-15T18:05:59Z"), Rome),
+            "2026-01-15T19:05"
+        );
+    }
+
     #[test]
     fn typed_times_are_read_as_local_time() {
         assert_eq!(
```

In `crates/web/tests/common/mod.rs`:

```diff
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -13,14 +13,14 @@ use axum::http::{HeaderName, Request, Response, StatusCode};
 use chrono::{DateTime, TimeDelta, Utc};
 use http_body_util::BodyExt;
 use smoke_domain::clock::{Clock, FakeClock, SystemClock};
-use smoke_domain::logbook::{EventChoice, Logged, SmellEntry, log_smell};
-use smoke_domain::records::ReportValues;
+use smoke_domain::logbook::{EventChoice, Logged, SmellEntry, log_reading, log_smell};
 use smoke_domain::records::{NewUser, User};
+use smoke_domain::records::{Pm25Reading, ReportValues};
 use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
 use smoke_domain::schedule::DailySchedule;
 use smoke_domain::starter::seed_starter_lists;
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{Intensity, RoomId};
+use smoke_domain::types::{Intensity, Pm25, RoomId};
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
 use smoke_web::config::{BaseUrl, WebConfig};
@@ -138,6 +138,22 @@ impl TestApp {
         logged
     }
 
+    /// Logs a PM2.5 reading on its own, as "Log PM2.5 only" would.
+    pub async fn log_pm25(&self, user: &User, at: &str, value: f64) -> Pm25Reading {
+        let mut tx = self.store.begin(Some(user.id)).await.unwrap();
+        let reading = log_reading(
+            &mut tx,
+            &user.actor(),
+            t(at),
+            Pm25::new(value).unwrap(),
+            self.clock.now(),
+        )
+        .await
+        .unwrap();
+        tx.commit().await.unwrap();
+        reading
+    }
+
     /// Runs the auto-close sweep at the fake clock's time, as the server's timer would.
     pub async fn sweep(&self) {
         auto_close(&self.store, &schedule(), self.clock.now())
```

Create `crates/web/tests/events.rs`:

```rust
//! The Events list and an event's page: timeline, large-gap warning, close and reopen
//! (spec §7.3, §7.7, §9 screen 3).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::EventRepo;
use smoke_domain::types::{CloseReason, Locale, Role};

#[tokio::test]
async fn the_list_groups_a_months_events_by_day_newest_first() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let first = app
        .log(&me, "2026-01-10T18:00:00Z", 3, &[bedroom], EventChoice::New)
        .await;
    let join = EventChoice::Join(first.report.event_id);
    app.log(&me, "2026-01-10T19:00:00Z", 7, &[], join).await;
    let second = app
        .log(&me, "2026-01-12T07:30:00Z", 5, &[], EventChoice::New)
        .await;
    app.log_pm25(&me, "2026-01-11T13:00:00Z", 12.5).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(html.contains("<strong>01/2026</strong>"), "{html}");
    assert!(html.contains(r#"href="/events?month=2025-12""#), "{html}");
    assert!(html.contains(r#"href="/events?month=2026-02""#), "{html}");
    let day12 = html.find("<h2>12/01/2026</h2>").unwrap();
    let day11 = html.find("<h2>11/01/2026</h2>").unwrap();
    let day10 = html.find("<h2>10/01/2026</h2>").unwrap();
    assert!(day12 < day11 && day11 < day10, "newest first: {html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">19:00–20:00</a> · 2 reports · max 7"#,
            first.report.event_id
        )),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">08:30–"#,
            second.report.event_id
        )),
        "{html}"
    );
    assert!(html.contains("14:00 · PM2.5 12.5 µg/m³"), "{html}");

    let empty = text(app.send(get_with("/events?month=2025-11", &cookie)).await).await;
    assert!(empty.contains("No events this month."), "{empty}");
    let default = text(app.send(get_with("/events?month=junk", &cookie)).await).await;
    assert!(default.contains("<strong>01/2026</strong>"), "{default}");
}

#[tokio::test]
async fn guests_list_only_events_they_own_or_reported_in() {
    let app = TestApp::new().await;
    app.at("2026-01-20T12:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let private = app
        .log(&me, "2026-01-10T18:00:00Z", 8, &[], EventChoice::New)
        .await;
    let shared = app
        .log(&me, "2026-01-11T18:00:00Z", 9, &[], EventChoice::New)
        .await;
    let join = EventChoice::Join(shared.report.event_id);
    app.log(&guest, "2026-01-11T18:30:00Z", 4, &[], join).await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
    assert!(
        !html.contains(&format!("/events/{}", private.report.event_id)),
        "{html}"
    );
    assert!(
        html.contains(&format!(
            r#"<a href="/events/{}">19:00–19:30</a> · 1 report · max 4"#,
            shared.report.event_id
        )),
        "only the guest's own report counts: {html}"
    );
}

#[tokio::test]
async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let first = app
        .log(&me, "2026-01-15T09:00:00Z", 3, &[bedroom], EventChoice::New)
        .await;
    let id = first.report.event_id;
    app.log(
        &guest,
        "2026-01-15T10:00:00Z",
        5,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let late = app
        .log(&me, "2026-01-15T14:00:00Z", 6, &[], EventChoice::Join(id))
        .await;
    app.log_pm25(&me, "2026-01-15T10:30:00Z", 30.0).await;
    let cookie = app.sign_in("daniele").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("<h1>Event of 15/01/2026</h1>"), "{html}");
    assert!(html.contains("Started by daniele"), "{html}");
    assert!(
        html.contains(&format!(
            r#"<a href="/reports/{}">intensity 3 · Bedroom</a>"#,
            first.report.id
        )),
        "{html}"
    );
    assert!(html.contains("<span class=\"by\">(guest)</span>"), "{html}");
    let reading = html.find("PM2.5 30 µg/m³").unwrap();
    let gap = html
        .find("More than 3 hours since the previous report")
        .unwrap();
    let after_gap = html
        .find(&format!(r#"<a href="/reports/{}">"#, late.report.id))
        .unwrap();
    assert!(reading < gap && gap < after_gap, "{html}");
    assert_eq!(html.matches("More than 3 hours").count(), 1);
}

#[tokio::test]
async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let kitchen = app.room("room-kitchen").await;
    let shared = app
        .log(&me, "2026-01-15T09:00:00Z", 9, &[kitchen], EventChoice::New)
        .await;
    let id = shared.report.event_id;
    app.log(
        &guest,
        "2026-01-15T09:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let private = app
        .log(&me, "2026-01-15T19:00:00Z", 8, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("intensity 4"), "{html}");
    for leak in ["intensity 9", "Kitchen", "daniele", "Started by"] {
        assert!(!html.contains(leak), "{leak} leaked: {html}");
    }
    let other = app
        .send(get_with(
            &format!("/events/{}", private.report.event_id),
            &cookie,
        ))
        .await;
    assert_eq!(other.status(), StatusCode::NOT_FOUND);
    let missing = app.send(get_with("/events/999", &cookie)).await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn closing_by_hand_needs_an_end_between_the_last_report_and_now() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(
        html.contains(r#"name="closed_at" value="2026-01-15T21:00""#),
        "now is the default: {html}"
    );

    let path = format!("/events/{id}/close");
    let early = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T19:30",
            Some(&cookie),
        ))
        .await;
    assert_eq!(early.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(early)
            .await
            .contains("The end cannot be before the last report.")
    );
    let late = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T23:00",
            Some(&cookie),
        ))
        .await;
    assert!(
        text(late)
            .await
            .contains("The end cannot be in the future.")
    );

    let response = app
        .send(post_form(
            &path,
            "closed_at=2026-01-15T20:40",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/events/{id}")));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(
        event.closed.map(|c| (c.at, c.reason)),
        Some((t("2026-01-15T19:40:00Z"), CloseReason::Manual))
    );
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(html.contains("20:00–20:40 · closed by hand"), "{html}");
    assert!(html.contains(">Reopen</button>"), "{html}");
}

#[tokio::test]
async fn the_log_screen_closes_the_open_event_now() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with("/", &cookie)).await).await;
    assert!(
        html.contains(&format!(
            r#"<form method="post" action="/events/{id}/close">"#
        )),
        "{html}"
    );
    let response = app
        .send(post_form(
            &format!("/events/{id}/close"),
            "return_to=log",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some("/"));
    let event = app.store.event(id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T20:00:00Z")));
}

#[tokio::test]
async fn only_residents_and_owning_guests_can_close_or_reopen() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let open = app
        .log(&me, "2026-01-15T19:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = open.report.event_id;
    app.log(
        &guest,
        "2026-01-15T19:30:00Z",
        4,
        &[],
        EventChoice::Join(id),
    )
    .await;
    let cookie = app.sign_in("guest").await;

    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(!html.contains("Close event"), "{html}");
    let log = text(app.send(get_with("/", &cookie)).await).await;
    assert!(!log.contains("/close"), "{log}");
    let response = app
        .send(post_form(&format!("/events/{id}/close"), "", Some(&cookie)))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(app.store.event(id).await.unwrap().unwrap().is_open());
}

#[tokio::test]
async fn reopening_is_offered_only_while_no_other_event_is_open() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let earlier = app
        .log(&me, "2026-01-15T10:00:00Z", 5, &[], EventChoice::New)
        .await;
    let id = earlier.report.event_id;
    let current = app
        .log(&me, "2026-01-15T20:00:00Z", 5, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;
    let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
    assert!(!html.contains("Reopen"), "{html}");
    let refused = app
        .send(post_form(
            &format!("/events/{id}/reopen"),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(refused.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        text(refused)
            .await
            .contains("Another event is open. Close it first.")
    );

    let close = format!("/events/{}/close", current.report.event_id);
    app.send(post_form(&close, "", Some(&cookie))).await;
    let response = app
        .send(post_form(
            &format!("/events/{id}/reopen"),
            "",
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/events/{id}")));
    assert!(app.store.event(id).await.unwrap().unwrap().is_open());
}
```

In `crates/web/tests/security.rs`:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -154,6 +154,8 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         .await;
     let pages = [
         text(question).await,
+        text(app.send(get_with("/events", &cookie)).await).await,
+        text(app.send(get_with("/events/1", &cookie)).await).await,
         text(app.send(get("/login")).await).await,
         text(app.send(get_with("/", &cookie)).await).await,
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile error — `routes::events`, `labels::close_reason` and `time::input_value` do not exist yet.

- [ ] **Step 3: Implement**

In `crates/web/locales/en/main.ftl`:

```diff
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -102,6 +102,29 @@ intensity-9-desc = Like standing next to someone smoking.
 intensity-10-name = Unbearable
 intensity-10-desc = Like a smoke-filled room. You must leave.
 
+nav-log = Log
+nav-events = Events
+
+events-title = Events
+events-none = No events this month.
+events-previous = ← Previous
+events-next = Next →
+events-in-progress = in progress
+events-max-intensity = max { $level }
+event-title = Event of { $date }
+event-started-by = Started by { $username }
+event-gap = More than 3 hours since the previous report: is this still the same event?
+event-intensity = intensity { $level }
+event-pm25 = PM2.5 { $value } µg/m³
+event-close = Close event
+event-close-at = End time
+event-reopen = Reopen
+
+close-reason-manual = closed by hand
+close-reason-new-event = closed when a new event started
+close-reason-auto = closed automatically overnight
+close-reason-backdated = logged afterwards
+
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
```

In `crates/web/locales/it/main.ftl`:

```diff
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -102,6 +102,29 @@ intensity-9-desc = Come stare accanto a qualcuno che fuma.
 intensity-10-name = Insopportabile
 intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
+nav-log = Registra
+nav-events = Eventi
+
+events-title = Eventi
+events-none = Nessun evento questo mese.
+events-previous = ← Precedente
+events-next = Successivo →
+events-in-progress = in corso
+events-max-intensity = max { $level }
+event-title = Evento del { $date }
+event-started-by = Iniziato da { $username }
+event-gap = Più di 3 ore dalla segnalazione precedente: è ancora lo stesso evento?
+event-intensity = intensità { $level }
+event-pm25 = PM2.5 { $value } µg/m³
+event-close = Chiudi evento
+event-close-at = Ora di fine
+event-reopen = Riapri
+
+close-reason-manual = chiuso a mano
+close-reason-new-event = chiuso all'inizio di un nuovo evento
+close-reason-auto = chiuso automaticamente durante la notte
+close-reason-backdated = registrato in seguito
+
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
```

In `crates/web/locales/tr/main.ftl`:

```diff
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -102,6 +102,29 @@ intensity-9-desc = Sigara içen birinin yanında durmak gibi.
 intensity-10-name = Dayanılmaz
 intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
+nav-log = Kaydet
+nav-events = Olaylar
+
+events-title = Olaylar
+events-none = Bu ay olay yok.
+events-previous = ← Önceki
+events-next = Sonraki →
+events-in-progress = sürüyor
+events-max-intensity = en fazla { $level }
+event-title = { $date } tarihli olay
+event-started-by = Başlatan: { $username }
+event-gap = Önceki bildirimden bu yana 3 saatten fazla geçti: hâlâ aynı olay mı?
+event-intensity = yoğunluk { $level }
+event-pm25 = PM2.5 { $value } µg/m³
+event-close = Olayı kapat
+event-close-at = Bitiş saati
+event-reopen = Yeniden aç
+
+close-reason-manual = elle kapatıldı
+close-reason-new-event = yeni bir olay başlayınca kapatıldı
+close-reason-auto = gece otomatik olarak kapatıldı
+close-reason-backdated = sonradan kaydedildi
+
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
```

In `crates/web/src/labels.rs`:

```diff
--- a/crates/web/src/labels.rs
+++ b/crates/web/src/labels.rs
@@ -1,7 +1,8 @@
 //! Names shown for stored values: starter rooms and symptoms through their translation key,
-//! custom ones as typed.
+//! custom ones as typed, and the translation keys of enum values.
 
 use smoke_domain::records::{Room, Symptom};
+use smoke_domain::types::CloseReason;
 
 use crate::i18n::Tr;
 
@@ -19,6 +20,21 @@ pub(crate) fn symptom(tr: Tr, symptom: &Symptom) -> String {
     }
 }
 
+pub(crate) fn close_reason(reason: CloseReason) -> &'static str {
+    match reason {
+        CloseReason::Manual => "close-reason-manual",
+        CloseReason::NewEvent => "close-reason-new-event",
+        CloseReason::Auto0400 => "close-reason-auto",
+        CloseReason::Backdated => "close-reason-backdated",
+    }
+}
+
+/// Every key this module returns, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    CloseReason::ALL.iter().copied().map(close_reason).collect()
+}
+
 #[cfg(test)]
 mod tests {
     use smoke_domain::types::{ListName, Locale, RoomArea, RoomId, SymptomId};
```

Create `crates/web/src/routes/events.rs`:

```rust
//! The Events screens (spec §9, screen 3): a month of events grouped by day, and one event's
//! timeline of reports and PM2.5 readings with the large-gap warning, Close and Reopen.
//! Guests see only events they own or reported in, and only their own entries.

use std::collections::{BTreeMap, HashMap};

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
use serde::Deserialize;
use smoke_domain::events::{FUTURE_TOLERANCE, large_gap_indices};
use smoke_domain::logbook::{close_event, reopen_event};
use smoke_domain::permissions::{Action, Actor, can, can_see};
use smoke_domain::records::{Event, Pm25Reading, SmellReport, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{EventId, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::labels;
use crate::routes::refusal;
use crate::state::AppState;
use crate::time::{self, TimeInputError};

#[derive(Deserialize)]
pub(crate) struct ListQuery {
    /// `YYYY-MM`; the current month when absent or invalid.
    month: Option<String>,
}

struct EventRow {
    id: i64,
    span: String,
    reports: usize,
    max_intensity: Option<String>,
}

struct ReadingRow {
    at: String,
    value: String,
}

struct Day {
    date: String,
    events: Vec<EventRow>,
    readings: Vec<ReadingRow>,
}

#[derive(Template)]
#[template(path = "events.html")]
struct ListPage {
    tr: Tr,
    signed_in: Option<String>,
    month: String,
    previous: String,
    next: String,
    days: Vec<Day>,
}

/// `start–end`, `start–` while open; local clock times.
fn span(tr: Tr, event: &Event, tz: chrono_tz::Tz) -> String {
    let start = time::clock(event.opened_at, tz);
    match event.closed {
        Some(closed) => format!("{start}–{}", time::clock(closed.at, tz)),
        None => format!("{start}–{}", tr.t("events-in-progress")),
    }
}

/// The day's group, created on first use.
fn day(days: &mut BTreeMap<NaiveDate, Day>, tr: Tr, date: NaiveDate) -> &mut Day {
    days.entry(date).or_insert_with(|| Day {
        date: time::date(tr.0, date),
        events: Vec::new(),
        readings: Vec::new(),
    })
}

/// Whether `actor` may open this event: it is theirs to see, or they reported in it.
fn can_open(actor: &Actor, event: &Event, reports: &[SmellReport], now: DateTime<Utc>) -> bool {
    can_see(actor, event.owner_id, now) || reports.iter().any(|r| can_see(actor, r.user_id, now))
}

pub(crate) async fn list<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Query(query): Query<ListQuery>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let first = query
        .month
        .and_then(|m| NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d").ok())
        .unwrap_or_else(|| schedule.local_date(now));
    let (from, to) = schedule.month_span(first);
    let events = state.store.events_between(from, to).await?;
    // Reports lie within their event's span, so this window holds every report of `events`.
    let window_start = events.iter().map(|e| e.opened_at).min().unwrap_or(from);
    let window_end = events
        .iter()
        .map(|e| e.closed.map_or(now + FUTURE_TOLERANCE, |c| c.at))
        .max()
        .unwrap_or(to)
        .max(to);
    let mut by_event: HashMap<EventId, Vec<SmellReport>> = HashMap::new();
    for report in state
        .store
        .reports_between(window_start, window_end + TimeDelta::microseconds(1))
        .await?
    {
        by_event.entry(report.event_id).or_default().push(report);
    }

    let mut days: BTreeMap<NaiveDate, Day> = BTreeMap::new();
    for event in events.iter().rev() {
        let reports = by_event.remove(&event.id).unwrap_or_default();
        if !can_open(&actor, event, &reports, now) {
            continue;
        }
        let visible: Vec<_> = reports
            .iter()
            .filter(|r| can_see(&actor, r.user_id, now))
            .collect();
        let local = schedule.local_date(event.opened_at);
        if local < schedule.local_date(from) {
            continue;
        }
        day(&mut days, tr, local).events.push(EventRow {
            id: event.id.0,
            span: span(tr, event, tz),
            reports: visible.len(),
            max_intensity: visible
                .iter()
                .map(|r| r.values.intensity().get())
                .max()
                .map(|max| max.to_string()),
        });
    }
    for reading in state.store.readings_between(from, to).await?.iter().rev() {
        if reading.event_id.is_none() && can_see(&actor, reading.user_id, now) {
            day(&mut days, tr, schedule.local_date(reading.observed_at))
                .readings
                .push(ReadingRow {
                    at: time::clock(reading.observed_at, tz),
                    value: reading.value.ugm3().to_string(),
                });
        }
    }

    let month_param = |date: NaiveDate| date.format("%Y-%m").to_string();
    let first_of_month = first.with_day(1).expect("every month has a first day");
    page(
        StatusCode::OK,
        &ListPage {
            tr,
            signed_in: Some(user.username.to_string()),
            month: first_of_month.format("%m/%Y").to_string(),
            previous: month_param(first_of_month - Months::new(1)),
            next: month_param(first_of_month + Months::new(1)),
            days: days.into_values().rev().collect(),
        },
    )
}

/// One line of an event's timeline.
struct Entry {
    at: String,
    /// `Some` for a report (its id), `None` for a PM2.5 reading.
    report: Option<i64>,
    text: String,
    /// Shown to people who see everyone's data.
    by: Option<String>,
    /// More than three hours after the previous report (spec §7.7).
    gap_before: bool,
}

#[derive(Template)]
#[template(path = "event.html")]
struct DetailPage {
    tr: Tr,
    signed_in: Option<String>,
    id: i64,
    date: String,
    span: String,
    reason: Option<&'static str>,
    owner: Option<String>,
    entries: Vec<Entry>,
    /// The close form's default end: now, as a `datetime-local` value.
    close_at: Option<String>,
    can_reopen: bool,
    error: Option<&'static str>,
}

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let detail = detail(&state, &user, tr, EventId(id), None).await?;
    page(StatusCode::OK, &detail)
}

async fn detail<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: EventId,
    error: Option<&'static str>,
) -> Result<DetailPage, AppError> {
    let now = state.clock.now();
    let tz = state.config.schedule.tz();
    let actor = user.actor();
    let event = state.store.event(id).await?.ok_or(AppError::NotFound)?;
    let reports = state.store.reports_for_event(id).await?;
    if !can_open(&actor, &event, &reports, now) {
        return Err(AppError::NotFound);
    }
    let sees_all = can(&actor, Action::ViewAll, now);
    let names: HashMap<UserId, String> = if sees_all {
        state
            .store
            .users()
            .await?
            .into_iter()
            .map(|u| (u.id, u.username.to_string()))
            .collect()
    } else {
        HashMap::new()
    };
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let reports: Vec<SmellReport> = reports
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let readings: Vec<Pm25Reading> = state
        .store
        .readings_for_event(id)
        .await?
        .into_iter()
        .filter(|r| can_see(&actor, r.user_id, now))
        .collect();
    let gaps = large_gap_indices(&reports.iter().map(|r| r.observed_at).collect::<Vec<_>>());

    let mut timeline: Vec<(DateTime<Utc>, Entry)> = Vec::new();
    for (index, report) in reports.iter().enumerate() {
        let mut parts = vec![tr.t1(
            "event-intensity",
            "level",
            &report.values.intensity().get().to_string(),
        )];
        let room_names: Vec<String> = rooms
            .iter()
            .filter(|room| report.values.rooms().contains(&room.id))
            .map(|room| labels::room(tr, room))
            .collect();
        let symptom_names: Vec<String> = symptoms
            .iter()
            .filter(|s| report.values.symptoms().contains(&s.id))
            .map(|s| labels::symptom(tr, s))
            .collect();
        parts.extend([room_names.join(", "), symptom_names.join(", ")]);
        timeline.push((
            report.observed_at,
            Entry {
                at: time::clock(report.observed_at, tz),
                report: Some(report.id.0),
                text: parts
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect::<Vec<_>>()
                    .join(" · "),
                by: names.get(&report.user_id).cloned(),
                gap_before: gaps.contains(&index),
            },
        ));
    }
    for reading in &readings {
        timeline.push((
            reading.observed_at,
            Entry {
                at: time::clock(reading.observed_at, tz),
                report: None,
                text: tr.t1("event-pm25", "value", &reading.value.ugm3().to_string()),
                by: names.get(&reading.user_id).cloned(),
                gap_before: false,
            },
        ));
    }
    timeline.sort_by_key(|(at, _)| *at);

    let manages = can(
        &actor,
        Action::ManageEvent {
            owner: event.owner_id,
        },
        now,
    );
    let other_open = match state.store.open_event().await? {
        Some(open) => open.id != event.id,
        None => false,
    };
    Ok(DetailPage {
        tr,
        signed_in: Some(user.username.to_string()),
        id: id.0,
        date: time::date(tr.0, state.config.schedule.local_date(event.opened_at)),
        span: span(tr, &event, tz),
        reason: event.closed.map(|c| labels::close_reason(c.reason)),
        owner: names.get(&event.owner_id).cloned(),
        entries: timeline.into_iter().map(|(_, entry)| entry).collect(),
        close_at: (manages && event.is_open()).then(|| time::input_value(now, tz)),
        can_reopen: manages && !event.is_open() && !other_open,
        error,
    })
}

/// Closes the event by hand (spec §7.3). `closed_at` defaults to now; `return_to=log` comes
/// back to the Log screen.
pub(crate) async fn close<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let id = EventId(id);
    let closed_at = match form.get("closed_at") {
        None => Ok(now),
        Some(value) => {
            time::parse_input(value, state.config.schedule.tz()).map_err(|error| match error {
                TimeInputError::Malformed => "log-error-time",
                TimeInputError::Skipped => "log-error-time-skipped",
            })
        }
    };
    let result = match closed_at {
        Ok(at) => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            match close_event(&mut tx, &user.actor(), id, at, now).await {
                Ok(_) => {
                    tx.commit().await?;
                    Ok(())
                }
                Err(error) => Err(refusal(error)?),
            }
        }
        Err(key) => Err(key),
    };
    match result {
        Ok(()) if form.get("return_to") == Some("log") => Ok(Redirect::to("/").into_response()),
        Ok(()) => Ok(Redirect::to(&format!("/events/{}", id.0)).into_response()),
        Err(key) => {
            let detail = detail(&state, &user, tr, id, Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &detail)
        }
    }
}

pub(crate) async fn reopen<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let id = EventId(id);
    let mut tx = state.store.begin(Some(user.id)).await?;
    match reopen_event(&mut tx, &user.actor(), id, now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to(&format!("/events/{}", id.0)).into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal(error)?;
            let detail = detail(&state, &user, tr, id, Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &detail)
        }
    }
}
```

In `crates/web/src/routes/log.rs`:

```diff
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -86,6 +86,8 @@ pub(crate) struct LogPage {
     tr: Tr,
     signed_in: Option<String>,
     open_since: Option<String>,
+    /// The open event, when the user may close it.
+    close_event: Option<i64>,
     /// "The event from … was auto-closed", shown while another event is open (spec §7.4).
     notice: Option<String>,
     /// A translation key confirming what was just saved.
@@ -219,6 +221,10 @@ pub(crate) async fn log_page<S: Store>(
     Ok(LogPage {
         tr,
         signed_in: Some(user.username.to_string()),
+        close_event: open
+            .as_ref()
+            .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
+            .map(|e| e.id.0),
         open_since: open.map(|e| time::since(tr.0, e.opened_at, now, schedule.tz())),
         notice,
         saved,
```

In `crates/web/src/time.rs`:

```diff
--- a/crates/web/src/time.rs
+++ b/crates/web/src/time.rs
@@ -46,6 +46,11 @@ pub(crate) fn since(locale: Locale, at: DateTime<Utc>, now: DateTime<Utc>, tz: T
     }
 }
 
+/// The value that pre-fills a `datetime-local` input.
+pub(crate) fn input_value(at: DateTime<Utc>, tz: Tz) -> String {
+    at.with_timezone(&tz).format(INPUT_FORMAT).to_string()
+}
+
 /// Reads a `datetime-local` value (seconds optional) as local time in `tz`. A time skipped by a
 /// DST change is refused; a repeated one means its first occurrence.
 pub(crate) fn parse_input(value: &str, tz: Tz) -> Result<DateTime<Utc>, TimeInputError> {
```

In `crates/web/static/app.css`:

```diff
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -195,3 +195,42 @@ table.guide td {
   padding: 0.35rem 0.5rem;
   border-bottom: 1px solid rgb(128 128 128 / 30%);
 }
+
+.menu {
+  display: flex;
+  gap: var(--gap);
+}
+
+.status {
+  display: flex;
+  align-items: center;
+  justify-content: space-between;
+  gap: var(--gap);
+}
+
+.months {
+  display: flex;
+  justify-content: space-between;
+  align-items: baseline;
+}
+
+.rows,
+.timeline {
+  padding-left: 1.25rem;
+}
+
+.timeline .at {
+  font-variant-numeric: tabular-nums;
+  margin-right: 0.5rem;
+}
+
+.timeline .gap {
+  list-style: none;
+  margin: 0.5rem 0 0.5rem -1.25rem;
+  padding: 0.5rem 0.75rem;
+  border-left: 0.25rem solid var(--accent);
+}
+
+.by {
+  opacity: 0.7;
+}
```

In `crates/web/templates/base.html`:

```diff
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -14,6 +14,10 @@
   <header class="bar">
     <span class="brand">{{ tr.t("app-name") }}</span>
     {% if let Some(username) = signed_in %}
+    <nav class="menu">
+      <a href="/">{{ tr.t("nav-log") }}</a>
+      <a href="/events">{{ tr.t("nav-events") }}</a>
+    </nav>
     <form method="post" action="/logout" class="who">
       <span>{{ tr.t1("signed-in-as", "username", username) }}</span>
       <button type="submit" class="link">{{ tr.t("logout") }}</button>
```

Create `crates/web/templates/event.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t1("event-title", "date", date) }}{% endblock %}
{% block content %}
<h1>{{ tr.t1("event-title", "date", date) }}</h1>
<p>{{ span }}{% if let Some(reason) = reason %} · {{ tr.t(reason) }}{% endif %}</p>
{% if let Some(owner) = owner %}
<p>{{ tr.t1("event-started-by", "username", owner) }}</p>
{% endif %}
{% if let Some(error) = error %}
<p class="notice" role="alert">{{ tr.t(error) }}</p>
{% endif %}
<ol class="timeline">
  {% for entry in entries %}
  {% if entry.gap_before %}
  <li class="gap" role="note">{{ tr.t("event-gap") }}</li>
  {% endif %}
  <li>
    <span class="at">{{ entry.at }}</span>
    {% if let Some(report) = entry.report %}<a href="/reports/{{ report }}">{{ entry.text }}</a>{% else %}{{ entry.text }}{% endif %}
    {% if let Some(by) = entry.by %}<span class="by">({{ by }})</span>{% endif %}
  </li>
  {% endfor %}
</ol>
{% if let Some(close_at) = close_at %}
<form method="post" action="/events/{{ id }}/close" class="stack">
  <label>{{ tr.t("event-close-at") }}
    <input type="datetime-local" name="closed_at" value="{{ close_at }}" required>
  </label>
  <button type="submit">{{ tr.t("event-close") }}</button>
</form>
{% endif %}
{% if can_reopen %}
<form method="post" action="/events/{{ id }}/reopen">
  <button type="submit" class="secondary">{{ tr.t("event-reopen") }}</button>
</form>
{% endif %}
<p><a href="/events">{{ tr.t("events-title") }}</a></p>
{% endblock %}
```

Create `crates/web/templates/events.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("events-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("events-title") }}</h1>
<nav class="months">
  <a href="/events?month={{ previous }}">{{ tr.t("events-previous") }}</a>
  <strong>{{ month }}</strong>
  <a href="/events?month={{ next }}">{{ tr.t("events-next") }}</a>
</nav>
{% for day in days %}
<section>
  <h2>{{ day.date }}</h2>
  <ul class="rows">
    {% for event in day.events %}
    <li><a href="/events/{{ event.id }}">{{ event.span }}</a> · {{ tr.n("summary-reports", event.reports) }}{% if let Some(max) = event.max_intensity %} · {{ tr.t1("events-max-intensity", "level", max) }}{% endif %}</li>
    {% endfor %}
    {% for reading in day.readings %}
    <li>{{ reading.at }} · {{ tr.t1("event-pm25", "value", reading.value) }}</li>
    {% endfor %}
  </ul>
</section>
{% else %}
<p>{{ tr.t("events-none") }}</p>
{% endfor %}
{% endblock %}
```

In `crates/web/templates/log.html`:

```diff
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -11,9 +11,15 @@
   <button type="submit" class="link">{{ tr.t("notice-dismiss") }}</button>
 </form>
 {% endif %}
-<p class="status">
-  {% if let Some(since) = open_since %}{{ tr.t1("event-open-since", "time", since) }}{% else %}{{ tr.t("event-none") }}{% endif %}
-</p>
+<div class="status">
+  <p>{% if let Some(since) = open_since %}{{ tr.t1("event-open-since", "time", since) }}{% else %}{{ tr.t("event-none") }}{% endif %}</p>
+  {% if let Some(id) = close_event %}
+  <form method="post" action="/events/{{ id }}/close">
+    <input type="hidden" name="return_to" value="log">
+    <button type="submit" class="secondary">{{ tr.t("event-close") }}</button>
+  </form>
+  {% endif %}
+</div>
 <p class="summary">
   {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %}
 </p>
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `the_event_page_shows_the_timeline_and_warns_about_large_gaps`, `guests_see_only_their_own_entries_and_not_other_peoples_events`, `closing_by_hand_needs_an_end_between_the_last_report_and_now` and `reopening_is_offered_only_while_no_other_event_is_open`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/i18n.rs \
  crates/web/src/labels.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/time.rs \
  crates/web/static/app.css \
  crates/web/templates/base.html \
  crates/web/templates/event.html \
  crates/web/templates/events.html \
  crates/web/templates/log.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/events.rs \
  crates/web/tests/security.rs
git commit -m "feat(web): the Events list and an event's timeline with close, reopen and the gap warning

/events groups a month's events by local day (newest first) with standalone
PM2.5 readings; its reports come from one reports_between window. An event's
page merges reports and readings by time, flags gaps over three hours, names
who logged what for people who see everything, and offers Close (end time
defaulting to now) and Reopen per the permission table. Guests reach only
events they own or reported in, and see only their own entries.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Web — report pages: History, Correct a mistake, Move to another event

`/reports/{id}` shows the values, the observed and the recorded time, the "edited" badge and the
History (spec §6.1, §9 screen 4). "Correct a mistake" is a form pre-filled with the report; it
shares the Log form's fields through an Askama include (`report_fields.html`, plus
`rating_dialog.html` for the popup), and the form-reading code of Task 9 is split into
`read_time` and `read_values` for both. "Move to another event" offers that day's joinable events
and a new one. Someone else's report answers 404 to a guest.

**Files:**
- Create: `crates/web/src/routes/reports.rs`, `crates/web/templates/correct.html`, `crates/web/templates/rating_dialog.html`, `crates/web/templates/report.html`, `crates/web/templates/report_fields.html`, `crates/web/tests/reports.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/entries.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/mod.rs`, `crates/web/static/app.css`, `crates/web/templates/log.html`, `crates/web/tests/security.rs`

**Interfaces:**
- Consumes: `correct_report`, `move_report` (Task 6), `report_history`, `was_corrected`
  (Task 7), `audit_history`, `refusal`, `Draft`, `levels`.
- Produces: `crate::routes::reports::{show, correct_form, correct, relocate, HISTORY_KEYS}`;
  `crate::routes::log::{Fields, fields(tr, rooms, symptoms, &Draft, keep)}` (shared fields; hidden
  rooms and symptoms offered only when kept); `crate::routes::entries::{read_time, read_values}`;
  routes `GET /reports/{id}`, `GET|POST /reports/{id}/correct`, `POST /reports/{id}/move`
  (`target=event-<id>|new`).

- [ ] **Step 1: Write the failing tests**

In `crates/web/src/i18n.rs`, the tests:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,7 +127,7 @@ mod tests {
 
     use super::*;
     use crate::routes::login::Notice;
-    use crate::routes::{self, entries, guide};
+    use crate::routes::{self, entries, guide, reports};
     use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -217,6 +217,7 @@ mod tests {
         used.extend(labels::keys().into_iter().map(str::to_owned));
         used.extend(entries::FORM_ERRORS.map(str::to_owned));
         used.extend(routes::REFUSALS.map(str::to_owned));
+        used.extend(reports::HISTORY_KEYS.map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
```

In `crates/web/src/lib.rs`:

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -50,6 +50,12 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
         .route("/events/{id}", get(routes::events::show::<S>))
         .route("/events/{id}/close", post(routes::events::close::<S>))
         .route("/events/{id}/reopen", post(routes::events::reopen::<S>))
+        .route("/reports/{id}", get(routes::reports::show::<S>))
+        .route(
+            "/reports/{id}/correct",
+            get(routes::reports::correct_form::<S>).post(routes::reports::correct::<S>),
+        )
+        .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
```

In `crates/web/src/routes/mod.rs`:

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -7,6 +7,7 @@ pub(crate) mod lang;
 pub(crate) mod log;
 pub(crate) mod login;
 pub(crate) mod notices;
+pub(crate) mod reports;
 
 use smoke_domain::DomainError;
 use smoke_domain::logbook::LogError;
```

Create `crates/web/tests/reports.rs`:

```rust
//! A report's page: values, both times, History, "Correct a mistake" and "Move to another
//! event" (spec §6.1, §7.7, §9 screen 4).

mod common;

use axum::http::StatusCode;
use common::*;
use smoke_domain::logbook::EventChoice;
use smoke_domain::repos::{EventRepo, ReportRepo};
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn a_report_shows_both_times_its_values_and_its_history() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let bedroom = app.room("room-bedroom").await;
    let logged = app
        .log(&me, "2026-01-15T18:05:00Z", 6, &[bedroom], EventChoice::New)
        .await;
    let cookie = app.sign_in("daniele").await;

    let response = app
        .send(get_with(&format!("/reports/{}", logged.report.id), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(
        html.contains("<dd>15/01/2026 19:05</dd>"),
        "observed: {html}"
    );
    assert!(
        html.contains("<dd>15/01/2026 21:00</dd>"),
        "recorded: {html}"
    );
    assert!(html.contains("<dd>6 · Strong</dd>"), "{html}");
    assert!(html.contains("<dd>Bedroom</dd>"), "{html}");
    assert!(html.contains("<dd>daniele</dd>"), "{html}");
    assert!(
        html.contains("<li>15/01/2026 21:00 · daniele · logged</li>"),
        "{html}"
    );
    assert!(!html.contains("edited"), "{html}");
    assert!(html.contains(">Correct a mistake</a>"), "{html}");
}

#[tokio::test]
async fn correcting_a_mistake_is_recorded_and_badged() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let (bedroom, kitchen) = (
        app.room("room-bedroom").await,
        app.room("room-kitchen").await,
    );
    let logged = app
        .log(&me, "2026-01-15T18:05:00Z", 6, &[bedroom], EventChoice::New)
        .await;
    let id = logged.report.id;
    let cookie = app.sign_in("daniele").await;

    let form = text(
        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
            .await,
    )
    .await;
    assert!(form.contains(r#"value="6" required checked>"#), "{form}");
    assert!(
        form.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
        "{form}"
    );
    assert!(
        form.contains(r#"name="observed_at" value="2026-01-15T19:05""#),
        "{form}"
    );
    assert!(form.contains(r#"<dialog id="rating-guide""#), "{form}");

    app.at("2026-01-16T08:15:00Z");
    let body = format!(
        "intensity=7&room={}&room={}&observed_at=2026-01-15T19:05&note=",
        bedroom.0, kitchen.0
    );
    let response = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            &body,
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*format!("/reports/{id}")));
    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
    assert!(
        html.contains(r#"<span class="badge">edited</span>"#),
        "{html}"
    );
    assert!(
        html.contains(
            "<li>16/01/2026 09:15 · daniele · corrected: intensity 6 → 7, added Kitchen</li>"
        ),
        "{html}"
    );
}

#[tokio::test]
async fn a_correction_must_stay_inside_the_event() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let logged = app
        .log(&me, "2026-01-15T18:05:00Z", 6, &[], EventChoice::New)
        .await;
    let id = logged.report.id;
    let cookie = app.sign_in("daniele").await;
    let response = app
        .send(post_form(
            &format!("/reports/{id}/correct"),
            "intensity=6&observed_at=2026-01-15T18:00",
            Some(&cookie),
        ))
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let html = text(response).await;
    assert!(html.contains("That time is outside the event"), "{html}");
    let unchanged = app.store.report(id).await.unwrap().unwrap();
    assert_eq!(unchanged.observed_at, t("2026-01-15T18:05:00Z"));
}

#[tokio::test]
async fn moving_offers_that_days_events_and_a_new_one() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let morning = app
        .log(&me, "2026-01-15T10:00:00Z", 4, &[], EventChoice::New)
        .await;
    let evening = app
        .log(&me, "2026-01-15T20:00:00Z", 5, &[], EventChoice::New)
        .await;
    let joined = app
        .log(
            &me,
            "2026-01-15T11:00:00Z",
            5,
            &[],
            EventChoice::Join(morning.report.event_id),
        )
        .await;
    let stray = app
        .log(
            &me,
            "2026-01-15T20:00:00Z",
            3,
            &[],
            EventChoice::Join(evening.report.event_id),
        )
        .await;
    let cookie = app.sign_in("daniele").await;
    let path = format!("/reports/{}", joined.report.id);
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    assert!(
        html.contains(r#"<option value="new">New event</option>"#),
        "{html}"
    );
    assert!(
        !html.contains("event-"),
        "the evening event had not started: {html}"
    );

    let path = format!("/reports/{}", stray.report.id);
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    let morning_id = morning.report.event_id;
    assert!(
        html.contains(&format!(
            r#"<option value="event-{morning_id}">Started 11:00, ended 12:00</option>"#
        )),
        "{html}"
    );
    let response = app
        .send(post_form(
            &format!("{path}/move"),
            &format!("target=event-{morning_id}"),
            Some(&cookie),
        ))
        .await;
    assert_eq!(location(&response), Some(&*path));
    let moved = app.store.report(stray.report.id).await.unwrap().unwrap();
    assert_eq!(moved.event_id, morning_id);
    let event = app.store.event(morning_id).await.unwrap().unwrap();
    assert_eq!(event.closed.map(|c| c.at), Some(t("2026-01-15T20:00:00Z")));
    let html = text(app.send(get_with(&path, &cookie)).await).await;
    assert!(
        html.contains(&format!(
            "moved: event #{} → #{morning_id}",
            evening.report.event_id
        )),
        "{html}"
    );
}

#[tokio::test]
async fn guests_cannot_see_or_change_other_peoples_reports() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
    let mine = app
        .log(&me, "2026-01-15T19:00:00Z", 8, &[], EventChoice::New)
        .await;
    let theirs = app
        .log(
            &guest,
            "2026-01-15T19:30:00Z",
            4,
            &[],
            EventChoice::Join(mine.report.event_id),
        )
        .await;
    let cookie = app.sign_in("guest").await;
    let id = mine.report.id;
    for response in [
        app.send(get_with(&format!("/reports/{id}"), &cookie)).await,
        app.send(get_with(&format!("/reports/{id}/correct"), &cookie))
            .await,
        app.send(post_form(
            &format!("/reports/{id}/correct"),
            "intensity=1",
            Some(&cookie),
        ))
        .await,
        app.send(post_form(
            &format!("/reports/{id}/move"),
            "target=new",
            Some(&cookie),
        ))
        .await,
    ] {
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    assert_eq!(app.store.report(id).await.unwrap().unwrap(), mine.report);

    let own = text(
        app.send(get_with(&format!("/reports/{}", theirs.report.id), &cookie))
            .await,
    )
    .await;
    assert!(own.contains(">Correct a mistake</a>"), "{own}");
    assert!(
        !own.contains("daniele") && !own.contains("<dd>guest</dd>"),
        "{own}"
    );
}

#[tokio::test]
async fn viewers_see_reports_but_cannot_change_them() {
    let app = TestApp::new().await;
    app.at("2026-01-15T20:00:00Z");
    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let logged = app
        .log(&me, "2026-01-15T19:00:00Z", 8, &[], EventChoice::New)
        .await;
    let cookie = app.sign_in("lawyer").await;
    let id = logged.report.id;
    let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
    assert!(html.contains("<dd>daniele</dd>"), "{html}");
    assert!(
        !html.contains("Correct a mistake") && !html.contains("/move"),
        "{html}"
    );
    let response = app
        .send(get_with(&format!("/reports/{id}/correct"), &cookie))
        .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
```

In `crates/web/tests/security.rs`:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -156,6 +156,8 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         text(question).await,
         text(app.send(get_with("/events", &cookie)).await).await,
         text(app.send(get_with("/events/1", &cookie)).await).await,
+        text(app.send(get_with("/reports/1", &cookie)).await).await,
+        text(app.send(get_with("/reports/1/correct", &cookie)).await).await,
         text(app.send(get("/login")).await).await,
         text(app.send(get_with("/", &cookie)).await).await,
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile error — `routes::reports`, `fields` and `read_values` do not exist yet.

- [ ] **Step 3: Implement**

In `crates/web/locales/en/main.ftl`:

```diff
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -120,6 +120,33 @@ event-close = Close event
 event-close-at = End time
 event-reopen = Reopen
 
+report-title = Smell report
+report-event = Event
+report-observed = Observed
+report-recorded = Recorded
+report-note = Note
+report-by = Logged by
+report-none = none
+report-edited = edited
+report-deleted = deleted
+report-correct = Correct a mistake
+report-move = Move to another event
+report-move-submit = Move
+report-history = History
+history-create = logged
+history-update = corrected
+history-move = moved
+history-delete = deleted
+history-time = time { $change }
+history-intensity = intensity { $change }
+history-added = added { $name }
+history-removed = removed { $name }
+history-note = note changed
+history-moved = event { $change }
+correct-hint = Correct only mistakes. If the situation changed, log a new report instead.
+correct-time = Observed at
+correct-submit = Save the correction
+
 close-reason-manual = closed by hand
 close-reason-new-event = closed when a new event started
 close-reason-auto = closed automatically overnight
```

In `crates/web/locales/it/main.ftl`:

```diff
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -120,6 +120,33 @@ event-close = Chiudi evento
 event-close-at = Ora di fine
 event-reopen = Riapri
 
+report-title = Segnalazione
+report-event = Evento
+report-observed = Osservata
+report-recorded = Registrata
+report-note = Nota
+report-by = Registrata da
+report-none = nessuno
+report-edited = modificata
+report-deleted = eliminata
+report-correct = Correggi un errore
+report-move = Sposta in un altro evento
+report-move-submit = Sposta
+report-history = Cronologia
+history-create = registrata
+history-update = corretta
+history-move = spostata
+history-delete = eliminata
+history-time = ora { $change }
+history-intensity = intensità { $change }
+history-added = aggiunto { $name }
+history-removed = tolto { $name }
+history-note = nota modificata
+history-moved = evento { $change }
+correct-hint = Correggi solo gli errori. Se la situazione è cambiata, registra invece una nuova segnalazione.
+correct-time = Osservata alle
+correct-submit = Salva la correzione
+
 close-reason-manual = chiuso a mano
 close-reason-new-event = chiuso all'inizio di un nuovo evento
 close-reason-auto = chiuso automaticamente durante la notte
```

In `crates/web/locales/tr/main.ftl`:

```diff
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -120,6 +120,33 @@ event-close = Olayı kapat
 event-close-at = Bitiş saati
 event-reopen = Yeniden aç
 
+report-title = Koku bildirimi
+report-event = Olay
+report-observed = Gözlem
+report-recorded = Kayıt
+report-note = Not
+report-by = Kaydeden
+report-none = yok
+report-edited = düzenlendi
+report-deleted = silindi
+report-correct = Bir hatayı düzelt
+report-move = Başka bir olaya taşı
+report-move-submit = Taşı
+report-history = Geçmiş
+history-create = kaydedildi
+history-update = düzeltildi
+history-move = taşındı
+history-delete = silindi
+history-time = saat { $change }
+history-intensity = yoğunluk { $change }
+history-added = eklendi: { $name }
+history-removed = çıkarıldı: { $name }
+history-note = not değişti
+history-moved = olay { $change }
+correct-hint = Yalnızca hataları düzelt. Durum değiştiyse bunun yerine yeni bir bildirim kaydet.
+correct-time = Gözlem zamanı
+correct-submit = Düzeltmeyi kaydet
+
 close-reason-manual = elle kapatıldı
 close-reason-new-event = yeni bir olay başlayınca kapatıldı
 close-reason-auto = gece otomatik olarak kapatıldı
```

In `crates/web/src/routes/entries.rs`:

```diff
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -47,8 +47,56 @@ enum Entry {
     },
 }
 
-/// Reads and validates the form. `Err` is the translation key of what to fix. Only rooms and
-/// symptoms the form offers (not hidden) are accepted.
+/// Reads an optional `datetime-local` field; `None` when left blank.
+pub(crate) fn read_time(
+    form: &Pairs,
+    name: &str,
+    tz: Tz,
+) -> Result<Option<DateTime<Utc>>, &'static str> {
+    form.get(name)
+        .map(|value| {
+            time::parse_input(value, tz).map_err(|error| match error {
+                TimeInputError::Malformed => "log-error-time",
+                TimeInputError::Skipped => "log-error-time-skipped",
+            })
+        })
+        .transpose()
+}
+
+/// Reads intensity, rooms, symptoms and note. Only the offered rooms and symptoms are accepted.
+pub(crate) fn read_values(
+    form: &Pairs,
+    rooms: &[RoomId],
+    symptoms: &[SymptomId],
+) -> Result<ReportValues, &'static str> {
+    let intensity = form
+        .get("intensity")
+        .and_then(|value| value.parse().ok())
+        .and_then(|value| Intensity::new(value).ok())
+        .ok_or("log-error-intensity")?;
+    let room_ids: Vec<RoomId> = form
+        .ids("room")
+        .ok_or("log-error-form")?
+        .into_iter()
+        .map(RoomId)
+        .collect();
+    let symptom_ids: Vec<SymptomId> = form
+        .ids("symptom")
+        .ok_or("log-error-form")?
+        .into_iter()
+        .map(SymptomId)
+        .collect();
+    if !room_ids.iter().all(|id| rooms.contains(id))
+        || !symptom_ids.iter().all(|id| symptoms.contains(id))
+    {
+        return Err("log-error-form");
+    }
+    ReportValues::new(intensity, room_ids, symptom_ids, form.get("note"))
+        .map_err(|_| "log-error-note")
+}
+
+/// Reads and validates the Log form. `Err` is the translation key of what to fix. Only rooms
+/// and symptoms the form offers (not hidden) are accepted.
 fn read(
     form: &Pairs,
     rooms: &[Room],
@@ -56,13 +104,7 @@ fn read(
     tz: Tz,
     now: DateTime<Utc>,
 ) -> Result<Entry, &'static str> {
-    let observed_at = match form.get("observed_at") {
-        None => now,
-        Some(value) => time::parse_input(value, tz).map_err(|error| match error {
-            TimeInputError::Malformed => "log-error-time",
-            TimeInputError::Skipped => "log-error-time-skipped",
-        })?,
-    };
+    let observed_at = read_time(form, "observed_at", tz)?.unwrap_or(now);
     if classify_observed(observed_at, now).is_err() {
         return Err("refused-future");
     }
@@ -81,11 +123,6 @@ fn read(
         let value = pm25.ok_or("log-error-pm25-missing")?;
         return Ok(Entry::Pm25Only { observed_at, value });
     }
-    let intensity = form
-        .get("intensity")
-        .and_then(|value| value.parse().ok())
-        .and_then(|value| Intensity::new(value).ok())
-        .ok_or("log-error-intensity")?;
     let offered_rooms: Vec<RoomId> = rooms
         .iter()
         .filter(|r| r.hidden_at.is_none())
@@ -96,25 +133,7 @@ fn read(
         .filter(|s| s.hidden_at.is_none())
         .map(|s| s.id)
         .collect();
-    let room_ids: Vec<RoomId> = form
-        .ids("room")
-        .ok_or("log-error-form")?
-        .into_iter()
-        .map(RoomId)
-        .collect();
-    let symptom_ids: Vec<SymptomId> = form
-        .ids("symptom")
-        .ok_or("log-error-form")?
-        .into_iter()
-        .map(SymptomId)
-        .collect();
-    if !room_ids.iter().all(|id| offered_rooms.contains(id))
-        || !symptom_ids.iter().all(|id| offered_symptoms.contains(id))
-    {
-        return Err("log-error-form");
-    }
-    let values = ReportValues::new(intensity, room_ids, symptom_ids, form.get("note"))
-        .map_err(|_| "log-error-note")?;
+    let values = read_values(form, &offered_rooms, &offered_symptoms)?;
     Ok(Entry::Smell(SmellEntry {
         observed_at,
         values,
```

In `crates/web/src/routes/log.rs`:

```diff
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -7,7 +7,7 @@ use axum::response::Response;
 use chrono::{DateTime, Utc};
 use serde::Deserialize;
 use smoke_domain::permissions::{Action, can, can_see};
-use smoke_domain::records::User;
+use smoke_domain::records::{Room, Symptom, User};
 use smoke_domain::repos::Store;
 use smoke_domain::starter::BEDROOM_KEY;
 use smoke_domain::summary::{MonthSummary, summarise};
@@ -61,16 +61,64 @@ impl Draft {
 
 /// A radio button, chip or checkbox.
 pub(crate) struct Choice {
-    value: i64,
-    label: String,
-    checked: bool,
+    pub(crate) value: i64,
+    pub(crate) label: String,
+    pub(crate) checked: bool,
+}
+
+/// The intensity, room and symptom fields shared by the Log form and the correction form
+/// (`templates/report_fields.html`).
+pub(crate) struct Fields {
+    pub(crate) intensities: Vec<Choice>,
+    pub(crate) inside: Vec<Choice>,
+    pub(crate) common: Vec<Choice>,
+    pub(crate) symptoms: Vec<Choice>,
+}
+
+/// The choices for `draft`. Hidden rooms and symptoms are offered only if listed in `keep`
+/// (a correction keeps those its report already names).
+pub(crate) fn fields(
+    tr: Tr,
+    rooms: &[Room],
+    symptoms: &[Symptom],
+    draft: &Draft,
+    keep: (&[RoomId], &[SymptomId]),
+) -> Fields {
+    let room_choices = |area: RoomArea| {
+        rooms
+            .iter()
+            .filter(|r| r.area == area && (r.hidden_at.is_none() || keep.0.contains(&r.id)))
+            .map(|r| Choice {
+                value: r.id.0,
+                label: labels::room(tr, r),
+                checked: draft.rooms.contains(&r.id),
+            })
+            .collect()
+    };
+    Fields {
+        intensities: (Intensity::MIN..=Intensity::MAX)
+            .map(|value| Choice {
+                value: value.into(),
+                label: value.to_string(),
+                checked: draft.intensity == Some(value),
+            })
+            .collect(),
+        inside: room_choices(RoomArea::Inside),
+        common: room_choices(RoomArea::Common),
+        symptoms: symptoms
+            .iter()
+            .filter(|s| s.hidden_at.is_none() || keep.1.contains(&s.id))
+            .map(|s| Choice {
+                value: s.id.0,
+                label: labels::symptom(tr, s),
+                checked: draft.symptoms.contains(&s.id),
+            })
+            .collect(),
+    }
 }
 
 pub(crate) struct FormView {
-    intensities: Vec<Choice>,
-    inside: Vec<Choice>,
-    common: Vec<Choice>,
-    symptoms: Vec<Choice>,
+    fields: Fields,
     pm25: String,
     observed_at: String,
     note: String,
@@ -174,40 +222,9 @@ pub(crate) async fn log_page<S: Store>(
                 blank
             }
         };
-        let room_choices = |area: RoomArea| {
-            rooms
-                .iter()
-                .filter(|r| r.area == area && r.hidden_at.is_none())
-                .map(|r| Choice {
-                    value: r.id.0,
-                    label: labels::room(tr, r),
-                    checked: draft.rooms.contains(&r.id),
-                })
-                .collect()
-        };
-        let symptoms = state
-            .store
-            .symptoms()
-            .await?
-            .iter()
-            .filter(|s| s.hidden_at.is_none())
-            .map(|s| Choice {
-                value: s.id.0,
-                label: labels::symptom(tr, s),
-                checked: draft.symptoms.contains(&s.id),
-            })
-            .collect();
+        let symptoms = state.store.symptoms().await?;
         Some(FormView {
-            intensities: (Intensity::MIN..=Intensity::MAX)
-                .map(|value| Choice {
-                    value: value.into(),
-                    label: value.to_string(),
-                    checked: draft.intensity == Some(value),
-                })
-                .collect(),
-            inside: room_choices(RoomArea::Inside),
-            common: room_choices(RoomArea::Common),
-            symptoms,
+            fields: fields(tr, &rooms, &symptoms, &draft, (&[], &[])),
             pm25: draft.pm25,
             observed_at: draft.observed_at,
             note: draft.note,
```

Create `crates/web/src/routes/reports.rs`:

```rust
//! A report's page (spec §9, screen 4): its values, observed and recorded times, the "edited"
//! badge and History; "Correct a mistake" and "Move to another event" for those allowed to.

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::events::can_join_backdated;
use smoke_domain::history::{Change, report_history, was_corrected};
use smoke_domain::logbook::{MoveTarget, correct_report, move_report};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{Room, SmellReport, Symptom, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{EventId, ReportId, Role, RoomId, SymptomId, UserId};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::i18n::Tr;
use crate::routes::entries::{read_time, read_values};
use crate::routes::guide::{Level, levels};
use crate::routes::log::{Draft, Fields, fields};
use crate::routes::refusal;
use crate::state::AppState;
use crate::{labels, time};

/// The report, if it exists and `user` may see it; otherwise "not found", so nobody learns
/// that someone else's report exists.
async fn visible_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<SmellReport, AppError> {
    let now = state.clock.now();
    state
        .store
        .report(id)
        .await?
        .filter(|r| can_see(&user.actor(), r.user_id, now))
        .ok_or(AppError::NotFound)
}

fn room_names(tr: Tr, rooms: &[Room], ids: &[RoomId]) -> Vec<String> {
    rooms
        .iter()
        .filter(|r| ids.contains(&r.id))
        .map(|r| labels::room(tr, r))
        .collect()
}

fn symptom_names(tr: Tr, symptoms: &[Symptom], ids: &[SymptomId]) -> Vec<String> {
    symptoms
        .iter()
        .filter(|s| ids.contains(&s.id))
        .map(|s| labels::symptom(tr, s))
        .collect()
}

struct HistoryLine {
    at: String,
    by: Option<String>,
    action: &'static str,
    changes: Vec<String>,
}

struct MoveOption {
    value: String,
    label: String,
}

#[derive(Template)]
#[template(path = "report.html")]
struct ReportPage {
    tr: Tr,
    signed_in: Option<String>,
    id: i64,
    event_id: i64,
    event_date: String,
    observed: String,
    recorded: String,
    intensity: String,
    rooms: String,
    symptoms: String,
    note: Option<String>,
    by: Option<String>,
    edited: bool,
    deleted: bool,
    history: Vec<HistoryLine>,
    /// Shown to those who may correct and move the report.
    editable: bool,
    move_options: Vec<MoveOption>,
    error: Option<&'static str>,
}

fn action_key(action: AuditAction) -> &'static str {
    match action {
        AuditAction::Create => "history-create",
        AuditAction::Move => "history-move",
        AuditAction::Delete => "history-delete",
        _ => "history-update",
    }
}

/// Every key [`action_key`] can return, for the translation tests.
#[cfg(test)]
pub(crate) const HISTORY_KEYS: [&str; 4] = [
    "history-create",
    "history-move",
    "history-delete",
    "history-update",
];

pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let report_page = report_page(&state, &user, tr, ReportId(id), None).await?;
    page(StatusCode::OK, &report_page)
}

async fn report_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    id: ReportId,
    error: Option<&'static str>,
) -> Result<ReportPage, AppError> {
    let now = state.clock.now();
    let schedule = &state.config.schedule;
    let tz = schedule.tz();
    let actor = user.actor();
    let report = visible_report(state, user, id).await?;
    let event = state
        .store
        .event(report.event_id)
        .await?
        .ok_or(AppError::NotFound)?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let sees_all = can(&actor, Action::ViewAll, now);
    let name = |id: Option<UserId>, users: &[User]| -> Option<String> {
        users
            .iter()
            .find(|u| Some(u.id) == id)
            .map(|u| u.username.to_string())
    };
    let users = if sees_all {
        state.store.users().await?
    } else {
        Vec::new()
    };

    let entries = state
        .store
        .audit_history(AuditEntity::SmellReport, id.0)
        .await?;
    let history = report_history(&entries);
    let lines = history
        .iter()
        .map(|entry| HistoryLine {
            at: time::date_time(tr.0, entry.at, tz),
            by: name(entry.actor, &users),
            action: action_key(entry.action),
            changes: entry
                .changes
                .iter()
                .map(|change| match change {
                    Change::ObservedAt { from, to } => tr.t1(
                        "history-time",
                        "change",
                        &format!(
                            "{} → {}",
                            time::date_time(tr.0, *from, tz),
                            time::date_time(tr.0, *to, tz)
                        ),
                    ),
                    Change::Intensity { from, to } => {
                        tr.t1("history-intensity", "change", &format!("{from} → {to}"))
                    }
                    Change::RoomAdded(room) => tr.t1(
                        "history-added",
                        "name",
                        &room_names(tr, &rooms, &[*room]).concat(),
                    ),
                    Change::RoomRemoved(room) => tr.t1(
                        "history-removed",
                        "name",
                        &room_names(tr, &rooms, &[*room]).concat(),
                    ),
                    Change::SymptomAdded(symptom) => tr.t1(
                        "history-added",
                        "name",
                        &symptom_names(tr, &symptoms, &[*symptom]).concat(),
                    ),
                    Change::SymptomRemoved(symptom) => tr.t1(
                        "history-removed",
                        "name",
                        &symptom_names(tr, &symptoms, &[*symptom]).concat(),
                    ),
                    Change::Note { .. } => tr.t("history-note"),
                    Change::Moved { from, to } => {
                        tr.t1("history-moved", "change", &format!("#{from} → #{to}"))
                    }
                })
                .collect(),
        })
        .collect();

    let editable = !report.is_deleted()
        && can(
            &actor,
            Action::EditEntry {
                owner: report.user_id,
            },
            now,
        );
    let mut move_options = Vec::new();
    if editable {
        let (day_start, day_end) = schedule.day_span(schedule.local_date(report.observed_at));
        for target in state.store.events_between(day_start, day_end).await? {
            if target.id != report.event_id
                && can_join_backdated(&target, report.observed_at, schedule)
            {
                let mut label = tr.t1(
                    "prompt-started-at",
                    "time",
                    &time::clock(target.opened_at, tz),
                );
                if user.role != Role::Guest
                    && let Some(closed) = target.closed
                {
                    label.push_str(", ");
                    label.push_str(&tr.t1("prompt-ended-at", "time", &time::clock(closed.at, tz)));
                }
                move_options.push(MoveOption {
                    value: format!("event-{}", target.id),
                    label,
                });
            }
        }
        move_options.push(MoveOption {
            value: "new".to_owned(),
            label: tr.t("prompt-new-event"),
        });
    }

    let none = || tr.t("report-none");
    let join = |names: Vec<String>| {
        if names.is_empty() {
            none()
        } else {
            names.join(", ")
        }
    };
    let intensity = report.values.intensity().get();
    Ok(ReportPage {
        tr,
        signed_in: Some(user.username.to_string()),
        id: id.0,
        event_id: report.event_id.0,
        event_date: time::date(tr.0, schedule.local_date(event.opened_at)),
        observed: time::date_time(tr.0, report.observed_at, tz),
        recorded: time::date_time(tr.0, report.recorded_at, tz),
        intensity: format!(
            "{intensity} · {}",
            tr.t(&format!("intensity-{intensity}-name"))
        ),
        rooms: join(room_names(tr, &rooms, report.values.rooms())),
        symptoms: join(symptom_names(tr, &symptoms, report.values.symptoms())),
        note: report.values.note().map(str::to_owned),
        by: name(Some(report.user_id), &users),
        edited: was_corrected(&history),
        deleted: report.is_deleted(),
        history: lines,
        editable,
        move_options,
        error,
    })
}

pub(crate) struct CorrectForm {
    fields: Fields,
    observed_at: String,
    note: String,
    error: Option<&'static str>,
}

#[derive(Template)]
#[template(path = "correct.html")]
struct CorrectPage {
    tr: Tr,
    signed_in: Option<String>,
    id: i64,
    levels: Vec<Level>,
    form: CorrectForm,
}

/// The report, if `user` may correct and move it.
async fn editable_report<S: Store>(
    state: &AppState<S>,
    user: &User,
    id: ReportId,
) -> Result<SmellReport, AppError> {
    let report = visible_report(state, user, id).await?;
    let allowed = can(
        &user.actor(),
        Action::EditEntry {
            owner: report.user_id,
        },
        state.clock.now(),
    );
    if report.is_deleted() {
        Err(AppError::NotFound)
    } else if allowed {
        Ok(report)
    } else {
        Err(AppError::NotAllowed)
    }
}

async fn correct_page<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    report: &SmellReport,
    draft: Draft,
    error: Option<&'static str>,
) -> Result<CorrectPage, AppError> {
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let keep = (report.values.rooms(), report.values.symptoms());
    Ok(CorrectPage {
        tr,
        signed_in: Some(user.username.to_string()),
        id: report.id.0,
        levels: levels(tr),
        form: CorrectForm {
            fields: fields(tr, &rooms, &symptoms, &draft, keep),
            observed_at: draft.observed_at,
            note: draft.note,
            error,
        },
    })
}

/// "Correct a mistake": the form, pre-filled with the report as it is.
pub(crate) async fn correct_form<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let report = editable_report(&state, &user, ReportId(id)).await?;
    let draft = Draft {
        intensity: Some(report.values.intensity().get()),
        rooms: report.values.rooms().to_vec(),
        symptoms: report.values.symptoms().to_vec(),
        pm25: String::new(),
        observed_at: time::input_value(report.observed_at, state.config.schedule.tz()),
        note: report.values.note().unwrap_or_default().to_owned(),
    };
    let correct_page = correct_page(&state, &user, tr, &report, draft, None).await?;
    page(StatusCode::OK, &correct_page)
}

pub(crate) async fn correct<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let schedule = state.config.schedule;
    let report = editable_report(&state, &user, ReportId(id)).await?;
    let rooms = state.store.rooms().await?;
    let symptoms = state.store.symptoms().await?;
    let offered_rooms: Vec<RoomId> = rooms
        .iter()
        .filter(|r| r.hidden_at.is_none() || report.values.rooms().contains(&r.id))
        .map(|r| r.id)
        .collect();
    let offered_symptoms: Vec<SymptomId> = symptoms
        .iter()
        .filter(|s| s.hidden_at.is_none() || report.values.symptoms().contains(&s.id))
        .map(|s| s.id)
        .collect();
    let parsed = read_time(&form, "observed_at", schedule.tz()).and_then(|at| {
        let values = read_values(&form, &offered_rooms, &offered_symptoms)?;
        Ok((at.unwrap_or(report.observed_at), values))
    });
    let error = match parsed {
        Ok((observed_at, values)) => {
            let mut tx = state.store.begin(Some(user.id)).await?;
            match correct_report(
                &mut tx,
                &user.actor(),
                report.id,
                observed_at,
                values,
                &schedule,
                now,
            )
            .await
            {
                Ok(_) => {
                    tx.commit().await?;
                    return Ok(Redirect::to(&format!("/reports/{id}")).into_response());
                }
                Err(error) => refusal(error)?,
            }
        }
        Err(key) => key,
    };
    let draft = Draft::from_form(&form);
    let correct_page = correct_page(&state, &user, tr, &report, draft, Some(error)).await?;
    page(StatusCode::UNPROCESSABLE_ENTITY, &correct_page)
}

/// "Move to another event": `target` is `event-<id>` or `new`.
pub(crate) async fn relocate<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let target = match form.get("target") {
        Some("new") => MoveTarget::New,
        Some(value) => match value.strip_prefix("event-").and_then(|v| v.parse().ok()) {
            Some(event) => MoveTarget::Event(EventId(event)),
            None => return Err(AppError::NotFound),
        },
        None => return Err(AppError::NotFound),
    };
    visible_report(&state, &user, ReportId(id)).await?;
    let mut tx = state.store.begin(Some(user.id)).await?;
    let schedule = state.config.schedule;
    match move_report(&mut tx, &user.actor(), ReportId(id), target, &schedule, now).await {
        Ok(_) => {
            tx.commit().await?;
            Ok(Redirect::to(&format!("/reports/{id}")).into_response())
        }
        Err(error) => {
            drop(tx);
            let key = refusal(error)?;
            let report_page = report_page(&state, &user, tr, ReportId(id), Some(key)).await?;
            page(StatusCode::UNPROCESSABLE_ENTITY, &report_page)
        }
    }
}
```

In `crates/web/static/app.css`:

```diff
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -234,3 +234,32 @@ table.guide td {
 .by {
   opacity: 0.7;
 }
+
+.badge {
+  font-size: 0.75rem;
+  font-weight: 400;
+  padding: 0.1rem 0.5rem;
+  border: 1px solid currentColor;
+  border-radius: 999px;
+  vertical-align: middle;
+}
+
+.facts {
+  display: grid;
+  grid-template-columns: max-content 1fr;
+  gap: 0.25rem var(--gap);
+}
+
+.facts dt {
+  font-weight: 600;
+}
+
+.facts dd {
+  margin: 0;
+}
+
+select {
+  font: inherit;
+  padding: 0.75rem;
+  border-radius: 0.5rem;
+}
```

Create `crates/web/templates/correct.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("report-correct") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("report-correct") }}</h1>
<p>{{ tr.t("correct-hint") }}</p>
{% if let Some(error) = form.error %}
<p class="notice" role="alert">{{ tr.t(error) }}</p>
{% endif %}
<form method="post" action="/reports/{{ id }}/correct" class="stack">
  {% include "report_fields.html" %}
  <label>{{ tr.t("correct-time") }}
    <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" required>
  </label>
  <label>{{ tr.t("report-note") }}
    <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
  </label>
  <button type="submit">{{ tr.t("correct-submit") }}</button>
</form>
<p><a href="/reports/{{ id }}">{{ tr.t("prompt-cancel") }}</a></p>
{% include "rating_dialog.html" %}
{% endblock %}
```

In `crates/web/templates/log.html`:

```diff
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -28,33 +28,7 @@
 <p class="notice" role="alert">{{ tr.t(error) }}</p>
 {% endif %}
 <form method="post" action="/log" class="stack log">
-  <fieldset class="levels">
-    <legend>{{ tr.t("log-intensity") }}
-      <a href="/guide/rating" data-dialog="rating-guide" class="info">ⓘ {{ tr.t("log-how-to-rate") }}</a>
-    </legend>
-    {% for level in form.intensities %}
-    <label class="chip"><input type="radio" name="intensity" value="{{ level.value }}" required{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
-    {% endfor %}
-  </fieldset>
-  <fieldset class="chips">
-    <legend>{{ tr.t("log-rooms") }}</legend>
-    <p class="group">{{ tr.t("log-inside") }}</p>
-    {% for room in form.inside %}
-    <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
-    {% endfor %}
-    {% if !form.common.is_empty() %}
-    <p class="group">{{ tr.t("log-common") }}</p>
-    {% for room in form.common %}
-    <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
-    {% endfor %}
-    {% endif %}
-  </fieldset>
-  <fieldset class="checks">
-    <legend>{{ tr.t("log-symptoms") }}</legend>
-    {% for symptom in form.symptoms %}
-    <label><input type="checkbox" name="symptom" value="{{ symptom.value }}"{% if symptom.checked %} checked{% endif %}> {{ symptom.label }}</label>
-    {% endfor %}
-  </fieldset>
+  {% include "report_fields.html" %}
   <label>{{ tr.t("log-pm25") }}
     <input type="number" name="pm25" min="0" max="1000" step="0.1" inputmode="decimal" value="{{ form.pm25 }}">
   </label>
@@ -68,21 +42,7 @@
   <button type="submit" name="action" value="smell">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
   <button type="submit" name="action" value="pm25" formnovalidate class="secondary">{{ tr.t("log-pm25-only") }}</button>
 </form>
-<dialog id="rating-guide" aria-labelledby="rating-guide-title">
-  <div class="dialog-body">
-    <form method="dialog" class="dialog-close">
-      <button type="submit" class="link" aria-label="{{ tr.t("close") }}">✕</button>
-    </form>
-    <h2 id="rating-guide-title">{{ tr.t("rating-title") }}</h2>
-    <table class="guide compact">
-      {% for level in levels %}
-      <tr><th scope="row">{{ level.value }}</th><td>{{ level.name }}</td></tr>
-      {% endfor %}
-    </table>
-    <p>{{ tr.t("rating-tip") }}</p>
-    <p><a href="/guide/rating">{{ tr.t("rating-full-guide") }}</a></p>
-  </div>
-</dialog>
+{% include "rating_dialog.html" %}
 {% else %}
 <p>{{ tr.t("log-read-only") }}</p>
 {% endif %}
```

Create `crates/web/templates/rating_dialog.html`:

```html
{# The compact intensity guide that "How to rate" opens (spec §5). #}
<dialog id="rating-guide" aria-labelledby="rating-guide-title">
  <div class="dialog-body">
    <form method="dialog" class="dialog-close">
      <button type="submit" class="link" aria-label="{{ tr.t("close") }}">✕</button>
    </form>
    <h2 id="rating-guide-title">{{ tr.t("rating-title") }}</h2>
    <table class="guide compact">
      {% for level in levels %}
      <tr><th scope="row">{{ level.value }}</th><td>{{ level.name }}</td></tr>
      {% endfor %}
    </table>
    <p>{{ tr.t("rating-tip") }}</p>
    <p><a href="/guide/rating">{{ tr.t("rating-full-guide") }}</a></p>
  </div>
</dialog>
```

Create `crates/web/templates/report.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("report-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("report-title") }}{% if edited %} <span class="badge">{{ tr.t("report-edited") }}</span>{% endif %}{% if deleted %} <span class="badge">{{ tr.t("report-deleted") }}</span>{% endif %}</h1>
{% if let Some(error) = error %}
<p class="notice" role="alert">{{ tr.t(error) }}</p>
{% endif %}
<dl class="facts">
  <dt>{{ tr.t("report-event") }}</dt>
  <dd><a href="/events/{{ event_id }}">{{ tr.t1("event-title", "date", event_date) }}</a></dd>
  <dt>{{ tr.t("report-observed") }}</dt>
  <dd>{{ observed }}</dd>
  <dt>{{ tr.t("report-recorded") }}</dt>
  <dd>{{ recorded }}</dd>
  <dt>{{ tr.t("log-intensity") }}</dt>
  <dd>{{ intensity }}</dd>
  <dt>{{ tr.t("log-rooms") }}</dt>
  <dd>{{ rooms }}</dd>
  <dt>{{ tr.t("log-symptoms") }}</dt>
  <dd>{{ symptoms }}</dd>
  {% if let Some(note) = note %}
  <dt>{{ tr.t("report-note") }}</dt>
  <dd>{{ note }}</dd>
  {% endif %}
  {% if let Some(by) = by %}
  <dt>{{ tr.t("report-by") }}</dt>
  <dd>{{ by }}</dd>
  {% endif %}
</dl>
{% if editable %}
<p><a href="/reports/{{ id }}/correct">{{ tr.t("report-correct") }}</a></p>
<form method="post" action="/reports/{{ id }}/move" class="stack">
  <label>{{ tr.t("report-move") }}
    <select name="target">
      {% for option in move_options %}
      <option value="{{ option.value }}">{{ option.label }}</option>
      {% endfor %}
    </select>
  </label>
  <button type="submit" class="secondary">{{ tr.t("report-move-submit") }}</button>
</form>
{% endif %}
<h2>{{ tr.t("report-history") }}</h2>
<ol class="history">
  {% for line in history %}
  <li>{{ line.at }}{% if let Some(by) = line.by %} · {{ by }}{% endif %} · {{ tr.t(line.action) }}{% if !line.changes.is_empty() %}: {{ line.changes.join(", ") }}{% endif %}</li>
  {% endfor %}
</ol>
{% endblock %}
```

Create `crates/web/templates/report_fields.html`:

```html
{# The intensity, room and symptom fields of the Log and correction forms. #}
<fieldset class="levels">
  <legend>{{ tr.t("log-intensity") }}
    <a href="/guide/rating" data-dialog="rating-guide" class="info">ⓘ {{ tr.t("log-how-to-rate") }}</a>
  </legend>
  {% for level in form.fields.intensities %}
  <label class="chip"><input type="radio" name="intensity" value="{{ level.value }}" required{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
  {% endfor %}
</fieldset>
<fieldset class="chips">
  <legend>{{ tr.t("log-rooms") }}</legend>
  <p class="group">{{ tr.t("log-inside") }}</p>
  {% for room in form.fields.inside %}
  <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
  {% endfor %}
  {% if !form.fields.common.is_empty() %}
  <p class="group">{{ tr.t("log-common") }}</p>
  {% for room in form.fields.common %}
  <label class="chip"><input type="checkbox" name="room" value="{{ room.value }}"{% if room.checked %} checked{% endif %}><span>{{ room.label }}</span></label>
  {% endfor %}
  {% endif %}
</fieldset>
<fieldset class="checks">
  <legend>{{ tr.t("log-symptoms") }}</legend>
  {% for symptom in form.fields.symptoms %}
  <label><input type="checkbox" name="symptom" value="{{ symptom.value }}"{% if symptom.checked %} checked{% endif %}> {{ symptom.label }}</label>
  {% endfor %}
</fieldset>
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-web` and `cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `correcting_a_mistake_is_recorded_and_badged`, `moving_offers_that_days_events_and_a_new_one` and `guests_cannot_see_or_change_other_peoples_reports`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/entries.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/reports.rs \
  crates/web/static/app.css \
  crates/web/templates/correct.html \
  crates/web/templates/log.html \
  crates/web/templates/rating_dialog.html \
  crates/web/templates/report.html \
  crates/web/templates/report_fields.html \
  crates/web/tests/reports.rs \
  crates/web/tests/security.rs
git commit -m "feat(web): report pages with History, Correct a mistake and Move to another event

A report shows its observed and recorded times, an 'edited' badge and its
History, rendered from the audit snapshots. The correction form reuses the
Log form's fields through an Askama include; corrections and moves go through
the domain use cases, and someone else's report answers 404 to a guest, so its
existence never shows.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Decisions and roadmap

**Files:**
- Modify: `docs/DECISIONS.md` (append rows #86–#101; the file is append-only)
- Modify: `docs/plans/ROADMAP.md`

**Interfaces:** none (documentation).

- [ ] **Step 1: Append the decisions and update the roadmap**

{DOCS_DIFF}
In `docs/plans/ROADMAP.md` row 4 links `04-logging-flow.md` while the milestone is in progress;
add ✅ once it is merged (row 3 gets its ✅ now: milestone 3 is merged).

- [ ] **Step 2: Commit**

```bash
git add docs/DECISIONS.md docs/plans/ROADMAP.md
git commit -m "docs: decisions #86–#101 for the logging flow; roadmap links the milestone 4 plan

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: Spec wording — needs the user's approval; skip unless approved

Two clarifications so the spec matches decisions #89 and #96. **Executors: do not start this task
until the user has approved these exact edits.** Nothing in the code depends on them.

**Files:**
- Modify: `docs/specs/2026-10-13-smoke-tracker-design.md`

- [ ] **Step 1: Show the user the edits and wait for approval**

1. §12, *Errors*, second bullet — from *"Validation errors re-render the form with a translated
   message (htmx partial)."* to *"Validation errors re-render the form with a translated message
   (the same page, status 422)."*
2. §7.5, second bullet — from *"It lists the events on that day (guests see start times only)
   plus **New event**."* to *"It lists the events that had started by then and were still running
   that day (guests see start times only) plus **New event**."*

- [ ] **Step 2: Apply exactly the approved edits and commit**

```bash
git add docs/specs/2026-10-13-smoke-tracker-design.md
git commit -m "docs(spec): plain-form validation errors; which events a past report may join

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Development

Local loop for this milestone (one shared build folder for every worktree):

```sh
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
export TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/sqlx-prepare.sh --check
cargo deny check
```

To try it by hand: `DATABASE_URL=sqlite://data/smoke.db APP_BASE_URL=http://localhost:8080
cargo run -p smoke-tracker -- create-resident <name>`, then the same with `serve`, and open
`http://localhost:8080`.

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §4 | Guests log and see own data; viewers read all, never write; expired guests read-only | 5, 6, 8–11 |
| §4 | Close / reopen: residents, and guests on events they own | 6, 10 |
| §4 | Correct / move: residents all, guests own | 6, 11 |
| §5 | "Update: situation changed" main button while an event is open; edit only on the report page, labelled "Correct a mistake" | 8, 11 |
| §5 | Intensity guide 1–10 and tip, texts in the translation files | 8 |
| §5 | ⓘ *How to rate* opens a native `<dialog>` (✕, backdrop, back gesture) with the compact table, the tip and "Full guide →" | 8 |
| §7 | At most one open event | 5, 6 (on top of the partial unique index, #49) |
| §7.1 | Logging now with nothing open creates an event owned by the logger | 5, 9 |
| §7.2 | "Same smell still going?" — Yes joins; No closes the open event at its last report and starts a new one; guests see only the start time | 5, 9 |
| §7.3 | Manual close between the last report and now; new-event close; auto-close at 04:00 at the last report; an end never later than the last report unless closed by hand | 2, 5, 6, 10 |
| §7.4 | Auto-close notice to the owner (and residents for guests' events); continuation reopens (audited) and attaches; only the latest offered, the rest marked seen; a banner while another event is open | 4, 5, 9 |
| §7.5 | Backdating with no limit; observed and recorded times both shown; "Which event was this?" with that day's events and New event; a backdated new event open only if nothing is open and no 04:00 passed; a report after the end moves the end (audited) | 4, 5, 9, 11 |
| §7.6 | PM2.5 with a report (one transaction) or alone; linked to the event containing its time; never opens, extends or closes an event | 1, 5, 9 |
| §7.7 | Correct a mistake (audited before/after); move to another or a new event (audited); large-gap warning over 3 hours | 6, 10, 11 |
| §9 screen 1 | Log screen: status bar, 1–10 grid + ⓘ, room chips grouped Inside/Common pre-selected from the last report in the open event, symptom checkboxes, PM2.5, time "now" + change, note, Log/Update, Close event, Log PM2.5 only | 8, 9, 10 |
| §9 screen 2 | Summary card "This month: … events · … reports · … days with bedroom smoke" | 7, 8 |
| §9 screen 3 | Events grouped by day; event timeline of reports and readings, large-gap warning, Close / Reopen | 10 |
| §9 screen 4 | Report detail: values, observed and recorded times, Correct a mistake, Move to another event, History | 7, 11 |
| §9.2 | Every new text in en/it/tr; starter rooms and symptoms translated | 8–11 |
| §11 | Sweep: `jobs.last_run_at`, catch-up semantics, at startup, before every logging request, on a timer; DST days | 2, 3, 8, 9 |
| §12 | Validation errors re-render the form with a translated message; CSP holds with no inline code | 8–11 (see Task 13) |
| §13.3 | jobs: sweep at 04:00 vs catch-up at 08:00, DST days (fake clock); web: guest isolation, viewer can't write | 2, 9–11 |

Out of scope here and planned later: invites, People, Settings (language, password, lists), the
PWA manifest and service worker (milestone 5); graphs, the Guides section with *How to log*, the
README tables and CodeQL (milestone 6); `migrate-to-postgres` (milestone 7). The "save_session
upsert race" noted in milestone 3's final review is deferred to milestone 5.
