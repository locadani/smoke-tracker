# Milestone 3: Web foundation & auth — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A running, deployable Smoke Tracker server: Axum web layer with database-backed
sessions, login with rate limiting, security headers and a cross-site request check, translated
pages (en/it/tr), embedded assets, and the `smoke-tracker` binary with the `serve`, `migrate`,
`create-resident` and `verify-audit` commands, shipped as a small distroless image to GHCR and
scanned by Trivy.

**Architecture:** Two new crates on top of milestone 2. `smoke-web` (library) holds the routes,
middleware, Askama templates, Fluent translations and static files; it reaches the database only
through the `smoke-domain` traits, so it compiles once per backend and never sees SQL.
`smoke-tracker` (package; library `smoke_app` + a thin `main.rs`) reads the environment, picks the
backend from `DATABASE_URL` and runs a command generic over `Store`. Sessions use
`tower-sessions` for the cookie and our own `SessionStore` over a new domain `SessionRepo`, stored
in a `sessions` table in both backends. The request pipeline, outermost first: set request id →
trace span → propagate request id → security headers → body limit → (pages only) translated
error pages → Origin check → session cookie → load the signed-in user → catch panics → route.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, tower 0.5, tower-http 0.7,
tower-sessions 0.15, askama 0.16, fluent-templates 0.15, argon2 0.6, clap 4, rpassword 7,
tracing + tracing-subscriber, anyhow, getrandom 0.4, htmx 2.0.11 (vendored), Docker (musl +
distroless), GitHub Actions (GHCR, Trivy).

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` (§8.1, §8.2, §8.4, §9.2, §10.3–10.5,
§12, §13.1). Decisions: `docs/DECISIONS.md` (rows #56–#73 are added by Task 12).
Previous plan: `docs/plans/02-storage.md`.

**Provenance:** every code block in this plan was compiled and tested before the plan was
written: replayed onto `main` at `b293906`, `cargo fmt --check`, `cargo clippy --workspace
--all-targets -D warnings`, `cargo test --workspace` (SQLite and Postgres), `cargo deny check`,
`scripts/sqlx-prepare.sh --check`, a release build, and a live run of the binary all pass.

## Open questions for the user

Defaults were chosen so nothing blocks; each is a decision row and easy to change.

1. **Per-address limit:** 20 failed sign-ins per address in 15 minutes block that address for
   15 minutes (decision #60). The spec only says "coarse".
2. **Trivy as a required check** (Task 11) builds the Docker image on every PR, adding a few
   minutes to CI. The alternative is running it only on `main` and weekly.
3. **Snyk** is not added: Trivy (image + embedded crates) and cargo-deny (RustSec) cover the same
   ground for free with no token (decision #70). Snyk can still be added later.
4. **Spec wording** (Task 13) needs your explicit approval before it is applied.

## Global Constraints

- Personal project. Before every commit, `git config user.email` must print
  `61902944+locadani@users.noreply.github.com` and `git config user.name` must print `locadani`.
  Never put any email address in a tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings anywhere: local and CI Postgres use trust
  authentication. `.env` is git-ignored; only `.env.example` is committed, and it holds no secret.
- Never modify `description.md` or `claude.md`. Never `git add` them.
- **Never edit `docs/specs/2026-10-13-smoke-tracker-design.md` without the user's explicit
  approval.** Task 13 holds every spec edit and must not start until the user has approved it.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one, e.g.
  `feat(web): a tower-sessions SessionStore over the domain SessionRepo`.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI, `cargo fmt`
  enforced. Every new crate has `[lints] workspace = true`.
- `smoke-web` never depends on a storage crate except as a **dev-dependency** (its tests use
  SQLite). Only `smoke-tracker` (the app crate) depends on both storage crates.
- Free tier only and a small footprint: static musl binary on a distroless base, roughly
  15–20 MB. No CDN: every script, style and icon is served by the app itself.
- Content-Security-Policy is `'self'` only. No template may contain inline `<script>` code,
  `<style>`, `style=` attributes, `on*=` handlers, `hx-on` or `javascript:` URLs.
- Every "now" goes through `smoke_domain::clock::Clock`. (Exception: `tower-sessions` stamps the
  cookie expiry with the real time; the web tests start the fake clock at the real time.)
- Argon2 hashing and verification are CPU-heavy: in async code they run in
  `tokio::task::spawn_blocking`.
- Integration tests drive the router with `tower::ServiceExt::oneshot`; no browser tests in this
  milestone.
- Postgres tests need `TEST_POSTGRES_URL`; locally the container `smoke-pg` listens on
  **port 55432** (`postgres://postgres@localhost:55432/postgres`) because 5432 is used by an
  unrelated container. CI uses its own service container on 5432.
- After changing any `query!` text or a migration, run `scripts/sqlx-prepare.sh` (with
  `TEST_POSTGRES_URL` set) and commit the regenerated `.sqlx/` files.
- `Cargo.lock` is committed. Whenever a task adds a dependency, commit the `Cargo.lock` that
  `cargo build` produces in the same commit.

## Review Focus

1. **"Daniele" vs "daniele " at the login form, to dodge the lockout.** Expected: the failure
   counter is keyed on the trimmed, lowercased name, so every spelling shares one lock. Test:
   `changing_the_case_of_the_username_does_not_bypass_the_lock` (Task 7).
2. **Probing which accounts exist** by message or timing. Expected: unknown user, invalid name and
   wrong password give the same status and message, and each costs one Argon2 verification.
   Tests: `unknown_users_and_wrong_passwords_get_the_same_answer` (Task 6),
   `the_dummy_hash_costs_the_same_as_a_real_one` (Task 3).
3. **Running behind a reverse proxy** (TLS ends at the proxy; the app sees plain http and an
   internal `Host`). Expected: cookie flags, HSTS and the Origin check follow `APP_BASE_URL`, not
   the request, and the per-address limit uses the configured client-address header. Tests:
   `the_origin_check_uses_the_configured_url_not_the_host_header` (Task 6),
   `behind_a_proxy_the_configured_header_names_the_client` (Task 7).
4. **The strict CSP silently breaking htmx** (htmx injects an inline indicator style and can
   `eval` by default). Expected: htmx is configured not to, and no page has inline code. Test:
   `no_page_needs_inline_scripts_styles_or_eval` (Task 6).
5. **An unknown or missing language** (`lang=fr`, `Accept-Language: de`, a translation key
   missing in one language). Expected: fall through to the next source, then English; every
   language has exactly the English keys. Tests:
   `unusable_values_fall_through_to_the_next_source` and
   `every_language_has_exactly_the_english_keys` (Task 5).

---

## File Structure

```
smoke-tracker/
├─ Cargo.toml                         + web/app dependencies, release profile, argon2 opt-level
├─ Dockerfile, .dockerignore          NEW  musl build with cargo-auditable → distroless nonroot
├─ .env.example                       NEW  every variable of spec §10.5 (+ CLIENT_IP_HEADER)
├─ README.md                          + Running it, Container
├─ .github/
│  ├─ workflows/release.yml           NEW  build and push ghcr.io/locadani/smoke-tracker
│  ├─ workflows/security.yml          + trivy job
│  ├─ dependabot.yml                  + docker ecosystem
│  └─ rulesets/protect-main.json      + required check "trivy"
├─ docs/
│  ├─ DECISIONS.md                    + rows #56–#73
│  └─ plans/ROADMAP.md                + M3 plan link
└─ crates/
   ├─ domain/src/
   │  ├─ types.rs                     + NewPassword (10–128 chars, redacted Debug)
   │  ├─ error.rs                     + InvalidPasswordLength
   │  ├─ records.rs                   + StoredSession
   │  └─ repos.rs                     + SessionRepo, part of Store
   ├─ storage-sqlite/, storage-postgres/
   │  ├─ migrations/0002_sessions.sql NEW  (identical columns in both)
   │  ├─ src/sessions.rs              NEW  SessionRepo + delete_for_user
   │  ├─ src/users.rs                 set_password / expire_user end the user's sessions
   │  └─ .sqlx/                       regenerated
   ├─ storage-contract/src/sessions.rs NEW  10 contract tests
   ├─ web/                            NEW  package smoke-web
   │  ├─ src/lib.rs                   router::<S>() and serve()
   │  ├─ src/config.rs                BaseUrl, WebConfig
   │  ├─ src/state.rs                 AppState<S>
   │  ├─ src/sessions.rs              DbSessions<S>: tower-sessions SessionStore over SessionRepo
   │  ├─ src/security.rs              security headers, Origin / Sec-Fetch-Site check
   │  ├─ src/request_id.rs            random 16-hex request ids
   │  ├─ src/assets.rs                embedded static files with ETags, /healthz
   │  ├─ src/error.rs                 AppError, translated error pages, panic handler
   │  ├─ src/i18n.rs                  Fluent loader, Tr, language negotiation
   │  ├─ src/cookies.rs               cookie reading, the lang cookie
   │  ├─ src/auth/{mod,password,limiter,client_ip}.rs
   │  ├─ src/routes/{mod,home,login,lang}.rs
   │  ├─ templates/{base,error,login,home}.html
   │  ├─ locales/{en,it,tr}/main.ftl
   │  ├─ static/{app.css,favicon.svg,htmx-2.0.11.min.js}
   │  └─ tests/{common/mod.rs,security.rs,errors.rs,login.rs,i18n.rs}
   └─ app/                            NEW  package smoke-tracker (lib smoke_app, bin smoke-tracker)
      ├─ src/{lib,main,cli,config,backend}.rs
      ├─ src/commands/{mod,serve,create_resident,verify_audit}.rs
      └─ tests/commands.rs
```

**Dependency direction:** `smoke-tracker → smoke-web → smoke-domain ← storage-*`, and
`smoke-tracker → storage-sqlite, storage-postgres`. `smoke-web`'s tests use `storage-sqlite` as a
dev-dependency.

**Why our own session store:** the spec wants tower-sessions with the backend matching
`DATABASE_URL`. The published `tower-sessions-sqlx-store` 0.15.0 depends on `sqlx ^0.8` and
`tower-sessions ^0.14` (checked on crates.io), so it would pull a second sqlx next to our 0.9 and
an older tower-sessions. Implementing `tower_sessions::SessionStore` over a domain `SessionRepo`
is about 100 lines, keeps one sqlx, stores only hashed session ids, and lets a password change
end the user's sessions inside the same audited transaction (decision #56).

---

### Task 1: Domain — `NewPassword`

A password someone is choosing: 10–128 characters counted as characters (not bytes), no
composition rules, kept exactly as typed, never printed (spec §8.1).

**Files:**
- Modify: `crates/domain/src/error.rs`
- Modify: `crates/domain/src/types.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `smoke_domain::types::NewPassword` — `NewPassword::new(String) -> Result<Self, DomainError>`,
    `expose(&self) -> &str`, `MIN_CHARS = 10`, `MAX_CHARS = 128`; `Debug` prints
    `NewPassword(<redacted>)`; no `Display`, no serde.
  - `DomainError::InvalidPasswordLength(usize)` (the character count).

- [ ] **Step 1: Write the failing tests**

In `crates/domain/src/types.rs`, add to the `tests` module:

```diff
--- a/crates/domain/src/types.rs
+++ b/crates/domain/src/types.rs
@@ -388,6 +417,27 @@ mod tests {
         assert!(Username::new("abc").is_ok());
     }
 
+    #[test]
+    fn passwords_need_ten_to_128_characters_counted_as_characters() {
+        assert!(NewPassword::new("x".repeat(10)).is_ok());
+        assert!(NewPassword::new("ş".repeat(128)).is_ok());
+        assert_eq!(
+            NewPassword::new("x".repeat(9)),
+            Err(DomainError::InvalidPasswordLength(9))
+        );
+        assert_eq!(
+            NewPassword::new("x".repeat(129)),
+            Err(DomainError::InvalidPasswordLength(129))
+        );
+    }
+
+    #[test]
+    fn passwords_are_kept_as_typed_and_never_printed() {
+        let password = NewPassword::new("  spaced out  ".to_owned()).unwrap();
+        assert_eq!(password.expose(), "  spaced out  ");
+        assert_eq!(format!("{password:?}"), "NewPassword(<redacted>)");
+    }
+
     #[test]
     fn list_names_are_trimmed_and_bounded() {
         assert_eq!(ListName::new("  Balcony ").unwrap().as_str(), "Balcony");
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain passwords_`
Expected: compile error — `NewPassword` is not defined.

- [ ] **Step 3: Implement**

```diff
--- a/crates/domain/src/error.rs
+++ b/crates/domain/src/error.rs
@@ -41,4 +41,7 @@ pub enum DomainError {
 
     #[error("a note can be at most 2000 characters, got {0}")]
     NoteTooLong(usize),
+
+    #[error("a password needs 10–128 characters, got {0}")]
+    InvalidPasswordLength(usize),
 }
```

```diff
--- a/crates/domain/src/types.rs
+++ b/crates/domain/src/types.rs
@@ -195,6 +195,35 @@ impl fmt::Display for ListName {
     }
 }
 
+/// A password someone is choosing: 10–128 characters, no composition rules (spec §8.1). Kept
+/// exactly as typed, never trimmed, and never printed.
+#[derive(Clone, PartialEq, Eq)]
+pub struct NewPassword(String);
+
+impl NewPassword {
+    pub const MIN_CHARS: usize = 10;
+    pub const MAX_CHARS: usize = 128;
+
+    pub fn new(raw: String) -> Result<Self, DomainError> {
+        let chars = raw.chars().count();
+        if (Self::MIN_CHARS..=Self::MAX_CHARS).contains(&chars) {
+            Ok(Self(raw))
+        } else {
+            Err(DomainError::InvalidPasswordLength(chars))
+        }
+    }
+
+    pub fn expose(&self) -> &str {
+        &self.0
+    }
+}
+
+impl fmt::Debug for NewPassword {
+    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
+        f.write_str("NewPassword(<redacted>)")
+    }
+}
+
 /// How strong a smell is, on the personal 1–10 scale.
 #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
 #[serde(try_from = "u8", into = "u8")]
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all pass, including `passwords_need_ten_to_128_characters_counted_as_characters` and
`passwords_are_kept_as_typed_and_never_printed`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/error.rs crates/domain/src/types.rs
git commit -m "feat(domain): NewPassword newtype with a redacted Debug impl

Passwords are 10–128 characters counted with chars(), never trimmed, and a
hand-written Debug keeps them out of logs and panic messages.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Storage — sessions on both backends

A `sessions` table in both migrations (identical columns), a domain `SessionRepo` trait that is
part of `Store`, and its two implementations. Sessions are bookkeeping, not evidence: they are
never audited. `set_password` and `expire_user` delete the user's sessions **inside their
transaction**, so a rolled-back password change keeps them (spec §8.4).

**Files:**
- Modify: `crates/domain/src/records.rs`, `crates/domain/src/repos.rs`
- Create: `crates/storage-sqlite/migrations/0002_sessions.sql`,
  `crates/storage-postgres/migrations/0002_sessions.sql`
- Create: `crates/storage-sqlite/src/sessions.rs`, `crates/storage-postgres/src/sessions.rs`
- Modify: `crates/storage-sqlite/src/{lib,users}.rs`, `crates/storage-postgres/src/{lib,users}.rs`
- Create: `crates/storage-contract/src/sessions.rs`; Modify: `crates/storage-contract/src/lib.rs`
- Modify: `crates/storage-sqlite/tests/contract.rs`, `crates/storage-postgres/tests/schema_parity.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`, `crates/storage-postgres/.sqlx/`

**Interfaces:**
- Consumes: `fixture::{add_user, start}` and `Fixture<S>` from `storage-contract` (milestone 2);
  each backend's `convert::{instant, micros}` and `error::IntoStore`.
- Produces:
  - `smoke_domain::records::StoredSession { id_hash: String, user_id: Option<UserId>, data:
    String, expires_at: DateTime<Utc> }`.
  - `smoke_domain::repos::SessionRepo: Send + Sync` with
    `session(&self, id_hash: &str, now) -> StoreResult<Option<StoredSession>>` (none once
    `expires_at <= now`), `create_session(&self, &StoredSession) -> StoreResult<()>`
    (`Conflict::Duplicate` on a taken id, `Conflict::UnknownReference` for an unknown user),
    `save_session(&self, &StoredSession)` (upsert), `delete_session(&self, id_hash)`,
    `delete_expired_sessions(&self, now) -> StoreResult<u64>`. `Store` now requires it.

- [ ] **Step 1: Write the failing contract tests**

Create `crates/storage-contract/src/sessions.rs`:

```rust
//! Login sessions: stored by hash, unaudited, and ended with a password change or expiry.

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::clock::Clock;
use smoke_domain::records::{PasswordHash, StoredSession};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, SessionRepo, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{Role, UserId};

use crate::Fixture;
use crate::fixture::{add_user, start};

fn session(id_hash: &str, user_id: Option<UserId>, expires_at: DateTime<Utc>) -> StoredSession {
    StoredSession {
        id_hash: id_hash.to_owned(),
        user_id,
        data: format!(r#"{{"user_id":{}}}"#, user_id.map_or(0, |u| u.0)),
        expires_at,
    }
}

fn in_30_days() -> DateTime<Utc> {
    start() + TimeDelta::days(30)
}

pub async fn a_session_round_trips_until_it_expires<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    let stored = session("hash-a", Some(user.id), in_30_days());
    fx.store.create_session(&stored).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(stored)
    );
    assert_eq!(
        fx.store.session("hash-a", in_30_days()).await.unwrap(),
        None
    );
    assert_eq!(fx.store.session("unknown", start()).await.unwrap(), None);
}

pub async fn creating_a_session_twice_is_a_duplicate<S: SessionRepo>(fx: &Fixture<S>) {
    fx.store
        .create_session(&session("hash-a", None, in_30_days()))
        .await
        .unwrap();
    let error = fx
        .store
        .create_session(&session("hash-a", None, in_30_days()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::Duplicate)),
        "{error:?}"
    );
}

pub async fn a_session_for_an_unknown_user_is_refused<S: SessionRepo>(fx: &Fixture<S>) {
    let error = fx
        .store
        .create_session(&session("hash-a", Some(UserId(999)), in_30_days()))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
}

pub async fn saving_a_session_inserts_or_replaces_it<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    let anonymous = session("hash-a", None, start() + TimeDelta::days(1));
    fx.store.save_session(&anonymous).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(anonymous)
    );
    let signed_in = session("hash-a", Some(user.id), in_30_days());
    fx.store.save_session(&signed_in).await.unwrap();
    assert_eq!(
        fx.store.session("hash-a", start()).await.unwrap(),
        Some(signed_in)
    );
}

pub async fn deleting_a_session_removes_only_that_one<S: SessionRepo>(fx: &Fixture<S>) {
    for id_hash in ["hash-a", "hash-b"] {
        fx.store
            .create_session(&session(id_hash, None, in_30_days()))
            .await
            .unwrap();
    }
    fx.store.delete_session("hash-a").await.unwrap();
    fx.store.delete_session("never-existed").await.unwrap();
    assert_eq!(fx.store.session("hash-a", start()).await.unwrap(), None);
    assert!(fx.store.session("hash-b", start()).await.unwrap().is_some());
}

pub async fn expired_sessions_are_swept<S: SessionRepo>(fx: &Fixture<S>) {
    let now = fx.clock.now();
    fx.store
        .create_session(&session("old", None, now - TimeDelta::seconds(1)))
        .await
        .unwrap();
    fx.store
        .create_session(&session("ends-now", None, now))
        .await
        .unwrap();
    fx.store
        .create_session(&session("fresh", None, now + TimeDelta::seconds(1)))
        .await
        .unwrap();
    assert_eq!(fx.store.delete_expired_sessions(now).await.unwrap(), 2);
    assert!(fx.store.session("fresh", now).await.unwrap().is_some());
    assert_eq!(fx.store.delete_expired_sessions(now).await.unwrap(), 0);
}

pub async fn a_password_change_ends_the_users_sessions<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let daniele = add_user(fx, "daniele", Role::Resident).await;
    let other = add_user(fx, "partner", Role::Resident).await;
    for (id_hash, user) in [
        ("d1", Some(daniele.id)),
        ("d2", Some(daniele.id)),
        ("p1", Some(other.id)),
        ("anon", None),
    ] {
        fx.store
            .create_session(&session(id_hash, user, in_30_days()))
            .await
            .unwrap();
    }
    let mut tx = fx.store.begin(Some(daniele.id)).await.unwrap();
    tx.set_password(daniele.id, PasswordHash::new("new-hash".to_owned()))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    for (id_hash, survives) in [("d1", false), ("d2", false), ("p1", true), ("anon", true)] {
        assert_eq!(
            fx.store.session(id_hash, start()).await.unwrap().is_some(),
            survives,
            "{id_hash}"
        );
    }
}

pub async fn expiring_a_user_ends_their_sessions<S: SessionRepo + Transactional<Tx: UserWrites>>(
    fx: &Fixture<S>,
) {
    let guest = add_user(fx, "guest", Role::Guest).await;
    fx.store
        .create_session(&session("g1", Some(guest.id), in_30_days()))
        .await
        .unwrap();
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.expire_user(guest.id).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(fx.store.session("g1", start()).await.unwrap(), None);
}

pub async fn a_rolled_back_password_change_keeps_the_sessions<
    S: SessionRepo + Transactional<Tx: UserWrites>,
>(
    fx: &Fixture<S>,
) {
    let user = add_user(fx, "daniele", Role::Resident).await;
    fx.store
        .create_session(&session("d1", Some(user.id), in_30_days()))
        .await
        .unwrap();
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, PasswordHash::new("new-hash".to_owned()))
        .await
        .unwrap();
    drop(tx);
    assert!(fx.store.session("d1", start()).await.unwrap().is_some());
}

pub async fn sessions_are_not_audited<S: SessionRepo + AuditRepo>(fx: &Fixture<S>) {
    let stored = session("hash-a", None, in_30_days());
    fx.store.create_session(&stored).await.unwrap();
    fx.store.save_session(&stored).await.unwrap();
    fx.store.delete_session("hash-a").await.unwrap();
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}
```

Register the module and the tests in `crates/storage-contract/src/lib.rs`:

```diff
--- a/crates/storage-contract/src/lib.rs
+++ b/crates/storage-contract/src/lib.rs
@@ -14,6 +14,7 @@ pub mod fixture;
 pub mod lists;
 pub mod readings;
 pub mod reports;
+pub mod sessions;
 pub mod transactions;
 pub mod users;
 
@@ -78,6 +79,16 @@ macro_rules! contract_tests {
             audit::a_directly_edited_list_name_is_corrupt_not_normalised,
             audit::a_directly_edited_note_is_corrupt_not_normalised,
             audit::concurrent_writers_never_fork_the_chain,
+            sessions::a_session_round_trips_until_it_expires,
+            sessions::creating_a_session_twice_is_a_duplicate,
+            sessions::a_session_for_an_unknown_user_is_refused,
+            sessions::saving_a_session_inserts_or_replaces_it,
+            sessions::deleting_a_session_removes_only_that_one,
+            sessions::expired_sessions_are_swept,
+            sessions::a_password_change_ends_the_users_sessions,
+            sessions::expiring_a_user_ends_their_sessions,
+            sessions::a_rolled_back_password_change_keeps_the_sessions,
+            sessions::sessions_are_not_audited,
         );
     };
     (@each $fixture:path; $($module:ident::$test:ident),+ $(,)?) => {
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite --test contract`
Expected: compile errors — `StoredSession` and `SessionRepo` are not defined.

- [ ] **Step 3: Add the domain types and trait**

```diff
--- a/crates/domain/src/records.rs
+++ b/crates/domain/src/records.rs
@@ -310,6 +310,19 @@ pub struct NewReading {
     pub value: Pm25,
 }
 
+/// A login session as stored (spec §8.4). Not audited: sessions are not evidence.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct StoredSession {
+    /// SHA-256 of the session id in the cookie, so a copy of the database cannot be used to
+    /// take over a session.
+    pub id_hash: String,
+    /// The signed-in user; lets a password change or expiry end all of that user's sessions.
+    pub user_id: Option<UserId>,
+    /// The session's key/value data as JSON.
+    pub data: String,
+    pub expires_at: DateTime<Utc>,
+}
+
 #[cfg(test)]
 mod tests {
     use chrono::TimeDelta;
```

```diff
--- a/crates/domain/src/repos.rs
+++ b/crates/domain/src/repos.rs
@@ -14,7 +14,7 @@ use crate::DomainError;
 use crate::audit::{AuditEntity, ChainedEntry};
 use crate::records::{
     Closure, Event, NewEvent, NewReading, NewReport, NewRoom, NewSymptom, NewUser, PasswordHash,
-    Pm25Reading, ReportValues, Room, SmellReport, Symptom, User,
+    Pm25Reading, ReportValues, Room, SmellReport, StoredSession, Symptom, User,
 };
 use crate::types::{
     EventId, Locale, Pm25, ReadingId, ReportId, RoomId, SymptomId, UserId, Username,
@@ -154,6 +154,7 @@ pub trait UserWrites: Send {
     /// ([`crate::permissions::check_expiry`]) and with [`Conflict::UsernameTaken`].
     fn create_user(&mut self, new: NewUser) -> impl Future<Output = StoreResult<User>> + Send;
     /// Always audited (as `password_change`), although the snapshot only shows `has_password`.
+    /// Ends every session of the user in the same transaction (spec §8.4).
     fn set_password(
         &mut self,
         id: UserId,
@@ -164,7 +165,7 @@ pub trait UserWrites: Send {
         id: UserId,
         locale: Locale,
     ) -> impl Future<Output = StoreResult<User>> + Send;
-    /// Sets `expires_at` to now. Residents cannot expire.
+    /// Sets `expires_at` to now and ends every session of the user. Residents cannot expire.
     fn expire_user(&mut self, id: UserId) -> impl Future<Output = StoreResult<User>> + Send;
     fn reactivate_user(
         &mut self,
@@ -295,6 +296,34 @@ pub trait SnapshotRepo: Send + Sync {
     fn audit_state(&self) -> impl Future<Output = StoreResult<AuditState>> + Send;
 }
 
+/// Login sessions, for the web layer's session store. Not audited (decision #50).
+///
+/// Each method is its own small write or read, outside any [`Transactional::begin`]
+/// transaction; on SQLite the writes share the single writer connection, so never call them
+/// while holding a transaction.
+pub trait SessionRepo: Send + Sync {
+    /// The session, unless it has expired by `now`.
+    fn session(
+        &self,
+        id_hash: &str,
+        now: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<Option<StoredSession>>> + Send;
+    /// Fails with [`Conflict::Duplicate`] if a session with that `id_hash` exists.
+    fn create_session(
+        &self,
+        session: &StoredSession,
+    ) -> impl Future<Output = StoreResult<()>> + Send;
+    /// Inserts the session or replaces the stored one.
+    fn save_session(&self, session: &StoredSession)
+    -> impl Future<Output = StoreResult<()>> + Send;
+    fn delete_session(&self, id_hash: &str) -> impl Future<Output = StoreResult<()>> + Send;
+    /// Removes every session that has expired by `now` and returns how many there were.
+    fn delete_expired_sessions(
+        &self,
+        now: DateTime<Utc>,
+    ) -> impl Future<Output = StoreResult<u64>> + Send;
+}
+
 /// A complete storage backend.
 pub trait Store:
     Transactional<Tx: WriteTx>
@@ -306,6 +335,7 @@ pub trait Store:
     + JobRepo
     + AuditRepo
     + SnapshotRepo
+    + SessionRepo
 {
 }
 
@@ -319,6 +349,7 @@ impl<T> Store for T where
         + JobRepo
         + AuditRepo
         + SnapshotRepo
+        + SessionRepo
 {
 }
```

- [ ] **Step 4: Add the migrations**

`crates/storage-sqlite/migrations/0002_sessions.sql`:

```sql
-- Login sessions (spec §8.4), written by the web layer's session store. Not audited
-- (decision #50). `id_hash` is the SHA-256 of the cookie's session id, never the id itself.

CREATE TABLE sessions (
    id_hash    TEXT    NOT NULL PRIMARY KEY,
    user_id    INTEGER REFERENCES users (id),
    data       TEXT    NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX sessions_user ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);
```

`crates/storage-postgres/migrations/0002_sessions.sql`:

```sql
-- Login sessions (spec §8.4), written by the web layer's session store. Not audited
-- (decision #50). `id_hash` is the SHA-256 of the cookie's session id, never the id itself.
-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.

CREATE TABLE sessions (
    id_hash    TEXT   NOT NULL PRIMARY KEY,
    user_id    BIGINT REFERENCES users (id),
    data       TEXT   NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE INDEX sessions_user ON sessions (user_id);
CREATE INDEX sessions_expires_at ON sessions (expires_at);
```

Update the table list and the column count the existing schema tests expect:

```diff
--- a/crates/storage-sqlite/tests/contract.rs
+++ b/crates/storage-sqlite/tests/contract.rs
@@ -62,6 +62,7 @@ async fn migrations_create_every_table_and_can_run_again() {
             "jobs",
             "pm25_readings",
             "rooms",
+            "sessions",
             "smell_report_rooms",
             "smell_report_symptoms",
             "smell_reports",
```

```diff
--- a/crates/storage-postgres/tests/schema_parity.rs
+++ b/crates/storage-postgres/tests/schema_parity.rs
@@ -77,5 +77,5 @@ async fn sqlite_and_postgres_schemas_have_the_same_columns() {
         only_sqlite.is_empty() && only_postgres.is_empty(),
         "only in SQLite: {only_sqlite:#?}\nonly in Postgres: {only_postgres:#?}"
     );
-    assert_eq!(sqlite.len(), 62, "a table went missing from the comparison");
+    assert_eq!(sqlite.len(), 66, "a table went missing from the comparison");
 }
```

- [ ] **Step 5: Implement `SessionRepo` for SQLite**

Create `crates/storage-sqlite/src/sessions.rs`:

```rust
//! Login sessions. Bookkeeping, not evidence: not audited.

use chrono::{DateTime, Utc};
use smoke_domain::records::StoredSession;
use smoke_domain::repos::{SessionRepo, StoreResult};
use smoke_domain::types::UserId;
use sqlx::SqliteConnection;

use crate::SqliteStore;
use crate::convert::{instant, micros};
use crate::error::IntoStore;

struct SessionRow {
    id_hash: String,
    user_id: Option<i64>,
    data: String,
    expires_at: i64,
}

impl SessionRow {
    fn into_session(self) -> StoreResult<StoredSession> {
        Ok(StoredSession {
            id_hash: self.id_hash,
            user_id: self.user_id.map(UserId),
            data: self.data,
            expires_at: instant(self.expires_at)?,
        })
    }
}

/// Ends every session of `user`; called inside the transaction that changes their password or
/// expires them.
pub(crate) async fn delete_for_user(conn: &mut SqliteConnection, user: UserId) -> StoreResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user.0)
        .execute(conn)
        .await
        .db()?;
    Ok(())
}

impl SessionRepo for SqliteStore {
    async fn session(
        &self,
        id_hash: &str,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredSession>> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id_hash, user_id, data, expires_at FROM sessions \
             WHERE id_hash = $1 AND expires_at > $2",
            id_hash,
            micros(now),
        )
        .fetch_optional(&mut *self.read().await?)
        .await
        .db()?
        .map(SessionRow::into_session)
        .transpose()
    }

    async fn create_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4)",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn save_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (id_hash) DO UPDATE SET user_id = excluded.user_id, \
             data = excluded.data, expires_at = excluded.expires_at",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn delete_session(&self, id_hash: &str) -> StoreResult<()> {
        sqlx::query!("DELETE FROM sessions WHERE id_hash = $1", id_hash)
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(())
    }

    async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> StoreResult<u64> {
        let done = sqlx::query!("DELETE FROM sessions WHERE expires_at <= $1", micros(now))
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(done.rows_affected())
    }
}
```

Register the module and give single-statement writes the writer connection:

```diff
--- a/crates/storage-sqlite/src/lib.rs
+++ b/crates/storage-sqlite/src/lib.rs
@@ -12,6 +12,7 @@ mod jobs;
 mod lists;
 mod readings;
 mod reports;
+mod sessions;
 mod tx;
 mod users;
 
@@ -83,6 +84,11 @@ impl SqliteStore {
     async fn read(&self) -> StoreResult<PoolConnection<Sqlite>> {
         self.reader.acquire().await.db()
     }
+
+    /// The writer connection, for the unaudited single-statement writes (sessions).
+    async fn write(&self) -> StoreResult<PoolConnection<Sqlite>> {
+        self.writer.acquire().await.db()
+    }
 }
 
 impl Transactional for SqliteStore {
```

End the user's sessions inside `set_password` and `expire_user`:

```diff
--- a/crates/storage-sqlite/src/users.rs
+++ b/crates/storage-sqlite/src/users.rs
@@ -11,6 +11,7 @@ use sqlx::SqliteConnection;
 
 use crate::convert::{canonical, corrupt, instant, micros, opt_instant, parse};
 use crate::error::IntoStore;
+use crate::sessions;
 use crate::{SqliteStore, SqliteTx};
 
 struct UserRow {
@@ -137,6 +138,7 @@ impl UserWrites for SqliteTx {
         .execute(self.conn())
         .await
         .db()?;
+        sessions::delete_for_user(self.conn(), id).await?;
         audited(self, AuditAction::PasswordChange, before).await
     }
 
@@ -163,6 +165,7 @@ impl UserWrites for SqliteTx {
             .execute(self.conn())
             .await
             .db()?;
+        sessions::delete_for_user(self.conn(), id).await?;
         audited(self, AuditAction::Expire, before).await
     }
```

- [ ] **Step 6: Implement `SessionRepo` for Postgres**

Create `crates/storage-postgres/src/sessions.rs` — the same SQL text as SQLite:

```rust
//! Login sessions. Bookkeeping, not evidence: not audited.

use chrono::{DateTime, Utc};
use smoke_domain::records::StoredSession;
use smoke_domain::repos::{SessionRepo, StoreResult};
use smoke_domain::types::UserId;
use sqlx::PgConnection;

use crate::PostgresStore;
use crate::convert::{instant, micros};
use crate::error::IntoStore;

struct SessionRow {
    id_hash: String,
    user_id: Option<i64>,
    data: String,
    expires_at: i64,
}

impl SessionRow {
    fn into_session(self) -> StoreResult<StoredSession> {
        Ok(StoredSession {
            id_hash: self.id_hash,
            user_id: self.user_id.map(UserId),
            data: self.data,
            expires_at: instant(self.expires_at)?,
        })
    }
}

/// Ends every session of `user`; called inside the transaction that changes their password or
/// expires them.
pub(crate) async fn delete_for_user(conn: &mut PgConnection, user: UserId) -> StoreResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user.0)
        .execute(conn)
        .await
        .db()?;
    Ok(())
}

impl SessionRepo for PostgresStore {
    async fn session(
        &self,
        id_hash: &str,
        now: DateTime<Utc>,
    ) -> StoreResult<Option<StoredSession>> {
        sqlx::query_as!(
            SessionRow,
            "SELECT id_hash, user_id, data, expires_at FROM sessions \
             WHERE id_hash = $1 AND expires_at > $2",
            id_hash,
            micros(now),
        )
        .fetch_optional(&mut *self.read().await?)
        .await
        .db()?
        .map(SessionRow::into_session)
        .transpose()
    }

    async fn create_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4)",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn save_session(&self, session: &StoredSession) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO sessions (id_hash, user_id, data, expires_at) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (id_hash) DO UPDATE SET user_id = excluded.user_id, \
             data = excluded.data, expires_at = excluded.expires_at",
            session.id_hash,
            session.user_id.map(|u| u.0),
            session.data,
            micros(session.expires_at),
        )
        .execute(&mut *self.write().await?)
        .await
        .db()?;
        Ok(())
    }

    async fn delete_session(&self, id_hash: &str) -> StoreResult<()> {
        sqlx::query!("DELETE FROM sessions WHERE id_hash = $1", id_hash)
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(())
    }

    async fn delete_expired_sessions(&self, now: DateTime<Utc>) -> StoreResult<u64> {
        let done = sqlx::query!("DELETE FROM sessions WHERE expires_at <= $1", micros(now))
            .execute(&mut *self.write().await?)
            .await
            .db()?;
        Ok(done.rows_affected())
    }
}
```

```diff
--- a/crates/storage-postgres/src/lib.rs
+++ b/crates/storage-postgres/src/lib.rs
@@ -12,6 +12,7 @@ mod jobs;
 mod lists;
 mod readings;
 mod reports;
+mod sessions;
 mod tx;
 mod users;
 
@@ -55,6 +56,11 @@ impl PostgresStore {
     async fn read(&self) -> StoreResult<PoolConnection<Postgres>> {
         self.pool.acquire().await.db()
     }
+
+    /// A pooled connection for the unaudited single-statement writes (sessions).
+    async fn write(&self) -> StoreResult<PoolConnection<Postgres>> {
+        self.pool.acquire().await.db()
+    }
 }
 
 impl Transactional for PostgresStore {
```

```diff
--- a/crates/storage-postgres/src/users.rs
+++ b/crates/storage-postgres/src/users.rs
@@ -11,6 +11,7 @@ use sqlx::PgConnection;
 
 use crate::convert::{canonical, corrupt, instant, micros, opt_instant, parse};
 use crate::error::IntoStore;
+use crate::sessions;
 use crate::{PostgresStore, PostgresTx};
 
 struct UserRow {
@@ -137,6 +138,7 @@ impl UserWrites for PostgresTx {
         .execute(self.conn())
         .await
         .db()?;
+        sessions::delete_for_user(self.conn(), id).await?;
         audited(self, AuditAction::PasswordChange, before).await
     }
 
@@ -163,6 +165,7 @@ impl UserWrites for PostgresTx {
             .execute(self.conn())
             .await
             .db()?;
+        sessions::delete_for_user(self.conn(), id).await?;
         audited(self, AuditAction::Expire, before).await
     }
```

- [ ] **Step 7: Regenerate the query metadata and run the tests**

Run:
```sh
export TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres
scripts/sqlx-prepare.sh
cargo test -p smoke-storage-sqlite -p smoke-storage-postgres
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: six new `query-*.json` files in each crate's `.sqlx/`; every contract test passes on
both backends (10 new `sessions::` tests each), the schema-parity test sees 66 columns.

- [ ] **Step 8: Commit**

```bash
git add crates/domain crates/storage-contract crates/storage-sqlite crates/storage-postgres
git commit -m "feat(storage): SessionRepo trait with SQLite and Postgres implementations

Sessions are stored by the SHA-256 of their id, never audited, and ended
inside the same transaction as a password change or an expiry, so a
rolled-back change keeps them. Both migrations have identical columns.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Web crate — configuration and Argon2id

Creates `smoke-web` with the two pieces that need no server: the public base URL (which decides
cookie flags, HSTS and the Origin check) and password hashing. All web and app dependencies are
added to the workspace here, with versions checked on crates.io.

**Files:**
- Modify: `Cargo.toml` (workspace dependencies, profiles)
- Create: `crates/web/Cargo.toml`, `crates/web/src/lib.rs`, `crates/web/src/config.rs`,
  `crates/web/src/auth/mod.rs`, `crates/web/src/auth/password.rs`
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: `NewPassword` (Task 1), `smoke_domain::records::PasswordHash`.
- Produces:
  - `smoke_web::config::BaseUrl::parse(&str) -> Result<BaseUrl, BaseUrlError>`,
    `origin(&self) -> &str` (`scheme://host[:port]`, lowercase, no default port),
    `is_https(&self) -> bool`. Plain `http://` is accepted only for loopback hosts.
  - `smoke_web::config::WebConfig { base_url: BaseUrl, client_ip_header: Option<HeaderName> }`.
  - `smoke_web::password::hash(&NewPassword) -> Result<PasswordHash, HashError>` and
    `verify(Option<&PasswordHash>, &str) -> bool` (always one Argon2 verification).

- [ ] **Step 1: Add the workspace dependencies**

```diff
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -10,20 +10,50 @@ publish = false
 repository = "https://github.com/locadani/smoke-tracker"
 
 [workspace.dependencies]
+anyhow = "1.0.104"
+argon2 = "0.6.0"
+askama = { version = "0.16.1", default-features = false, features = ["derive", "std"] }
+async-trait = "0.1.92"
+axum = { version = "0.8.9", default-features = false, features = ["form", "http1", "tokio", "tracing"] }
 chrono = { version = "0.4", default-features = false, features = ["clock", "std", "serde"] }
 chrono-tz = "0.10"
+clap = { version = "4.6.7", features = ["derive"] }
+fluent-syntax = "0.12.0"
+fluent-templates = { version = "0.15.1", default-features = false, features = ["macros", "walkdir"] }
+getrandom = "0.4.3"
 hex = "0.4"
+http-body-util = "0.1.5"
 proptest = "1"
+rpassword = "7.5.4"
 serde = { version = "1", features = ["derive"] }
 serde_json = { version = "1", features = ["float_roundtrip"] }
 sha2 = "0.11"
 smoke-domain = { path = "crates/domain" }
 smoke-storage-contract = { path = "crates/storage-contract" }
+smoke-storage-postgres = { path = "crates/storage-postgres" }
 smoke-storage-sqlite = { path = "crates/storage-sqlite" }
+smoke-web = { path = "crates/web" }
 sqlx = { version = "0.9", default-features = false, features = ["runtime-tokio", "macros", "migrate", "sqlx-toml"] }
 tempfile = "3"
 thiserror = "2"
-tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
+tokio = { version = "1.53", features = ["rt-multi-thread", "macros"] }
+tower = { version = "0.5.3", default-features = false, features = ["util"] }
+tower-http = { version = "0.7.1", features = ["catch-panic", "request-id", "trace"] }
+tower-sessions = { version = "0.15.0", default-features = false, features = ["axum-core"] }
+tracing = "0.1.44"
+tracing-subscriber = { version = "0.3.23", default-features = false, features = ["ansi", "env-filter", "fmt"] }
+
+# Argon2 is deliberately slow; unoptimised it makes every login test take seconds.
+[profile.dev.package.argon2]
+opt-level = 3
+
+[profile.dev.package.blake2]
+opt-level = 3
+
+[profile.release]
+codegen-units = 1
+lto = "thin"
+strip = true
 
 [workspace.lints.rust]
 unsafe_code = "forbid"
```

Create `crates/web/Cargo.toml`:

```toml
[package]
name = "smoke-web"
version = "0.1.0"
description = "Axum routes, templates, translations and static assets of Smoke Tracker"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
argon2.workspace = true
askama.workspace = true
async-trait.workspace = true
axum.workspace = true
chrono.workspace = true
fluent-templates.workspace = true
getrandom.workspace = true
hex.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
smoke-domain.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["net"] }
tower-http.workspace = true
tower-sessions.workspace = true
tracing.workspace = true

[dev-dependencies]
fluent-syntax.workspace = true
http-body-util.workspace = true
smoke-storage-sqlite.workspace = true
sqlx = { workspace = true, features = ["sqlite-bundled"] }
tempfile.workspace = true
tower.workspace = true

[lints]
workspace = true
```

- [ ] **Step 2: Write the failing tests**

Create `crates/web/src/lib.rs`:

```rust
//! The web layer of Smoke Tracker: Axum routes, Askama templates, Fluent translations and the
//! embedded static assets. It reaches the database only through the `smoke-domain` traits, so
//! every route works the same on SQLite and Postgres.

mod auth;
pub mod config;

pub use crate::auth::password;
```

Create `crates/web/src/auth/mod.rs`:

```rust
//! Who is signed in: passwords, the session's user and the sign-in limits.

pub mod password;
```

Create `crates/web/src/config.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_origin_drops_trailing_slashes_default_ports_and_case() {
        for raw in [
            "https://Smoke.Example.org",
            "https://smoke.example.org/",
            "https://smoke.example.org:443/",
            " https://smoke.example.org ",
        ] {
            let url = BaseUrl::parse(raw).unwrap();
            assert_eq!(url.origin(), "https://smoke.example.org", "{raw}");
            assert!(url.is_https());
        }
        assert_eq!(
            BaseUrl::parse("https://smoke.example.org:8443")
                .unwrap()
                .origin(),
            "https://smoke.example.org:8443"
        );
    }

    #[test]
    fn plain_http_is_only_for_loopback_hosts() {
        for raw in [
            "http://localhost:8080",
            "http://127.0.0.1:8080",
            "http://[::1]:8080",
            "http://app.localhost",
        ] {
            let url = BaseUrl::parse(raw).unwrap();
            assert!(!url.is_https(), "{raw}");
        }
        assert_eq!(
            BaseUrl::parse("http://192.168.1.20:8080"),
            Err(BaseUrlError::InsecureHost(
                "http://192.168.1.20:8080".into()
            ))
        );
        assert_eq!(
            BaseUrl::parse("http://smoke.example.org"),
            Err(BaseUrlError::InsecureHost(
                "http://smoke.example.org".into()
            ))
        );
    }

    #[test]
    fn base_urls_with_paths_or_odd_schemes_are_rejected() {
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/app"),
            Err(BaseUrlError::HasPath(_))
        ));
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/?a=1"),
            Err(BaseUrlError::HasPath(_))
        ));
        assert!(matches!(
            BaseUrl::parse("https://smoke.example.org/#top"),
            Err(BaseUrlError::HasPath(_))
        ));
        for raw in [
            "smoke.example.org",
            "ftp://smoke.example.org",
            "https://u@smoke.example.org",
            "",
        ] {
            assert!(
                matches!(BaseUrl::parse(raw), Err(BaseUrlError::NotAbsolute(_))),
                "{raw}"
            );
        }
    }
}
```

Create `crates/web/src/auth/password.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const PREFIX: &str = "$argon2id$v=19$m=19456,t=2,p=1$";

    fn password(text: &str) -> NewPassword {
        NewPassword::new(text.to_owned()).unwrap()
    }

    #[test]
    fn a_hash_verifies_only_its_own_password() {
        let stored = hash(&password("correct horse battery")).unwrap();
        assert!(verify(Some(&stored), "correct horse battery"));
        assert!(!verify(Some(&stored), "correct horse batter"));
        assert!(!verify(Some(&stored), ""));
    }

    #[test]
    fn hashes_are_salted_argon2id_with_the_default_cost() {
        let a = hash(&password("correct horse battery")).unwrap();
        let b = hash(&password("correct horse battery")).unwrap();
        assert!(a.as_str().starts_with(PREFIX), "{}", a.as_str());
        assert_ne!(a, b);
    }

    #[test]
    fn the_dummy_hash_costs_the_same_as_a_real_one() {
        assert!(DUMMY_HASH.starts_with(PREFIX), "{}", *DUMMY_HASH);
    }

    #[test]
    fn a_missing_or_broken_hash_never_verifies() {
        assert!(!verify(None, "no account has this password"));
        let broken = PasswordHash::new("not-a-phc-string".to_owned());
        assert!(!verify(Some(&broken), "anything"));
    }
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile errors — `BaseUrl`, `hash`, `verify` and `DUMMY_HASH` are not defined.

- [ ] **Step 4: Implement**

Put this above the tests in `crates/web/src/config.rs`:

```rust
//! Web settings derived from the environment by the binary.

use std::net::IpAddr;

use axum::http::{HeaderName, Uri};
use thiserror::Error;

/// The public address of the app (`APP_BASE_URL`). It decides the cookie flags, HSTS, and the
/// origin that state-changing requests must come from, so a proxy's internal `Host` header
/// never matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseUrl {
    origin: String,
    https: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BaseUrlError {
    #[error("APP_BASE_URL must be an absolute http:// or https:// URL, got {0:?}")]
    NotAbsolute(String),
    #[error("APP_BASE_URL must not have a path, query or fragment, got {0:?}")]
    HasPath(String),
    #[error("APP_BASE_URL must use https:// unless the host is localhost, got {0:?}")]
    InsecureHost(String),
}

impl BaseUrl {
    pub fn parse(raw: &str) -> Result<Self, BaseUrlError> {
        let trimmed = raw.trim().trim_end_matches('/');
        let not_absolute = || BaseUrlError::NotAbsolute(raw.to_owned());
        if trimmed.contains('#') {
            return Err(BaseUrlError::HasPath(raw.to_owned()));
        }
        let uri: Uri = trimmed.parse().map_err(|_| not_absolute())?;
        let (Some(scheme), Some(authority)) = (uri.scheme_str(), uri.authority()) else {
            return Err(not_absolute());
        };
        if authority.as_str().contains('@') {
            return Err(not_absolute());
        }
        if !matches!(uri.path(), "" | "/") || uri.query().is_some() {
            return Err(BaseUrlError::HasPath(raw.to_owned()));
        }
        let https = match scheme.to_ascii_lowercase().as_str() {
            "https" => true,
            "http" => false,
            _ => return Err(not_absolute()),
        };
        let host = authority.host().to_ascii_lowercase();
        if !https && !is_loopback(&host) {
            return Err(BaseUrlError::InsecureHost(raw.to_owned()));
        }
        let default_port = if https { 443 } else { 80 };
        let scheme = if https { "https" } else { "http" };
        let origin = match authority.port_u16().filter(|&port| port != default_port) {
            Some(port) => format!("{scheme}://{host}:{port}"),
            None => format!("{scheme}://{host}"),
        };
        Ok(Self { origin, https })
    }

    /// `scheme://host[:port]`, lowercase, without a default port: what browsers send as `Origin`.
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn is_https(&self) -> bool {
        self.https
    }
}

fn is_loopback(host: &str) -> bool {
    host == "localhost"
        || host.ends_with(".localhost")
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

#[derive(Debug, Clone)]
pub struct WebConfig {
    pub base_url: BaseUrl,
    /// A header set by a trusted reverse proxy that carries the client's address
    /// (`CLIENT_IP_HEADER`). Without it the peer address is used.
    pub client_ip_header: Option<HeaderName>,
}
```

Put this above the tests in `crates/web/src/auth/password.rs`:

```rust
//! Argon2id password hashing (spec §8.1), with the library's default cost: 19 MiB of memory,
//! two passes, one lane.

use std::sync::LazyLock;

use argon2::Argon2;
use argon2::password_hash::phc::PasswordHash as Phc;
use argon2::password_hash::{PasswordHasher, PasswordVerifier};
use smoke_domain::records::PasswordHash;
use smoke_domain::types::NewPassword;
use thiserror::Error;

#[derive(Debug, Error)]
#[error("could not hash the password: {0}")]
pub struct HashError(String);

/// Hashes with a fresh random salt. CPU-heavy: call it from a blocking thread in async code.
pub fn hash(password: &NewPassword) -> Result<PasswordHash, HashError> {
    Argon2::default()
        .hash_password(password.expose().as_bytes())
        .map(|phc| PasswordHash::new(phc.to_string()))
        .map_err(|e| HashError(e.to_string()))
}

/// A hash of a password nobody has, checked when there is no real hash so that an unknown
/// username costs as much time as a wrong password.
static DUMMY_HASH: LazyLock<String> = LazyLock::new(|| {
    Argon2::default()
        .hash_password(b"no account has this password")
        .expect("hashing a constant cannot fail")
        .to_string()
});

/// Whether `attempt` matches `stored`. Always runs one Argon2 verification, also when there is
/// no stored hash. CPU-heavy: call it from a blocking thread in async code.
pub fn verify(stored: Option<&PasswordHash>, attempt: &str) -> bool {
    let argon2 = Argon2::default();
    if let Some(stored) = stored {
        match Phc::new(stored.as_str()) {
            Ok(phc) => return argon2.verify_password(attempt.as_bytes(), &phc).is_ok(),
            Err(error) => tracing::error!(%error, "a stored password hash is not a PHC string"),
        }
    }
    let dummy = Phc::new(&DUMMY_HASH).expect("the dummy hash is a PHC string");
    let _ = argon2.verify_password(attempt.as_bytes(), &dummy);
    false
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings && cargo deny check`
Expected: 7 tests pass; no warnings; `advisories ok, bans ok, licenses ok, sources ok`.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/web
git commit -m "feat(web): BaseUrl parsing and Argon2id hashing with a timing-safe verify

APP_BASE_URL is parsed once into an origin that later drives cookie flags,
HSTS and the Origin check. verify() always runs one Argon2 verification,
against a dummy PHC hash when there is no account, so timing does not tell
which usernames exist.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Web — the server skeleton: state, sessions, security headers, assets

The router with everything except pages: request ids, tracing, security headers, a body limit,
the Origin check, the session layer over `DbSessions`, embedded static files and `/healthz`.
Unknown pages answer a bare `404` until Task 5 adds error pages.

**Files:**
- Modify: `crates/web/src/lib.rs`
- Create: `crates/web/src/{state,sessions,security,request_id,assets}.rs`
- Create: `crates/web/static/app.css`, `crates/web/static/favicon.svg`,
  `crates/web/static/htmx-2.0.11.min.js`
- Create: `crates/web/tests/common/mod.rs`, `crates/web/tests/security.rs`

**Interfaces:**
- Consumes: `SessionRepo`, `StoredSession` (Task 2); `BaseUrl`, `WebConfig`, `password` (Task 3);
  `smoke_domain::clock::{Clock, FakeClock, SystemClock}`.
- Produces:
  - `smoke_web::AppState<S>` — `AppState::new(store: S, clock: Arc<dyn Clock>, config:
    WebConfig)`; fields `store`, `clock`, `config: Arc<WebConfig>` are `pub(crate)`.
  - `smoke_web::router<S: Store + Clone + 'static>(AppState<S>) -> axum::Router`.
  - `smoke_web::serve(TcpListener, Router, shutdown: impl Future<Output = ()> + Send + 'static)
    -> io::Result<()>` (with `ConnectInfo<SocketAddr>` and graceful shutdown).
  - `crate::sessions::{USER_ID_KEY = "user_id", SECURE_COOKIE = "__Host-session",
    LOCAL_COOKIE = "session", LIFETIME_DAYS = 30, layer(store, clock, &BaseUrl)}`.
  - `crate::request_id::of(&HeaderMap) -> &str`; `crate::security::{headers, check_origin, CSP}`.
  - Test helpers in `tests/common/mod.rs`: `TestApp::{new, with_config, add_user, send,
    attempt, sign_in}`, `get`, `get_with`, `post_form`, `login_request`, `from`,
    `set_cookies`, `session_cookie`, `location`, `text`, consts `BASE`, `PASSWORD`, `CLIENT`.

- [ ] **Step 1: Add the static files**

Download htmx 2.0.11 from npm and check it is the published file:

```sh
curl -fsSL https://registry.npmjs.org/htmx.org/-/htmx.org-2.0.11.tgz | tar -xzO package/dist/htmx.min.js > crates/web/static/htmx-2.0.11.min.js
shasum -a 256 crates/web/static/htmx-2.0.11.min.js
```
Expected: `d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717`.

`crates/web/static/app.css`:

```css
:root {
  color-scheme: light dark;
  --gap: 1rem;
  --accent: #b4532a;
  font-family: system-ui, sans-serif;
  line-height: 1.5;
}

body {
  margin: 0 auto;
  max-width: 32rem;
  padding: 0 var(--gap);
}

.bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--gap) 0;
}

.brand {
  font-weight: 700;
}

.stack {
  display: flex;
  flex-direction: column;
  gap: var(--gap);
}

.stack label {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

input,
button {
  font: inherit;
  padding: 0.75rem;
  border-radius: 0.5rem;
}

button {
  border: none;
  background: var(--accent);
  color: #fff;
}

button.link {
  background: none;
  color: inherit;
  padding: 0.25rem;
  text-decoration: underline;
}

.notice {
  padding: 0.75rem;
  border-left: 0.25rem solid var(--accent);
}

footer {
  margin-top: calc(var(--gap) * 3);
  font-size: 0.875rem;
}
```

`crates/web/static/favicon.svg`:

```xml
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="6" fill="#b4532a"/><path d="M10 24c0-6 6-6 6-12m4 12c0-5 4-5 4-10" stroke="#fff" stroke-width="2.5" fill="none" stroke-linecap="round"/></svg>
```

- [ ] **Step 2: Write the failing integration tests**

Create `crates/web/tests/common/mod.rs`:

```rust
//! An app on a temporary SQLite database with a fake clock, driven with in-memory requests.

#![allow(dead_code)]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::{CONTENT_TYPE, COOKIE, LOCATION, SET_COOKIE};
use axum::http::{HeaderName, Request, Response, StatusCode};
use chrono::TimeDelta;
use http_body_util::BodyExt;
use smoke_domain::clock::{Clock, FakeClock, SystemClock};
use smoke_domain::records::{NewUser, User};
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_storage_sqlite::SqliteStore;
use smoke_web::config::{BaseUrl, WebConfig};
use smoke_web::{AppState, password};
use tempfile::TempDir;
use tower::ServiceExt;

pub const BASE: &str = "https://smoke.example.org";
pub const PASSWORD: &str = "correct horse battery";
pub const CLIENT: IpAddr = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10));

pub struct TestApp {
    pub router: Router,
    pub store: SqliteStore,
    pub clock: Arc<FakeClock>,
    pub db_url: String,
    _dir: TempDir,
}

impl TestApp {
    pub async fn new() -> Self {
        Self::with_config(BASE, None).await
    }

    /// `tower-sessions` stamps session expiry with the real time, so the fake clock starts at
    /// the real time too.
    pub async fn with_config(base_url: &str, client_ip_header: Option<&'static str>) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let db_url = format!("sqlite://{}", dir.path().join("web.db").display());
        let clock = Arc::new(FakeClock::new(SystemClock.now()));
        let store = SqliteStore::connect(&db_url, clock.clone()).await.unwrap();
        store.migrate().await.unwrap();
        let config = WebConfig {
            base_url: BaseUrl::parse(base_url).unwrap(),
            client_ip_header: client_ip_header.map(HeaderName::from_static),
        };
        let router = smoke_web::router(AppState::new(store.clone(), clock.clone(), config));
        Self {
            router,
            store,
            clock,
            db_url,
            _dir: dir,
        }
    }

    /// Guests and viewers expire in a week; residents never.
    pub async fn add_user(&self, name: &str, role: Role, locale: Locale) -> User {
        let hash = password::hash(&NewPassword::new(PASSWORD.to_owned()).unwrap()).unwrap();
        let mut tx = self.store.begin(None).await.unwrap();
        let user = tx
            .create_user(NewUser {
                username: Username::new(name).unwrap(),
                password_hash: Some(hash),
                role,
                locale,
                expires_at: (role != Role::Resident).then(|| self.clock.now() + TimeDelta::days(7)),
            })
            .await
            .unwrap();
        tx.commit().await.unwrap();
        user
    }

    pub async fn send(&self, request: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(request).await.unwrap()
    }

    pub async fn attempt(&self, username: &str, password: &str) -> Response<Body> {
        self.send(login_request(username, password)).await
    }

    /// Signs in and returns the session cookie as `name=value`.
    pub async fn sign_in(&self, username: &str) -> String {
        let response = self.attempt(username, PASSWORD).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "sign-in failed");
        session_cookie(&response).expect("a session cookie")
    }
}

pub fn get(path: &str) -> Request<Body> {
    Request::get(path).body(Body::empty()).unwrap()
}

pub fn get_with(path: &str, cookie: &str) -> Request<Body> {
    Request::get(path)
        .header(COOKIE, cookie)
        .body(Body::empty())
        .unwrap()
}

/// A form post as a browser on the app's own pages sends it.
pub fn post_form(path: &str, body: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::post(path)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .header("origin", BASE)
        .header("sec-fetch-site", "same-origin");
    if let Some(cookie) = cookie {
        builder = builder.header(COOKIE, cookie);
    }
    let mut request = builder.body(Body::from(body.to_owned())).unwrap();
    from(&mut request, CLIENT);
    request
}

pub fn login_request(username: &str, password: &str) -> Request<Body> {
    let body = format!(
        "username={}&password={}",
        encode(username),
        encode(password)
    );
    post_form("/login", &body, None)
}

/// Marks the request as coming from `ip`, as the server's `ConnectInfo` would.
pub fn from(request: &mut Request<Body>, ip: IpAddr) {
    request
        .extensions_mut()
        .insert(ConnectInfo(SocketAddr::new(ip, 40_000)));
}

fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'.' | b'_' => (b as char).to_string(),
            b' ' => "+".to_owned(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn set_cookies(response: &Response<Body>) -> Vec<String> {
    response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect()
}

/// The session cookie a response sets, as `name=value`.
pub fn session_cookie(response: &Response<Body>) -> Option<String> {
    set_cookies(response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session=") || c.starts_with("session="))
        .map(|c| c.split(';').next().unwrap().to_owned())
}

pub fn location(response: &Response<Body>) -> Option<&str> {
    response
        .headers()
        .get(LOCATION)
        .map(|v| v.to_str().unwrap())
}

pub async fn text(response: Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}
```

Create `crates/web/tests/security.rs`:

```rust
//! Security headers, the cross-site request check and asset serving (spec §12).

mod common;

use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use common::*;

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; \
    connect-src 'self'; font-src 'self'; manifest-src 'self'; worker-src 'self'; \
    object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";

#[tokio::test]
async fn every_response_carries_the_security_headers() {
    let app = TestApp::new().await;
    for path in ["/missing", "/static/app.css", "/healthz"] {
        let response = app.send(get(path)).await;
        let headers = response.headers();
        assert_eq!(headers["content-security-policy"], CSP, "{path}");
        assert_eq!(headers["x-content-type-options"], "nosniff", "{path}");
        assert_eq!(headers["referrer-policy"], "same-origin", "{path}");
        assert_eq!(headers["x-frame-options"], "DENY", "{path}");
        assert_eq!(
            headers["cross-origin-opener-policy"], "same-origin",
            "{path}"
        );
        assert_eq!(
            headers["strict-transport-security"], "max-age=63072000",
            "{path}"
        );
        assert_eq!(headers["x-request-id"].len(), 16, "{path}");
    }
    let page = app.send(get("/missing")).await;
    assert_eq!(page.headers()["cache-control"], "no-store");
}

#[tokio::test]
async fn anonymous_requests_set_no_cookies() {
    let app = TestApp::new().await;
    for path in ["/missing", "/static/app.css", "/healthz"] {
        let response = app.send(get(path)).await;
        assert!(set_cookies(&response).is_empty(), "{path}");
    }
}

#[tokio::test]
async fn cross_site_posts_are_refused() {
    let app = TestApp::new().await;
    let cases: [&[(&str, &str)]; 4] = [
        &[("sec-fetch-site", "cross-site"), ("origin", BASE)],
        &[("sec-fetch-site", "same-site")],
        &[("origin", "https://evil.example")],
        &[],
    ];
    for headers in cases {
        let mut builder =
            Request::post("/login").header(CONTENT_TYPE, "application/x-www-form-urlencoded");
        for (name, value) in headers {
            builder = builder.header(*name, *value);
        }
        let request = builder
            .body(Body::from(format!("username=daniele&password={PASSWORD}")))
            .unwrap();
        let response = app.send(request).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{headers:?}");
        assert!(session_cookie(&response).is_none());
    }
}

#[tokio::test]
async fn reading_pages_is_never_origin_checked() {
    let app = TestApp::new().await;
    let request = Request::get("/missing")
        .header("sec-fetch-site", "cross-site")
        .body(Body::empty())
        .unwrap();
    assert_eq!(app.send(request).await.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn assets_are_served_with_their_type_and_revalidated() {
    let app = TestApp::new().await;
    let response = app.send(get("/static/htmx-2.0.11.min.js")).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/javascript; charset=utf-8"
    );
    assert_eq!(response.headers()["cache-control"], "no-cache");
    let etag = response.headers()["etag"].clone();

    let revalidate = Request::get("/static/htmx-2.0.11.min.js")
        .header("if-none-match", etag)
        .body(Body::empty())
        .unwrap();
    let not_modified = app.send(revalidate).await;
    assert_eq!(not_modified.status(), StatusCode::NOT_MODIFIED);
    assert!(text(not_modified).await.is_empty());

    assert_eq!(
        app.send(get("/static/../Cargo.toml")).await.status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(text(app.send(get("/healthz")).await).await, "ok");
}
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test security`
Expected: compile errors — `smoke_web::router` and `smoke_web::AppState` are not defined.

- [ ] **Step 4: Implement the state and request ids**

`crates/web/src/state.rs`:

```rust
//! What every handler can reach.

use std::sync::Arc;

use smoke_domain::clock::Clock;

use crate::config::WebConfig;

/// Shared application state. `S` is the storage backend, chosen once at startup; it is a type
/// parameter rather than a trait object, so each backend gets its own compiled router.
pub struct AppState<S> {
    pub(crate) store: S,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) config: Arc<WebConfig>,
}

impl<S> AppState<S> {
    pub fn new(store: S, clock: Arc<dyn Clock>, config: WebConfig) -> Self {
        Self {
            store,
            clock,
            config: Arc::new(config),
        }
    }
}

impl<S: Clone> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            clock: self.clock.clone(),
            config: self.config.clone(),
        }
    }
}
```

`crates/web/src/request_id.rs`:

```rust
//! Request ids: generated for every request, logged in its span, returned as `x-request-id`,
//! and shown on error pages so a user can quote one.

use axum::http::{HeaderMap, HeaderValue, Request};
use tower_http::request_id::{MakeRequestId, RequestId};

pub(crate) const HEADER: &str = "x-request-id";

/// 16 hex characters from the operating system's random source.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RandomRequestId;

impl MakeRequestId for RandomRequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<RequestId> {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).ok()?;
        HeaderValue::from_str(&hex::encode(bytes))
            .ok()
            .map(RequestId::new)
    }
}

/// The request's id, or `-` if it has none.
pub(crate) fn of(headers: &HeaderMap) -> &str {
    headers
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_sixteen_hex_characters_and_differ() {
        let request = Request::new(());
        let mut make = RandomRequestId;
        let a = make.make_request_id(&request).unwrap();
        let b = make.make_request_id(&request).unwrap();
        let text = a.header_value().to_str().unwrap();
        assert_eq!(text.len(), 16);
        assert!(text.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a.header_value(), b.header_value());
    }
}
```

- [ ] **Step 5: Implement the session store**

`crates/web/src/sessions.rs`:

```rust
//! Server-side sessions (spec §8.4): `tower-sessions` keeps the cookie and the session data;
//! this module stores them through the domain's [`SessionRepo`], so both database backends
//! work without a second database driver.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use smoke_domain::clock::Clock;
use smoke_domain::records::StoredSession;
use smoke_domain::repos::{Conflict, SessionRepo, StoreError};
use smoke_domain::types::UserId;
use tower_sessions::cookie::SameSite;
use tower_sessions::cookie::time::{Duration, OffsetDateTime};
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{self, SessionStore};
use tower_sessions::{Expiry, SessionManagerLayer};

use crate::config::BaseUrl;

/// The signed-in user's id, as an `i64`.
pub(crate) const USER_ID_KEY: &str = "user_id";
/// Over https the cookie is `__Host-` prefixed: Secure, `Path=/`, no `Domain`.
pub(crate) const SECURE_COOKIE: &str = "__Host-session";
/// Over plain http (localhost only) browsers refuse `Secure` cookies.
pub(crate) const LOCAL_COOKIE: &str = "session";
pub(crate) const LIFETIME_DAYS: i64 = 30;

/// Sessions live 30 days from their last save.
pub(crate) fn layer<S>(
    store: S,
    clock: Arc<dyn Clock>,
    base_url: &BaseUrl,
) -> SessionManagerLayer<DbSessions<S>>
where
    S: SessionRepo + Clone + 'static,
{
    let name = if base_url.is_https() {
        SECURE_COOKIE
    } else {
        LOCAL_COOKIE
    };
    SessionManagerLayer::new(DbSessions { store, clock })
        .with_name(name)
        .with_secure(base_url.is_https())
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_path("/")
        .with_expiry(Expiry::OnInactivity(Duration::days(LIFETIME_DAYS)))
}

/// A `tower-sessions` store over any storage backend.
#[derive(Clone)]
pub(crate) struct DbSessions<S> {
    store: S,
    clock: Arc<dyn Clock>,
}

impl<S> fmt::Debug for DbSessions<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DbSessions")
    }
}

/// The database key of a session: SHA-256 of its id, so a copy of the database holds no
/// usable session ids.
fn id_hash(id: &Id) -> String {
    hex::encode(Sha256::digest(id.0.to_le_bytes()))
}

fn backend(error: impl fmt::Display) -> session_store::Error {
    session_store::Error::Backend(error.to_string())
}

fn to_stored(record: &Record) -> session_store::Result<StoredSession> {
    let micros = record.expiry_date.unix_timestamp_nanos() / 1_000;
    let expires_at = i64::try_from(micros)
        .ok()
        .and_then(DateTime::<Utc>::from_timestamp_micros)
        .ok_or_else(|| session_store::Error::Encode("expiry out of range".to_owned()))?;
    Ok(StoredSession {
        id_hash: id_hash(&record.id),
        user_id: record
            .data
            .get(USER_ID_KEY)
            .and_then(serde_json::Value::as_i64)
            .map(UserId),
        data: serde_json::to_string(&record.data)
            .map_err(|e| session_store::Error::Encode(e.to_string()))?,
        expires_at,
    })
}

fn to_record(id: Id, stored: StoredSession) -> session_store::Result<Record> {
    let nanos = i128::from(stored.expires_at.timestamp_micros()) * 1_000;
    Ok(Record {
        id,
        data: serde_json::from_str(&stored.data)
            .map_err(|e| session_store::Error::Decode(e.to_string()))?,
        expiry_date: OffsetDateTime::from_unix_timestamp_nanos(nanos)
            .map_err(|e| session_store::Error::Decode(e.to_string()))?,
    })
}

#[async_trait]
impl<S> SessionStore for DbSessions<S>
where
    S: SessionRepo + Clone + 'static,
{
    /// Picks a fresh id on the (astronomically unlikely) collision with a stored one.
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        loop {
            match self.store.create_session(&to_stored(record)?).await {
                Ok(()) => return Ok(()),
                Err(StoreError::Conflict(Conflict::Duplicate)) => record.id = Id::default(),
                Err(error) => return Err(backend(error)),
            }
        }
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        self.store
            .save_session(&to_stored(record)?)
            .await
            .map_err(backend)
    }

    async fn load(&self, id: &Id) -> session_store::Result<Option<Record>> {
        self.store
            .session(&id_hash(id), self.clock.now())
            .await
            .map_err(backend)?
            .map(|stored| to_record(*id, stored))
            .transpose()
    }

    async fn delete(&self, id: &Id) -> session_store::Result<()> {
        self.store
            .delete_session(&id_hash(id))
            .await
            .map_err(backend)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[test]
    fn the_stored_key_is_a_hash_not_the_cookie_value() {
        let id = Id(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        let key = id_hash(&id);
        assert_eq!(key.len(), 64);
        assert!(!key.contains(&id.to_string()));
        assert_eq!(key, id_hash(&id));
        assert_ne!(key, id_hash(&Id(1)));
    }

    #[test]
    fn records_round_trip_through_the_stored_form() {
        let record = Record {
            id: Id(42),
            data: HashMap::from([
                (USER_ID_KEY.to_owned(), serde_json::json!(7)),
                ("note".to_owned(), serde_json::json!("kept as JSON")),
            ]),
            expiry_date: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        };
        let stored = to_stored(&record).unwrap();
        assert_eq!(stored.user_id, Some(UserId(7)));
        assert_eq!(stored.expires_at.timestamp(), 1_800_000_000);
        assert_eq!(to_record(Id(42), stored).unwrap(), record);
    }

    #[test]
    fn a_session_without_a_user_stores_no_user_id() {
        let record = Record {
            id: Id(1),
            data: HashMap::new(),
            expiry_date: OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap(),
        };
        assert_eq!(to_stored(&record).unwrap().user_id, None);
    }
}
```

- [ ] **Step 6: Implement the security headers and the Origin check**

`crates/web/src/security.rs`:

```rust
//! Response headers every page carries, and the cross-site request check (spec §12).

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, ORIGIN, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::config::WebConfig;

/// Same-origin resources only: no inline scripts or styles, no CDNs, no framing.
pub(crate) const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
    img-src 'self'; connect-src 'self'; font-src 'self'; manifest-src 'self'; \
    worker-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; \
    frame-ancestors 'none'";

const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), payment=()";
const HSTS: &str = "max-age=63072000";

/// Adds the security headers. HSTS only over https; `Cache-Control: no-store` unless the
/// response set its own (static assets do), so signed-in pages never sit in a shared cache.
pub(crate) async fn headers(State(https): State<bool>, request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(CONTENT_SECURITY_POLICY, HeaderValue::from_static(CSP));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("same-origin"));
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(
        HeaderName::from_static("cross-origin-opener-policy"),
        HeaderValue::from_static("same-origin"),
    );
    headers.insert(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(PERMISSIONS_POLICY),
    );
    if https {
        headers.insert(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static(HSTS));
    }
    headers
        .entry(CACHE_CONTROL)
        .or_insert(HeaderValue::from_static("no-store"));
    response
}

/// Refuses state-changing requests that a browser marks as cross-site (CSRF defence on top of
/// `SameSite=Lax`).
pub(crate) async fn check_origin(
    State(config): State<Arc<WebConfig>>,
    request: Request,
    next: Next,
) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    if safe || is_same_origin(request.headers(), config.base_url.origin()) {
        next.run(request).await
    } else {
        tracing::warn!("refused a cross-site request");
        StatusCode::FORBIDDEN.into_response()
    }
}

/// `Sec-Fetch-Site` decides when the browser sends it; otherwise `Origin` must equal the app's
/// own origin. A request with neither is refused: every browser this app supports sends one.
fn is_same_origin(headers: &HeaderMap, origin: &str) -> bool {
    if let Some(site) = headers.get("sec-fetch-site") {
        return site.as_bytes() == b"same-origin";
    }
    headers
        .get(ORIGIN)
        .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(origin.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const APP: &str = "https://smoke.example.org";

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        pairs
            .iter()
            .map(|(k, v)| (HeaderName::from_static(k), HeaderValue::from_static(v)))
            .collect()
    }

    #[test]
    fn fetch_metadata_decides_when_present() {
        assert!(is_same_origin(
            &headers(&[("sec-fetch-site", "same-origin")]),
            APP
        ));
        for site in ["same-site", "cross-site", "none"] {
            let h = headers(&[("sec-fetch-site", site), ("origin", APP)]);
            assert!(!is_same_origin(&h, APP), "{site}");
        }
    }

    #[test]
    fn without_fetch_metadata_the_origin_must_match() {
        assert!(is_same_origin(&headers(&[("origin", APP)]), APP));
        assert!(is_same_origin(
            &headers(&[("origin", "https://SMOKE.example.org")]),
            APP
        ));
        assert!(!is_same_origin(
            &headers(&[("origin", "https://evil.example")]),
            APP
        ));
        assert!(!is_same_origin(&headers(&[("origin", "null")]), APP));
        assert!(!is_same_origin(&HeaderMap::new(), APP));
    }
}
```

- [ ] **Step 7: Implement the embedded assets**

`crates/web/src/assets.rs`:

```rust
//! Static files, compiled into the binary (spec §10.3). Each is listed here explicitly; a test
//! checks the list matches the `static/` folder.

use std::sync::LazyLock;

use axum::extract::Path;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

struct Asset {
    path: &'static str,
    content_type: &'static str,
    body: &'static [u8],
}

macro_rules! asset {
    ($path:literal, $content_type:literal) => {
        Asset {
            path: $path,
            content_type: $content_type,
            body: include_bytes!(concat!("../static/", $path)),
        }
    };
}

const ASSETS: &[Asset] = &[
    asset!("app.css", "text/css; charset=utf-8"),
    asset!("favicon.svg", "image/svg+xml"),
    asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
];

/// A strong validator per asset: the first 16 hex characters of its SHA-256.
static ETAGS: LazyLock<Vec<String>> = LazyLock::new(|| {
    ASSETS
        .iter()
        .map(|asset| format!("\"{}\"", &hex::encode(Sha256::digest(asset.body))[..16]))
        .collect()
});

/// `Cache-Control: no-cache` makes browsers revalidate, which costs a `304` and no body.
pub(crate) async fn serve(Path(path): Path<String>, headers: HeaderMap) -> Response {
    let Some(index) = ASSETS.iter().position(|asset| asset.path == path) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let asset = &ASSETS[index];
    let etag = ETAGS[index].as_str();
    let cache = [(ETAG, etag), (CACHE_CONTROL, "no-cache")];
    if headers
        .get(IF_NONE_MATCH)
        .is_some_and(|value| value.as_bytes() == etag.as_bytes())
    {
        return (StatusCode::NOT_MODIFIED, cache).into_response();
    }
    (cache, [(CONTENT_TYPE, asset.content_type)], asset.body).into_response()
}

pub(crate) async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    /// The published `dist/htmx.min.js` of htmx.org 2.0.11 on npm.
    const HTMX_SHA256: &str = "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";

    #[test]
    fn every_file_in_the_static_folder_is_served() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
        let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let listed: BTreeSet<String> = ASSETS.iter().map(|a| a.path.to_owned()).collect();
        assert_eq!(on_disk, listed);
    }

    #[test]
    fn the_vendored_htmx_is_the_published_release() {
        let htmx = ASSETS.iter().find(|a| a.path.starts_with("htmx-")).unwrap();
        assert_eq!(hex::encode(Sha256::digest(htmx.body)), HTMX_SHA256);
    }
}
```

- [ ] **Step 8: Wire the router**

Replace `crates/web/src/lib.rs` with:

```rust
//! The web layer of Smoke Tracker: Axum routes, Askama templates, Fluent translations and the
//! embedded static assets. It reaches the database only through the `smoke-domain` traits, so
//! every route works the same on SQLite and Postgres.

mod assets;
mod auth;
pub mod config;
mod request_id;
mod security;
mod sessions;
mod state;

use std::future::Future;
use std::net::SocketAddr;

use axum::Router;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::StatusCode;
use axum::middleware;
use axum::routing::get;
use smoke_domain::repos::Store;
use tokio::net::TcpListener;
use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::{DefaultOnResponse, TraceLayer};
use tracing::Level;

pub use crate::auth::password;
pub use crate::state::AppState;

/// Form posts are small; anything larger is refused before a handler runs.
const BODY_LIMIT_BYTES: usize = 16 * 1024;

/// The whole application. Static assets and the health check skip sessions and the database.
pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
    let pages = Router::new()
        .fallback(|| async { StatusCode::NOT_FOUND })
        .layer(sessions::layer(
            state.store.clone(),
            state.clock.clone(),
            &state.config.base_url,
        ))
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            security::check_origin,
        ))
        .with_state(state.clone());

    Router::new()
        .route("/static/{*path}", get(assets::serve))
        .route("/healthz", get(assets::healthz))
        .merge(pages)
        .layer(DefaultBodyLimit::max(BODY_LIMIT_BYTES))
        .layer(middleware::from_fn_with_state(
            state.config.base_url.is_https(),
            security::headers,
        ))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(request_span)
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(SetRequestIdLayer::x_request_id(request_id::RandomRequestId))
}

fn request_span(request: &Request) -> tracing::Span {
    let id = request_id::of(request.headers());
    tracing::info_span!("request", id, method = %request.method(), path = request.uri().path())
}

/// Serves `app` until `shutdown` completes, then finishes the requests in flight.
pub async fn serve(
    listener: TcpListener,
    app: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}
```

- [ ] **Step 9: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings`
Expected: all pass, including `every_response_carries_the_security_headers`,
`cross_site_posts_are_refused`, `the_vendored_htmx_is_the_published_release` and
`records_round_trip_through_the_stored_form`; no warnings.

- [ ] **Step 10: Commit**

```bash
git add Cargo.lock crates/web
git commit -m "feat(web): generic Axum router with DB sessions, security headers and embedded assets

AppState<S> carries the store as a type parameter, so each backend gets its
own monomorphised router. DbSessions implements tower-sessions' async
SessionStore over the domain SessionRepo; assets are include_bytes!-ed with
ETags; every response gets a strict CSP and a request id.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Web — translated error pages, i18n and the base layout

Fluent translations for en/it/tr embedded with `fluent-templates`, the language negotiation
(account → `lang` cookie → `Accept-Language` → English), the base layout, and `AppError`, which
every handler returns and a middleware turns into a translated page. Internal errors are logged
with their cause chain; the user sees only the request id.

**Files:**
- Create: `crates/web/locales/{en,it,tr}/main.ftl`
- Create: `crates/web/src/{i18n,error,cookies}.rs`, `crates/web/src/routes/mod.rs`
- Create: `crates/web/templates/{base,error}.html`
- Modify: `crates/web/src/{lib,security}.rs`
- Create: `crates/web/tests/errors.rs`; Modify: `crates/web/tests/security.rs`

**Interfaces:**
- Consumes: `request_id::of` (Task 4); `smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS}`
  and their `i18n_key`s (milestone 2); `Locale::{ALL, as_str}` and `FromStr`.
- Produces:
  - `crate::i18n::Tr(pub Locale)` — `lang()`, `t(key)`, `t1(key, arg_name, value)`; an axum
    extractor (from request extensions, English by default).
  - `crate::i18n::{LANG_COOKIE = "lang", negotiate(account: Option<Locale>, cookie:
    Option<&str>, accept_language: Option<&str>) -> Locale}`.
  - `crate::error::AppError::{NotFound, Forbidden, Internal(..)}`, `AppError::internal(e)`,
    `From` for `StoreError`, `askama::Error`, `tower_sessions::session::Error`, `JoinError`;
    `render_errors` middleware; `panic_response`; `#[cfg(test)] KEYS`.
  - `crate::cookies::{get(&HeaderMap, name) -> Option<String>, accept_language(&HeaderMap)}`.
  - Every page template has fields `tr: Tr` and `signed_in: Option<String>` (used by
    `base.html`).

- [ ] **Step 1: Write the translations**

`crates/web/locales/en/main.ftl`:

```ftl
app-name = Smoke Tracker
language = Language

login-title = Sign in
login-username = Username
login-password = Password
login-submit = Sign in
login-failed = Invalid username or password.
login-locked = Too many attempts. Try again in 15 minutes.
login-expired = This account has expired. Ask a resident to reactivate it.
logout = Sign out

home-title = Home
home-signed-in = Signed in as { $username }.
home-coming-soon = Logging smells arrives in the next version.

error-not-found-title = Page not found
error-not-found = This page does not exist.
error-forbidden-title = Request blocked
error-forbidden = This request did not come from this app, so it was blocked. Go back and try again.
error-internal-title = Something went wrong
error-internal = Something went wrong (ref: { $ref }).
error-back-home = Back to the start

room-living-room = Living room
room-kitchen = Kitchen
room-bedroom = Bedroom
room-bathroom = Bathroom
room-entrance-hall = Entrance hall
room-stairwell = Stairwell (shared)

symptom-cough = Cough
symptom-headache = Headache
symptom-sore-throat = Sore throat
symptom-eye-irritation = Eye irritation
symptom-nausea = Nausea
symptom-difficulty-sleeping = Difficulty sleeping
```

`crates/web/locales/it/main.ftl`:

```ftl
app-name = Smoke Tracker
language = Lingua

login-title = Accedi
login-username = Nome utente
login-password = Password
login-submit = Accedi
login-failed = Nome utente o password non validi.
login-locked = Troppi tentativi. Riprova tra 15 minuti.
login-expired = Questo account è scaduto. Chiedi a un residente di riattivarlo.
logout = Esci

home-title = Home
home-signed-in = Accesso effettuato come { $username }.
home-coming-soon = La registrazione degli odori arriva nella prossima versione.

error-not-found-title = Pagina non trovata
error-not-found = Questa pagina non esiste.
error-forbidden-title = Richiesta bloccata
error-forbidden = Questa richiesta non proviene da questa app, quindi è stata bloccata. Torna indietro e riprova.
error-internal-title = Qualcosa è andato storto
error-internal = Qualcosa è andato storto (rif: { $ref }).
error-back-home = Torna all'inizio

room-living-room = Soggiorno
room-kitchen = Cucina
room-bedroom = Camera da letto
room-bathroom = Bagno
room-entrance-hall = Ingresso
room-stairwell = Vano scale (condominiale)

symptom-cough = Tosse
symptom-headache = Mal di testa
symptom-sore-throat = Mal di gola
symptom-eye-irritation = Irritazione agli occhi
symptom-nausea = Nausea
symptom-difficulty-sleeping = Difficoltà a dormire
```

`crates/web/locales/tr/main.ftl`:

```ftl
app-name = Smoke Tracker
language = Dil

login-title = Giriş yap
login-username = Kullanıcı adı
login-password = Şifre
login-submit = Giriş yap
login-failed = Kullanıcı adı veya şifre geçersiz.
login-locked = Çok fazla deneme. 15 dakika sonra tekrar deneyin.
login-expired = Bu hesabın süresi doldu. Yeniden etkinleştirmesi için bir ev sakinine başvurun.
logout = Çıkış yap

home-title = Ana sayfa
home-signed-in = { $username } olarak giriş yapıldı.
home-coming-soon = Koku kaydı bir sonraki sürümde geliyor.

error-not-found-title = Sayfa bulunamadı
error-not-found = Bu sayfa mevcut değil.
error-forbidden-title = İstek engellendi
error-forbidden = Bu istek bu uygulamadan gelmediği için engellendi. Geri dönüp tekrar deneyin.
error-internal-title = Bir şeyler ters gitti
error-internal = Bir şeyler ters gitti (ref: { $ref }).
error-back-home = Başa dön

room-living-room = Oturma odası
room-kitchen = Mutfak
room-bedroom = Yatak odası
room-bathroom = Banyo
room-entrance-hall = Antre
room-stairwell = Merdiven boşluğu (ortak)

symptom-cough = Öksürük
symptom-headache = Baş ağrısı
symptom-sore-throat = Boğaz ağrısı
symptom-eye-irritation = Göz tahrişi
symptom-nausea = Mide bulantısı
symptom-difficulty-sleeping = Uyku güçlüğü
```

The keys for later screens (login, home) are added now so the files change once; the Italian and
Turkish strings need a native speaker's review (spec §9.2).

- [ ] **Step 2: Write the failing tests**

Create `crates/web/tests/errors.rs`:

```rust
//! Error pages: translated, and with the request id for internal errors (spec §12).

mod common;

use axum::http::StatusCode;
use common::*;

#[tokio::test]
async fn unknown_pages_get_a_translated_not_found_page() {
    let app = TestApp::new().await;
    let response = app.send(get("/nothing-here")).await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response.headers()["content-type"],
        "text/html; charset=utf-8"
    );
    assert!(text(response).await.contains("<h1>Page not found</h1>"));

    let italian = app.send(get_with("/nothing-here", "lang=it")).await;
    assert!(text(italian).await.contains("<h1>Pagina non trovata</h1>"));

    let turkish = app
        .send(
            axum::http::Request::get("/nothing-here")
                .header("accept-language", "tr-TR,tr;q=0.9")
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await;
    assert!(text(turkish).await.contains("<h1>Sayfa bulunamadı</h1>"));
}
```

Update `crates/web/tests/security.rs` — a refused request now gets a page:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -45,7 +45,7 @@ async fn anonymous_requests_set_no_cookies() {
 }
 
 #[tokio::test]
-async fn cross_site_posts_are_refused() {
+async fn cross_site_posts_are_refused_with_a_page() {
     let app = TestApp::new().await;
     let cases: [&[(&str, &str)]; 4] = [
         &[("sec-fetch-site", "cross-site"), ("origin", BASE)],
@@ -65,6 +65,10 @@ async fn cross_site_posts_are_refused() {
         let response = app.send(request).await;
         assert_eq!(response.status(), StatusCode::FORBIDDEN, "{headers:?}");
         assert!(session_cookie(&response).is_none());
+        assert!(
+            text(response).await.contains("<h1>Request blocked</h1>"),
+            "{headers:?}"
+        );
     }
 }
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test errors --test security`
Expected: FAIL — the 404 and 403 responses have empty bodies.

- [ ] **Step 4: Implement cookies and i18n**

`crates/web/src/cookies.rs`:

```rust
//! Reading cookies from a request.

use axum::http::HeaderMap;
use axum::http::header::{ACCEPT_LANGUAGE, COOKIE};
use tower_sessions::cookie::Cookie;

/// The value of the first cookie called `name`, across all `Cookie` headers.
pub(crate) fn get(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(Cookie::split_parse)
        .filter_map(Result::ok)
        .find(|cookie| cookie.name() == name)
        .map(|cookie| cookie.value().to_owned())
}

pub(crate) fn accept_language(headers: &HeaderMap) -> Option<&str> {
    headers.get(ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok())
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn cookies_are_found_across_headers() {
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, HeaderValue::from_static("a=1; lang=it"));
        headers.append(COOKIE, HeaderValue::from_static("b=2"));
        assert_eq!(get(&headers, "lang").as_deref(), Some("it"));
        assert_eq!(get(&headers, "b").as_deref(), Some("2"));
        assert_eq!(get(&headers, "missing"), None);
    }
}
```

`crates/web/src/i18n.rs` (its tests check that every language has exactly the English keys,
that every key a template or error page uses exists, and the negotiation order):

```rust
//! Translations (Fluent, one `.ftl` file per language, embedded at compile time) and the choice
//! of language for a request.

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use fluent_templates::fluent_bundle::FluentValue;
use fluent_templates::{LanguageIdentifier, Loader, langid, static_loader};
use smoke_domain::types::Locale;

static_loader! {
    static LOCALES = {
        locales: "./locales",
        fallback_language: "en",
        customise: |bundle| bundle.set_use_isolating(false),
    };
}

static EN: LanguageIdentifier = langid!("en");
static IT: LanguageIdentifier = langid!("it");
static TR: LanguageIdentifier = langid!("tr");

fn language(locale: Locale) -> &'static LanguageIdentifier {
    match locale {
        Locale::En => &EN,
        Locale::It => &IT,
        Locale::Tr => &TR,
    }
}

/// Translates for one language. Templates call `tr.t("key")`; a key missing from a language
/// falls back to English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tr(pub Locale);

impl Tr {
    pub(crate) fn lang(&self) -> &'static str {
        self.0.as_str()
    }

    pub(crate) fn t(&self, key: &str) -> String {
        LOCALES.lookup(language(self.0), key)
    }

    /// A message with one argument, e.g. `tr.t1("home-signed-in", "username", name)`.
    pub(crate) fn t1(&self, key: &str, name: &str, value: &str) -> String {
        let args: HashMap<Cow<'static, str>, FluentValue> = HashMap::from([(
            Cow::Owned(name.to_owned()),
            FluentValue::from(value.to_owned()),
        )]);
        LOCALES.lookup_with_args(language(self.0), key, &args)
    }
}

/// The language chosen by [`crate::auth::load_user`]; English if that middleware did not run.
impl<S: Send + Sync> FromRequestParts<S> for Tr {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<Tr>()
            .copied()
            .unwrap_or(Tr(Locale::En)))
    }
}

/// The cookie that remembers the language before sign-in (spec §9.2).
pub(crate) const LANG_COOKIE: &str = "lang";

/// The account's language when signed in, else the `lang` cookie, else the browser's
/// `Accept-Language`, else English. Unknown values are skipped, never an error.
pub(crate) fn negotiate(
    account: Option<Locale>,
    cookie: Option<&str>,
    accept_language: Option<&str>,
) -> Locale {
    account
        .or_else(|| cookie.and_then(|c| c.trim().parse().ok()))
        .or_else(|| accept_language.and_then(from_accept_language))
        .unwrap_or(Locale::En)
}

/// The supported language with the highest quality; the first one listed wins a tie.
fn from_accept_language(header: &str) -> Option<Locale> {
    let mut best: Option<(f32, Locale)> = None;
    for item in header.split(',') {
        let mut parts = item.split(';');
        let tag = parts.next().unwrap_or_default().trim();
        let quality = parts
            .find_map(|p| p.trim().strip_prefix("q="))
            .map_or(Some(1.0), |q| q.trim().parse::<f32>().ok())
            .unwrap_or(0.0);
        let primary = tag
            .split('-')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if let Ok(locale) = primary.parse::<Locale>()
            && quality > 0.0
            && best.is_none_or(|(q, _)| quality > q)
        {
            best = Some((quality, locale));
        }
    }
    best.map(|(_, locale)| locale)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;

    use fluent_syntax::ast::Entry;
    use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS};

    use super::*;
    use crate::error;

    const DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn message_ids(locale: Locale) -> BTreeSet<String> {
        let path = format!("{DIR}/locales/{locale}/main.ftl");
        let source = fs::read_to_string(&path).unwrap();
        let resource = fluent_syntax::parser::parse(source.as_str())
            .unwrap_or_else(|(_, errors)| panic!("{path}: {errors:?}"));
        resource
            .body
            .iter()
            .filter_map(|entry| match entry {
                Entry::Message(message) => Some(message.id.name.to_owned()),
                _ => None,
            })
            .collect()
    }

    /// Every literal key in `tr.t("…")` and `tr.t1("…", …)` calls in the templates.
    fn template_keys() -> BTreeSet<String> {
        let mut keys = BTreeSet::new();
        for entry in fs::read_dir(format!("{DIR}/templates")).unwrap() {
            let text = fs::read_to_string(entry.unwrap().path()).unwrap();
            for call in ["t(\"", "t1(\""] {
                for (start, _) in text.match_indices(call) {
                    let rest = &text[start + call.len()..];
                    keys.insert(rest[..rest.find('"').unwrap()].to_owned());
                }
            }
        }
        keys
    }

    #[test]
    fn every_language_has_exactly_the_english_keys() {
        let english = message_ids(Locale::En);
        for locale in Locale::ALL {
            let ids = message_ids(*locale);
            let missing: Vec<_> = english.difference(&ids).collect();
            let extra: Vec<_> = ids.difference(&english).collect();
            assert!(
                missing.is_empty() && extra.is_empty(),
                "{locale}: missing {missing:?}, not in English {extra:?}"
            );
        }
    }

    #[test]
    fn the_starter_lists_are_translated() {
        let english = message_ids(Locale::En);
        let keys = STARTER_ROOMS
            .iter()
            .map(|room| room.i18n_key)
            .chain(STARTER_SYMPTOMS.iter().map(|symptom| symptom.i18n_key));
        for key in keys {
            assert!(english.contains(key), "{key} has no translation");
        }
    }

    #[test]
    fn every_key_the_pages_use_is_translated() {
        let mut used = template_keys();
        assert!(used.contains("app-name"), "the template scan found nothing");
        used.extend(error::KEYS.map(str::to_owned));
        let english = message_ids(Locale::En);
        let missing: Vec<_> = used.difference(&english).collect();
        assert!(
            missing.is_empty(),
            "keys without a translation: {missing:?}"
        );
    }

    #[test]
    fn there_is_a_folder_for_each_supported_language_and_no_other() {
        let folders: BTreeSet<String> = fs::read_dir(format!("{DIR}/locales"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        let supported: BTreeSet<String> =
            Locale::ALL.iter().map(|l| l.as_str().to_owned()).collect();
        assert_eq!(folders, supported);
    }

    #[test]
    fn the_account_language_wins_then_the_cookie_then_the_browser() {
        assert_eq!(
            negotiate(Some(Locale::Tr), Some("it"), Some("en")),
            Locale::Tr
        );
        assert_eq!(negotiate(None, Some("it"), Some("tr")), Locale::It);
        assert_eq!(negotiate(None, None, Some("tr")), Locale::Tr);
        assert_eq!(negotiate(None, None, None), Locale::En);
    }

    #[test]
    fn unusable_values_fall_through_to_the_next_source() {
        assert_eq!(negotiate(None, Some("fr"), Some("it-IT")), Locale::It);
        assert_eq!(
            negotiate(None, Some(""), Some("de-DE,fr;q=0.9")),
            Locale::En
        );
        assert_eq!(negotiate(None, Some("IT"), None), Locale::En);
    }

    #[test]
    fn accept_language_uses_the_primary_tag_and_quality() {
        assert_eq!(
            from_accept_language("it-IT,it;q=0.9,en;q=0.8"),
            Some(Locale::It)
        );
        assert_eq!(from_accept_language("tr-TR"), Some(Locale::Tr));
        assert_eq!(
            from_accept_language("de-DE, en;q=0.5, it;q=0.8"),
            Some(Locale::It)
        );
        assert_eq!(from_accept_language("EN-us"), Some(Locale::En));
        assert_eq!(from_accept_language("it;q=0, tr;q=0.1"), Some(Locale::Tr));
        assert_eq!(from_accept_language("en, it"), Some(Locale::En));
        assert_eq!(from_accept_language("*"), None);
        assert_eq!(from_accept_language("it;q=abc"), None);
        assert_eq!(from_accept_language(""), None);
    }

    #[test]
    fn messages_are_translated_and_take_arguments() {
        assert_eq!(Tr(Locale::En).t("login-title"), "Sign in");
        assert_eq!(Tr(Locale::It).t("login-title"), "Accedi");
        assert_eq!(Tr(Locale::Tr).t("login-title"), "Giriş yap");
        assert_eq!(
            Tr(Locale::En).t1("home-signed-in", "username", "daniele"),
            "Signed in as daniele."
        );
    }
}
```

- [ ] **Step 5: Implement the error pages and the layout**

`crates/web/src/error.rs`:

```rust
//! Errors a handler can return, and the translated error pages they become.

use std::any::Any;
use std::error::Error as StdError;

use askama::Template;
use axum::body::Body;
use axum::extract::Request;
use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use smoke_domain::repos::StoreError;

use crate::cookies;
use crate::i18n::{LANG_COOKIE, Tr, negotiate};
use crate::request_id;

#[derive(Debug)]
pub(crate) enum AppError {
    NotFound,
    Forbidden,
    /// Anything unexpected. Logged with the request id; the user sees only the id.
    Internal(Box<dyn StdError + Send + Sync>),
}

impl AppError {
    pub(crate) fn internal(error: impl Into<Box<dyn StdError + Send + Sync>>) -> Self {
        Self::Internal(error.into())
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<StoreError> for AppError {
    fn from(error: StoreError) -> Self {
        Self::internal(error)
    }
}

impl From<askama::Error> for AppError {
    fn from(error: askama::Error) -> Self {
        Self::internal(error)
    }
}

impl From<tower_sessions::session::Error> for AppError {
    fn from(error: tower_sessions::session::Error) -> Self {
        Self::internal(error)
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(error: tokio::task::JoinError) -> Self {
        Self::internal(error)
    }
}

/// Marks a response whose body [`render_errors`] should replace with a translated page.
#[derive(Debug, Clone, Copy)]
struct ErrorPage(StatusCode);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = self.status();
        if let Self::Internal(error) = &self {
            tracing::error!(error = %chain(error.as_ref()), "request failed");
        }
        let mut response = status.into_response();
        response.extensions_mut().insert(ErrorPage(status));
        response
    }
}

/// The error and all its sources, e.g. `database error: no such table: users`.
fn chain(error: &(dyn StdError + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

/// For `CatchPanicLayer`: a panicking handler becomes an ordinary internal error.
pub(crate) fn panic_response(panic: Box<dyn Any + Send + 'static>) -> Response {
    let message = panic
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "a handler panicked".to_owned());
    AppError::internal(message).into_response()
}

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorTemplate {
    tr: Tr,
    signed_in: Option<String>,
    title_key: &'static str,
    message_key: &'static str,
    request_id: String,
    internal: bool,
}

/// Replaces the body of every [`AppError`] response with a translated page. The language comes
/// from the `lang` cookie or the browser; sign-in sets the cookie to the account's language.
pub(crate) async fn render_errors(request: Request, next: Next) -> Response {
    let tr = Tr(negotiate(
        None,
        cookies::get(request.headers(), LANG_COOKIE).as_deref(),
        cookies::accept_language(request.headers()),
    ));
    let request_id = request_id::of(request.headers()).to_owned();
    let response = next.run(request).await;
    let Some(ErrorPage(status)) = response.extensions().get::<ErrorPage>().copied() else {
        return response;
    };
    let (title_key, message_key) = match status {
        StatusCode::NOT_FOUND => ("error-not-found-title", "error-not-found"),
        StatusCode::FORBIDDEN => ("error-forbidden-title", "error-forbidden"),
        _ => ("error-internal-title", "error-internal"),
    };
    let page = ErrorTemplate {
        tr,
        signed_in: None,
        title_key,
        message_key,
        request_id,
        internal: status.is_server_error(),
    };
    let html = match page.render() {
        Ok(html) => html,
        Err(error) => {
            tracing::error!(%error, "the error page failed to render");
            return (status, "Something went wrong").into_response();
        }
    };
    let (mut parts, _) = response.into_parts();
    parts.headers.remove(CONTENT_LENGTH);
    parts.headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    Response::from_parts(parts, Body::from(html))
}

/// Every message key the error pages use, for the translation tests.
#[cfg(test)]
pub(crate) const KEYS: [&str; 6] = [
    "error-not-found-title",
    "error-not-found",
    "error-forbidden-title",
    "error-forbidden",
    "error-internal-title",
    "error-internal",
];
```

`crates/web/src/routes/mod.rs`:

```rust
//! One module per feature.

use crate::error::AppError;

pub(crate) async fn not_found() -> AppError {
    AppError::NotFound
}
```

`crates/web/templates/base.html` — the `htmx-config` meta tag stops htmx from injecting its
indicator `<style>` and from using `eval`, both of which the CSP would block:

```html
<!doctype html>
<html lang="{{ tr.lang() }}">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>
  <title>{% block title %}{% endblock %} · {{ tr.t("app-name") }}</title>
  <link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
  <link rel="stylesheet" href="/static/app.css">
  <script src="/static/htmx-2.0.11.min.js" defer></script>
</head>
<body>
  <header class="bar">
    <span class="brand">{{ tr.t("app-name") }}</span>
    {% if signed_in.is_some() %}
    <form method="post" action="/logout">
      <button type="submit" class="link">{{ tr.t("logout") }}</button>
    </form>
    {% endif %}
  </header>
  <main>
    {% block content %}{% endblock %}
  </main>
  <footer>
    <form method="post" action="/lang" class="languages">
      <span>{{ tr.t("language") }}:</span>
      <button type="submit" name="lang" value="en" lang="en" class="link">English</button>
      <button type="submit" name="lang" value="it" lang="it" class="link">Italiano</button>
      <button type="submit" name="lang" value="tr" lang="tr" class="link">Türkçe</button>
    </form>
  </footer>
</body>
</html>
```

`crates/web/templates/error.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t(title_key) }}{% endblock %}
{% block content %}
<h1>{{ tr.t(title_key) }}</h1>
{% if internal %}
<p>{{ tr.t1(message_key, "ref", request_id) }}</p>
{% else %}
<p>{{ tr.t(message_key) }}</p>
{% endif %}
<p><a href="/">{{ tr.t("error-back-home") }}</a></p>
{% endblock %}
```

- [ ] **Step 6: Wire it**

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -5,7 +5,11 @@
 mod assets;
 mod auth;
 pub mod config;
+mod cookies;
+mod error;
+mod i18n;
 mod request_id;
+mod routes;
 mod security;
 mod sessions;
 mod state;
@@ -15,11 +19,11 @@ use std::net::SocketAddr;
 
 use axum::Router;
 use axum::extract::{DefaultBodyLimit, Request};
-use axum::http::StatusCode;
 use axum::middleware;
 use axum::routing::get;
 use smoke_domain::repos::Store;
 use tokio::net::TcpListener;
+use tower_http::catch_panic::CatchPanicLayer;
 use tower_http::request_id::{PropagateRequestIdLayer, SetRequestIdLayer};
 use tower_http::trace::{DefaultOnResponse, TraceLayer};
 use tracing::Level;
@@ -33,7 +37,8 @@ const BODY_LIMIT_BYTES: usize = 16 * 1024;
 /// The whole application. Static assets and the health check skip sessions and the database.
 pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
     let pages = Router::new()
-        .fallback(|| async { StatusCode::NOT_FOUND })
+        .fallback(routes::not_found)
+        .layer(CatchPanicLayer::custom(error::panic_response))
         .layer(sessions::layer(
             state.store.clone(),
             state.clock.clone(),
@@ -43,6 +48,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             state.config.clone(),
             security::check_origin,
         ))
+        .layer(middleware::from_fn(error::render_errors))
         .with_state(state.clone());
 
     Router::new()
```

```diff
--- a/crates/web/src/security.rs
+++ b/crates/web/src/security.rs
@@ -7,11 +7,12 @@ use axum::http::header::{
     CACHE_CONTROL, CONTENT_SECURITY_POLICY, ORIGIN, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
     X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
 };
-use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
+use axum::http::{HeaderMap, HeaderName, HeaderValue, Method};
 use axum::middleware::Next;
 use axum::response::{IntoResponse, Response};
 
 use crate::config::WebConfig;
+use crate::error::AppError;
 
 /// Same-origin resources only: no inline scripts or styles, no CDNs, no framing.
 pub(crate) const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; \
@@ -63,7 +64,7 @@ pub(crate) async fn check_origin(
         next.run(request).await
     } else {
         tracing::warn!("refused a cross-site request");
-        StatusCode::FORBIDDEN.into_response()
+        AppError::Forbidden.into_response()
     }
 }
```

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings`
Expected: all pass, including `every_language_has_exactly_the_english_keys`,
`the_starter_lists_are_translated`, `unusable_values_fall_through_to_the_next_source` and
`unknown_pages_get_a_translated_not_found_page`; no warnings.

- [ ] **Step 8: Commit**

```bash
git add crates/web
git commit -m "feat(web): Fluent i18n with a static_loader and translated error pages

Translations are embedded at compile time; the language comes from the
account, the lang cookie, Accept-Language, then English. AppError marks its
response with an extension that a middleware renders as a page, so handlers
stay free of presentation and internal details never reach the user.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Web — sign in, sign out and the signed-in user

`load_user` reads the session's user on every page request, signs out anyone who may no longer
sign in (an expired viewer, a deleted account), renews the 30-day lifetime at most daily, and
picks the page language. `/login` verifies the password off the async runtime, cycles the
session id, and aligns the `lang` cookie with the account. The landing page is a placeholder until
milestone 4.

**Files:**
- Modify: `crates/web/src/auth/mod.rs`, `crates/web/src/{cookies,error,i18n,lib,sessions}.rs`,
  `crates/web/src/routes/mod.rs`
- Create: `crates/web/src/routes/{home,login}.rs`, `crates/web/templates/{home,login}.html`
- Create: `crates/web/tests/{login,i18n}.rs`; Modify: `crates/web/tests/{errors,security}.rs`

**Interfaces:**
- Consumes: `UserRepo::{user, user_by_username}`, `User::actor().can_sign_in(now)`,
  `Username::new` (milestones 1–2); everything from Tasks 3–5.
- Produces:
  - `crate::auth::CurrentUser(pub User)` — extractor (redirects to `/login` when absent) and
    optional extractor; `load_user::<S>` and `require_user` middleware.
  - `crate::sessions::RENEWED_AT_KEY = "renewed_at"`.
  - `crate::routes::login::{form, submit::<S>, logout, Notice::{Failed, Expired}, Notice::key}`.
  - `crate::cookies::lang(Locale, secure: bool) -> HeaderValue`.
  - `crate::error::page(StatusCode, &impl Template) -> Result<Response, AppError>`.
  - Routes: `GET /` (signed in), `GET|POST /login`, `POST /logout`.

- [ ] **Step 1: Write the failing tests**

Create `crates/web/tests/login.rs`:

```rust
//! Signing in and out, the failure limits and session lifetime (spec §8.1, §8.4).

mod common;

use axum::http::StatusCode;
use chrono::TimeDelta;
use common::*;
use smoke_domain::records::PasswordHash;
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, Role};

#[tokio::test]
async fn anonymous_visitors_are_sent_to_the_login_page() {
    let app = TestApp::new().await;
    let response = app.send(get("/")).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/login"));

    let login = app.send(get("/login")).await;
    assert_eq!(login.status(), StatusCode::OK);
    let html = text(login).await;
    assert!(html.contains("<h1>Sign in</h1>"), "{html}");
}

#[tokio::test]
async fn htmx_requests_are_redirected_with_hx_redirect() {
    let app = TestApp::new().await;
    let mut request = get("/");
    request
        .headers_mut()
        .insert("hx-request", "true".parse().unwrap());
    let response = app.send(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["hx-redirect"], "/login");
}

#[tokio::test]
async fn signing_in_sets_a_host_prefixed_session_cookie() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let response = app.attempt("daniele", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response), Some("/"));
    let cookie = set_cookies(&response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session="))
        .expect("a __Host- session cookie");
    for flag in [
        "HttpOnly",
        "Secure",
        "SameSite=Lax",
        "Path=/",
        "Max-Age=2592000",
    ] {
        assert!(cookie.contains(flag), "{flag} missing: {cookie}");
    }
    assert!(!cookie.contains("Domain"), "{cookie}");
}

#[tokio::test]
async fn a_signed_in_user_sees_the_home_page() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(html.contains("Signed in as daniele."), "{html}");

    let login = app.send(get_with("/login", &cookie)).await;
    assert_eq!(location(&login), Some("/"));
}

#[tokio::test]
async fn usernames_are_matched_ignoring_case_and_spaces() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let response = app.attempt("  Daniele ", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
}

#[tokio::test]
async fn unknown_users_and_wrong_passwords_get_the_same_answer() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let wrong_password = app.attempt("daniele", "not the password").await;
    let unknown_user = app.attempt("nobody", "not the password").await;
    let invalid_name = app.attempt("no such user!", "not the password").await;
    assert_eq!(wrong_password.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_user.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(invalid_name.status(), StatusCode::UNAUTHORIZED);
    let wrong_password = text(wrong_password).await.replace("daniele", "NAME");
    let unknown_user = text(unknown_user).await.replace("nobody", "NAME");
    assert!(wrong_password.contains("Invalid username or password."));
    assert_eq!(wrong_password, unknown_user);
    assert!(
        text(invalid_name)
            .await
            .contains("Invalid username or password.")
    );
}

#[tokio::test]
async fn an_expired_viewer_cannot_sign_in() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    app.clock.advance(TimeDelta::days(7));
    let response = app.attempt("lawyer", PASSWORD).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(session_cookie(&response).is_none());
    assert!(text(response).await.contains("This account has expired."));

    let wrong = app.attempt("lawyer", "not the password").await;
    assert!(text(wrong).await.contains("Invalid username or password."));
}

#[tokio::test]
async fn an_expired_guest_can_still_sign_in() {
    let app = TestApp::new().await;
    app.add_user("guest", Role::Guest, Locale::En).await;
    app.clock.advance(TimeDelta::days(8));
    let cookie = app.sign_in("guest").await;
    assert_eq!(
        app.send(get_with("/", &cookie)).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn a_viewer_who_expires_mid_session_is_signed_out() {
    let app = TestApp::new().await;
    app.add_user("lawyer", Role::Viewer, Locale::En).await;
    let cookie = app.sign_in("lawyer").await;
    assert_eq!(
        app.send(get_with("/", &cookie)).await.status(),
        StatusCode::OK
    );
    app.clock.advance(TimeDelta::days(7));
    let response = app.send(get_with("/", &cookie)).await;
    assert_eq!(location(&response), Some("/login"));
}

#[tokio::test]
async fn signing_out_ends_the_session() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let response = app.send(post_form("/logout", "", Some(&cookie))).await;
    assert_eq!(location(&response), Some("/login"));
    let cleared = set_cookies(&response)
        .into_iter()
        .find(|c| c.starts_with("__Host-session="))
        .expect("the cookie is cleared");
    assert!(cleared.contains("Max-Age=0"), "{cleared}");
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}

#[tokio::test]
async fn every_sign_in_gets_a_fresh_session_id() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let first = app.sign_in("daniele").await;
    let mut again = login_request("daniele", PASSWORD);
    again.headers_mut().insert("cookie", first.parse().unwrap());
    let second = session_cookie(&app.send(again).await).unwrap();
    assert_ne!(first, second);
    assert_eq!(
        location(&app.send(get_with("/", &first)).await),
        Some("/login")
    );
    assert_eq!(
        app.send(get_with("/", &second)).await.status(),
        StatusCode::OK
    );
}

#[tokio::test]
async fn an_idle_session_ends_after_thirty_days() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    app.clock
        .advance(TimeDelta::days(30) + TimeDelta::minutes(1));
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}

#[tokio::test]
async fn an_active_session_is_renewed_at_most_daily() {
    let app = TestApp::new().await;
    app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let same_day = app.send(get_with("/", &cookie)).await;
    assert!(session_cookie(&same_day).is_none(), "no write within a day");
    app.clock.advance(TimeDelta::days(1));
    let next_day = app.send(get_with("/", &cookie)).await;
    assert_eq!(session_cookie(&next_day), Some(cookie));
}

#[tokio::test]
async fn a_password_change_signs_the_user_out() {
    let app = TestApp::new().await;
    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
    let cookie = app.sign_in("daniele").await;
    let mut tx = app.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, PasswordHash::new("$argon2id$other".to_owned()))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        location(&app.send(get_with("/", &cookie)).await),
        Some("/login")
    );
}
```

Create `crates/web/tests/i18n.rs`:

```rust
//! Which language a page is in (spec §9.2).

mod common;

use axum::body::Body;
use axum::http::Request;
use common::*;
use smoke_domain::types::{Locale, Role};

async fn heading(app: &TestApp, request: Request<Body>) -> String {
    let html = text(app.send(request).await).await;
    let start = html.find("<h1>").unwrap() + 4;
    html[start..start + html[start..].find("</h1>").unwrap()].to_owned()
}

#[tokio::test]
async fn before_sign_in_the_lang_cookie_then_the_browser_decide() {
    let app = TestApp::new().await;
    assert_eq!(heading(&app, get("/login")).await, "Sign in");
    assert_eq!(heading(&app, get_with("/login", "lang=it")).await, "Accedi");
    let browser = Request::get("/login")
        .header("accept-language", "tr-TR,tr;q=0.9,en;q=0.8")
        .body(Body::empty())
        .unwrap();
    assert_eq!(heading(&app, browser).await, "Giriş yap");
    let unknown_cookie = Request::get("/login")
        .header("cookie", "lang=fr")
        .header("accept-language", "it-IT")
        .body(Body::empty())
        .unwrap();
    assert_eq!(heading(&app, unknown_cookie).await, "Accedi");
}

#[tokio::test]
async fn once_signed_in_the_account_language_wins_over_the_cookie() {
    let app = TestApp::new().await;
    app.add_user("partner", Role::Resident, Locale::It).await;
    let response = app.attempt("partner", PASSWORD).await;
    assert!(
        set_cookies(&response)
            .iter()
            .any(|c| c.starts_with("lang=it;")),
        "sign-in aligns the cookie with the account"
    );
    let session = session_cookie(&response).unwrap();
    let cookies = format!("{session}; lang=tr");
    assert_eq!(heading(&app, get_with("/", &cookies)).await, "Home");
    let html = text(app.send(get_with("/", &cookies)).await).await;
    assert!(html.contains(r#"<html lang="it">"#), "{html}");
    assert!(html.contains("Accesso effettuato come partner."), "{html}");
}
```

Add to `crates/web/tests/errors.rs`:

```diff
--- a/crates/web/tests/errors.rs
+++ b/crates/web/tests/errors.rs
@@ -4,6 +4,8 @@ mod common;
 
 use axum::http::StatusCode;
 use common::*;
+use smoke_domain::types::{Locale, Role};
+use sqlx::SqlitePool;
 
 #[tokio::test]
 async fn unknown_pages_get_a_translated_not_found_page() {
@@ -29,3 +31,28 @@ async fn unknown_pages_get_a_translated_not_found_page() {
         .await;
     assert!(text(turkish).await.contains("<h1>Sayfa bulunamadı</h1>"));
 }
+
+#[tokio::test]
+async fn an_internal_error_shows_the_request_id_and_nothing_else() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let pool = SqlitePool::connect(&app.db_url).await.unwrap();
+    sqlx::raw_sql("ALTER TABLE users RENAME TO users_gone")
+        .execute(&pool)
+        .await
+        .unwrap();
+
+    let response = app.send(get_with("/", &cookie)).await;
+    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
+    let request_id = response.headers()["x-request-id"]
+        .to_str()
+        .unwrap()
+        .to_owned();
+    let html = text(response).await;
+    assert!(
+        html.contains(&format!("Something went wrong (ref: {request_id}).")),
+        "{html}"
+    );
+    assert!(!html.contains("users"), "no internals leak: {html}");
+}
```

Add to `crates/web/tests/security.rs`:

```diff
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -6,6 +6,7 @@ use axum::body::Body;
 use axum::http::header::CONTENT_TYPE;
 use axum::http::{Request, StatusCode};
 use common::*;
+use smoke_domain::types::{Locale, Role};
 
 const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; \
     connect-src 'self'; font-src 'self'; manifest-src 'self'; worker-src 'self'; \
@@ -108,3 +109,76 @@ async fn assets_are_served_with_their_type_and_revalidated() {
     );
     assert_eq!(text(app.send(get("/healthz")).await).await, "ok");
 }
+
+#[tokio::test]
+async fn on_plain_http_localhost_there_is_no_hsts_and_no_secure_cookie() {
+    let app = TestApp::with_config("http://localhost:8080", None).await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let mut request = login_request("daniele", PASSWORD);
+    request
+        .headers_mut()
+        .insert("origin", "http://localhost:8080".parse().unwrap());
+    request.headers_mut().remove("sec-fetch-site");
+    let response = app.send(request).await;
+    assert_eq!(response.status(), StatusCode::SEE_OTHER);
+    assert!(!response.headers().contains_key("strict-transport-security"));
+    let cookie = set_cookies(&response)
+        .into_iter()
+        .find(|c| c.starts_with("session="))
+        .expect("a plain session cookie");
+    assert!(!cookie.contains("Secure"), "{cookie}");
+    assert!(cookie.contains("HttpOnly"), "{cookie}");
+}
+
+#[tokio::test]
+async fn the_origin_check_uses_the_configured_url_not_the_host_header() {
+    let app = TestApp::with_config("https://smoke.example.org/", None).await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let mut request = login_request("daniele", PASSWORD);
+    request.headers_mut().remove("sec-fetch-site");
+    request
+        .headers_mut()
+        .insert("host", "10.0.0.5:8080".parse().unwrap());
+    assert_eq!(app.send(request).await.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn no_page_needs_inline_scripts_styles_or_eval() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let pages = [
+        text(app.send(get("/login")).await).await,
+        text(app.send(get_with("/", &cookie)).await).await,
+        text(app.send(get("/missing")).await).await,
+        text(app.attempt("daniele", "not the password").await).await,
+    ];
+    for html in pages {
+        assert!(
+            html.contains(r#"<meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>"#),
+            "htmx must not inject styles or eval: {html}"
+        );
+        for forbidden in [
+            "<style",
+            " style=",
+            "<script>",
+            " onclick=",
+            " onload=",
+            " hx-on",
+            "javascript:",
+        ] {
+            assert!(!html.contains(forbidden), "{forbidden} in {html}");
+        }
+        for script in html.match_indices("<script").map(|(i, _)| &html[i..]) {
+            assert!(script.starts_with("<script src=\"/static/"), "{script}");
+        }
+    }
+}
+
+#[tokio::test]
+async fn oversized_form_posts_are_refused() {
+    let app = TestApp::new().await;
+    let body = format!("username=daniele&password={}", "x".repeat(20 * 1024));
+    let response = app.send(post_form("/login", &body, None)).await;
+    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test login`
Expected: FAIL — `/` answers 404 instead of redirecting to `/login`.

- [ ] **Step 3: Implement the signed-in user**

Replace `crates/web/src/auth/mod.rs` with:

```rust
//! Who is signed in: the session's user, checked against expiry on every request.

pub mod password;

use std::convert::Infallible;

use axum::extract::{FromRequestParts, OptionalFromRequestParts, Request, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::records::User;
use smoke_domain::repos::Store;
use smoke_domain::types::UserId;
use tower_sessions::Session;

use crate::cookies;
use crate::error::AppError;
use crate::i18n::{LANG_COOKIE, Tr, negotiate};
use crate::sessions::{RENEWED_AT_KEY, USER_ID_KEY};
use crate::state::AppState;

/// How often an active session's 30-day lifetime is extended (one database write).
const RENEW_AFTER: TimeDelta = TimeDelta::days(1);

/// The signed-in user, put in the request by [`load_user`].
#[derive(Debug, Clone)]
pub(crate) struct CurrentUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = Redirect;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or_else(|| Redirect::to("/login"))
    }
}

impl<S: Send + Sync> OptionalFromRequestParts<S> for CurrentUser {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Option<Self>, Self::Rejection> {
        Ok(parts.extensions.get::<CurrentUser>().cloned())
    }
}

/// Loads the session's user, signs out anyone who may no longer sign in (an expired viewer, a
/// vanished account), renews the session at most daily, and picks the page language.
pub(crate) async fn load_user<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let mut account_locale = None;
    if let Some(id) = session.get::<i64>(USER_ID_KEY).await? {
        match state.store.user(UserId(id)).await? {
            Some(user) if user.actor().can_sign_in(now) => {
                renew(&session, now).await?;
                account_locale = Some(user.locale);
                request.extensions_mut().insert(CurrentUser(user));
            }
            _ => session.flush().await?,
        }
    }
    let tr = Tr(negotiate(
        account_locale,
        cookies::get(request.headers(), LANG_COOKIE).as_deref(),
        cookies::accept_language(request.headers()),
    ));
    request.extensions_mut().insert(tr);
    Ok(next.run(request).await)
}

async fn renew(session: &Session, now: DateTime<Utc>) -> Result<(), AppError> {
    let renewed = session
        .get::<i64>(RENEWED_AT_KEY)
        .await?
        .and_then(DateTime::<Utc>::from_timestamp_micros);
    if renewed.is_none_or(|at| now - at >= RENEW_AFTER) {
        session
            .insert(RENEWED_AT_KEY, now.timestamp_micros())
            .await?;
    }
    Ok(())
}

/// Sends visitors who are not signed in to the login page; htmx requests get `HX-Redirect`
/// so the whole page navigates instead of swapping the login form into a fragment.
pub(crate) async fn require_user(request: Request, next: Next) -> Response {
    if request.extensions().get::<CurrentUser>().is_some() {
        next.run(request).await
    } else if request.headers().contains_key("hx-request") {
        (StatusCode::OK, [("hx-redirect", "/login")]).into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}
```

```diff
--- a/crates/web/src/sessions.rs
+++ b/crates/web/src/sessions.rs
@@ -22,13 +22,16 @@ use crate::config::BaseUrl;
 
 /// The signed-in user's id, as an `i64`.
 pub(crate) const USER_ID_KEY: &str = "user_id";
+/// When the session's lifetime was last extended, in microseconds since the Unix epoch.
+pub(crate) const RENEWED_AT_KEY: &str = "renewed_at";
 /// Over https the cookie is `__Host-` prefixed: Secure, `Path=/`, no `Domain`.
 pub(crate) const SECURE_COOKIE: &str = "__Host-session";
 /// Over plain http (localhost only) browsers refuse `Secure` cookies.
 pub(crate) const LOCAL_COOKIE: &str = "session";
 pub(crate) const LIFETIME_DAYS: i64 = 30;
 
-/// Sessions live 30 days from their last save.
+/// Sessions live 30 days from their last renewal; [`crate::auth::load_user`] renews at most
+/// daily, so an active session never expires and an idle one ends after a month.
 pub(crate) fn layer<S>(
     store: S,
     clock: Arc<dyn Clock>,
```

- [ ] **Step 4: Implement the pages**

```diff
--- a/crates/web/src/cookies.rs
+++ b/crates/web/src/cookies.rs
@@ -1,8 +1,12 @@
-//! Reading cookies from a request.
+//! Reading cookies from a request and building the `lang` cookie.
 
-use axum::http::HeaderMap;
 use axum::http::header::{ACCEPT_LANGUAGE, COOKIE};
-use tower_sessions::cookie::Cookie;
+use axum::http::{HeaderMap, HeaderValue};
+use smoke_domain::types::Locale;
+use tower_sessions::cookie::time::Duration;
+use tower_sessions::cookie::{Cookie, SameSite};
+
+use crate::i18n::LANG_COOKIE;
 
 /// The value of the first cookie called `name`, across all `Cookie` headers.
 pub(crate) fn get(headers: &HeaderMap, name: &str) -> Option<String> {
@@ -20,10 +24,20 @@ pub(crate) fn accept_language(headers: &HeaderMap) -> Option<&str> {
     headers.get(ACCEPT_LANGUAGE).and_then(|v| v.to_str().ok())
 }
 
+/// Remembers the language for a year. Not secret, but still `HttpOnly`: no script needs it.
+pub(crate) fn lang(locale: Locale, secure: bool) -> HeaderValue {
+    let cookie = Cookie::build((LANG_COOKIE, locale.as_str()))
+        .path("/")
+        .max_age(Duration::days(365))
+        .same_site(SameSite::Lax)
+        .http_only(true)
+        .secure(secure)
+        .build();
+    HeaderValue::from_str(&cookie.to_string()).expect("a cookie made of ASCII is a valid header")
+}
+
 #[cfg(test)]
 mod tests {
-    use axum::http::HeaderValue;
-
     use super::*;
 
     #[test]
@@ -35,4 +49,13 @@ mod tests {
         assert_eq!(get(&headers, "b").as_deref(), Some("2"));
         assert_eq!(get(&headers, "missing"), None);
     }
+
+    #[test]
+    fn the_lang_cookie_is_long_lived_and_follows_the_scheme() {
+        assert_eq!(
+            lang(Locale::Tr, true),
+            "lang=tr; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=31536000"
+        );
+        assert!(!lang(Locale::It, false).to_str().unwrap().contains("Secure"));
+    }
 }
```

```diff
--- a/crates/web/src/error.rs
+++ b/crates/web/src/error.rs
@@ -9,7 +9,7 @@ use axum::extract::Request;
 use axum::http::header::{CONTENT_LENGTH, CONTENT_TYPE};
 use axum::http::{HeaderValue, StatusCode};
 use axum::middleware::Next;
-use axum::response::{IntoResponse, Response};
+use axum::response::{Html, IntoResponse, Response};
 use smoke_domain::repos::StoreError;
 
 use crate::cookies;
@@ -153,6 +153,11 @@ pub(crate) async fn render_errors(request: Request, next: Next) -> Response {
     Response::from_parts(parts, Body::from(html))
 }
 
+/// Renders a page template with the given status.
+pub(crate) fn page(status: StatusCode, template: &impl Template) -> Result<Response, AppError> {
+    Ok((status, Html(template.render()?)).into_response())
+}
+
 /// Every message key the error pages use, for the translation tests.
 #[cfg(test)]
 pub(crate) const KEYS: [&str; 6] = [
```

`crates/web/src/routes/login.rs`:

```rust
//! Sign in and sign out (spec §8.1).

use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::repos::Store;
use smoke_domain::types::Username;
use tower_sessions::Session;

use crate::auth::{CurrentUser, password};
use crate::cookies;
use crate::error::{AppError, page};
use crate::i18n::Tr;
use crate::sessions::{RENEWED_AT_KEY, USER_ID_KEY};
use crate::state::AppState;

/// What the login page tells the user after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Notice {
    /// Wrong password or unknown username: deliberately the same message.
    Failed,
    /// Correct password, but a viewer account past its expiry.
    Expired,
}

impl Notice {
    #[cfg(test)]
    pub(crate) const ALL: [Notice; 2] = [Notice::Failed, Notice::Expired];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Notice::Failed => "login-failed",
            Notice::Expired => "login-expired",
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginPage {
    tr: Tr,
    signed_in: Option<String>,
    username: String,
    notice: Option<Notice>,
}

fn login_page(
    status: StatusCode,
    tr: Tr,
    username: String,
    notice: Option<Notice>,
) -> Result<Response, AppError> {
    page(
        status,
        &LoginPage {
            tr,
            signed_in: None,
            username,
            notice,
        },
    )
}

pub(crate) async fn form(user: Option<CurrentUser>, tr: Tr) -> Result<Response, AppError> {
    if user.is_some() {
        return Ok(Redirect::to("/").into_response());
    }
    login_page(StatusCode::OK, tr, String::new(), None)
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

/// Unknown usernames and wrong passwords get the same answer after the same work (one Argon2
/// verification), so neither the message nor the timing tells which accounts exist.
pub(crate) async fn submit<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    session: Session,
    tr: Tr,
    Form(form): Form<LoginForm>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let user = match Username::new(&form.username) {
        Ok(username) => state.store.user_by_username(&username).await?,
        Err(_) => None,
    };
    let stored = user.as_ref().and_then(|u| u.password_hash.clone());
    let attempt = form.password;
    let valid =
        tokio::task::spawn_blocking(move || password::verify(stored.as_ref(), &attempt)).await?;
    let Some(user) = user.filter(|_| valid) else {
        return login_page(
            StatusCode::UNAUTHORIZED,
            tr,
            form.username,
            Some(Notice::Failed),
        );
    };
    if !user.actor().can_sign_in(now) {
        return login_page(
            StatusCode::FORBIDDEN,
            tr,
            form.username,
            Some(Notice::Expired),
        );
    }
    session.cycle_id().await?;
    session.insert(USER_ID_KEY, user.id.0).await?;
    session
        .insert(RENEWED_AT_KEY, now.timestamp_micros())
        .await?;
    tracing::info!(user = user.id.0, "signed in");
    let mut response = Redirect::to("/").into_response();
    response.headers_mut().append(
        SET_COOKIE,
        cookies::lang(user.locale, state.config.base_url.is_https()),
    );
    Ok(response)
}

/// Deletes the session from the database and clears the cookie.
pub(crate) async fn logout(session: Session) -> Result<Redirect, AppError> {
    session.flush().await?;
    Ok(Redirect::to("/login"))
}
```

`crates/web/src/routes/home.rs`:

```rust
//! The signed-in landing page. A placeholder until the Log screen (milestone 4).

use askama::Template;
use axum::http::StatusCode;
use axum::response::Response;

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::i18n::Tr;

#[derive(Template)]
#[template(path = "home.html")]
struct HomePage {
    tr: Tr,
    signed_in: Option<String>,
    username: String,
}

pub(crate) async fn show(CurrentUser(user): CurrentUser, tr: Tr) -> Result<Response, AppError> {
    let username = user.username.to_string();
    page(
        StatusCode::OK,
        &HomePage {
            tr,
            signed_in: Some(username.clone()),
            username,
        },
    )
}
```

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,5 +1,8 @@
 //! One module per feature.
 
+pub(crate) mod home;
+pub(crate) mod login;
+
 use crate::error::AppError;
 
 pub(crate) async fn not_found() -> AppError {
```

`crates/web/templates/login.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("login-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("login-title") }}</h1>
{% if let Some(notice) = notice %}
<p class="notice" role="alert">{{ tr.t(notice.key()) }}</p>
{% endif %}
<form method="post" action="/login" class="stack">
  <label>{{ tr.t("login-username") }}
    <input name="username" value="{{ username }}" autocomplete="username"
           autocapitalize="none" autocorrect="off" spellcheck="false" maxlength="64" required>
  </label>
  <label>{{ tr.t("login-password") }}
    <input type="password" name="password" autocomplete="current-password" maxlength="128" required>
  </label>
  <button type="submit">{{ tr.t("login-submit") }}</button>
</form>
{% endblock %}
```

`crates/web/templates/home.html`:

```html
{% extends "base.html" %}
{% block title %}{{ tr.t("home-title") }}{% endblock %}
{% block content %}
<h1>{{ tr.t("home-title") }}</h1>
<p>{{ tr.t1("home-signed-in", "username", username) }}</p>
<p>{{ tr.t("home-coming-soon") }}</p>
{% endblock %}
```

The translation test must also see the login notices:

```diff
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -119,6 +119,7 @@ mod tests {
 
     use super::*;
     use crate::error;
+    use crate::routes::login::Notice;
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
 
@@ -183,6 +184,7 @@ mod tests {
         let mut used = template_keys();
         assert!(used.contains("app-name"), "the template scan found nothing");
         used.extend(error::KEYS.map(str::to_owned));
+        used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
```

- [ ] **Step 5: Wire the routes**

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -20,7 +20,7 @@ use std::net::SocketAddr;
 use axum::Router;
 use axum::extract::{DefaultBodyLimit, Request};
 use axum::middleware;
-use axum::routing::get;
+use axum::routing::{get, post};
 use smoke_domain::repos::Store;
 use tokio::net::TcpListener;
 use tower_http::catch_panic::CatchPanicLayer;
@@ -36,9 +36,22 @@ const BODY_LIMIT_BYTES: usize = 16 * 1024;
 
 /// The whole application. Static assets and the health check skip sessions and the database.
 pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
+    let signed_in = Router::new()
+        .route("/", get(routes::home::show))
+        .route_layer(middleware::from_fn(auth::require_user));
     let pages = Router::new()
+        .merge(signed_in)
+        .route(
+            "/login",
+            get(routes::login::form).post(routes::login::submit::<S>),
+        )
+        .route("/logout", post(routes::login::logout))
         .fallback(routes::not_found)
         .layer(CatchPanicLayer::custom(error::panic_response))
+        .layer(middleware::from_fn_with_state(
+            state.clone(),
+            auth::load_user::<S>,
+        ))
         .layer(sessions::layer(
             state.store.clone(),
             state.clock.clone(),
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings`
Expected: all pass, including `unknown_users_and_wrong_passwords_get_the_same_answer`,
`an_expired_viewer_cannot_sign_in`, `a_password_change_signs_the_user_out`,
`every_sign_in_gets_a_fresh_session_id`, `no_page_needs_inline_scripts_styles_or_eval` and
`the_origin_check_uses_the_configured_url_not_the_host_header`; no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/web
git commit -m "feat(auth): session sign-in with Argon2 on spawn_blocking and a CurrentUser extractor

load_user is a middleware that checks the account on every request, so an
expired viewer is signed out mid-session; CurrentUser implements both
FromRequestParts and OptionalFromRequestParts. Sign-in cycles the session id
against fixation and renews the 30-day lifetime at most once a day.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Web — sign-in limits

In memory, one instance (spec §8.1): 5 failures for a username within 15 minutes lock it for 15
minutes; 20 failures from one address within 15 minutes block that address for 15 minutes. The
username key is trimmed and lowercased like `Username::new`, so changing case cannot bypass a
lock. Behind a proxy, `CLIENT_IP_HEADER` names the client; the last entry of the header is used
because that is the one the trusted proxy wrote.

**Files:**
- Create: `crates/web/src/auth/{limiter,client_ip}.rs`
- Modify: `crates/web/src/auth/mod.rs`, `crates/web/src/routes/login.rs`, `crates/web/src/state.rs`
- Modify: `crates/web/tests/login.rs`

**Interfaces:**
- Consumes: `AppState` (Task 4), `routes::login` (Task 6), `WebConfig::client_ip_header`.
- Produces:
  - `crate::auth::limiter::LoginLimiter` — `key(&str) -> String`,
    `is_blocked(&self, key: &str, IpAddr, now) -> bool`,
    `record_failure(&self, key: String, IpAddr, now)`, `record_success(&self, key: &str)`;
    rules `PER_USERNAME`, `PER_ADDRESS`.
  - `crate::auth::client_ip::ClientIp(pub IpAddr)` — extractor over `AppState<S>`.
  - `AppState.limiter: Arc<LoginLimiter>`; `Notice::Locked` (`login-locked`, status 429).

- [ ] **Step 1: Write the failing tests**

Add to `crates/web/tests/login.rs`:

```diff
--- a/crates/web/tests/login.rs
+++ b/crates/web/tests/login.rs
@@ -217,3 +217,85 @@ async fn a_password_change_signs_the_user_out() {
         Some("/login")
     );
 }
+
+#[tokio::test]
+async fn five_failures_lock_the_username_for_fifteen_minutes() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    for _ in 0..5 {
+        app.attempt("daniele", "not the password").await;
+    }
+    let locked = app.attempt("daniele", PASSWORD).await;
+    assert_eq!(locked.status(), StatusCode::TOO_MANY_REQUESTS);
+    assert!(text(locked).await.contains("Too many attempts."));
+
+    app.clock.advance(TimeDelta::minutes(15));
+    let after = app.attempt("daniele", PASSWORD).await;
+    assert_eq!(after.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn changing_the_case_of_the_username_does_not_bypass_the_lock() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    for name in ["daniele", "Daniele", "DANIELE", " daniele", "daniele "] {
+        app.attempt(name, "not the password").await;
+    }
+    for name in ["daniele", "DaNiElE", "  DANIELE  "] {
+        let response = app.attempt(name, PASSWORD).await;
+        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{name:?}");
+    }
+}
+
+#[tokio::test]
+async fn a_success_resets_the_username_failures() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    for _ in 0..4 {
+        app.attempt("daniele", "not the password").await;
+    }
+    app.sign_in("daniele").await;
+    for _ in 0..4 {
+        app.attempt("daniele", "not the password").await;
+    }
+    assert_eq!(
+        app.attempt("daniele", PASSWORD).await.status(),
+        StatusCode::SEE_OTHER
+    );
+}
+
+#[tokio::test]
+async fn one_address_cannot_try_many_usernames() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    for n in 0..20 {
+        app.attempt(&format!("guess{n}"), "not the password").await;
+    }
+    let blocked = app.attempt("daniele", PASSWORD).await;
+    assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
+
+    let mut elsewhere = login_request("daniele", PASSWORD);
+    from(&mut elsewhere, "198.51.100.1".parse().unwrap());
+    assert_eq!(app.send(elsewhere).await.status(), StatusCode::SEE_OTHER);
+}
+
+#[tokio::test]
+async fn behind_a_proxy_the_configured_header_names_the_client() {
+    let app = TestApp::with_config(BASE, Some("x-forwarded-for")).await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let attempt = |client: &'static str, password: &str| {
+        let mut request = login_request("guess", password);
+        request
+            .headers_mut()
+            .insert("x-forwarded-for", client.parse().unwrap());
+        request
+    };
+    for _ in 0..20 {
+        app.send(attempt("203.0.113.1", "not the password")).await;
+    }
+    let mut other_client = login_request("daniele", PASSWORD);
+    other_client
+        .headers_mut()
+        .insert("x-forwarded-for", "203.0.113.2".parse().unwrap());
+    assert_eq!(app.send(other_client).await.status(), StatusCode::SEE_OTHER);
+}
```

Create `crates/web/src/auth/limiter.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn t0() -> DateTime<Utc> {
        "2026-01-15T20:00:00Z".parse().unwrap()
    }

    fn ip(last: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 0, 2, last))
    }

    #[test]
    fn the_fifth_failure_locks_the_username_for_fifteen_minutes() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        assert!(!limiter.is_blocked("daniele", ip(2), t0()));
        limiter.record_failure("daniele".into(), ip(1), t0());
        assert!(limiter.is_blocked("daniele", ip(2), t0()));
        assert!(limiter.is_blocked(
            "daniele",
            ip(2),
            t0() + TimeDelta::minutes(15) - TimeDelta::seconds(1)
        ));
        assert!(!limiter.is_blocked("daniele", ip(2), t0() + TimeDelta::minutes(15)));
        assert!(!limiter.is_blocked("partner", ip(2), t0()));
    }

    #[test]
    fn failures_older_than_the_window_are_forgotten() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        let later = t0() + TimeDelta::minutes(15);
        limiter.record_failure("daniele".into(), ip(1), later);
        assert!(!limiter.is_blocked("daniele", ip(1), later));
    }

    #[test]
    fn a_success_clears_the_username_count() {
        let limiter = LoginLimiter::default();
        for _ in 0..4 {
            limiter.record_failure("daniele".into(), ip(1), t0());
        }
        limiter.record_success("daniele");
        limiter.record_failure("daniele".into(), ip(1), t0());
        assert!(!limiter.is_blocked("daniele", ip(1), t0()));
    }

    #[test]
    fn twenty_failures_from_one_address_block_it_for_every_username() {
        let limiter = LoginLimiter::default();
        for n in 0..19 {
            limiter.record_failure(format!("user{n}"), ip(1), t0());
        }
        assert!(!limiter.is_blocked("anyone", ip(1), t0()));
        limiter.record_failure("user19".into(), ip(1), t0());
        assert!(limiter.is_blocked("anyone", ip(1), t0()));
        assert!(!limiter.is_blocked("anyone", ip(2), t0()));
    }

    #[test]
    fn keys_ignore_case_and_surrounding_spaces() {
        assert_eq!(LoginLimiter::key("  Daniele "), "daniele");
        assert_eq!(LoginLimiter::key("DANIELE"), LoginLimiter::key("daniele"));
        assert_eq!(LoginLimiter::key(&"x".repeat(500)).len(), MAX_KEY_CHARS);
    }

    #[test]
    fn finished_entries_are_pruned_once_there_are_many() {
        let limiter = LoginLimiter::default();
        for n in 0..=PRUNE_ABOVE {
            limiter.record_failure(format!("user{n}"), ip(1), t0());
        }
        let later = t0() + TimeDelta::hours(1);
        limiter.record_failure("one-more".into(), ip(3), later);
        assert_eq!(limiter.usernames.lock().len(), 1);
    }
}
```

Create `crates/web/src/auth/client_ip.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    const PEER: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));

    fn forwarded(value: &'static str) -> HeaderMap {
        HeaderMap::from_iter([(
            HeaderName::from_static("x-forwarded-for"),
            HeaderValue::from_static(value),
        )])
    }

    #[test]
    fn without_a_configured_header_the_peer_is_used() {
        assert_eq!(resolve(&forwarded("203.0.113.9"), None, Some(PEER)), PEER);
    }

    #[test]
    fn the_last_entry_of_the_configured_header_wins() {
        let name = HeaderName::from_static("x-forwarded-for");
        assert_eq!(
            resolve(
                &forwarded("198.51.100.7, 203.0.113.9"),
                Some(&name),
                Some(PEER)
            ),
            "203.0.113.9".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            resolve(&forwarded("garbage"), Some(&name), Some(PEER)),
            PEER
        );
        assert_eq!(
            resolve(&HeaderMap::new(), Some(&name), None),
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        );
    }
}
```

```diff
--- a/crates/web/src/auth/mod.rs
+++ b/crates/web/src/auth/mod.rs
@@ -1,5 +1,7 @@
 //! Who is signed in: the session's user, checked against expiry on every request.
 
+pub(crate) mod client_ip;
+pub(crate) mod limiter;
 pub mod password;
 
 use std::convert::Infallible;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: compile errors — `LoginLimiter` and `resolve` are not defined.

- [ ] **Step 3: Implement the limiter and the client address**

Put this above the tests in `crates/web/src/auth/limiter.rs`:

```rust
//! Failed sign-in limits, kept in memory (one instance; spec §8.1).
//!
//! - A username is locked for 15 minutes after 5 failures within 15 minutes.
//! - An address is blocked for 15 minutes after 20 failures within 15 minutes, whatever the
//!   usernames, so one client cannot try every account.

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;
use std::net::IpAddr;
use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rule {
    pub(crate) max_failures: u32,
    pub(crate) window: TimeDelta,
    pub(crate) lockout: TimeDelta,
}

pub(crate) const PER_USERNAME: Rule = Rule {
    max_failures: 5,
    window: TimeDelta::minutes(15),
    lockout: TimeDelta::minutes(15),
};

pub(crate) const PER_ADDRESS: Rule = Rule {
    max_failures: 20,
    window: TimeDelta::minutes(15),
    lockout: TimeDelta::minutes(15),
};

/// Above this many tracked keys, finished entries are dropped on the next failure, so random
/// usernames cannot grow memory without bound.
const PRUNE_ABOVE: usize = 1_000;
/// Usernames longer than this are cut for the key; real ones have at most 32 characters.
const MAX_KEY_CHARS: usize = 64;

#[derive(Debug, Clone, Copy)]
struct Strikes {
    failures: u32,
    since: DateTime<Utc>,
    locked_until: Option<DateTime<Utc>>,
}

impl Strikes {
    fn new(now: DateTime<Utc>) -> Self {
        Self {
            failures: 0,
            since: now,
            locked_until: None,
        }
    }

    fn is_locked(&self, now: DateTime<Utc>) -> bool {
        self.locked_until.is_some_and(|until| now < until)
    }

    fn is_finished(&self, rule: Rule, now: DateTime<Utc>) -> bool {
        !self.is_locked(now) && now >= self.since + rule.window
    }
}

#[derive(Debug)]
struct Counter<K> {
    rule: Rule,
    entries: Mutex<HashMap<K, Strikes>>,
}

impl<K: Eq + Hash> Counter<K> {
    fn new(rule: Rule) -> Self {
        Self {
            rule,
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn is_locked<Q>(&self, key: &Q, now: DateTime<Utc>) -> bool
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.lock().get(key).is_some_and(|s| s.is_locked(now))
    }

    fn strike(&self, key: K, now: DateTime<Utc>) {
        let rule = self.rule;
        let mut entries = self.lock();
        if entries.len() > PRUNE_ABOVE {
            entries.retain(|_, s| !s.is_finished(rule, now));
        }
        let strikes = entries.entry(key).or_insert_with(|| Strikes::new(now));
        if strikes.is_finished(rule, now) {
            *strikes = Strikes::new(now);
        }
        strikes.failures += 1;
        if strikes.failures >= rule.max_failures {
            *strikes = Strikes {
                locked_until: Some(now + rule.lockout),
                ..Strikes::new(now)
            };
        }
    }

    fn forget<Q>(&self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Eq + Hash + ?Sized,
    {
        self.lock().remove(key);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<K, Strikes>> {
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

#[derive(Debug)]
pub(crate) struct LoginLimiter {
    usernames: Counter<String>,
    addresses: Counter<IpAddr>,
}

impl Default for LoginLimiter {
    fn default() -> Self {
        Self {
            usernames: Counter::new(PER_USERNAME),
            addresses: Counter::new(PER_ADDRESS),
        }
    }
}

impl LoginLimiter {
    /// The username as typed, trimmed and lowercased like `Username::new`, so "Daniele " and
    /// "daniele" share one counter, and so do names that are not valid usernames at all.
    pub(crate) fn key(raw: &str) -> String {
        raw.trim()
            .to_lowercase()
            .chars()
            .take(MAX_KEY_CHARS)
            .collect()
    }

    pub(crate) fn is_blocked(&self, key: &str, address: IpAddr, now: DateTime<Utc>) -> bool {
        self.usernames.is_locked(key, now) || self.addresses.is_locked(&address, now)
    }

    pub(crate) fn record_failure(&self, key: String, address: IpAddr, now: DateTime<Utc>) {
        self.usernames.strike(key, now);
        self.addresses.strike(address, now);
    }

    /// A successful sign-in clears the username's failures; the address keeps its count.
    pub(crate) fn record_success(&self, key: &str) {
        self.usernames.forget(key);
    }
}
```

Put this above the tests in `crates/web/src/auth/client_ip.rs`:

```rust
//! The client's address for the per-address sign-in limit.

use std::convert::Infallible;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderName};

use crate::state::AppState;

/// The configured proxy header's address if there is one, else the TCP peer's address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientIp(pub IpAddr);

impl<S: Send + Sync> FromRequestParts<AppState<S>> for ClientIp {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState<S>,
    ) -> Result<Self, Self::Rejection> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|info| info.0.ip());
        Ok(Self(resolve(
            &parts.headers,
            state.config.client_ip_header.as_ref(),
            peer,
        )))
    }
}

/// For a list such as `X-Forwarded-For: client, proxy1`, the last entry is the one the trusted
/// proxy added; earlier entries are whatever the client claimed.
fn resolve(headers: &HeaderMap, header: Option<&HeaderName>, peer: Option<IpAddr>) -> IpAddr {
    header
        .and_then(|name| headers.get(name))
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.rsplit(',').next())
        .and_then(|value| value.trim().parse().ok())
        .or(peer)
        .unwrap_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED))
}
```

- [ ] **Step 4: Use them at sign-in**

```diff
--- a/crates/web/src/state.rs
+++ b/crates/web/src/state.rs
@@ -4,6 +4,7 @@ use std::sync::Arc;
 
 use smoke_domain::clock::Clock;
 
+use crate::auth::limiter::LoginLimiter;
 use crate::config::WebConfig;
 
 /// Shared application state. `S` is the storage backend, chosen once at startup; it is a type
@@ -12,6 +13,7 @@ pub struct AppState<S> {
     pub(crate) store: S,
     pub(crate) clock: Arc<dyn Clock>,
     pub(crate) config: Arc<WebConfig>,
+    pub(crate) limiter: Arc<LoginLimiter>,
 }
 
 impl<S> AppState<S> {
@@ -20,6 +22,7 @@ impl<S> AppState<S> {
             store,
             clock,
             config: Arc::new(config),
+            limiter: Arc::new(LoginLimiter::default()),
         }
     }
 }
@@ -30,6 +33,7 @@ impl<S: Clone> Clone for AppState<S> {
             store: self.store.clone(),
             clock: self.clock.clone(),
             config: self.config.clone(),
+            limiter: self.limiter.clone(),
         }
     }
 }
```

```diff
--- a/crates/web/src/routes/login.rs
+++ b/crates/web/src/routes/login.rs
@@ -11,6 +11,8 @@ use smoke_domain::repos::Store;
 use smoke_domain::types::Username;
 use tower_sessions::Session;
 
+use crate::auth::client_ip::ClientIp;
+use crate::auth::limiter::LoginLimiter;
 use crate::auth::{CurrentUser, password};
 use crate::cookies;
 use crate::error::{AppError, page};
@@ -23,17 +25,20 @@ use crate::state::AppState;
 pub(crate) enum Notice {
     /// Wrong password or unknown username: deliberately the same message.
     Failed,
+    /// Too many failures for this username or from this address.
+    Locked,
     /// Correct password, but a viewer account past its expiry.
     Expired,
 }
 
 impl Notice {
     #[cfg(test)]
-    pub(crate) const ALL: [Notice; 2] = [Notice::Failed, Notice::Expired];
+    pub(crate) const ALL: [Notice; 3] = [Notice::Failed, Notice::Locked, Notice::Expired];
 
     pub(crate) fn key(self) -> &'static str {
         match self {
             Notice::Failed => "login-failed",
+            Notice::Locked => "login-locked",
             Notice::Expired => "login-expired",
         }
     }
@@ -83,10 +88,20 @@ pub(crate) struct LoginForm {
 pub(crate) async fn submit<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     session: Session,
+    ClientIp(address): ClientIp,
     tr: Tr,
     Form(form): Form<LoginForm>,
 ) -> Result<Response, AppError> {
     let now = state.clock.now();
+    let key = LoginLimiter::key(&form.username);
+    if state.limiter.is_blocked(&key, address, now) {
+        return login_page(
+            StatusCode::TOO_MANY_REQUESTS,
+            tr,
+            form.username,
+            Some(Notice::Locked),
+        );
+    }
     let user = match Username::new(&form.username) {
         Ok(username) => state.store.user_by_username(&username).await?,
         Err(_) => None,
@@ -96,6 +111,7 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
     let valid =
         tokio::task::spawn_blocking(move || password::verify(stored.as_ref(), &attempt)).await?;
     let Some(user) = user.filter(|_| valid) else {
+        state.limiter.record_failure(key, address, now);
         return login_page(
             StatusCode::UNAUTHORIZED,
             tr,
@@ -103,6 +119,7 @@ pub(crate) async fn submit<S: Store + Clone + 'static>(
             Some(Notice::Failed),
         );
     };
+    state.limiter.record_success(&key);
     if !user.actor().can_sign_in(now) {
         return login_page(
             StatusCode::FORBIDDEN,
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings`
Expected: all pass, including `changing_the_case_of_the_username_does_not_bypass_the_lock`,
`one_address_cannot_try_many_usernames` and `behind_a_proxy_the_configured_header_names_the_client`;
no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/web
git commit -m "feat(auth): in-memory sign-in limits per username and per address

A Mutex<HashMap> per key type with a generic Counter<K: Eq + Hash>; lookups
take &Q via Borrow so no key is allocated to check a lock. Finished entries
are pruned once there are many, so random usernames cannot grow memory.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Web — choosing the language

`POST /lang` sets the `lang` cookie and, when signed in, saves the language to the account in an
audited transaction (spec §9.2). An unknown language changes nothing.

**Files:**
- Create: `crates/web/src/routes/lang.rs`
- Modify: `crates/web/src/routes/mod.rs`, `crates/web/src/lib.rs`, `crates/web/tests/i18n.rs`

**Interfaces:**
- Consumes: `UserWrites::set_locale`, `Transactional::begin`, `Commit` (milestone 2);
  `CurrentUser` (Task 6); `cookies::lang` (Task 6).
- Produces: route `POST /lang` (`routes::lang::choose::<S>`), redirecting to `/` when signed in,
  else `/login`.

- [ ] **Step 1: Write the failing tests**

Add to `crates/web/tests/i18n.rs`:

```diff
--- a/crates/web/tests/i18n.rs
+++ b/crates/web/tests/i18n.rs
@@ -5,6 +5,7 @@ mod common;
 use axum::body::Body;
 use axum::http::Request;
 use common::*;
+use smoke_domain::repos::UserRepo;
 use smoke_domain::types::{Locale, Role};
 
 async fn heading(app: &TestApp, request: Request<Body>) -> String {
@@ -49,3 +50,38 @@ async fn once_signed_in_the_account_language_wins_over_the_cookie() {
     assert!(html.contains(r#"<html lang="it">"#), "{html}");
     assert!(html.contains("Accesso effettuato come partner."), "{html}");
 }
+
+#[tokio::test]
+async fn choosing_a_language_sets_the_cookie() {
+    let app = TestApp::new().await;
+    let response = app.send(post_form("/lang", "lang=tr", None)).await;
+    assert_eq!(location(&response), Some("/login"));
+    assert!(
+        set_cookies(&response)
+            .iter()
+            .any(|c| c.starts_with("lang=tr;") && c.contains("Max-Age=31536000")),
+        "{:?}",
+        set_cookies(&response)
+    );
+    let ignored = app.send(post_form("/lang", "lang=xx", None)).await;
+    assert_eq!(location(&ignored), Some("/login"));
+    assert!(
+        set_cookies(&ignored)
+            .iter()
+            .all(|c| !c.starts_with("lang="))
+    );
+}
+
+#[tokio::test]
+async fn choosing_a_language_while_signed_in_saves_it_to_the_account() {
+    let app = TestApp::new().await;
+    let user = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let session = app.sign_in("daniele").await;
+    let response = app
+        .send(post_form("/lang", "lang=tr", Some(&session)))
+        .await;
+    assert_eq!(location(&response), Some("/"));
+    let saved = app.store.user(user.id).await.unwrap().unwrap();
+    assert_eq!(saved.locale, Locale::Tr);
+    assert_eq!(heading(&app, get_with("/", &session)).await, "Ana sayfa");
+}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test i18n`
Expected: FAIL — `POST /lang` answers 404.

- [ ] **Step 3: Implement**

`crates/web/src/routes/lang.rs`:

```rust
//! Choosing the interface language (spec §9.2): a cookie before sign-in, the account after.

use axum::Form;
use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use smoke_domain::repos::{Commit, Store, UserWrites};
use smoke_domain::types::Locale;

use crate::auth::CurrentUser;
use crate::cookies;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub(crate) struct LangForm {
    lang: String,
}

/// Sets the `lang` cookie and, when signed in, saves the language to the account (audited).
/// An unknown language changes nothing.
pub(crate) async fn choose<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    user: Option<CurrentUser>,
    Form(form): Form<LangForm>,
) -> Result<Response, AppError> {
    let target = if user.is_some() { "/" } else { "/login" };
    let Ok(locale) = form.lang.parse::<Locale>() else {
        return Ok(Redirect::to(target).into_response());
    };
    if let Some(CurrentUser(user)) = user
        && user.locale != locale
    {
        let mut tx = state.store.begin(Some(user.id)).await?;
        tx.set_locale(user.id, locale).await?;
        tx.commit().await?;
    }
    let mut response = Redirect::to(target).into_response();
    response.headers_mut().append(
        SET_COOKIE,
        cookies::lang(locale, state.config.base_url.is_https()),
    );
    Ok(response)
}
```

```diff
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -1,6 +1,7 @@
 //! One module per feature.
 
 pub(crate) mod home;
+pub(crate) mod lang;
 pub(crate) mod login;
 
 use crate::error::AppError;
```

```diff
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -46,6 +46,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
             get(routes::login::form).post(routes::login::submit::<S>),
         )
         .route("/logout", post(routes::login::logout))
+        .route("/lang", post(routes::lang::choose::<S>))
         .fallback(routes::not_found)
         .layer(CatchPanicLayer::custom(error::panic_response))
         .layer(middleware::from_fn_with_state(
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-web && cargo clippy -p smoke-web --all-targets -- -D warnings`
Expected: all pass, including `choosing_a_language_while_signed_in_saves_it_to_the_account`; no
warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/web
git commit -m "feat(i18n): language switch saved to the cookie and, when signed in, the account

The account update goes through the audited set_locale transaction; the
form uses let-else and a let-chain so an unknown language is a no-op.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: App crate — configuration and backend selection

The `smoke-tracker` package (library `smoke_app`): configuration from the environment
(spec §10.5) and the backend chosen from the `DATABASE_URL` scheme. Commands stay generic over
`Store`; the `with_store!` macro runs one with the concrete store, so each backend gets its own
monomorphised copy and no call goes through a trait object (extends decision #53).

**Files:**
- Create: `crates/app/Cargo.toml`, `crates/app/src/{lib,config,backend}.rs`,
  `crates/app/tests/commands.rs`
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: `SqliteStore::connect/migrate`, `PostgresStore::connect/migrate`,
  `seed_starter_lists` (milestone 2); `DailySchedule::{new, tz, hour}` (milestone 1);
  `BaseUrl`, `WebConfig` (Task 3).
- Produces:
  - `smoke_app::config::{env(&str) -> Option<String>, database_url(lookup) -> Result<String,
    ConfigError>, ServeConfig { web, schedule, bind_addr }, ServeConfig::from_lookup(lookup)}`;
    `ConfigError::{Missing(&'static str), Invalid { name, reason }}`. Lookups are
    `impl Fn(&str) -> Option<String>`, so tests never touch the process environment.
  - `smoke_app::backend::{BackendKind::of(&str), Backend::{Sqlite, Postgres},
    Backend::connect(url, clock), Backend::migrate, Backend::prepare}` and the crate-local macro
    `with_store!(backend, |store| expr)`.

- [ ] **Step 1: Create the crate with its failing tests**

`crates/app/Cargo.toml` (the `[[bin]]` target arrives with `main.rs` in Task 10):

```toml
[package]
name = "smoke-tracker"
version = "0.1.0"
description = "The Smoke Tracker binary: configuration, wiring, background tasks and the CLI"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[lib]
name = "smoke_app"

[dependencies]
anyhow.workspace = true
chrono-tz.workspace = true
clap.workspace = true
rpassword.workspace = true
smoke-domain.workspace = true
smoke-storage-postgres.workspace = true
smoke-storage-sqlite.workspace = true
smoke-web.workspace = true
thiserror.workspace = true
tokio = { workspace = true, features = ["net", "signal", "time"] }
tracing.workspace = true
tracing-subscriber.workspace = true

[dev-dependencies]
sqlx = { workspace = true, features = ["sqlite-bundled"] }
tempfile.workspace = true

[lints]
workspace = true
```

`crates/app/src/lib.rs`:

```rust
//! The Smoke Tracker binary as a library: configuration, backend selection, the commands and
//! the background tasks.

pub mod backend;
pub mod config;
```

`crates/app/src/config.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        move |name| map.get(name).cloned()
    }

    fn error_for(pairs: &[(&str, &str)]) -> ConfigError {
        let mut all = vec![("APP_BASE_URL", "https://smoke.example.org")];
        all.extend_from_slice(pairs);
        ServeConfig::from_lookup(lookup(&all)).unwrap_err()
    }

    #[test]
    fn only_the_base_url_is_required_to_serve() {
        let config =
            ServeConfig::from_lookup(lookup(&[("APP_BASE_URL", "https://smoke.example.org/")]))
                .unwrap();
        assert_eq!(config.web.base_url.origin(), "https://smoke.example.org");
        assert_eq!(config.schedule.tz(), chrono_tz::Europe::Rome);
        assert_eq!(config.schedule.hour(), 4);
        assert_eq!(config.bind_addr, "127.0.0.1:8080".parse().unwrap());
        assert_eq!(config.web.client_ip_header, None);
        assert_eq!(
            ServeConfig::from_lookup(lookup(&[])).unwrap_err(),
            ConfigError::Missing("APP_BASE_URL")
        );
    }

    #[test]
    fn every_variable_can_be_set() {
        let config = ServeConfig::from_lookup(lookup(&[
            ("APP_BASE_URL", "https://smoke.example.org"),
            ("APP_TIMEZONE", "Europe/Istanbul"),
            ("AUTO_CLOSE_HOUR", "5"),
            ("BIND_ADDR", "0.0.0.0:3000"),
            ("CLIENT_IP_HEADER", "Fly-Client-IP"),
        ]))
        .unwrap();
        assert_eq!(config.schedule.tz(), chrono_tz::Europe::Istanbul);
        assert_eq!(config.schedule.hour(), 5);
        assert_eq!(config.bind_addr, "0.0.0.0:3000".parse().unwrap());
        assert_eq!(config.web.client_ip_header.unwrap(), "fly-client-ip");
    }

    #[test]
    fn invalid_values_name_the_variable() {
        for (pairs, name) in [
            (
                &[("APP_BASE_URL", "http://192.168.1.2")][..],
                "APP_BASE_URL",
            ),
            (&[("APP_TIMEZONE", "Mars/Olympus")][..], "APP_TIMEZONE"),
            (&[("AUTO_CLOSE_HOUR", "24")][..], "AUTO_CLOSE_HOUR"),
            (&[("AUTO_CLOSE_HOUR", "four")][..], "AUTO_CLOSE_HOUR"),
            (&[("BIND_ADDR", "8080")][..], "BIND_ADDR"),
            (
                &[("CLIENT_IP_HEADER", "bad header")][..],
                "CLIENT_IP_HEADER",
            ),
        ] {
            match error_for(pairs) {
                ConfigError::Invalid { name: got, .. } => assert_eq!(got, name, "{pairs:?}"),
                other => panic!("{pairs:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn the_database_url_is_required_and_not_blank() {
        assert_eq!(
            database_url(lookup(&[("DATABASE_URL", "sqlite://data/smoke.db")])).unwrap(),
            "sqlite://data/smoke.db"
        );
        assert_eq!(
            database_url(lookup(&[("DATABASE_URL", " ")])),
            Err(ConfigError::Missing("DATABASE_URL"))
        );
        assert_eq!(
            database_url(lookup(&[])),
            Err(ConfigError::Missing("DATABASE_URL"))
        );
    }
}
```

`crates/app/src/backend.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_picks_the_backend() {
        assert_eq!(
            BackendKind::of("sqlite://data/smoke.db"),
            Ok(BackendKind::Sqlite)
        );
        assert_eq!(BackendKind::of("sqlite:smoke.db"), Ok(BackendKind::Sqlite));
        assert_eq!(
            BackendKind::of("postgres://app@db.example/smoke?sslmode=require"),
            Ok(BackendKind::Postgres)
        );
        assert_eq!(
            BackendKind::of("postgresql://app@db/smoke"),
            Ok(BackendKind::Postgres)
        );
        for url in ["mysql://db/smoke", "data/smoke.db", ""] {
            assert_eq!(BackendKind::of(url), Err(UnknownBackend), "{url}");
        }
    }
}
```

`crates/app/tests/commands.rs`:

```rust
//! The commands against a temporary SQLite database.

use std::sync::Arc;

use smoke_app::backend::Backend;
use smoke_domain::clock::SystemClock;
use smoke_domain::repos::{AuditRepo, ListRepo};
use smoke_storage_sqlite::SqliteStore;
use tempfile::TempDir;

async fn prepared() -> (TempDir, String, SqliteStore) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("app.db").display());
    let backend = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    backend.prepare().await.unwrap();
    let Backend::Sqlite(store) = backend else {
        unreachable!("a sqlite URL")
    };
    (dir, url, store)
}

#[tokio::test]
async fn prepare_can_run_again_and_seeds_the_lists_once() {
    let (_dir, url, store) = prepared().await;
    let again = Backend::connect(&url, Arc::new(SystemClock)).await.unwrap();
    again.prepare().await.unwrap();
    assert_eq!(store.rooms().await.unwrap().len(), 6);
    assert_eq!(store.symptoms().await.unwrap().len(), 6);
    assert_eq!(store.audit_log().await.unwrap().len(), 12);
}

#[tokio::test]
async fn an_unknown_scheme_is_refused_before_connecting() {
    let error = Backend::connect("mysql://db/smoke", Arc::new(SystemClock))
        .await
        .err()
        .unwrap();
    assert_eq!(
        error.to_string(),
        "DATABASE_URL must start with sqlite: or postgres://"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-tracker`
Expected: compile errors — `ServeConfig`, `BackendKind` and `Backend` are not defined.

- [ ] **Step 3: Implement**

Put this above the tests in `crates/app/src/config.rs`:

```rust
//! Configuration from the environment only (spec §10.5); `.env.example` lists every variable.

use std::net::SocketAddr;

use chrono_tz::Tz;
use smoke_domain::schedule::DailySchedule;
use smoke_web::config::{BaseUrl, WebConfig};
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{0} is not set (see .env.example)")]
    Missing(&'static str),
    #[error("{name} is invalid: {reason}")]
    Invalid { name: &'static str, reason: String },
}

/// Reads a variable from the process environment.
pub fn env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// `DATABASE_URL`, needed by every command: `sqlite://…` or `postgres://…`.
pub fn database_url(get: impl Fn(&str) -> Option<String>) -> Result<String, ConfigError> {
    get("DATABASE_URL")
        .filter(|url| !url.trim().is_empty())
        .ok_or(ConfigError::Missing("DATABASE_URL"))
}

/// What `serve` needs beyond the database.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub web: WebConfig,
    /// The local time zone and the auto-close hour (used by the sweep in milestone 4).
    pub schedule: DailySchedule,
    pub bind_addr: SocketAddr,
}

impl ServeConfig {
    pub const DEFAULT_TIMEZONE: &str = "Europe/Rome";
    pub const DEFAULT_AUTO_CLOSE_HOUR: u32 = 4;
    pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:8080";

    pub fn from_lookup(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let base_url = get("APP_BASE_URL").ok_or(ConfigError::Missing("APP_BASE_URL"))?;
        let base_url = BaseUrl::parse(&base_url).map_err(|e| invalid("APP_BASE_URL", e))?;
        let timezone = get("APP_TIMEZONE").unwrap_or_else(|| Self::DEFAULT_TIMEZONE.to_owned());
        let timezone: Tz = timezone
            .trim()
            .parse()
            .map_err(|_| invalid("APP_TIMEZONE", format!("unknown time zone {timezone:?}")))?;
        let hour = match get("AUTO_CLOSE_HOUR") {
            Some(hour) => hour
                .trim()
                .parse()
                .map_err(|_| invalid("AUTO_CLOSE_HOUR", format!("not an hour: {hour:?}")))?,
            None => Self::DEFAULT_AUTO_CLOSE_HOUR,
        };
        let schedule =
            DailySchedule::new(timezone, hour).map_err(|e| invalid("AUTO_CLOSE_HOUR", e))?;
        let bind_addr = get("BIND_ADDR").unwrap_or_else(|| Self::DEFAULT_BIND_ADDR.to_owned());
        let bind_addr = bind_addr
            .trim()
            .parse()
            .map_err(|_| invalid("BIND_ADDR", format!("not an address:port: {bind_addr:?}")))?;
        let client_ip_header = get("CLIENT_IP_HEADER")
            .filter(|h| !h.trim().is_empty())
            .map(|h| {
                h.trim()
                    .parse()
                    .map_err(|_| invalid("CLIENT_IP_HEADER", format!("not a header name: {h:?}")))
            })
            .transpose()?;
        if !base_url.is_https() {
            warn!("APP_BASE_URL is plain http: for local development only");
        }
        Ok(Self {
            web: WebConfig {
                base_url,
                client_ip_header,
            },
            schedule,
            bind_addr,
        })
    }
}

fn invalid(name: &'static str, reason: impl ToString) -> ConfigError {
    ConfigError::Invalid {
        name,
        reason: reason.to_string(),
    }
}
```

Put this above the tests in `crates/app/src/backend.rs`:

```rust
//! The storage backend, picked once from `DATABASE_URL` (spec §10.1).
//!
//! Commands are generic over [`Store`](smoke_domain::repos::Store); [`with_store!`] runs one
//! with whichever backend is configured, so the compiler builds a copy of it per backend and no
//! call goes through a trait object.

use std::sync::Arc;

use anyhow::Context;
use smoke_domain::clock::Clock;
use smoke_domain::repos::StoreResult;
use smoke_domain::starter::seed_starter_lists;
use smoke_storage_postgres::PostgresStore;
use smoke_storage_sqlite::SqliteStore;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Sqlite,
    Postgres,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("DATABASE_URL must start with sqlite: or postgres://")]
pub struct UnknownBackend;

impl BackendKind {
    pub fn of(url: &str) -> Result<Self, UnknownBackend> {
        let url = url.trim();
        if url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else {
            Err(UnknownBackend)
        }
    }
}

pub enum Backend {
    Sqlite(SqliteStore),
    Postgres(PostgresStore),
}

impl Backend {
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        let url = url.trim();
        Ok(match BackendKind::of(url)? {
            BackendKind::Sqlite => Self::Sqlite(
                SqliteStore::connect(url, clock)
                    .await
                    .context("could not open the SQLite database")?,
            ),
            BackendKind::Postgres => Self::Postgres(
                PostgresStore::connect(url, clock)
                    .await
                    .context("could not connect to Postgres")?,
            ),
        })
    }

    pub async fn migrate(&self) -> StoreResult<()> {
        match self {
            Self::Sqlite(store) => store.migrate().await,
            Self::Postgres(store) => store.migrate().await,
        }
    }

    /// Migrations, then the starter rooms and symptoms if the lists are empty. Safe to repeat.
    pub async fn prepare(&self) -> anyhow::Result<()> {
        self.migrate().await.context("migrations failed")?;
        with_store!(self, |store| seed_starter_lists(store).await)
            .context("seeding the starter lists failed")?;
        Ok(())
    }
}

/// `with_store!(backend, |store| expr)` evaluates `expr` with `store` bound to the concrete
/// store inside `backend` (by value, or by reference for `&backend`).
macro_rules! with_store {
    ($backend:expr, |$store:ident| $body:expr) => {
        match $backend {
            $crate::backend::Backend::Sqlite($store) => $body,
            $crate::backend::Backend::Postgres($store) => $body,
        }
    };
}
pub(crate) use with_store;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-tracker && cargo clippy -p smoke-tracker --all-targets -- -D warnings`
Expected: all pass, including `invalid_values_name_the_variable` and
`prepare_can_run_again_and_seeds_the_lists_once`; no warnings.

- [ ] **Step 5: Commit**

```bash
git add Cargo.lock crates/app
git commit -m "feat(app): environment config and backend dispatch with a with_store! macro

Config reads through an injected lookup function, so tests never mutate the
process environment. A Backend enum picked from the DATABASE_URL scheme plus
a macro_rules! match gives each command a monomorphised copy per backend.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: App crate — the commands

`serve` (migrations, then the server and an hourly expired-session sweep, until SIGTERM or
Ctrl-C), `migrate` (migrations + starter lists; safe to repeat), `create-resident <username>`
(password prompted twice, never echoed, never an argument), and `verify-audit` (prints the head
hash and every problem; exit code 1 when anything is wrong). Errors exit with 2. Logs go to
stderr so command output on stdout stays clean.

**Files:**
- Create: `crates/app/src/{cli,main}.rs`, `crates/app/src/commands/{mod,serve,create_resident,verify_audit}.rs`
- Modify: `crates/app/Cargo.toml`, `crates/app/src/lib.rs`, `crates/app/tests/commands.rs`

**Interfaces:**
- Consumes: `smoke_web::{router, serve, AppState, password}`; `ServeConfig`, `Backend`,
  `with_store!` (Task 9); `smoke_domain::verify::{verify_store, AuditReport, SnapshotProblem}`
  (milestone 2); `SessionRepo::delete_expired_sessions` (Task 2).
- Produces:
  - `smoke_app::cli::{Cli, Command::{Serve, Migrate, CreateResident { username, locale },
    VerifyAudit}}`.
  - `smoke_app::commands::create_resident::{prompt_password() -> anyhow::Result<NewPassword>,
    create(&S, Username, NewPassword, Locale) -> anyhow::Result<User>}`.
  - `smoke_app::commands::verify_audit::{run(&S) -> anyhow::Result<AuditReport>,
    describe(&AuditReport) -> String}`.
  - `smoke_app::commands::serve::run(store, clock, ServeConfig)`.
  - `smoke_app::{init_logging(), run(Cli) -> anyhow::Result<ExitCode>}`; binary `smoke-tracker`.

- [ ] **Step 1: Write the failing tests**

```diff
--- a/crates/app/tests/commands.rs
+++ b/crates/app/tests/commands.rs
@@ -3,11 +3,17 @@
 use std::sync::Arc;
 
 use smoke_app::backend::Backend;
+use smoke_app::commands::{create_resident, verify_audit};
 use smoke_domain::clock::SystemClock;
-use smoke_domain::repos::{AuditRepo, ListRepo};
+use smoke_domain::repos::{AuditRepo, Conflict, ListRepo, StoreError, UserRepo};
+use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
+use smoke_web::password;
+use sqlx::SqlitePool;
 use tempfile::TempDir;
 
+const PASSWORD: &str = "correct horse battery";
+
 async fn prepared() -> (TempDir, String, SqliteStore) {
     let dir = tempfile::tempdir().unwrap();
     let url = format!("sqlite://{}", dir.path().join("app.db").display());
@@ -19,6 +25,14 @@ async fn prepared() -> (TempDir, String, SqliteStore) {
     (dir, url, store)
 }
 
+fn username(name: &str) -> Username {
+    Username::new(name).unwrap()
+}
+
+fn password() -> NewPassword {
+    NewPassword::new(PASSWORD.to_owned()).unwrap()
+}
+
 #[tokio::test]
 async fn prepare_can_run_again_and_seeds_the_lists_once() {
     let (_dir, url, store) = prepared().await;
@@ -40,3 +54,69 @@ async fn an_unknown_scheme_is_refused_before_connecting() {
         "DATABASE_URL must start with sqlite: or postgres://"
     );
 }
+
+#[tokio::test]
+async fn create_resident_stores_an_argon2_hash_and_audits_without_it() {
+    let (_dir, _url, store) = prepared().await;
+    let user = create_resident::create(&store, username(" Daniele "), password(), Locale::It)
+        .await
+        .unwrap();
+    assert_eq!(user.username.as_str(), "daniele");
+    assert_eq!(
+        (user.role, user.locale, user.expires_at),
+        (Role::Resident, Locale::It, None)
+    );
+    let stored = store.user(user.id).await.unwrap().unwrap();
+    assert!(password::verify(stored.password_hash.as_ref(), PASSWORD));
+    let log = format!("{:?}", store.audit_log().await.unwrap());
+    assert!(!log.contains("argon2"), "the hash reached the audit log");
+}
+
+#[tokio::test]
+async fn create_resident_refuses_a_taken_username() {
+    let (_dir, _url, store) = prepared().await;
+    create_resident::create(&store, username("daniele"), password(), Locale::En)
+        .await
+        .unwrap();
+    let error = create_resident::create(&store, username("DANIELE"), password(), Locale::En)
+        .await
+        .unwrap_err();
+    assert!(
+        matches!(
+            error.downcast_ref::<StoreError>(),
+            Some(StoreError::Conflict(Conflict::UsernameTaken))
+        ),
+        "{error:?}"
+    );
+}
+
+#[tokio::test]
+async fn verify_audit_passes_on_honest_data() {
+    let (_dir, _url, store) = prepared().await;
+    create_resident::create(&store, username("daniele"), password(), Locale::En)
+        .await
+        .unwrap();
+    let report = verify_audit::run(&store).await.unwrap();
+    assert!(report.is_ok());
+    assert_eq!(report.entries, 13);
+    let text = verify_audit::describe(&report);
+    assert!(text.contains("hash chain:    ok"), "{text}");
+    assert!(text.contains(&report.head_hash), "{text}");
+}
+
+#[tokio::test]
+async fn verify_audit_reports_a_direct_database_edit() {
+    let (_dir, url, store) = prepared().await;
+    let pool = SqlitePool::connect(&url).await.unwrap();
+    sqlx::raw_sql("UPDATE rooms SET name = 'Garage' WHERE id = 1")
+        .execute(&pool)
+        .await
+        .unwrap();
+    let report = verify_audit::run(&store).await.unwrap();
+    assert!(!report.is_ok());
+    let text = verify_audit::describe(&report);
+    assert!(
+        text.contains("room 1: differs from audit entry 1 (edited outside the app)"),
+        "{text}"
+    );
+}
```

`crates/app/src/cli.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn parse(args: &[&str]) -> Result<Command, clap::Error> {
        Cli::try_parse_from(std::iter::once("smoke-tracker").chain(args.iter().copied()))
            .map(|cli| cli.command)
    }

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_command_parses() {
        assert_eq!(parse(&["serve"]).unwrap(), Command::Serve);
        assert_eq!(parse(&["migrate"]).unwrap(), Command::Migrate);
        assert_eq!(parse(&["verify-audit"]).unwrap(), Command::VerifyAudit);
        assert_eq!(
            parse(&["create-resident", "daniele"]).unwrap(),
            Command::CreateResident {
                username: "daniele".into(),
                locale: Locale::En
            }
        );
        assert_eq!(
            parse(&["create-resident", "daniele", "--locale", "it"]).unwrap(),
            Command::CreateResident {
                username: "daniele".into(),
                locale: Locale::It
            }
        );
    }

    #[test]
    fn a_password_cannot_be_passed_as_an_argument() {
        assert!(parse(&["create-resident", "daniele", "--password", "x"]).is_err());
        assert!(parse(&["create-resident", "daniele", "secret-password"]).is_err());
        assert!(parse(&["create-resident", "daniele", "--locale", "fr"]).is_err());
    }
}
```

`crates/app/src/commands/verify_audit.rs` with only its tests for now:

```rust
#[cfg(test)]
mod tests {
    use smoke_domain::audit::{AuditEntity, ChainError, GENESIS_HASH};

    use super::*;

    fn report(chain: Result<(), ChainError>, problems: Vec<SnapshotProblem>) -> AuditReport {
        AuditReport {
            entries: 3,
            head_hash: "ab12".into(),
            chain,
            problems,
        }
    }

    #[test]
    fn a_clean_report_says_ok_twice() {
        assert_eq!(
            describe(&report(Ok(()), vec![])),
            "audit entries: 3\nhead hash:     ab12\nhash chain:    ok\nrecords:       ok\n"
        );
        let empty = AuditReport {
            entries: 0,
            head_hash: GENESIS_HASH.into(),
            chain: Ok(()),
            problems: vec![],
        };
        assert!(describe(&empty).contains(GENESIS_HASH));
    }

    #[test]
    fn every_problem_is_listed_with_its_likely_cause() {
        let text = describe(&report(
            Err(ChainError::HashMismatch { id: 2 }),
            vec![
                SnapshotProblem::Unaudited {
                    entity: AuditEntity::User,
                    entity_id: 3,
                },
                SnapshotProblem::Vanished {
                    entity: AuditEntity::Room,
                    entity_id: 2,
                },
                SnapshotProblem::Differs {
                    entity: AuditEntity::SmellReport,
                    entity_id: 4,
                    audit_id: 9,
                },
            ],
        ));
        assert!(
            text.contains(
                "hash chain:    BROKEN: audit entry 2: its content does not match its hash"
            )
        );
        assert!(text.contains("records:       3 problem(s)"));
        assert!(text.contains("  user 3: has no audit entry (inserted outside the app)"));
        assert!(text.contains("  room 2: is audited but missing (deleted outside the app)"));
        assert!(
            text.contains("  smell_report 4: differs from audit entry 9 (edited outside the app)")
        );
    }
}
```

`crates/app/src/commands/mod.rs`:

```rust
//! One module per CLI command.

pub mod create_resident;
pub mod serve;
pub mod verify_audit;
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-tracker`
Expected: compile errors — `Cli`, `describe` and `create_resident` are not defined.

- [ ] **Step 3: Implement the command line and the commands**

Put this above the tests in `crates/app/src/cli.rs`:

```rust
//! The command line (spec §10.4).

use clap::{Parser, Subcommand};
use smoke_domain::types::Locale;

#[derive(Debug, Parser)]
#[command(
    name = "smoke-tracker",
    version,
    about = "Log cigarette smoke drifting in from the neighbours"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum Command {
    /// Apply migrations, then serve the web app on BIND_ADDR.
    Serve,
    /// Apply the database migrations and add the starter rooms and symptoms.
    Migrate,
    /// Create a resident account. Prompts for the password; nothing is echoed.
    CreateResident {
        /// 3–32 characters: letters, digits, '.', '_' or '-'. Stored in lowercase.
        username: String,
        /// Interface language: en, it or tr.
        #[arg(long, default_value = "en", value_parser = parse_locale)]
        locale: Locale,
    },
    /// Check the audit hash chain and that every record matches its latest audit entry.
    /// Exits with 1 when anything is wrong.
    VerifyAudit,
}

fn parse_locale(text: &str) -> Result<Locale, String> {
    text.parse()
        .map_err(|_| format!("expected en, it or tr, got {text:?}"))
}
```

Put this above the tests in `crates/app/src/commands/verify_audit.rs`:

```rust
//! `verify-audit` (spec §6.1): recomputes the hash chain and compares every record with its
//! latest audit entry. Read-only.

use std::fmt::Write;

use smoke_domain::repos::SnapshotRepo;
use smoke_domain::verify::{AuditReport, SnapshotProblem, verify_store};

pub async fn run<S: SnapshotRepo>(store: &S) -> anyhow::Result<AuditReport> {
    Ok(verify_store(store).await?)
}

/// The report as printed: the head hash to compare with the one pinned in backups, then each
/// check's outcome.
pub fn describe(report: &AuditReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "audit entries: {}", report.entries);
    let _ = writeln!(out, "head hash:     {}", report.head_hash);
    match &report.chain {
        Ok(()) => out.push_str("hash chain:    ok\n"),
        Err(error) => {
            let _ = writeln!(out, "hash chain:    BROKEN: {error}");
        }
    }
    if report.problems.is_empty() {
        out.push_str("records:       ok\n");
    } else {
        let _ = writeln!(out, "records:       {} problem(s)", report.problems.len());
        for problem in &report.problems {
            let _ = writeln!(out, "  {}", explain(problem));
        }
    }
    out
}

fn explain(problem: &SnapshotProblem) -> String {
    match problem {
        SnapshotProblem::Unaudited { entity, entity_id } => {
            format!("{entity} {entity_id}: has no audit entry (inserted outside the app)")
        }
        SnapshotProblem::Vanished { entity, entity_id } => {
            format!("{entity} {entity_id}: is audited but missing (deleted outside the app)")
        }
        SnapshotProblem::Differs {
            entity,
            entity_id,
            audit_id,
        } => format!(
            "{entity} {entity_id}: differs from audit entry {audit_id} (edited outside the app)"
        ),
    }
}
```

`crates/app/src/commands/create_resident.rs`:

```rust
//! `create-resident <username>`: the bootstrap account (spec §8.2). No default credentials
//! exist anywhere; the password is typed at a prompt that does not echo.

use anyhow::bail;
use smoke_domain::records::{NewUser, User};
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_web::password;

/// Asks twice, so a typo cannot lock the only resident out.
pub fn prompt_password() -> anyhow::Result<NewPassword> {
    let first = rpassword::prompt_password("Password (10–128 characters): ")?;
    let again = rpassword::prompt_password("Repeat the password: ")?;
    if first != again {
        bail!("the passwords do not match");
    }
    Ok(NewPassword::new(first)?)
}

/// Creates the resident in one audited transaction, as the system (no actor).
pub async fn create<S>(
    store: &S,
    username: Username,
    password: NewPassword,
    locale: Locale,
) -> anyhow::Result<User>
where
    S: Transactional<Tx: UserWrites>,
{
    let hash = tokio::task::spawn_blocking(move || password::hash(&password)).await??;
    let mut tx = store.begin(None).await?;
    let user = tx
        .create_user(NewUser {
            username,
            password_hash: Some(hash),
            role: Role::Resident,
            locale,
            expires_at: None,
        })
        .await?;
    tx.commit().await?;
    Ok(user)
}
```

`crates/app/src/commands/serve.rs`:

```rust
//! `serve`: the web app plus its background tasks, until SIGTERM or Ctrl-C.

use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use smoke_domain::clock::Clock;
use smoke_domain::repos::{SessionRepo, Store};
use smoke_web::AppState;
use tokio::net::TcpListener;
use tracing::{info, warn};

use crate::config::ServeConfig;

/// How often expired sessions are deleted.
const SESSION_SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);

pub async fn run<S: Store + Clone + 'static>(
    store: S,
    clock: Arc<dyn Clock>,
    config: ServeConfig,
) -> anyhow::Result<()> {
    let listener = TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("could not listen on {}", config.bind_addr))?;
    info!(
        address = %listener.local_addr()?,
        origin = config.web.base_url.origin(),
        timezone = %config.schedule.tz(),
        auto_close_hour = config.schedule.hour(),
        "listening"
    );
    let sweeper = tokio::spawn(sweep_sessions(store.clone(), clock.clone()));
    let app = smoke_web::router(AppState::new(store, clock, config.web));
    let served = smoke_web::serve(listener, app, shutdown_signal()).await;
    sweeper.abort();
    served.context("the server stopped with an error")
}

/// Deletes expired sessions at startup and then every hour.
async fn sweep_sessions<S: SessionRepo>(store: S, clock: Arc<dyn Clock>) {
    let mut ticks = tokio::time::interval(SESSION_SWEEP_EVERY);
    loop {
        ticks.tick().await;
        match store.delete_expired_sessions(clock.now()).await {
            Ok(0) => {}
            Ok(removed) => info!(removed, "deleted expired sessions"),
            Err(error) => warn!(%error, "could not delete expired sessions"),
        }
    }
}

/// Resolves on Ctrl-C, or on SIGTERM (what `docker stop` sends).
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => {
                warn!(%error, "cannot listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    info!("shutting down");
}
```

- [ ] **Step 4: Wire the binary**

Replace `crates/app/src/lib.rs` with:

```rust
//! The Smoke Tracker binary as a library: configuration, backend selection, the commands and
//! the background tasks. `main.rs` only parses the command line and calls [`run`].

pub mod backend;
pub mod cli;
pub mod commands;
pub mod config;

use std::io::IsTerminal;
use std::process::ExitCode;
use std::sync::Arc;

use smoke_domain::clock::{Clock, SystemClock};
use smoke_domain::types::Username;
use tracing_subscriber::EnvFilter;

use crate::backend::{Backend, with_store};
use crate::cli::{Cli, Command};
use crate::commands::{create_resident, serve, verify_audit};
use crate::config::{ServeConfig, env};

/// Logs go to stderr, filtered by `RUST_LOG` (default `info`), so command output on stdout
/// stays clean. Colours only on a terminal, not in container logs.
pub fn init_logging() {
    let stderr = std::io::stderr();
    tracing_subscriber::fmt()
        .with_ansi(stderr.is_terminal())
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();
}

/// Runs one command. `Ok` carries the exit code: 0, or 1 when `verify-audit` finds problems.
/// Errors exit with 2.
pub async fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let database_url = config::database_url(env)?;
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    match cli.command {
        Command::Serve => {
            let config = ServeConfig::from_lookup(env)?;
            let backend = Backend::connect(&database_url, clock.clone()).await?;
            backend.prepare().await?;
            with_store!(backend, |store| serve::run(store, clock, config).await)?;
        }
        Command::Migrate => {
            Backend::connect(&database_url, clock)
                .await?
                .prepare()
                .await?;
            println!("The database is up to date.");
        }
        Command::CreateResident { username, locale } => {
            let username = Username::new(&username)?;
            let password = create_resident::prompt_password()?;
            let backend = Backend::connect(&database_url, clock).await?;
            backend.prepare().await?;
            let user = with_store!(&backend, |store| {
                create_resident::create(store, username, password, locale).await
            })?;
            println!("Created resident {}.", user.username);
        }
        Command::VerifyAudit => {
            let backend = Backend::connect(&database_url, clock).await?;
            let report = with_store!(&backend, |store| verify_audit::run(store).await)?;
            print!("{}", verify_audit::describe(&report));
            if !report.is_ok() {
                return Ok(ExitCode::from(1));
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
```

`crates/app/src/main.rs`:

```rust
use std::process::ExitCode;

use clap::Parser;
use smoke_app::cli::Cli;

#[tokio::main]
async fn main() -> ExitCode {
    smoke_app::init_logging();
    match smoke_app::run(Cli::parse()).await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::from(2)
        }
    }
}
```

```diff
--- a/crates/app/Cargo.toml
+++ b/crates/app/Cargo.toml
@@ -11,6 +11,10 @@ repository.workspace = true
 [lib]
 name = "smoke_app"
 
+[[bin]]
+name = "smoke-tracker"
+path = "src/main.rs"
+
 [dependencies]
 anyhow.workspace = true
 chrono-tz.workspace = true
```

- [ ] **Step 5: Run the tests and the binary**

Run: `cargo test -p smoke-tracker && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all pass, including `create_resident_stores_an_argon2_hash_and_audits_without_it` and
`verify_audit_reports_a_direct_database_edit`; no warnings.

Then a manual run (needs a terminal for the password prompt):
```sh
mkdir -p data
export DATABASE_URL=sqlite://data/smoke.db?mode=rwc APP_BASE_URL=http://localhost:8080
cargo run -p smoke-tracker -- migrate             # "The database is up to date."
cargo run -p smoke-tracker -- create-resident daniele
cargo run -p smoke-tracker -- verify-audit; echo "exit $?"   # hash chain ok, records ok, exit 0
cargo run -p smoke-tracker -- serve               # sign in at http://localhost:8080, Ctrl-C stops
```

- [ ] **Step 6: Commit**

```bash
git add crates/app
git commit -m "feat(app): clap CLI with serve, migrate, create-resident and verify-audit

The binary is a thin main over a library, so every command is testable. serve
shuts down gracefully on SIGTERM via tokio::select!, and a spawned task sweeps
expired sessions hourly. verify-audit maps problems to exit code 1.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Container image, release workflow and Trivy

A static musl binary built with `cargo-auditable` (so Trivy can see the crates inside it) on
`gcr.io/distroless/static-debian13:nonroot`: no shell, no package manager, a non-root user and a
`/data` volume for SQLite. `release.yml` pushes the image to GHCR on every push to `main` and on
`v*` tags; there is no deploy job until a host is chosen. `security.yml` gains a `trivy` job that
builds the same image (not pushed) and fails on fixable HIGH/CRITICAL findings.

**Files:**
- Create: `Dockerfile`, `.dockerignore`, `.env.example`, `.github/workflows/release.yml`
- Modify: `.github/workflows/security.yml`, `.github/dependabot.yml`,
  `.github/rulesets/protect-main.json`, `README.md`

**Interfaces:**
- Consumes: the `smoke-tracker` binary (Task 10).
- Produces: image `ghcr.io/locadani/smoke-tracker` (tags: branch, semver, sha); CI check `trivy`.

- [ ] **Step 1: Write the image**

`Dockerfile`:

```dockerfile
# syntax=docker/dockerfile:1
# A static musl binary on a distroless base: no shell, no package manager, about 15–20 MB.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
WORKDIR /src
COPY . .
ENV SQLX_OFFLINE=true
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin smoke-tracker \
    && cp target/release/smoke-tracker /smoke-tracker \
    && mkdir /data

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /smoke-tracker /usr/local/bin/smoke-tracker
COPY --from=build --chown=65532:65532 /data /data
ENV DATABASE_URL=sqlite:///data/smoke.db \
    BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
VOLUME ["/data"]
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/smoke-tracker"]
CMD ["serve"]
```

`.dockerignore`:

```text
# Only the sources the release build needs.
target
.git
.github
.githooks
.superpowers
docs
data
*.db
*.db-*
.env
.env.*
# The image's own stable toolchain builds the release; the file would download another one
# with rustfmt and clippy for nothing.
rust-toolchain.toml
```

`.env.example` (every variable of spec §10.5, plus `CLIENT_IP_HEADER` from Task 7):

```text
# Copy to .env (git-ignored), adjust, and load it into your shell:
#   set -a; . ./.env; set +a
# Never put a password in this file: use a local database with trust authentication.

# sqlite://<file> or postgres://<user>@<host>/<database>
DATABASE_URL=sqlite://data/smoke.db
# The address people open. https everywhere except http://localhost for development.
APP_BASE_URL=http://localhost:8080
APP_TIMEZONE=Europe/Rome
AUTO_CLOSE_HOUR=4
BIND_ADDR=127.0.0.1:8080
RUST_LOG=info,tower_http=info
# Only behind a reverse proxy that writes the client's address into this header
# (e.g. Fly-Client-IP, CF-Connecting-IP, X-Forwarded-For). Leave unset otherwise.
# CLIENT_IP_HEADER=
```

- [ ] **Step 2: Build and run it locally**

```sh
docker build -t smoke-tracker .
docker image ls smoke-tracker --format '{{.Size}}'          # expected: about 20 MB (20.4 MB when this plan was verified)
docker volume create smoke-data
docker run --rm -v smoke-data:/data smoke-tracker migrate    # "The database is up to date."
docker run -d --name smoke-try -p 8080:8080 -v smoke-data:/data -e APP_BASE_URL=http://localhost:8080 smoke-tracker
curl -s localhost:8080/healthz                               # ok
docker rm -f smoke-try && docker volume rm smoke-data
```

- [ ] **Step 3: Add the workflows**

`.github/workflows/release.yml`:

```yaml
name: Release

on:
  push:
    branches: [main]
    tags: ["v*"]

permissions:
  contents: read
  packages: write

jobs:
  image:
    name: image
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: docker/setup-buildx-action@v4
      - uses: docker/login-action@v4
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - id: meta
        uses: docker/metadata-action@v6
        with:
          images: ghcr.io/${{ github.repository }}
          tags: |
            type=ref,event=branch
            type=semver,pattern={{version}}
            type=sha
      - uses: docker/build-push-action@v7
        with:
          context: .
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
      # No deploy job yet: the hosting provider is still to be chosen (ROADMAP "Still open").
```

```diff
--- a/.github/workflows/security.yml
+++ b/.github/workflows/security.yml
@@ -28,3 +28,25 @@ jobs:
       - uses: gitleaks/gitleaks-action@v3
         env:
           GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
+
+  trivy:
+    name: trivy
+    runs-on: ubuntu-latest
+    steps:
+      - uses: actions/checkout@v7
+      - uses: docker/setup-buildx-action@v4
+      - name: build the release image (not pushed)
+        uses: docker/build-push-action@v7
+        with:
+          context: .
+          load: true
+          tags: smoke-tracker:scan
+          cache-from: type=gha
+      - name: scan the image and the crates compiled into it
+        # Pinned to a commit, not a tag: a third-party action's tags can be moved.
+        uses: aquasecurity/trivy-action@ed142fd0673e97e23eac54620cfb913e5ce36c25 # v0.36.0
+        with:
+          image-ref: smoke-tracker:scan
+          severity: CRITICAL,HIGH
+          ignore-unfixed: true
+          exit-code: "1"
```

```diff
--- a/.github/dependabot.yml
+++ b/.github/dependabot.yml
@@ -11,3 +11,7 @@ updates:
     directory: /
     schedule:
       interval: weekly
+  - package-ecosystem: docker
+    directory: /
+    schedule:
+      interval: weekly
```

Make `trivy` a required check:

```diff
--- a/.github/rulesets/protect-main.json
+++ b/.github/rulesets/protect-main.json
@@ -32,7 +32,8 @@
           { "context": "fmt · clippy · test", "integration_id": 15368 },
           { "context": "sqlx offline data", "integration_id": 15368 },
           { "context": "cargo-deny", "integration_id": 15368 },
-          { "context": "gitleaks", "integration_id": 15368 }
+          { "context": "gitleaks", "integration_id": 15368 },
+          { "context": "trivy", "integration_id": 15368 }
         ]
       }
     }
```

After the merge, the user re-imports `.github/rulesets/protect-main.json` in GitHub → Settings →
Rules → Rulesets (the ruleset in GitHub does not update itself from the file).

- [ ] **Step 4: Document running it**

````diff
--- a/README.md
+++ b/README.md
@@ -27,6 +27,31 @@ scripts/sqlx-prepare.sh        # after changing a query or a migration; commit t
 
 Building needs no database: queries are checked against the committed `.sqlx/` metadata.
 
+## Running it
+
+Configuration comes from environment variables only; [`.env.example`](.env.example) lists
+every one.
+
+```sh
+cp .env.example .env && mkdir -p data
+set -a; . ./.env; set +a
+cargo run -p smoke-tracker -- create-resident <username>   # prompts for the password
+cargo run -p smoke-tracker -- serve                        # http://localhost:8080
+cargo run -p smoke-tracker -- verify-audit                 # exits with 1 if the history was altered
+```
+
+### Container
+
+Every push to `main` publishes `ghcr.io/locadani/smoke-tracker`: one static binary on a
+distroless base, with the database in the `/data` volume.
+
+```sh
+docker build -t smoke-tracker .
+docker volume create smoke-data
+docker run --rm -it -v smoke-data:/data smoke-tracker create-resident <username>
+docker run -d -p 8080:8080 -v smoke-data:/data -e APP_BASE_URL=http://localhost:8080 smoke-tracker
+```
+
 ## Documentation
 
 - [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
````

- [ ] **Step 5: Check and commit**

Run: `gitleaks dir --no-banner . && cargo deny check`
Expected: no leaks; `advisories ok, bans ok, licenses ok, sources ok`.

```bash
git add Dockerfile .dockerignore .env.example .github README.md
git commit -m "build: static musl image on distroless, GHCR release and a Trivy scan

cargo-auditable embeds the dependency list so Trivy scans the Rust crates
inside the binary, not just the base image. The Trivy action is pinned to a
commit; Dependabot now also watches the Docker base images.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Decisions and roadmap

**Files:**
- Modify: `docs/DECISIONS.md` (append rows #56–#73; the file is append-only)
- Modify: `docs/plans/ROADMAP.md`

**Interfaces:** none (documentation).

- [ ] **Step 1: Append the decisions**

Append to the table in `docs/DECISIONS.md`:

```markdown
| 56 | 2026-10-13 | Sessions | Our own `tower_sessions::SessionStore` over a domain **`SessionRepo`**, with a `sessions` table in both migrations | `tower-sessions-sqlx-store` | Its 0.15.0 release needs `sqlx ^0.8` and `tower-sessions ^0.14`: a second sqlx next to our 0.9. Our store is ~100 lines, works on both backends, and keeps `web` free of SQL |
| 57 | 2026-10-13 | Sessions | The database stores the **SHA-256 of the session id**, never the id | Store the id | A copy of the database (a backup, a leak) cannot be used to take over a session |
| 58 | 2026-10-13 | Sessions | `set_password` and `expire_user` **delete the user's sessions in the same transaction** | Delete them from the web layer afterwards | The spec's "deleted on password change / expiry" can never be skipped, and a rolled-back change keeps them |
| 59 | 2026-10-13 | Sessions | 30-day lifetime **renewed at most once a day**; sign-in cycles the session id | Renew on every request | One database write per day per active user instead of one per request; a new id at sign-in prevents session fixation |
| 60 | 2026-10-13 | Auth | Sign-in limits in memory: **5 failures per username / 15 min → 15-min lock**; **20 failures per address / 15 min → 15-min block**. Usernames keyed trimmed + lowercased; a success clears only the username's count | A database table; per-address only | Spec §8.1 asks for an in-memory limiter on one instance; case changes must not bypass the lock |
| 61 | 2026-10-13 | Auth | The client address is the TCP peer, or the **last** entry of a configured **`CLIENT_IP_HEADER`** | Always trust `X-Forwarded-For` | Earlier entries are whatever the client claims; the last one is written by the trusted proxy |
| 62 | 2026-10-13 | Security | `APP_BASE_URL` decides **cookie flags, HSTS and the Origin check**; it must be https except on loopback hosts, where the cookie is `session` without `Secure` | Derive from the request's `Host` / scheme | Behind a proxy the app sees plain http and an internal host; browsers refuse `__Host-`/`Secure` cookies on http://localhost |
| 63 | 2026-10-13 | Security | CSRF check: **`Sec-Fetch-Site` decides when present**, else `Origin` must equal the base URL's origin; a state-changing request with neither is refused | Origin only; CSRF tokens | Fetch metadata is sent by every supported browser and also covers `Origin: null`; no token plumbing in every form |
| 64 | 2026-10-13 | Security | **htmx 2.0.11 vendored** (SHA-256 pinned by a test) and configured with `includeIndicatorStyles:false, allowEval:false, allowScriptTags:false`; CSP is `'self'` only | CDN; `'unsafe-inline'` | No third-party requests; the strict CSP holds without exceptions |
| 65 | 2026-10-13 | Assets | Static files embedded with **`include_bytes!` from an explicit list** (a test checks it matches the folder), served with a strong ETag and `Cache-Control: no-cache` | `rust-embed`; fingerprinted file names | No extra dependency for three files; revalidation costs a `304` with no body |
| 66 | 2026-10-13 | i18n | **`fluent-templates` `static_loader!`**; language = account → `lang` cookie → `Accept-Language` → English; a test requires every language to have exactly the English keys | Runtime file loading | Translations are compiled into the binary (spec §10.3); a missing key fails CI instead of showing a raw id |
| 67 | 2026-10-13 | Structure | The binary picks the backend with a **`Backend` enum and a `with_store!` macro**; commands and the router are generic over `Store` | `Box<dyn Store>` | Extends #53: native async traits are not object-safe, and generics compile one copy per backend at no runtime cost |
| 68 | 2026-10-13 | Errors | Handlers return **`AppError`**; a middleware renders a translated page. Internal errors log the cause chain with the request id; the user sees only *"Something went wrong (ref: …)"* | Render in each handler | Spec §12; no internal detail reaches the page; panics become the same page |
| 69 | 2026-10-13 | Auth | **Argon2id with the `argon2` 0.6 defaults** (m=19456 KiB, t=2, p=1), on `spawn_blocking`; unknown users are verified against a **dummy hash** | Tuned parameters; skip work for unknown users | OWASP-recommended defaults; equal work hides which usernames exist |
| 70 | 2026-10-13 | Deployment | **Static musl binary built with `cargo-auditable` on `distroless/static-debian13:nonroot`**; image to GHCR; **Trivy** (pinned by commit) as a required check; **no Snyk** | Debian slim; Snyk | Small (~15–20 MB), no shell; Trivy + cargo-deny cover image and RustSec advisories for free with no token |
| 71 | 2026-10-13 | CLI | `serve` and `migrate` both run migrations and seed the starter lists (idempotent) | Only `migrate` | A fresh volume works with a single `docker run … serve` |
| 72 | 2026-10-13 | Structure | The app crate is a **library (`smoke_app`) plus a thin `main.rs`**; the package and binary are `smoke-tracker` | Everything in `main.rs` | Commands are testable without spawning a process |
| 73 | 2026-10-13 | CLI | `create-resident` **prompts twice** for the password (no echo) and refuses it as an argument | `--password` flag | Arguments end up in shell history and `ps`; a typo cannot lock out the only resident |
```

- [ ] **Step 2: Update the roadmap**

In `docs/plans/ROADMAP.md`, change the end of row 3 from `| — |` to `| `03-web-and-auth.md` ✅ |`
(only once the milestone is merged; while it is in progress use `` `03-web-and-auth.md` ``).

- [ ] **Step 3: Commit**

```bash
git add docs/DECISIONS.md docs/plans/ROADMAP.md
git commit -m "docs: decisions #56–#73 for the web layer, sessions and the container

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Development

Local loop for this milestone:

```sh
export TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
scripts/sqlx-prepare.sh --check
cargo deny check
```

---

### Task 13: Spec wording — **only with the user's explicit approval**

Do not start this task until the user has approved these exact edits. They align the spec with
decisions #56, #61 and #62; nothing in the code depends on them.

**Files:**
- Modify: `docs/specs/2026-10-13-smoke-tracker-design.md`

- [ ] **Step 1: Show the user the edits and wait for approval**

1. §8.4, first bullet — from *"Server-side sessions stored in the database via **tower-sessions**,
   using the backend matching `DATABASE_URL`."* to *"Server-side sessions via **tower-sessions**,
   stored by our own session store in the database matching `DATABASE_URL` (only a hash of the
   session id is stored)."*
2. §8.4, second bullet — append: *"On `http://localhost` (development only) the cookie is named
   `session` and is not `Secure`, because browsers refuse `Secure` cookies there."*
3. §10.5 — append `CLIENT_IP_HEADER` (optional; the header a trusted reverse proxy writes the
   client address into) to the list of variables.
4. §11, *Cleanup* — from *"expired sessions are removed by tower-sessions"* to *"expired
   sessions are removed hourly by the server"*.

- [ ] **Step 2: Apply exactly the approved edits and commit**

```bash
git add docs/specs/2026-10-13-smoke-tracker-design.md
git commit -m "docs(spec): session storage, local cookie name, CLIENT_IP_HEADER

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §8.1 | Argon2id, 10–128 chars, no composition rules | 1, 3 |
| §8.1 | Generic error message | 6 |
| §8.1 | 5 failures → 15-min lock; coarse per-IP limit; in memory | 7 |
| §8.1 | No self-registration | 6 (no sign-up route) |
| §8.2 | `create-resident` prompts for the password; no default credentials | 10 |
| §8.4 | DB sessions via tower-sessions on the `DATABASE_URL` backend | 2, 4 |
| §8.4 | `__Host-`, `HttpOnly`, `Secure`, `SameSite=Lax` | 4, 6 |
| §8.4 | 30 days, renewed on activity; deleted on logout, expiry, password change | 2, 6, 10 |
| §8.4 | Deleted on deactivation | 2 (`expire_user`) |
| §9.2 | en/it/tr with Fluent; `users.locale` after login, `lang` cookie before | 5, 6, 8 |
| §9.2 | Starter lists translated via `i18n_key` | 5 (`the_starter_lists_are_translated`) |
| §10 | `web` and `app` crates; `web` never sees SQL | 3–10 |
| §10.3 | Templates, translations, assets embedded; musl + distroless ~15–20 MB | 4, 5, 11 |
| §10.4 | `serve`, `migrate`, `create-resident`, `verify-audit` (`migrate-to-postgres` is milestone 7) | 10 |
| §10.5 | Config from the environment, defaults, `.env.example` | 9, 11 |
| §11 | Expiry checked on every request (`expires_at <= now`) | 6 (`load_user`) |
| §11 | Expired sessions cleaned up | 2, 10 |
| §12 | `thiserror` in libraries, `anyhow` in the binary | 3–10 |
| §12 | Unexpected errors logged with a request id; "Something went wrong (ref: …)" | 4, 5 |
| §12 | Askama auto-escaping; Origin / `Sec-Fetch-Site` check; strict CSP, HSTS, nosniff, Referrer-Policy, frame-ancestors | 4, 5, 6 |
| §13.1 | Trivy image scan; `release.yml` to GHCR (no deploy yet); Dependabot for Docker | 11 |

Out of scope here and planned later: the auto-close sweep and the Log screen (milestone 4),
invites, password reset and PWA (milestone 5), graphs and CodeQL (milestone 6),
`migrate-to-postgres` (milestone 7).
