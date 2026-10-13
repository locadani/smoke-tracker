# Milestone 5: People, invites & PWA — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Residents invite guests, viewers and other residents with a one-time QR link, reset
passwords the same way, and end or restore people's access; everyone sets their language and
password in Settings, and residents edit the room and symptom lists there; the app installs as a
PWA with an offline page and an Install button. A session ended by a password change can no
longer come back through a late renewal.

**Architecture:** Same layering as milestones 3–4. `smoke-domain` gets the invite record and its
repository traits (`InviteRepo`, `InviteWrites`), and two use-case modules in the style of the
logbook: `people` (invite, setup/reset link, accept, expire, reactivate, change password, expiry
quick picks) and `lists` (add, hide/show rooms and symptoms), each checking permissions inside
the write transaction. Both storage crates implement the invite traits; the `invites` table has
existed since migration 0001, so **there is no migration**. `smoke-web` adds Settings, People,
the invite/QR pages, the invitee's setup page, the install and offline pages, the manifest, the
service worker and a header that knows who sees People. `smoke-tracker` (app) deletes expired
invites daily.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, askama 0.16, fluent-templates 0.15,
tower-sessions 0.15, chrono + chrono-tz, sqlx 0.9 (SQLite + Postgres), tokio. **One new crate:
`qrcodegen` 1.8.0** (MIT, no dependencies; decision #119).

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` §4 (roles, expiry, sessions ended on
expiry), §6 (`invites`), §8.3 (invites and password reset), §8.4 (sessions ended on password
change), §9 screens 6–8, §9.1 (PWA), §11 (expired invites removed daily), §12 (CSP). Decisions:
`docs/DECISIONS.md` rows **#111–#121**, each appended by the task that makes it. Previous plans:
`docs/plans/04-logging-flow.md`, `docs/plans/04b-room-levels.md`.

**Provenance:** every code block in this plan was compiled and tested before the plan was
written, as one commit per task on the branch **`m5-proto`** (worktree
`/Users/daniele/Documents/Personal/smoke-tracker-m5proto`, from `main` at `5528ee0`; commits
`f7e8a20`, `1f04e74`, `904cdb5`, `3c9dcc3`, `8fa4c8e`, `87a9c4f`, `7e3f5b7`, `da8d7ed` = Tasks
1–8). At every task boundary `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
-- -D warnings` and `cargo test --workspace` (SQLite **and** Postgres) passed;
`scripts/sqlx-prepare.sh --check` passed after Tasks 1 and 2, and `cargo deny check licenses
bans sources` after Task 7. Test totals with `TEST_POSTGRES_URL` set: 490 at `5528ee0`, then
493, 506, 525, 530, 536, 545, 555, **562** at the head (domain 123, SQLite contract 129, Postgres
contract 124, web unit 58, web integration 107, app 18, other 3). Executors may consult that
branch (e.g. `git -C …/smoke-tracker-m5proto show 7e3f5b7`); this plan is the source of truth.

## Decisions for the user to confirm

Defaults were chosen so nothing blocks; each is a decision row and easy to change.

1. **Changing your password keeps this phone signed in** and signs out every other device
   (decision #116). It needs the current password, checked like a sign-in attempt (5 wrong tries
   lock the username for 15 minutes, as at sign-in).
2. **Only the newest link works**: making a new setup or reset QR code for someone kills their
   earlier unused one; expired links are deleted at startup and daily, without audit entries
   (decision #112).
3. **Only an expired account can be reactivated** (spec §4). To shorten or extend an active
   guest's stay, End access now, then Reactivate with the new end (decision #114).
4. **Guest quick picks** (decision #115): 1 day / 1 week / 2 weeks count from now; *weekend* lasts
   until the coming **Monday 00:00** (on a Monday, the next one); a chosen day lasts until that
   day's midnight. Viewers get "never" (default) or a chosen day.
5. **Room and symptom lists**: residents add and hide/show items; there is **no rename or
   delete**, and duplicate names are allowed (decision #117).
6. **New dependency `qrcodegen` 1.8.0** for the QR code, drawn as inline SVG (decision #119).
   The QR page also shows the link as plain text (copyable, deliberately not clickable).
7. **86 new translation keys**: 86 Italian and 86 Turkish strings (172) are drafts for native
   review (Task 4: 14, Task 5: 9, Task 6: 25, Task 7: 24, Task 8: 14).
8. **No database migration**: the schema is unchanged, so nothing changes on the live Neon
   database when this deploys (`invites` exists since migration 0001, empty in production).

## Global Constraints

- Personal project. Before every commit, `git config user.name` must print `locadani` and
  `git config user.email` must print locadani's GitHub noreply address (the repository's local
  config already holds it). Never put any email address in a tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings: local and CI Postgres use trust authentication.
- Never modify `description.md` or `CLAUDE.md`. **Never edit
  `docs/specs/2026-10-13-smoke-tracker-design.md`** without the user's explicit approval; no task
  touches it.
- Every commit message ends with a blank line followed by exactly
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI, `cargo fmt`
  enforced.
- `smoke-web` never sees SQL and depends on a storage crate only as a dev-dependency. It writes
  accounts only through `smoke_domain::people`, the lists only through `smoke_domain::lists`, the
  language only through the existing `/lang` handler, and logging data only through
  `smoke_domain::logbook`.
- **No migration in this milestone**; `0001`–`0004` stay untouched and `schema_parity` keeps
  passing. Production is live on Neon with real data: a migration would need its own plan.
- After changing any `query!` text, run `scripts/sqlx-prepare.sh` (with `TEST_POSTGRES_URL`
  set) and commit the regenerated `.sqlx/` files (Tasks 1 and 2).
- Free tier and a small footprint. The only new crate is `qrcodegen` 1.8.0 (Task 7);
  `Cargo.lock` changes only by that entry.
- Content-Security-Policy is `'self'` only. No template may contain inline `<script>` code,
  `<style>`, `style=` attributes, `on*=` handlers, `hx-on` or `javascript:` URLs; inline SVG uses
  presentation attributes (`fill`, `stroke`) only. The only scripts are the vendored htmx,
  `static/app.js` and `static/sw.js`. Every new page is added to
  `no_page_needs_inline_scripts_styles_or_eval`.
- An invite token is a password until used: it is **never stored** (only its SHA-256), never in
  an audit snapshot, **never logged** (paths under `/invite/` are cut from the request span), and
  never a clickable link on the resident's page.
- Every "now" goes through `smoke_domain::clock::Clock`; local dates and times (expiry picks,
  "valid until 20:05", "until 22/01/2026 20:00") go through `DailySchedule` and `APP_TIMEZONE`.
- Role refusals answer **403** with the "Not allowed" page (decision #101); unknown ids answer
  **404**; a dead link answers **410** with its own page.
- Every new translation key exists in `en`, `it` and `tr` (the existing test fails otherwise);
  keys passed through variables are listed in the module's `keys()`/`KEYS` for
  `every_key_the_pages_use_is_translated`. Italian and Turkish texts are drafts for native review.
- Production runs on Cloud Run with **max-instances = 1** (the sign-in limiter, which now also
  guards password changes, is in memory) and `CLIENT_IP_HEADER=X-Forwarded-For`.
- Postgres tests need `TEST_POSTGRES_URL`; locally the container `smoke-pg` listens on **port
  55432** (`postgres://postgres@localhost:55432/postgres`). The contract tests create one
  database per test and never drop it: if tests fail with `could not resize shared memory
  segment … No space left on device`, run `docker restart smoke-pg`, wait for
  `docker exec smoke-pg pg_isready`, and drop the leftover `smoke_test_%` databases (the planner
  had to drop 8 188 of them).
- `docs/DECISIONS.md` is append-only; this plan adds rows **#111–#121**.
- Disk: all builds share one `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target`;
  never create another target folder, never run `cargo clean` or `rm -rf` on build folders (the
  user does that). Check `df -h /System/Volumes/Data` before big builds.

## Review Focus

1. **A session coming back after it was ended**: a request that loaded its session before a
   password change, a reset or an expiry, and renews it afterwards. Expected: the session stays
   ended on every other device; only the device that changed the password keeps (a new) one.
   Tests: `a_renewal_racing_a_password_change_cannot_revive_the_session` (Task 1, both
   backends), `a_late_save_cannot_revive_a_session_a_password_change_ended` (Task 1),
   `a_password_change_keeps_this_device_and_signs_out_the_others` (Task 4),
   `a_reset_link_replaces_the_password_and_ends_every_session` (Task 3),
   `a_reset_link_replaces_the_password_and_signs_out_every_device` (Task 7),
   `ending_a_guests_access_signs_them_out_and_leaves_them_read_only` (Task 6).
2. **A link that works twice, late, or after a newer one** (two phones, a reload, a resident who
   pressed the button again). Expected: 410 and nothing changed. Tests:
   `a_new_invite_replaces_the_users_unused_ones`, `an_invite_is_used_once_and_that_is_audited`
   (Task 2); `an_invited_guest_sets_up_the_account_once`,
   `a_link_stops_working_after_five_minutes` (Task 3);
   `an_invited_guest_scans_sets_a_password_and_is_signed_in`, `a_link_works_for_five_minutes_only`,
   `a_newer_link_replaces_the_one_shown_before` (Task 7).
3. **The token leaking** into the database, the audit log, the request logs or a tap on the
   resident's own phone. Expected: only the hash is stored, no snapshot or log line carries
   either, the page shows the link as text. Tests: `an_invite_is_found_by_its_token_hash_only`
   (Task 2), `a_token_hash_is_the_sha256_of_the_token_in_hex` (Task 2),
   `invite_tokens_are_cut_from_logged_paths` and
   `the_drawing_needs_no_style_and_carries_only_the_code` (Task 7).
4. **Guests or viewers reaching administration** by URL: People, invite, links, expire,
   reactivate, list edits. Expected: 403, nothing written. Tests: `only_residents_manage_people`
   (Task 3), `only_residents_edit_the_lists` (Task 5, contract) and
   `only_residents_see_and_edit_the_lists` (Task 5, web),
   `only_residents_reach_people_and_see_it_in_the_menu` (Task 6), `only_residents_create_links`
   (Task 7).
5. **A page that needs an inline script or style, or a service worker that breaks the app**
   (a precached URL missing, a cookie on assets, posts intercepted). Expected: every page passes
   the CSP scan; the worker installs and never touches posts. Tests:
   `no_page_needs_inline_scripts_styles_or_eval` (extended in Tasks 4, 6, 7, 8),
   `anonymous_requests_set_no_cookies` (Task 8), `everything_the_service_worker_precaches_exists`,
   `the_service_worker_is_served_at_the_root`, `the_manifest_has_what_installing_needs` (Task 8).

## Out of scope (recorded so nobody hunts for it)

- **History shows raw event ids** ("event 3 → 5", deferred M4 UX): it lives on the report page,
  which this milestone does not touch. Still deferred.
- **Settings → Guides → How to log**: the guide pages are milestone 6; Settings links the
  existing *How to rate* page only.
- **Deferred M4b minors** (`smoke-tracker-m4/.superpowers/sdd/04b-room-levels/final-review.md`,
  "can wait"): taken because their files change anyway — **M-1** (the `:has()` selector split
  in `app.css`, Task 4) and **T3-f** (the CSP page list now covers the backdated no-smell
  question and a check's page, Task 4). Not taken (files untouched): T1-a, T1-b/M-4, T2-a, T2-d,
  T3-a, T3-b, T3-c, T3-d.
- Changing an account's role, renaming list items, offline logging (spec §14), push reminders.

## File Structure

```
smoke-tracker/
├─ Cargo.toml, Cargo.lock                 + qrcodegen 1.8.0 (Task 7)
├─ docs/DECISIONS.md, plans/ROADMAP.md    + rows #111–#121; roadmap links this plan
└─ crates/
   ├─ domain/src/
   │  ├─ types.rs      InviteId, InvitePurpose, TokenHash
   │  ├─ records.rs    Invite (LIFETIME, is_usable), NewInvite
   │  ├─ repos.rs      InviteRepo, InviteWrites, UserWrites::user, Conflict::InviteUsed;
   │  │                save_session is update-only
   │  ├─ people.rs     NEW  account use cases + ExpiryPick
   │  └─ lists.rs      NEW  list use cases
   ├─ storage-sqlite/src/, storage-postgres/src/
   │  ├─ invites.rs    NEW  InviteRepo + InviteWrites
   │  ├─ sessions.rs   save_session: UPDATE only
   │  ├─ users.rs      UserWrites::user
   │  └─ .sqlx/        regenerated (Tasks 1, 2)
   ├─ storage-contract/src/
   │  ├─ invites.rs, people.rs   NEW
   │  └─ sessions.rs, lists.rs   race test; list use cases
   ├─ app/src/commands/serve.rs  daily invite sweep
   └─ web/
      ├─ src/header.rs           NEW  SignedIn: what the header shows
      ├─ src/qr.rs               NEW  QR code as inline SVG
      ├─ src/routes/settings.rs  NEW  language, password
      ├─ src/routes/lists.rs     NEW  room/symptom edits (Settings)
      ├─ src/routes/people/      NEW  mod.rs (list, expire, reactivate), expiry.rs, links.rs
      ├─ src/routes/accept.rs    NEW  /invite/{token}
      ├─ src/routes/install.rs   NEW  /install, /offline
      ├─ src/{lib,assets,error,security,i18n}.rs, src/auth/mod.rs, src/routes/*.rs
      ├─ templates/              settings, list_item, people, expiry_fields, invite, link,
      │                          accept, install, offline (NEW); base.html
      ├─ static/                 manifest.webmanifest, sw.js, icon.svg, icon-{180,192,512}.png
      │                          (NEW); app.js, app.css
      ├─ locales/{en,it,tr}/main.ftl   +86 keys each
      └─ tests/                  settings, people, invites, pwa (NEW); sessions, security
```

**Dependency direction** is unchanged: `smoke-tracker → smoke-web → smoke-domain ← storage-*`.

**Why these eight tasks:** the storage boundary first (Tasks 1–2), then the rules on top of it
(Task 3), then one screen per task (Tasks 4–8), each usable and tested on its own: Settings
works before People exists, People before invites, invites before the PWA's welcome page.

---

### Task 1: Sessions — a late save never revives an ended session

Carry-over from milestone 4. `save_session` was an upsert, so a request that loaded its session
before a password change (or an expiry, or a sign-out elsewhere) and renewed it afterwards
(decision #59: once a day) wrote the deleted session back, and the other phone stayed signed in.
`save_session` becomes an `UPDATE … WHERE id_hash = …` on both backends; tower-sessions creates
new sessions through `create`, so nothing needs the insert (decision #111). A contract test
replays the race step by step on SQLite and Postgres, and a web test does the same through
`DbSessions`, the tower-sessions store.

**Files:**
- Modify: `crates/domain/src/repos.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/sessions.rs`, `crates/storage-postgres/src/sessions.rs`, `crates/storage-sqlite/src/sessions.rs`, `crates/web/src/sessions.rs`, `crates/web/tests/sessions.rs`, `docs/DECISIONS.md`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/` (`scripts/sqlx-prepare.sh`)

**Interfaces:**
- Consumes: `SessionRepo`, `UserWrites::set_password` (deletes the user's sessions, #58),
  `smoke_web::DbSessions` (milestone 3).
- Produces: `SessionRepo::save_session(&self, &StoredSession)` — updates the stored session,
  **does nothing and returns `Ok(())` when it no longer exists**. Contract tests
  `sessions::saving_a_session_updates_it_but_never_recreates_it` (replaces
  `saving_a_session_inserts_or_replaces_it`) and
  `sessions::a_renewal_racing_a_password_change_cannot_revive_the_session`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index 8491e14..cdd7908 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -90,10 +90,11 @@ macro_rules! contract_tests {
             sessions::a_session_round_trips_until_it_expires,
             sessions::creating_a_session_twice_is_a_duplicate,
             sessions::a_session_for_an_unknown_user_is_refused,
-            sessions::saving_a_session_inserts_or_replaces_it,
+            sessions::saving_a_session_updates_it_but_never_recreates_it,
             sessions::deleting_a_session_removes_only_that_one,
             sessions::expired_sessions_are_swept,
             sessions::a_password_change_ends_the_users_sessions,
+            sessions::a_renewal_racing_a_password_change_cannot_revive_the_session,
             sessions::expiring_a_user_ends_their_sessions,
             sessions::a_rolled_back_password_change_keeps_the_sessions,
             sessions::sessions_are_not_audited,
diff --git a/crates/storage-contract/src/sessions.rs b/crates/storage-contract/src/sessions.rs
index 6dc9e73..fbfbc7b 100644
--- a/crates/storage-contract/src/sessions.rs
+++ b/crates/storage-contract/src/sessions.rs
@@ -71,24 +71,24 @@ pub async fn a_session_for_an_unknown_user_is_refused<S: SessionRepo>(fx: &Fixtu
     );
 }
 
-pub async fn saving_a_session_inserts_or_replaces_it<
+pub async fn saving_a_session_updates_it_but_never_recreates_it<
     S: SessionRepo + Transactional<Tx: UserWrites>,
 >(
     fx: &Fixture<S>,
 ) {
     let user = add_user(fx, "daniele", Role::Resident).await;
     let anonymous = session("hash-a", None, start() + TimeDelta::days(1));
-    fx.store.save_session(&anonymous).await.unwrap();
-    assert_eq!(
-        fx.store.session("hash-a", start()).await.unwrap(),
-        Some(anonymous)
-    );
+    fx.store.create_session(&anonymous).await.unwrap();
     let signed_in = session("hash-a", Some(user.id), in_30_days());
     fx.store.save_session(&signed_in).await.unwrap();
     assert_eq!(
         fx.store.session("hash-a", start()).await.unwrap(),
-        Some(signed_in)
+        Some(signed_in.clone())
     );
+
+    fx.store.delete_session("hash-a").await.unwrap();
+    fx.store.save_session(&signed_in).await.unwrap();
+    assert_eq!(fx.store.session("hash-a", start()).await.unwrap(), None);
 }
 
 pub async fn deleting_a_session_removes_only_that_one<S: SessionRepo>(fx: &Fixture<S>) {
@@ -155,6 +155,34 @@ pub async fn a_password_change_ends_the_users_sessions<
     }
 }
 
+/// A request loads its session, a password change on another device ends every session of
+/// the user, then the first request renews and saves the session it loaded: it must stay ended.
+pub async fn a_renewal_racing_a_password_change_cannot_revive_the_session<
+    S: SessionRepo + Transactional<Tx: UserWrites>,
+>(
+    fx: &Fixture<S>,
+) {
+    let user = add_user(fx, "daniele", Role::Resident).await;
+    fx.store
+        .create_session(&session("phone", Some(user.id), in_30_days()))
+        .await
+        .unwrap();
+    let loaded = fx.store.session("phone", start()).await.unwrap().unwrap();
+
+    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
+    tx.set_password(user.id, PasswordHash::new("new-hash".to_owned()))
+        .await
+        .unwrap();
+    tx.commit().await.unwrap();
+
+    let renewed = StoredSession {
+        expires_at: in_30_days() + TimeDelta::days(1),
+        ..loaded
+    };
+    fx.store.save_session(&renewed).await.unwrap();
+    assert_eq!(fx.store.session("phone", start()).await.unwrap(), None);
+}
+
 pub async fn expiring_a_user_ends_their_sessions<S: SessionRepo + Transactional<Tx: UserWrites>>(
     fx: &Fixture<S>,
 ) {
diff --git a/crates/web/tests/sessions.rs b/crates/web/tests/sessions.rs
index a260582..165a2b4 100644
--- a/crates/web/tests/sessions.rs
+++ b/crates/web/tests/sessions.rs
@@ -1,15 +1,18 @@
-//! Expired sessions are removed through tower-sessions' `ExpiredDeletion` (spec §11).
+//! Expired sessions are removed through tower-sessions' `ExpiredDeletion` (spec §11), and a
+//! session ended by a password change stays ended.
 
 mod common;
 
 use chrono::{DateTime, TimeDelta, Utc};
 use common::*;
 use smoke_domain::clock::Clock;
-use smoke_domain::records::StoredSession;
-use smoke_domain::repos::SessionRepo;
+use smoke_domain::records::{PasswordHash, StoredSession};
+use smoke_domain::repos::{Commit, SessionRepo, Transactional, UserWrites};
+use smoke_domain::types::{Locale, Role};
 use smoke_web::DbSessions;
 use sqlx::SqlitePool;
-use tower_sessions::session_store::ExpiredDeletion;
+use tower_sessions::session::Id;
+use tower_sessions::session_store::{ExpiredDeletion, SessionStore};
 
 fn session(id_hash: &str, expires_at: DateTime<Utc>) -> StoredSession {
     StoredSession {
@@ -54,3 +57,24 @@ async fn delete_expired_removes_only_expired_sessions() {
         "running again is harmless"
     );
 }
+
+#[tokio::test]
+async fn a_late_save_cannot_revive_a_session_a_password_change_ended() {
+    let app = TestApp::new().await;
+    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let id: Id = cookie.split_once('=').unwrap().1.parse().unwrap();
+    let sessions = DbSessions::new(app.store.clone(), app.clock.clone());
+    let loaded = sessions.load(&id).await.unwrap().expect("signed in");
+
+    let mut tx = app.store.begin(Some(user.id)).await.unwrap();
+    tx.set_password(user.id, PasswordHash::new("new-hash".to_owned()))
+        .await
+        .unwrap();
+    tx.commit().await.unwrap();
+
+    sessions.save(&loaded).await.unwrap();
+    assert_eq!(sessions.load(&id).await.unwrap(), None);
+    let page = app.send(get_with("/", &cookie)).await;
+    assert_eq!(location(&page), Some("/login"));
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: everything compiles; `a_renewal_racing_a_password_change_cannot_revive_the_session` and
`saving_a_session_updates_it_but_never_recreates_it` fail on both backends (`assertion left ==
right` — the saved session came back), and
`a_late_save_cannot_revive_a_session_a_password_change_ended` fails in `crates/web/tests/sessions.rs`.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/domain/src/repos.rs b/crates/domain/src/repos.rs
index c4715be..37e38f1 100644
--- a/crates/domain/src/repos.rs
+++ b/crates/domain/src/repos.rs
@@ -333,7 +333,9 @@ pub trait SessionRepo: Send + Sync {
         &self,
         session: &StoredSession,
     ) -> impl Future<Output = StoreResult<()>> + Send;
-    /// Inserts the session or replaces the stored one.
+    /// Updates a stored session. A session that no longer exists (ended by sign-out, a
+    /// password change or expiry) stays ended: a request that loaded it earlier and saves it
+    /// late never brings it back.
     fn save_session(&self, session: &StoredSession)
     -> impl Future<Output = StoreResult<()>> + Send;
     fn delete_session(&self, id_hash: &str) -> impl Future<Output = StoreResult<()>> + Send;
diff --git a/crates/storage-postgres/src/sessions.rs b/crates/storage-postgres/src/sessions.rs
index 78a2696..c07c1fd 100644
--- a/crates/storage-postgres/src/sessions.rs
+++ b/crates/storage-postgres/src/sessions.rs
@@ -74,13 +74,11 @@ impl SessionRepo for PostgresStore {
 
     async fn save_session(&self, session: &StoredSession) -> StoreResult<()> {
         sqlx::query!(
-            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4) \
-             ON CONFLICT (id_hash) DO UPDATE SET user_id = excluded.user_id, \
-             data = excluded.data, expires_at = excluded.expires_at",
-            session.id_hash,
+            "UPDATE sessions SET user_id = $1, data = $2, expires_at = $3 WHERE id_hash = $4",
             session.user_id.map(|u| u.0),
             session.data,
             micros(session.expires_at),
+            session.id_hash,
         )
         .execute(&mut *self.write().await?)
         .await
diff --git a/crates/storage-sqlite/src/sessions.rs b/crates/storage-sqlite/src/sessions.rs
index d2142e1..69c7a06 100644
--- a/crates/storage-sqlite/src/sessions.rs
+++ b/crates/storage-sqlite/src/sessions.rs
@@ -74,13 +74,11 @@ impl SessionRepo for SqliteStore {
 
     async fn save_session(&self, session: &StoredSession) -> StoreResult<()> {
         sqlx::query!(
-            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4) \
-             ON CONFLICT (id_hash) DO UPDATE SET user_id = excluded.user_id, \
-             data = excluded.data, expires_at = excluded.expires_at",
-            session.id_hash,
+            "UPDATE sessions SET user_id = $1, data = $2, expires_at = $3 WHERE id_hash = $4",
             session.user_id.map(|u| u.0),
             session.data,
             micros(session.expires_at),
+            session.id_hash,
         )
         .execute(&mut *self.write().await?)
         .await
diff --git a/crates/web/src/sessions.rs b/crates/web/src/sessions.rs
index 47eb3fd..a009ceb 100644
--- a/crates/web/src/sessions.rs
+++ b/crates/web/src/sessions.rs
@@ -130,6 +130,8 @@ where
         }
     }
 
+    /// Never recreates a session ended meanwhile (decision #111): the request's response still
+    /// sets the cookie, but the next request finds no session and is signed out.
     async fn save(&self, record: &Record) -> session_store::Result<()> {
         self.store
             .save_session(&to_stored(record)?)
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index bf1e14b..8737c71 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -117,3 +117,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 108 | 2026-10-13 | UI | Summary card: **reports** = reports in an event, **clean checks** = checks outside any event, **days with bedroom smoke** = days with the bedroom at level 1 or more; the Events list shows each day's checks, which guests see only for themselves | Count every no-smell report as a check | The two counts never overlap; a clean bedroom is not a bedroom-smoke day |
 | 109 | 2026-10-13 | Storage | Migration 0004 makes `smell_reports.event_id` **nullable**: SQLite **rebuilds** `smell_reports` and both link tables inside the migration's transaction (copy, drop, rename) with foreign keys on; Postgres drops `NOT NULL` | `PRAGMA foreign_keys = OFF` in a no-transaction migration; a sentinel "no event" event | Atomic, no switched-off checks, no sentinel rows to filter out everywhere |
 | 110 | 2026-10-13 | Events | An event whose only smell report is **corrected to 0** (or moved out) **is kept**, with intensity 0 and **still counted** in the summary; confirmed by the user. Refines #105: #105 rejects *opening* an event at zero intensity, a correction afterwards is allowed (#106) | Delete the event; hide it from the list and the summary | A correction records what was true; the event still happened and the log stays complete |
+| 111 | 2026-10-14 | Sessions | `save_session` **only updates** a stored session (`UPDATE … WHERE id_hash`); a session ended meanwhile by sign-out, a password change or expiry is never written back. Replaces the upsert of #56 | Keep the upsert and re-check the user's password version on every request | A request that loaded its session before a password change and renewed it afterwards (#59) brought the ended session back, so the change did not sign out the other phone; tower-sessions creates new sessions through `create`, so nothing needs the insert |
```

Then regenerate the query metadata (the `UPDATE` replaces the `INSERT … ON CONFLICT` query, so
one `.sqlx/query-*.json` file per backend is replaced by another):

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
Expected: no formatting changes, no clippy warnings; every test passes, including the three named in
Step 2 and the unchanged `a_password_change_ends_the_users_sessions`. With `TEST_POSTGRES_URL`
set: **493** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/repos.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/sessions.rs \
  crates/storage-postgres/src/sessions.rs \
  crates/storage-sqlite/src/sessions.rs \
  crates/web/src/sessions.rs \
  crates/web/tests/sessions.rs \
  docs/DECISIONS.md \
  crates/storage-postgres/.sqlx \
  crates/storage-sqlite/.sqlx
git commit -m "fix(sessions): update-only save so a late renewal never revives an ended session

\`save_session\` was an upsert: a request that loaded its session before a password change
and renewed it afterwards wrote the ended session back. It is now an UPDATE; new sessions
still come through \`create\`. Contract and web tests pin the race.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Invites stored by token hash, cleaned up daily

The `invites` table has existed since migration 0001; this task gives it a domain record and a
place behind the storage boundary. A link's token never reaches the database: `TokenHash` is
its SHA-256 in hex, and the `Invite` record (and so its audit snapshot) leaves even the hash
out. Creating an invite is audited as `invite`, using it as an `update` that sets `used_at`
(using it twice is `Conflict::InviteUsed`). A new invite for a person removes their earlier
unused ones, so only the newest QR code works. `serve` deletes expired invites at startup and
every 24 hours, without audit entries: invites are not evidence (`has_snapshot` is false, so
`verify-audit` stays clean). Decision #112.

**Files:**
- Create: `crates/storage-contract/src/invites.rs`, `crates/storage-postgres/src/invites.rs`, `crates/storage-sqlite/src/invites.rs`
- Modify: `crates/app/src/commands/serve.rs`, `crates/domain/src/records.rs`, `crates/domain/src/repos.rs`, `crates/domain/src/types.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-postgres/src/lib.rs`, `crates/storage-sqlite/src/lib.rs`, `docs/DECISIONS.md`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/` (`scripts/sqlx-prepare.sh`)

**Interfaces:**
- Consumes: `Audited`, `AuditAction::Invite`, `AuditEntity::Invite` (milestone 1);
  `SqliteTx`/`PostgresTx::record`, `convert::{instant, micros, opt_instant, parse}` (milestone 2).
- Produces (in `smoke_domain`):
  - `types::InviteId(pub i64)`; `types::InvitePurpose { Setup => "setup", PasswordReset =>
    "password_reset" }` (a `str_enum`); `types::TokenHash::of(token: &str) -> TokenHash`,
    `as_str(&self) -> &str` (Debug shows only 8 hex characters).
  - `records::Invite { id, user_id, purpose, created_by, created_at, expires_at, used_at:
    Option<DateTime<Utc>> }`, `Invite::LIFETIME: TimeDelta` (5 minutes),
    `Invite::is_usable(&self, now) -> bool` (unused and `now < expires_at`);
    `records::NewInvite { user_id, purpose, token_hash: TokenHash, created_by: UserId,
    expires_at }`.
  - `repos::InviteRepo { invite_by_token(&self, &TokenHash) -> Option<Invite>,
    delete_expired_invites(&self, now) -> u64 }`; `repos::InviteWrites { create_invite(&mut
    self, NewInvite) -> Invite, invite_by_token(&mut self, &TokenHash) -> Option<Invite>,
    use_invite(&mut self, InviteId) -> Invite }`; `Conflict::InviteUsed`. `Store` includes
    `InviteRepo`, `WriteTx` includes `InviteWrites`.
- Produces (app): `commands::serve::sweep_invites` (private), started with the other loops.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/invites.rs b/crates/storage-contract/src/invites.rs
new file mode 100644
index 0000000..ca063cd
--- /dev/null
+++ b/crates/storage-contract/src/invites.rs
@@ -0,0 +1,161 @@
+//! Invites: stored by token hash, used once, replaced by a newer link, cleaned up daily.
+
+use chrono::TimeDelta;
+use smoke_domain::audit::{AuditAction, AuditEntity};
+use smoke_domain::clock::Clock;
+use smoke_domain::records::{Audited, Invite, NewInvite, User};
+use smoke_domain::repos::{Commit, Conflict, InviteWrites, Store, StoreError, Transactional};
+use smoke_domain::types::{InvitePurpose, Role, TokenHash, UserId};
+use smoke_domain::verify::verify_store;
+
+use crate::Fixture;
+use crate::fixture::{add_user, last_entry, start};
+
+fn new_invite(user: &User, by: &User, token: &str) -> NewInvite {
+    NewInvite {
+        user_id: user.id,
+        purpose: InvitePurpose::Setup,
+        token_hash: TokenHash::of(token),
+        created_by: by.id,
+        expires_at: start() + Invite::LIFETIME,
+    }
+}
+
+async fn add_invite<S: Transactional<Tx: InviteWrites>>(fx: &Fixture<S>, new: NewInvite) -> Invite {
+    let mut tx = fx.store.begin(Some(new.created_by)).await.unwrap();
+    let invite = tx.create_invite(new).await.unwrap();
+    tx.commit().await.unwrap();
+    invite
+}
+
+pub async fn an_invite_is_found_by_its_token_hash_only<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let invite = add_invite(fx, new_invite(&guest, &resident, "token-a")).await;
+    assert_eq!(
+        (invite.user_id, invite.created_by, invite.purpose),
+        (guest.id, resident.id, InvitePurpose::Setup)
+    );
+    assert_eq!(
+        (invite.created_at, invite.expires_at, invite.used_at),
+        (start(), start() + Invite::LIFETIME, None)
+    );
+    assert_eq!(
+        fx.store
+            .invite_by_token(&TokenHash::of("token-a"))
+            .await
+            .unwrap(),
+        Some(invite.clone())
+    );
+    assert_eq!(
+        fx.store
+            .invite_by_token(&TokenHash::of("token-b"))
+            .await
+            .unwrap(),
+        None
+    );
+
+    let entry = last_entry(fx).await.record;
+    assert_eq!(
+        (entry.entity, entry.entity_id, entry.action),
+        (AuditEntity::Invite, invite.id.0, AuditAction::Invite)
+    );
+    assert_eq!(entry.after, Some(invite.snapshot()));
+    let log = format!("{:?}", fx.store.audit_log().await.unwrap());
+    assert!(!log.contains(TokenHash::of("token-a").as_str()));
+}
+
+pub async fn an_invite_is_used_once_and_that_is_audited<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let invite = add_invite(fx, new_invite(&guest, &resident, "token-a")).await;
+    fx.clock.advance(TimeDelta::minutes(1));
+
+    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
+    let found = tx.invite_by_token(&TokenHash::of("token-a")).await.unwrap();
+    assert_eq!(found, Some(invite.clone()));
+    let used = tx.use_invite(invite.id).await.unwrap();
+    tx.commit().await.unwrap();
+    assert_eq!(used.used_at, Some(fx.clock.now()));
+    let entry = last_entry(fx).await.record;
+    assert_eq!(
+        (entry.entity, entry.action, entry.actor_id),
+        (AuditEntity::Invite, AuditAction::Update, Some(guest.id))
+    );
+    assert_eq!(entry.before, Some(invite.snapshot()));
+
+    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
+    let error = tx.use_invite(invite.id).await.unwrap_err();
+    assert!(
+        matches!(error, StoreError::Conflict(Conflict::InviteUsed)),
+        "{error:?}"
+    );
+}
+
+pub async fn a_new_invite_replaces_the_users_unused_ones<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    let used = add_invite(fx, new_invite(&guest, &resident, "used")).await;
+    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
+    tx.use_invite(used.id).await.unwrap();
+    tx.commit().await.unwrap();
+    add_invite(fx, new_invite(&guest, &resident, "first")).await;
+    add_invite(fx, new_invite(&viewer, &resident, "other-user")).await;
+
+    add_invite(fx, new_invite(&guest, &resident, "second")).await;
+    for (token, kept) in [
+        ("used", true),
+        ("first", false),
+        ("other-user", true),
+        ("second", true),
+    ] {
+        let found = fx
+            .store
+            .invite_by_token(&TokenHash::of(token))
+            .await
+            .unwrap();
+        assert_eq!(found.is_some(), kept, "{token}");
+    }
+    assert!(verify_store(&fx.store).await.unwrap().is_ok());
+}
+
+pub async fn an_invite_for_an_unknown_user_is_refused<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let mut ghost = resident.clone();
+    ghost.id = UserId(999);
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let error = tx
+        .create_invite(new_invite(&ghost, &resident, "token-a"))
+        .await
+        .unwrap_err();
+    assert!(
+        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
+        "{error:?}"
+    );
+}
+
+pub async fn expired_invites_are_removed_without_an_audit_entry<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    add_invite(fx, new_invite(&guest, &resident, "old")).await;
+    fx.clock.advance(TimeDelta::minutes(1));
+    let mut fresh = new_invite(&viewer, &resident, "fresh");
+    fresh.expires_at = fx.clock.now() + Invite::LIFETIME;
+    add_invite(fx, fresh).await;
+    let entries = fx.store.audit_log().await.unwrap().len();
+
+    let at_expiry = start() + Invite::LIFETIME;
+    assert_eq!(fx.store.delete_expired_invites(at_expiry).await.unwrap(), 1);
+    assert_eq!(fx.store.delete_expired_invites(at_expiry).await.unwrap(), 0);
+    assert!(
+        fx.store
+            .invite_by_token(&TokenHash::of("fresh"))
+            .await
+            .unwrap()
+            .is_some()
+    );
+    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
+    assert!(verify_store(&fx.store).await.unwrap().is_ok());
+}
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index cdd7908..b45722b 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -12,6 +12,7 @@ pub mod audit;
 pub mod corrections;
 pub mod events;
 pub mod fixture;
+pub mod invites;
 pub mod lists;
 pub mod logbook;
 pub mod migrations;
@@ -98,6 +99,11 @@ macro_rules! contract_tests {
             sessions::expiring_a_user_ends_their_sessions,
             sessions::a_rolled_back_password_change_keeps_the_sessions,
             sessions::sessions_are_not_audited,
+            invites::an_invite_is_found_by_its_token_hash_only,
+            invites::an_invite_is_used_once_and_that_is_audited,
+            invites::a_new_invite_replaces_the_users_unused_ones,
+            invites::an_invite_for_an_unknown_user_is_refused,
+            invites::expired_invites_are_removed_without_an_audit_entry,
             logbook::logging_now_with_nothing_open_starts_an_open_event,
             logbook::same_smell_joins_the_open_event,
             logbook::a_new_event_closes_the_open_one_at_its_last_report,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: compile errors — `InviteWrites`, `InviteRepo`, `NewInvite`, `Invite`, `InvitePurpose` and
`TokenHash` do not exist.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/app/src/commands/serve.rs b/crates/app/src/commands/serve.rs
index 297caf3..4a891f8 100644
--- a/crates/app/src/commands/serve.rs
+++ b/crates/app/src/commands/serve.rs
@@ -5,7 +5,7 @@ use std::time::Duration;
 
 use anyhow::Context;
 use smoke_domain::clock::Clock;
-use smoke_domain::repos::{SessionRepo, Store};
+use smoke_domain::repos::{InviteRepo, SessionRepo, Store};
 use smoke_domain::schedule::DailySchedule;
 use smoke_domain::sweep::{SweepOutcome, auto_close};
 use smoke_web::{AppState, DbSessions};
@@ -18,6 +18,9 @@ use crate::config::ServeConfig;
 /// How often expired sessions are deleted.
 const SESSION_SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);
 
+/// How often expired invites are deleted (spec §11: daily).
+const INVITE_SWEEP_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
+
 /// How often the auto-close sweep checks whether a 04:00 boundary has passed. A check that
 /// finds nothing to do is one small read.
 const EVENT_SWEEP_EVERY: Duration = Duration::from_secs(5 * 60);
@@ -46,10 +49,12 @@ pub async fn run<S: Store + Clone + 'static>(
         clock.clone(),
         config.web.schedule,
     ));
+    let invites = tokio::spawn(sweep_invites(store.clone(), clock.clone()));
     let app = smoke_web::router(AppState::new(store, clock, config.web));
     let served = smoke_web::serve(listener, app, shutdown_signal()).await;
     sweeper.abort();
     closer.abort();
+    invites.abort();
     served.context("the server stopped with an error")
 }
 
@@ -79,6 +84,20 @@ async fn sweep_sessions<S: SessionRepo + Clone + 'static>(sessions: DbSessions<S
     }
 }
 
+/// Deletes expired invites at startup and then daily. Cloud Run may sleep the app for days;
+/// an expired link is refused anyway, so a late run only tidies up.
+async fn sweep_invites<S: InviteRepo>(store: S, clock: Arc<dyn Clock>) {
+    let mut ticks = tokio::time::interval(INVITE_SWEEP_EVERY);
+    loop {
+        ticks.tick().await;
+        match store.delete_expired_invites(clock.now()).await {
+            Ok(0) => {}
+            Ok(removed) => info!(removed, "deleted expired invites"),
+            Err(error) => warn!(%error, "could not delete expired invites"),
+        }
+    }
+}
+
 /// Resolves on Ctrl-C, or on SIGTERM (what `docker stop` sends).
 async fn shutdown_signal() {
     let ctrl_c = async {
@@ -109,23 +128,24 @@ async fn shutdown_signal() {
 mod tests {
     use chrono::{DateTime, TimeDelta, Utc};
     use smoke_domain::clock::FakeClock;
-    use smoke_domain::records::{NewEvent, NewUser};
-    use smoke_domain::repos::{Commit, EventRepo, EventWrites, Transactional, UserWrites};
-    use smoke_domain::types::{CloseReason, Locale, Role, Username};
+    use smoke_domain::records::{Invite, NewEvent, NewInvite, NewUser, User};
+    use smoke_domain::repos::{
+        Commit, EventRepo, EventWrites, InviteWrites, Transactional, UserWrites,
+    };
+    use smoke_domain::types::{CloseReason, InvitePurpose, Locale, Role, TokenHash, Username};
     use smoke_storage_sqlite::SqliteStore;
 
     use super::*;
 
-    #[tokio::test]
-    async fn the_event_sweep_runs_as_soon_as_the_server_starts() {
-        let dir = tempfile::tempdir().unwrap();
+    async fn store_with_resident(
+        dir: &tempfile::TempDir,
+        clock: Arc<FakeClock>,
+    ) -> (SqliteStore, User) {
         let url = format!("sqlite://{}", dir.path().join("serve.db").display());
-        let evening: DateTime<Utc> = "2026-01-15T20:00:00Z".parse().unwrap();
-        let clock = Arc::new(FakeClock::new(evening));
-        let store = SqliteStore::connect(&url, clock.clone()).await.unwrap();
+        let store = SqliteStore::connect(&url, clock).await.unwrap();
         store.migrate().await.unwrap();
         let mut tx = store.begin(None).await.unwrap();
-        let owner = tx
+        let resident = tx
             .create_user(NewUser {
                 username: Username::new("daniele").unwrap(),
                 password_hash: None,
@@ -135,6 +155,17 @@ mod tests {
             })
             .await
             .unwrap();
+        tx.commit().await.unwrap();
+        (store, resident)
+    }
+
+    #[tokio::test]
+    async fn the_event_sweep_runs_as_soon_as_the_server_starts() {
+        let dir = tempfile::tempdir().unwrap();
+        let evening: DateTime<Utc> = "2026-01-15T20:00:00Z".parse().unwrap();
+        let clock = Arc::new(FakeClock::new(evening));
+        let (store, owner) = store_with_resident(&dir, clock.clone()).await;
+        let mut tx = store.begin(None).await.unwrap();
         let event = tx
             .create_event(NewEvent {
                 owner_id: owner.id,
@@ -159,4 +190,37 @@ mod tests {
         task.abort();
         assert_eq!(closed.map(|c| c.reason), Some(CloseReason::Auto0400));
     }
+
+    #[tokio::test]
+    async fn the_invite_sweep_runs_as_soon_as_the_server_starts() {
+        let dir = tempfile::tempdir().unwrap();
+        let evening: DateTime<Utc> = "2026-01-15T20:00:00Z".parse().unwrap();
+        let clock = Arc::new(FakeClock::new(evening));
+        let (store, resident) = store_with_resident(&dir, clock.clone()).await;
+        let token = TokenHash::of("token");
+        let mut tx = store.begin(Some(resident.id)).await.unwrap();
+        tx.create_invite(NewInvite {
+            user_id: resident.id,
+            purpose: InvitePurpose::PasswordReset,
+            token_hash: token.clone(),
+            created_by: resident.id,
+            expires_at: evening + Invite::LIFETIME,
+        })
+        .await
+        .unwrap();
+        tx.commit().await.unwrap();
+        clock.advance(TimeDelta::hours(1));
+
+        let task = tokio::spawn(sweep_invites(store.clone(), clock));
+        let mut found = Some(());
+        for _ in 0..100 {
+            found = store.invite_by_token(&token).await.unwrap().map(|_| ());
+            if found.is_none() {
+                break;
+            }
+            tokio::time::sleep(Duration::from_millis(20)).await;
+        }
+        task.abort();
+        assert_eq!(found, None);
+    }
 }
diff --git a/crates/domain/src/records.rs b/crates/domain/src/records.rs
index d1d6c49..7a00324 100644
--- a/crates/domain/src/records.rs
+++ b/crates/domain/src/records.rs
@@ -4,7 +4,7 @@
 use std::borrow::Cow;
 use std::fmt;
 
-use chrono::{DateTime, Utc};
+use chrono::{DateTime, TimeDelta, Utc};
 use serde::{Deserialize, Serialize, Serializer};
 use serde_json::Value;
 
@@ -13,8 +13,8 @@ use crate::audit::AuditEntity;
 use crate::events::EventSummary;
 use crate::permissions::Actor;
 use crate::types::{
-    CloseReason, EventId, Level, ListName, Locale, Pm25, ReadingId, ReportId, Role, RoomArea,
-    RoomId, SymptomId, UserId, Username,
+    CloseReason, EventId, InviteId, InvitePurpose, Level, ListName, Locale, Pm25, ReadingId,
+    ReportId, Role, RoomArea, RoomId, SymptomId, TokenHash, UserId, Username,
 };
 
 /// A record the audit log keeps snapshots of.
@@ -86,6 +86,46 @@ impl Audited for User {
     }
 }
 
+/// A one-time link to set up an account or reset its password (spec §8.3). The token's hash is
+/// stored but never part of the record or its audit snapshot.
+#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
+pub struct Invite {
+    pub id: InviteId,
+    pub user_id: UserId,
+    pub purpose: InvitePurpose,
+    pub created_by: UserId,
+    pub created_at: DateTime<Utc>,
+    pub expires_at: DateTime<Utc>,
+    pub used_at: Option<DateTime<Utc>>,
+}
+
+impl Invite {
+    /// How long a link works after it is created.
+    pub const LIFETIME: TimeDelta = TimeDelta::minutes(5);
+
+    /// Unused and not yet expired: from `expires_at` on, the link is dead.
+    pub fn is_usable(&self, now: DateTime<Utc>) -> bool {
+        self.used_at.is_none() && now < self.expires_at
+    }
+}
+
+impl Audited for Invite {
+    const ENTITY: AuditEntity = AuditEntity::Invite;
+
+    fn audit_id(&self) -> i64 {
+        self.id.0
+    }
+}
+
+#[derive(Debug, Clone, PartialEq)]
+pub struct NewInvite {
+    pub user_id: UserId,
+    pub purpose: InvitePurpose,
+    pub token_hash: TokenHash,
+    pub created_by: UserId,
+    pub expires_at: DateTime<Utc>,
+}
+
 #[derive(Debug, Clone, PartialEq, Serialize)]
 pub struct Room {
     pub id: RoomId,
@@ -653,6 +693,37 @@ mod tests {
         assert_eq!(Event::ENTITY, AuditEntity::Event);
         assert_eq!(SmellReport::ENTITY, AuditEntity::SmellReport);
         assert_eq!(Pm25Reading::ENTITY, AuditEntity::Pm25Reading);
+        assert_eq!(Invite::ENTITY, AuditEntity::Invite);
+    }
+
+    #[test]
+    fn an_invite_works_once_and_for_five_minutes() {
+        let created_at = t("2026-01-15T20:00:00Z");
+        let invite = Invite {
+            id: InviteId(1),
+            user_id: UserId(2),
+            purpose: InvitePurpose::Setup,
+            created_by: UserId(1),
+            created_at,
+            expires_at: created_at + Invite::LIFETIME,
+            used_at: None,
+        };
+        assert!(invite.is_usable(created_at));
+        assert!(invite.is_usable(created_at + TimeDelta::seconds(299)));
+        assert!(!invite.is_usable(created_at + Invite::LIFETIME));
+        let used = Invite {
+            used_at: Some(created_at),
+            ..invite.clone()
+        };
+        assert!(!used.is_usable(created_at));
+        assert_eq!(
+            invite.snapshot(),
+            json!({
+                "id": 1, "user_id": 2, "purpose": "setup", "created_by": 1,
+                "created_at": "2026-01-15T20:00:00Z", "expires_at": "2026-01-15T20:05:00Z",
+                "used_at": null
+            })
+        );
     }
 
     #[test]
diff --git a/crates/domain/src/repos.rs b/crates/domain/src/repos.rs
index 37e38f1..9e03086 100644
--- a/crates/domain/src/repos.rs
+++ b/crates/domain/src/repos.rs
@@ -13,11 +13,13 @@ use thiserror::Error;
 use crate::DomainError;
 use crate::audit::{AuditEntity, ChainedEntry};
 use crate::records::{
-    Closure, Event, NewEvent, NewReading, NewReport, NewRoom, NewSymptom, NewUser, PasswordHash,
-    Pm25Reading, ReportValues, Room, SmellReport, StoredSession, Symptom, User,
+    Closure, Event, Invite, NewEvent, NewInvite, NewReading, NewReport, NewRoom, NewSymptom,
+    NewUser, PasswordHash, Pm25Reading, ReportValues, Room, SmellReport, StoredSession, Symptom,
+    User,
 };
 use crate::types::{
-    EventId, Locale, Pm25, ReadingId, ReportId, RoomId, SymptomId, UserId, Username,
+    EventId, InviteId, Locale, Pm25, ReadingId, ReportId, RoomId, SymptomId, TokenHash, UserId,
+    Username,
 };
 use crate::verify::AuditState;
 
@@ -42,6 +44,8 @@ pub enum Conflict {
     ChainForked,
     #[error("the change breaks a data rule")]
     RuleViolated,
+    #[error("the invite has already been used")]
+    InviteUsed,
 }
 
 #[derive(Debug, Error)]
@@ -88,6 +92,21 @@ pub trait UserRepo: Send + Sync {
     fn users(&self) -> impl Future<Output = StoreResult<Vec<User>>> + Send;
 }
 
+pub trait InviteRepo: Send + Sync {
+    /// The invite whose token has this hash, used or expired ones included.
+    fn invite_by_token(
+        &self,
+        token: &TokenHash,
+    ) -> impl Future<Output = StoreResult<Option<Invite>>> + Send;
+    /// The daily cleanup (spec §11): removes every invite that expired by `now`, used or not,
+    /// and returns how many there were. Not audited: invites are not evidence and have no
+    /// snapshot (decision #112).
+    fn delete_expired_invites(
+        &self,
+        now: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<u64>> + Send;
+}
+
 pub trait ListRepo: Send + Sync {
     /// Every room, hidden ones included, ordered by id.
     fn rooms(&self) -> impl Future<Output = StoreResult<Vec<Room>>> + Send;
@@ -189,6 +208,20 @@ pub trait UserWrites: Send {
     ) -> impl Future<Output = StoreResult<User>> + Send;
 }
 
+pub trait InviteWrites: Send {
+    /// Audited as `invite`. Removes the user's earlier unused invites, so only the newest link
+    /// works (decision #112). `created_at` is now.
+    fn create_invite(&mut self, new: NewInvite)
+    -> impl Future<Output = StoreResult<Invite>> + Send;
+    /// Reads inside the transaction, so checking an invite and using it see the same data.
+    fn invite_by_token(
+        &mut self,
+        token: &TokenHash,
+    ) -> impl Future<Output = StoreResult<Option<Invite>>> + Send;
+    /// Sets `used_at` to now. Fails with [`Conflict::InviteUsed`] if it is already set.
+    fn use_invite(&mut self, id: InviteId) -> impl Future<Output = StoreResult<Invite>> + Send;
+}
+
 pub trait ListWrites: Send {
     fn create_room(&mut self, new: NewRoom) -> impl Future<Output = StoreResult<Room>> + Send;
     /// Hiding keeps the first `hidden_at`; unhiding clears it.
@@ -301,12 +334,26 @@ pub trait JobWrites: Send {
 
 /// Everything a write transaction can do.
 pub trait WriteTx:
-    Commit + UserWrites + ListWrites + EventWrites + ReportWrites + ReadingWrites + JobWrites
+    Commit
+    + UserWrites
+    + InviteWrites
+    + ListWrites
+    + EventWrites
+    + ReportWrites
+    + ReadingWrites
+    + JobWrites
 {
 }
 
 impl<T> WriteTx for T where
-    T: Commit + UserWrites + ListWrites + EventWrites + ReportWrites + ReadingWrites + JobWrites
+    T: Commit
+        + UserWrites
+        + InviteWrites
+        + ListWrites
+        + EventWrites
+        + ReportWrites
+        + ReadingWrites
+        + JobWrites
 {
 }
 
@@ -350,6 +397,7 @@ pub trait SessionRepo: Send + Sync {
 pub trait Store:
     Transactional<Tx: WriteTx>
     + UserRepo
+    + InviteRepo
     + ListRepo
     + EventRepo
     + ReportRepo
@@ -364,6 +412,7 @@ pub trait Store:
 impl<T> Store for T where
     T: Transactional<Tx: WriteTx>
         + UserRepo
+        + InviteRepo
         + ListRepo
         + EventRepo
         + ReportRepo
diff --git a/crates/domain/src/types.rs b/crates/domain/src/types.rs
index 13aeae6..5adc568 100644
--- a/crates/domain/src/types.rs
+++ b/crates/domain/src/types.rs
@@ -3,6 +3,7 @@
 use std::fmt;
 
 use serde::{Deserialize, Serialize};
+use sha2::{Digest, Sha256};
 
 use crate::DomainError;
 
@@ -34,6 +35,8 @@ id_type!(
     RoomId,
     /// Identifies a symptom.
     SymptomId,
+    /// Identifies an invite (a one-time setup or password-reset link).
+    InviteId,
 );
 
 /// Defines a field-less enum stored as text: `as_str`, `FromStr`, `Display`, serde, and `ALL`.
@@ -127,6 +130,16 @@ str_enum!(
     }
 );
 
+str_enum!(
+    /// What an invite link lets its holder do (spec §8.3).
+    InvitePurpose {
+        /// Choose the first password of a new account.
+        Setup => "setup",
+        /// Choose a new password for an existing account.
+        PasswordReset => "password_reset",
+    }
+);
+
 str_enum!(
     /// Interface language.
     Locale {
@@ -224,6 +237,27 @@ impl fmt::Debug for NewPassword {
     }
 }
 
+/// The SHA-256 of an invite token, as lowercase hex: the only form of the token the database
+/// keeps (spec §6), so a copy of the database holds no usable link.
+#[derive(Clone, PartialEq, Eq, Hash)]
+pub struct TokenHash(String);
+
+impl TokenHash {
+    pub fn of(token: &str) -> Self {
+        Self(hex::encode(Sha256::digest(token.as_bytes())))
+    }
+
+    pub fn as_str(&self) -> &str {
+        &self.0
+    }
+}
+
+impl fmt::Debug for TokenHash {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        write!(f, "TokenHash({}…)", &self.0[..8])
+    }
+}
+
 /// How strong a smell is in one room, on the personal 0–10 scale: 0 means the room was checked
 /// and smelled clean.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
@@ -357,6 +391,10 @@ mod tests {
         for locale in Locale::ALL {
             assert_eq!(locale.as_str().parse::<Locale>().unwrap(), *locale);
         }
+        for purpose in InvitePurpose::ALL {
+            assert_eq!(purpose.as_str().parse::<InvitePurpose>().unwrap(), *purpose);
+        }
+        assert_eq!(InvitePurpose::PasswordReset.as_str(), "password_reset");
     }
 
     #[test]
@@ -451,6 +489,18 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_token_hash_is_the_sha256_of_the_token_in_hex() {
+        let hash = TokenHash::of("abc");
+        assert_eq!(
+            hash.as_str(),
+            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
+        );
+        assert_eq!(hash, TokenHash::of("abc"));
+        assert_ne!(hash, TokenHash::of("abd"));
+        assert_eq!(format!("{hash:?}"), "TokenHash(ba7816bf…)");
+    }
+
     #[test]
     fn passwords_are_kept_as_typed_and_never_printed() {
         let password = NewPassword::new("  spaced out  ".to_owned()).unwrap();
diff --git a/crates/storage-postgres/src/invites.rs b/crates/storage-postgres/src/invites.rs
new file mode 100644
index 0000000..3117f68
--- /dev/null
+++ b/crates/storage-postgres/src/invites.rs
@@ -0,0 +1,133 @@
+//! Invites: one-time links for account setup and password reset (spec §8.3). Only the token's
+//! SHA-256 is stored, and it is never read back into a record.
+
+use chrono::{DateTime, Utc};
+use smoke_domain::audit::{AuditAction, AuditEntity};
+use smoke_domain::records::{Invite, NewInvite};
+use smoke_domain::repos::{Conflict, InviteRepo, InviteWrites, StoreError, StoreResult};
+use smoke_domain::types::{InviteId, TokenHash, UserId};
+use sqlx::PgConnection;
+
+use crate::convert::{instant, micros, opt_instant, parse};
+use crate::error::IntoStore;
+use crate::{PostgresStore, PostgresTx};
+
+struct InviteRow {
+    id: i64,
+    user_id: i64,
+    purpose: String,
+    created_by: i64,
+    created_at: i64,
+    expires_at: i64,
+    used_at: Option<i64>,
+}
+
+impl InviteRow {
+    fn into_invite(self) -> StoreResult<Invite> {
+        Ok(Invite {
+            id: InviteId(self.id),
+            user_id: UserId(self.user_id),
+            purpose: parse(&self.purpose)?,
+            created_by: UserId(self.created_by),
+            created_at: instant(self.created_at)?,
+            expires_at: instant(self.expires_at)?,
+            used_at: opt_instant(self.used_at)?,
+        })
+    }
+}
+
+async fn require(conn: &mut PgConnection, id: InviteId) -> StoreResult<Invite> {
+    sqlx::query_as!(
+        InviteRow,
+        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
+         FROM invites WHERE id = $1",
+        id.0,
+    )
+    .fetch_optional(conn)
+    .await
+    .db()?
+    .ok_or(StoreError::NotFound {
+        entity: AuditEntity::Invite,
+        id: id.0,
+    })?
+    .into_invite()
+}
+
+async fn by_token(conn: &mut PgConnection, token: &TokenHash) -> StoreResult<Option<Invite>> {
+    sqlx::query_as!(
+        InviteRow,
+        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
+         FROM invites WHERE token_hash = $1",
+        token.as_str(),
+    )
+    .fetch_optional(conn)
+    .await
+    .db()?
+    .map(InviteRow::into_invite)
+    .transpose()
+}
+
+impl InviteRepo for PostgresStore {
+    async fn invite_by_token(&self, token: &TokenHash) -> StoreResult<Option<Invite>> {
+        by_token(&mut *self.read().await?, token).await
+    }
+
+    async fn delete_expired_invites(&self, now: DateTime<Utc>) -> StoreResult<u64> {
+        let done = sqlx::query!("DELETE FROM invites WHERE expires_at <= $1", micros(now))
+            .execute(&mut *self.write().await?)
+            .await
+            .db()?;
+        Ok(done.rows_affected())
+    }
+}
+
+impl InviteWrites for PostgresTx {
+    async fn create_invite(&mut self, new: NewInvite) -> StoreResult<Invite> {
+        sqlx::query!(
+            "DELETE FROM invites WHERE user_id = $1 AND used_at IS NULL",
+            new.user_id.0,
+        )
+        .execute(self.conn())
+        .await
+        .db()?;
+        let id = sqlx::query_scalar!(
+            "INSERT INTO invites (user_id, purpose, token_hash, created_by, created_at, expires_at) \
+             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
+            new.user_id.0,
+            new.purpose.as_str(),
+            new.token_hash.as_str(),
+            new.created_by.0,
+            micros(self.now()),
+            micros(new.expires_at),
+        )
+        .fetch_one(self.conn())
+        .await
+        .db()?;
+        let invite = require(self.conn(), InviteId(id)).await?;
+        self.record(AuditAction::Invite, None, &invite).await?;
+        Ok(invite)
+    }
+
+    async fn invite_by_token(&mut self, token: &TokenHash) -> StoreResult<Option<Invite>> {
+        by_token(self.conn(), token).await
+    }
+
+    async fn use_invite(&mut self, id: InviteId) -> StoreResult<Invite> {
+        let before = require(self.conn(), id).await?;
+        if before.used_at.is_some() {
+            return Err(Conflict::InviteUsed.into());
+        }
+        sqlx::query!(
+            "UPDATE invites SET used_at = $1 WHERE id = $2",
+            micros(self.now()),
+            id.0,
+        )
+        .execute(self.conn())
+        .await
+        .db()?;
+        let after = require(self.conn(), id).await?;
+        self.record(AuditAction::Update, Some(&before), &after)
+            .await?;
+        Ok(after)
+    }
+}
diff --git a/crates/storage-postgres/src/lib.rs b/crates/storage-postgres/src/lib.rs
index 3de4ff8..0c84dce 100644
--- a/crates/storage-postgres/src/lib.rs
+++ b/crates/storage-postgres/src/lib.rs
@@ -8,6 +8,7 @@ mod audit;
 mod convert;
 mod error;
 mod events;
+mod invites;
 mod jobs;
 mod lists;
 mod readings;
diff --git a/crates/storage-sqlite/src/invites.rs b/crates/storage-sqlite/src/invites.rs
new file mode 100644
index 0000000..b272b57
--- /dev/null
+++ b/crates/storage-sqlite/src/invites.rs
@@ -0,0 +1,133 @@
+//! Invites: one-time links for account setup and password reset (spec §8.3). Only the token's
+//! SHA-256 is stored, and it is never read back into a record.
+
+use chrono::{DateTime, Utc};
+use smoke_domain::audit::{AuditAction, AuditEntity};
+use smoke_domain::records::{Invite, NewInvite};
+use smoke_domain::repos::{Conflict, InviteRepo, InviteWrites, StoreError, StoreResult};
+use smoke_domain::types::{InviteId, TokenHash, UserId};
+use sqlx::SqliteConnection;
+
+use crate::convert::{instant, micros, opt_instant, parse};
+use crate::error::IntoStore;
+use crate::{SqliteStore, SqliteTx};
+
+struct InviteRow {
+    id: i64,
+    user_id: i64,
+    purpose: String,
+    created_by: i64,
+    created_at: i64,
+    expires_at: i64,
+    used_at: Option<i64>,
+}
+
+impl InviteRow {
+    fn into_invite(self) -> StoreResult<Invite> {
+        Ok(Invite {
+            id: InviteId(self.id),
+            user_id: UserId(self.user_id),
+            purpose: parse(&self.purpose)?,
+            created_by: UserId(self.created_by),
+            created_at: instant(self.created_at)?,
+            expires_at: instant(self.expires_at)?,
+            used_at: opt_instant(self.used_at)?,
+        })
+    }
+}
+
+async fn require(conn: &mut SqliteConnection, id: InviteId) -> StoreResult<Invite> {
+    sqlx::query_as!(
+        InviteRow,
+        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
+         FROM invites WHERE id = $1",
+        id.0,
+    )
+    .fetch_optional(conn)
+    .await
+    .db()?
+    .ok_or(StoreError::NotFound {
+        entity: AuditEntity::Invite,
+        id: id.0,
+    })?
+    .into_invite()
+}
+
+async fn by_token(conn: &mut SqliteConnection, token: &TokenHash) -> StoreResult<Option<Invite>> {
+    sqlx::query_as!(
+        InviteRow,
+        "SELECT id, user_id, purpose, created_by, created_at, expires_at, used_at \
+         FROM invites WHERE token_hash = $1",
+        token.as_str(),
+    )
+    .fetch_optional(conn)
+    .await
+    .db()?
+    .map(InviteRow::into_invite)
+    .transpose()
+}
+
+impl InviteRepo for SqliteStore {
+    async fn invite_by_token(&self, token: &TokenHash) -> StoreResult<Option<Invite>> {
+        by_token(&mut *self.read().await?, token).await
+    }
+
+    async fn delete_expired_invites(&self, now: DateTime<Utc>) -> StoreResult<u64> {
+        let done = sqlx::query!("DELETE FROM invites WHERE expires_at <= $1", micros(now))
+            .execute(&mut *self.write().await?)
+            .await
+            .db()?;
+        Ok(done.rows_affected())
+    }
+}
+
+impl InviteWrites for SqliteTx {
+    async fn create_invite(&mut self, new: NewInvite) -> StoreResult<Invite> {
+        sqlx::query!(
+            "DELETE FROM invites WHERE user_id = $1 AND used_at IS NULL",
+            new.user_id.0,
+        )
+        .execute(self.conn())
+        .await
+        .db()?;
+        let id = sqlx::query_scalar!(
+            "INSERT INTO invites (user_id, purpose, token_hash, created_by, created_at, expires_at) \
+             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id AS \"id!\"",
+            new.user_id.0,
+            new.purpose.as_str(),
+            new.token_hash.as_str(),
+            new.created_by.0,
+            micros(self.now()),
+            micros(new.expires_at),
+        )
+        .fetch_one(self.conn())
+        .await
+        .db()?;
+        let invite = require(self.conn(), InviteId(id)).await?;
+        self.record(AuditAction::Invite, None, &invite).await?;
+        Ok(invite)
+    }
+
+    async fn invite_by_token(&mut self, token: &TokenHash) -> StoreResult<Option<Invite>> {
+        by_token(self.conn(), token).await
+    }
+
+    async fn use_invite(&mut self, id: InviteId) -> StoreResult<Invite> {
+        let before = require(self.conn(), id).await?;
+        if before.used_at.is_some() {
+            return Err(Conflict::InviteUsed.into());
+        }
+        sqlx::query!(
+            "UPDATE invites SET used_at = $1 WHERE id = $2",
+            micros(self.now()),
+            id.0,
+        )
+        .execute(self.conn())
+        .await
+        .db()?;
+        let after = require(self.conn(), id).await?;
+        self.record(AuditAction::Update, Some(&before), &after)
+            .await?;
+        Ok(after)
+    }
+}
diff --git a/crates/storage-sqlite/src/lib.rs b/crates/storage-sqlite/src/lib.rs
index ff56454..a28ffd0 100644
--- a/crates/storage-sqlite/src/lib.rs
+++ b/crates/storage-sqlite/src/lib.rs
@@ -8,6 +8,7 @@ mod audit;
 mod convert;
 mod error;
 mod events;
+mod invites;
 mod jobs;
 mod lists;
 mod readings;
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 8737c71..5751d2f 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -118,3 +118,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 109 | 2026-10-13 | Storage | Migration 0004 makes `smell_reports.event_id` **nullable**: SQLite **rebuilds** `smell_reports` and both link tables inside the migration's transaction (copy, drop, rename) with foreign keys on; Postgres drops `NOT NULL` | `PRAGMA foreign_keys = OFF` in a no-transaction migration; a sentinel "no event" event | Atomic, no switched-off checks, no sentinel rows to filter out everywhere |
 | 110 | 2026-10-13 | Events | An event whose only smell report is **corrected to 0** (or moved out) **is kept**, with intensity 0 and **still counted** in the summary; confirmed by the user. Refines #105: #105 rejects *opening* an event at zero intensity, a correction afterwards is allowed (#106) | Delete the event; hide it from the list and the summary | A correction records what was true; the event still happened and the log stays complete |
 | 111 | 2026-10-14 | Sessions | `save_session` **only updates** a stored session (`UPDATE … WHERE id_hash`); a session ended meanwhile by sign-out, a password change or expiry is never written back. Replaces the upsert of #56 | Keep the upsert and re-check the user's password version on every request | A request that loaded its session before a password change and renewed it afterwards (#59) brought the ended session back, so the change did not sign out the other phone; tower-sessions creates new sessions through `create`, so nothing needs the insert |
+| 112 | 2026-10-14 | Invites | An invite stores only the **SHA-256 of its token** (`TokenHash`); the record and its audit snapshot leave the hash out. Creating one is audited as `invite`, using it as an `update` that sets `used_at`; a new link for a person **removes their earlier unused links**; expired invites are **deleted at startup and daily without an audit entry** | Keep every link until it expires; audit each cleanup deletion | Only the newest QR code shown works; invites are not evidence (`has_snapshot` is false), so deleting them leaves `verify-audit` clean, and a cleanup that writes audit rows would bury the real entries |
```

Then regenerate the query metadata (six new queries per backend):

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
Expected: every test passes, including the five `invites::` contract tests on both backends,
`a_token_hash_is_the_sha256_of_the_token_in_hex`, `an_invite_works_once_and_for_five_minutes`
and `the_invite_sweep_runs_as_soon_as_the_server_starts`. With `TEST_POSTGRES_URL` set:
**506** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/commands/serve.rs \
  crates/domain/src/records.rs \
  crates/domain/src/repos.rs \
  crates/domain/src/types.rs \
  crates/storage-contract/src/invites.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-postgres/src/invites.rs \
  crates/storage-postgres/src/lib.rs \
  crates/storage-sqlite/src/invites.rs \
  crates/storage-sqlite/src/lib.rs \
  docs/DECISIONS.md \
  crates/storage-postgres/.sqlx \
  crates/storage-sqlite/.sqlx
git commit -m "feat(invites): store one-time links by token hash, behind repository traits

\`InviteRepo\`/\`InviteWrites\` join the storage boundary with SQLite and Postgres
implementations: the token's SHA-256 is the only stored form, a new link removes the
person's unused ones, using one is audited, and \`serve\` deletes expired invites at startup
and daily.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Account use cases in `smoke_domain::people`

The rules of spec §4 and §8.3 in one place, in the style of the logbook (decision #86): generic
functions over the write-transaction traits that check permissions against data read inside the
transaction (decision #113). Inviting creates the account without a password plus its setup
link; a new link is a setup link if the person never chose a password, otherwise a password
reset; accepting a link uses it up and sets the password and language in one transaction (the
password change ends every session, #58). Only an expired account can be reactivated, and
expiring an expired one changes nothing (decision #114). `ExpiryPick` turns the guest quick
picks into an instant in the configured time zone (decision #115). `UserWrites` gains a read
inside the transaction. The storage contract runs every use case on both backends.

**Files:**
- Create: `crates/domain/src/people.rs`, `crates/storage-contract/src/people.rs`
- Modify: `crates/domain/src/lib.rs`, `crates/domain/src/repos.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-postgres/src/users.rs`, `crates/storage-sqlite/src/users.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `InviteWrites`, `Invite`, `NewInvite`, `TokenHash`, `InvitePurpose` (Task 2);
  `UserWrites`, `check_expiry`, `can`, `Action::ManageUsers`, `DailySchedule::{local_date,
  day_span}` (milestones 1–2).
- Produces (in `smoke_domain`):
  - `repos::UserWrites::user(&mut self, UserId) -> StoreResult<Option<User>>` (implemented by
    both transactions).
  - `people::PeopleError { Forbidden, NotFound, UsernameTaken, NotExpired, LinkInvalid,
    Invalid(DomainError), Store(StoreError) }` (`From<StoreError>` maps
    `Conflict::UsernameTaken`, `Conflict::InviteUsed` and `StoreError::Invalid`).
  - `people::ExpiryPick { OneDay, Weekend, OneWeek, TwoWeeks, Until(NaiveDate), Never }`,
    `resolve(self, now, &DailySchedule) -> Option<DateTime<Utc>>`.
  - `people::NewPerson { username: Username, role: Role, locale: Locale, expires_at:
    Option<DateTime<Utc>> }`.
  - `people::invite(tx, &Actor, NewPerson, TokenHash, now) -> Result<(User, Invite)>`,
    `people::issue_link(tx, &Actor, UserId, TokenHash, now) -> Result<(User, Invite)>`,
    `people::usable_invite(tx, &TokenHash, now) -> Result<Invite>`,
    `people::accept(tx, &TokenHash, PasswordHash, Locale, now) -> Result<User>`,
    `people::expire(tx, &Actor, UserId, now) -> Result<User>`,
    `people::reactivate(tx, &Actor, UserId, Option<DateTime<Utc>>, now) -> Result<User>`,
    `people::change_password(tx, &Actor, PasswordHash, now) -> Result<User>` (all
    `Result<_, PeopleError>`).

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index b45722b..30afcfb 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -16,6 +16,7 @@ pub mod invites;
 pub mod lists;
 pub mod logbook;
 pub mod migrations;
+pub mod people;
 pub mod readings;
 pub mod reports;
 pub mod sessions;
@@ -104,6 +105,14 @@ macro_rules! contract_tests {
             invites::a_new_invite_replaces_the_users_unused_ones,
             invites::an_invite_for_an_unknown_user_is_refused,
             invites::expired_invites_are_removed_without_an_audit_entry,
+            people::an_invited_guest_sets_up_the_account_once,
+            people::a_link_stops_working_after_five_minutes,
+            people::a_reset_link_replaces_the_password_and_ends_every_session,
+            people::a_new_link_for_someone_who_never_set_up_is_a_setup_link,
+            people::only_residents_manage_people,
+            people::a_taken_username_is_refused_and_nothing_is_written,
+            people::expiry_and_reactivation_follow_the_role_rules,
+            people::a_password_change_needs_an_account_that_can_sign_in,
             logbook::logging_now_with_nothing_open_starts_an_open_event,
             logbook::same_smell_joins_the_open_event,
             logbook::a_new_event_closes_the_open_one_at_its_last_report,
diff --git a/crates/storage-contract/src/people.rs b/crates/storage-contract/src/people.rs
new file mode 100644
index 0000000..c68450d
--- /dev/null
+++ b/crates/storage-contract/src/people.rs
@@ -0,0 +1,369 @@
+//! The account use cases of `smoke_domain::people`, run against each backend: invitations,
+//! setup and reset links, expiry, reactivation and password changes.
+
+use chrono::{DateTime, TimeDelta, Utc};
+use smoke_domain::DomainError;
+use smoke_domain::audit::{AuditAction, AuditEntity};
+use smoke_domain::clock::Clock;
+use smoke_domain::people::{
+    NewPerson, PeopleError, accept, change_password, expire, invite, issue_link, reactivate,
+};
+use smoke_domain::records::{Invite, PasswordHash, StoredSession, User};
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{InvitePurpose, Locale, Role, TokenHash, Username};
+
+use crate::Fixture;
+use crate::fixture::{add_user, start};
+
+fn person(name: &str, role: Role, expires_at: Option<DateTime<Utc>>) -> NewPerson {
+    NewPerson {
+        username: Username::new(name).unwrap(),
+        role,
+        locale: Locale::It,
+        expires_at,
+    }
+}
+
+fn next_week() -> Option<DateTime<Utc>> {
+    Some(start() + TimeDelta::days(7))
+}
+
+fn hash(text: &str) -> PasswordHash {
+    PasswordHash::new(text.to_owned())
+}
+
+async fn invite_guest<S: Store>(fx: &Fixture<S>, by: &User, token: &str) -> (User, Invite) {
+    let mut tx = fx.store.begin(Some(by.id)).await.unwrap();
+    let invited = invite(
+        &mut tx,
+        &by.actor(),
+        person("guest", Role::Guest, next_week()),
+        TokenHash::of(token),
+        fx.clock.now(),
+    )
+    .await
+    .unwrap();
+    tx.commit().await.unwrap();
+    invited
+}
+
+async fn accept_link<S: Store>(
+    fx: &Fixture<S>,
+    user: &User,
+    token: &str,
+) -> Result<User, PeopleError> {
+    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
+    let accepted = accept(
+        &mut tx,
+        &TokenHash::of(token),
+        hash("chosen-hash"),
+        Locale::Tr,
+        fx.clock.now(),
+    )
+    .await?;
+    tx.commit().await.unwrap();
+    Ok(accepted)
+}
+
+async fn actions<S: Store>(fx: &Fixture<S>) -> Vec<(AuditEntity, AuditAction)> {
+    fx.store
+        .audit_log()
+        .await
+        .unwrap()
+        .iter()
+        .map(|e| (e.record.entity, e.record.action))
+        .collect()
+}
+
+pub async fn an_invited_guest_sets_up_the_account_once<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let (guest, link) = invite_guest(fx, &resident, "setup-token").await;
+    assert_eq!(guest.password_hash, None);
+    assert_eq!((guest.role, guest.expires_at), (Role::Guest, next_week()));
+    assert_eq!(link.purpose, InvitePurpose::Setup);
+    assert_eq!(link.expires_at, start() + Invite::LIFETIME);
+
+    fx.clock.advance(TimeDelta::minutes(4));
+    let set_up = accept_link(fx, &guest, "setup-token").await.unwrap();
+    assert_eq!(set_up.password_hash, Some(hash("chosen-hash")));
+    assert_eq!(set_up.locale, Locale::Tr);
+    assert!(matches!(
+        accept_link(fx, &guest, "setup-token").await,
+        Err(PeopleError::LinkInvalid)
+    ));
+    assert_eq!(
+        actions(fx).await,
+        [
+            (AuditEntity::User, AuditAction::Create),
+            (AuditEntity::User, AuditAction::Create),
+            (AuditEntity::Invite, AuditAction::Invite),
+            (AuditEntity::Invite, AuditAction::Update),
+            (AuditEntity::User, AuditAction::PasswordChange),
+            (AuditEntity::User, AuditAction::Update),
+        ]
+    );
+}
+
+pub async fn a_link_stops_working_after_five_minutes<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let (guest, _) = invite_guest(fx, &resident, "setup-token").await;
+    fx.clock.advance(Invite::LIFETIME);
+    assert!(matches!(
+        accept_link(fx, &guest, "setup-token").await,
+        Err(PeopleError::LinkInvalid)
+    ));
+    assert!(matches!(
+        accept_link(fx, &guest, "never-issued").await,
+        Err(PeopleError::LinkInvalid)
+    ));
+    let stored = fx.store.user(guest.id).await.unwrap().unwrap();
+    assert_eq!(stored.password_hash, None);
+}
+
+pub async fn a_reset_link_replaces_the_password_and_ends_every_session<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let partner = add_user(fx, "partner", Role::Resident).await;
+    fx.store
+        .create_session(&StoredSession {
+            id_hash: "partner-phone".to_owned(),
+            user_id: Some(partner.id),
+            data: "{}".to_owned(),
+            expires_at: start() + TimeDelta::days(30),
+        })
+        .await
+        .unwrap();
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let (_, first) = issue_link(
+        &mut tx,
+        &resident.actor(),
+        partner.id,
+        TokenHash::of("first"),
+        start(),
+    )
+    .await
+    .unwrap();
+    let (_, second) = issue_link(
+        &mut tx,
+        &resident.actor(),
+        partner.id,
+        TokenHash::of("second"),
+        start(),
+    )
+    .await
+    .unwrap();
+    tx.commit().await.unwrap();
+    assert_eq!(first.purpose, InvitePurpose::PasswordReset);
+    assert_eq!(second.purpose, InvitePurpose::PasswordReset);
+
+    assert!(matches!(
+        accept_link(fx, &partner, "first").await,
+        Err(PeopleError::LinkInvalid)
+    ));
+    let reset = accept_link(fx, &partner, "second").await.unwrap();
+    assert_eq!(reset.password_hash, Some(hash("chosen-hash")));
+    assert_eq!(
+        fx.store.session("partner-phone", start()).await.unwrap(),
+        None
+    );
+}
+
+pub async fn a_new_link_for_someone_who_never_set_up_is_a_setup_link<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let (guest, _) = invite_guest(fx, &resident, "lost").await;
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let (_, again) = issue_link(
+        &mut tx,
+        &resident.actor(),
+        guest.id,
+        TokenHash::of("again"),
+        start(),
+    )
+    .await
+    .unwrap();
+    tx.commit().await.unwrap();
+    assert_eq!(again.purpose, InvitePurpose::Setup);
+    assert!(accept_link(fx, &guest, "lost").await.is_err());
+    assert!(accept_link(fx, &guest, "again").await.is_ok());
+}
+
+pub async fn only_residents_manage_people<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    let before = actions(fx).await;
+    for outsider in [&guest, &viewer] {
+        let actor = outsider.actor();
+        let mut tx = fx.store.begin(Some(outsider.id)).await.unwrap();
+        let refused = [
+            invite(
+                &mut tx,
+                &actor,
+                person("friend", Role::Guest, next_week()),
+                TokenHash::of("x"),
+                start(),
+            )
+            .await
+            .map(|_| ()),
+            issue_link(&mut tx, &actor, resident.id, TokenHash::of("y"), start())
+                .await
+                .map(|_| ()),
+            expire(&mut tx, &actor, resident.id, start())
+                .await
+                .map(|_| ()),
+            reactivate(&mut tx, &actor, guest.id, next_week(), start())
+                .await
+                .map(|_| ()),
+        ];
+        for result in refused {
+            assert!(
+                matches!(result, Err(PeopleError::Forbidden)),
+                "{:?}: {result:?}",
+                outsider.role
+            );
+        }
+        tx.commit().await.unwrap();
+    }
+    assert_eq!(actions(fx).await, before);
+}
+
+pub async fn a_taken_username_is_refused_and_nothing_is_written<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let before = actions(fx).await;
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let result = invite(
+        &mut tx,
+        &resident.actor(),
+        person("Daniele", Role::Guest, next_week()),
+        TokenHash::of("x"),
+        start(),
+    )
+    .await;
+    assert!(
+        matches!(result, Err(PeopleError::UsernameTaken)),
+        "{result:?}"
+    );
+    drop(tx);
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let result = invite(
+        &mut tx,
+        &resident.actor(),
+        person("friend", Role::Guest, None),
+        TokenHash::of("x"),
+        start(),
+    )
+    .await;
+    assert!(
+        matches!(
+            result,
+            Err(PeopleError::Invalid(DomainError::GuestsMustExpire))
+        ),
+        "{result:?}"
+    );
+    drop(tx);
+    assert_eq!(actions(fx).await, before);
+    assert!(
+        fx.store
+            .invite_by_token(&TokenHash::of("x"))
+            .await
+            .unwrap()
+            .is_none()
+    );
+}
+
+pub async fn expiry_and_reactivation_follow_the_role_rules<S: Store>(fx: &Fixture<S>) {
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    let actor = resident.actor();
+    let now = start() + TimeDelta::hours(1);
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+
+    let result = reactivate(&mut tx, &actor, guest.id, next_week(), now).await;
+    assert!(matches!(result, Err(PeopleError::NotExpired)), "{result:?}");
+    let result = expire(&mut tx, &actor, resident.id, now).await;
+    assert!(
+        matches!(
+            result,
+            Err(PeopleError::Invalid(DomainError::ResidentsNeverExpire))
+        ),
+        "{result:?}"
+    );
+
+    fx.clock.set(now);
+    let expired = expire(&mut tx, &actor, guest.id, now).await.unwrap();
+    assert_eq!(expired.expires_at, Some(now));
+    let again = expire(&mut tx, &actor, guest.id, now + TimeDelta::hours(1))
+        .await
+        .unwrap();
+    assert_eq!(
+        again.expires_at,
+        Some(now),
+        "an expired account stays as it was"
+    );
+
+    let later = now + TimeDelta::hours(1);
+    let result = reactivate(&mut tx, &actor, guest.id, None, later).await;
+    assert!(
+        matches!(
+            result,
+            Err(PeopleError::Invalid(DomainError::GuestsMustExpire))
+        ),
+        "{result:?}"
+    );
+    let back = reactivate(&mut tx, &actor, guest.id, next_week(), later)
+        .await
+        .unwrap();
+    assert_eq!(back.expires_at, next_week());
+    assert_eq!(
+        back.password_hash, guest.password_hash,
+        "the old password still works"
+    );
+
+    expire(&mut tx, &actor, viewer.id, now).await.unwrap();
+    let unlimited = reactivate(&mut tx, &actor, viewer.id, None, later)
+        .await
+        .unwrap();
+    assert_eq!(unlimited.expires_at, None);
+    tx.commit().await.unwrap();
+
+    let user_actions: Vec<_> = actions(fx)
+        .await
+        .into_iter()
+        .skip(3)
+        .map(|(_, action)| action)
+        .collect();
+    assert_eq!(
+        user_actions,
+        [
+            AuditAction::Expire,
+            AuditAction::Reactivate,
+            AuditAction::Expire,
+            AuditAction::Reactivate
+        ]
+    );
+}
+
+pub async fn a_password_change_needs_an_account_that_can_sign_in<S: Store>(fx: &Fixture<S>) {
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    let after_expiry = start() + TimeDelta::days(8);
+    assert!(guest.actor().is_expired(after_expiry));
+    let mut expired_viewer = viewer.actor();
+    expired_viewer.expires_at = Some(start());
+
+    let mut tx = fx.store.begin(Some(guest.id)).await.unwrap();
+    let changed = change_password(&mut tx, &guest.actor(), hash("new"), after_expiry)
+        .await
+        .unwrap();
+    assert_eq!(
+        changed.password_hash,
+        Some(hash("new")),
+        "an expired guest still signs in"
+    );
+    let refused = change_password(&mut tx, &expired_viewer, hash("new"), after_expiry).await;
+    assert!(
+        matches!(refused, Err(PeopleError::Forbidden)),
+        "{refused:?}"
+    );
+    tx.commit().await.unwrap();
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: compile errors — `smoke_domain::people` does not exist.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/domain/src/lib.rs b/crates/domain/src/lib.rs
index 1576e40..1429662 100644
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -9,6 +9,7 @@ pub mod error;
 pub mod events;
 pub mod history;
 pub mod logbook;
+pub mod people;
 pub mod permissions;
 pub mod records;
 pub mod repos;
diff --git a/crates/domain/src/people.rs b/crates/domain/src/people.rs
new file mode 100644
index 0000000..d763b85
--- /dev/null
+++ b/crates/domain/src/people.rs
@@ -0,0 +1,297 @@
+//! The account use cases (spec §4, §8.3, §8.4): inviting people, setup and password-reset
+//! links, expiry, reactivation and password changes. Like the logbook, each one runs inside a
+//! write transaction the caller begins and commits, and checks permissions against the data read
+//! inside it.
+
+use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
+use thiserror::Error;
+
+use crate::DomainError;
+use crate::permissions::{Action, Actor, can, check_expiry};
+use crate::records::{Invite, NewInvite, NewUser, PasswordHash, User};
+use crate::repos::{Conflict, InviteWrites, StoreError, UserWrites};
+use crate::schedule::DailySchedule;
+use crate::types::{InvitePurpose, Locale, Role, TokenHash, UserId, Username};
+
+#[derive(Debug, Error)]
+pub enum PeopleError {
+    #[error("not allowed")]
+    Forbidden,
+    /// No such user.
+    #[error("not found")]
+    NotFound,
+    #[error("that username is taken")]
+    UsernameTaken,
+    /// Only an expired account can be reactivated.
+    #[error("the account has not expired")]
+    NotExpired,
+    /// The link is unknown, already used, or older than five minutes.
+    #[error("the link is no longer valid")]
+    LinkInvalid,
+    #[error(transparent)]
+    Invalid(#[from] DomainError),
+    #[error(transparent)]
+    Store(StoreError),
+}
+
+impl From<StoreError> for PeopleError {
+    fn from(error: StoreError) -> Self {
+        match error {
+            StoreError::Conflict(Conflict::UsernameTaken) => Self::UsernameTaken,
+            StoreError::Conflict(Conflict::InviteUsed) => Self::LinkInvalid,
+            StoreError::Invalid(invalid) => Self::Invalid(invalid),
+            other => Self::Store(other),
+        }
+    }
+}
+
+/// When a new or reactivated account stops working (spec §8.3): the quick picks for guests, a
+/// chosen day, or never.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum ExpiryPick {
+    OneDay,
+    /// Until the start of the coming Monday.
+    Weekend,
+    OneWeek,
+    TwoWeeks,
+    /// Until the end of this local day.
+    Until(NaiveDate),
+    Never,
+}
+
+impl ExpiryPick {
+    /// The instant the account expires, in the configured time zone; `None` for never.
+    pub fn resolve(self, now: DateTime<Utc>, schedule: &DailySchedule) -> Option<DateTime<Utc>> {
+        match self {
+            Self::OneDay => Some(now + TimeDelta::days(1)),
+            Self::OneWeek => Some(now + TimeDelta::days(7)),
+            Self::TwoWeeks => Some(now + TimeDelta::days(14)),
+            Self::Weekend => {
+                let today = schedule.local_date(now);
+                let days = 7 - i64::from(today.weekday().num_days_from_monday());
+                let monday = today + TimeDelta::days(days);
+                Some(schedule.day_span(monday).0)
+            }
+            Self::Until(date) => Some(schedule.day_span(date).1),
+            Self::Never => None,
+        }
+    }
+}
+
+/// A person a resident invites: the account is created without a password, which the invitee
+/// chooses through the setup link.
+#[derive(Debug, Clone, PartialEq)]
+pub struct NewPerson {
+    pub username: Username,
+    pub role: Role,
+    /// Shown first on the setup page; the invitee confirms or changes it there.
+    pub locale: Locale,
+    pub expires_at: Option<DateTime<Utc>>,
+}
+
+fn require_manager(actor: &Actor, now: DateTime<Utc>) -> Result<(), PeopleError> {
+    if can(actor, Action::ManageUsers, now) {
+        Ok(())
+    } else {
+        Err(PeopleError::Forbidden)
+    }
+}
+
+async fn require_user<T: UserWrites>(tx: &mut T, id: UserId) -> Result<User, PeopleError> {
+    tx.user(id).await?.ok_or(PeopleError::NotFound)
+}
+
+async fn new_link<T: InviteWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    user: &User,
+    token: TokenHash,
+    now: DateTime<Utc>,
+) -> Result<Invite, PeopleError> {
+    let purpose = if user.password_hash.is_some() {
+        InvitePurpose::PasswordReset
+    } else {
+        InvitePurpose::Setup
+    };
+    let invite = tx
+        .create_invite(NewInvite {
+            user_id: user.id,
+            purpose,
+            token_hash: token,
+            created_by: actor.id,
+            expires_at: now + Invite::LIFETIME,
+        })
+        .await?;
+    Ok(invite)
+}
+
+/// Creates the account and its setup link (spec §8.3). `token` is the hash of the link's
+/// token, which only the QR code carries.
+pub async fn invite<T: UserWrites + InviteWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    person: NewPerson,
+    token: TokenHash,
+    now: DateTime<Utc>,
+) -> Result<(User, Invite), PeopleError> {
+    require_manager(actor, now)?;
+    check_expiry(person.role, person.expires_at, now)?;
+    let user = tx
+        .create_user(NewUser {
+            username: person.username,
+            password_hash: None,
+            role: person.role,
+            locale: person.locale,
+            expires_at: person.expires_at,
+        })
+        .await?;
+    let invite = new_link(tx, actor, &user, token, now).await?;
+    Ok((user, invite))
+}
+
+/// A fresh link for an existing account: a password reset, or a new setup link if the person
+/// never chose a password. Earlier unused links stop working.
+pub async fn issue_link<T: UserWrites + InviteWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    user_id: UserId,
+    token: TokenHash,
+    now: DateTime<Utc>,
+) -> Result<(User, Invite), PeopleError> {
+    require_manager(actor, now)?;
+    let user = require_user(tx, user_id).await?;
+    let invite = new_link(tx, actor, &user, token, now).await?;
+    Ok((user, invite))
+}
+
+/// The usable invite behind a link, or [`PeopleError::LinkInvalid`].
+pub async fn usable_invite<T: InviteWrites>(
+    tx: &mut T,
+    token: &TokenHash,
+    now: DateTime<Utc>,
+) -> Result<Invite, PeopleError> {
+    tx.invite_by_token(token)
+        .await?
+        .filter(|invite| invite.is_usable(now))
+        .ok_or(PeopleError::LinkInvalid)
+}
+
+/// The invitee chose a password and a language through the link (spec §8.3). The link is used
+/// up, and every session the account had ends with the password change.
+pub async fn accept<T: UserWrites + InviteWrites>(
+    tx: &mut T,
+    token: &TokenHash,
+    password: PasswordHash,
+    locale: Locale,
+    now: DateTime<Utc>,
+) -> Result<User, PeopleError> {
+    let invite = usable_invite(tx, token, now).await?;
+    tx.use_invite(invite.id).await?;
+    tx.set_password(invite.user_id, password).await?;
+    Ok(tx.set_locale(invite.user_id, locale).await?)
+}
+
+/// Ends a guest's or viewer's access now (spec §4) and every session they have. Expiring an
+/// account that has already expired changes nothing.
+pub async fn expire<T: UserWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    user_id: UserId,
+    now: DateTime<Utc>,
+) -> Result<User, PeopleError> {
+    require_manager(actor, now)?;
+    let user = require_user(tx, user_id).await?;
+    if user.actor().is_expired(now) {
+        return Ok(user);
+    }
+    Ok(tx.expire_user(user_id).await?)
+}
+
+/// Gives an expired account a new expiry (or none, for a viewer); the old password still works
+/// (spec §4).
+pub async fn reactivate<T: UserWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    user_id: UserId,
+    expires_at: Option<DateTime<Utc>>,
+    now: DateTime<Utc>,
+) -> Result<User, PeopleError> {
+    require_manager(actor, now)?;
+    let user = require_user(tx, user_id).await?;
+    if !user.actor().is_expired(now) {
+        return Err(PeopleError::NotExpired);
+    }
+    Ok(tx.reactivate_user(user_id, expires_at).await?)
+}
+
+/// The signed-in person sets a new password (spec §9 screen 7). Every session of the account
+/// ends; the caller signs the current device in again.
+pub async fn change_password<T: UserWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    password: PasswordHash,
+    now: DateTime<Utc>,
+) -> Result<User, PeopleError> {
+    if !actor.can_sign_in(now) {
+        return Err(PeopleError::Forbidden);
+    }
+    Ok(tx.set_password(actor.id, password).await?)
+}
+
+#[cfg(test)]
+mod tests {
+    use chrono_tz::Europe::Rome;
+
+    use super::*;
+
+    fn t(s: &str) -> DateTime<Utc> {
+        s.parse().unwrap()
+    }
+
+    fn rome() -> DailySchedule {
+        DailySchedule::new(Rome, 4).unwrap()
+    }
+
+    #[test]
+    fn quick_picks_count_from_now() {
+        let now = t("2026-01-15T18:30:00Z");
+        assert_eq!(
+            ExpiryPick::OneDay.resolve(now, &rome()),
+            Some(t("2026-01-16T18:30:00Z"))
+        );
+        assert_eq!(
+            ExpiryPick::OneWeek.resolve(now, &rome()),
+            Some(t("2026-01-22T18:30:00Z"))
+        );
+        assert_eq!(
+            ExpiryPick::TwoWeeks.resolve(now, &rome()),
+            Some(t("2026-01-29T18:30:00Z"))
+        );
+        assert_eq!(ExpiryPick::Never.resolve(now, &rome()), None);
+    }
+
+    #[test]
+    fn the_weekend_lasts_until_the_coming_monday_in_local_time() {
+        let thursday = t("2026-01-15T18:30:00Z");
+        let monday = Some(t("2026-01-18T23:00:00Z"));
+        assert_eq!(ExpiryPick::Weekend.resolve(thursday, &rome()), monday);
+        let sunday_night = t("2026-01-18T22:30:00Z");
+        assert_eq!(ExpiryPick::Weekend.resolve(sunday_night, &rome()), monday);
+        let monday_morning = t("2026-01-19T07:00:00Z");
+        assert_eq!(
+            ExpiryPick::Weekend.resolve(monday_morning, &rome()),
+            Some(t("2026-01-25T23:00:00Z")),
+            "on a Monday the weekend is the coming one"
+        );
+    }
+
+    #[test]
+    fn a_chosen_day_lasts_until_its_local_midnight() {
+        let day = NaiveDate::from_ymd_opt(2026, 3, 28).unwrap();
+        assert_eq!(
+            ExpiryPick::Until(day).resolve(t("2026-01-15T18:30:00Z"), &rome()),
+            Some(t("2026-03-28T23:00:00Z")),
+            "winter time still holds at that midnight"
+        );
+    }
+}
diff --git a/crates/domain/src/repos.rs b/crates/domain/src/repos.rs
index 9e03086..e7bd9eb 100644
--- a/crates/domain/src/repos.rs
+++ b/crates/domain/src/repos.rs
@@ -184,6 +184,8 @@ pub trait AuditRepo: Send + Sync {
 }
 
 pub trait UserWrites: Send {
+    /// Reads inside the transaction, so a decision and the write that follows it see the same data.
+    fn user(&mut self, id: UserId) -> impl Future<Output = StoreResult<Option<User>>> + Send;
     /// Fails with [`DomainError`] if the expiry breaks the role rules
     /// ([`crate::permissions::check_expiry`]) and with [`Conflict::UsernameTaken`].
     fn create_user(&mut self, new: NewUser) -> impl Future<Output = StoreResult<User>> + Send;
diff --git a/crates/storage-postgres/src/users.rs b/crates/storage-postgres/src/users.rs
index f3311d8..fd4a53a 100644
--- a/crates/storage-postgres/src/users.rs
+++ b/crates/storage-postgres/src/users.rs
@@ -107,6 +107,10 @@ impl UserRepo for PostgresStore {
 }
 
 impl UserWrites for PostgresTx {
+    async fn user(&mut self, id: UserId) -> StoreResult<Option<User>> {
+        fetch(self.conn(), id).await
+    }
+
     async fn create_user(&mut self, new: NewUser) -> StoreResult<User> {
         let now = self.now();
         check_expiry(new.role, new.expires_at, now)?;
diff --git a/crates/storage-sqlite/src/users.rs b/crates/storage-sqlite/src/users.rs
index da845f9..5174b80 100644
--- a/crates/storage-sqlite/src/users.rs
+++ b/crates/storage-sqlite/src/users.rs
@@ -107,6 +107,10 @@ impl UserRepo for SqliteStore {
 }
 
 impl UserWrites for SqliteTx {
+    async fn user(&mut self, id: UserId) -> StoreResult<Option<User>> {
+        fetch(self.conn(), id).await
+    }
+
     async fn create_user(&mut self, new: NewUser) -> StoreResult<User> {
         let now = self.now();
         check_expiry(new.role, new.expires_at, now)?;
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 5751d2f..2f7ca1b 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -119,3 +119,6 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 110 | 2026-10-13 | Events | An event whose only smell report is **corrected to 0** (or moved out) **is kept**, with intensity 0 and **still counted** in the summary; confirmed by the user. Refines #105: #105 rejects *opening* an event at zero intensity, a correction afterwards is allowed (#106) | Delete the event; hide it from the list and the summary | A correction records what was true; the event still happened and the log stays complete |
 | 111 | 2026-10-14 | Sessions | `save_session` **only updates** a stored session (`UPDATE … WHERE id_hash`); a session ended meanwhile by sign-out, a password change or expiry is never written back. Replaces the upsert of #56 | Keep the upsert and re-check the user's password version on every request | A request that loaded its session before a password change and renewed it afterwards (#59) brought the ended session back, so the change did not sign out the other phone; tower-sessions creates new sessions through `create`, so nothing needs the insert |
 | 112 | 2026-10-14 | Invites | An invite stores only the **SHA-256 of its token** (`TokenHash`); the record and its audit snapshot leave the hash out. Creating one is audited as `invite`, using it as an `update` that sets `used_at`; a new link for a person **removes their earlier unused links**; expired invites are **deleted at startup and daily without an audit entry** | Keep every link until it expires; audit each cleanup deletion | Only the newest QR code shown works; invites are not evidence (`has_snapshot` is false), so deleting them leaves `verify-audit` clean, and a cleanup that writes audit rows would bury the real entries |
+| 113 | 2026-10-14 | Structure | The account use cases (**`smoke_domain::people`**: invite, new link, accept a link, expire, reactivate, change password) are generic functions over the write-transaction traits that check permissions against data read inside the transaction, like the logbook (#86); the storage contract runs them on both backends | Account rules in the web handlers | One place for the §4 and §8.3 rules, tested on SQLite and Postgres |
+| 114 | 2026-10-14 | Accounts | A new link for someone who **never chose a password is a setup link**, otherwise a password reset; accepting one sets the password and language and uses up the link in one transaction; **only an expired account can be reactivated**, and expiring an expired one changes nothing | Reactivate also changes an active guest's expiry; two buttons for setup and reset | Spec §4 reactivates expired accounts; a lost setup QR is fixed the same way as a forgotten password; a double tap on Expire cannot move the end later |
+| 115 | 2026-10-14 | Accounts | Guest expiry quick picks: **1 day, 1 week (default), 2 weeks** count from now; **weekend** lasts until the coming **Monday 00:00** local time (on a Monday, the next one); a **chosen day** lasts until the end of that local day | All picks end at midnight; weekend ends on Sunday at 20:00 | Spec §8.3 names the picks but not their ends; a guest invited in the evening for "1 day" keeps the whole next day, a chosen date includes that whole day |
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including the eight `people::` contract tests on both backends and the
three `ExpiryPick` unit tests (`quick_picks_count_from_now`,
`the_weekend_lasts_until_the_coming_monday_in_local_time`,
`a_chosen_day_lasts_until_its_local_midnight`). With `TEST_POSTGRES_URL` set: **525** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/lib.rs \
  crates/domain/src/people.rs \
  crates/domain/src/repos.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/people.rs \
  crates/storage-postgres/src/users.rs \
  crates/storage-sqlite/src/users.rs \
  docs/DECISIONS.md
git commit -m "feat(people): account use cases as generic functions over the transaction traits

\`smoke_domain::people\` invites people, issues setup and reset links, accepts a link,
expires and reactivates accounts and changes passwords, checking permissions inside the
transaction. \`ExpiryPick\` resolves the guest quick picks in the configured time zone. The
storage contract runs every use case on SQLite and Postgres.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Settings — language and password

Spec §9 screen 7, and the carry-over that lets the second resident choose her own password. The
Settings page (linked from the header for everyone) shows the language buttons, which post to
the existing `/lang` with `return_to=settings`, and a password form. The current password is
checked like a sign-in attempt, under the same per-username and per-address limits; the change
ends every session of the account, then `auth::start_session` (now shared with sign-in) gives
this device a fresh session id, so only the other devices are signed out (decision #116). The
"How to rate" guide is linked under Guides. Two M4b minors ride along: the `:has()` rule in
`app.css` is split so browsers without `:has` keep the room headings on their own line (M-1),
and the CSP page scan now also covers the backdated no-smell question and a check's page (T3-f).

**Files:**
- Create: `crates/web/src/routes/settings.rs`, `crates/web/templates/settings.html`, `crates/web/tests/settings.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/auth/mod.rs`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/lang.rs`, `crates/web/src/routes/login.rs`, `crates/web/src/routes/mod.rs`, `crates/web/static/app.css`, `crates/web/templates/base.html`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `people::change_password` (Task 3); `LoginLimiter::{key, begin, succeeded}`,
  `ClientIp`, `password::{run_limited, verify, hash}` (milestone 3).
- Produces (web):
  - `auth::start_session(&Session, &User, now) -> Result<(), AppError>` (cycles the id, stores
    the user and the renewal time); `routes::login::submit` uses it.
  - `routes::people_refusal(PeopleError) -> AppError` (`Forbidden` → 403, `NotFound` → 404,
    anything else → 500).
  - `routes::settings::{LANGUAGES: [(Locale, &str); 3], LanguageChoice { value, name, current },
    show, change, keys()}`; routes `GET /settings`, `POST /settings/password`;
    `/lang` accepts `return_to=settings` → `/settings?saved=language`.
  - Translation keys (14): `nav-settings`, `settings-title`, `settings-password`,
    `settings-password-current`, `settings-password-new`, `settings-password-repeat`,
    `settings-password-hint`, `settings-password-submit`, `settings-password-mismatch`,
    `settings-password-length`, `settings-password-wrong`, `settings-saved-language`,
    `settings-saved-password`, `settings-guides`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index de3a1db..20b6aa9 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -181,8 +181,11 @@ fn the_event_handler_scan_finds_attributes_and_not_words() {
 #[tokio::test]
 async fn no_page_needs_inline_scripts_styles_or_eval() {
     let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
     app.add_user("daniele", Role::Resident, Locale::En).await;
     let cookie = app.sign_in("daniele").await;
+    app.send(post_form("/log", "action=no-smell", Some(&cookie)))
+        .await;
     app.send(post_form(
         "/log",
         "room=1&level-1=4&action=smell",
@@ -196,18 +199,30 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
             Some(&cookie),
         ))
         .await;
+    let past_check_question = app
+        .send(post_form(
+            "/log",
+            "action=no-smell&observed_at=2026-01-15T12:00",
+            Some(&cookie),
+        ))
+        .await;
     let pages = [
         text(question).await,
+        text(past_check_question).await,
         text(app.send(get_with("/events", &cookie)).await).await,
         text(app.send(get_with("/events/1", &cookie)).await).await,
         text(app.send(get_with("/reports/1", &cookie)).await).await,
-        text(app.send(get_with("/reports/1/correct", &cookie)).await).await,
+        text(app.send(get_with("/reports/2", &cookie)).await).await,
+        text(app.send(get_with("/reports/2/correct", &cookie)).await).await,
         text(app.send(get("/login")).await).await,
         text(app.send(get_with("/", &cookie)).await).await,
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
+        text(app.send(get_with("/settings", &cookie)).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
+    assert!(pages[1].contains("No event: just a check"), "{}", pages[1]);
+    assert!(pages[4].contains("None: a no-smell check"), "{}", pages[4]);
     for html in pages {
         assert!(
             html.contains(r#"<meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>"#),
diff --git a/crates/web/tests/settings.rs b/crates/web/tests/settings.rs
new file mode 100644
index 0000000..b9f62ea
--- /dev/null
+++ b/crates/web/tests/settings.rs
@@ -0,0 +1,154 @@
+//! Settings: language and password for everyone (spec §9 screen 7).
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use smoke_domain::repos::UserRepo;
+use smoke_domain::types::{Locale, Role};
+
+const NEW_PASSWORD: &str = "a brand new passphrase";
+
+fn change(current: &str, new: &str, repeat: &str) -> String {
+    format!("current={current}&new={new}&repeat={repeat}").replace(' ', "+")
+}
+
+#[tokio::test]
+async fn the_settings_page_shows_the_language_and_password_forms() {
+    let app = TestApp::new().await;
+    app.add_user("guest", Role::Guest, Locale::It).await;
+    let cookie = app.sign_in("guest").await;
+    let html = text(app.send(get_with("/settings", &cookie)).await).await;
+    assert!(html.contains("<h1>Impostazioni</h1>"), "{html}");
+    assert!(
+        html.contains(
+            r#"value="it" lang="it" class="current" aria-pressed="true">Italiano</button>"#
+        ),
+        "{html}"
+    );
+    assert!(html.contains(r#"action="/settings/password""#), "{html}");
+    assert!(
+        html.contains(r#"<a href="/settings">Impostazioni</a>"#),
+        "{html}"
+    );
+}
+
+#[tokio::test]
+async fn choosing_a_language_in_settings_saves_it_and_comes_back() {
+    let app = TestApp::new().await;
+    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let response = app
+        .send(post_form(
+            "/lang",
+            "lang=tr&return_to=settings",
+            Some(&cookie),
+        ))
+        .await;
+    assert_eq!(location(&response), Some("/settings?saved=language"));
+    let stored = app.store.user(user.id).await.unwrap().unwrap();
+    assert_eq!(stored.locale, Locale::Tr);
+    let html = text(
+        app.send(get_with("/settings?saved=language", &cookie))
+            .await,
+    )
+    .await;
+    assert!(html.contains("Dil kaydedildi."), "{html}");
+}
+
+#[tokio::test]
+async fn a_password_change_keeps_this_device_and_signs_out_the_others() {
+    let app = TestApp::new().await;
+    app.add_user("partner", Role::Resident, Locale::En).await;
+    let other_device = app.sign_in("partner").await;
+    let this_device = app.sign_in("partner").await;
+
+    let response = app
+        .send(post_form(
+            "/settings/password",
+            &change(PASSWORD, NEW_PASSWORD, NEW_PASSWORD),
+            Some(&this_device),
+        ))
+        .await;
+    assert_eq!(location(&response), Some("/settings?saved=password"));
+    let renewed = session_cookie(&response).expect("a new session for this device");
+    assert_ne!(renewed, this_device);
+
+    let page = app
+        .send(get_with("/settings?saved=password", &renewed))
+        .await;
+    assert_eq!(page.status(), StatusCode::OK);
+    assert!(
+        text(page)
+            .await
+            .contains("Your other devices are signed out.")
+    );
+    for ended in [&other_device, &this_device] {
+        let page = app.send(get_with("/", ended)).await;
+        assert_eq!(location(&page), Some("/login"), "{ended}");
+    }
+    assert_eq!(
+        app.attempt("partner", PASSWORD).await.status(),
+        StatusCode::UNAUTHORIZED
+    );
+    let response = app.send(login_request("partner", NEW_PASSWORD)).await;
+    assert_eq!(response.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn a_refused_password_change_says_why_and_changes_nothing() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let other_device = app.sign_in("daniele").await;
+    for (body, message) in [
+        (
+            change(PASSWORD, NEW_PASSWORD, "something else entirely"),
+            "The two new passwords do not match.",
+        ),
+        (
+            change(PASSWORD, "too short", "too short"),
+            "The new password needs 10 to 128 characters.",
+        ),
+        (
+            change("not the password", NEW_PASSWORD, NEW_PASSWORD),
+            "The current password is not correct.",
+        ),
+    ] {
+        let response = app
+            .send(post_form("/settings/password", &body, Some(&cookie)))
+            .await;
+        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+        let html = text(response).await;
+        assert!(html.contains(message), "{message}: {html}");
+    }
+    assert_eq!(
+        location(&app.send(get_with("/", &other_device)).await),
+        None,
+        "still signed in"
+    );
+    assert_eq!(
+        app.attempt("daniele", PASSWORD).await.status(),
+        StatusCode::SEE_OTHER
+    );
+}
+
+#[tokio::test]
+async fn guessing_the_current_password_locks_like_signing_in() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let wrong = change("not the password", NEW_PASSWORD, NEW_PASSWORD);
+    for _ in 0..5 {
+        let response = app
+            .send(post_form("/settings/password", &wrong, Some(&cookie)))
+            .await;
+        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+    }
+    let right = change(PASSWORD, NEW_PASSWORD, NEW_PASSWORD);
+    let response = app
+        .send(post_form("/settings/password", &right, Some(&cookie)))
+        .await;
+    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
+    assert!(text(response).await.contains("Too many attempts."));
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: the new tests in `crates/web/tests/settings.rs` fail (`/settings` and the password post
answer 404, `/lang` ignores `return_to`). The extended
`no_page_needs_inline_scripts_styles_or_eval` already passes: its new backdated no-smell
question and check page exist since milestone 4, and `/settings` is still the 404 page.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 0154e94..586b731 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Like a smoke-filled room. You must leave.
 
 nav-log = Log
 nav-events = Events
+nav-settings = Settings
 
 events-title = Events
 events-none = No events this month.
@@ -169,6 +170,20 @@ close-reason-new-event = closed when a new event started
 close-reason-auto = closed automatically overnight
 close-reason-backdated = logged afterwards
 
+settings-title = Settings
+settings-password = Password
+settings-password-current = Current password
+settings-password-new = New password
+settings-password-repeat = Repeat the new password
+settings-password-hint = 10 to 128 characters. Your other devices will be signed out.
+settings-password-submit = Change password
+settings-password-mismatch = The two new passwords do not match.
+settings-password-length = The new password needs 10 to 128 characters.
+settings-password-wrong = The current password is not correct.
+settings-saved-language = Language saved.
+settings-saved-password = Password changed. Your other devices are signed out.
+settings-guides = Guides
+
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index f3272dd..ca3c61a 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
 nav-log = Registra
 nav-events = Eventi
+nav-settings = Impostazioni
 
 events-title = Eventi
 events-none = Nessun evento questo mese.
@@ -169,6 +170,20 @@ close-reason-new-event = chiuso all'inizio di un nuovo evento
 close-reason-auto = chiuso automaticamente durante la notte
 close-reason-backdated = registrato in seguito
 
+settings-title = Impostazioni
+settings-password = Password
+settings-password-current = Password attuale
+settings-password-new = Nuova password
+settings-password-repeat = Ripeti la nuova password
+settings-password-hint = Da 10 a 128 caratteri. Gli altri dispositivi verranno disconnessi.
+settings-password-submit = Cambia password
+settings-password-mismatch = Le due nuove password non coincidono.
+settings-password-length = La nuova password deve avere da 10 a 128 caratteri.
+settings-password-wrong = La password attuale non è corretta.
+settings-saved-language = Lingua salvata.
+settings-saved-password = Password cambiata. Gli altri dispositivi sono stati disconnessi.
+settings-guides = Guide
+
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 9d02206..d79d0e9 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
 nav-log = Kaydet
 nav-events = Olaylar
+nav-settings = Ayarlar
 
 events-title = Olaylar
 events-none = Bu ay olay yok.
@@ -169,6 +170,20 @@ close-reason-new-event = yeni bir olay başlayınca kapatıldı
 close-reason-auto = gece otomatik olarak kapatıldı
 close-reason-backdated = sonradan kaydedildi
 
+settings-title = Ayarlar
+settings-password = Şifre
+settings-password-current = Mevcut şifre
+settings-password-new = Yeni şifre
+settings-password-repeat = Yeni şifreyi tekrarlayın
+settings-password-hint = 10 ile 128 karakter arası. Diğer cihazlarınızda oturum kapatılacak.
+settings-password-submit = Şifreyi değiştir
+settings-password-mismatch = İki yeni şifre aynı değil.
+settings-password-length = Yeni şifre 10 ile 128 karakter arasında olmalı.
+settings-password-wrong = Mevcut şifre doğru değil.
+settings-saved-language = Dil kaydedildi.
+settings-saved-password = Şifre değiştirildi. Diğer cihazlarınızda oturum kapatıldı.
+settings-guides = Kılavuzlar
+
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
diff --git a/crates/web/src/auth/mod.rs b/crates/web/src/auth/mod.rs
index ca416ab..7483ed8 100644
--- a/crates/web/src/auth/mod.rs
+++ b/crates/web/src/auth/mod.rs
@@ -92,6 +92,22 @@ async fn renew(session: &Session, now: DateTime<Utc>) -> Result<(), AppError> {
     Ok(())
 }
 
+/// Signs `user` in on this device with a fresh session id, so an id planted before sign-in is
+/// worthless (session fixation). Used at sign-in, after accepting a link, and after a password
+/// change, which ends every earlier session of the account.
+pub(crate) async fn start_session(
+    session: &Session,
+    user: &User,
+    now: DateTime<Utc>,
+) -> Result<(), AppError> {
+    session.cycle_id().await?;
+    session.insert(USER_ID_KEY, user.id.0).await?;
+    session
+        .insert(RENEWED_AT_KEY, now.timestamp_micros())
+        .await?;
+    Ok(())
+}
+
 /// Sends visitors who are not signed in to the login page; htmx requests get `HX-Redirect`
 /// so the whole page navigates instead of swapping the login form into a fragment.
 pub(crate) async fn require_user(request: Request, next: Next) -> Response {
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index 9e9178f..6f26143 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,7 +127,7 @@ mod tests {
 
     use super::*;
     use crate::routes::login::Notice;
-    use crate::routes::{self, entries, guide, reports};
+    use crate::routes::{self, entries, guide, reports, settings};
     use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -218,6 +218,7 @@ mod tests {
         used.extend(entries::FORM_ERRORS.map(str::to_owned));
         used.extend(routes::REFUSALS.map(str::to_owned));
         used.extend(reports::HISTORY_KEYS.map(str::to_owned));
+        used.extend(settings::keys().into_iter().map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 9b86ab0..7765460 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -56,6 +56,8 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::reports::correct_form::<S>).post(routes::reports::correct::<S>),
         )
         .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
+        .route("/settings", get(routes::settings::show))
+        .route("/settings/password", post(routes::settings::change::<S>))
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
diff --git a/crates/web/src/routes/lang.rs b/crates/web/src/routes/lang.rs
index ff7a716..618d9fc 100644
--- a/crates/web/src/routes/lang.rs
+++ b/crates/web/src/routes/lang.rs
@@ -16,6 +16,8 @@ use crate::state::AppState;
 #[derive(Deserialize)]
 pub(crate) struct LangForm {
     lang: String,
+    /// `settings` when posted from the Settings page, which it then shows again.
+    return_to: Option<String>,
 }
 
 /// Sets the `lang` cookie and, when signed in, saves the language to the account (audited).
@@ -25,7 +27,11 @@ pub(crate) async fn choose<S: Store + Clone + 'static>(
     user: Option<CurrentUser>,
     Form(form): Form<LangForm>,
 ) -> Result<Response, AppError> {
-    let target = if user.is_some() { "/" } else { "/login" };
+    let target = match (&user, form.return_to.as_deref()) {
+        (None, _) => "/login",
+        (Some(_), Some("settings")) => "/settings?saved=language",
+        (Some(_), _) => "/",
+    };
     let Ok(locale) = form.lang.parse::<Locale>() else {
         return Ok(Redirect::to(target).into_response());
     };
diff --git a/crates/web/src/routes/login.rs b/crates/web/src/routes/login.rs
index c975641..78decd7 100644
--- a/crates/web/src/routes/login.rs
+++ b/crates/web/src/routes/login.rs
@@ -13,11 +13,10 @@ use tower_sessions::Session;
 
 use crate::auth::client_ip::ClientIp;
 use crate::auth::limiter::LoginLimiter;
-use crate::auth::{CurrentUser, password};
+use crate::auth::{self, CurrentUser, password};
 use crate::cookies;
 use crate::error::{AppError, page};
 use crate::i18n::Tr;
-use crate::sessions::{RENEWED_AT_KEY, USER_ID_KEY};
 use crate::state::AppState;
 
 /// What the login page tells the user after a failed attempt.
@@ -131,11 +130,7 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
             Some(Notice::Expired),
         );
     }
-    session.cycle_id().await?;
-    session.insert(USER_ID_KEY, user.id.0).await?;
-    session
-        .insert(RENEWED_AT_KEY, now.timestamp_micros())
-        .await?;
+    auth::start_session(&session, &user, now).await?;
     tracing::info!(user = user.id.0, "signed in");
     let mut response = Redirect::to("/").into_response();
     response.headers_mut().append(
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index cfb3d6d..3d1466f 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -8,10 +8,12 @@ pub(crate) mod log;
 pub(crate) mod login;
 pub(crate) mod notices;
 pub(crate) mod reports;
+pub(crate) mod settings;
 
 use chrono::{DateTime, Utc};
 use smoke_domain::DomainError;
 use smoke_domain::logbook::LogError;
+use smoke_domain::people::PeopleError;
 use smoke_domain::permissions::{Actor, can_see};
 use smoke_domain::records::{Event, SmellReport};
 
@@ -53,6 +55,15 @@ pub(crate) fn refusal(error: LogError) -> Result<&'static str, AppError> {
     }
 }
 
+/// What an account use case's refusal becomes when the page has no message of its own for it.
+pub(crate) fn people_refusal(error: PeopleError) -> AppError {
+    match error {
+        PeopleError::Forbidden => AppError::NotAllowed,
+        PeopleError::NotFound => AppError::NotFound,
+        other => AppError::internal(other),
+    }
+}
+
 /// Every key [`refusal`] can return, for the translation tests.
 #[cfg(test)]
 pub(crate) const REFUSALS: [&str; 8] = [
diff --git a/crates/web/src/routes/settings.rs b/crates/web/src/routes/settings.rs
new file mode 100644
index 0000000..1ff43a3
--- /dev/null
+++ b/crates/web/src/routes/settings.rs
@@ -0,0 +1,188 @@
+//! Settings (spec §9 screen 7): language and password for everyone.
+
+use askama::Template;
+use axum::Form;
+use axum::extract::{Query, State};
+use axum::http::StatusCode;
+use axum::response::{IntoResponse, Redirect, Response};
+use serde::Deserialize;
+use smoke_domain::people::change_password;
+use smoke_domain::records::User;
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{Locale, NewPassword};
+use tower_sessions::Session;
+
+use crate::auth::client_ip::ClientIp;
+use crate::auth::limiter::LoginLimiter;
+use crate::auth::{self, CurrentUser, password};
+use crate::error::{AppError, page};
+use crate::i18n::Tr;
+use crate::routes::people_refusal;
+use crate::state::AppState;
+
+/// The languages, each named in itself.
+pub(crate) const LANGUAGES: [(Locale, &str); 3] = [
+    (Locale::En, "English"),
+    (Locale::It, "Italiano"),
+    (Locale::Tr, "Türkçe"),
+];
+
+pub(crate) struct LanguageChoice {
+    pub(crate) value: &'static str,
+    pub(crate) name: &'static str,
+    pub(crate) current: bool,
+}
+
+#[derive(Template)]
+#[template(path = "settings.html")]
+struct SettingsPage {
+    tr: Tr,
+    signed_in: Option<String>,
+    /// For password managers, which file the new password under this name.
+    username: String,
+    languages: Vec<LanguageChoice>,
+    /// A translation key confirming what was just saved.
+    saved: Option<&'static str>,
+    /// A translation key saying why the password was not changed.
+    password_error: Option<&'static str>,
+}
+
+fn settings_page(
+    status: StatusCode,
+    user: &User,
+    tr: Tr,
+    saved: Option<&'static str>,
+    password_error: Option<&'static str>,
+) -> Result<Response, AppError> {
+    page(
+        status,
+        &SettingsPage {
+            tr,
+            signed_in: Some(user.username.to_string()),
+            username: user.username.to_string(),
+            languages: LANGUAGES
+                .iter()
+                .map(|&(locale, name)| LanguageChoice {
+                    value: locale.as_str(),
+                    name,
+                    current: locale == user.locale,
+                })
+                .collect(),
+            saved,
+            password_error,
+        },
+    )
+}
+
+#[derive(Deserialize)]
+pub(crate) struct ShowQuery {
+    saved: Option<String>,
+}
+
+pub(crate) async fn show(
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Query(query): Query<ShowQuery>,
+) -> Result<Response, AppError> {
+    let saved = match query.saved.as_deref() {
+        Some("language") => Some("settings-saved-language"),
+        Some("password") => Some("settings-saved-password"),
+        _ => None,
+    };
+    settings_page(StatusCode::OK, &user, tr, saved, None)
+}
+
+#[derive(Deserialize)]
+pub(crate) struct PasswordForm {
+    current: String,
+    new: String,
+    repeat: String,
+}
+
+/// Why a password change was refused, as the message shown above the form.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+enum Refused {
+    Mismatch,
+    Length,
+    WrongCurrent,
+    Locked,
+}
+
+impl Refused {
+    #[cfg(test)]
+    const ALL: [Refused; 4] = [
+        Refused::Mismatch,
+        Refused::Length,
+        Refused::WrongCurrent,
+        Refused::Locked,
+    ];
+
+    fn key(self) -> &'static str {
+        match self {
+            Self::Mismatch => "settings-password-mismatch",
+            Self::Length => "settings-password-length",
+            Self::WrongCurrent => "settings-password-wrong",
+            Self::Locked => "login-locked",
+        }
+    }
+
+    fn status(self) -> StatusCode {
+        match self {
+            Self::Locked => StatusCode::TOO_MANY_REQUESTS,
+            _ => StatusCode::UNPROCESSABLE_ENTITY,
+        }
+    }
+}
+
+/// Changes the signed-in person's password (decision #116). The current password is checked
+/// like a sign-in attempt, under the same limits. Every session of the account ends; this
+/// device gets a new one, so only the other devices are signed out.
+pub(crate) async fn change<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    session: Session,
+    ClientIp(address): ClientIp,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Form(form): Form<PasswordForm>,
+) -> Result<Response, AppError> {
+    let refuse =
+        |refused: Refused| settings_page(refused.status(), &user, tr, None, Some(refused.key()));
+    if form.new != form.repeat {
+        return refuse(Refused::Mismatch);
+    }
+    let Ok(new_password) = NewPassword::new(form.new) else {
+        return refuse(Refused::Length);
+    };
+    let now = state.clock.now();
+    let key = LoginLimiter::key(user.username.as_str());
+    if !state.limiter.begin(&key, address, now) {
+        return refuse(Refused::Locked);
+    }
+    let stored = user.password_hash.clone();
+    let current = form.current;
+    let new_hash = password::run_limited(state.hash_permits.clone(), move || {
+        password::verify(stored.as_ref(), &current).then(|| password::hash(&new_password))
+    })
+    .await?;
+    let Some(new_hash) = new_hash else {
+        return refuse(Refused::WrongCurrent);
+    };
+    state.limiter.succeeded(&key, address, now);
+    let new_hash = new_hash.map_err(AppError::internal)?;
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    let changed = change_password(&mut tx, &user.actor(), new_hash, now)
+        .await
+        .map_err(people_refusal)?;
+    tx.commit().await?;
+    auth::start_session(&session, &changed, now).await?;
+    tracing::info!(user = user.id.0, "password changed");
+    Ok(Redirect::to("/settings?saved=password").into_response())
+}
+
+/// Every key this module looks up through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    let mut keys: Vec<_> = Refused::ALL.iter().map(|r| r.key()).collect();
+    keys.extend(["settings-saved-language", "settings-saved-password"]);
+    keys
+}
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 8cee9b4..088aecc 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -99,7 +99,11 @@ legend {
 }
 
 .rooms legend,
-.rooms .group,
+.rooms .group {
+  flex-basis: 100%;
+  margin: 0;
+}
+
 .room:has(input[name="room"]:checked) {
   flex-basis: 100%;
   margin: 0;
@@ -271,3 +275,20 @@ select {
   padding: 0.75rem;
   border-radius: 0.5rem;
 }
+
+.choices {
+  display: flex;
+  flex-wrap: wrap;
+  gap: 0.5rem;
+}
+
+button.current {
+  background: var(--accent);
+  color: #fff;
+}
+
+.hint {
+  font-size: 0.875rem;
+  margin: 0;
+  opacity: 0.8;
+}
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index 5c23de1..ad6cf09 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -17,6 +17,7 @@
     <nav class="menu">
       <a href="/">{{ tr.t("nav-log") }}</a>
       <a href="/events">{{ tr.t("nav-events") }}</a>
+      <a href="/settings">{{ tr.t("nav-settings") }}</a>
     </nav>
     <form method="post" action="/logout" class="who">
       <span>{{ tr.t1("signed-in-as", "username", username) }}</span>
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
new file mode 100644
index 0000000..45a1461
--- /dev/null
+++ b/crates/web/templates/settings.html
@@ -0,0 +1,43 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("settings-title") }}{% endblock %}
+{% block content %}
+<h1>{{ tr.t("settings-title") }}</h1>
+{% if let Some(saved) = saved %}
+<p class="notice" role="status">{{ tr.t(saved) }}</p>
+{% endif %}
+<section>
+  <h2>{{ tr.t("language") }}</h2>
+  <form method="post" action="/lang" class="choices">
+    <input type="hidden" name="return_to" value="settings">
+    {% for language in languages %}
+    <button type="submit" name="lang" value="{{ language.value }}" lang="{{ language.value }}"{% if language.current %} class="current" aria-pressed="true"{% else %} class="secondary" aria-pressed="false"{% endif %}>{{ language.name }}</button>
+    {% endfor %}
+  </form>
+</section>
+<section>
+  <h2>{{ tr.t("settings-password") }}</h2>
+  {% if let Some(error) = password_error %}
+  <p class="notice" role="alert">{{ tr.t(error) }}</p>
+  {% endif %}
+  <form method="post" action="/settings/password" class="stack">
+    <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
+    <label>{{ tr.t("settings-password-current") }}
+      <input type="password" name="current" autocomplete="current-password" maxlength="128" required>
+    </label>
+    <label>{{ tr.t("settings-password-new") }}
+      <input type="password" name="new" autocomplete="new-password" minlength="10" maxlength="128" required>
+    </label>
+    <label>{{ tr.t("settings-password-repeat") }}
+      <input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>
+    </label>
+    <p class="hint">{{ tr.t("settings-password-hint") }}</p>
+    <button type="submit">{{ tr.t("settings-password-submit") }}</button>
+  </form>
+</section>
+<section>
+  <h2>{{ tr.t("settings-guides") }}</h2>
+  <ul class="rows">
+    <li><a href="/guide/rating">{{ tr.t("rating-title") }}</a></li>
+  </ul>
+</section>
+{% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 2f7ca1b..26bdc34 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -122,3 +122,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 113 | 2026-10-14 | Structure | The account use cases (**`smoke_domain::people`**: invite, new link, accept a link, expire, reactivate, change password) are generic functions over the write-transaction traits that check permissions against data read inside the transaction, like the logbook (#86); the storage contract runs them on both backends | Account rules in the web handlers | One place for the §4 and §8.3 rules, tested on SQLite and Postgres |
 | 114 | 2026-10-14 | Accounts | A new link for someone who **never chose a password is a setup link**, otherwise a password reset; accepting one sets the password and language and uses up the link in one transaction; **only an expired account can be reactivated**, and expiring an expired one changes nothing | Reactivate also changes an active guest's expiry; two buttons for setup and reset | Spec §4 reactivates expired accounts; a lost setup QR is fixed the same way as a forgotten password; a double tap on Expire cannot move the end later |
 | 115 | 2026-10-14 | Accounts | Guest expiry quick picks: **1 day, 1 week (default), 2 weeks** count from now; **weekend** lasts until the coming **Monday 00:00** local time (on a Monday, the next one); a **chosen day** lasts until the end of that local day | All picks end at midnight; weekend ends on Sunday at 20:00 | Spec §8.3 names the picks but not their ends; a guest invited in the evening for "1 day" keeps the whole next day, a chosen date includes that whole day |
+| 116 | 2026-10-14 | Settings | Changing one's password needs the **current password**, checked as a sign-in attempt under the same limits (#60); the change ends **every** session of the account (#58) and then **signs this device in again** with a new session id; the language buttons post to `/lang`, which returns to Settings | No current password (the session is enough); keep this device's session id | A phone left unlocked cannot take the account over or guess the password faster than at sign-in; the person who changed it stays signed in while every other device is signed out (spec §8.4) |
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including `a_password_change_keeps_this_device_and_signs_out_the_others`,
`a_refused_password_change_says_why_and_changes_nothing`,
`guessing_the_current_password_locks_like_signing_in` and
`every_key_the_pages_use_is_translated`. With `TEST_POSTGRES_URL` set: **530** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/auth/mod.rs \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/lang.rs \
  crates/web/src/routes/login.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/settings.rs \
  crates/web/static/app.css \
  crates/web/templates/base.html \
  crates/web/templates/settings.html \
  crates/web/tests/security.rs \
  crates/web/tests/settings.rs \
  docs/DECISIONS.md
git commit -m "feat(settings): language and password change, reusing the sign-in limiter

The Settings page sets the language and changes the password. The current password is
checked like a sign-in attempt; the change ends every session and starts a fresh one for
this device (\`auth::start_session\`, now shared with sign-in). The CSP page test also covers
the backdated no-smell question and a check's page.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Settings — room and symptom lists

Spec §4 ("Edit room & symptom lists: residents") and §9 screen 7. `smoke_domain::lists` checks
the edit-lists permission inside the transaction and adds custom items (no translation key,
shown as typed) or hides/shows any item; nothing is renamed or deleted, so old reports keep
reading the same and hidden items stay selectable when correcting them (#90). Settings shows
residents every room (with its area) and symptom, each with a Hide / Show again button, plus two
add forms (decision #117). The Settings page becomes asynchronous (it reads the lists), with its
notices grouped in `Notices`.

**Files:**
- Create: `crates/domain/src/lists.rs`, `crates/web/src/routes/lists.rs`, `crates/web/templates/list_item.html`
- Modify: `crates/domain/src/lib.rs`, `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/lists.rs`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/routes/settings.rs`, `crates/web/static/app.css`, `crates/web/templates/settings.html`, `crates/web/tests/settings.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `ListWrites::{create_room, set_room_hidden, create_symptom, set_symptom_hidden}`,
  `ListName`, `RoomArea`, `labels::{room, symptom}` (milestones 2–4); `Action::EditLists`.
- Produces:
  - `smoke_domain::lists::{ListError { Forbidden, NotFound, Store(StoreError) }, add_room(tx,
    &Actor, ListName, RoomArea, now) -> Result<Room>, set_room_hidden(tx, &Actor, RoomId, bool,
    now) -> Result<Room>, add_symptom(tx, &Actor, ListName, now) -> Result<Symptom>,
    set_symptom_hidden(tx, &Actor, SymptomId, bool, now) -> Result<Symptom>}`.
  - `routes::settings::{Notices { saved, password_error, list_error }, settings_page(&AppState,
    &User, Tr, StatusCode, Notices)}` (async; `show` now takes `State`).
  - `routes::lists::{ListItem { label, action, area, hidden }, ListsView { rooms, symptoms },
    view, create_room, hide_room, create_symptom, hide_symptom, KEYS}`; routes
    `POST /settings/rooms` (`name`, `area`), `POST /settings/rooms/{id}/hidden` (`hidden=true|false`),
    `POST /settings/symptoms`, `POST /settings/symptoms/{id}/hidden`; template
    `list_item.html` (expects `item`).
  - Translation keys (9): `settings-lists-hint`, `settings-list-hidden`, `settings-list-hide`,
    `settings-list-show`, `settings-room-add`, `settings-symptom-add`, `settings-list-add`,
    `settings-list-name-invalid`, `settings-saved-lists`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/storage-contract/src/lib.rs b/crates/storage-contract/src/lib.rs
index 30afcfb..86471d1 100644
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -48,6 +48,7 @@ macro_rules! contract_tests {
             lists::hiding_a_room_is_audited_once,
             lists::symptoms_can_be_created_and_hidden,
             lists::hiding_an_unknown_symptom_is_not_found,
+            lists::only_residents_edit_the_lists,
             events::a_new_event_can_be_the_open_event,
             events::only_one_event_can_be_open,
             events::closing_and_reopening_are_audited,
diff --git a/crates/storage-contract/src/lists.rs b/crates/storage-contract/src/lists.rs
index bb7dfcd..f865a01 100644
--- a/crates/storage-contract/src/lists.rs
+++ b/crates/storage-contract/src/lists.rs
@@ -2,13 +2,16 @@
 
 use chrono::TimeDelta;
 use smoke_domain::audit::{AuditAction, AuditEntity};
+use smoke_domain::lists::{ListError, add_room, add_symptom, set_room_hidden, set_symptom_hidden};
 use smoke_domain::records::{NewRoom, NewSymptom};
-use smoke_domain::repos::{AuditRepo, Commit, ListRepo, ListWrites, StoreError, Transactional};
+use smoke_domain::repos::{
+    AuditRepo, Commit, ListRepo, ListWrites, Store, StoreError, Transactional,
+};
 use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS, seed_starter_lists};
-use smoke_domain::types::{ListName, RoomArea, SymptomId};
+use smoke_domain::types::{ListName, Role, RoomArea, RoomId, SymptomId};
 
 use crate::Fixture;
-use crate::fixture::start;
+use crate::fixture::{add_user, start};
 
 pub async fn seeding_adds_the_starter_lists_once<S>(fx: &Fixture<S>)
 where
@@ -142,3 +145,48 @@ where
         "{error:?}"
     );
 }
+
+pub async fn only_residents_edit_the_lists<S: Store>(fx: &Fixture<S>) {
+    seed_starter_lists(&fx.store).await.unwrap();
+    let resident = add_user(fx, "daniele", Role::Resident).await;
+    let guest = add_user(fx, "guest", Role::Guest).await;
+    let viewer = add_user(fx, "lawyer", Role::Viewer).await;
+    let balcony = ListName::new("Balcony").unwrap();
+    let entries = fx.store.audit_log().await.unwrap().len();
+
+    for outsider in [&guest, &viewer] {
+        let actor = outsider.actor();
+        let mut tx = fx.store.begin(Some(outsider.id)).await.unwrap();
+        let refused = [
+            add_room(&mut tx, &actor, balcony.clone(), RoomArea::Common, start())
+                .await
+                .map(|_| ()),
+            set_room_hidden(&mut tx, &actor, RoomId(1), true, start())
+                .await
+                .map(|_| ()),
+            add_symptom(&mut tx, &actor, balcony.clone(), start())
+                .await
+                .map(|_| ()),
+            set_symptom_hidden(&mut tx, &actor, SymptomId(1), true, start())
+                .await
+                .map(|_| ()),
+        ];
+        for result in refused {
+            assert!(matches!(result, Err(ListError::Forbidden)), "{result:?}");
+        }
+    }
+    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries);
+
+    let actor = resident.actor();
+    let mut tx = fx.store.begin(Some(resident.id)).await.unwrap();
+    let room = add_room(&mut tx, &actor, balcony, RoomArea::Common, start())
+        .await
+        .unwrap();
+    assert_eq!((room.i18n_key, room.area), (None, RoomArea::Common));
+    let hidden = set_room_hidden(&mut tx, &actor, room.id, true, start())
+        .await
+        .unwrap();
+    assert_eq!(hidden.hidden_at, Some(start()));
+    let missing = set_symptom_hidden(&mut tx, &actor, SymptomId(99), true, start()).await;
+    assert!(matches!(missing, Err(ListError::NotFound)), "{missing:?}");
+}
diff --git a/crates/web/tests/settings.rs b/crates/web/tests/settings.rs
index b9f62ea..2556ef2 100644
--- a/crates/web/tests/settings.rs
+++ b/crates/web/tests/settings.rs
@@ -4,8 +4,8 @@ mod common;
 
 use axum::http::StatusCode;
 use common::*;
-use smoke_domain::repos::UserRepo;
-use smoke_domain::types::{Locale, Role};
+use smoke_domain::repos::{ListRepo, UserRepo};
+use smoke_domain::types::{Locale, Role, RoomArea};
 
 const NEW_PASSWORD: &str = "a brand new passphrase";
 
@@ -152,3 +152,112 @@ async fn guessing_the_current_password_locks_like_signing_in() {
     assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
     assert!(text(response).await.contains("Too many attempts."));
 }
+
+#[tokio::test]
+async fn residents_add_rooms_and_symptoms_and_hide_them_from_the_forms() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+
+    for (path, body) in [
+        ("/settings/rooms", "name=Balcony&area=common"),
+        ("/settings/symptoms", "name=Dizziness"),
+    ] {
+        let response = app.send(post_form(path, body, Some(&cookie))).await;
+        assert_eq!(location(&response), Some("/settings?saved=lists"), "{path}");
+    }
+    let rooms = app.store.rooms().await.unwrap();
+    let balcony = rooms.last().unwrap();
+    assert_eq!(
+        (
+            balcony.name.as_str(),
+            balcony.area,
+            balcony.i18n_key.as_deref()
+        ),
+        ("Balcony", RoomArea::Common, None)
+    );
+    let log = text(app.send(get_with("/", &cookie)).await).await;
+    assert!(log.contains("<span>Balcony</span>"), "{log}");
+    assert!(log.contains("Dizziness"), "{log}");
+
+    let hide = format!("/settings/rooms/{}/hidden", balcony.id);
+    let response = app
+        .send(post_form(&hide, "hidden=true", Some(&cookie)))
+        .await;
+    assert_eq!(location(&response), Some("/settings?saved=lists"));
+    let log = text(app.send(get_with("/", &cookie)).await).await;
+    assert!(!log.contains("Balcony"), "{log}");
+    let settings = text(app.send(get_with("/settings?saved=lists", &cookie)).await).await;
+    assert!(settings.contains("Lists saved."), "{settings}");
+    assert!(
+        settings.contains("Balcony · Common areas · <em>hidden</em>"),
+        "{settings}"
+    );
+    assert!(settings.contains(">Show again</button>"), "{settings}");
+
+    app.send(post_form(&hide, "hidden=false", Some(&cookie)))
+        .await;
+    let log = text(app.send(get_with("/", &cookie)).await).await;
+    assert!(log.contains("<span>Balcony</span>"), "{log}");
+}
+
+#[tokio::test]
+async fn a_name_must_have_one_to_sixty_characters() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let long = format!("name={}", "x".repeat(61));
+    for (path, body) in [
+        ("/settings/rooms", "name=+++&area=inside"),
+        ("/settings/symptoms", long.as_str()),
+    ] {
+        let response = app.send(post_form(path, body, Some(&cookie))).await;
+        assert_eq!(
+            response.status(),
+            StatusCode::UNPROCESSABLE_ENTITY,
+            "{path}"
+        );
+        let html = text(response).await;
+        assert!(html.contains("A name needs 1 to 60 characters."), "{html}");
+    }
+    assert_eq!(app.store.rooms().await.unwrap().len(), 6);
+    assert_eq!(app.store.symptoms().await.unwrap().len(), 6);
+}
+
+#[tokio::test]
+async fn only_residents_see_and_edit_the_lists() {
+    let app = TestApp::new().await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    app.add_user("lawyer", Role::Viewer, Locale::En).await;
+    for name in ["guest", "lawyer"] {
+        let cookie = app.sign_in(name).await;
+        let html = text(app.send(get_with("/settings", &cookie)).await).await;
+        assert!(!html.contains("/settings/rooms"), "{name}: {html}");
+        for (path, body) in [
+            ("/settings/rooms", "name=Balcony&area=common"),
+            ("/settings/rooms/1/hidden", "hidden=true"),
+            ("/settings/symptoms", "name=Dizziness"),
+            ("/settings/symptoms/1/hidden", "hidden=true"),
+        ] {
+            let response = app.send(post_form(path, body, Some(&cookie))).await;
+            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name} {path}");
+        }
+    }
+    assert_eq!(app.store.rooms().await.unwrap().len(), 6);
+    assert!(app.store.rooms().await.unwrap()[0].hidden_at.is_none());
+}
+
+#[tokio::test]
+async fn hiding_an_unknown_item_is_not_found() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let response = app
+        .send(post_form(
+            "/settings/rooms/99/hidden",
+            "hidden=true",
+            Some(&cookie),
+        ))
+        .await;
+    assert_eq!(response.status(), StatusCode::NOT_FOUND);
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: compile errors — `smoke_domain::lists` does not exist (the contract test imports it).

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/domain/src/lib.rs b/crates/domain/src/lib.rs
index 1429662..b9f821c 100644
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -8,6 +8,7 @@ pub mod clock;
 pub mod error;
 pub mod events;
 pub mod history;
+pub mod lists;
 pub mod logbook;
 pub mod people;
 pub mod permissions;
diff --git a/crates/domain/src/lists.rs b/crates/domain/src/lists.rs
new file mode 100644
index 0000000..f1a7bfd
--- /dev/null
+++ b/crates/domain/src/lists.rs
@@ -0,0 +1,92 @@
+//! Editing the room and symptom lists (spec §4: residents only). Items are hidden, never
+//! deleted, so the reports that name them keep their names; custom items have no translation
+//! key and are shown as typed.
+
+use chrono::{DateTime, Utc};
+use thiserror::Error;
+
+use crate::permissions::{Action, Actor, can};
+use crate::records::{NewRoom, NewSymptom, Room, Symptom};
+use crate::repos::{ListWrites, StoreError};
+use crate::types::{ListName, RoomArea, RoomId, SymptomId};
+
+#[derive(Debug, Error)]
+pub enum ListError {
+    #[error("not allowed")]
+    Forbidden,
+    #[error("not found")]
+    NotFound,
+    #[error(transparent)]
+    Store(StoreError),
+}
+
+impl From<StoreError> for ListError {
+    fn from(error: StoreError) -> Self {
+        match error {
+            StoreError::NotFound { .. } => Self::NotFound,
+            other => Self::Store(other),
+        }
+    }
+}
+
+fn require_editor(actor: &Actor, now: DateTime<Utc>) -> Result<(), ListError> {
+    if can(actor, Action::EditLists, now) {
+        Ok(())
+    } else {
+        Err(ListError::Forbidden)
+    }
+}
+
+pub async fn add_room<T: ListWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    name: ListName,
+    area: RoomArea,
+    now: DateTime<Utc>,
+) -> Result<Room, ListError> {
+    require_editor(actor, now)?;
+    let new = NewRoom {
+        name,
+        i18n_key: None,
+        area,
+    };
+    Ok(tx.create_room(new).await?)
+}
+
+/// Hiding stops offering the room on the forms; showing offers it again.
+pub async fn set_room_hidden<T: ListWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: RoomId,
+    hidden: bool,
+    now: DateTime<Utc>,
+) -> Result<Room, ListError> {
+    require_editor(actor, now)?;
+    Ok(tx.set_room_hidden(id, hidden).await?)
+}
+
+pub async fn add_symptom<T: ListWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    name: ListName,
+    now: DateTime<Utc>,
+) -> Result<Symptom, ListError> {
+    require_editor(actor, now)?;
+    let new = NewSymptom {
+        name,
+        i18n_key: None,
+    };
+    Ok(tx.create_symptom(new).await?)
+}
+
+/// Hiding stops offering the symptom on the forms; showing offers it again.
+pub async fn set_symptom_hidden<T: ListWrites>(
+    tx: &mut T,
+    actor: &Actor,
+    id: SymptomId,
+    hidden: bool,
+    now: DateTime<Utc>,
+) -> Result<Symptom, ListError> {
+    require_editor(actor, now)?;
+    Ok(tx.set_symptom_hidden(id, hidden).await?)
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 586b731..50d595a 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -183,6 +183,15 @@ settings-password-wrong = The current password is not correct.
 settings-saved-language = Language saved.
 settings-saved-password = Password changed. Your other devices are signed out.
 settings-guides = Guides
+settings-lists-hint = Hidden items are no longer offered on the forms; reports that name them keep them.
+settings-list-hidden = hidden
+settings-list-hide = Hide
+settings-list-show = Show again
+settings-room-add = New room
+settings-symptom-add = New symptom
+settings-list-add = Add
+settings-list-name-invalid = A name needs 1 to 60 characters.
+settings-saved-lists = Lists saved.
 
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index ca3c61a..7b1e375 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -183,6 +183,15 @@ settings-password-wrong = La password attuale non è corretta.
 settings-saved-language = Lingua salvata.
 settings-saved-password = Password cambiata. Gli altri dispositivi sono stati disconnessi.
 settings-guides = Guide
+settings-lists-hint = Le voci nascoste non sono più proposte nei moduli; le segnalazioni che le citano le mantengono.
+settings-list-hidden = nascosto
+settings-list-hide = Nascondi
+settings-list-show = Mostra di nuovo
+settings-room-add = Nuova stanza
+settings-symptom-add = Nuovo sintomo
+settings-list-add = Aggiungi
+settings-list-name-invalid = Un nome deve avere da 1 a 60 caratteri.
+settings-saved-lists = Elenchi salvati.
 
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index d79d0e9..b6831c1 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -183,6 +183,15 @@ settings-password-wrong = Mevcut şifre doğru değil.
 settings-saved-language = Dil kaydedildi.
 settings-saved-password = Şifre değiştirildi. Diğer cihazlarınızda oturum kapatıldı.
 settings-guides = Kılavuzlar
+settings-lists-hint = Gizlenen öğeler artık formlarda sunulmaz; onları içeren kayıtlar korunur.
+settings-list-hidden = gizli
+settings-list-hide = Gizle
+settings-list-show = Yeniden göster
+settings-room-add = Yeni oda
+settings-symptom-add = Yeni belirti
+settings-list-add = Ekle
+settings-list-name-invalid = Bir ad 1 ile 60 karakter arasında olmalı.
+settings-saved-lists = Listeler kaydedildi.
 
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index 6f26143..760593b 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,7 +127,7 @@ mod tests {
 
     use super::*;
     use crate::routes::login::Notice;
-    use crate::routes::{self, entries, guide, reports, settings};
+    use crate::routes::{self, entries, guide, lists, reports, settings};
     use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -219,6 +219,7 @@ mod tests {
         used.extend(routes::REFUSALS.map(str::to_owned));
         used.extend(reports::HISTORY_KEYS.map(str::to_owned));
         used.extend(settings::keys().into_iter().map(str::to_owned));
+        used.extend(lists::KEYS.map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 7765460..4515b89 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -56,8 +56,21 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::reports::correct_form::<S>).post(routes::reports::correct::<S>),
         )
         .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
-        .route("/settings", get(routes::settings::show))
+        .route("/settings", get(routes::settings::show::<S>))
         .route("/settings/password", post(routes::settings::change::<S>))
+        .route("/settings/rooms", post(routes::lists::create_room::<S>))
+        .route(
+            "/settings/rooms/{id}/hidden",
+            post(routes::lists::hide_room::<S>),
+        )
+        .route(
+            "/settings/symptoms",
+            post(routes::lists::create_symptom::<S>),
+        )
+        .route(
+            "/settings/symptoms/{id}/hidden",
+            post(routes::lists::hide_symptom::<S>),
+        )
         .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
         .merge(signed_in)
diff --git a/crates/web/src/routes/lists.rs b/crates/web/src/routes/lists.rs
new file mode 100644
index 0000000..c53e0a8
--- /dev/null
+++ b/crates/web/src/routes/lists.rs
@@ -0,0 +1,182 @@
+//! Editing the room and symptom lists from Settings (spec §9 screen 7, residents only). Items
+//! are added or hidden, never renamed or deleted, so old reports keep reading the same.
+
+use axum::Form;
+use axum::extract::{Path, State};
+use axum::http::StatusCode;
+use axum::response::{IntoResponse, Redirect, Response};
+use serde::Deserialize;
+use smoke_domain::lists::{ListError, add_room, add_symptom, set_room_hidden, set_symptom_hidden};
+use smoke_domain::records::User;
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{ListName, RoomArea, RoomId, SymptomId};
+
+use crate::auth::CurrentUser;
+use crate::error::AppError;
+use crate::i18n::Tr;
+use crate::labels;
+use crate::routes::settings::{Notices, settings_page};
+use crate::state::AppState;
+
+/// A room or symptom as Settings lists it.
+pub(crate) struct ListItem {
+    pub(crate) label: String,
+    /// Where its Hide / Show button posts.
+    pub(crate) action: String,
+    /// The translation key of a room's area; `None` for symptoms.
+    pub(crate) area: Option<&'static str>,
+    pub(crate) hidden: bool,
+}
+
+pub(crate) struct ListsView {
+    pub(crate) rooms: Vec<ListItem>,
+    pub(crate) symptoms: Vec<ListItem>,
+}
+
+fn area_key(area: RoomArea) -> &'static str {
+    match area {
+        RoomArea::Inside => "log-inside",
+        RoomArea::Common => "log-common",
+    }
+}
+
+/// Every room and symptom, hidden ones included, in the order the forms show them.
+pub(crate) async fn view<S: Store>(state: &AppState<S>, tr: Tr) -> Result<ListsView, AppError> {
+    let rooms = state.store.rooms().await?;
+    let symptoms = state.store.symptoms().await?;
+    Ok(ListsView {
+        rooms: rooms
+            .iter()
+            .map(|room| ListItem {
+                action: format!("/settings/rooms/{}/hidden", room.id),
+                label: labels::room(tr, room),
+                area: Some(area_key(room.area)),
+                hidden: room.hidden_at.is_some(),
+            })
+            .collect(),
+        symptoms: symptoms
+            .iter()
+            .map(|symptom| ListItem {
+                action: format!("/settings/symptoms/{}/hidden", symptom.id),
+                label: labels::symptom(tr, symptom),
+                area: None,
+                hidden: symptom.hidden_at.is_some(),
+            })
+            .collect(),
+    })
+}
+
+fn refusal(error: ListError) -> AppError {
+    match error {
+        ListError::Forbidden => AppError::NotAllowed,
+        ListError::NotFound => AppError::NotFound,
+        ListError::Store(error) => error.into(),
+    }
+}
+
+fn saved() -> Response {
+    Redirect::to("/settings?saved=lists").into_response()
+}
+
+async fn refuse<S: Store>(
+    state: &AppState<S>,
+    user: &User,
+    tr: Tr,
+    key: &'static str,
+) -> Result<Response, AppError> {
+    let notices = Notices {
+        list_error: Some(key),
+        ..Notices::default()
+    };
+    settings_page(state, user, tr, StatusCode::UNPROCESSABLE_ENTITY, notices).await
+}
+
+#[derive(Deserialize)]
+pub(crate) struct NewItemForm {
+    name: String,
+    /// Rooms only: `inside` or `common`.
+    area: Option<String>,
+}
+
+#[derive(Deserialize)]
+pub(crate) struct HiddenForm {
+    hidden: bool,
+}
+
+pub(crate) async fn create_room<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Form(form): Form<NewItemForm>,
+) -> Result<Response, AppError> {
+    let Ok(name) = ListName::new(&form.name) else {
+        return refuse(&state, &user, tr, "settings-list-name-invalid").await;
+    };
+    let Some(area) = form.area.and_then(|area| area.parse().ok()) else {
+        return refuse(&state, &user, tr, "log-error-form").await;
+    };
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    add_room(&mut tx, &user.actor(), name, area, now)
+        .await
+        .map_err(refusal)?;
+    tx.commit().await?;
+    Ok(saved())
+}
+
+pub(crate) async fn hide_room<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    Path(id): Path<i64>,
+    Form(form): Form<HiddenForm>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    set_room_hidden(&mut tx, &user.actor(), RoomId(id), form.hidden, now)
+        .await
+        .map_err(refusal)?;
+    tx.commit().await?;
+    Ok(saved())
+}
+
+pub(crate) async fn create_symptom<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Form(form): Form<NewItemForm>,
+) -> Result<Response, AppError> {
+    let Ok(name) = ListName::new(&form.name) else {
+        return refuse(&state, &user, tr, "settings-list-name-invalid").await;
+    };
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    add_symptom(&mut tx, &user.actor(), name, now)
+        .await
+        .map_err(refusal)?;
+    tx.commit().await?;
+    Ok(saved())
+}
+
+pub(crate) async fn hide_symptom<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    Path(id): Path<i64>,
+    Form(form): Form<HiddenForm>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    set_symptom_hidden(&mut tx, &user.actor(), SymptomId(id), form.hidden, now)
+        .await
+        .map_err(refusal)?;
+    tx.commit().await?;
+    Ok(saved())
+}
+
+/// Every key this module passes through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) const KEYS: [&str; 4] = [
+    "settings-list-name-invalid",
+    "log-error-form",
+    "log-inside",
+    "log-common",
+];
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index 3d1466f..7ec2892 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -4,6 +4,7 @@ pub(crate) mod entries;
 pub(crate) mod events;
 pub(crate) mod guide;
 pub(crate) mod lang;
+pub(crate) mod lists;
 pub(crate) mod log;
 pub(crate) mod login;
 pub(crate) mod notices;
diff --git a/crates/web/src/routes/settings.rs b/crates/web/src/routes/settings.rs
index 1ff43a3..c02a600 100644
--- a/crates/web/src/routes/settings.rs
+++ b/crates/web/src/routes/settings.rs
@@ -1,4 +1,5 @@
-//! Settings (spec §9 screen 7): language and password for everyone.
+//! Settings (spec §9 screen 7): language and password for everyone, and the room and symptom
+//! lists for residents (edited in [`crate::routes::lists`]).
 
 use askama::Template;
 use axum::Form;
@@ -7,6 +8,7 @@ use axum::http::StatusCode;
 use axum::response::{IntoResponse, Redirect, Response};
 use serde::Deserialize;
 use smoke_domain::people::change_password;
+use smoke_domain::permissions::{Action, can};
 use smoke_domain::records::User;
 use smoke_domain::repos::{Commit, Store};
 use smoke_domain::types::{Locale, NewPassword};
@@ -17,6 +19,7 @@ use crate::auth::limiter::LoginLimiter;
 use crate::auth::{self, CurrentUser, password};
 use crate::error::{AppError, page};
 use crate::i18n::Tr;
+use crate::routes::lists::{self, ListsView};
 use crate::routes::people_refusal;
 use crate::state::AppState;
 
@@ -33,6 +36,14 @@ pub(crate) struct LanguageChoice {
     pub(crate) current: bool,
 }
 
+/// The messages above the page's sections: what was just saved, or why a form was refused.
+#[derive(Debug, Clone, Copy, Default)]
+pub(crate) struct Notices {
+    pub(crate) saved: Option<&'static str>,
+    pub(crate) password_error: Option<&'static str>,
+    pub(crate) list_error: Option<&'static str>,
+}
+
 #[derive(Template)]
 #[template(path = "settings.html")]
 struct SettingsPage {
@@ -41,19 +52,23 @@ struct SettingsPage {
     /// For password managers, which file the new password under this name.
     username: String,
     languages: Vec<LanguageChoice>,
-    /// A translation key confirming what was just saved.
-    saved: Option<&'static str>,
-    /// A translation key saying why the password was not changed.
-    password_error: Option<&'static str>,
+    notices: Notices,
+    /// `None` for people who may not edit the lists.
+    lists: Option<ListsView>,
 }
 
-fn settings_page(
-    status: StatusCode,
+pub(crate) async fn settings_page<S: Store>(
+    state: &AppState<S>,
     user: &User,
     tr: Tr,
-    saved: Option<&'static str>,
-    password_error: Option<&'static str>,
+    status: StatusCode,
+    notices: Notices,
 ) -> Result<Response, AppError> {
+    let lists = if can(&user.actor(), Action::EditLists, state.clock.now()) {
+        Some(lists::view(state, tr).await?)
+    } else {
+        None
+    };
     page(
         status,
         &SettingsPage {
@@ -68,8 +83,8 @@ fn settings_page(
                     current: locale == user.locale,
                 })
                 .collect(),
-            saved,
-            password_error,
+            notices,
+            lists,
         },
     )
 }
@@ -79,7 +94,8 @@ pub(crate) struct ShowQuery {
     saved: Option<String>,
 }
 
-pub(crate) async fn show(
+pub(crate) async fn show<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
     Query(query): Query<ShowQuery>,
@@ -87,9 +103,14 @@ pub(crate) async fn show(
     let saved = match query.saved.as_deref() {
         Some("language") => Some("settings-saved-language"),
         Some("password") => Some("settings-saved-password"),
+        Some("lists") => Some("settings-saved-lists"),
         _ => None,
     };
-    settings_page(StatusCode::OK, &user, tr, saved, None)
+    let notices = Notices {
+        saved,
+        ..Notices::default()
+    };
+    settings_page(&state, &user, tr, StatusCode::OK, notices).await
 }
 
 #[derive(Deserialize)]
@@ -145,18 +166,23 @@ pub(crate) async fn change<S: Store + Clone + 'static>(
     tr: Tr,
     Form(form): Form<PasswordForm>,
 ) -> Result<Response, AppError> {
-    let refuse =
-        |refused: Refused| settings_page(refused.status(), &user, tr, None, Some(refused.key()));
+    let refuse = async |refused: Refused| {
+        let notices = Notices {
+            password_error: Some(refused.key()),
+            ..Notices::default()
+        };
+        settings_page(&state, &user, tr, refused.status(), notices).await
+    };
     if form.new != form.repeat {
-        return refuse(Refused::Mismatch);
+        return refuse(Refused::Mismatch).await;
     }
     let Ok(new_password) = NewPassword::new(form.new) else {
-        return refuse(Refused::Length);
+        return refuse(Refused::Length).await;
     };
     let now = state.clock.now();
     let key = LoginLimiter::key(user.username.as_str());
     if !state.limiter.begin(&key, address, now) {
-        return refuse(Refused::Locked);
+        return refuse(Refused::Locked).await;
     }
     let stored = user.password_hash.clone();
     let current = form.current;
@@ -165,7 +191,7 @@ pub(crate) async fn change<S: Store + Clone + 'static>(
     })
     .await?;
     let Some(new_hash) = new_hash else {
-        return refuse(Refused::WrongCurrent);
+        return refuse(Refused::WrongCurrent).await;
     };
     state.limiter.succeeded(&key, address, now);
     let new_hash = new_hash.map_err(AppError::internal)?;
@@ -183,6 +209,10 @@ pub(crate) async fn change<S: Store + Clone + 'static>(
 #[cfg(test)]
 pub(crate) fn keys() -> Vec<&'static str> {
     let mut keys: Vec<_> = Refused::ALL.iter().map(|r| r.key()).collect();
-    keys.extend(["settings-saved-language", "settings-saved-password"]);
+    keys.extend([
+        "settings-saved-language",
+        "settings-saved-password",
+        "settings-saved-lists",
+    ]);
     keys
 }
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 088aecc..73da96b 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -292,3 +292,15 @@ button.current {
   margin: 0;
   opacity: 0.8;
 }
+
+.items {
+  list-style: none;
+  padding: 0;
+}
+
+.item {
+  display: flex;
+  align-items: center;
+  justify-content: space-between;
+  gap: var(--gap);
+}
diff --git a/crates/web/templates/list_item.html b/crates/web/templates/list_item.html
new file mode 100644
index 0000000..ee8f1b7
--- /dev/null
+++ b/crates/web/templates/list_item.html
@@ -0,0 +1,13 @@
+{# A room or symptom in Settings, with the button that hides or shows it again. Expects `item`. #}
+<li>
+  <form method="post" action="{{ item.action }}" class="item">
+    <span>{{ item.label }}{% if let Some(area) = item.area %} · {{ tr.t(area) }}{% endif %}{% if item.hidden %} · <em>{{ tr.t("settings-list-hidden") }}</em>{% endif %}</span>
+    {% if item.hidden %}
+    <input type="hidden" name="hidden" value="false">
+    <button type="submit" class="link">{{ tr.t("settings-list-show") }}</button>
+    {% else %}
+    <input type="hidden" name="hidden" value="true">
+    <button type="submit" class="link">{{ tr.t("settings-list-hide") }}</button>
+    {% endif %}
+  </form>
+</li>
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
index 45a1461..a54fcdc 100644
--- a/crates/web/templates/settings.html
+++ b/crates/web/templates/settings.html
@@ -2,7 +2,7 @@
 {% block title %}{{ tr.t("settings-title") }}{% endblock %}
 {% block content %}
 <h1>{{ tr.t("settings-title") }}</h1>
-{% if let Some(saved) = saved %}
+{% if let Some(saved) = notices.saved %}
 <p class="notice" role="status">{{ tr.t(saved) }}</p>
 {% endif %}
 <section>
@@ -16,7 +16,7 @@
 </section>
 <section>
   <h2>{{ tr.t("settings-password") }}</h2>
-  {% if let Some(error) = password_error %}
+  {% if let Some(error) = notices.password_error %}
   <p class="notice" role="alert">{{ tr.t(error) }}</p>
   {% endif %}
   <form method="post" action="/settings/password" class="stack">
@@ -34,6 +34,42 @@
     <button type="submit">{{ tr.t("settings-password-submit") }}</button>
   </form>
 </section>
+{% if let Some(lists) = lists %}
+<section id="lists">
+  <h2>{{ tr.t("log-rooms") }}</h2>
+  <p class="hint">{{ tr.t("settings-lists-hint") }}</p>
+  {% if let Some(error) = notices.list_error %}
+  <p class="notice" role="alert">{{ tr.t(error) }}</p>
+  {% endif %}
+  <ul class="items">
+    {% for item in lists.rooms %}
+    {% include "list_item.html" %}
+    {% endfor %}
+  </ul>
+  <form method="post" action="/settings/rooms" class="stack">
+    <label>{{ tr.t("settings-room-add") }}
+      <input name="name" maxlength="60" required>
+    </label>
+    <fieldset class="choices">
+      <label><input type="radio" name="area" value="inside" checked> {{ tr.t("log-inside") }}</label>
+      <label><input type="radio" name="area" value="common"> {{ tr.t("log-common") }}</label>
+    </fieldset>
+    <button type="submit" class="secondary">{{ tr.t("settings-list-add") }}</button>
+  </form>
+  <h2>{{ tr.t("log-symptoms") }}</h2>
+  <ul class="items">
+    {% for item in lists.symptoms %}
+    {% include "list_item.html" %}
+    {% endfor %}
+  </ul>
+  <form method="post" action="/settings/symptoms" class="stack">
+    <label>{{ tr.t("settings-symptom-add") }}
+      <input name="name" maxlength="60" required>
+    </label>
+    <button type="submit" class="secondary">{{ tr.t("settings-list-add") }}</button>
+  </form>
+</section>
+{% endif %}
 <section>
   <h2>{{ tr.t("settings-guides") }}</h2>
   <ul class="rows">
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 26bdc34..3b7964a 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -123,3 +123,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 114 | 2026-10-14 | Accounts | A new link for someone who **never chose a password is a setup link**, otherwise a password reset; accepting one sets the password and language and uses up the link in one transaction; **only an expired account can be reactivated**, and expiring an expired one changes nothing | Reactivate also changes an active guest's expiry; two buttons for setup and reset | Spec §4 reactivates expired accounts; a lost setup QR is fixed the same way as a forgotten password; a double tap on Expire cannot move the end later |
 | 115 | 2026-10-14 | Accounts | Guest expiry quick picks: **1 day, 1 week (default), 2 weeks** count from now; **weekend** lasts until the coming **Monday 00:00** local time (on a Monday, the next one); a **chosen day** lasts until the end of that local day | All picks end at midnight; weekend ends on Sunday at 20:00 | Spec §8.3 names the picks but not their ends; a guest invited in the evening for "1 day" keeps the whole next day, a chosen date includes that whole day |
 | 116 | 2026-10-14 | Settings | Changing one's password needs the **current password**, checked as a sign-in attempt under the same limits (#60); the change ends **every** session of the account (#58) and then **signs this device in again** with a new session id; the language buttons post to `/lang`, which returns to Settings | No current password (the session is enough); keep this device's session id | A phone left unlocked cannot take the account over or guess the password faster than at sign-in; the person who changed it stays signed in while every other device is signed out (spec §8.4) |
+| 117 | 2026-10-14 | Lists | Residents **add** rooms (with their area) and symptoms and **hide or show** any of them from Settings, through `smoke_domain::lists`; items are **never renamed or deleted**, and a name may repeat | Rename items; refuse duplicate names | A rename would change what every old report says (the evidence); hidden items stay readable on old reports and selectable when correcting them (#90); two "Balcony" entries are a visible, fixable nuisance, not data loss |
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including `lists::only_residents_edit_the_lists` on both backends,
`residents_add_rooms_and_symptoms_and_hide_them_from_the_forms`,
`a_name_must_have_one_to_sixty_characters`, `only_residents_see_and_edit_the_lists` and
`hiding_an_unknown_item_is_not_found`. With `TEST_POSTGRES_URL` set: **536** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/lib.rs \
  crates/domain/src/lists.rs \
  crates/storage-contract/src/lib.rs \
  crates/storage-contract/src/lists.rs \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/lists.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/settings.rs \
  crates/web/static/app.css \
  crates/web/templates/list_item.html \
  crates/web/templates/settings.html \
  crates/web/tests/settings.rs \
  docs/DECISIONS.md
git commit -m "feat(lists): residents add and hide rooms and symptoms from Settings

\`smoke_domain::lists\` checks the edit-lists permission inside the transaction; the Settings
page lists every room and symptom with a Hide / Show button and an add form, for residents
only.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: People — list, end access, reactivate

Spec §9 screen 6 and §4. Residents see every account with its role, its end of access ("until
…", "expired …", "no end date") and "waiting for setup" when no password was chosen yet. *End
access now* expires a guest or viewer at once (their sessions end, #58); an expired account
shows a Reactivate form with the expiry fields: the guest quick picks, or for a viewer "never"
or a day. Others get 403 and no menu link: the header learns, through `SignedIn`, whether to
show People (decision #118); every page template's `signed_in` becomes `Option<SignedIn>`. The
expiry fields (`routes::people::expiry`) are shared with the invite form of Task 7.

**Files:**
- Create: `crates/web/src/header.rs`, `crates/web/src/routes/people/expiry.rs`, `crates/web/src/routes/people/mod.rs`, `crates/web/templates/expiry_fields.html`, `crates/web/templates/people.html`, `crates/web/tests/people.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/error.rs`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/entries.rs`, `crates/web/src/routes/events.rs`, `crates/web/src/routes/guide.rs`, `crates/web/src/routes/log.rs`, `crates/web/src/routes/login.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/routes/reports.rs`, `crates/web/src/routes/settings.rs`, `crates/web/static/app.css`, `crates/web/templates/base.html`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `people::{expire, reactivate, PeopleError, ExpiryPick}` (Task 3),
  `routes::people_refusal` (Task 4), `UserRepo::users`, `time::date_time`, `Pairs`.
- Produces (web):
  - `header::SignedIn { username: String, people: bool }`, `SignedIn::of(&User, now) ->
    Option<SignedIn>`; every page struct's `signed_in: Option<SignedIn>`; `routes::guide::rating`
    becomes generic over `S` (it needs the clock).
  - `routes::people::{PersonRow, role_key(Role) -> &'static str, require_manager(&User, now) ->
    Result<(), AppError>, list, end_access, restore_access, keys()}`; routes `GET /people`,
    `POST /people/{id}/expire`, `POST /people/{id}/reactivate` (`expiry`, `expiry_date`).
  - `routes::people::expiry::{PickChoice, ExpiryFields { role, pick_name, date_name, picks, date
    }, ExpiryDraft { pick, date }, ExpiryDraft::from_form(&Pairs, prefix), fields(Role, prefix,
    &ExpiryDraft) -> Option<ExpiryFields>, ExpiryInputError { Missing, Date }, read(Role,
    &ExpiryDraft) -> Result<ExpiryPick, ExpiryInputError>, keys()}`; template
    `expiry_fields.html` (expects `expiry`).
  - Translation keys (25): `nav-people`, `people-title`, `role-resident`, `role-guest`,
    `role-viewer`, `people-status-until`, `people-status-expired`, `people-status-no-end`,
    `people-status-setup`, `people-expire`, `people-reactivate`, `people-saved-expired`,
    `people-saved-reactivated`, `people-error-expiry`, `people-error-date`, `people-error-past`,
    `people-error-not-expired`, `people-error-resident`, `expiry-legend`, `expiry-1-day`,
    `expiry-weekend`, `expiry-1-week`, `expiry-2-weeks`, `expiry-date`, `expiry-never`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/web/tests/people.rs b/crates/web/tests/people.rs
new file mode 100644
index 0000000..ae4e15f
--- /dev/null
+++ b/crates/web/tests/people.rs
@@ -0,0 +1,174 @@
+//! People (spec §9 screen 6): residents see everyone and end or restore their access.
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use smoke_domain::clock::Clock;
+use smoke_domain::repos::UserRepo;
+use smoke_domain::types::{Locale, Role};
+
+#[tokio::test]
+async fn only_residents_reach_people_and_see_it_in_the_menu() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    app.add_user("lawyer", Role::Viewer, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let home = text(app.send(get_with("/", &resident)).await).await;
+    assert!(home.contains(r#"<a href="/people">People</a>"#), "{home}");
+    for name in ["guest", "lawyer"] {
+        let cookie = app.sign_in(name).await;
+        let home = text(app.send(get_with("/", &cookie)).await).await;
+        assert!(!home.contains(r#"href="/people""#), "{name}: {home}");
+        let page = app.send(get_with("/people", &cookie)).await;
+        assert_eq!(page.status(), StatusCode::FORBIDDEN, "{name}");
+        for path in ["/people/1/expire", "/people/2/reactivate"] {
+            let response = app.send(post_form(path, "expiry=1w", Some(&cookie))).await;
+            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{name} {path}");
+        }
+    }
+}
+
+#[tokio::test]
+async fn people_lists_everyone_with_their_role_and_expiry() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T19:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let html = text(app.send(get_with("/people", &cookie)).await).await;
+    assert!(
+        html.contains("<strong>daniele</strong> · resident · no end date"),
+        "{html}"
+    );
+    assert!(
+        html.contains("<strong>guest</strong> · guest · until 22/01/2026 20:00"),
+        "{html}"
+    );
+    assert!(html.contains(r#"action="/people/2/expire""#), "{html}");
+    assert!(!html.contains(r#"action="/people/1/expire""#), "{html}");
+}
+
+#[tokio::test]
+async fn ending_a_guests_access_signs_them_out_and_leaves_them_read_only() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let guest_cookie = app.sign_in("guest").await;
+
+    let path = format!("/people/{}/expire", guest.id);
+    let response = app.send(post_form(&path, "", Some(&resident))).await;
+    assert_eq!(location(&response), Some("/people?saved=expired"));
+    let expired = app.store.user(guest.id).await.unwrap().unwrap();
+    assert!(expired.actor().is_expired(app.clock.now()));
+
+    let page = app.send(get_with("/", &guest_cookie)).await;
+    assert_eq!(location(&page), Some("/login"), "signed out");
+    let again = app.sign_in("guest").await;
+    let home = text(app.send(get_with("/", &again)).await).await;
+    assert!(
+        home.contains("You can look at the data but not log."),
+        "{home}"
+    );
+
+    let html = text(app.send(get_with("/people?saved=expired", &resident)).await).await;
+    assert!(html.contains("Access ended."), "{html}");
+    assert!(html.contains(r#"action="/people/2/reactivate""#), "{html}");
+}
+
+#[tokio::test]
+async fn an_expired_viewer_can_be_reactivated_without_an_end() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let viewer = app.add_user("lawyer", Role::Viewer, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let path = format!("/people/{}/expire", viewer.id);
+    app.send(post_form(&path, "", Some(&resident))).await;
+    assert_eq!(
+        app.attempt("lawyer", PASSWORD).await.status(),
+        StatusCode::FORBIDDEN
+    );
+
+    let path = format!("/people/{}/reactivate", viewer.id);
+    let response = app
+        .send(post_form(&path, "expiry=never", Some(&resident)))
+        .await;
+    assert_eq!(location(&response), Some("/people?saved=reactivated"));
+    let back = app.store.user(viewer.id).await.unwrap().unwrap();
+    assert_eq!(back.expires_at, None);
+    assert_eq!(
+        app.attempt("lawyer", PASSWORD).await.status(),
+        StatusCode::SEE_OTHER,
+        "the old password still works"
+    );
+}
+
+#[tokio::test]
+async fn a_guest_is_reactivated_only_with_a_future_end() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T19:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let reactivate = format!("/people/{}/reactivate", guest.id);
+
+    let response = app
+        .send(post_form(&reactivate, "expiry=1w", Some(&resident)))
+        .await;
+    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+    assert!(
+        text(response)
+            .await
+            .contains("This account has not expired.")
+    );
+
+    let expire = format!("/people/{}/expire", guest.id);
+    app.send(post_form(&expire, "", Some(&resident))).await;
+    for (body, message) in [
+        ("expiry=never", "Pick how long a guest may stay."),
+        (
+            "expiry=date&expiry_date=2026-01-14",
+            "The end must be in the future.",
+        ),
+        ("expiry=date&expiry_date=soon", "Enter a valid date."),
+    ] {
+        let response = app
+            .send(post_form(&reactivate, body, Some(&resident)))
+            .await;
+        assert_eq!(
+            response.status(),
+            StatusCode::UNPROCESSABLE_ENTITY,
+            "{body}"
+        );
+        let html = text(response).await;
+        assert!(html.contains(message), "{body}: {html}");
+    }
+    let response = app
+        .send(post_form(
+            &reactivate,
+            "expiry=date&expiry_date=2026-01-31",
+            Some(&resident),
+        ))
+        .await;
+    assert_eq!(location(&response), Some("/people?saved=reactivated"));
+    let back = app.store.user(guest.id).await.unwrap().unwrap();
+    assert_eq!(back.expires_at, Some(t("2026-01-31T23:00:00Z")));
+}
+
+#[tokio::test]
+async fn a_resident_cannot_be_expired() {
+    let app = TestApp::new().await;
+    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let path = format!("/people/{}/expire", partner.id);
+    let response = app.send(post_form(&path, "", Some(&cookie))).await;
+    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+    assert!(text(response).await.contains("Residents never expire."));
+    let missing = app
+        .send(post_form("/people/99/expire", "", Some(&cookie)))
+        .await;
+    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index 20b6aa9..7ac2858 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -218,6 +218,7 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         text(app.send(get_with("/", &cookie)).await).await,
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
         text(app.send(get_with("/settings", &cookie)).await).await,
+        text(app.send(get_with("/people", &cookie)).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: the new tests in `crates/web/tests/people.rs` fail (`/people` answers 404, there is no People
link). `no_page_needs_inline_scripts_styles_or_eval` still passes (its `/people` entry is the 404
page until Step 3).

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 50d595a..7276494 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Like a smoke-filled room. You must leave.
 
 nav-log = Log
 nav-events = Events
+nav-people = People
 nav-settings = Settings
 
 events-title = Events
@@ -193,6 +194,31 @@ settings-list-add = Add
 settings-list-name-invalid = A name needs 1 to 60 characters.
 settings-saved-lists = Lists saved.
 
+people-title = People
+role-resident = resident
+role-guest = guest
+role-viewer = viewer
+people-status-until = until { $when }
+people-status-expired = expired { $when }
+people-status-no-end = no end date
+people-status-setup = waiting for setup
+people-expire = End access now
+people-reactivate = Reactivate
+people-saved-expired = Access ended. They are signed out everywhere.
+people-saved-reactivated = Account reactivated. The old password still works.
+people-error-expiry = Pick how long a guest may stay.
+people-error-date = Enter a valid date.
+people-error-past = The end must be in the future.
+people-error-not-expired = This account has not expired.
+people-error-resident = Residents never expire.
+expiry-legend = Access ends
+expiry-1-day = in 1 day
+expiry-weekend = after the weekend
+expiry-1-week = in 1 week
+expiry-2-weeks = in 2 weeks
+expiry-date = at the end of this day:
+expiry-never = never
+
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 7b1e375..2dd35d0 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
 nav-log = Registra
 nav-events = Eventi
+nav-people = Persone
 nav-settings = Impostazioni
 
 events-title = Eventi
@@ -193,6 +194,31 @@ settings-list-add = Aggiungi
 settings-list-name-invalid = Un nome deve avere da 1 a 60 caratteri.
 settings-saved-lists = Elenchi salvati.
 
+people-title = Persone
+role-resident = residente
+role-guest = ospite
+role-viewer = osservatore
+people-status-until = fino al { $when }
+people-status-expired = scaduto il { $when }
+people-status-no-end = senza scadenza
+people-status-setup = in attesa di configurazione
+people-expire = Termina l'accesso ora
+people-reactivate = Riattiva
+people-saved-expired = Accesso terminato. La persona è stata disconnessa ovunque.
+people-saved-reactivated = Account riattivato. La vecchia password funziona ancora.
+people-error-expiry = Scegli quanto può restare l'ospite.
+people-error-date = Inserisci una data valida.
+people-error-past = La scadenza deve essere nel futuro.
+people-error-not-expired = Questo account non è scaduto.
+people-error-resident = I residenti non scadono mai.
+expiry-legend = L'accesso termina
+expiry-1-day = tra 1 giorno
+expiry-weekend = dopo il fine settimana
+expiry-1-week = tra 1 settimana
+expiry-2-weeks = tra 2 settimane
+expiry-date = alla fine di questo giorno:
+expiry-never = mai
+
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index b6831c1..ba98783 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
 nav-log = Kaydet
 nav-events = Olaylar
+nav-people = Kişiler
 nav-settings = Ayarlar
 
 events-title = Olaylar
@@ -193,6 +194,31 @@ settings-list-add = Ekle
 settings-list-name-invalid = Bir ad 1 ile 60 karakter arasında olmalı.
 settings-saved-lists = Listeler kaydedildi.
 
+people-title = Kişiler
+role-resident = ev sakini
+role-guest = misafir
+role-viewer = izleyici
+people-status-until = { $when } tarihine kadar
+people-status-expired = { $when } tarihinde süresi doldu
+people-status-no-end = bitiş tarihi yok
+people-status-setup = kurulum bekleniyor
+people-expire = Erişimi şimdi sonlandır
+people-reactivate = Yeniden etkinleştir
+people-saved-expired = Erişim sonlandırıldı. Tüm cihazlarda oturumu kapatıldı.
+people-saved-reactivated = Hesap yeniden etkinleştirildi. Eski şifre hâlâ geçerli.
+people-error-expiry = Misafirin ne kadar kalabileceğini seçin.
+people-error-date = Geçerli bir tarih girin.
+people-error-past = Bitiş gelecekte olmalı.
+people-error-not-expired = Bu hesabın süresi dolmadı.
+people-error-resident = Ev sakinlerinin süresi hiç dolmaz.
+expiry-legend = Erişimin bitişi
+expiry-1-day = 1 gün sonra
+expiry-weekend = hafta sonundan sonra
+expiry-1-week = 1 hafta sonra
+expiry-2-weeks = 2 hafta sonra
+expiry-date = bu günün sonunda:
+expiry-never = hiçbir zaman
+
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
diff --git a/crates/web/src/error.rs b/crates/web/src/error.rs
index e316fa9..7242acd 100644
--- a/crates/web/src/error.rs
+++ b/crates/web/src/error.rs
@@ -13,6 +13,7 @@ use axum::response::{Html, IntoResponse, Response};
 use smoke_domain::repos::StoreError;
 
 use crate::cookies;
+use crate::header::SignedIn;
 use crate::i18n::{LANG_COOKIE, Tr, negotiate};
 use crate::request_id;
 
@@ -124,7 +125,7 @@ pub(crate) fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
 #[template(path = "error.html")]
 struct ErrorTemplate {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     title_key: &'static str,
     message_key: &'static str,
     request_id: String,
diff --git a/crates/web/src/header.rs b/crates/web/src/header.rs
new file mode 100644
index 0000000..4a836a4
--- /dev/null
+++ b/crates/web/src/header.rs
@@ -0,0 +1,20 @@
+//! What the page header shows for the signed-in person (spec §9 screen 8).
+
+use chrono::{DateTime, Utc};
+use smoke_domain::permissions::{Action, can};
+use smoke_domain::records::User;
+
+pub(crate) struct SignedIn {
+    pub(crate) username: String,
+    /// The menu links to People (residents).
+    pub(crate) people: bool,
+}
+
+impl SignedIn {
+    pub(crate) fn of(user: &User, now: DateTime<Utc>) -> Option<Self> {
+        Some(Self {
+            username: user.username.to_string(),
+            people: can(&user.actor(), Action::ManageUsers, now),
+        })
+    }
+}
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index 760593b..abaa2fd 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,7 +127,7 @@ mod tests {
 
     use super::*;
     use crate::routes::login::Notice;
-    use crate::routes::{self, entries, guide, lists, reports, settings};
+    use crate::routes::{self, entries, guide, lists, people, reports, settings};
     use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -220,6 +220,7 @@ mod tests {
         used.extend(reports::HISTORY_KEYS.map(str::to_owned));
         used.extend(settings::keys().into_iter().map(str::to_owned));
         used.extend(lists::KEYS.map(str::to_owned));
+        used.extend(people::keys().into_iter().map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 4515b89..6cf6a93 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -8,6 +8,7 @@ pub mod config;
 mod cookies;
 mod error;
 mod forms;
+mod header;
 mod i18n;
 mod labels;
 mod request_id;
@@ -43,7 +44,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
     auth::password::prepare_dummy_hash();
     let signed_in = Router::new()
         .route("/", get(routes::log::show::<S>))
-        .route("/guide/rating", get(routes::guide::rating))
+        .route("/guide/rating", get(routes::guide::rating::<S>))
         .route("/log", post(routes::entries::submit::<S>))
         .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
         .route("/events", get(routes::events::list::<S>))
@@ -56,6 +57,12 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::reports::correct_form::<S>).post(routes::reports::correct::<S>),
         )
         .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
+        .route("/people", get(routes::people::list::<S>))
+        .route("/people/{id}/expire", post(routes::people::end_access::<S>))
+        .route(
+            "/people/{id}/reactivate",
+            post(routes::people::restore_access::<S>),
+        )
         .route("/settings", get(routes::settings::show::<S>))
         .route("/settings/password", post(routes::settings::change::<S>))
         .route("/settings/rooms", post(routes::lists::create_room::<S>))
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index 14a55d2..49e60e9 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -22,6 +22,7 @@ use smoke_domain::types::{EventId, Level, Pm25, Role, RoomId, SymptomId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::routes::log::{Draft, log_page, offered};
 use crate::routes::{notices, refusal};
@@ -321,7 +322,7 @@ struct Answer {
 #[template(path = "prompt.html")]
 struct PromptPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     title: String,
     detail: Option<String>,
     /// The form as posted, re-sent with the answer.
@@ -434,7 +435,7 @@ async fn ask<S: Store>(
         StatusCode::OK,
         &PromptPage {
             tr,
-            signed_in: Some(user.username.to_string()),
+            signed_in: SignedIn::of(user, now),
             title,
             detail,
             hidden: form.without(&["choice", "asked"]),
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index 14f451d..bea716a 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -21,6 +21,7 @@ use smoke_domain::types::{EventId, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::labels;
 use crate::routes::entries::read_time;
@@ -63,7 +64,7 @@ struct Day {
 #[template(path = "events.html")]
 struct ListPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     month: String,
     previous: String,
     next: String,
@@ -184,7 +185,7 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
         StatusCode::OK,
         &ListPage {
             tr,
-            signed_in: Some(user.username.to_string()),
+            signed_in: SignedIn::of(&user, now),
             month: first_of_month.format("%m/%Y").to_string(),
             previous: month_param(first_of_month - Months::new(1)),
             next: month_param(first_of_month + Months::new(1)),
@@ -209,7 +210,7 @@ struct Entry {
 #[template(path = "event.html")]
 struct DetailPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     id: i64,
     date: String,
     span: String,
@@ -343,7 +344,7 @@ async fn detail<S: Store>(
     };
     Ok(DetailPage {
         tr,
-        signed_in: Some(user.username.to_string()),
+        signed_in: SignedIn::of(user, now),
         id: id.0,
         date: time::date(tr.0, state.config.schedule.local_date(event.opened_at)),
         span: span(tr, &event, tz),
diff --git a/crates/web/src/routes/guide.rs b/crates/web/src/routes/guide.rs
index 0d5ec7a..4173eb3 100644
--- a/crates/web/src/routes/guide.rs
+++ b/crates/web/src/routes/guide.rs
@@ -2,13 +2,17 @@
 //! page. The texts live in the translation files so they are easy to edit.
 
 use askama::Template;
+use axum::extract::State;
 use axum::http::StatusCode;
 use axum::response::Response;
+use smoke_domain::repos::Store;
 use smoke_domain::types::Level;
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
+use crate::header::SignedIn;
 use crate::i18n::Tr;
+use crate::state::AppState;
 
 /// One row of the guide.
 pub(crate) struct GuideRow {
@@ -39,16 +43,20 @@ pub(crate) fn keys() -> Vec<String> {
 #[template(path = "guide.html")]
 struct GuidePage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     levels: Vec<GuideRow>,
 }
 
-pub(crate) async fn rating(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
+pub(crate) async fn rating<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+) -> Result<Response, AppError> {
     page(
         StatusCode::OK,
         &GuidePage {
             tr,
-            signed_in: Some(user.username.to_string()),
+            signed_in: SignedIn::of(&user, state.clock.now()),
             levels: levels(tr),
         },
     )
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index ba1f73f..f1b712b 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -17,6 +17,7 @@ use smoke_domain::types::{Level, RoomArea, RoomId, SymptomId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::routes::entries::level_field;
 use crate::routes::guide::{GuideRow, levels};
@@ -182,7 +183,7 @@ pub(crate) struct FormView {
 #[template(path = "log.html")]
 pub(crate) struct LogPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     status: Status,
     /// The open event, when the user may close it.
     close_event: Option<i64>,
@@ -292,7 +293,7 @@ pub(crate) async fn log_page<S: Store>(
 
     Ok(LogPage {
         tr,
-        signed_in: Some(user.username.to_string()),
+        signed_in: SignedIn::of(user, now),
         close_event: shown
             .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
             .map(|e| e.id.0),
diff --git a/crates/web/src/routes/login.rs b/crates/web/src/routes/login.rs
index 78decd7..839c111 100644
--- a/crates/web/src/routes/login.rs
+++ b/crates/web/src/routes/login.rs
@@ -16,6 +16,7 @@ use crate::auth::limiter::LoginLimiter;
 use crate::auth::{self, CurrentUser, password};
 use crate::cookies;
 use crate::error::{AppError, page};
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::state::AppState;
 
@@ -47,7 +48,7 @@ impl Notice {
 #[template(path = "login.html")]
 struct LoginPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     username: String,
     notice: Option<Notice>,
 }
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index 7ec2892..3252c1d 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -8,6 +8,7 @@ pub(crate) mod lists;
 pub(crate) mod log;
 pub(crate) mod login;
 pub(crate) mod notices;
+pub(crate) mod people;
 pub(crate) mod reports;
 pub(crate) mod settings;
 
diff --git a/crates/web/src/routes/people/expiry.rs b/crates/web/src/routes/people/expiry.rs
new file mode 100644
index 0000000..d193433
--- /dev/null
+++ b/crates/web/src/routes/people/expiry.rs
@@ -0,0 +1,199 @@
+//! The expiry fields of the invite and reactivation forms (spec §8.3): quick picks for guests,
+//! an optional date for viewers, nothing for residents.
+
+use chrono::NaiveDate;
+use smoke_domain::people::ExpiryPick;
+use smoke_domain::types::Role;
+
+use crate::forms::Pairs;
+
+/// The guest quick picks: (form value, translation key).
+const GUEST_PICKS: [(&str, &str); 5] = [
+    ("1d", "expiry-1-day"),
+    ("weekend", "expiry-weekend"),
+    ("1w", "expiry-1-week"),
+    ("2w", "expiry-2-weeks"),
+    ("date", "expiry-date"),
+];
+
+/// A viewer may have no end, or end on a chosen day.
+const VIEWER_PICKS: [(&str, &str); 2] = [("never", "expiry-never"), ("date", "expiry-date")];
+
+/// One radio button of the expiry fields.
+pub(crate) struct PickChoice {
+    pub(crate) value: &'static str,
+    pub(crate) key: &'static str,
+    pub(crate) checked: bool,
+}
+
+/// The expiry fields for one role, named `<prefix>expiry` and `<prefix>expiry_date`
+/// (`templates/expiry_fields.html`).
+pub(crate) struct ExpiryFields {
+    pub(crate) role: &'static str,
+    pub(crate) pick_name: String,
+    pub(crate) date_name: String,
+    pub(crate) picks: Vec<PickChoice>,
+    pub(crate) date: String,
+}
+
+/// The fields as typed, to show them again.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub(crate) struct ExpiryDraft {
+    pub(crate) pick: Option<String>,
+    pub(crate) date: String,
+}
+
+impl ExpiryDraft {
+    pub(crate) fn from_form(form: &Pairs, prefix: &str) -> Self {
+        Self {
+            pick: form.get(&format!("{prefix}expiry")).map(str::to_owned),
+            date: form
+                .get(&format!("{prefix}expiry_date"))
+                .unwrap_or_default()
+                .to_owned(),
+        }
+    }
+}
+
+/// The fields for `role`, with `draft` picked (the role's default when nothing is); `None` for
+/// residents, who never expire.
+pub(crate) fn fields(role: Role, prefix: &str, draft: &ExpiryDraft) -> Option<ExpiryFields> {
+    let (picks, default): (&[(&'static str, &'static str)], &str) = match role {
+        Role::Resident => return None,
+        Role::Guest => (&GUEST_PICKS, "1w"),
+        Role::Viewer => (&VIEWER_PICKS, "never"),
+    };
+    let picked = draft.pick.as_deref().unwrap_or(default);
+    Some(ExpiryFields {
+        role: role.as_str(),
+        pick_name: format!("{prefix}expiry"),
+        date_name: format!("{prefix}expiry_date"),
+        picks: picks
+            .iter()
+            .map(|&(value, key)| PickChoice {
+                value,
+                key,
+                checked: value == picked,
+            })
+            .collect(),
+        date: draft.date.clone(),
+    })
+}
+
+/// Why the expiry fields cannot be read.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum ExpiryInputError {
+    /// A guest needs one of the picks.
+    Missing,
+    /// "Until a day" without a valid date.
+    Date,
+}
+
+impl ExpiryInputError {
+    pub(crate) fn key(self) -> &'static str {
+        match self {
+            Self::Missing => "people-error-expiry",
+            Self::Date => "people-error-date",
+        }
+    }
+}
+
+/// Reads the pick for `role`. Residents never expire, whatever the form says.
+pub(crate) fn read(role: Role, draft: &ExpiryDraft) -> Result<ExpiryPick, ExpiryInputError> {
+    let date = || {
+        NaiveDate::parse_from_str(draft.date.trim(), "%Y-%m-%d")
+            .map(ExpiryPick::Until)
+            .map_err(|_| ExpiryInputError::Date)
+    };
+    match (role, draft.pick.as_deref()) {
+        (Role::Resident, _) => Ok(ExpiryPick::Never),
+        (Role::Guest, Some("1d")) => Ok(ExpiryPick::OneDay),
+        (Role::Guest, Some("weekend")) => Ok(ExpiryPick::Weekend),
+        (Role::Guest, Some("1w")) => Ok(ExpiryPick::OneWeek),
+        (Role::Guest, Some("2w")) => Ok(ExpiryPick::TwoWeeks),
+        (Role::Guest | Role::Viewer, Some("date")) => date(),
+        (Role::Viewer, None | Some("never")) => Ok(ExpiryPick::Never),
+        _ => Err(ExpiryInputError::Missing),
+    }
+}
+
+/// Every key this module returns, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    GUEST_PICKS
+        .iter()
+        .chain(&VIEWER_PICKS)
+        .map(|&(_, key)| key)
+        .chain([
+            ExpiryInputError::Missing.key(),
+            ExpiryInputError::Date.key(),
+        ])
+        .collect()
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn draft(pick: Option<&str>, date: &str) -> ExpiryDraft {
+        ExpiryDraft {
+            pick: pick.map(str::to_owned),
+            date: date.to_owned(),
+        }
+    }
+
+    #[test]
+    fn guests_need_a_pick_and_viewers_default_to_no_end() {
+        assert_eq!(
+            read(Role::Guest, &draft(Some("weekend"), "")),
+            Ok(ExpiryPick::Weekend)
+        );
+        assert_eq!(
+            read(Role::Guest, &draft(None, "")),
+            Err(ExpiryInputError::Missing)
+        );
+        assert_eq!(
+            read(Role::Guest, &draft(Some("never"), "")),
+            Err(ExpiryInputError::Missing)
+        );
+        assert_eq!(read(Role::Viewer, &draft(None, "")), Ok(ExpiryPick::Never));
+        assert_eq!(
+            read(Role::Viewer, &draft(Some("1w"), "")),
+            Err(ExpiryInputError::Missing),
+            "the quick picks are for guests"
+        );
+        assert_eq!(
+            read(Role::Resident, &draft(Some("1d"), "")),
+            Ok(ExpiryPick::Never)
+        );
+    }
+
+    #[test]
+    fn a_chosen_day_needs_a_valid_date() {
+        let day = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
+        assert_eq!(
+            read(Role::Viewer, &draft(Some("date"), " 2026-02-01 ")),
+            Ok(ExpiryPick::Until(day))
+        );
+        for bad in ["", "2026-02-30", "01/02/2026"] {
+            assert_eq!(
+                read(Role::Guest, &draft(Some("date"), bad)),
+                Err(ExpiryInputError::Date),
+                "{bad}"
+            );
+        }
+    }
+
+    #[test]
+    fn the_fields_preselect_the_default_or_the_draft() {
+        let guest = fields(Role::Guest, "guest_", &ExpiryDraft::default()).unwrap();
+        assert_eq!(guest.pick_name, "guest_expiry");
+        let checked: Vec<_> = guest.picks.iter().filter(|p| p.checked).collect();
+        assert_eq!(checked.len(), 1);
+        assert_eq!(checked[0].value, "1w");
+        let viewer = fields(Role::Viewer, "", &draft(Some("date"), "2026-02-01")).unwrap();
+        assert_eq!(viewer.date, "2026-02-01");
+        assert!(viewer.picks.iter().any(|p| p.value == "date" && p.checked));
+        assert!(fields(Role::Resident, "", &ExpiryDraft::default()).is_none());
+    }
+}
diff --git a/crates/web/src/routes/people/mod.rs b/crates/web/src/routes/people/mod.rs
new file mode 100644
index 0000000..41e9c61
--- /dev/null
+++ b/crates/web/src/routes/people/mod.rs
@@ -0,0 +1,244 @@
+//! People (spec §9 screen 6, residents only): everyone with their role and expiry, and the
+//! buttons that end or restore their access.
+
+pub(crate) mod expiry;
+
+use askama::Template;
+use axum::extract::{Path, Query, State};
+use axum::http::StatusCode;
+use axum::response::{IntoResponse, Redirect, Response};
+use chrono::{DateTime, Utc};
+use serde::Deserialize;
+use smoke_domain::DomainError;
+use smoke_domain::people::{PeopleError, expire, reactivate};
+use smoke_domain::permissions::{Action, can};
+use smoke_domain::records::User;
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{Role, UserId};
+
+use crate::auth::CurrentUser;
+use crate::error::{AppError, page};
+use crate::forms::Pairs;
+use crate::header::SignedIn;
+use crate::i18n::Tr;
+use crate::routes::people::expiry::{ExpiryDraft, ExpiryFields};
+use crate::routes::people_refusal;
+use crate::state::AppState;
+use crate::time;
+
+/// One person on the page.
+pub(crate) struct PersonRow {
+    pub(crate) id: i64,
+    pub(crate) username: String,
+    pub(crate) role: &'static str,
+    /// "until 22/01/2026 19:05", "expired 15/01/2026 20:00" or "no end date".
+    pub(crate) status: String,
+    /// Invited but has not chosen a password yet.
+    pub(crate) pending_setup: bool,
+    pub(crate) can_expire: bool,
+    /// The reactivation form, for expired accounts.
+    pub(crate) reactivate: Option<ExpiryFields>,
+}
+
+#[derive(Template)]
+#[template(path = "people.html")]
+struct PeoplePage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    saved: Option<&'static str>,
+    error: Option<&'static str>,
+    people: Vec<PersonRow>,
+}
+
+pub(crate) fn role_key(role: Role) -> &'static str {
+    match role {
+        Role::Resident => "role-resident",
+        Role::Guest => "role-guest",
+        Role::Viewer => "role-viewer",
+    }
+}
+
+fn status(tr: Tr, user: &User, now: DateTime<Utc>, tz: chrono_tz::Tz) -> String {
+    match user.expires_at {
+        None => tr.t("people-status-no-end"),
+        Some(at) => {
+            let when = time::date_time(tr.0, at, tz);
+            let key = if at <= now {
+                "people-status-expired"
+            } else {
+                "people-status-until"
+            };
+            tr.t1(key, "when", &when)
+        }
+    }
+}
+
+/// Answers 403 unless `user` manages people.
+pub(crate) fn require_manager(user: &User, now: DateTime<Utc>) -> Result<(), AppError> {
+    if can(&user.actor(), Action::ManageUsers, now) {
+        Ok(())
+    } else {
+        Err(AppError::NotAllowed)
+    }
+}
+
+/// What the page says after a refused change, or the error page it becomes.
+fn refusal_key(error: PeopleError) -> Result<&'static str, AppError> {
+    match error {
+        PeopleError::NotExpired => Ok("people-error-not-expired"),
+        PeopleError::Invalid(DomainError::ExpiryNotInFuture) => Ok("people-error-past"),
+        PeopleError::Invalid(DomainError::GuestsMustExpire) => Ok("people-error-expiry"),
+        PeopleError::Invalid(DomainError::ResidentsNeverExpire) => Ok("people-error-resident"),
+        other => Err(people_refusal(other)),
+    }
+}
+
+/// The page, with `draft` shown again in the reactivation form of the person it names.
+async fn people_page<S: Store>(
+    state: &AppState<S>,
+    user: &User,
+    tr: Tr,
+    status_code: StatusCode,
+    notices: (Option<&'static str>, Option<&'static str>),
+    draft: Option<(UserId, ExpiryDraft)>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let tz = state.config.schedule.tz();
+    let people = state
+        .store
+        .users()
+        .await?
+        .iter()
+        .map(|person| {
+            let expired = person.actor().is_expired(now);
+            let typed = draft
+                .as_ref()
+                .filter(|(id, _)| *id == person.id)
+                .map(|(_, typed)| typed.clone())
+                .unwrap_or_default();
+            PersonRow {
+                id: person.id.0,
+                username: person.username.to_string(),
+                role: role_key(person.role),
+                status: status(tr, person, now, tz),
+                pending_setup: person.password_hash.is_none(),
+                can_expire: person.role != Role::Resident && !expired,
+                reactivate: expired
+                    .then(|| expiry::fields(person.role, "", &typed))
+                    .flatten(),
+            }
+        })
+        .collect();
+    let (saved, error) = notices;
+    page(
+        status_code,
+        &PeoplePage {
+            tr,
+            signed_in: SignedIn::of(user, now),
+            saved,
+            error,
+            people,
+        },
+    )
+}
+
+#[derive(Deserialize)]
+pub(crate) struct ListQuery {
+    saved: Option<String>,
+}
+
+pub(crate) async fn list<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Query(query): Query<ListQuery>,
+) -> Result<Response, AppError> {
+    require_manager(&user, state.clock.now())?;
+    let saved = match query.saved.as_deref() {
+        Some("expired") => Some("people-saved-expired"),
+        Some("reactivated") => Some("people-saved-reactivated"),
+        _ => None,
+    };
+    people_page(&state, &user, tr, StatusCode::OK, (saved, None), None).await
+}
+
+/// Ends a guest's or viewer's access now; they are signed out everywhere.
+pub(crate) async fn end_access<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Path(id): Path<i64>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    match expire(&mut tx, &user.actor(), UserId(id), now).await {
+        Ok(_) => {
+            tx.commit().await?;
+            Ok(Redirect::to("/people?saved=expired").into_response())
+        }
+        Err(error) => {
+            drop(tx);
+            let key = refusal_key(error)?;
+            let code = StatusCode::UNPROCESSABLE_ENTITY;
+            people_page(&state, &user, tr, code, (None, Some(key)), None).await
+        }
+    }
+}
+
+/// Gives an expired account a new expiry, read from the form's expiry fields.
+pub(crate) async fn restore_access<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Path(id): Path<i64>,
+    form: Pairs,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    require_manager(&user, now)?;
+    let person = state
+        .store
+        .user(UserId(id))
+        .await?
+        .ok_or(AppError::NotFound)?;
+    let draft = ExpiryDraft::from_form(&form, "");
+    let refuse = async |key: &'static str| {
+        let code = StatusCode::UNPROCESSABLE_ENTITY;
+        let draft = Some((person.id, draft.clone()));
+        people_page(&state, &user, tr, code, (None, Some(key)), draft).await
+    };
+    let pick = match expiry::read(person.role, &draft) {
+        Ok(pick) => pick,
+        Err(error) => return refuse(error.key()).await,
+    };
+    let expires_at = pick.resolve(now, &state.config.schedule);
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    match reactivate(&mut tx, &user.actor(), person.id, expires_at, now).await {
+        Ok(_) => {
+            tx.commit().await?;
+            Ok(Redirect::to("/people?saved=reactivated").into_response())
+        }
+        Err(error) => {
+            drop(tx);
+            refuse(refusal_key(error)?).await
+        }
+    }
+}
+
+/// Every key this module passes through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    let mut keys = vec![
+        "people-status-no-end",
+        "people-status-expired",
+        "people-status-until",
+        "people-error-not-expired",
+        "people-error-past",
+        "people-error-expiry",
+        "people-error-resident",
+        "people-saved-expired",
+        "people-saved-reactivated",
+    ];
+    keys.extend(Role::ALL.iter().map(|&role| role_key(role)));
+    keys.extend(expiry::keys());
+    keys
+}
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index 565d56b..0853837 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -17,6 +17,7 @@ use smoke_domain::types::{EventId, ReportId, SymptomId, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::routes::entries::{Smell, joinable_events, read_time, read_values};
 use crate::routes::guide::{GuideRow, levels};
@@ -70,7 +71,7 @@ struct MoveOption {
 #[template(path = "report.html")]
 struct ReportPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     id: i64,
     /// `None` for a no-smell check outside any event.
     event: Option<EventLink>,
@@ -251,7 +252,7 @@ async fn report_page<S: Store>(
     let intensity = report.values.intensity().get();
     Ok(ReportPage {
         tr,
-        signed_in: Some(user.username.to_string()),
+        signed_in: SignedIn::of(user, now),
         id: id.0,
         event: event.map(|event| EventLink {
             id: event.id.0,
@@ -287,7 +288,7 @@ pub(crate) struct CorrectForm {
 #[template(path = "correct.html")]
 struct CorrectPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     id: i64,
     levels: Vec<GuideRow>,
     /// A no-smell check: its levels stay at 0.
@@ -332,7 +333,7 @@ async fn correct_page<S: Store>(
     let keep = (room_ids.as_slice(), report.values.symptoms());
     Ok(CorrectPage {
         tr,
-        signed_in: Some(user.username.to_string()),
+        signed_in: SignedIn::of(user, state.clock.now()),
         id: report.id.0,
         levels: levels(tr),
         check: report.event_id.is_none(),
diff --git a/crates/web/src/routes/settings.rs b/crates/web/src/routes/settings.rs
index c02a600..1a66c82 100644
--- a/crates/web/src/routes/settings.rs
+++ b/crates/web/src/routes/settings.rs
@@ -18,6 +18,7 @@ use crate::auth::client_ip::ClientIp;
 use crate::auth::limiter::LoginLimiter;
 use crate::auth::{self, CurrentUser, password};
 use crate::error::{AppError, page};
+use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::routes::lists::{self, ListsView};
 use crate::routes::people_refusal;
@@ -48,7 +49,7 @@ pub(crate) struct Notices {
 #[template(path = "settings.html")]
 struct SettingsPage {
     tr: Tr,
-    signed_in: Option<String>,
+    signed_in: Option<SignedIn>,
     /// For password managers, which file the new password under this name.
     username: String,
     languages: Vec<LanguageChoice>,
@@ -64,7 +65,8 @@ pub(crate) async fn settings_page<S: Store>(
     status: StatusCode,
     notices: Notices,
 ) -> Result<Response, AppError> {
-    let lists = if can(&user.actor(), Action::EditLists, state.clock.now()) {
+    let now = state.clock.now();
+    let lists = if can(&user.actor(), Action::EditLists, now) {
         Some(lists::view(state, tr).await?)
     } else {
         None
@@ -73,7 +75,7 @@ pub(crate) async fn settings_page<S: Store>(
         status,
         &SettingsPage {
             tr,
-            signed_in: Some(user.username.to_string()),
+            signed_in: SignedIn::of(user, now),
             username: user.username.to_string(),
             languages: LANGUAGES
                 .iter()
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 73da96b..96e9a19 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -304,3 +304,19 @@ button.current {
   justify-content: space-between;
   gap: var(--gap);
 }
+
+.people {
+  list-style: none;
+  padding: 0;
+}
+
+.people li {
+  padding: 0.5rem 0;
+  border-bottom: 1px solid rgb(128 128 128 / 30%);
+}
+
+.expiry input[type="date"] {
+  font: inherit;
+  padding: 0.5rem;
+  border-radius: 0.5rem;
+}
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index ad6cf09..733b51c 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -13,14 +13,15 @@
 <body>
   <header class="bar">
     <span class="brand">{{ tr.t("app-name") }}</span>
-    {% if let Some(username) = signed_in %}
+    {% if let Some(me) = signed_in %}
     <nav class="menu">
       <a href="/">{{ tr.t("nav-log") }}</a>
       <a href="/events">{{ tr.t("nav-events") }}</a>
+      {% if me.people %}<a href="/people">{{ tr.t("nav-people") }}</a>{% endif %}
       <a href="/settings">{{ tr.t("nav-settings") }}</a>
     </nav>
     <form method="post" action="/logout" class="who">
-      <span>{{ tr.t1("signed-in-as", "username", username) }}</span>
+      <span>{{ tr.t1("signed-in-as", "username", me.username) }}</span>
       <button type="submit" class="link">{{ tr.t("logout") }}</button>
     </form>
     {% endif %}
diff --git a/crates/web/templates/expiry_fields.html b/crates/web/templates/expiry_fields.html
new file mode 100644
index 0000000..7f83334
--- /dev/null
+++ b/crates/web/templates/expiry_fields.html
@@ -0,0 +1,8 @@
+{# When an account stops working: quick picks or a day (spec §8.3). Expects `expiry`. #}
+<fieldset class="expiry choices" data-role="{{ expiry.role }}">
+  <legend>{{ tr.t("expiry-legend") }}</legend>
+  {% for pick in expiry.picks %}
+  <label class="chip"><input type="radio" name="{{ expiry.pick_name }}" value="{{ pick.value }}"{% if pick.checked %} checked{% endif %}><span>{{ tr.t(pick.key) }}</span></label>
+  {% endfor %}
+  <input type="date" name="{{ expiry.date_name }}" value="{{ expiry.date }}" aria-label="{{ tr.t("expiry-date") }}">
+</fieldset>
diff --git a/crates/web/templates/people.html b/crates/web/templates/people.html
new file mode 100644
index 0000000..311b4e9
--- /dev/null
+++ b/crates/web/templates/people.html
@@ -0,0 +1,29 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("people-title") }}{% endblock %}
+{% block content %}
+<h1>{{ tr.t("people-title") }}</h1>
+{% if let Some(saved) = saved %}
+<p class="notice" role="status">{{ tr.t(saved) }}</p>
+{% endif %}
+{% if let Some(error) = error %}
+<p class="notice" role="alert">{{ tr.t(error) }}</p>
+{% endif %}
+<ul class="people">
+  {% for person in people %}
+  <li>
+    <p><strong>{{ person.username }}</strong> · {{ tr.t(person.role) }} · {{ person.status }}{% if person.pending_setup %} · {{ tr.t("people-status-setup") }}{% endif %}</p>
+    {% if person.can_expire %}
+    <form method="post" action="/people/{{ person.id }}/expire">
+      <button type="submit" class="secondary">{{ tr.t("people-expire") }}</button>
+    </form>
+    {% endif %}
+    {% if let Some(expiry) = person.reactivate %}
+    <form method="post" action="/people/{{ person.id }}/reactivate" class="stack">
+      {% include "expiry_fields.html" %}
+      <button type="submit" class="secondary">{{ tr.t("people-reactivate") }}</button>
+    </form>
+    {% endif %}
+  </li>
+  {% endfor %}
+</ul>
+{% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 3b7964a..9d7f9ba 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -124,3 +124,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 115 | 2026-10-14 | Accounts | Guest expiry quick picks: **1 day, 1 week (default), 2 weeks** count from now; **weekend** lasts until the coming **Monday 00:00** local time (on a Monday, the next one); a **chosen day** lasts until the end of that local day | All picks end at midnight; weekend ends on Sunday at 20:00 | Spec §8.3 names the picks but not their ends; a guest invited in the evening for "1 day" keeps the whole next day, a chosen date includes that whole day |
 | 116 | 2026-10-14 | Settings | Changing one's password needs the **current password**, checked as a sign-in attempt under the same limits (#60); the change ends **every** session of the account (#58) and then **signs this device in again** with a new session id; the language buttons post to `/lang`, which returns to Settings | No current password (the session is enough); keep this device's session id | A phone left unlocked cannot take the account over or guess the password faster than at sign-in; the person who changed it stays signed in while every other device is signed out (spec §8.4) |
 | 117 | 2026-10-14 | Lists | Residents **add** rooms (with their area) and symptoms and **hide or show** any of them from Settings, through `smoke_domain::lists`; items are **never renamed or deleted**, and a name may repeat | Rename items; refuse duplicate names | A rename would change what every old report says (the evidence); hidden items stay readable on old reports and selectable when correcting them (#90); two "Balcony" entries are a visible, fixable nuisance, not data loss |
+| 118 | 2026-10-14 | UI | **People** lists every account (role, end of access, "waiting for setup") for residents only; others get **403** and no menu link (the header's `SignedIn` knows whether to show it). **End access now** expires a guest or viewer at once; an expired account shows a **Reactivate** form with the guest quick picks or, for a viewer, "never" or a day | Hide the page with 404; a separate page per person | Spec §9 screen 6 on one screen; a refused role sees the same "Not allowed" page as elsewhere (#101) |
```

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including `only_residents_reach_people_and_see_it_in_the_menu`,
`ending_a_guests_access_signs_them_out_and_leaves_them_read_only`,
`an_expired_viewer_can_be_reactivated_without_an_end`,
`a_guest_is_reactivated_only_with_a_future_end`, `a_resident_cannot_be_expired` and the three
`expiry` unit tests. With `TEST_POSTGRES_URL` set: **545** tests.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/error.rs \
  crates/web/src/header.rs \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/entries.rs \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/guide.rs \
  crates/web/src/routes/log.rs \
  crates/web/src/routes/login.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/people/expiry.rs \
  crates/web/src/routes/people/mod.rs \
  crates/web/src/routes/reports.rs \
  crates/web/src/routes/settings.rs \
  crates/web/static/app.css \
  crates/web/templates/base.html \
  crates/web/templates/expiry_fields.html \
  crates/web/templates/people.html \
  crates/web/tests/people.rs \
  crates/web/tests/security.rs \
  docs/DECISIONS.md
git commit -m "feat(people): residents list accounts and end or restore their access

The People page shows every account with its role and end of access; End access now
expires a guest or viewer (signing them out everywhere) and an expired account gets a
Reactivate form with the expiry picks. The header learns, through \`SignedIn\`, to show the
People link to residents only.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Invites and password reset with a QR code

Spec §8.3 steps 1–3. From People, a resident opens *Invite someone*: username, role (guest by
default) and, shown by CSS `:has` for the role picked, the guest quick picks or the viewer's
optional day. The post creates the account and answers with the QR page directly (the token
exists only in that response): the QR code is inline SVG drawn with `qrcodegen` (no `data:`
image, which the CSP forbids), the time the link stops working, and the link as plain text, not
clickable (decision #119). Every person on People gets *New setup QR code* or *Reset password*.
The invitee opens `/invite/{token}` without an account, chooses a password (twice) and a
language, and is signed in; a used, replaced, unknown or five-minute-old link answers 410 with
its own page (decision #120). Paths under `/invite/` are cut from the request logs, and from the
cross-site refusal's log line. The setup lands on `/` here; Task 8 sends it to the install page.

**Files:**
- Create: `crates/web/src/qr.rs`, `crates/web/src/routes/accept.rs`, `crates/web/src/routes/people/links.rs`, `crates/web/templates/accept.html`, `crates/web/templates/invite.html`, `crates/web/templates/link.html`, `crates/web/tests/invites.rs`
- Modify: `Cargo.toml`, `crates/web/Cargo.toml`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/error.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/routes/people/mod.rs`, `crates/web/src/security.rs`, `crates/web/static/app.css`, `crates/web/templates/people.html`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `people::{invite, issue_link, accept, NewPerson, PeopleError}` (Task 3),
  `InviteRepo::invite_by_token`, `Invite::is_usable` (Task 2), `auth::start_session`,
  `settings::{LANGUAGES, LanguageChoice}` (Task 4), `people::{require_manager, role_key}`,
  `expiry::{fields, read, ExpiryDraft, ExpiryFields}` (Task 6), `getrandom::fill`,
  `cookies::lang`.
- Produces (web):
  - `qr::svg(text: &str) -> String` (one `M{x},{y}h1v1h-1z` square per dark module, 4-module
    quiet zone, `fill` attributes only).
  - `AppError::LinkInvalid` (410; keys `error-link-title`, `error-link`).
  - `logged_path(&str) -> &str` in `lib.rs` (`/invite/…` for any invite path).
  - `routes::people::links::{form, create, renew, KEYS}`: `GET`/`POST /people/invite`
    (`username`, `role`, `guest_expiry`, `guest_expiry_date`, `viewer_expiry`,
    `viewer_expiry_date`), `POST /people/{id}/link`; templates `invite.html`, `link.html`.
  - `routes::accept::{form, submit}`: `GET`/`POST /invite/{token}` (`password`, `repeat`,
    `lang`); template `accept.html`.
  - Translation keys (24): `people-error-username`, `people-error-username-taken`,
    `people-invite`, `people-new-setup-link`, `people-reset-password`, `invite-title`,
    `invite-username-hint`, `invite-role`, `invite-submit`, `link-title`, `link-setup-title`,
    `link-reset-title`, `link-scan`, `link-valid-until`, `link-text`, `link-done`,
    `accept-title`, `accept-welcome`, `accept-setup-intro`, `accept-reset-intro`,
    `accept-password-hint`, `accept-submit`, `error-link-title`, `error-link`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/web/tests/invites.rs b/crates/web/tests/invites.rs
new file mode 100644
index 0000000..8d69981
--- /dev/null
+++ b/crates/web/tests/invites.rs
@@ -0,0 +1,237 @@
+//! Invites and password resets with a QR code (spec §8.3).
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use smoke_domain::records::Invite;
+use smoke_domain::repos::UserRepo;
+use smoke_domain::types::{Locale, Role, Username};
+
+const CHOSEN: &str = "my own long passphrase";
+
+/// The token in the link a QR page shows.
+fn token(page: &str) -> String {
+    let start = page.find("/invite/").expect("a link") + "/invite/".len();
+    page[start..start + 64].to_owned()
+}
+
+fn choose(password: &str, repeat: &str, lang: &str) -> String {
+    format!("password={password}&repeat={repeat}&lang={lang}").replace(' ', "+")
+}
+
+async fn invite_guest(app: &TestApp, cookie: &str, name: &str) -> String {
+    let body = format!("username={name}&role=guest&guest_expiry=1w");
+    let response = app
+        .send(post_form("/people/invite", &body, Some(cookie)))
+        .await;
+    assert_eq!(response.status(), StatusCode::OK);
+    text(response).await
+}
+
+#[tokio::test]
+async fn an_invited_guest_scans_sets_a_password_and_is_signed_in() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let form = text(app.send(get_with("/people/invite", &resident)).await).await;
+    assert!(
+        form.contains(r#"name="guest_expiry" value="1w" checked"#),
+        "{form}"
+    );
+
+    let page = invite_guest(&app, &resident, "Friend").await;
+    assert!(page.contains("<h1>Setup link for friend</h1>"), "{page}");
+    assert!(page.contains(r#"<svg class="qr""#), "{page}");
+    let token = token(&page);
+    assert!(token.bytes().all(|b| b.is_ascii_hexdigit()), "{token}");
+    assert!(page.contains(&format!("{BASE}/invite/{token}")), "{page}");
+    let friend = Username::new("friend").unwrap();
+    let invited = app.store.user_by_username(&friend).await.unwrap().unwrap();
+    assert_eq!(invited.password_hash, None);
+    assert_eq!(invited.role, Role::Guest);
+
+    let link = format!("/invite/{token}");
+    let welcome = text(app.send(get(&link)).await).await;
+    assert!(welcome.contains("<h1>Welcome, friend</h1>"), "{welcome}");
+    let response = app
+        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "it"), None))
+        .await;
+    assert_eq!(location(&response), Some("/"));
+    let cookie = session_cookie(&response).expect("signed in");
+    let home = text(app.send(get_with("/", &cookie)).await).await;
+    assert!(home.contains("Accesso effettuato come friend."), "{home}");
+
+    assert_eq!(app.send(get(&link)).await.status(), StatusCode::GONE);
+    let again = app
+        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
+        .await;
+    assert_eq!(again.status(), StatusCode::GONE);
+    assert!(text(again).await.contains("This link no longer works"));
+    let response = app.send(login_request("friend", CHOSEN)).await;
+    assert_eq!(response.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn a_link_works_for_five_minutes_only() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let link = format!(
+        "/invite/{}",
+        token(&invite_guest(&app, &resident, "friend").await)
+    );
+    app.clock.advance(Invite::LIFETIME);
+    assert_eq!(app.send(get(&link)).await.status(), StatusCode::GONE);
+    let response = app
+        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
+        .await;
+    assert_eq!(response.status(), StatusCode::GONE);
+    assert_eq!(
+        app.send(get("/invite/not-a-token")).await.status(),
+        StatusCode::GONE
+    );
+}
+
+#[tokio::test]
+async fn a_reset_link_replaces_the_password_and_signs_out_every_device() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let partner_phone = app.sign_in("partner").await;
+
+    let path = format!("/people/{}/link", partner.id);
+    let page = text(app.send(post_form(&path, "", Some(&resident))).await).await;
+    assert!(
+        page.contains("<h1>Password reset for partner</h1>"),
+        "{page}"
+    );
+    let link = format!("/invite/{}", token(&page));
+    let form = text(app.send(get(&link)).await).await;
+    assert!(form.contains("<h1>Choose a new password</h1>"), "{form}");
+
+    let response = app
+        .send(post_form(&link, &choose(CHOSEN, CHOSEN, "en"), None))
+        .await;
+    assert_eq!(location(&response), Some("/"));
+    assert_eq!(
+        location(&app.send(get_with("/", &partner_phone)).await),
+        Some("/login")
+    );
+    assert_eq!(
+        app.attempt("partner", PASSWORD).await.status(),
+        StatusCode::UNAUTHORIZED
+    );
+    let response = app.send(login_request("partner", CHOSEN)).await;
+    assert_eq!(response.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn a_newer_link_replaces_the_one_shown_before() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let first = token(&invite_guest(&app, &resident, "friend").await);
+    let friend = Username::new("friend").unwrap();
+    let invited = app.store.user_by_username(&friend).await.unwrap().unwrap();
+    let path = format!("/people/{}/link", invited.id);
+    let page = text(app.send(post_form(&path, "", Some(&resident))).await).await;
+    assert!(page.contains("<h1>Setup link for friend</h1>"), "{page}");
+    assert_eq!(
+        app.send(get(&format!("/invite/{first}"))).await.status(),
+        StatusCode::GONE
+    );
+    let second = format!("/invite/{}", token(&page));
+    assert_eq!(app.send(get(&second)).await.status(), StatusCode::OK);
+}
+
+#[tokio::test]
+async fn the_invite_form_says_what_to_fix_and_keeps_what_was_typed() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    for (body, message) in [
+        (
+            "username=no+spaces&role=guest&guest_expiry=1w",
+            "A username needs 3 to 32 characters",
+        ),
+        ("username=daniele&role=viewer", "That username is taken."),
+        (
+            "username=friend&role=guest&guest_expiry=date",
+            "Enter a valid date.",
+        ),
+    ] {
+        let response = app
+            .send(post_form("/people/invite", body, Some(&cookie)))
+            .await;
+        assert_eq!(
+            response.status(),
+            StatusCode::UNPROCESSABLE_ENTITY,
+            "{body}"
+        );
+        let html = text(response).await;
+        assert!(html.contains(message), "{body}: {html}");
+    }
+    let response = app
+        .send(post_form(
+            "/people/invite",
+            "username=friend&role=viewer",
+            Some(&cookie),
+        ))
+        .await;
+    assert_eq!(response.status(), StatusCode::OK);
+    let friend = Username::new("friend").unwrap();
+    let viewer = app.store.user_by_username(&friend).await.unwrap().unwrap();
+    assert_eq!((viewer.role, viewer.expires_at), (Role::Viewer, None));
+    assert_eq!(app.store.users().await.unwrap().len(), 2);
+}
+
+#[tokio::test]
+async fn the_chosen_password_must_be_valid_and_repeated() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    let link = format!(
+        "/invite/{}",
+        token(&invite_guest(&app, &resident, "friend").await)
+    );
+    for (body, message) in [
+        (
+            choose(CHOSEN, "something else", "en"),
+            "The two new passwords do not match.",
+        ),
+        (
+            choose("short", "short", "en"),
+            "The new password needs 10 to 128 characters.",
+        ),
+    ] {
+        let response = app.send(post_form(&link, &body, None)).await;
+        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
+        assert!(text(response).await.contains(message), "{message}");
+    }
+    assert_eq!(
+        app.send(get(&link)).await.status(),
+        StatusCode::OK,
+        "still usable"
+    );
+}
+
+#[tokio::test]
+async fn only_residents_create_links() {
+    let app = TestApp::new().await;
+    let resident = app.add_user("daniele", Role::Resident, Locale::En).await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let cookie = app.sign_in("guest").await;
+    let form = app.send(get_with("/people/invite", &cookie)).await;
+    assert_eq!(form.status(), StatusCode::FORBIDDEN);
+    let body = "username=friend&role=guest&guest_expiry=1w";
+    let create = app
+        .send(post_form("/people/invite", body, Some(&cookie)))
+        .await;
+    assert_eq!(create.status(), StatusCode::FORBIDDEN);
+    let path = format!("/people/{}/link", resident.id);
+    let reset = app.send(post_form(&path, "", Some(&cookie))).await;
+    assert_eq!(reset.status(), StatusCode::FORBIDDEN);
+    assert_eq!(app.store.users().await.unwrap().len(), 2);
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index 7ac2858..eb2ece7 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -219,11 +219,25 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         text(app.send(get_with("/guide/rating", &cookie)).await).await,
         text(app.send(get_with("/settings", &cookie)).await).await,
         text(app.send(get_with("/people", &cookie)).await).await,
+        text(app.send(get_with("/people/invite", &cookie)).await).await,
+        text(
+            app.send(post_form("/people/1/link", "", Some(&cookie)))
+                .await,
+        )
+        .await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
     assert!(pages[1].contains("No event: just a check"), "{}", pages[1]);
     assert!(pages[4].contains("None: a no-smell check"), "{}", pages[4]);
+    let link = pages[13].find("/invite/").expect("a reset link");
+    let accept = &pages[13][link..link + "/invite/".len() + 64];
+    let accept_page = text(app.send(get(accept)).await).await;
+    assert!(
+        accept_page.contains("<h1>Choose a new password</h1>"),
+        "{accept_page}"
+    );
+    let pages = pages.into_iter().chain([accept_page]);
     for html in pages {
         assert!(
             html.contains(r#"<meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>"#),
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: the new tests in `crates/web/tests/invites.rs` fail (`/people/invite`, `/people/{id}/link` and
`/invite/{token}` answer 404), and `no_page_needs_inline_scripts_styles_or_eval` panics with
"a reset link" (the link page is a 404).

- [ ] **Step 3: Implement**

```diff
diff --git a/Cargo.toml b/Cargo.toml
index 15bf2ae..0c4108b 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -24,6 +24,7 @@ getrandom = "0.4.3"
 hex = "0.4"
 http-body-util = "0.1.5"
 proptest = "1"
+qrcodegen = "1.8.0"
 rpassword = "7.5.4"
 serde = { version = "1", features = ["derive"] }
 serde_json = { version = "1", features = ["float_roundtrip"] }
diff --git a/crates/web/Cargo.toml b/crates/web/Cargo.toml
index ac2ea29..6391236 100644
--- a/crates/web/Cargo.toml
+++ b/crates/web/Cargo.toml
@@ -18,6 +18,7 @@ chrono-tz.workspace = true
 fluent-templates.workspace = true
 getrandom.workspace = true
 hex.workspace = true
+qrcodegen.workspace = true
 serde.workspace = true
 serde_json.workspace = true
 sha2.workspace = true
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 7276494..cc7f827 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -211,6 +211,28 @@ people-error-date = Enter a valid date.
 people-error-past = The end must be in the future.
 people-error-not-expired = This account has not expired.
 people-error-resident = Residents never expire.
+people-error-username = A username needs 3 to 32 characters: letters, digits, ".", "_" or "-".
+people-error-username-taken = That username is taken.
+people-invite = Invite someone
+people-new-setup-link = New setup QR code
+people-reset-password = Reset password
+invite-title = Invite someone
+invite-username-hint = They sign in with this name. Letters, digits, ".", "_" or "-".
+invite-role = Role
+invite-submit = Create the QR code
+link-title = QR code
+link-setup-title = Setup link for { $username }
+link-reset-title = Password reset for { $username }
+link-scan = Let them scan this code with their phone's camera.
+link-valid-until = It works once, until { $time } (5 minutes).
+link-text = Or send them the link:
+link-done = Done
+accept-title = Choose a new password
+accept-welcome = Welcome, { $username }
+accept-setup-intro = Choose your password and language to finish setting up your account.
+accept-reset-intro = Choose a new password for { $username }. Every device signed in to this account is signed out.
+accept-password-hint = 10 to 128 characters.
+accept-submit = Save and sign in
 expiry-legend = Access ends
 expiry-1-day = in 1 day
 expiry-weekend = after the weekend
@@ -227,6 +249,8 @@ error-not-allowed-title = Not allowed
 error-not-allowed = Your account cannot do this.
 error-internal-title = Something went wrong
 error-internal = Something went wrong (ref: { $ref }).
+error-link-title = This link no longer works
+error-link = Links work once and for 5 minutes. Ask a resident for a new QR code.
 error-back-home = Back to the start
 
 room-living-room = Living room
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 2dd35d0..cedda1e 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -211,6 +211,28 @@ people-error-date = Inserisci una data valida.
 people-error-past = La scadenza deve essere nel futuro.
 people-error-not-expired = Questo account non è scaduto.
 people-error-resident = I residenti non scadono mai.
+people-error-username = Un nome utente deve avere da 3 a 32 caratteri: lettere, cifre, ".", "_" o "-".
+people-error-username-taken = Questo nome utente è già in uso.
+people-invite = Invita qualcuno
+people-new-setup-link = Nuovo codice QR di configurazione
+people-reset-password = Reimposta la password
+invite-title = Invita qualcuno
+invite-username-hint = Accederà con questo nome. Lettere, cifre, ".", "_" o "-".
+invite-role = Ruolo
+invite-submit = Crea il codice QR
+link-title = Codice QR
+link-setup-title = Link di configurazione per { $username }
+link-reset-title = Reimpostazione password per { $username }
+link-scan = Fai inquadrare questo codice con la fotocamera del telefono.
+link-valid-until = Funziona una sola volta, fino alle { $time } (5 minuti).
+link-text = Oppure invia il link:
+link-done = Fatto
+accept-title = Scegli una nuova password
+accept-welcome = Benvenuto, { $username }
+accept-setup-intro = Scegli password e lingua per completare il tuo account.
+accept-reset-intro = Scegli una nuova password per { $username }. Tutti i dispositivi collegati a questo account verranno disconnessi.
+accept-password-hint = Da 10 a 128 caratteri.
+accept-submit = Salva e accedi
 expiry-legend = L'accesso termina
 expiry-1-day = tra 1 giorno
 expiry-weekend = dopo il fine settimana
@@ -227,6 +249,8 @@ error-not-allowed-title = Non consentito
 error-not-allowed = Il tuo account non può farlo.
 error-internal-title = Qualcosa è andato storto
 error-internal = Qualcosa è andato storto (rif: { $ref }).
+error-link-title = Questo link non funziona più
+error-link = I link funzionano una sola volta e per 5 minuti. Chiedi un nuovo codice QR a un residente.
 error-back-home = Torna all'inizio
 
 room-living-room = Soggiorno
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index ba98783..bd49234 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -211,6 +211,28 @@ people-error-date = Geçerli bir tarih girin.
 people-error-past = Bitiş gelecekte olmalı.
 people-error-not-expired = Bu hesabın süresi dolmadı.
 people-error-resident = Ev sakinlerinin süresi hiç dolmaz.
+people-error-username = Kullanıcı adı 3 ile 32 karakter olmalı: harf, rakam, ".", "_" veya "-".
+people-error-username-taken = Bu kullanıcı adı alınmış.
+people-invite = Birini davet et
+people-new-setup-link = Yeni kurulum QR kodu
+people-reset-password = Şifreyi sıfırla
+invite-title = Birini davet et
+invite-username-hint = Bu adla giriş yapacak. Harf, rakam, ".", "_" veya "-".
+invite-role = Rol
+invite-submit = QR kodunu oluştur
+link-title = QR kodu
+link-setup-title = { $username } için kurulum bağlantısı
+link-reset-title = { $username } için şifre sıfırlama
+link-scan = Bu kodu telefonunun kamerasıyla taratsın.
+link-valid-until = Yalnızca bir kez, saat { $time } kadar geçerli (5 dakika).
+link-text = Ya da bağlantıyı gönderin:
+link-done = Tamam
+accept-title = Yeni bir şifre seçin
+accept-welcome = Hoş geldiniz, { $username }
+accept-setup-intro = Hesabınızı tamamlamak için şifrenizi ve dilinizi seçin.
+accept-reset-intro = { $username } için yeni bir şifre seçin. Bu hesaba bağlı tüm cihazlarda oturum kapatılır.
+accept-password-hint = 10 ile 128 karakter arası.
+accept-submit = Kaydet ve giriş yap
 expiry-legend = Erişimin bitişi
 expiry-1-day = 1 gün sonra
 expiry-weekend = hafta sonundan sonra
@@ -227,6 +249,8 @@ error-not-allowed-title = İzin yok
 error-not-allowed = Hesabın bunu yapamaz.
 error-internal-title = Bir şeyler ters gitti
 error-internal = Bir şeyler ters gitti (ref: { $ref }).
+error-link-title = Bu bağlantı artık çalışmıyor
+error-link = Bağlantılar yalnızca bir kez ve 5 dakika geçerlidir. Bir ev sakininden yeni bir QR kodu isteyin.
 error-back-home = Başa dön
 
 room-living-room = Oturma odası
diff --git a/crates/web/src/error.rs b/crates/web/src/error.rs
index 7242acd..2c43f9c 100644
--- a/crates/web/src/error.rs
+++ b/crates/web/src/error.rs
@@ -24,6 +24,8 @@ pub(crate) enum AppError {
     Forbidden,
     /// The signed-in user's role does not allow this (spec §4).
     NotAllowed,
+    /// An invite or reset link that is unknown, used, or older than five minutes.
+    LinkInvalid,
     /// Anything unexpected. Logged with the request id; the user sees only the id.
     Internal(Box<dyn StdError + Send + Sync>),
 }
@@ -37,6 +39,7 @@ impl AppError {
         match self {
             Self::NotFound => StatusCode::NOT_FOUND,
             Self::Forbidden | Self::NotAllowed => StatusCode::FORBIDDEN,
+            Self::LinkInvalid => StatusCode::GONE,
             Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
         }
     }
@@ -47,6 +50,7 @@ impl AppError {
             Self::NotFound => ("error-not-found-title", "error-not-found"),
             Self::Forbidden => ("error-forbidden-title", "error-forbidden"),
             Self::NotAllowed => ("error-not-allowed-title", "error-not-allowed"),
+            Self::LinkInvalid => ("error-link-title", "error-link"),
             Self::Internal(_) => ("error-internal-title", "error-internal"),
         }
     }
@@ -181,13 +185,15 @@ pub(crate) fn page(status: StatusCode, template: &impl Template) -> Result<Respo
 
 /// Every message key the error pages use, for the translation tests.
 #[cfg(test)]
-pub(crate) const KEYS: [&str; 8] = [
+pub(crate) const KEYS: [&str; 10] = [
     "error-not-found-title",
     "error-not-found",
     "error-forbidden-title",
     "error-forbidden",
     "error-not-allowed-title",
     "error-not-allowed",
+    "error-link-title",
+    "error-link",
     "error-internal-title",
     "error-internal",
 ];
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 6cf6a93..b2ac938 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -11,6 +11,7 @@ mod forms;
 mod header;
 mod i18n;
 mod labels;
+mod qr;
 mod request_id;
 mod routes;
 mod security;
@@ -58,6 +59,11 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
         )
         .route("/reports/{id}/move", post(routes::reports::relocate::<S>))
         .route("/people", get(routes::people::list::<S>))
+        .route(
+            "/people/invite",
+            get(routes::people::links::form::<S>).post(routes::people::links::create::<S>),
+        )
+        .route("/people/{id}/link", post(routes::people::links::renew::<S>))
         .route("/people/{id}/expire", post(routes::people::end_access::<S>))
         .route(
             "/people/{id}/reactivate",
@@ -86,6 +92,10 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::login::form).post(routes::login::submit::<S>),
         )
         .route("/logout", post(routes::login::logout))
+        .route(
+            "/invite/{token}",
+            get(routes::accept::form::<S>).post(routes::accept::submit::<S>),
+        )
         .route("/lang", post(routes::lang::choose::<S>))
         .fallback(routes::not_found)
         .layer(CatchPanicLayer::custom(error::panic_response))
@@ -125,7 +135,18 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
 
 fn request_span(request: &Request) -> tracing::Span {
     let id = request_id::of(request.headers());
-    tracing::info_span!("request", id, method = %request.method(), path = request.uri().path())
+    let path = logged_path(request.uri().path());
+    tracing::info_span!("request", id, method = %request.method(), path)
+}
+
+/// The path as logs show it. An invite token works like a password until it is used, so it
+/// never reaches the logs (decision #119).
+pub(crate) fn logged_path(path: &str) -> &str {
+    if path.starts_with("/invite/") {
+        "/invite/…"
+    } else {
+        path
+    }
 }
 
 /// Serves `app` until `shutdown` completes, then finishes the requests in flight.
@@ -141,3 +162,15 @@ pub async fn serve(
     .with_graceful_shutdown(shutdown)
     .await
 }
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn invite_tokens_are_cut_from_logged_paths() {
+        assert_eq!(logged_path("/invite/0123abcd"), "/invite/…");
+        assert_eq!(logged_path("/events/3"), "/events/3");
+        assert_eq!(logged_path("/people/invite"), "/people/invite");
+    }
+}
diff --git a/crates/web/src/qr.rs b/crates/web/src/qr.rs
new file mode 100644
index 0000000..8b4a65c
--- /dev/null
+++ b/crates/web/src/qr.rs
@@ -0,0 +1,67 @@
+//! QR codes as inline SVG (spec §8.3). The page embeds the markup directly: the
+//! Content-Security-Policy allows no `data:` images, and a link's token exists only while the
+//! page that shows it is rendered.
+
+use std::fmt::Write;
+
+use qrcodegen::{QrCode, QrCodeEcc};
+
+/// The light margin around the code, in modules, as the QR specification asks.
+const QUIET_ZONE: i32 = 4;
+
+/// An SVG drawing of `text` as a QR code with medium error correction, one square per dark
+/// module. Colours are presentation attributes, so no style is needed.
+pub(crate) fn svg(text: &str) -> String {
+    let code =
+        QrCode::encode_text(text, QrCodeEcc::Medium).expect("a link is far below the QR capacity");
+    let side = code.size() + 2 * QUIET_ZONE;
+    let mut path = String::new();
+    for y in 0..code.size() {
+        for x in 0..code.size() {
+            if code.get_module(x, y) {
+                let (left, top) = (x + QUIET_ZONE, y + QUIET_ZONE);
+                write!(path, "M{left},{top}h1v1h-1z").expect("writing to a String cannot fail");
+            }
+        }
+    }
+    format!(
+        "<svg class=\"qr\" xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {side} {side}\" \
+         shape-rendering=\"crispEdges\" aria-hidden=\"true\">\
+         <rect width=\"{side}\" height=\"{side}\" fill=\"#fff\"/>\
+         <path d=\"{path}\" fill=\"#000\"/></svg>"
+    )
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    const LINK: &str = "https://smoke.example.org/invite/0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
+
+    #[test]
+    fn the_drawing_has_one_square_per_dark_module_inside_a_quiet_zone() {
+        let code = QrCode::encode_text(LINK, QrCodeEcc::Medium).unwrap();
+        let dark = (0..code.size())
+            .flat_map(|y| (0..code.size()).map(move |x| (x, y)))
+            .filter(|&(x, y)| code.get_module(x, y))
+            .count();
+        let drawing = svg(LINK);
+        let side = code.size() + 8;
+        assert!(drawing.contains(&format!("viewBox=\"0 0 {side} {side}\"")));
+        assert_eq!(drawing.matches("h1v1h-1z").count(), dark);
+        assert!(
+            drawing.contains("M4,4h1v1h-1z"),
+            "the finder pattern starts at the margin"
+        );
+    }
+
+    #[test]
+    fn the_drawing_needs_no_style_and_carries_only_the_code() {
+        let drawing = svg(LINK);
+        assert!(!drawing.contains("style"));
+        assert!(
+            !drawing.contains("invite"),
+            "the link is drawn, never written"
+        );
+    }
+}
diff --git a/crates/web/src/routes/accept.rs b/crates/web/src/routes/accept.rs
new file mode 100644
index 0000000..da91fba
--- /dev/null
+++ b/crates/web/src/routes/accept.rs
@@ -0,0 +1,163 @@
+//! Opening an invite or reset link (spec §8.3 step 3): the invitee chooses a password and a
+//! language and is signed in. Works without an account; the link itself is the credential, for
+//! five minutes and once.
+
+use askama::Template;
+use axum::Form;
+use axum::extract::{Path, State};
+use axum::http::StatusCode;
+use axum::http::header::SET_COOKIE;
+use axum::response::{IntoResponse, Redirect, Response};
+use serde::Deserialize;
+use smoke_domain::people::{PeopleError, accept};
+use smoke_domain::records::Invite;
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{InvitePurpose, Locale, NewPassword, TokenHash};
+use tower_sessions::Session;
+
+use crate::auth::{self, password};
+use crate::cookies;
+use crate::error::{AppError, page};
+use crate::header::SignedIn;
+use crate::i18n::Tr;
+use crate::routes::people_refusal;
+use crate::routes::settings::{LANGUAGES, LanguageChoice};
+use crate::state::AppState;
+
+#[derive(Template)]
+#[template(path = "accept.html")]
+struct AcceptPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    username: String,
+    setup: bool,
+    languages: Vec<LanguageChoice>,
+    error: Option<&'static str>,
+}
+
+/// The usable invite behind `token` and its person's username, or the "link no longer works"
+/// page.
+async fn usable<S: Store>(
+    state: &AppState<S>,
+    token: &TokenHash,
+) -> Result<(Invite, String), AppError> {
+    let now = state.clock.now();
+    let invite = state
+        .store
+        .invite_by_token(token)
+        .await?
+        .filter(|invite| invite.is_usable(now))
+        .ok_or(AppError::LinkInvalid)?;
+    let person = state
+        .store
+        .user(invite.user_id)
+        .await?
+        .ok_or(AppError::LinkInvalid)?;
+    Ok((invite, person.username.to_string()))
+}
+
+fn accept_page(
+    status: StatusCode,
+    tr: Tr,
+    invite: &Invite,
+    username: String,
+    error: Option<&'static str>,
+) -> Result<Response, AppError> {
+    page(
+        status,
+        &AcceptPage {
+            tr,
+            signed_in: None,
+            username,
+            setup: invite.purpose == InvitePurpose::Setup,
+            languages: LANGUAGES
+                .iter()
+                .map(|&(locale, name)| LanguageChoice {
+                    value: locale.as_str(),
+                    name,
+                    current: locale == tr.0,
+                })
+                .collect(),
+            error,
+        },
+    )
+}
+
+pub(crate) async fn form<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    tr: Tr,
+    Path(token): Path<String>,
+) -> Result<Response, AppError> {
+    let (invite, username) = usable(&state, &TokenHash::of(&token)).await?;
+    accept_page(StatusCode::OK, tr, &invite, username, None)
+}
+
+#[derive(Deserialize)]
+pub(crate) struct AcceptForm {
+    password: String,
+    repeat: String,
+    lang: Option<String>,
+}
+
+/// Sets the password and language, uses up the link, and signs the person in on this device.
+/// After a first setup the next page offers to install the app (spec §8.3 step 4).
+pub(crate) async fn submit<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    session: Session,
+    tr: Tr,
+    Path(token): Path<String>,
+    Form(form): Form<AcceptForm>,
+) -> Result<Response, AppError> {
+    let token = TokenHash::of(&token);
+    let (invite, username) = usable(&state, &token).await?;
+    let locale = form
+        .lang
+        .and_then(|lang| lang.parse::<Locale>().ok())
+        .unwrap_or(tr.0);
+    if form.password != form.repeat {
+        let error = Some("settings-password-mismatch");
+        return accept_page(
+            StatusCode::UNPROCESSABLE_ENTITY,
+            tr,
+            &invite,
+            username,
+            error,
+        );
+    }
+    let Ok(new_password) = NewPassword::new(form.password) else {
+        let error = Some("settings-password-length");
+        return accept_page(
+            StatusCode::UNPROCESSABLE_ENTITY,
+            tr,
+            &invite,
+            username,
+            error,
+        );
+    };
+    let hash = password::run_limited(state.hash_permits.clone(), move || {
+        password::hash(&new_password)
+    })
+    .await?
+    .map_err(AppError::internal)?;
+    let now = state.clock.now();
+    let mut tx = state.store.begin(Some(invite.user_id)).await?;
+    let person = match accept(&mut tx, &token, hash, locale, now).await {
+        Ok(person) => person,
+        Err(PeopleError::LinkInvalid) => return Err(AppError::LinkInvalid),
+        Err(other) => return Err(people_refusal(other)),
+    };
+    tx.commit().await?;
+    tracing::info!(user = person.id.0, purpose = %invite.purpose, "link used");
+    let target = if !person.actor().can_sign_in(now) {
+        "/login"
+    } else {
+        auth::start_session(&session, &person, now).await?;
+        "/"
+    };
+    let mut response = Redirect::to(target).into_response();
+    response.headers_mut().append(
+        SET_COOKIE,
+        cookies::lang(person.locale, state.config.base_url.is_https()),
+    );
+    Ok(response)
+}
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index 3252c1d..f3d0625 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,5 +1,6 @@
 //! One module per feature.
 
+pub(crate) mod accept;
 pub(crate) mod entries;
 pub(crate) mod events;
 pub(crate) mod guide;
diff --git a/crates/web/src/routes/people/links.rs b/crates/web/src/routes/people/links.rs
new file mode 100644
index 0000000..189d884
--- /dev/null
+++ b/crates/web/src/routes/people/links.rs
@@ -0,0 +1,242 @@
+//! Inviting someone and resetting a password (spec §8.3): the resident's side. The app shows a
+//! QR code for `{APP_BASE_URL}/invite/{token}`; the token exists only in that page, the database
+//! keeps its hash.
+
+use askama::Template;
+use axum::extract::{Path, State};
+use axum::http::StatusCode;
+use axum::response::Response;
+use chrono::{DateTime, Utc};
+use smoke_domain::DomainError;
+use smoke_domain::people::{NewPerson, PeopleError, invite, issue_link};
+use smoke_domain::records::{Invite, User};
+use smoke_domain::repos::{Commit, Store};
+use smoke_domain::types::{InvitePurpose, Role, TokenHash, UserId, Username};
+
+use crate::auth::CurrentUser;
+use crate::error::{AppError, page};
+use crate::forms::Pairs;
+use crate::header::SignedIn;
+use crate::i18n::Tr;
+use crate::routes::people::expiry::{self, ExpiryDraft, ExpiryFields};
+use crate::routes::people::{require_manager, role_key};
+use crate::routes::people_refusal;
+use crate::state::AppState;
+use crate::{qr, time};
+
+/// The roles a resident can give, guest first: the usual case (spec §8.3).
+const ROLES: [Role; 3] = [Role::Guest, Role::Viewer, Role::Resident];
+
+/// A new link's token: 32 random bytes, as 64 hex characters.
+fn new_token() -> Result<String, AppError> {
+    let mut bytes = [0u8; 32];
+    getrandom::fill(&mut bytes).map_err(|e| AppError::internal(e.to_string()))?;
+    Ok(hex::encode(bytes))
+}
+
+pub(crate) struct RoleChoice {
+    pub(crate) value: &'static str,
+    pub(crate) key: &'static str,
+    pub(crate) checked: bool,
+}
+
+/// The invite form as typed, to show it again after a refusal.
+#[derive(Debug, Clone, Default)]
+struct InviteDraft {
+    username: String,
+    role: Option<Role>,
+    guest: ExpiryDraft,
+    viewer: ExpiryDraft,
+}
+
+impl InviteDraft {
+    fn from_form(form: &Pairs) -> Self {
+        Self {
+            username: form.get("username").unwrap_or_default().to_owned(),
+            role: form.get("role").and_then(|role| role.parse().ok()),
+            guest: ExpiryDraft::from_form(form, "guest_"),
+            viewer: ExpiryDraft::from_form(form, "viewer_"),
+        }
+    }
+}
+
+#[derive(Template)]
+#[template(path = "invite.html")]
+struct InvitePage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    error: Option<&'static str>,
+    username: String,
+    roles: Vec<RoleChoice>,
+    guest: Option<ExpiryFields>,
+    viewer: Option<ExpiryFields>,
+}
+
+fn invite_page(
+    status: StatusCode,
+    user: &User,
+    tr: Tr,
+    now: DateTime<Utc>,
+    draft: InviteDraft,
+    error: Option<&'static str>,
+) -> Result<Response, AppError> {
+    let role = draft.role.unwrap_or(Role::Guest);
+    page(
+        status,
+        &InvitePage {
+            tr,
+            signed_in: SignedIn::of(user, now),
+            error,
+            username: draft.username,
+            roles: ROLES
+                .iter()
+                .map(|&choice| RoleChoice {
+                    value: choice.as_str(),
+                    key: role_key(choice),
+                    checked: choice == role,
+                })
+                .collect(),
+            guest: expiry::fields(Role::Guest, "guest_", &draft.guest),
+            viewer: expiry::fields(Role::Viewer, "viewer_", &draft.viewer),
+        },
+    )
+}
+
+pub(crate) async fn form<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    require_manager(&user, now)?;
+    invite_page(StatusCode::OK, &user, tr, now, InviteDraft::default(), None)
+}
+
+#[derive(Template)]
+#[template(path = "link.html")]
+struct LinkPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    username: String,
+    setup: bool,
+    url: String,
+    qr: String,
+    until: String,
+}
+
+/// The QR code page. Rendered in the response to the post that made the link, never stored,
+/// so reloading it cannot show the link again.
+fn link_page<S>(
+    state: &AppState<S>,
+    user: &User,
+    tr: Tr,
+    person: &User,
+    link: &Invite,
+    token: &str,
+) -> Result<Response, AppError> {
+    let url = format!("{}/invite/{token}", state.config.base_url.origin());
+    let now = state.clock.now();
+    page(
+        StatusCode::OK,
+        &LinkPage {
+            tr,
+            signed_in: SignedIn::of(user, now),
+            username: person.username.to_string(),
+            setup: link.purpose == InvitePurpose::Setup,
+            qr: qr::svg(&url),
+            url,
+            until: time::clock(link.expires_at, state.config.schedule.tz()),
+        },
+    )
+}
+
+/// Creates the account and shows its setup QR code (spec §8.3 steps 1–2).
+pub(crate) async fn create<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    form: Pairs,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    require_manager(&user, now)?;
+    let draft = InviteDraft::from_form(&form);
+    let refuse = |draft: InviteDraft, key: &'static str| {
+        invite_page(
+            StatusCode::UNPROCESSABLE_ENTITY,
+            &user,
+            tr,
+            now,
+            draft,
+            Some(key),
+        )
+    };
+    let Ok(username) = Username::new(&draft.username) else {
+        return refuse(draft, "people-error-username");
+    };
+    let Some(role) = draft.role else {
+        return refuse(draft, "log-error-form");
+    };
+    let typed = match role {
+        Role::Viewer => &draft.viewer,
+        _ => &draft.guest,
+    };
+    let pick = match expiry::read(role, typed) {
+        Ok(pick) => pick,
+        Err(error) => return refuse(draft, error.key()),
+    };
+    let person = NewPerson {
+        username,
+        role,
+        locale: user.locale,
+        expires_at: pick.resolve(now, &state.config.schedule),
+    };
+    let token = new_token()?;
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    let (invited, link) =
+        match invite(&mut tx, &user.actor(), person, TokenHash::of(&token), now).await {
+            Ok(created) => created,
+            Err(error) => {
+                drop(tx);
+                let key = match error {
+                    PeopleError::UsernameTaken => "people-error-username-taken",
+                    PeopleError::Invalid(DomainError::ExpiryNotInFuture) => "people-error-past",
+                    other => return Err(people_refusal(other)),
+                };
+                return refuse(draft, key);
+            }
+        };
+    tx.commit().await?;
+    tracing::info!(user = invited.id.0, role = %invited.role, "invited");
+    link_page(&state, &user, tr, &invited, &link, &token)
+}
+
+/// A new setup or password-reset QR code for an existing account (spec §8.3).
+pub(crate) async fn renew<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Path(id): Path<i64>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let token = new_token()?;
+    let mut tx = state.store.begin(Some(user.id)).await?;
+    let (person, link) = issue_link(
+        &mut tx,
+        &user.actor(),
+        UserId(id),
+        TokenHash::of(&token),
+        now,
+    )
+    .await
+    .map_err(people_refusal)?;
+    tx.commit().await?;
+    link_page(&state, &user, tr, &person, &link, &token)
+}
+
+/// Every key this module passes through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) const KEYS: [&str; 3] = [
+    "people-error-username",
+    "people-error-username-taken",
+    "log-error-form",
+];
diff --git a/crates/web/src/routes/people/mod.rs b/crates/web/src/routes/people/mod.rs
index 41e9c61..2a4e420 100644
--- a/crates/web/src/routes/people/mod.rs
+++ b/crates/web/src/routes/people/mod.rs
@@ -2,6 +2,7 @@
 //! buttons that end or restore their access.
 
 pub(crate) mod expiry;
+pub(crate) mod links;
 
 use askama::Template;
 use axum::extract::{Path, Query, State};
@@ -240,5 +241,6 @@ pub(crate) fn keys() -> Vec<&'static str> {
     ];
     keys.extend(Role::ALL.iter().map(|&role| role_key(role)));
     keys.extend(expiry::keys());
+    keys.extend(links::KEYS);
     keys
 }
diff --git a/crates/web/src/security.rs b/crates/web/src/security.rs
index 42811b0..952ccc6 100644
--- a/crates/web/src/security.rs
+++ b/crates/web/src/security.rs
@@ -66,7 +66,7 @@ pub(crate) async fn check_origin(
         let headers = request.headers();
         tracing::warn!(
             method = %request.method(),
-            path = ?request.uri().path(),
+            path = crate::logged_path(request.uri().path()),
             sec_fetch_site = ?headers.get("sec-fetch-site"),
             origin = ?headers.get(ORIGIN),
             "refused a cross-site request"
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 96e9a19..51b34aa 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -320,3 +320,28 @@ button.current {
   padding: 0.5rem;
   border-radius: 0.5rem;
 }
+
+.invite:has(input[name="role"]:checked) .expiry {
+  display: none;
+}
+
+.invite:has(input[name="role"][value="guest"]:checked) .expiry[data-role="guest"],
+.invite:has(input[name="role"][value="viewer"]:checked) .expiry[data-role="viewer"] {
+  display: flex;
+}
+
+.qr-code {
+  margin: 0;
+  max-width: 18rem;
+}
+
+.qr {
+  display: block;
+  width: 100%;
+  height: auto;
+}
+
+.link-text {
+  overflow-wrap: anywhere;
+  font-size: 0.75rem;
+}
diff --git a/crates/web/templates/accept.html b/crates/web/templates/accept.html
new file mode 100644
index 0000000..48abbd4
--- /dev/null
+++ b/crates/web/templates/accept.html
@@ -0,0 +1,26 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("accept-title") }}{% endblock %}
+{% block content %}
+<h1>{% if setup %}{{ tr.t1("accept-welcome", "username", username) }}{% else %}{{ tr.t("accept-title") }}{% endif %}</h1>
+<p>{% if setup %}{{ tr.t("accept-setup-intro") }}{% else %}{{ tr.t1("accept-reset-intro", "username", username) }}{% endif %}</p>
+{% if let Some(error) = error %}
+<p class="notice" role="alert">{{ tr.t(error) }}</p>
+{% endif %}
+<form method="post" class="stack">
+  <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
+  <label>{{ tr.t("settings-password-new") }}
+    <input type="password" name="password" autocomplete="new-password" minlength="10" maxlength="128" required>
+  </label>
+  <label>{{ tr.t("settings-password-repeat") }}
+    <input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>
+  </label>
+  <p class="hint">{{ tr.t("accept-password-hint") }}</p>
+  <fieldset class="choices">
+    <legend>{{ tr.t("language") }}</legend>
+    {% for language in languages %}
+    <label class="chip"><input type="radio" name="lang" value="{{ language.value }}"{% if language.current %} checked{% endif %}><span lang="{{ language.value }}">{{ language.name }}</span></label>
+    {% endfor %}
+  </fieldset>
+  <button type="submit">{{ tr.t("accept-submit") }}</button>
+</form>
+{% endblock %}
diff --git a/crates/web/templates/invite.html b/crates/web/templates/invite.html
new file mode 100644
index 0000000..5aa7179
--- /dev/null
+++ b/crates/web/templates/invite.html
@@ -0,0 +1,24 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("invite-title") }}{% endblock %}
+{% block content %}
+<h1>{{ tr.t("invite-title") }}</h1>
+{% if let Some(error) = error %}
+<p class="notice" role="alert">{{ tr.t(error) }}</p>
+{% endif %}
+<form method="post" action="/people/invite" class="stack invite">
+  <label>{{ tr.t("login-username") }}
+    <input name="username" value="{{ username }}" autocomplete="off" autocapitalize="none"
+           autocorrect="off" spellcheck="false" maxlength="32" required>
+  </label>
+  <p class="hint">{{ tr.t("invite-username-hint") }}</p>
+  <fieldset class="choices">
+    <legend>{{ tr.t("invite-role") }}</legend>
+    {% for role in roles %}
+    <label class="chip"><input type="radio" name="role" value="{{ role.value }}"{% if role.checked %} checked{% endif %}><span>{{ tr.t(role.key) }}</span></label>
+    {% endfor %}
+  </fieldset>
+  {% if let Some(expiry) = guest %}{% include "expiry_fields.html" %}{% endif %}
+  {% if let Some(expiry) = viewer %}{% include "expiry_fields.html" %}{% endif %}
+  <button type="submit">{{ tr.t("invite-submit") }}</button>
+</form>
+{% endblock %}
diff --git a/crates/web/templates/link.html b/crates/web/templates/link.html
new file mode 100644
index 0000000..4400d9f
--- /dev/null
+++ b/crates/web/templates/link.html
@@ -0,0 +1,11 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("link-title") }}{% endblock %}
+{% block content %}
+<h1>{% if setup %}{{ tr.t1("link-setup-title", "username", username) }}{% else %}{{ tr.t1("link-reset-title", "username", username) }}{% endif %}</h1>
+<p>{{ tr.t("link-scan") }}</p>
+<figure class="qr-code">{{ qr|safe }}</figure>
+<p class="notice">{{ tr.t1("link-valid-until", "time", until) }}</p>
+<p class="hint">{{ tr.t("link-text") }}</p>
+<p><code class="link-text">{{ url }}</code></p>
+<p><a href="/people">{{ tr.t("link-done") }}</a></p>
+{% endblock %}
diff --git a/crates/web/templates/people.html b/crates/web/templates/people.html
index 311b4e9..a67e97a 100644
--- a/crates/web/templates/people.html
+++ b/crates/web/templates/people.html
@@ -8,10 +8,14 @@
 {% if let Some(error) = error %}
 <p class="notice" role="alert">{{ tr.t(error) }}</p>
 {% endif %}
+<p><a href="/people/invite">{{ tr.t("people-invite") }}</a></p>
 <ul class="people">
   {% for person in people %}
   <li>
     <p><strong>{{ person.username }}</strong> · {{ tr.t(person.role) }} · {{ person.status }}{% if person.pending_setup %} · {{ tr.t("people-status-setup") }}{% endif %}</p>
+    <form method="post" action="/people/{{ person.id }}/link">
+      <button type="submit" class="secondary">{% if person.pending_setup %}{{ tr.t("people-new-setup-link") }}{% else %}{{ tr.t("people-reset-password") }}{% endif %}</button>
+    </form>
     {% if person.can_expire %}
     <form method="post" action="/people/{{ person.id }}/expire">
       <button type="submit" class="secondary">{{ tr.t("people-expire") }}</button>
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 9d7f9ba..b8feb69 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -125,3 +125,5 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 116 | 2026-10-14 | Settings | Changing one's password needs the **current password**, checked as a sign-in attempt under the same limits (#60); the change ends **every** session of the account (#58) and then **signs this device in again** with a new session id; the language buttons post to `/lang`, which returns to Settings | No current password (the session is enough); keep this device's session id | A phone left unlocked cannot take the account over or guess the password faster than at sign-in; the person who changed it stays signed in while every other device is signed out (spec §8.4) |
 | 117 | 2026-10-14 | Lists | Residents **add** rooms (with their area) and symptoms and **hide or show** any of them from Settings, through `smoke_domain::lists`; items are **never renamed or deleted**, and a name may repeat | Rename items; refuse duplicate names | A rename would change what every old report says (the evidence); hidden items stay readable on old reports and selectable when correcting them (#90); two "Balcony" entries are a visible, fixable nuisance, not data loss |
 | 118 | 2026-10-14 | UI | **People** lists every account (role, end of access, "waiting for setup") for residents only; others get **403** and no menu link (the header's `SignedIn` knows whether to show it). **End access now** expires a guest or viewer at once; an expired account shows a **Reactivate** form with the guest quick picks or, for a viewer, "never" or a day | Hide the page with 404; a separate page per person | Spec §9 screen 6 on one screen; a refused role sees the same "Not allowed" page as elsewhere (#101) |
+| 119 | 2026-10-14 | Invites | A link's token is **32 random bytes in hex**; the QR code is **inline SVG drawn with `qrcodegen` 1.8** (MIT, no dependencies, Project Nayuki's reference encoder) in the response to the post that made it, never stored; the page also shows the link as **plain text, not clickable**; paths under `/invite/` are **cut from the logs**; a dead link answers **410** with its own page | A `data:` PNG (blocked by the CSP); the `qrcode` crate (pulls in `image` by default); a stored token to show the QR again; a clickable link | No image library, no CSP exception, a few kB more binary; a resident who taps the link would sign their own phone in as the invitee; Cloud Run keeps request logs, and an unused token is a password |
+| 120 | 2026-10-14 | Invites | Opening a link asks for the **password twice and a language** (preselected from the page's language); saving uses up the link, ends the account's sessions and **signs this device in**, unless the account may not sign in (an expired viewer goes to the sign-in page); a new account starts in the **inviter's language** until then | Ask for the username too; email confirmation | Spec §8.3 step 3; the username is fixed by the resident and shown on the page |
```

Add the dependency before building (`Cargo.lock` gains the `qrcodegen 1.8.0` entry by itself on
the next build; commit it). `cargo deny check licenses bans sources` passes (MIT).

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including `an_invited_guest_scans_sets_a_password_and_is_signed_in`,
`a_link_works_for_five_minutes_only`, `a_reset_link_replaces_the_password_and_signs_out_every_device`,
`a_newer_link_replaces_the_one_shown_before`,
`the_invite_form_says_what_to_fix_and_keeps_what_was_typed`,
`the_chosen_password_must_be_valid_and_repeated`, `only_residents_create_links`,
`invite_tokens_are_cut_from_logged_paths` and the two `qr` unit tests. With `TEST_POSTGRES_URL`
set: **555** tests. No test decodes the QR code (no decoder is installed): list "scan one
invite QR code with a phone; it opens `{APP_BASE_URL}/invite/…`" among the user's checks in the
pull request description.

- [ ] **Step 5: Commit**

```bash
git add Cargo.lock \
  Cargo.toml \
  crates/web/Cargo.toml \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/error.rs \
  crates/web/src/lib.rs \
  crates/web/src/qr.rs \
  crates/web/src/routes/accept.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/routes/people/links.rs \
  crates/web/src/routes/people/mod.rs \
  crates/web/src/security.rs \
  crates/web/static/app.css \
  crates/web/templates/accept.html \
  crates/web/templates/invite.html \
  crates/web/templates/link.html \
  crates/web/templates/people.html \
  crates/web/tests/invites.rs \
  crates/web/tests/security.rs \
  docs/DECISIONS.md
git commit -m "feat(invites): QR-code links for account setup and password reset

Residents invite people with the guest quick picks and get a QR code (inline SVG from
\`qrcodegen\`) for \`/invite/{token}\`; People also issues setup or reset links for existing
accounts. Opening a link asks for a password and a language, uses it up and signs the
person in. Dead links answer 410, and invite paths never reach the logs.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: PWA — manifest, service worker, offline page, install prompt

Spec §9.1, §9 screen 8 and §8.3 step 4. The manifest (`/static/manifest.webmanifest`, scope and
start URL `/`, a "New smell report" shortcut to the Log screen) lists PNG icons of 192 and 512
px (the 512 also as maskable) and an SVG; every page links it, plus an Apple touch icon and a
theme colour. The service worker is served at `/sw.js` (root scope) and goes to the network
first: static assets fall back to their cached copy and page navigations to a cached `/offline`
page; posts and other origins are never touched (no offline logging, spec §14). The header's
*Install app* link opens the browser's prompt where there is one (`beforeinstallprompt`) and
otherwise `/install`, which also has the native button (shown by script) and the illustrated
iPhone steps; CSS hides the link in standalone mode. A first account setup now lands on
`/install?welcome` (decision #121). The roadmap links this plan.

**Files:**
- Create: `crates/web/src/routes/install.rs`, `crates/web/static/icon.svg`, `crates/web/static/manifest.webmanifest`, `crates/web/static/sw.js`, `crates/web/templates/install.html`, `crates/web/templates/offline.html`, `crates/web/tests/pwa.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/assets.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/accept.rs`, `crates/web/src/routes/mod.rs`, `crates/web/static/app.css`, `crates/web/static/app.js`, `crates/web/templates/base.html`, `crates/web/tests/invites.rs`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`, `docs/plans/ROADMAP.md`
- Create (binary, see Step 3): `crates/web/static/icon-180.png`, `crates/web/static/icon-192.png`, `crates/web/static/icon-512.png`

**Interfaces:**
- Consumes: `accept::submit` (Task 7), `SignedIn` (Task 6), the asset list of decision #65.
- Produces (web):
  - `assets::service_worker` (route `GET /sw.js`, outside sessions); `ASSETS` gains
    `icon-180.png`, `icon-192.png`, `icon-512.png`, `icon.svg`, `manifest.webmanifest`
    (`application/manifest+json`) and `sw.js`.
  - `routes::install::{show, offline}`: `GET /install` (`?welcome`), `GET /offline`, both public;
    templates `install.html`, `offline.html`.
  - `static/app.js`: registers `/sw.js`; any `[data-install]` element opens the deferred
    install prompt; `button[data-install]` is revealed on `beforeinstallprompt` and everything
    `[data-install]` is hidden on `appinstalled`.
  - Translation keys (14): `install-app`, `install-title`, `install-welcome`, `install-intro`,
    `install-button`, `install-ios-title`, `install-ios-share`, `install-ios-add`,
    `install-ios-confirm`, `install-other`, `install-continue`, `offline-title`, `offline-text`,
    `offline-retry`.

- [ ] **Step 1: Write the failing tests**

The integration and contract tests below (the storage contract lives in `crates/storage-contract/src/` and runs on both backends). Unit tests inside source files are part of the Step 3 diff, in their `#[cfg(test)]` modules.

```diff
diff --git a/crates/web/tests/invites.rs b/crates/web/tests/invites.rs
index 8d69981..d84c77f 100644
--- a/crates/web/tests/invites.rs
+++ b/crates/web/tests/invites.rs
@@ -57,8 +57,10 @@ async fn an_invited_guest_scans_sets_a_password_and_is_signed_in() {
     let response = app
         .send(post_form(&link, &choose(CHOSEN, CHOSEN, "it"), None))
         .await;
-    assert_eq!(location(&response), Some("/"));
+    assert_eq!(location(&response), Some("/install?welcome"));
     let cookie = session_cookie(&response).expect("signed in");
+    let welcome = text(app.send(get_with("/install?welcome", &cookie)).await).await;
+    assert!(welcome.contains("Il tuo account è pronto."), "{welcome}");
     let home = text(app.send(get_with("/", &cookie)).await).await;
     assert!(home.contains("Accesso effettuato come friend."), "{home}");
 
diff --git a/crates/web/tests/pwa.rs b/crates/web/tests/pwa.rs
new file mode 100644
index 0000000..e631e31
--- /dev/null
+++ b/crates/web/tests/pwa.rs
@@ -0,0 +1,90 @@
+//! The installable app (spec §9.1): manifest, service worker, offline page and install page.
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use smoke_domain::types::{Locale, Role};
+
+#[tokio::test]
+async fn every_page_links_the_manifest_and_the_install_page() {
+    let app = TestApp::new().await;
+    for html in [
+        text(app.send(get("/login")).await).await,
+        text(app.send(get("/missing")).await).await,
+    ] {
+        assert!(
+            html.contains(r#"<link rel="manifest" href="/static/manifest.webmanifest">"#),
+            "{html}"
+        );
+        assert!(
+            html.contains(r#"<link rel="apple-touch-icon" href="/static/icon-180.png">"#),
+            "{html}"
+        );
+        assert!(
+            html.contains(r#"<a href="/install" class="install" data-install>Install app</a>"#),
+            "{html}"
+        );
+    }
+    let manifest = app.send(get("/static/manifest.webmanifest")).await;
+    assert_eq!(
+        manifest.headers()["content-type"],
+        "application/manifest+json"
+    );
+}
+
+#[tokio::test]
+async fn the_service_worker_is_served_at_the_root() {
+    let app = TestApp::new().await;
+    let response = app.send(get("/sw.js")).await;
+    assert_eq!(response.status(), StatusCode::OK);
+    assert_eq!(
+        response.headers()["content-type"],
+        "text/javascript; charset=utf-8"
+    );
+    assert_eq!(response.headers()["cache-control"], "no-cache");
+    let script = text(response).await;
+    assert!(script.contains("caches.match(\"/offline\")"), "{script}");
+}
+
+/// The service worker's install fails as a whole if one precached URL does not answer 200.
+#[tokio::test]
+async fn everything_the_service_worker_precaches_exists() {
+    let app = TestApp::new().await;
+    let script = text(app.send(get("/sw.js")).await).await;
+    let list = &script[script.find("PRECACHE = [").unwrap()..];
+    let list = &list[..list.find("];").unwrap()];
+    let paths: Vec<&str> = list.split('"').skip(1).step_by(2).collect();
+    assert!(paths.contains(&"/offline"), "{paths:?}");
+    for path in paths {
+        assert_eq!(app.send(get(path)).await.status(), StatusCode::OK, "{path}");
+    }
+}
+
+#[tokio::test]
+async fn the_offline_page_needs_no_session() {
+    let app = TestApp::new().await;
+    let html = text(app.send(get("/offline")).await).await;
+    assert!(html.contains("<h1>No connection</h1>"), "{html}");
+}
+
+#[tokio::test]
+async fn the_install_page_offers_the_button_and_the_iphone_steps() {
+    let app = TestApp::new().await;
+    let html = text(app.send(get("/install")).await).await;
+    assert!(
+        html.contains(r#"<button type="button" data-install hidden>Install</button>"#),
+        "{html}"
+    );
+    assert!(html.contains("Tap the Share button"), "{html}");
+    assert!(!html.contains("Your account is ready."), "{html}");
+
+    app.add_user("guest", Role::Guest, Locale::Tr).await;
+    let cookie = app.sign_in("guest").await;
+    let html = text(app.send(get_with("/install?welcome", &cookie)).await).await;
+    assert!(html.contains("Hesabınız hazır."), "{html}");
+    assert!(
+        html.contains(r#"<a href="/settings">"#),
+        "signed in: {html}"
+    );
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index eb2ece7..94d93e7 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -39,7 +39,14 @@ async fn every_response_carries_the_security_headers() {
 #[tokio::test]
 async fn anonymous_requests_set_no_cookies() {
     let app = TestApp::new().await;
-    for path in ["/missing", "/static/app.css", "/healthz"] {
+    for path in [
+        "/missing",
+        "/static/app.css",
+        "/healthz",
+        "/sw.js",
+        "/install",
+        "/offline",
+    ] {
         let response = app.send(get(path)).await;
         assert!(set_cookies(&response).is_empty(), "{path}");
     }
@@ -225,6 +232,8 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
                 .await,
         )
         .await,
+        text(app.send(get_with("/install?welcome", &cookie)).await).await,
+        text(app.send(get("/offline")).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace`
Expected: the new tests in `crates/web/tests/pwa.rs` fail (no manifest link; `/sw.js`,
`/static/manifest.webmanifest`, `/install` and `/offline` answer 404), and
`an_invited_guest_scans_sets_a_password_and_is_signed_in` fails (the setup still lands on `/`).
`anonymous_requests_set_no_cookies` and `no_page_needs_inline_scripts_styles_or_eval` still pass
on the 404 pages until Step 3.

- [ ] **Step 3: Implement**

```diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index cc7f827..c68385d 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -241,6 +241,21 @@ expiry-2-weeks = in 2 weeks
 expiry-date = at the end of this day:
 expiry-never = never
 
+install-app = Install app
+install-title = Add Smoke Tracker to your home screen
+install-welcome = Your account is ready.
+install-intro = It then opens like an app, straight on the Log screen, so logging takes a few seconds.
+install-button = Install
+install-ios-title = iPhone and iPad (Safari)
+install-ios-share = Tap the Share button
+install-ios-add = Choose "Add to Home Screen"
+install-ios-confirm = Tap "Add".
+install-other = On Android or a computer, if no Install button appears, open the browser menu and choose "Install app" or "Add to Home screen".
+install-continue = Continue to the app
+offline-title = No connection
+offline-text = Smoke Tracker needs the internet to save what you log. Try again when you are back online.
+offline-retry = Try again
+
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index cedda1e..0c377a1 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -241,6 +241,21 @@ expiry-2-weeks = tra 2 settimane
 expiry-date = alla fine di questo giorno:
 expiry-never = mai
 
+install-app = Installa l'app
+install-title = Aggiungi Smoke Tracker alla schermata Home
+install-welcome = Il tuo account è pronto.
+install-intro = Si aprirà come un'app, direttamente sulla schermata Registra: registrare richiede pochi secondi.
+install-button = Installa
+install-ios-title = iPhone e iPad (Safari)
+install-ios-share = Tocca il pulsante Condividi
+install-ios-add = Scegli "Aggiungi alla schermata Home"
+install-ios-confirm = Tocca "Aggiungi".
+install-other = Su Android o sul computer, se non compare il pulsante Installa, apri il menu del browser e scegli "Installa app" o "Aggiungi a schermata Home".
+install-continue = Vai all'app
+offline-title = Nessuna connessione
+offline-text = Smoke Tracker ha bisogno di internet per salvare ciò che registri. Riprova quando sei di nuovo online.
+offline-retry = Riprova
+
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index bd49234..8ce32c7 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -241,6 +241,21 @@ expiry-2-weeks = 2 hafta sonra
 expiry-date = bu günün sonunda:
 expiry-never = hiçbir zaman
 
+install-app = Uygulamayı yükle
+install-title = Smoke Tracker'ı ana ekranınıza ekleyin
+install-welcome = Hesabınız hazır.
+install-intro = Böylece doğrudan Kaydet ekranında bir uygulama gibi açılır ve kayıt birkaç saniye sürer.
+install-button = Yükle
+install-ios-title = iPhone ve iPad (Safari)
+install-ios-share = Paylaş düğmesine dokunun
+install-ios-add = "Ana Ekrana Ekle"yi seçin
+install-ios-confirm = "Ekle"ye dokunun.
+install-other = Android'de veya bilgisayarda Yükle düğmesi görünmezse tarayıcı menüsünü açıp "Uygulamayı yükle" veya "Ana ekrana ekle"yi seçin.
+install-continue = Uygulamaya devam et
+offline-title = Bağlantı yok
+offline-text = Smoke Tracker'ın kaydettiklerinizi saklamak için internete ihtiyacı var. Yeniden çevrimiçi olduğunuzda tekrar deneyin.
+offline-retry = Tekrar dene
+
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 3652f2c..57bc1b2 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -30,6 +30,12 @@ const ASSETS: &[Asset] = &[
     asset!("app.js", "text/javascript; charset=utf-8"),
     asset!("favicon.svg", "image/svg+xml"),
     asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
+    asset!("icon-180.png", "image/png"),
+    asset!("icon-192.png", "image/png"),
+    asset!("icon-512.png", "image/png"),
+    asset!("icon.svg", "image/svg+xml"),
+    asset!("manifest.webmanifest", "application/manifest+json"),
+    asset!("sw.js", "text/javascript; charset=utf-8"),
 ];
 
 /// A strong validator per asset: the first 16 hex characters of its SHA-256.
@@ -42,6 +48,15 @@ static ETAGS: LazyLock<Vec<String>> = LazyLock::new(|| {
 
 /// `Cache-Control: no-cache` makes browsers revalidate, which costs a `304` and no body.
 pub(crate) async fn serve(Path(path): Path<String>, headers: HeaderMap) -> Response {
+    respond(&path, &headers)
+}
+
+/// The service worker at the root, so that its scope is the whole app rather than `/static/`.
+pub(crate) async fn service_worker(headers: HeaderMap) -> Response {
+    respond("sw.js", &headers)
+}
+
+fn respond(path: &str, headers: &HeaderMap) -> Response {
     let Some(index) = ASSETS.iter().position(|asset| asset.path == path) else {
         return StatusCode::NOT_FOUND.into_response();
     };
@@ -81,6 +96,50 @@ mod tests {
         assert_eq!(on_disk, listed);
     }
 
+    fn asset(path: &str) -> &'static Asset {
+        ASSETS.iter().find(|a| a.path == path).unwrap()
+    }
+
+    /// Width and height from a PNG's header chunk.
+    fn png_size(bytes: &[u8]) -> (u32, u32) {
+        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
+        let number = |at: usize| u32::from_be_bytes(bytes[at..at + 4].try_into().unwrap());
+        (number(16), number(20))
+    }
+
+    #[test]
+    fn the_manifest_has_what_installing_needs() {
+        let manifest: serde_json::Value =
+            serde_json::from_slice(asset("manifest.webmanifest").body).unwrap();
+        assert_eq!(manifest["display"], "standalone");
+        assert_eq!(manifest["start_url"], "/");
+        assert_eq!(manifest["scope"], "/");
+        assert_eq!(manifest["shortcuts"][0]["name"], "New smell report");
+        assert_eq!(manifest["shortcuts"][0]["url"], "/");
+        let icons = manifest["icons"].as_array().unwrap();
+        for icon in icons {
+            let src = icon["src"].as_str().unwrap();
+            let file = asset(src.strip_prefix("/static/").unwrap());
+            if let Some(size) = icon["sizes"].as_str().unwrap().split_once('x') {
+                let (width, height) = png_size(file.body);
+                assert_eq!(
+                    (width.to_string(), height.to_string()),
+                    (size.0.to_owned(), size.1.to_owned()),
+                    "{src}"
+                );
+            }
+        }
+        for size in ["192x192", "512x512"] {
+            assert!(icons.iter().any(|i| i["sizes"] == size), "{size}");
+        }
+        assert!(icons.iter().any(|i| i["purpose"] == "maskable"));
+    }
+
+    #[test]
+    fn the_apple_touch_icon_is_180_pixels() {
+        assert_eq!(png_size(asset("icon-180.png").body), (180, 180));
+    }
+
     #[test]
     fn the_vendored_htmx_is_the_published_release() {
         let htmx = ASSETS.iter().find(|a| a.path.starts_with("htmx-")).unwrap();
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index b2ac938..11660bb 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -97,6 +97,8 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::accept::form::<S>).post(routes::accept::submit::<S>),
         )
         .route("/lang", post(routes::lang::choose::<S>))
+        .route("/install", get(routes::install::show::<S>))
+        .route("/offline", get(routes::install::offline))
         .fallback(routes::not_found)
         .layer(CatchPanicLayer::custom(error::panic_response))
         .layer(middleware::from_fn_with_state(
@@ -117,6 +119,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
 
     Router::new()
         .route("/static/{*path}", get(assets::serve))
+        .route("/sw.js", get(assets::service_worker))
         .route("/healthz", get(assets::healthz))
         .merge(pages)
         .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
diff --git a/crates/web/src/routes/accept.rs b/crates/web/src/routes/accept.rs
index da91fba..7809892 100644
--- a/crates/web/src/routes/accept.rs
+++ b/crates/web/src/routes/accept.rs
@@ -152,7 +152,10 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
         "/login"
     } else {
         auth::start_session(&session, &person, now).await?;
-        "/"
+        match invite.purpose {
+            InvitePurpose::Setup => "/install?welcome",
+            InvitePurpose::PasswordReset => "/",
+        }
     };
     let mut response = Redirect::to(target).into_response();
     response.headers_mut().append(
diff --git a/crates/web/src/routes/install.rs b/crates/web/src/routes/install.rs
new file mode 100644
index 0000000..7bdec28
--- /dev/null
+++ b/crates/web/src/routes/install.rs
@@ -0,0 +1,65 @@
+//! Installing the app (spec §8.3 step 4, §9.1): the page with the install button and the iPhone
+//! steps, and the page the service worker shows when there is no connection.
+
+use askama::Template;
+use axum::extract::{Query, State};
+use axum::http::StatusCode;
+use axum::response::Response;
+use serde::Deserialize;
+use smoke_domain::repos::Store;
+
+use crate::auth::CurrentUser;
+use crate::error::{AppError, page};
+use crate::header::SignedIn;
+use crate::i18n::Tr;
+use crate::state::AppState;
+
+#[derive(Template)]
+#[template(path = "install.html")]
+struct InstallPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    /// Shown right after an account was set up.
+    welcome: bool,
+}
+
+#[derive(Deserialize)]
+pub(crate) struct InstallQuery {
+    welcome: Option<String>,
+}
+
+/// Open to everyone: someone may want to install before signing in.
+pub(crate) async fn show<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    user: Option<CurrentUser>,
+    tr: Tr,
+    Query(query): Query<InstallQuery>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    page(
+        StatusCode::OK,
+        &InstallPage {
+            tr,
+            signed_in: user.and_then(|CurrentUser(user)| SignedIn::of(&user, now)),
+            welcome: query.welcome.is_some(),
+        },
+    )
+}
+
+#[derive(Template)]
+#[template(path = "offline.html")]
+struct OfflinePage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+}
+
+/// Cached by the service worker when it installs, in the language of that moment.
+pub(crate) async fn offline(tr: Tr) -> Result<Response, AppError> {
+    page(
+        StatusCode::OK,
+        &OfflinePage {
+            tr,
+            signed_in: None,
+        },
+    )
+}
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index f3d0625..0aa1ed3 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -4,6 +4,7 @@ pub(crate) mod accept;
 pub(crate) mod entries;
 pub(crate) mod events;
 pub(crate) mod guide;
+pub(crate) mod install;
 pub(crate) mod lang;
 pub(crate) mod lists;
 pub(crate) mod log;
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 51b34aa..75cc3da 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -345,3 +345,25 @@ button.current {
   overflow-wrap: anywhere;
   font-size: 0.75rem;
 }
+
+.install {
+  margin-left: auto;
+  margin-right: var(--gap);
+  font-size: 0.875rem;
+}
+
+@media (display-mode: standalone) {
+  .install {
+    display: none;
+  }
+}
+
+.steps li {
+  margin: 0.5rem 0;
+}
+
+.glyph {
+  width: 1.25em;
+  height: 1.25em;
+  vertical-align: text-bottom;
+}
diff --git a/crates/web/static/app.js b/crates/web/static/app.js
index a0178fe..8a4de27 100644
--- a/crates/web/static/app.js
+++ b/crates/web/static/app.js
@@ -1,7 +1,22 @@
-// Progressive enhancement, loaded by every page. Links with a data-dialog attribute open that
-// <dialog> instead of navigating (without JavaScript they open the full page), and tapping a
-// dialog's backdrop closes it. The close button is a method="dialog" form and needs no script.
+// Progressive enhancement, loaded by every page.
+//
+// - Links with a data-dialog attribute open that <dialog> instead of navigating (without
+//   JavaScript they open the full page), and tapping a dialog's backdrop closes it. The close
+//   button is a method="dialog" form and needs no script.
+// - The service worker is registered (spec §9.1).
+// - Where the browser offers its own install prompt (Chromium, Android), every data-install
+//   element opens it; a hidden install button is shown. Elsewhere the Install link leads to the
+//   page with the steps (spec §8.3).
+let installPrompt = null;
+
 document.addEventListener("click", (event) => {
+  const installer = event.target.closest("[data-install]");
+  if (installer && installPrompt) {
+    event.preventDefault();
+    installPrompt.prompt();
+    installPrompt = null;
+    return;
+  }
   const opener = event.target.closest("[data-dialog]");
   if (opener) {
     const dialog = document.getElementById(opener.dataset.dialog);
@@ -15,3 +30,22 @@ document.addEventListener("click", (event) => {
     event.target.close();
   }
 });
+
+window.addEventListener("beforeinstallprompt", (event) => {
+  event.preventDefault();
+  installPrompt = event;
+  for (const button of document.querySelectorAll("button[data-install]")) {
+    button.hidden = false;
+  }
+});
+
+window.addEventListener("appinstalled", () => {
+  installPrompt = null;
+  for (const element of document.querySelectorAll("[data-install]")) {
+    element.hidden = true;
+  }
+});
+
+if ("serviceWorker" in navigator) {
+  navigator.serviceWorker.register("/sw.js");
+}
diff --git a/crates/web/static/icon.svg b/crates/web/static/icon.svg
new file mode 100644
index 0000000..af77428
--- /dev/null
+++ b/crates/web/static/icon.svg
@@ -0,0 +1 @@
+<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" fill="#b4532a"/><path d="M10 24c0-6 6-6 6-12m4 12c0-5 4-5 4-10" stroke="#fff" stroke-width="2.5" fill="none" stroke-linecap="round"/></svg>
diff --git a/crates/web/static/manifest.webmanifest b/crates/web/static/manifest.webmanifest
new file mode 100644
index 0000000..42d2a64
--- /dev/null
+++ b/crates/web/static/manifest.webmanifest
@@ -0,0 +1,25 @@
+{
+  "id": "/",
+  "name": "Smoke Tracker",
+  "short_name": "Smoke Tracker",
+  "description": "Log cigarette smoke drifting in from the neighbours.",
+  "start_url": "/",
+  "scope": "/",
+  "display": "standalone",
+  "background_color": "#ffffff",
+  "theme_color": "#b4532a",
+  "icons": [
+    { "src": "/static/icon-192.png", "sizes": "192x192", "type": "image/png" },
+    { "src": "/static/icon-512.png", "sizes": "512x512", "type": "image/png" },
+    { "src": "/static/icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "maskable" },
+    { "src": "/static/icon.svg", "sizes": "any", "type": "image/svg+xml" }
+  ],
+  "shortcuts": [
+    {
+      "name": "New smell report",
+      "short_name": "New report",
+      "url": "/",
+      "icons": [{ "src": "/static/icon-192.png", "sizes": "192x192", "type": "image/png" }]
+    }
+  ]
+}
diff --git a/crates/web/static/sw.js b/crates/web/static/sw.js
new file mode 100644
index 0000000..12d71eb
--- /dev/null
+++ b/crates/web/static/sw.js
@@ -0,0 +1,54 @@
+// The service worker (spec §9.1), served at /sw.js so its scope is the whole app. It keeps the
+// static assets and an offline page. Pages and form posts always go to the network: there is
+// no offline logging. Assets are fetched first and the cached copy is the fallback, so a new
+// deploy is never hidden behind an old cache.
+const CACHE = "smoke-tracker-v1";
+const PRECACHE = [
+  "/offline",
+  "/static/app.css",
+  "/static/app.js",
+  "/static/htmx-2.0.11.min.js",
+  "/static/favicon.svg",
+  "/static/icon-192.png",
+];
+
+self.addEventListener("install", (event) => {
+  event.waitUntil(
+    caches
+      .open(CACHE)
+      .then((cache) => cache.addAll(PRECACHE))
+      .then(() => self.skipWaiting()),
+  );
+});
+
+self.addEventListener("activate", (event) => {
+  event.waitUntil(
+    caches
+      .keys()
+      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
+      .then(() => self.clients.claim()),
+  );
+});
+
+self.addEventListener("fetch", (event) => {
+  const request = event.request;
+  const url = new URL(request.url);
+  if (request.method !== "GET" || url.origin !== self.location.origin) {
+    return;
+  }
+  if (request.mode === "navigate") {
+    event.respondWith(fetch(request).catch(() => caches.match("/offline")));
+  } else if (url.pathname.startsWith("/static/")) {
+    event.respondWith(
+      fetch(request)
+        .then((response) => {
+          if (response.ok) {
+            const copy = response.clone();
+            event.waitUntil(caches.open(CACHE).then((cache) => cache.put(request, copy)));
+          }
+          return response;
+        })
+        .catch(() => caches.match(request)),
+    );
+  }
+});
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index 733b51c..af0dbc7 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -5,7 +5,10 @@
   <meta name="viewport" content="width=device-width, initial-scale=1">
   <meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>
   <title>{% block title %}{% endblock %} · {{ tr.t("app-name") }}</title>
+  <meta name="theme-color" content="#b4532a">
   <link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
+  <link rel="apple-touch-icon" href="/static/icon-180.png">
+  <link rel="manifest" href="/static/manifest.webmanifest">
   <link rel="stylesheet" href="/static/app.css">
   <script src="/static/htmx-2.0.11.min.js" defer></script>
   <script src="/static/app.js" defer></script>
@@ -13,6 +16,7 @@
 <body>
   <header class="bar">
     <span class="brand">{{ tr.t("app-name") }}</span>
+    <a href="/install" class="install" data-install>{{ tr.t("install-app") }}</a>
     {% if let Some(me) = signed_in %}
     <nav class="menu">
       <a href="/">{{ tr.t("nav-log") }}</a>
diff --git a/crates/web/templates/install.html b/crates/web/templates/install.html
new file mode 100644
index 0000000..7330d00
--- /dev/null
+++ b/crates/web/templates/install.html
@@ -0,0 +1,18 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("install-title") }}{% endblock %}
+{% block content %}
+{% if welcome %}
+<p class="notice" role="status">{{ tr.t("install-welcome") }}</p>
+{% endif %}
+<h1>{{ tr.t("install-title") }}</h1>
+<p>{{ tr.t("install-intro") }}</p>
+<p><button type="button" data-install hidden>{{ tr.t("install-button") }}</button></p>
+<h2>{{ tr.t("install-ios-title") }}</h2>
+<ol class="steps">
+  <li>{{ tr.t("install-ios-share") }} <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15V3m-4 4 4-4 4 4M6 10H5v11h14V10h-1" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg></li>
+  <li>{{ tr.t("install-ios-add") }} <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 8v8m-4-4h8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg></li>
+  <li>{{ tr.t("install-ios-confirm") }}</li>
+</ol>
+<p class="hint">{{ tr.t("install-other") }}</p>
+<p><a href="/">{{ tr.t("install-continue") }}</a></p>
+{% endblock %}
diff --git a/crates/web/templates/offline.html b/crates/web/templates/offline.html
new file mode 100644
index 0000000..d56e430
--- /dev/null
+++ b/crates/web/templates/offline.html
@@ -0,0 +1,7 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("offline-title") }}{% endblock %}
+{% block content %}
+<h1>{{ tr.t("offline-title") }}</h1>
+<p>{{ tr.t("offline-text") }}</p>
+<p><a href="/">{{ tr.t("offline-retry") }}</a></p>
+{% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index b8feb69..770404c 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -127,3 +127,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 118 | 2026-10-14 | UI | **People** lists every account (role, end of access, "waiting for setup") for residents only; others get **403** and no menu link (the header's `SignedIn` knows whether to show it). **End access now** expires a guest or viewer at once; an expired account shows a **Reactivate** form with the guest quick picks or, for a viewer, "never" or a day | Hide the page with 404; a separate page per person | Spec §9 screen 6 on one screen; a refused role sees the same "Not allowed" page as elsewhere (#101) |
 | 119 | 2026-10-14 | Invites | A link's token is **32 random bytes in hex**; the QR code is **inline SVG drawn with `qrcodegen` 1.8** (MIT, no dependencies, Project Nayuki's reference encoder) in the response to the post that made it, never stored; the page also shows the link as **plain text, not clickable**; paths under `/invite/` are **cut from the logs**; a dead link answers **410** with its own page | A `data:` PNG (blocked by the CSP); the `qrcode` crate (pulls in `image` by default); a stored token to show the QR again; a clickable link | No image library, no CSP exception, a few kB more binary; a resident who taps the link would sign their own phone in as the invitee; Cloud Run keeps request logs, and an unused token is a password |
 | 120 | 2026-10-14 | Invites | Opening a link asks for the **password twice and a language** (preselected from the page's language); saving uses up the link, ends the account's sessions and **signs this device in**, unless the account may not sign in (an expired viewer goes to the sign-in page); a new account starts in the **inviter's language** until then | Ask for the username too; email confirmation | Spec §8.3 step 3; the username is fixed by the resident and shown on the page |
+| 121 | 2026-10-14 | PWA | The **manifest** (`/static/manifest.webmanifest`, scope and start URL `/`, a "New smell report" shortcut to the Log screen) lists PNG icons of 192 and 512 px (the 512 also as maskable) and an SVG, all drawn from `static/icon.svg` with `rsvg-convert`; the **service worker** is served at `/sw.js` and goes to the network first: static assets fall back to their cached copy and pages to a cached **`/offline`** page; posts are never touched. The header's **Install app** link opens the browser's prompt where there is one and otherwise the **`/install`** page (button plus iPhone steps); CSS hides it in standalone mode; a first setup lands on `/install?welcome` | Cache-first assets; an install banner; a separate offline logging queue | Spec §9.1 and §8.3 step 4 with no inline script; a deploy is never hidden behind an old cache (assets are already revalidated with ETags, #65); offline logging is out of scope for v1 (§14) |
diff --git a/docs/plans/ROADMAP.md b/docs/plans/ROADMAP.md
index 757e26d..642990d 100644
--- a/docs/plans/ROADMAP.md
+++ b/docs/plans/ROADMAP.md
@@ -11,7 +11,7 @@ guesses.
 | 2 | **Storage** | Repository traits in domain; storage-sqlite and storage-postgres crates with migrations; an audit entry inside every write transaction; one storage-contract test suite run on both backends; schema-parity check; verify-audit logic | §6, §6.1, §10.1, §13.3 | `02-storage.md` ✅ |
 | 3 | **Web foundation & auth** | Axum server, config, CLI (`serve`, `migrate`, `create-resident`, `verify-audit`), sessions, login + rate limit, security headers, Origin check, i18n (en/it/tr), base layout, embedded assets, Dockerfile, release workflow, Trivy | §8.1, §8.2, §8.4, §9.2, §10.3–10.5, §12, §13.1 | `03-web-and-auth.md` ✅ |
 | 4 | **Logging flow** | Log screen, every prompt (same smell / which event / auto-close notice), close/reopen, PM2.5, events list and detail, report detail (correct, move, history), large-gap warning, auto-close sweep, summary card, "How to rate" popup | §5, §7, §9 (1–4), §11 | `04-logging-flow.md`, `04b-room-levels.md` |
-| 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | — |
+| 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | `05-people-and-pwa.md` |
 | 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | — |
 | 7 | **Neon readiness** | ~~`migrate-to-postgres` command and its CI test~~ Not needed: production starts on Postgres (decision #77) | §10.2 | — |
 | 8 | **Learning pack** | `docs/learning/` notes per milestone, optional exercises with tests, walkthroughs | Decision #43 | — |
```

Then draw the PNG icons from `crates/web/static/icon.svg` (created by the diff above;
`rsvg-convert` comes with Homebrew's `librsvg`):

```sh
cd crates/web/static
for n in 180 192 512; do rsvg-convert -w $n -h $n icon.svg -o icon-$n.png; done
cd -
```

The tests check only each PNG's signature and size, so another `rsvg-convert` version is fine
(the prototype's files, 3–10 kB each, can also be copied: `git checkout m5-proto --
crates/web/static/icon-180.png crates/web/static/icon-192.png crates/web/static/icon-512.png`).

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes, including the five tests of `crates/web/tests/pwa.rs`,
`the_manifest_has_what_installing_needs`, `the_apple_touch_icon_is_180_pixels`,
`every_file_in_the_static_folder_is_served` and `anonymous_requests_set_no_cookies` (now also
`/sw.js`, `/install`, `/offline`). With `TEST_POSTGRES_URL` set: **562** tests. No test runs a
browser (spec §13.3): list for the user, in the pull request description, "after the deploy,
Chromium DevTools → Application shows the manifest as installable and the service worker as
activated; offline, a reload shows the offline page; Install app works on Android and the
iPhone steps on Safari".

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/assets.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/accept.rs \
  crates/web/src/routes/install.rs \
  crates/web/src/routes/mod.rs \
  crates/web/static/app.css \
  crates/web/static/app.js \
  crates/web/static/icon-180.png \
  crates/web/static/icon-192.png \
  crates/web/static/icon-512.png \
  crates/web/static/icon.svg \
  crates/web/static/manifest.webmanifest \
  crates/web/static/sw.js \
  crates/web/templates/base.html \
  crates/web/templates/install.html \
  crates/web/templates/offline.html \
  crates/web/tests/invites.rs \
  crates/web/tests/pwa.rs \
  crates/web/tests/security.rs \
  docs/DECISIONS.md \
  docs/plans/ROADMAP.md
git commit -m "feat(pwa): manifest, service worker, offline page and install prompt

The app becomes installable: a web manifest with PNG and SVG icons and a \"New smell report\"
shortcut, a network-first service worker at /sw.js that keeps the static assets and an
offline page, and an Install app link that opens the browser's prompt or the /install page
with the iPhone steps. A first account setup lands on that page.

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

Trying the flow by hand: create a resident with `cargo run -- create-resident daniele`, serve
with `APP_BASE_URL=http://localhost:8080`, invite someone from People and open the shown link in
a private window (on `http://localhost` the session cookie is `session`, decision #62).

After the merge: nothing to migrate. The second resident can open Settings → Password to choose
her own password, or a resident can make her a *Reset password* QR code from People.

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §4 | Invite / expire / reactivate / reset password: residents only | 3, 6, 7 |
| §4 | Edit room & symptom lists: residents only | 5 |
| §4 | Guests always expire, viewers optionally, residents never; reactivation with a new expiry keeps the old password | 3, 6 |
| §4 | On expiry/deactivation all of the user's sessions are deleted | 6 (`expire_user`, #58) |
| §6 | `invites`: purpose setup / password_reset, SHA-256 token hash, plaintext never stored, expires 5 min after creation, `used_at` | 2 |
| §8.3 1 | Username, role and expiry: guest quick picks 1 day / weekend / 1 week (default) / 2 weeks / custom; viewer optional; resident none | 6, 7 |
| §8.3 2 | QR code for `{APP_BASE_URL}/invite/{token}`, valid 5 minutes, works once | 2, 3, 7 |
| §8.3 3 | The invitee sets a password and language and is logged in | 3, 7 |
| §8.3 4 | After the first login: "Add Smoke Tracker to your home screen" — native button on Chromium/Android, illustrated steps on iOS Safari | 8 |
| §8.3 | Password reset by the same mechanism, QR made by a resident, no email | 3, 7 |
| §8.4 | Sessions deleted on password change (and not revived by a late renewal) | 1, 4 |
| §9 screen 6 | People: users with role and expiry; Invite, Expire, Reactivate, Reset password (QR) | 6, 7 |
| §9 screen 7 | Settings: language, password change, room and symptom lists (residents), Guides | 4, 5 (How to log: milestone 6) |
| §9 screen 8 | Header: app name, Install app (hidden in standalone mode), menu | 6, 8 |
| §9.1 | Manifest: name, icons, `display: standalone`, `start_url` = Log screen, shortcut "New smell report" | 8 |
| §9.1 | A minimal service worker caches static assets and an offline page; no offline logging | 8 |
| §9.2 | Every new text in en/it/tr; it/tr drafts for native review | 4–8 |
| §11 | Expired invites removed daily | 2 |
| §12 | CSP `'self'` only; hashed invite tokens | 4–8, 2 |
| §13.3 | Web tests: 5-minute invite expiry; guests and viewers cannot manage people | 3, 6, 7 |
