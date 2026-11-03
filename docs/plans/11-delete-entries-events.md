# Deleting entries and events — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let people delete their own log entries, and whole events whose live entries they may all delete, through the audited store, with "an event exists only while it has a live entry" as the model.

**Architecture:** Storage hides events with no live entry from every event read and enforces the one-open-event rule itself (the `events_one_open` index goes). Two new store primitives (`move_event_start`, in-transaction `entries_for_event`) let the domain recalculate an event when its earliest entry leaves. A new domain module `deleting.rs` holds the delete rule (pure) and the two use cases; the web layer adds two POST routes and in-page confirmation via a `?delete` query.

**Tech Stack:** Rust, sqlx (SQLite + Postgres, offline `.sqlx` data), axum, Askama, Fluent.

**Spec:** `docs/specs/2026-11-03-delete-entries-events-design.md` (approved 2026-11-03).

## Global Constraints

- No hard deletes; deleting an entry is the existing audited `delete_entry` (one `log_entry` `delete` audit row each).
- No new column on `events`; "event exists" = `EXISTS (SELECT 1 FROM log_entries le WHERE le.event_id = events.id AND le.deleted_at IS NULL)`.
- Deleting never touches `closed_at` / `close_reason`.
- Migration must be safe on live Neon data: index drop only. Number it `0007`; if `feat/readonly-role` (Postgres `0007_readonly_role`) lands on `main` first, renumber both backends' file to `0008` before merging.
- Every changed query: run `scripts/sqlx-prepare.sh` (needs `TEST_POSTGRES_URL`, local container `smoke-pg` on port 55432) and commit `.sqlx/` changes.
- All builds use the one shared `CARGO_TARGET_DIR` (`/Users/daniele/Documents/Personal/smoke-tracker/target`); never a per-agent target.
- Commit as `locadani` (repo-local config); no push.
- Copy is Italian-first; en/it/tr keys added together; it/tr drafts flagged for native review in the commit message.
- No browser dialogs; confirmation works without JavaScript.
- No tutorial comments in code; doc comments state behaviour, as in the existing code.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, SQLite contract, Postgres contract and web tests green at every task end.

## Review Focus

1. **A guest deletes their own entry in a resident's event** → the event still exists but the guest may no longer open it; they must land on History, not "Page not found". Test in Task 6.
2. **Double tap / someone deleted first** → the second POST answers 404 and writes no audit row. Tests in Tasks 5 (use case) and 6/7 (routes).
3. **Deleting the earliest entry of a closed event when the next one is after `closed_at`** (backdated entries) → `opened_at` stops at `closed_at`, `close_reason` unchanged. Test in Task 5.
4. **An open event emptied by deletion** → no banner or "Same smell still going?" on the Log screen, the 04:00 sweep leaves it alone, and the next smell opens a fresh open event. Tests in Tasks 1, 2 and 7.
5. **Viewers and expired guests** → no Elimina on either page; a forged POST answers 403 (visible entry/event) or 404 (not visible). Tests in Tasks 6 and 7.

## Two calls not in the spec (proposed defaults, confirm before Task 7)

- **Guests never see other people's names** (the event page shows names only to `ViewAll`). For a guest the recap says `tu 1 · altri 3` and always uses the "gli altri devono…" sentence.
- After deleting an entry, the redirect goes to its event only if the event still exists **and** the actor may still open it; otherwise History.

---

### Task 1: Events without a live entry are invisible to every event read

**Files:**
- Modify: `crates/storage-sqlite/src/events.rs`, `crates/storage-postgres/src/events.rs`
- Modify: `crates/storage-contract/src/fixture.rs`, `crates/storage-contract/src/{events,sweep,logbook,corrections,audit,entries}.rs` (tests that read back bare events)
- Modify: `crates/app/src/commands/serve.rs` (test `the_event_sweep_runs_as_soon_as_the_server_starts` creates a bare event)
- Modify: `crates/domain/src/repos.rs` (doc comments on `EventRepo` / `EventWrites` reads)
- Test: `crates/storage-contract/src/events.rs`, registered in `crates/storage-contract/src/lib.rs`

**Interfaces:**
- Produces: `fixture::add_live_event(fx, owner: UserId, opened_at, closed: Option<Closure>) -> Event` — `add_event` plus one PM2.5-only entry (no report, value 10) by `owner` at `opened_at` in the same transaction, so `latest_report_at` is unaffected. `add_event` stays a bare create.
- Produces: in each backend, a private `fetch_live(conn, id)`; the unfiltered `fetch`/`require` remain for writes, audit snapshots and `fetch_all`.

Filtered (live only): `EventRepo::{event, open_event, events_between, unseen_auto_closed}`, `EventWrites::{open_event, event, event_at}`. Unfiltered: `require` inside writes, `audited`, `create_event`'s read-back, `fetch_all`.

- [ ] **Step 1: Write the failing contract tests** in `storage-contract/src/events.rs`:

```rust
pub async fn an_event_without_live_entries_is_not_an_event<S: Store>(fx: &Fixture<S>)
// bare open event (add_event, no entry): store.event(id) == None, store.open_event() == None,
// events_between(..) excludes it, tx.event(id) == None, tx.open_event() == None, tx.event_at(t) == None.
pub async fn an_event_whose_entries_are_all_deleted_disappears<S: Store>(fx: &Fixture<S>)
// add_live_event (auto-closed, unseen) → visible in all reads incl. unseen_auto_closed;
// delete its only entry → None / excluded from every read above; audit_state still lists it.
pub async fn the_sweep_ignores_an_emptied_open_event<S>(fx: &Fixture<S>)   // in sweep.rs
// open event emptied by delete_entry, clock past 04:00, auto_close → Ran { closed: None };
// raw event row (via audit_state) still has closed_at None.
```

- [ ] **Step 2: Run** `cargo test -p smoke-storage-sqlite --test contract` — Expected: the three new tests FAIL (events still returned).
- [ ] **Step 3: Implement** the live filter in both backends' queries listed above (add `AND EXISTS (…)` from Global Constraints; `fetch_open` adds `ORDER BY opened_at DESC, id DESC LIMIT 1`).
- [ ] **Step 4: Fix existing tests** that read back a bare event: switch them to `add_live_event`, or add an entry, keeping each test's intent. `scene()` uses `add_live_event`; adjust any assertion that counts the scene event's entries or expects the last audit row to be the event's. In `serve.rs` add a PM2.5-only entry after `create_event`.
- [ ] **Step 5: Run** `scripts/sqlx-prepare.sh`, then `cargo test -p smoke-storage-sqlite -p smoke-storage-postgres -p smoke-tracker -p smoke-web` — Expected: all PASS.
- [ ] **Step 6: Commit** `feat(storage): an event exists only while it has a live entry`.

### Task 2: The app, not an index, keeps one open event

**Files:**
- Create: `crates/storage-sqlite/migrations/0007_drop_events_one_open.sql`, `crates/storage-postgres/migrations/0007_drop_events_one_open.sql` (`DROP INDEX events_one_open;` plus a one-line comment naming decision #149)
- Modify: `crates/storage-{sqlite,postgres}/src/events.rs` (`create_event`, `reopen_event`)
- Modify: `crates/storage-{sqlite,postgres}/src/error.rs` (remove the `events_one_open` mapping and its test lines)
- Modify: `crates/storage-sqlite/tests/contract.rs` / `crates/storage-postgres/tests/contract.rs` only if they list indexes or migrations
- Test: `crates/storage-contract/src/events.rs`

**Interfaces:**
- Consumes: Task 1's live filter.
- Produces: private `another_live_open_event(conn, except: Option<EventId>) -> StoreResult<bool>` per backend; `create_event` with `closed: None` and `reopen_event` return `Conflict::AnotherEventOpen` when it is true. Both run inside the write transaction (SQLite: single writer + `BEGIN IMMEDIATE`; Postgres: `LOCK TABLE audit_log IN EXCLUSIVE MODE` in `begin`), so the check is race-free. Doc comment on `EventWrites::create_event`/`reopen_event` says so.

- [ ] **Step 1: Write failing contract tests:**

```rust
pub async fn an_emptied_open_event_does_not_block_a_new_one<S: Store>(fx: &Fixture<S>)
// add_live_event open; delete its entry; create_event(closed: None) succeeds and is the open
// event once it has an entry.
pub async fn reopening_ignores_an_emptied_open_event<S: Store>(fx: &Fixture<S>)
// closed live event A; open event B emptied by deletion; reopen_event(A) succeeds.
```
Existing `only_one_event_can_be_open` and `reopening_is_refused_while_another_event_is_open` must keep passing (switch them to `add_live_event` — with a bare event they now pass silently).

- [ ] **Step 2: Run** them — Expected: FAIL with `Conflict(AnotherEventOpen)` from the index.
- [ ] **Step 3: Add the migrations and the store check.**
- [ ] **Step 4: Run** `scripts/sqlx-prepare.sh` and the full SQLite + Postgres contract suites — Expected: PASS, including `migrations::*` (a live-data migration test that inserts two open events, if any, must still pass).
- [ ] **Step 5: Commit** `feat(storage): enforce one open event in the store, drop events_one_open`.

### Task 3: Store primitives for recalculating an event

**Files:**
- Modify: `crates/domain/src/repos.rs`, `crates/storage-{sqlite,postgres}/src/{events,entries}.rs`
- Test: `crates/storage-contract/src/events.rs`, `crates/storage-contract/src/entries.rs`

**Interfaces:**
- Produces: `EventWrites::move_event_start(&mut self, id: EventId, opened_at: DateTime<Utc>, owner_id: UserId) -> StoreResult<Event>` — audited `update` (no row when nothing changes, as `record` already does); `opened_at` after `closed_at` is `Conflict::RuleViolated` (the existing CHECK).
- Produces: `EntryWrites::entries_for_event(&mut self, event: EventId) -> StoreResult<Vec<LogEntry>>` — live entries, `ORDER BY observed_at, id`, same loading as `EntryRepo::entries_for_event` (share one private function taking a connection).

- [ ] **Step 1: Write failing tests:** `moving_an_event_start_changes_owner_and_is_audited` (one `event`/`update` audit row with before/after; a second identical call adds none; `opened_at` past `closed_at` → `Conflict::RuleViolated`), and `entries_for_event_in_a_transaction_sees_its_own_writes` (create an entry in the tx, it is listed; a deleted one is not).
- [ ] **Step 2: Run** — Expected: FAIL to compile (methods missing).
- [ ] **Step 3: Implement** in both backends.
- [ ] **Step 4: Run** `scripts/sqlx-prepare.sh` and both contract suites — Expected: PASS.
- [ ] **Step 5: Commit** `feat(storage): move an event's start and owner; list an event's entries in a transaction`.

### Task 4: Who may delete (pure rules)

**Files:**
- Modify: `crates/domain/src/permissions.rs`
- Create: `crates/domain/src/deleting.rs` (rule part); register in `crates/domain/src/lib.rs`

**Interfaces:**
- Produces: `Action::DeleteEntry { author: UserId, author_role: Role }` — Resident: `author == actor.id || author_role == Role::Guest`; Guest: active and `author == actor.id`; Viewer: never. Add both owner cases × both roles to the `ACTIONS` test table.
- Produces in `deleting.rs`:

```rust
pub struct EntryAuthor { pub author: UserId, pub role: Role }
pub struct AuthorCount { pub author: UserId, pub entries: usize }
pub enum EventDeletion {
    Hidden,                                   // may delete none of the live entries
    Allowed { entries: usize },               // may delete all of them
    Blocked { recap: Vec<AuthorCount>, blockers: Vec<UserId> },
}
pub fn event_deletion(actor: &Actor, entries: &[EntryAuthor], now: DateTime<Utc>) -> EventDeletion
```
`recap`: one line per author, actor first, then by `entries` descending, then by `UserId`. `blockers`: authors with an entry the actor may not delete, in recap order. Empty `entries` → `Hidden`.

- [ ] **Step 1: Write failing unit tests** in `deleting.rs`: `only_my_entries_is_allowed` (3 mine → `Allowed { entries: 3 }`); `a_resident_may_delete_a_guests_entries` (2 mine + 1 guest → `Allowed { entries: 3 }`); `another_resident_blocks` (me 3, Duygu(resident) 1 → `Blocked { recap: [me 3, duygu 1], blockers: [duygu] }`); `guests_never_block_a_resident` (me 2, Duygu 2, guest Marco 1 → blockers `[duygu]`, recap `[me 2, duygu 2, marco 1]`); `a_viewer_or_expired_guest_sees_nothing` (`Hidden`); `none_of_mine_is_hidden` (resident, only another resident's → `Hidden`); `a_guest_is_blocked_by_anyone_else`.
- [ ] **Step 2: Run** `cargo test -p smoke-domain deleting` — Expected: FAIL to compile.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p smoke-domain` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(domain): who may delete an entry or an event`.

### Task 5: Delete use cases, and events that follow their remaining entries

**Files:**
- Modify: `crates/domain/src/deleting.rs`, `crates/domain/src/logbook.rs`
- Create: `crates/storage-contract/src/deleting.rs`; register module and tests in `crates/storage-contract/src/lib.rs`

**Interfaces:**
- Consumes: Tasks 3 and 4.
- Produces in `logbook.rs`:
  - `LogError::OthersLogged` — "someone else's entries are in the event".
  - `pub(crate) async fn is_earliest_entry<T: EntryWrites>(tx: &mut T, event: EventId, entry: LogEntryId) -> StoreResult<bool>` — first of `entries_for_event`.
  - `pub(crate) async fn follow_remaining_entries<T: EventWrites + EntryWrites>(tx: &mut T, event: EventId) -> StoreResult<()>` — if live entries remain: `move_event_start(event, min(first.observed_at, closed_at if closed), first.user_id)`; otherwise nothing.
  - `move_report`: when the entry leaves an event (`Event(other)` or `New`) and was its earliest, call `follow_remaining_entries` on the old event after `move_entry`.
- Produces in `deleting.rs`:

```rust
/// Returns the entry's event if it still exists afterwards.
pub async fn delete_entry<T: EventWrites + EntryWrites + UserWrites>(
    tx: &mut T, actor: &Actor, id: LogEntryId, now: DateTime<Utc>,
) -> Result<Option<EventId>, LogError>
/// Returns how many entries were deleted.
pub async fn delete_event<T: EventWrites + EntryWrites + UserWrites>(
    tx: &mut T, actor: &Actor, id: EventId, now: DateTime<Utc>,
) -> Result<usize, LogError>
```
`delete_entry`: missing or deleted → `NotFound`; author's role from `tx.user`; not allowed → `Forbidden`; earliest check before `tx.delete_entry`, `follow_remaining_entries` after. `delete_event`: `tx.event` (live) missing → `NotFound`; `event_deletion` → `Hidden` ⇒ `Forbidden`, `Blocked` ⇒ `OthersLogged`, `Allowed` ⇒ `tx.delete_entry` for each.
- Web refusal mapping (`crates/web/src/routes/mod.rs::refusal`) must stay exhaustive: `LogError::OthersLogged => Err(AppError::NotAllowed)` (the event route handles it before calling `refusal`).

- [ ] **Step 1: Write failing contract tests** in `storage-contract/src/deleting.rs`:
  - `deleting_the_only_entry_removes_the_event` → returns `None`; `store.event` None; exactly one new audit row (`log_entry`/`delete`).
  - `deleting_a_later_entry_leaves_the_event_alone` → `Some(event)`; event unchanged; one audit row.
  - `deleting_the_earliest_entry_moves_the_start_and_owner` → Daniele 01:00, Duygu (resident) 01:30: Daniele deletes his → `opened_at` 01:30, `owner_id` Duygu, audit rows `log_entry/delete` then `event/update`; `closed`/`close_reason` unchanged.
  - `the_start_never_passes_the_end` → `add_live_event` closed 01:00–01:40 (entry at 01:00), plus an entry at 01:50 written straight with `tx.create_entry` (the store does not check spans); delete the 01:00 entry → `opened_at` 01:40, `closed` unchanged.
  - `a_check_with_no_event_is_deleted` → `None`, one audit row.
  - `deleting_twice_is_not_found` → second call `NotFound`, audit count unchanged.
  - `a_resident_deletes_a_guests_entry_but_not_another_residents` (and an expired guest's entry); `a_guest_or_viewer_cannot_delete_others` → `Forbidden`.
  - `deleting_an_event_deletes_every_entry_in_one_transaction` → 3 entries (2 mine, 1 guest's) → `Ok(3)`, 3 audit rows, event gone; `deleting_an_event_with_another_residents_entry_is_refused` → `OthersLogged`, nothing written; `deleting_an_event_none_of_mine_is_forbidden`; `deleting_an_emptied_event_is_not_found`.
  - `moving_the_earliest_entry_out_moves_the_start` and `moving_the_only_entry_out_removes_the_event` (via `move_report` with `MoveTarget::New`).
  - `verify_audit_is_clean_after_deletions` → after an entry delete, an earliest-entry delete and an event delete, `verify_chain(&audit_log)` is `Ok` and `verify_store(store).await.unwrap().is_ok()`.
- [ ] **Step 2: Run** `cargo test -p smoke-storage-sqlite --test contract deleting` — Expected: FAIL to compile.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** both contract suites and `cargo test -p smoke-domain -p smoke-web` — Expected: PASS.
- [ ] **Step 5: Commit** `feat(domain): delete entries and events; an event follows its remaining entries`.

### Task 6: Delete an entry from its page

**Files:**
- Create: `crates/web/src/routes/deleting.rs` (both POST handlers live here; this task adds `delete_entry`)
- Modify: `crates/web/src/lib.rs` (route `POST /entries/{id}/delete`), `crates/web/src/routes/mod.rs`, `crates/web/src/routes/entry_page.rs`, `crates/web/templates/entry.html`, `crates/web/templates/components.html`, `crates/web/locales/{en,it,tr}/main.ftl`
- Test: `crates/web/tests/deleting.rs`

**Interfaces:**
- Consumes: `deleting::delete_entry`, `Action::DeleteEntry`.
- Produces: `components.html` macro `confirm_delete(question, action, cancel_href)` — a card with the question, a POST form with a `danger` submit labelled `delete`, and a `secondary` link labelled `delete-cancel` to `cancel_href`. Task 7 reuses it.
- Entry page: `EntryPage` gains `deletable: bool` and `confirm_delete: bool`. `GET /entries/{id}?delete` sets `confirm_delete` when `deletable`. The `Elimina` link (`href="?delete"`, `data-variant="danger"`) sits next to `Correggi un errore`, also on entries without a report. Run auto-close first, as the other POST handlers do.
- After success: `/events/{id}` when `delete_entry` returns `Some(id)` and the actor can open it (`store.event` + `can_open`), else `/events`. `NotFound` → 404; `Forbidden` → 403.

Keys (it shown; en: `Delete`, `Cancel`, `Delete this entry?`, `This can't be undone.`):

```
delete = Elimina
delete-cancel = Annulla
delete-cannot-undo = Non si può annullare.
entry-delete-question = Eliminare questa registrazione?
```

- [ ] **Step 1: Write failing web tests:** `the_author_deletes_an_entry_and_lands_on_its_event`; `deleting_the_last_entry_lands_on_history`; `a_reading_only_entry_offers_delete`; `the_confirm_step_shows_question_and_cancel` (GET `?delete` contains the it question, `Non si può annullare.`, a form posting to `/entries/{id}/delete`, cancel link to `/entries/{id}`); `a_resident_may_delete_a_guests_entry_but_sees_no_button_on_another_residents`; `a_viewer_and_an_expired_guest_see_no_button_and_are_refused` (POST → 403); `a_guest_deleting_in_a_residents_event_lands_on_history` (Review Focus 1); `deleting_twice_is_not_found` (second POST 404).
- [ ] **Step 2: Run** `cargo test -p smoke-web --test deleting` — Expected: FAIL.
- [ ] **Step 3: Implement**, including the en/it/tr keys (tr: draft).
- [ ] **Step 4: Run** `cargo test -p smoke-web` (includes the translation-parity tests) — Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): delete an entry from its page` (body: "it/tr drafts need native review").

### Task 7: Delete an event from its page

**Files:**
- Modify: `crates/web/src/routes/deleting.rs` (`delete_event`), `crates/web/src/lib.rs` (route `POST /events/{id}/delete`), `crates/web/src/routes/events.rs` (`DetailPage`), `crates/web/templates/event.html`, `crates/web/locales/{en,it,tr}/main.ftl`, `tests/e2e/specs/accessibility.spec.js` (visit `/events/{id}?delete`)
- Test: `crates/web/tests/deleting.rs`, `crates/web/tests/log.rs` (Review Focus 4)

**Interfaces:**
- Consumes: `event_deletion`, `deleting::delete_event`, `confirm_delete` macro.
- `DetailPage` gains `deletion: DeletionView` where

```rust
enum DeletionView { Hidden, Allowed { question: String, confirm: bool }, Blocked { recap: String, reason: String } }
```
built from `event_deletion` with every live entry's author role (from `store.users()`, already loaded when `ViewAll`; load it always for roles, but names stay `ViewAll`-only). `Blocked` renders a disabled `danger` button plus `recap` and `reason` below it; `Allowed` renders an `Elimina` link to `?delete` next to `Riapri`/`Chiudi`, and the confirm card when `?delete`.
- Recap text: `{count} registrazioni: tu 3 · Duygu 1`; for a guest viewer, other authors collapse into `altri N` (proposed default above). Reason: exactly one blocker and the viewer sees names → `event-delete-blocked-one`; else `event-delete-blocked-many`.
- POST: `OthersLogged` → 303 to `/events/{id}` (fresh recap; spec "refused at save time"); `NotFound` → 404; `Forbidden` → 403; success → `/events`.

Keys (it; en/tr equivalents, tr draft):

```
event-delete-question = { $count ->
    [one] Eliminare questo evento e la sua registrazione?
   *[other] Eliminare questo evento e le sue { $count } registrazioni?
}
event-delete-recap = { $count } registrazioni: { $people }
event-delete-you = tu { $count }
event-delete-others = altri { $count }
event-delete-blocked-one = Per eliminare l'evento, { $name } deve prima eliminare le sue.
event-delete-blocked-many = Per eliminare l'evento, gli altri devono prima eliminare le loro.
```
(en: "Delete this event and its entry?" / "…and its {count} entries?", "{count} entries: {people}", "you {count}", "others {count}", "To delete the event, {name} must first delete theirs.", "To delete the event, the others must first delete theirs.")

- [ ] **Step 1: Write failing web tests:** `only_my_entries_offers_delete_and_confirms_with_the_count` (3 → "le sue 3 registrazioni", 1 → "la sua registrazione"); `deleting_an_event_lands_on_history_and_the_event_is_gone` (GET `/events/{id}` → 404, History no longer lists it); `another_resident_greys_the_button_with_recap` (exact `4 registrazioni: tu 3 · duygu 1` and `Per eliminare l'evento, duygu deve prima eliminare le sue.`, button `disabled`); `three_people_use_the_others_sentence`; `a_guests_entries_never_block_a_resident`; `a_guest_sees_others_counted_not_named`; `no_button_when_none_is_mine` and for a viewer; `a_forged_post_when_blocked_redirects_to_the_event` (nothing deleted); `deleting_an_emptied_event_is_not_found`. In `log.rs`: `an_emptied_open_event_asks_nothing_and_a_new_smell_opens_a_fresh_event` (Review Focus 4).
- [ ] **Step 2: Run** — Expected: FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test -p smoke-web`, then `npx playwright test specs/accessibility.spec.js` in `tests/e2e` (as the CI job does) — Expected: PASS.
- [ ] **Step 5: Commit** `feat(web): delete an event from its page` (body: "it/tr drafts need native review").

### Task 8: Docs (approval-gated)

**Files:**
- Modify: `docs/DECISIONS.md` (#149: an event exists only while it has a live entry; the store enforces one open event, `events_one_open` dropped — rejected: a `deleted_at` column on events, keeping the index; #150: residents may delete guests' entries, an event is deleted only when all its live entries may be; #151 the two proposed defaults, if approved)
- Modify: main spec `docs/specs/2026-10-13-smoke-tracker-design.md` (deletion section; one-open-event rule now app-enforced), `docs/specs/2026-10-19-log-entries-design.md:24,44` ("no delete UI exists"), README "Who can do what" table, spec status line of the delete design → "approved 2026-11-03"
- Modify: `docs/plans/ROADMAP.md` if it lists open work

- [ ] **Step 1: Draft the edits** and show Daniele a one-screen diff summary; wait for approval (CLAUDE.md: no .md changes without approval).
- [ ] **Step 2: Apply** the approved wording; renumber decisions if `main` gained #149+ meanwhile.
- [ ] **Step 3: Commit** `docs: deleting entries and events`.
