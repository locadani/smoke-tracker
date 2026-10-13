# Milestone 2: Storage — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The storage boundary of Smoke Tracker: repository traits in `smoke-domain`, implemented
for SQLite and Postgres, with every write recorded in the hash-chained audit log inside the same
transaction, one contract test suite run against both backends, a schema-parity check, and the
`verify-audit` logic.

**Architecture:** `smoke-domain` gains the stored record types (`records.rs`), the storage traits
(`repos.rs`), audit verification (`verify.rs`) and the starter lists (`starter.rs`). Reads go
through a store; writes go through a transaction object returned by `begin(actor)`, and **every
write method appends its own audit entry**, so no code path can change data without recording it.
Two backend crates (`storage-sqlite`, `storage-postgres`) implement the traits with
compile-time-checked `sqlx` queries; their SQL text is identical. A third crate,
`storage-contract`, holds one test suite written against the traits; each backend runs it through
the `contract_tests!` macro. The CLI command `verify-audit` itself arrives in milestone 3; this
milestone delivers and tests the logic it calls (`smoke_domain::verify::verify_store`).

**Tech Stack:** Rust (edition 2024, MSRV 1.94), sqlx 0.9 (`sqlx.toml` per crate, offline query
metadata committed), SQLite (bundled, WAL mode), Postgres 17 (CI service container), tokio,
tempfile, sqlx-cli 0.9 (dev tool), GitHub Actions.

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` (§6, §6.1, §10.1, §13.3).
Decisions: `docs/DECISIONS.md` (rows #45–#55 are added by Task 6).
Previous plan: `docs/plans/01-foundation-and-domain.md`.

## Global Constraints

- Personal project. Before every commit, `git config user.email` must print
  `61902944+locadani@users.noreply.github.com` and `git config user.name` must print `locadani`.
  Never put any email address in a tracked file.
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings anywhere: local and CI Postgres use
  `POSTGRES_HOST_AUTH_METHOD=trust` and `postgres://postgres@localhost:5432/postgres`.
- Never modify `description.md` or `claude.md`. Never `git add` them.
- **Never edit `docs/specs/2026-10-13-smoke-tracker-design.md` without the user's explicit
  approval.** Task 14 holds every spec edit and must not start until the user has approved it.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one, e.g.
  `feat(storage): transactions append their audit entries before commit`.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI
  (`RUSTFLAGS=-D warnings`), `cargo fmt` enforced. Every new crate has `[lints] workspace = true`.
- `sqlx` is used with `default-features = false`. Allowed features: `runtime-tokio`, `macros`,
  `migrate`, `sqlx-toml`, plus `sqlite-bundled` (SQLite crate) or `postgres` and
  `tls-rustls-ring-webpki` (Postgres crate). Not `chrono`, not `json`, not `any`, not `sqlite`
  (that one also enables extension loading).
- Every instant is stored as **`BIGINT`/`INTEGER` microseconds since the Unix epoch, UTC**, in both
  backends. Conversion happens only in each backend's `convert.rs`.
- The SQL text of every query is **identical** in the two backend crates (`$1`-style parameters
  work in both). Only connection setup, transaction start and error mapping differ.
- In every query, parameters **first appear in the order `$1`, `$2`, `$3`, …** (repeating an
  earlier one later is fine). SQLite treats `$N` as a named parameter and may number names by
  first appearance, so out-of-order first use could silently swap bindings.
- Audit hash rule is unchanged from milestone 1: SHA-256 over `prev_hash`, `"\n"`, then the
  canonical JSON of the record. Genesis `prev_hash` is 64 `0` characters.
- An audit snapshot never contains a password hash.
- `cargo build` must work with no database: `.cargo/config.toml` sets `SQLX_OFFLINE=true`, and
  each storage crate commits its `.sqlx/` directory. After changing any `query!` text or a
  migration, run `scripts/sqlx-prepare.sh` and commit the regenerated `.sqlx/` files.
- Integration tests use runtime `sqlx::query`/`sqlx::raw_sql`, never the `query!` macros, so they
  need no offline metadata.

## Review Focus

1. **Two people log at the same moment.** Concurrent write transactions must never fork the audit
   chain (two entries with the same `prev_hash`). Expected: writes run one after another; the
   chain verifies. Test: `concurrent_writers_never_fork_the_chain` (Task 11), on both backends.
2. **"Daniele" and "daniele " typed at sign-up / login.** They must be the same account, not two.
   Expected: usernames are trimmed and lowercased on construction; a second create is
   `Conflict::UsernameTaken`. Tests: `usernames_are_normalised` (Task 2),
   `usernames_are_unique_ignoring_case` (Task 6).
3. **A browser form sends an instant with nanoseconds.** The stored value, the returned record and
   the audit snapshot must all agree, or `verify-audit` would flag honest data. Expected: every
   record returned by a write is re-read from the database, so it is exactly what was stored.
   Test: `instants_are_kept_to_the_microsecond` (Task 6).
4. **A password hash leaking into the append-only audit log**, where it could never be removed.
   Expected: the user snapshot has `has_password: bool` instead. Tests:
   `user_snapshot_hides_the_password_hash` (Task 3),
   `creating_a_user_is_audited_without_the_password_hash` (Task 6).
5. **A write fails halfway** (e.g. a report naming a room id that does not exist). Expected: the
   error is `Conflict::UnknownReference` and, once the transaction is dropped, neither the report
   nor any audit entry remains. Tests: `an_unknown_room_rolls_back_the_report` (Task 9),
   `a_dropped_transaction_leaves_no_trace` (Task 6).

---

## File Structure

```
smoke-tracker/
├─ Cargo.toml                         + new members' shared deps, rust-version 1.94
├─ .cargo/config.toml                 NEW  SQLX_OFFLINE=true
├─ scripts/sqlx-prepare.sh            NEW  regenerate / --check the .sqlx metadata of both crates
├─ deny.toml                          + licences pulled in by sqlx/rustls
├─ README.md                          + Postgres test instructions
├─ .github/
│  ├─ workflows/ci.yml                + Postgres service, "sqlx offline data" job
│  └─ rulesets/protect-main.json      + required check "sqlx offline data"
├─ docs/
│  ├─ DECISIONS.md                    + rows #45–#55
│  └─ plans/ROADMAP.md                + M2 plan link, M2 crate names, issue #3
└─ crates/
   ├─ domain/src/
   │  ├─ lib.rs                       + records, repos, starter, verify modules
   │  ├─ audit.rs                     AuditEntity enum replaces the free `entity: String`
   │  ├─ error.rs                     + username, list name, expiry, note errors
   │  ├─ events.rs                    thresholds become `pub const`; edge tests; doc fix
   │  ├─ permissions.rs               + check_expiry (role ↔ expiry rules)
   │  ├─ types.rs                     + Username, ListName; test import cleanup
   │  ├─ records.rs                   NEW  User, Room, Symptom, Event, SmellReport, Pm25Reading,
   │  │                                    inputs (NewUser, …), ReportValues, Audited trait
   │  ├─ repos.rs                     NEW  StoreError, Conflict, read traits, write traits,
   │  │                                    Transactional/Commit, Store/WriteTx aliases
   │  ├─ verify.rs                    NEW  Snapshot, verify_snapshots, verify_store, AuditReport
   │  └─ starter.rs                   NEW  starter rooms & symptoms, seed_starter_lists
   ├─ storage-sqlite/                 NEW  package smoke-storage-sqlite
   │  ├─ Cargo.toml, sqlx.toml        sqlx.toml: database-url-var = SQLITE_DATABASE_URL
   │  ├─ migrations/0001_initial.sql
   │  ├─ .sqlx/                       generated query metadata (committed)
   │  ├─ src/lib.rs                   SqliteStore: connect (writer + reader pools), migrate, begin
   │  ├─ src/tx.rs                    SqliteTx: commit, audit append (`record`)
   │  ├─ src/error.rs                 sqlx::Error → StoreError
   │  ├─ src/convert.rs               micros ↔ DateTime, text → enums
   │  ├─ src/{users,lists,events,reports,readings,jobs,audit}.rs   one feature per file
   │  └─ tests/contract.rs            SQLite fixture + contract_tests!
   ├─ storage-postgres/               NEW  package smoke-storage-postgres (same layout)
   │  ├─ … (sqlx.toml: POSTGRES_DATABASE_URL)
   │  ├─ tests/common/mod.rs          creates a fresh database per test on TEST_POSTGRES_URL
   │  ├─ tests/contract.rs            Postgres fixture + contract_tests!
   │  └─ tests/schema_parity.rs       SQLite and Postgres schemas have the same columns
   └─ storage-contract/               NEW  package smoke-storage-contract
      └─ src/
         ├─ lib.rs                    contract_tests! macro (the list of every contract test)
         ├─ fixture.rs                Fixture<S>, raw SQL runner, shared test data helpers
         └─ {transactions,users,lists,events,reports,readings,audit}.rs
```

**Why three storage crates instead of the spec's one `storage/` crate:** sqlx's `query!` macros
check SQL against one database per crate. sqlx 0.9's `sqlx.toml` lets each crate read its own URL
variable, so each backend crate is checked against its own database (decision #45).

**Dependency direction:** `storage-sqlite → domain`, `storage-postgres → domain`,
`storage-contract → domain`; the backends use `storage-contract` as a **dev-dependency** only.

**Local tooling (once per machine)**, needed from Task 6 on:

```sh
cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite,postgres,rustls,sqlx-toml
docker run -d --name smoke-pg -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432 postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
```

Without `TEST_POSTGRES_URL` the Postgres tests print `skipped` and pass locally; in CI (where the
`CI` variable is set) a missing URL fails them.

---

### Task 1: Domain — deferred review fixes and `AuditEntity`

Folds in the milestone-1 review minors: named threshold constants, edge tests at exactly
`now + 1 min` and `now + 1 min + 1 s`, the `new_event_state` caller note, the redundant test import,
and a closed enum for the audited entity kind.

**Files:**
- Modify: `crates/domain/src/events.rs`
- Modify: `crates/domain/src/audit.rs`
- Modify: `crates/domain/src/types.rs` (tests module only)

**Interfaces:**
- Consumes: nothing new.
- Produces:
  - `smoke_domain::events::{BACKDATE_THRESHOLD, FUTURE_TOLERANCE, LARGE_GAP}: chrono::TimeDelta`
    (replace the functions `backdate_threshold()`, `future_tolerance()`, `large_gap()`).
  - `smoke_domain::audit::AuditEntity` — `User | Invite | Room | Symptom | Event | SmellReport |
    Pm25Reading`, text `user | invite | room | symptom | event | smell_report | pm25_reading`, with
    `as_str`, `FromStr`, `Display`, serde, `ALL`, and `fn has_snapshot(self) -> bool`.
  - `AuditRecord.entity: AuditEntity` (was `String`).

- [ ] **Step 1: Write the failing tests**

In `crates/domain/src/events.rs`, add to the `tests` module (after `future_times_are_rejected`):

```rust
    #[test]
    fn the_future_tolerance_is_inclusive_to_the_second() {
        assert_eq!(
            classify_observed(now() + FUTURE_TOLERANCE, now()),
            Ok(ObservedTime::Now)
        );
        assert_eq!(
            classify_observed(now() + FUTURE_TOLERANCE + TimeDelta::seconds(1), now()),
            Err(DomainError::ObservedInFuture)
        );
    }
```

and after `closing_in_the_future_is_rejected`:

```rust
    #[test]
    fn closing_exactly_at_the_future_tolerance_is_allowed_but_not_a_second_later() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(now() + FUTURE_TOLERANCE, last, now()),
            Ok(())
        );
        assert_eq!(
            validate_manual_close(now() + FUTURE_TOLERANCE + TimeDelta::seconds(1), last, now()),
            Err(DomainError::CloseInFuture)
        );
    }
```

In `crates/domain/src/audit.rs`, add to the `tests` module:

```rust
    #[test]
    fn audit_entities_use_table_names() {
        assert_eq!(AuditEntity::SmellReport.as_str(), "smell_report");
        assert_eq!(AuditEntity::Pm25Reading.as_str(), "pm25_reading");
        assert_eq!("event".parse::<AuditEntity>(), Ok(AuditEntity::Event));
        assert!("events".parse::<AuditEntity>().is_err());
    }

    #[test]
    fn invites_are_the_only_entity_without_a_snapshot() {
        let without: Vec<_> = AuditEntity::ALL
            .iter()
            .filter(|e| !e.has_snapshot())
            .collect();
        assert_eq!(without, [&AuditEntity::Invite]);
    }

    /// The entity is hashed as its table name, exactly as the free-text field was.
    #[test]
    fn the_entity_serialises_as_its_table_name() {
        let value = serde_json::to_value(record(1, json!({}))).unwrap();
        assert_eq!(value["entity"], "smell_report");
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compile errors — `FUTURE_TOLERANCE` and `AuditEntity` are not defined.

- [ ] **Step 3: Implement**

In `crates/domain/src/events.rs`, replace the three threshold functions with constants:

```rust
/// Reports observed longer ago than this count as backdated.
pub const BACKDATE_THRESHOLD: TimeDelta = TimeDelta::minutes(10);

/// Phone clocks drift; times this far ahead of the server still count as valid.
pub const FUTURE_TOLERANCE: TimeDelta = TimeDelta::minutes(1);

/// Consecutive reports in one event further apart than this get a warning.
pub const LARGE_GAP: TimeDelta = TimeDelta::hours(3);
```

Replace every call: `future_tolerance()` → `FUTURE_TOLERANCE`, `backdate_threshold()` →
`BACKDATE_THRESHOLD`, `large_gap()` → `LARGE_GAP`, and in the `large_gap_indices` doc comment
``[`large_gap`]`` → ``[`LARGE_GAP`]``. (`TimeDelta::minutes` is a `const fn` in chrono 0.4.45;
verified.)

Replace the doc comment of `new_event_state` with:

```rust
/// At most one event is open at a time, and nothing stays open across a 04:00 boundary.
///
/// When the user answers "No, it had stopped: new event" to the same-smell prompt, close the
/// open event first and then pass `another_event_open = false`; otherwise the new event is
/// wrongly created already closed.
```

In `crates/domain/src/audit.rs`, change the import and add the enum after `AuditAction`:

```rust
use crate::types::{UserId, str_enum};
```

stays as it is; add:

```rust
str_enum!(
    /// The kind of record an audit entry describes, named after its table.
    AuditEntity {
        User => "user",
        Invite => "invite",
        Room => "room",
        Symptom => "symptom",
        Event => "event",
        SmellReport => "smell_report",
        Pm25Reading => "pm25_reading",
    }
);

impl AuditEntity {
    /// Whether `verify-audit` compares current records against their audited snapshots.
    /// Invites are short-lived, are deleted by the daily cleanup and are not evidence.
    pub fn has_snapshot(self) -> bool {
        self != AuditEntity::Invite
    }
}
```

Change the field in `AuditRecord`:

```rust
    pub entity: AuditEntity,
```

and in the `tests` module's `record` helper: `entity: AuditEntity::SmellReport,`.

In `crates/domain/src/types.rs`, delete the line `use crate::DomainError;` inside `mod tests`
(`use super::*;` already brings it in).

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all tests pass (61 existing + 5 new), no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/events.rs crates/domain/src/audit.rs crates/domain/src/types.rs
git commit -m "refactor(domain): const TimeDelta thresholds and a closed AuditEntity enum

Folds in the milestone-1 review minors: thresholds become pub const, edge tests
pin the inclusive one-minute tolerance, new_event_state documents the
close-first order, and audit entries name their entity with a str_enum
instead of free text. The serialised form (and so every hash) is unchanged.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Domain — usernames, list names and expiry rules

**Files:**
- Modify: `crates/domain/src/error.rs`
- Modify: `crates/domain/src/types.rs`
- Modify: `crates/domain/src/permissions.rs`

**Interfaces:**
- Consumes: `Role` (types.rs).
- Produces:
  - `smoke_domain::types::Username` — `new(&str) -> Result<Self, DomainError>`, `as_str(&self) -> &str`,
    `Display`, `Serialize` (transparent string), `Ord`, `Hash`. Trimmed, lowercased; 3–32 chars of
    `a–z 0–9 . _ -`.
  - `smoke_domain::types::ListName` — `new(&str) -> Result<Self, DomainError>`, `as_str`,
    `Display`, `Serialize`. Trimmed; 1–60 characters.
  - `smoke_domain::permissions::check_expiry(role: Role, expires_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Result<(), DomainError>`.
  - New `DomainError` variants: `InvalidUsername(String)`, `InvalidListName(String)`,
    `ResidentsNeverExpire`, `GuestsMustExpire`, `ExpiryNotInFuture`, `NoteTooLong(usize)`.

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` in `crates/domain/src/types.rs`:

```rust
    #[test]
    fn usernames_are_normalised() {
        assert_eq!(Username::new("  Daniele ").unwrap().as_str(), "daniele");
        assert_eq!(
            Username::new("Daniele").unwrap(),
            Username::new("daniele").unwrap()
        );
        assert_eq!(Username::new("guest.2026_a-b").unwrap().to_string(), "guest.2026_a-b");
    }

    #[test]
    fn usernames_reject_bad_lengths_and_characters() {
        for bad in ["ab", "", "   ", "has space", "ünïcode", "semi;colon", &"x".repeat(33)] {
            assert_eq!(
                Username::new(bad),
                Err(DomainError::InvalidUsername(bad.to_owned())),
                "{bad:?}"
            );
        }
        assert!(Username::new(&"x".repeat(32)).is_ok());
        assert!(Username::new("abc").is_ok());
    }

    #[test]
    fn list_names_are_trimmed_and_bounded() {
        assert_eq!(ListName::new("  Balcony ").unwrap().as_str(), "Balcony");
        assert_eq!(ListName::new("Çalışma odası").unwrap().as_str(), "Çalışma odası");
        assert!(ListName::new(&"ç".repeat(60)).is_ok());
        for bad in ["", "   ", &"x".repeat(61)] {
            assert_eq!(
                ListName::new(bad),
                Err(DomainError::InvalidListName(bad.to_owned()))
            );
        }
    }
```

Add to `mod tests` in `crates/domain/src/permissions.rs`:

```rust
    #[test]
    fn expiry_rules_follow_the_role() {
        let later = Some(now() + TimeDelta::days(7));
        assert_eq!(check_expiry(Role::Resident, None, now()), Ok(()));
        assert_eq!(
            check_expiry(Role::Resident, later, now()),
            Err(DomainError::ResidentsNeverExpire)
        );
        assert_eq!(check_expiry(Role::Guest, later, now()), Ok(()));
        assert_eq!(
            check_expiry(Role::Guest, None, now()),
            Err(DomainError::GuestsMustExpire)
        );
        assert_eq!(check_expiry(Role::Viewer, None, now()), Ok(()));
        assert_eq!(check_expiry(Role::Viewer, later, now()), Ok(()));
    }

    #[test]
    fn a_new_expiry_must_be_in_the_future() {
        assert_eq!(
            check_expiry(Role::Guest, Some(now()), now()),
            Err(DomainError::ExpiryNotInFuture)
        );
        assert_eq!(
            check_expiry(Role::Viewer, Some(now() - TimeDelta::seconds(1)), now()),
            Err(DomainError::ExpiryNotInFuture)
        );
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compile errors — `Username`, `ListName`, `check_expiry` not found.

- [ ] **Step 3: Implement**

Add to `DomainError` in `crates/domain/src/error.rs` (after `CloseInFuture`):

```rust
    #[error("a username needs 3–32 characters: letters, digits, '.', '_' or '-'; got {0:?}")]
    InvalidUsername(String),

    #[error("a name needs 1–60 characters, got {0:?}")]
    InvalidListName(String),

    #[error("residents never expire")]
    ResidentsNeverExpire,

    #[error("guests always need an expiry date")]
    GuestsMustExpire,

    #[error("the expiry date must be in the future")]
    ExpiryNotInFuture,

    #[error("a note can be at most 2000 characters, got {0}")]
    NoteTooLong(usize),
```

Add to `crates/domain/src/types.rs`, after the `Locale` enum:

```rust
/// A login name. Construction trims and lowercases it, so "Daniele " and "daniele" are the same
/// account.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Username(String);

impl Username {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 32;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let name = raw.trim().to_ascii_lowercase();
        let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || "._-".contains(c);
        if (Self::MIN_LEN..=Self::MAX_LEN).contains(&name.len()) && name.chars().all(allowed) {
            Ok(Self(name))
        } else {
            Err(DomainError::InvalidUsername(raw.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The name of a room or symptom, as typed by a resident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ListName(String);

impl ListName {
    pub const MAX_CHARS: usize = 60;

    pub fn new(raw: &str) -> Result<Self, DomainError> {
        let name = raw.trim();
        if name.is_empty() || name.chars().count() > Self::MAX_CHARS {
            Err(DomainError::InvalidListName(raw.to_owned()))
        } else {
            Ok(Self(name.to_owned()))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ListName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
```

(`"ünïcode"` fails because `to_ascii_lowercase` leaves non-ASCII characters unchanged and they are
not in the allowed set; `&"x".repeat(33)` fails on length.)

Add to `crates/domain/src/permissions.rs`, after `impl Actor`:

```rust
/// Residents never expire; guests always do; viewers may. A new expiry must lie in the future.
pub fn check_expiry(
    role: Role,
    expires_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Result<(), DomainError> {
    match (role, expires_at) {
        (Role::Resident, Some(_)) => Err(DomainError::ResidentsNeverExpire),
        (Role::Guest, None) => Err(DomainError::GuestsMustExpire),
        (_, Some(at)) if at <= now => Err(DomainError::ExpiryNotInFuture),
        _ => Ok(()),
    }
}
```

and add `use crate::DomainError;` to its imports.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: PASS, no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src
git commit -m "feat(domain): Username and ListName newtypes normalise input; check_expiry encodes role rules

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Domain — stored records and their audit snapshots

**Files:**
- Create: `crates/domain/src/records.rs`
- Modify: `crates/domain/src/lib.rs` (add `pub mod records;`)

**Interfaces:**
- Consumes: `Username`, `ListName`, ids, `Role`, `Locale`, `RoomArea`, `CloseReason`, `Intensity`,
  `Pm25` (types.rs); `AuditEntity` (Task 1); `Actor` (permissions.rs); `EventSummary` (events.rs).
- Produces (all in `smoke_domain::records`):
  - `trait Audited: Serialize { const ENTITY: AuditEntity; fn audit_id(&self) -> i64; fn snapshot(&self) -> serde_json::Value }`
  - `PasswordHash` — `new(String)`, `as_str() -> &str`, redacted `Debug`.
  - `User { id: UserId, username: Username, password_hash: Option<PasswordHash>, role: Role, locale: Locale, expires_at: Option<DateTime<Utc>>, created_at: DateTime<Utc> }` + `fn actor(&self) -> Actor`
  - `Room { id: RoomId, name: ListName, i18n_key: Option<String>, area: RoomArea, hidden_at: Option<DateTime<Utc>> }`
  - `Symptom { id: SymptomId, name: ListName, i18n_key: Option<String>, hidden_at: Option<DateTime<Utc>> }`
  - `Closure { at: DateTime<Utc>, reason: CloseReason }` (Copy)
  - `Event { id: EventId, owner_id: UserId, opened_at: DateTime<Utc>, closed: Option<Closure>, auto_close_notice_seen_at: Option<DateTime<Utc>>, recorded_at: DateTime<Utc> }` + `is_open()`, `summary() -> EventSummary`
  - `ReportValues` — `new(Intensity, Vec<RoomId>, Vec<SymptomId>, Option<&str>) -> Result<Self, DomainError>`; getters `intensity()`, `rooms() -> &[RoomId]`, `symptoms() -> &[SymptomId]`, `note() -> Option<&str>`; `NOTE_MAX_CHARS = 2000`
  - `SmellReport { id: ReportId, event_id: EventId, user_id: UserId, observed_at, recorded_at, values: ReportValues, deleted_at: Option<DateTime<Utc>> }` + `is_deleted()`
  - `Pm25Reading { id: ReadingId, user_id: UserId, event_id: Option<EventId>, observed_at, recorded_at, value: Pm25, deleted_at: Option<DateTime<Utc>> }` + `is_deleted()`
  - Inputs: `NewUser { username, password_hash: Option<PasswordHash>, role, locale, expires_at }`,
    `NewRoom { name: ListName, i18n_key: Option<String>, area: RoomArea }`,
    `NewSymptom { name: ListName, i18n_key: Option<String> }`,
    `NewEvent { owner_id: UserId, opened_at, closed: Option<Closure> }`,
    `NewReport { event_id, user_id, observed_at, values: ReportValues }`,
    `NewReading { user_id, event_id: Option<EventId>, observed_at, value: Pm25 }`.

- [ ] **Step 1: Write the failing tests**

Create `crates/domain/src/records.rs` containing only the test module for now:

```rust
#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use serde_json::json;

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn user(password: Option<&str>) -> User {
        User {
            id: UserId(1),
            username: Username::new("daniele").unwrap(),
            password_hash: password.map(|p| PasswordHash::new(p.to_owned())),
            role: Role::Resident,
            locale: Locale::It,
            expires_at: None,
            created_at: t("2026-01-15T20:00:00Z"),
        }
    }

    fn values(rooms: Vec<i64>, note: Option<&str>) -> ReportValues {
        ReportValues::new(
            Intensity::new(6).unwrap(),
            rooms.into_iter().map(RoomId).collect(),
            vec![SymptomId(2), SymptomId(1), SymptomId(2)],
            note,
        )
        .unwrap()
    }

    #[test]
    fn user_snapshot_hides_the_password_hash() {
        let snapshot = user(Some("argon-hash-value")).snapshot();
        assert_eq!(snapshot["has_password"], true);
        assert!(!snapshot.to_string().contains("argon-hash-value"));
        assert_eq!(user(None).snapshot()["has_password"], false);
    }

    #[test]
    fn password_hash_debug_is_redacted() {
        let printed = format!("{:?}", user(Some("argon-hash-value")));
        assert!(!printed.contains("argon-hash-value"), "{printed}");
        assert!(printed.contains("<redacted>"));
    }

    #[test]
    fn user_snapshot_has_exactly_the_audited_fields() {
        assert_eq!(
            user(Some("x")).snapshot(),
            json!({
                "id": 1,
                "username": "daniele",
                "has_password": true,
                "role": "resident",
                "locale": "it",
                "expires_at": null,
                "created_at": "2026-01-15T20:00:00Z",
            })
        );
    }

    #[test]
    fn report_values_sort_and_dedup_ids_and_trim_the_note() {
        let v = values(vec![3, 1, 3], Some("  smell from the balcony "));
        assert_eq!(v.rooms(), [RoomId(1), RoomId(3)]);
        assert_eq!(v.symptoms(), [SymptomId(1), SymptomId(2)]);
        assert_eq!(v.note(), Some("smell from the balcony"));
    }

    #[test]
    fn a_blank_note_becomes_none() {
        assert_eq!(values(vec![], Some("   ")).note(), None);
        assert_eq!(values(vec![], None).note(), None);
    }

    #[test]
    fn notes_are_limited_in_characters_not_bytes() {
        let at_limit = "ç".repeat(ReportValues::NOTE_MAX_CHARS);
        assert!(ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&at_limit)).is_ok());
        let over = "x".repeat(ReportValues::NOTE_MAX_CHARS + 1);
        assert_eq!(
            ReportValues::new(Intensity::new(1).unwrap(), vec![], vec![], Some(&over)),
            Err(DomainError::NoteTooLong(2001))
        );
    }

    #[test]
    fn report_snapshot_is_flat() {
        let report = SmellReport {
            id: ReportId(9),
            event_id: EventId(4),
            user_id: UserId(1),
            observed_at: t("2026-01-15T19:00:00Z"),
            recorded_at: t("2026-01-15T19:00:00.123456Z"),
            values: values(vec![2], None),
            deleted_at: None,
        };
        assert_eq!(
            report.snapshot(),
            json!({
                "id": 9,
                "event_id": 4,
                "user_id": 1,
                "observed_at": "2026-01-15T19:00:00Z",
                "recorded_at": "2026-01-15T19:00:00.123456Z",
                "intensity": 6,
                "rooms": [2],
                "symptoms": [1, 2],
                "note": null,
                "deleted_at": null,
            })
        );
        assert_eq!(report.audit_id(), 9);
    }

    #[test]
    fn an_event_is_open_until_it_has_a_closure() {
        let mut event = Event {
            id: EventId(4),
            owner_id: UserId(1),
            opened_at: t("2026-01-15T18:05:00Z"),
            closed: None,
            auto_close_notice_seen_at: None,
            recorded_at: t("2026-01-15T18:05:01Z"),
        };
        assert!(event.is_open());
        assert_eq!(event.summary().opened_at, t("2026-01-15T18:05:00Z"));
        event.closed = Some(Closure {
            at: event.opened_at + TimeDelta::hours(1),
            reason: CloseReason::Manual,
        });
        assert!(!event.is_open());
        assert_eq!(
            event.snapshot()["closed"],
            json!({ "at": "2026-01-15T19:05:00Z", "reason": "manual" })
        );
    }

    #[test]
    fn each_record_names_its_entity() {
        assert_eq!(User::ENTITY, AuditEntity::User);
        assert_eq!(Room::ENTITY, AuditEntity::Room);
        assert_eq!(Symptom::ENTITY, AuditEntity::Symptom);
        assert_eq!(Event::ENTITY, AuditEntity::Event);
        assert_eq!(SmellReport::ENTITY, AuditEntity::SmellReport);
        assert_eq!(Pm25Reading::ENTITY, AuditEntity::Pm25Reading);
    }

    #[test]
    fn user_converts_to_its_permission_actor() {
        let actor = user(None).actor();
        assert_eq!(actor.id, UserId(1));
        assert_eq!(actor.role, Role::Resident);
    }
}
```

Add `pub mod records;` to `crates/domain/src/lib.rs` (keep modules alphabetical).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain records`
Expected: compile errors — `User`, `ReportValues`, … not found.

- [ ] **Step 3: Implement**

Put this above the test module in `crates/domain/src/records.rs`:

```rust
//! Stored records and the inputs that create them. Each record serialises to the snapshot the
//! audit log keeps of it.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Serialize, Serializer};
use serde_json::Value;

use crate::DomainError;
use crate::audit::AuditEntity;
use crate::events::EventSummary;
use crate::permissions::Actor;
use crate::types::{
    CloseReason, EventId, Intensity, ListName, Locale, Pm25, ReadingId, ReportId, Role, RoomArea,
    RoomId, SymptomId, UserId, Username,
};

/// A record the audit log keeps snapshots of.
pub trait Audited: Serialize {
    const ENTITY: AuditEntity;

    fn audit_id(&self) -> i64;

    fn snapshot(&self) -> Value {
        serde_json::to_value(self).expect("records always serialise")
    }
}

/// An Argon2id hash in PHC string format. Never printed and never audited.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(String);

impl PasswordHash {
    pub fn new(phc: String) -> Self {
        Self(phc)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasswordHash(<redacted>)")
    }
}

fn serialize_is_some<T, S: Serializer>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bool(value.is_some())
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    /// Audited only as whether one is set.
    #[serde(rename = "has_password", serialize_with = "serialize_is_some")]
    pub password_hash: Option<PasswordHash>,
    pub role: Role,
    pub locale: Locale,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn actor(&self) -> Actor {
        Actor {
            id: self.id,
            role: self.role,
            expires_at: self.expires_at,
        }
    }
}

impl Audited for User {
    const ENTITY: AuditEntity = AuditEntity::User;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Room {
    pub id: RoomId,
    pub name: ListName,
    /// Set for starter rooms, which are shown translated; custom rooms are shown as typed.
    pub i18n_key: Option<String>,
    pub area: RoomArea,
    pub hidden_at: Option<DateTime<Utc>>,
}

impl Audited for Room {
    const ENTITY: AuditEntity = AuditEntity::Room;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Symptom {
    pub id: SymptomId,
    pub name: ListName,
    pub i18n_key: Option<String>,
    pub hidden_at: Option<DateTime<Utc>>,
}

impl Audited for Symptom {
    const ENTITY: AuditEntity = AuditEntity::Symptom;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// How and when an event ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Closure {
    pub at: DateTime<Utc>,
    pub reason: CloseReason,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Event {
    pub id: EventId,
    pub owner_id: UserId,
    pub opened_at: DateTime<Utc>,
    pub closed: Option<Closure>,
    pub auto_close_notice_seen_at: Option<DateTime<Utc>>,
    pub recorded_at: DateTime<Utc>,
}

impl Event {
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }

    pub fn summary(&self) -> EventSummary {
        EventSummary {
            id: self.id,
            opened_at: self.opened_at,
        }
    }
}

impl Audited for Event {
    const ENTITY: AuditEntity = AuditEntity::Event;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

/// What a smell report says. Room and symptom ids are kept sorted and unique, and a blank note
/// is no note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReportValues {
    intensity: Intensity,
    rooms: Vec<RoomId>,
    symptoms: Vec<SymptomId>,
    note: Option<String>,
}

impl ReportValues {
    pub const NOTE_MAX_CHARS: usize = 2000;

    pub fn new(
        intensity: Intensity,
        mut rooms: Vec<RoomId>,
        mut symptoms: Vec<SymptomId>,
        note: Option<&str>,
    ) -> Result<Self, DomainError> {
        rooms.sort_unstable();
        rooms.dedup();
        symptoms.sort_unstable();
        symptoms.dedup();
        let note = note.map(str::trim).filter(|n| !n.is_empty());
        if let Some(n) = note {
            let chars = n.chars().count();
            if chars > Self::NOTE_MAX_CHARS {
                return Err(DomainError::NoteTooLong(chars));
            }
        }
        Ok(Self {
            intensity,
            rooms,
            symptoms,
            note: note.map(str::to_owned),
        })
    }

    pub fn intensity(&self) -> Intensity {
        self.intensity
    }

    pub fn rooms(&self) -> &[RoomId] {
        &self.rooms
    }

    pub fn symptoms(&self) -> &[SymptomId] {
        &self.symptoms
    }

    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SmellReport {
    pub id: ReportId,
    pub event_id: EventId,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    #[serde(flatten)]
    pub values: ReportValues,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl SmellReport {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Audited for SmellReport {
    const ENTITY: AuditEntity = AuditEntity::SmellReport;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pm25Reading {
    pub id: ReadingId,
    pub user_id: UserId,
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub recorded_at: DateTime<Utc>,
    pub value: Pm25,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Pm25Reading {
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

impl Audited for Pm25Reading {
    const ENTITY: AuditEntity = AuditEntity::Pm25Reading;

    fn audit_id(&self) -> i64 {
        self.id.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewUser {
    pub username: Username,
    /// `None` until the invitee sets a password.
    pub password_hash: Option<PasswordHash>,
    pub role: Role,
    pub locale: Locale,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewRoom {
    pub name: ListName,
    pub i18n_key: Option<String>,
    pub area: RoomArea,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewSymptom {
    pub name: ListName,
    pub i18n_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewEvent {
    pub owner_id: UserId,
    pub opened_at: DateTime<Utc>,
    /// `Some` for an event created already over (a backdated report).
    pub closed: Option<Closure>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewReport {
    pub event_id: EventId,
    pub user_id: UserId,
    pub observed_at: DateTime<Utc>,
    pub values: ReportValues,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewReading {
    pub user_id: UserId,
    pub event_id: Option<EventId>,
    pub observed_at: DateTime<Utc>,
    pub value: Pm25,
}
```

If the compiler rejects `serialize_with` on a field of type `Option<PasswordHash>` because the
helper is generic, make it concrete: `fn serialize_is_some<S: Serializer>(value: &Option<PasswordHash>, serializer: S)`.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: PASS. Also run `cargo test -p smoke-domain --lib --features serde_json/preserve_order` —
the `json!` comparisons are map equality, so they pass either way.

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/records.rs crates/domain/src/lib.rs
git commit -m "feat(domain): record types whose serde form is their audit snapshot

Serialize is the snapshot: serialize_with turns the password hash into a
has_password flag, #[serde(flatten)] keeps report values at the top level.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Domain — the storage traits

**Files:**
- Create: `crates/domain/src/repos.rs`
- Modify: `crates/domain/src/lib.rs` (add `pub mod repos;`)

**Interfaces:**
- Consumes: records (Task 3), `ChainedEntry`/`AuditEntity` (audit.rs), `DomainError`.
  `Snapshot` is defined in Task 5; to keep this task compiling on its own, `SnapshotRepo` is added
  in Task 5 too.
- Produces (all in `smoke_domain::repos`):
  - `enum Conflict { UsernameTaken, AnotherEventOpen, EventNotOpen, EventNotClosed, AlreadyDeleted, UnknownReference, Duplicate, ChainForked }` (`Copy`, `PartialEq`, `thiserror::Error`).
  - `enum StoreError { NotFound { entity: AuditEntity, id: i64 }, Conflict(Conflict), Invalid(DomainError), Corrupt(String), Backend(Box<dyn Error + Send + Sync>) }` with `From<Conflict>` and `From<DomainError>`; `type StoreResult<T> = Result<T, StoreError>`.
  - `trait Transactional: Send + Sync { type Tx: Commit; fn begin(&self, actor: Option<UserId>) -> impl Future<Output = StoreResult<Self::Tx>> + Send; }`
  - `trait Commit: Send { fn commit(self) -> impl Future<Output = StoreResult<()>> + Send; }`
  - Read traits (`&self`, on the store): `UserRepo`, `ListRepo`, `EventRepo`, `ReportRepo`,
    `ReadingRepo`, `JobRepo`, `AuditRepo` — signatures below.
  - Write traits (`&mut self`, on the transaction): `UserWrites`, `ListWrites`, `EventWrites`,
    `ReportWrites`, `ReadingWrites`, `JobWrites` — signatures below.
  - Alias traits with blanket impls: `WriteTx` (all write traits + `Commit`) and `Store`
    (`Transactional<Tx: WriteTx>` + all read traits + `SnapshotRepo`); `Store` is completed in Task 5.

- [ ] **Step 1: Write the failing test**

Create `crates/domain/src/repos.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conflicts_and_rule_violations_convert_into_store_errors() {
        let conflict: StoreError = Conflict::AnotherEventOpen.into();
        assert!(matches!(conflict, StoreError::Conflict(Conflict::AnotherEventOpen)));
        assert_eq!(conflict.to_string(), "another event is already open");

        let invalid: StoreError = DomainError::GuestsMustExpire.into();
        assert!(matches!(invalid, StoreError::Invalid(DomainError::GuestsMustExpire)));
        assert_eq!(invalid.to_string(), "guests always need an expiry date");
    }

    #[test]
    fn not_found_names_the_entity_and_id() {
        let error = StoreError::NotFound {
            entity: AuditEntity::SmellReport,
            id: 4,
        };
        assert_eq!(error.to_string(), "smell_report 4 does not exist");
    }

    #[test]
    fn backend_errors_keep_their_source() {
        let error = StoreError::Backend("disk full".into());
        assert_eq!(error.to_string(), "database error");
        assert_eq!(
            std::error::Error::source(&error).unwrap().to_string(),
            "disk full"
        );
    }
}
```

Add `pub mod repos;` to `lib.rs`.

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p smoke-domain repos`
Expected: compile errors — `StoreError`, `Conflict` not found.

- [ ] **Step 3: Implement**

Put above the tests in `crates/domain/src/repos.rs`:

```rust
//! The storage boundary: traits the database crates implement and the rest of the app uses.
//!
//! Reads go through the store. Writes go through a transaction from [`Transactional::begin`]:
//! every write method appends its own audit entry inside that transaction, so data cannot change
//! without being recorded. Dropping a transaction without [`Commit::commit`] rolls it back.

use std::error::Error as StdError;
use std::future::Future;

use chrono::{DateTime, Utc};
use thiserror::Error;

use crate::DomainError;
use crate::audit::{AuditEntity, ChainedEntry};
use crate::records::{
    Closure, Event, NewEvent, NewReading, NewReport, NewRoom, NewSymptom, NewUser, PasswordHash,
    Pm25Reading, ReportValues, Room, SmellReport, Symptom, User,
};
use crate::types::{EventId, Locale, Pm25, ReadingId, ReportId, RoomId, SymptomId, UserId, Username};

/// A write the current data does not allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum Conflict {
    #[error("that username is taken")]
    UsernameTaken,
    #[error("another event is already open")]
    AnotherEventOpen,
    #[error("the event is not open")]
    EventNotOpen,
    #[error("the event is not closed")]
    EventNotClosed,
    #[error("the entry has been deleted")]
    AlreadyDeleted,
    #[error("a referenced user, event, room or symptom does not exist")]
    UnknownReference,
    #[error("an identical entry already exists")]
    Duplicate,
    #[error("another write extended the audit log at the same time")]
    ChainForked,
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("{entity} {id} does not exist")]
    NotFound { entity: AuditEntity, id: i64 },
    #[error(transparent)]
    Conflict(#[from] Conflict),
    #[error(transparent)]
    Invalid(#[from] DomainError),
    #[error("stored data is corrupt: {0}")]
    Corrupt(String),
    #[error("database error")]
    Backend(#[source] Box<dyn StdError + Send + Sync>),
}

pub type StoreResult<T> = Result<T, StoreError>;

/// A store that can start write transactions.
pub trait Transactional: Send + Sync {
    type Tx: Commit;

    /// Starts a write transaction whose audit entries name `actor` (`None` for the system:
    /// the sweep, seeding, the CLI). Write transactions run one at a time, so do not call
    /// `begin` again while holding one on the same task: it waits for the first to finish.
    fn begin(&self, actor: Option<UserId>) -> impl Future<Output = StoreResult<Self::Tx>> + Send;
}

pub trait Commit: Send {
    fn commit(self) -> impl Future<Output = StoreResult<()>> + Send;
}

pub trait UserRepo: Send + Sync {
    fn user(&self, id: UserId) -> impl Future<Output = StoreResult<Option<User>>> + Send;
    fn user_by_username(
        &self,
        username: &Username,
    ) -> impl Future<Output = StoreResult<Option<User>>> + Send;
    /// Every user, ordered by username.
    fn users(&self) -> impl Future<Output = StoreResult<Vec<User>>> + Send;
}

pub trait ListRepo: Send + Sync {
    /// Every room, hidden ones included, ordered by id.
    fn rooms(&self) -> impl Future<Output = StoreResult<Vec<Room>>> + Send;
    /// Every symptom, hidden ones included, ordered by id.
    fn symptoms(&self) -> impl Future<Output = StoreResult<Vec<Symptom>>> + Send;
}

pub trait EventRepo: Send + Sync {
    fn event(&self, id: EventId) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    fn open_event(&self) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    /// Events whose span overlaps `[from, to)`, an open event counting as still running;
    /// ordered by `opened_at`.
    fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Vec<Event>>> + Send;
}

pub trait ReportRepo: Send + Sync {
    /// A report, deleted or not.
    fn report(&self, id: ReportId) -> impl Future<Output = StoreResult<Option<SmellReport>>> + Send;
    /// The event's reports that are not deleted, ordered by observed time.
    fn reports_for_event(
        &self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Vec<SmellReport>>> + Send;
}

pub trait ReadingRepo: Send + Sync {
    /// A reading, deleted or not.
    fn reading(&self, id: ReadingId) -> impl Future<Output = StoreResult<Option<Pm25Reading>>> + Send;
    /// The event's readings that are not deleted, ordered by observed time.
    fn readings_for_event(
        &self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Vec<Pm25Reading>>> + Send;
}

pub trait JobRepo: Send + Sync {
    fn job_last_run(&self, name: &str) -> impl Future<Output = StoreResult<Option<DateTime<Utc>>>> + Send;
}

pub trait AuditRepo: Send + Sync {
    /// The whole chain, oldest first.
    fn audit_log(&self) -> impl Future<Output = StoreResult<Vec<ChainedEntry>>> + Send;
    /// One record's entries, oldest first: its History view.
    fn audit_history(
        &self,
        entity: AuditEntity,
        entity_id: i64,
    ) -> impl Future<Output = StoreResult<Vec<ChainedEntry>>> + Send;
}

pub trait UserWrites: Send {
    /// Fails with [`DomainError`] if the expiry breaks the role rules
    /// ([`crate::permissions::check_expiry`]) and with [`Conflict::UsernameTaken`].
    fn create_user(&mut self, new: NewUser) -> impl Future<Output = StoreResult<User>> + Send;
    /// Always audited (as `password_change`), although the snapshot only shows `has_password`.
    fn set_password(
        &mut self,
        id: UserId,
        hash: PasswordHash,
    ) -> impl Future<Output = StoreResult<User>> + Send;
    fn set_locale(&mut self, id: UserId, locale: Locale) -> impl Future<Output = StoreResult<User>> + Send;
    /// Sets `expires_at` to now. Residents cannot expire.
    fn expire_user(&mut self, id: UserId) -> impl Future<Output = StoreResult<User>> + Send;
    fn reactivate_user(
        &mut self,
        id: UserId,
        expires_at: Option<DateTime<Utc>>,
    ) -> impl Future<Output = StoreResult<User>> + Send;
}

pub trait ListWrites: Send {
    fn create_room(&mut self, new: NewRoom) -> impl Future<Output = StoreResult<Room>> + Send;
    /// Hiding keeps the first `hidden_at`; unhiding clears it.
    fn set_room_hidden(&mut self, id: RoomId, hidden: bool) -> impl Future<Output = StoreResult<Room>> + Send;
    fn create_symptom(&mut self, new: NewSymptom) -> impl Future<Output = StoreResult<Symptom>> + Send;
    fn set_symptom_hidden(
        &mut self,
        id: SymptomId,
        hidden: bool,
    ) -> impl Future<Output = StoreResult<Symptom>> + Send;
}

pub trait EventWrites: Send {
    /// Reads inside the transaction, so a decision and the write that follows it see the same data.
    fn open_event(&mut self) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    fn event(&mut self, id: EventId) -> impl Future<Output = StoreResult<Option<Event>>> + Send;
    /// The observed time of the event's latest report that is not deleted.
    fn latest_report_at(
        &mut self,
        event: EventId,
    ) -> impl Future<Output = StoreResult<Option<DateTime<Utc>>>> + Send;
    /// Fails with [`Conflict::AnotherEventOpen`] when creating an open event while one is open.
    fn create_event(&mut self, new: NewEvent) -> impl Future<Output = StoreResult<Event>> + Send;
    fn close_event(&mut self, id: EventId, closure: Closure) -> impl Future<Output = StoreResult<Event>> + Send;
    fn reopen_event(&mut self, id: EventId) -> impl Future<Output = StoreResult<Event>> + Send;
    /// Moves a closed event's end, keeping its close reason (e.g. a backdated report after it).
    fn move_event_end(
        &mut self,
        id: EventId,
        closed_at: DateTime<Utc>,
    ) -> impl Future<Output = StoreResult<Event>> + Send;
    /// Records that the auto-close notice was shown; later calls keep the first time.
    fn mark_auto_close_notice_seen(&mut self, id: EventId) -> impl Future<Output = StoreResult<Event>> + Send;
}

pub trait ReportWrites: Send {
    fn report(&mut self, id: ReportId) -> impl Future<Output = StoreResult<Option<SmellReport>>> + Send;
    /// `recorded_at` is set to now by the store.
    fn create_report(&mut self, new: NewReport) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    /// "Correct a mistake". Deleted reports cannot be corrected.
    fn correct_report(
        &mut self,
        id: ReportId,
        observed_at: DateTime<Utc>,
        values: ReportValues,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    fn move_report(
        &mut self,
        id: ReportId,
        to: EventId,
    ) -> impl Future<Output = StoreResult<SmellReport>> + Send;
    /// Soft delete: sets `deleted_at`.
    fn delete_report(&mut self, id: ReportId) -> impl Future<Output = StoreResult<SmellReport>> + Send;
}

pub trait ReadingWrites: Send {
    fn create_reading(&mut self, new: NewReading) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
    fn correct_reading(
        &mut self,
        id: ReadingId,
        observed_at: DateTime<Utc>,
        value: Pm25,
    ) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
    /// Soft delete: sets `deleted_at`.
    fn delete_reading(&mut self, id: ReadingId) -> impl Future<Output = StoreResult<Pm25Reading>> + Send;
}

/// Bookkeeping for background jobs. Not audited: it is not evidence and changes daily.
pub trait JobWrites: Send {
    fn record_job_run(&mut self, name: &str, at: DateTime<Utc>) -> impl Future<Output = StoreResult<()>> + Send;
}

/// Everything a write transaction can do.
pub trait WriteTx:
    Commit + UserWrites + ListWrites + EventWrites + ReportWrites + ReadingWrites + JobWrites
{
}

impl<T> WriteTx for T where
    T: Commit + UserWrites + ListWrites + EventWrites + ReportWrites + ReadingWrites + JobWrites
{
}
```

(`Store` is added in Task 5, once `SnapshotRepo` exists.) Run `cargo fmt` afterwards; it will
re-wrap the long signatures.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: PASS, no warnings (unused imports would be warnings: every import above is used by a
signature).

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/repos.rs crates/domain/src/lib.rs
git commit -m "feat(domain): storage traits with async fn in traits (impl Future + Send)

Reads live on the store, writes on a transaction from begin(actor), so the
audit entry can be appended inside every write. Return-position impl Future
with an explicit Send bound keeps the traits usable from multi-threaded Axum.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Domain — audit verification and the starter lists

**Files:**
- Create: `crates/domain/src/verify.rs`
- Create: `crates/domain/src/starter.rs`
- Modify: `crates/domain/src/repos.rs` (add `SnapshotRepo` and `Store`)
- Modify: `crates/domain/src/lib.rs` (add `pub mod starter; pub mod verify;`)

**Interfaces:**
- Consumes: `ChainedEntry`, `verify_chain`, `ChainError`, `GENESIS_HASH`, `AuditEntity`
  (audit.rs); `Audited` and records (Task 3); traits (Task 4).
- Produces:
  - `smoke_domain::verify::Snapshot { entity: AuditEntity, entity_id: i64, value: Value }` + `Snapshot::of<T: Audited>(&T) -> Snapshot`
  - `smoke_domain::verify::SnapshotProblem` — `Unaudited { entity, entity_id }`, `Vanished { entity, entity_id }`, `Differs { entity, entity_id, audit_id }`
  - `smoke_domain::verify::verify_snapshots(entries: &[ChainedEntry], current: &[Snapshot]) -> Vec<SnapshotProblem>` (sorted by entity text, then id)
  - `smoke_domain::verify::AuditReport { entries: usize, head_hash: String, chain: Result<(), ChainError>, problems: Vec<SnapshotProblem> }` + `is_ok()`
  - `smoke_domain::verify::AuditState { entries: Vec<ChainedEntry>, snapshots: Vec<Snapshot> }`
  - `smoke_domain::verify::verify_store<S: SnapshotRepo>(&S) -> StoreResult<AuditReport>`
  - `smoke_domain::repos::SnapshotRepo { fn audit_state(&self) -> impl Future<Output = StoreResult<AuditState>> + Send; }` — the whole audit log plus a snapshot of every user, room, symptom, event, report and reading (deleted ones included), **read in one consistent read transaction**, so a write landing between the two reads cannot cause a false alarm.
  - `smoke_domain::repos::Store` — alias for `Transactional<Tx: WriteTx>` + all read traits + `SnapshotRepo`.
  - `smoke_domain::starter::{StarterRoom, StarterSymptom, STARTER_ROOMS, STARTER_SYMPTOMS, seed_starter_lists}` —
    `seed_starter_lists<S>(store: &S) -> StoreResult<()> where S: Transactional<Tx: ListWrites> + ListRepo`; inserts the six rooms when there are no rooms and the six symptoms when there are no symptoms, in one transaction with actor `None`.

- [ ] **Step 1: Write the failing tests**

Create `crates/domain/src/verify.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::audit::{AuditAction, AuditRecord, ChainError};

    /// A chain built from `(entity, entity_id, action, after)` steps.
    fn chain(steps: &[(AuditEntity, i64, AuditAction, Value)]) -> Vec<ChainedEntry> {
        let mut entries: Vec<ChainedEntry> = Vec::new();
        for (i, (entity, entity_id, action, after)) in steps.iter().enumerate() {
            let prev = entries
                .last()
                .map_or(GENESIS_HASH, |e| e.hash.as_str())
                .to_owned();
            let record = AuditRecord {
                id: i as i64 + 1,
                at_micros: 1_768_500_000_000_000,
                actor_id: None,
                entity: *entity,
                entity_id: *entity_id,
                action: *action,
                before: None,
                after: Some(after.clone()),
            };
            entries.push(ChainedEntry::seal(record, &prev));
        }
        entries
    }

    fn snap(entity: AuditEntity, entity_id: i64, value: Value) -> Snapshot {
        Snapshot {
            entity,
            entity_id,
            value,
        }
    }

    #[test]
    fn records_matching_their_latest_entry_have_no_problems() {
        let entries = chain(&[
            (AuditEntity::Room, 1, AuditAction::Create, json!({ "name": "Kitchen" })),
            (AuditEntity::Room, 1, AuditAction::Update, json!({ "name": "Kitchenette" })),
        ]);
        let current = [snap(AuditEntity::Room, 1, json!({ "name": "Kitchenette" }))];
        assert_eq!(verify_snapshots(&entries, &current), []);
    }

    #[test]
    fn a_record_edited_outside_the_app_differs() {
        let entries = chain(&[(AuditEntity::SmellReport, 7, AuditAction::Create, json!({ "intensity": 6 }))]);
        let current = [snap(AuditEntity::SmellReport, 7, json!({ "intensity": 9 }))];
        assert_eq!(
            verify_snapshots(&entries, &current),
            [SnapshotProblem::Differs {
                entity: AuditEntity::SmellReport,
                entity_id: 7,
                audit_id: 1
            }]
        );
    }

    #[test]
    fn a_record_without_any_entry_is_unaudited() {
        let current = [snap(AuditEntity::Symptom, 3, json!({}))];
        assert_eq!(
            verify_snapshots(&[], &current),
            [SnapshotProblem::Unaudited {
                entity: AuditEntity::Symptom,
                entity_id: 3
            }]
        );
    }

    #[test]
    fn an_audited_record_that_is_gone_has_vanished() {
        let entries = chain(&[(AuditEntity::Pm25Reading, 2, AuditAction::Create, json!({}))]);
        assert_eq!(
            verify_snapshots(&entries, &[]),
            [SnapshotProblem::Vanished {
                entity: AuditEntity::Pm25Reading,
                entity_id: 2
            }]
        );
    }

    #[test]
    fn invites_are_not_compared() {
        let entries = chain(&[(AuditEntity::Invite, 1, AuditAction::Invite, json!({}))]);
        assert_eq!(verify_snapshots(&entries, &[]), []);
    }

    #[test]
    fn problems_are_sorted_by_entity_then_id() {
        let current = [
            snap(AuditEntity::User, 2, json!({})),
            snap(AuditEntity::Event, 9, json!({})),
            snap(AuditEntity::User, 1, json!({})),
        ];
        let ids: Vec<_> = verify_snapshots(&[], &current)
            .iter()
            .map(SnapshotProblem::key)
            .collect();
        assert_eq!(ids, [("event", 9), ("user", 1), ("user", 2)]);
    }

    #[test]
    fn a_report_is_ok_only_with_an_intact_chain_and_no_problems() {
        let ok = AuditReport {
            entries: 0,
            head_hash: GENESIS_HASH.to_owned(),
            chain: Ok(()),
            problems: vec![],
        };
        assert!(ok.is_ok());
        let broken = AuditReport {
            chain: Err(ChainError::BrokenLink { id: 2 }),
            ..ok.clone()
        };
        assert!(!broken.is_ok());
        let tampered = AuditReport {
            problems: vec![SnapshotProblem::Unaudited {
                entity: AuditEntity::Room,
                entity_id: 1,
            }],
            ..ok
        };
        assert!(!tampered.is_ok());
    }
}
```

Create `crates/domain/src/starter.rs` with only:

```rust
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn the_starter_lists_match_the_spec() {
        assert_eq!(STARTER_ROOMS.len(), 6);
        assert_eq!(STARTER_SYMPTOMS.len(), 6);
        let common: Vec<_> = STARTER_ROOMS
            .iter()
            .filter(|r| r.area == RoomArea::Common)
            .map(|r| r.i18n_key)
            .collect();
        assert_eq!(common, ["room-stairwell"]);
    }

    #[test]
    fn starter_keys_are_unique_and_names_are_valid() {
        let rooms = STARTER_ROOMS.iter().map(|r| (r.i18n_key, r.name));
        let symptoms = STARTER_SYMPTOMS.iter().map(|s| (s.i18n_key, s.name));
        let items: Vec<_> = rooms.chain(symptoms).collect();
        let keys: HashSet<_> = items.iter().map(|(key, _)| key).collect();
        assert_eq!(keys.len(), 12);
        for (_, name) in items {
            assert!(ListName::new(name).is_ok(), "{name}");
        }
    }
}
```

Add `pub mod starter;` and `pub mod verify;` to `lib.rs`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compile errors — `verify_snapshots`, `STARTER_ROOMS`, … not found.

- [ ] **Step 3: Implement**

Above the tests in `crates/domain/src/verify.rs`:

```rust
//! What `verify-audit` checks: the hash chain is intact, and every record equals its latest
//! audited snapshot, which catches edits made directly in the database (spec §6.1).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::audit::{AuditEntity, ChainError, ChainedEntry, GENESIS_HASH, verify_chain};
use crate::records::Audited;
use crate::repos::{SnapshotRepo, StoreResult};

/// The audit log and every audited record, read at one moment.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditState {
    pub entries: Vec<ChainedEntry>,
    pub snapshots: Vec<Snapshot>,
}

/// A record as it is stored now.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub entity: AuditEntity,
    pub entity_id: i64,
    pub value: Value,
}

impl Snapshot {
    pub fn of<T: Audited>(record: &T) -> Self {
        Self {
            entity: T::ENTITY,
            entity_id: record.audit_id(),
            value: record.snapshot(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnapshotProblem {
    /// A record exists that no audit entry describes: it was inserted directly.
    Unaudited { entity: AuditEntity, entity_id: i64 },
    /// The audit log describes a record that no longer exists: it was deleted directly.
    Vanished { entity: AuditEntity, entity_id: i64 },
    /// The record differs from its latest audited snapshot: it was edited directly.
    Differs {
        entity: AuditEntity,
        entity_id: i64,
        audit_id: i64,
    },
}

impl SnapshotProblem {
    pub fn key(&self) -> (&'static str, i64) {
        match *self {
            Self::Unaudited { entity, entity_id }
            | Self::Vanished { entity, entity_id }
            | Self::Differs {
                entity, entity_id, ..
            } => (entity.as_str(), entity_id),
        }
    }
}

/// The outcome of a full verification.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditReport {
    pub entries: usize,
    /// The latest hash, to compare with the one pinned in backups.
    pub head_hash: String,
    pub chain: Result<(), ChainError>,
    pub problems: Vec<SnapshotProblem>,
}

impl AuditReport {
    pub fn is_ok(&self) -> bool {
        self.chain.is_ok() && self.problems.is_empty()
    }
}

/// Compares the current records with the latest audit entry of each.
pub fn verify_snapshots(entries: &[ChainedEntry], current: &[Snapshot]) -> Vec<SnapshotProblem> {
    let mut latest: BTreeMap<(&'static str, i64), &ChainedEntry> = BTreeMap::new();
    for entry in entries.iter().filter(|e| e.record.entity.has_snapshot()) {
        latest.insert((entry.record.entity.as_str(), entry.record.entity_id), entry);
    }

    let mut problems = Vec::new();
    let mut present = BTreeSet::new();
    for snapshot in current.iter().filter(|s| s.entity.has_snapshot()) {
        let key = (snapshot.entity.as_str(), snapshot.entity_id);
        present.insert(key);
        match latest.get(&key) {
            None => problems.push(SnapshotProblem::Unaudited {
                entity: snapshot.entity,
                entity_id: snapshot.entity_id,
            }),
            Some(entry) if entry.record.after.as_ref() != Some(&snapshot.value) => {
                problems.push(SnapshotProblem::Differs {
                    entity: snapshot.entity,
                    entity_id: snapshot.entity_id,
                    audit_id: entry.record.id,
                })
            }
            Some(_) => {}
        }
    }
    for (key, entry) in &latest {
        if !present.contains(key) && entry.record.after.is_some() {
            problems.push(SnapshotProblem::Vanished {
                entity: entry.record.entity,
                entity_id: entry.record.entity_id,
            });
        }
    }
    problems.sort_by_key(SnapshotProblem::key);
    problems
}

/// Runs both checks against a store.
pub async fn verify_store<S: SnapshotRepo>(store: &S) -> StoreResult<AuditReport> {
    let AuditState {
        entries,
        snapshots: current,
    } = store.audit_state().await?;
    Ok(AuditReport {
        entries: entries.len(),
        head_hash: entries
            .last()
            .map_or(GENESIS_HASH, |e| e.hash.as_str())
            .to_owned(),
        chain: verify_chain(&entries),
        problems: verify_snapshots(&entries, &current),
    })
}
```

Above the tests in `crates/domain/src/starter.rs`:

```rust
//! The rooms and symptoms a new installation starts with (spec §6). They are shown through
//! their translation keys; the English name is stored as the fallback.

use crate::records::{NewRoom, NewSymptom};
use crate::repos::{Commit, ListRepo, ListWrites, StoreResult, Transactional};
use crate::types::{ListName, RoomArea};

pub struct StarterRoom {
    pub i18n_key: &'static str,
    pub name: &'static str,
    pub area: RoomArea,
}

pub struct StarterSymptom {
    pub i18n_key: &'static str,
    pub name: &'static str,
}

const fn room(i18n_key: &'static str, name: &'static str, area: RoomArea) -> StarterRoom {
    StarterRoom {
        i18n_key,
        name,
        area,
    }
}

const fn symptom(i18n_key: &'static str, name: &'static str) -> StarterSymptom {
    StarterSymptom { i18n_key, name }
}

pub const STARTER_ROOMS: [StarterRoom; 6] = [
    room("room-living-room", "Living room", RoomArea::Inside),
    room("room-kitchen", "Kitchen", RoomArea::Inside),
    room("room-bedroom", "Bedroom", RoomArea::Inside),
    room("room-bathroom", "Bathroom", RoomArea::Inside),
    room("room-entrance-hall", "Entrance hall", RoomArea::Inside),
    room("room-stairwell", "Stairwell (shared)", RoomArea::Common),
];

pub const STARTER_SYMPTOMS: [StarterSymptom; 6] = [
    symptom("symptom-cough", "Cough"),
    symptom("symptom-headache", "Headache"),
    symptom("symptom-sore-throat", "Sore throat"),
    symptom("symptom-eye-irritation", "Eye irritation"),
    symptom("symptom-nausea", "Nausea"),
    symptom("symptom-difficulty-sleeping", "Difficulty sleeping"),
];

/// Adds the starter rooms if there are no rooms and the starter symptoms if there are no
/// symptoms, through one audited transaction. Safe to run on every `migrate`.
pub async fn seed_starter_lists<S>(store: &S) -> StoreResult<()>
where
    S: Transactional<Tx: ListWrites> + ListRepo,
{
    let seed_rooms = store.rooms().await?.is_empty();
    let seed_symptoms = store.symptoms().await?.is_empty();
    if !seed_rooms && !seed_symptoms {
        return Ok(());
    }
    let mut tx = store.begin(None).await?;
    if seed_rooms {
        for room in &STARTER_ROOMS {
            tx.create_room(NewRoom {
                name: ListName::new(room.name)?,
                i18n_key: Some(room.i18n_key.to_owned()),
                area: room.area,
            })
            .await?;
        }
    }
    if seed_symptoms {
        for symptom in &STARTER_SYMPTOMS {
            tx.create_symptom(NewSymptom {
                name: ListName::new(symptom.name)?,
                i18n_key: Some(symptom.i18n_key.to_owned()),
            })
            .await?;
        }
    }
    tx.commit().await
}
```

Append to `crates/domain/src/repos.rs` (before `mod tests`), and add
`use crate::verify::AuditState;` to its imports:

```rust
pub trait SnapshotRepo: Send + Sync {
    /// The audit log and every record with audit snapshots (deleted ones included), read in one
    /// read transaction.
    fn audit_state(&self) -> impl Future<Output = StoreResult<AuditState>> + Send;
}

/// A complete storage backend.
pub trait Store:
    Transactional<Tx: WriteTx>
    + UserRepo
    + ListRepo
    + EventRepo
    + ReportRepo
    + ReadingRepo
    + JobRepo
    + AuditRepo
    + SnapshotRepo
{
}

impl<T> Store for T where
    T: Transactional<Tx: WriteTx>
        + UserRepo
        + ListRepo
        + EventRepo
        + ReportRepo
        + ReadingRepo
        + JobRepo
        + AuditRepo
        + SnapshotRepo
{
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: PASS, no warnings. (`seed_starter_lists` and `verify_store` are exercised by the contract
tests from Task 7 and Task 11.)

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src
git commit -m "feat(domain): verify-audit logic and audited starter-list seeding

verify_snapshots compares each record with its latest audit entry using a
BTreeMap keyed by (entity, id); verify_store is generic over the store traits.
Store and WriteTx are trait aliases with blanket impls.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: SQLite store, contract suite and users — the first audited write end to end

This task sets up everything the storage crates need (workspace dependencies, offline sqlx,
decisions) and proves the full path on one feature: a user is created through a transaction, the
audit entry is appended inside it, and the contract suite checks both on SQLite. It is the largest
task because a store skeleton without a write would leave unused code, which CI rejects.

**Files:**
- Modify: `Cargo.toml` (workspace)
- Create: `.cargo/config.toml`
- Create: `scripts/sqlx-prepare.sh`
- Modify: `docs/DECISIONS.md` (append rows #45–#55)
- Modify: `deny.toml` (only if `cargo deny check` asks for a licence)
- Create: `crates/storage-contract/Cargo.toml`, `src/lib.rs`, `src/fixture.rs`,
  `src/transactions.rs`, `src/users.rs`
- Create: `crates/storage-sqlite/Cargo.toml`, `sqlx.toml`, `migrations/0001_initial.sql`,
  `src/lib.rs`, `src/error.rs`, `src/convert.rs`, `src/tx.rs`, `src/audit.rs`, `src/users.rs`,
  `tests/contract.rs`, `.sqlx/` (generated)

**Interfaces:**
- Consumes: everything in `smoke_domain::{records, repos, audit, permissions, clock, types}`.
- Produces:
  - `smoke_storage_sqlite::SqliteStore` (`Clone`) — `connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self>`, `migrate(&self) -> StoreResult<()>`; implements `Transactional<Tx = SqliteTx>`, `UserRepo`, `AuditRepo`.
  - `smoke_storage_sqlite::SqliteTx` — implements `Commit`, `UserWrites`. Crate-internal:
    `conn(&mut self) -> &mut SqliteConnection`, `now(&self) -> DateTime<Utc>`,
    `record<T: Audited>(&mut self, action: AuditAction, before: Option<&T>, after: &T) -> StoreResult<()>`.
  - Crate-internal helpers: `error::IntoStore::db(self) -> StoreResult<T>` on `Result<T, sqlx::Error>`;
    `convert::{micros, instant, opt_instant, parse, corrupt}`.
  - `smoke_storage_contract::fixture::{Fixture, RawSql, raw_runner, start, PASSWORD, new_user, add_user, last_entry}` and the `contract_tests!` macro.
  - The complete database schema for SQLite (all tables of spec §6, used by later tasks).

- [ ] **Step 1: Workspace configuration and decisions**

In the root `Cargo.toml`, set `rust-version = "1.94"` under `[workspace.package]` (sqlx 0.9 needs
it) and replace `[workspace.dependencies]` with:

```toml
[workspace.dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std", "serde"] }
chrono-tz = "0.10"
hex = "0.4"
proptest = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
smoke-domain = { path = "crates/domain" }
smoke-storage-contract = { path = "crates/storage-contract" }
sqlx = { version = "0.9", default-features = false, features = ["runtime-tokio", "macros", "migrate", "sqlx-toml"] }
tempfile = "3"
thiserror = "2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Create `.cargo/config.toml`:

```toml
[env]
# Compile against the committed query metadata in each storage crate's `.sqlx/`, so building
# needs no database. `scripts/sqlx-prepare.sh` regenerates it.
SQLX_OFFLINE = "true"
```

Append to the table in `docs/DECISIONS.md` (date 2026-10-13 on every row):

```markdown
| 45 | 2026-10-13 | Structure | Storage is **three crates**: `storage-sqlite`, `storage-postgres` and `storage-contract` (the shared test suite) | One `storage` crate with `sqlite/` and `postgres/` modules (spec §10) | sqlx's `query!` checks SQL against one database per crate; sqlx 0.9's `sqlx.toml` gives each crate its own URL variable. The contract crate is one test suite both backends must pass |
| 46 | 2026-10-13 | Database | Instants are stored as **integer microseconds since the Unix epoch (UTC)** in both backends | `TEXT` in SQLite + `TIMESTAMPTZ` in Postgres | Identical values on both, exact ordering, same precision as the audit hash; time zones are handled in Rust anyway |
| 47 | 2026-10-13 | Database | Writes go through a **transaction object** from `begin(actor)`; every write method appends its own audit entry before returning | CRUD repositories plus a separate audit call; one method per use case | Forgetting the audit is impossible; the web layer can still combine several writes (close + new event + report) atomically |
| 48 | 2026-10-13 | Evidence | Writes are **serialised**: SQLite uses one writer connection (plus read-only connections in WAL mode), Postgres locks `audit_log` at the start of each write transaction; `prev_hash` is `UNIQUE`; triggers make `audit_log` append-only | Optimistic retries | Two simultaneous writes would otherwise both link to the same previous hash and fork the chain |
| 49 | 2026-10-13 | Events | "At most one open event" is also enforced by a **partial unique index** | Application check only | Two phones logging at the same moment cannot both open an event |
| 50 | 2026-10-13 | Evidence | Audit snapshots **exclude the password hash** (`has_password` instead). Invites, sessions and jobs are not compared by `verify-audit` | Snapshot every column | The audit log can never be cleaned, so secrets must never enter it; invites are deleted daily and are not evidence |
| 51 | 2026-10-13 | Users | Usernames are **normalised**: trimmed, lowercase, 3–32 of `a-z 0-9 . _ -`. Room/symptom names 1–60 characters; notes at most 2000 | Free text | "Daniele" and "daniele " must be one login; bounded input |
| 52 | 2026-10-13 | Tooling | **sqlx 0.9**, MSRV raised to **1.94** | sqlx 0.8 | 0.9 adds `sqlx.toml` (needed for #45) |
| 53 | 2026-10-13 | Structure | Storage traits use **native async functions in traits** (`impl Future + Send`), and the store is a generic type, not `dyn` | `async-trait` crate (boxed futures) | No boxing or macro; the backend is chosen once at startup, so generics cost nothing |
| 54 | 2026-10-13 | Lists | Starter rooms and symptoms are **seeded through an audited transaction**, not by SQL in a migration | `INSERT`s in the migration | Rows inserted by a migration would have no audit entry, and `verify-audit` would flag them |
| 55 | 2026-10-13 | Evidence | Every write **re-reads the record** it wrote and audits that; snapshots are stored as JSON `TEXT` (not `JSONB`) | Audit the input values; `JSONB` | The audit snapshot is exactly what is stored, so honest data always verifies; `TEXT` keeps the bytes the same on both backends |
```

- [ ] **Step 2: Create the contract crate with the first tests**

`crates/storage-contract/Cargo.toml`:

```toml
[package]
name = "smoke-storage-contract"
version = "0.1.0"
description = "One behaviour test suite for every Smoke Tracker storage backend"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
chrono.workspace = true
serde_json.workspace = true
smoke-domain.workspace = true
tokio.workspace = true

[lints]
workspace = true
```

`crates/storage-contract/src/lib.rs`:

```rust
//! The storage contract: behaviour every backend must share, written once against the
//! `smoke-domain` traits. A backend crate runs all of it with
//!
//! ```ignore
//! smoke_storage_contract::contract_tests!(fixture);
//! ```
//!
//! where `fixture` is an `async fn() -> Option<Fixture<MyStore>>` that gives each test a fresh,
//! migrated database, or `None` to skip.

pub mod fixture;
pub mod transactions;
pub mod users;

pub use fixture::Fixture;

/// Generates one `#[tokio::test]` per contract test, each with its own fixture.
#[macro_export]
macro_rules! contract_tests {
    ($fixture:path) => {
        $crate::contract_tests!(@each $fixture;
            transactions::an_empty_transaction_commits,
            transactions::the_audit_log_starts_empty,
            users::creates_and_finds_a_user,
            users::usernames_are_unique_ignoring_case,
            users::creating_a_user_is_audited_without_the_password_hash,
            users::a_password_change_is_always_audited,
            users::a_locale_change_is_audited_but_a_no_op_is_not,
            users::guests_can_be_expired_and_reactivated,
            users::residents_cannot_expire,
            users::a_dropped_transaction_leaves_no_trace,
            users::instants_are_kept_to_the_microsecond,
            users::writing_an_unknown_user_is_not_found,
        );
    };
    (@each $fixture:path; $($module:ident::$test:ident),+ $(,)?) => {
        $(
            #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
            async fn $test() {
                if let Some(fixture) = $fixture().await {
                    $crate::$module::$test(&fixture).await;
                }
            }
        )+
    };
}
```

Every later task adds its tests to this list. Test names must be unique across modules.

`crates/storage-contract/src/fixture.rs`:

```rust
//! Per-test database plumbing that backends provide, and test data shared by the suites.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, TimeDelta, Utc};
use smoke_domain::audit::ChainedEntry;
use smoke_domain::clock::FakeClock;
use smoke_domain::records::{NewUser, PasswordHash, User};
use smoke_domain::repos::{AuditRepo, Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, Role, Username};

/// Runs SQL directly against the test database, bypassing the store: how tests tamper.
pub type RawSql = Box<
    dyn Fn(&'static str) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send>> + Send + Sync,
>;

pub fn raw_runner<F, Fut>(run: F) -> RawSql
where
    F: Fn(&'static str) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), String>> + Send + 'static,
{
    Box::new(move |sql| Box::pin(run(sql)))
}

/// A migrated, empty database behind a store, with a fake clock set to [`start`].
pub struct Fixture<S> {
    pub store: S,
    pub clock: Arc<FakeClock>,
    raw: RawSql,
    drop_audit_guard: &'static str,
    _keep_alive: Box<dyn Any + Send + Sync>,
}

impl<S> Fixture<S> {
    /// `drop_audit_guard` is the backend's SQL that removes the append-only triggers on
    /// `audit_log`, as someone with database access could. `keep_alive` lives as long as the
    /// fixture (e.g. the temporary directory of a SQLite file).
    pub fn new(
        store: S,
        clock: Arc<FakeClock>,
        raw: RawSql,
        drop_audit_guard: &'static str,
        keep_alive: impl Any + Send + Sync,
    ) -> Self {
        Self {
            store,
            clock,
            raw,
            drop_audit_guard,
            _keep_alive: Box::new(keep_alive),
        }
    }

    pub async fn raw(&self, sql: &'static str) -> Result<(), String> {
        (self.raw)(sql).await
    }

    pub async fn drop_audit_guard(&self) {
        self.raw(self.drop_audit_guard)
            .await
            .expect("drop the audit_log triggers");
    }
}

/// Where the fake clock starts in every contract test.
pub fn start() -> DateTime<Utc> {
    "2026-01-15T20:00:00Z".parse().expect("valid instant")
}

/// The password hash every test user gets; it must never appear in the audit log.
pub const PASSWORD: &str = "test-password-hash";

/// Guests expire a week after [`start`]; residents and viewers never.
pub fn new_user(name: &str, role: Role) -> NewUser {
    NewUser {
        username: Username::new(name).expect("valid username"),
        password_hash: Some(PasswordHash::new(PASSWORD.to_owned())),
        role,
        locale: Locale::En,
        expires_at: (role == Role::Guest).then(|| start() + TimeDelta::days(7)),
    }
}

pub async fn add_user<S>(fx: &Fixture<S>, name: &str, role: Role) -> User
where
    S: Transactional<Tx: UserWrites>,
{
    let mut tx = fx.store.begin(None).await.expect("begin");
    let user = tx.create_user(new_user(name, role)).await.expect("create user");
    tx.commit().await.expect("commit");
    user
}

pub async fn last_entry<S: AuditRepo>(fx: &Fixture<S>) -> ChainedEntry {
    fx.store
        .audit_log()
        .await
        .expect("audit log")
        .pop()
        .expect("at least one audit entry")
}
```

`crates/storage-contract/src/transactions.rs`:

```rust
//! Transactions in general.

use smoke_domain::repos::{AuditRepo, Commit, Transactional};

use crate::Fixture;

pub async fn an_empty_transaction_commits<S: Transactional>(fx: &Fixture<S>) {
    let tx = fx.store.begin(None).await.expect("begin");
    tx.commit().await.expect("commit");
}

pub async fn the_audit_log_starts_empty<S: AuditRepo>(fx: &Fixture<S>) {
    assert!(fx.store.audit_log().await.expect("audit log").is_empty());
}
```

`crates/storage-contract/src/users.rs`:

```rust
//! Users: accounts, passwords, languages and expiry.

use chrono::TimeDelta;
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::{Clock, truncate_to_micros};
use smoke_domain::records::{Audited, NewUser, PasswordHash};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, StoreError, Transactional, UserRepo, UserWrites,
};
use smoke_domain::types::{Locale, Role, UserId, Username};

use crate::Fixture;
use crate::fixture::{PASSWORD, add_user, last_entry, new_user, start};

pub async fn creates_and_finds_a_user<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo,
{
    let user = add_user(fx, " Daniele ", Role::Resident).await;
    assert_eq!(user.username.as_str(), "daniele");
    assert_eq!(user.created_at, start());
    assert_eq!(user.expires_at, None);
    assert_eq!(fx.store.user(user.id).await.unwrap(), Some(user.clone()));
    let by_name = Username::new("DANIELE").unwrap();
    assert_eq!(
        fx.store.user_by_username(&by_name).await.unwrap(),
        Some(user.clone())
    );
    assert_eq!(fx.store.users().await.unwrap(), [user]);
}

pub async fn usernames_are_unique_ignoring_case<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx
        .create_user(new_user("Daniele", Role::Guest))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UsernameTaken)),
        "{error:?}"
    );
}

pub async fn creating_a_user_is_audited_without_the_password_hash<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 1);
    let entry = &log[0].record;
    assert_eq!(
        (entry.entity, entry.entity_id, entry.action),
        (AuditEntity::User, user.id.0, AuditAction::Create)
    );
    assert_eq!(entry.actor_id, None);
    assert_eq!(entry.at_micros, start().timestamp_micros());
    assert_eq!(entry.before, None);
    assert_eq!(entry.after, Some(user.snapshot()));
    assert_eq!(entry.after.as_ref().unwrap()["has_password"], true);
    assert!(
        !format!("{log:?}").contains(PASSWORD),
        "the password hash reached the audit log"
    );
}

pub async fn a_password_change_is_always_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let new_hash = PasswordHash::new("another-hash".to_owned());
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    tx.set_password(user.id, new_hash.clone()).await.unwrap();
    tx.commit().await.unwrap();

    let stored = fx.store.user(user.id).await.unwrap().unwrap();
    assert_eq!(stored.password_hash, Some(new_hash));
    let entry = last_entry(fx).await.record;
    assert_eq!(entry.action, AuditAction::PasswordChange);
    assert_eq!(entry.actor_id, Some(user.id));
    assert_eq!(entry.before, entry.after);
}

pub async fn a_locale_change_is_audited_but_a_no_op_is_not<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let user = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(Some(user.id)).await.unwrap();
    let changed = tx.set_locale(user.id, Locale::Tr).await.unwrap();
    assert_eq!(changed.locale, Locale::Tr);
    tx.set_locale(user.id, Locale::Tr).await.unwrap();
    tx.commit().await.unwrap();

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 2);
    let update = &log[1].record;
    assert_eq!(update.action, AuditAction::Update);
    assert_eq!(update.before.as_ref().unwrap()["locale"], "en");
    assert_eq!(update.after.as_ref().unwrap()["locale"], "tr");
}

pub async fn guests_can_be_expired_and_reactivated<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + AuditRepo,
{
    let guest = add_user(fx, "guest", Role::Guest).await;
    fx.clock.advance(TimeDelta::hours(1));
    let now = fx.clock.now();

    let mut tx = fx.store.begin(None).await.unwrap();
    let expired = tx.expire_user(guest.id).await.unwrap();
    assert_eq!(expired.expires_at, Some(now));
    assert!(expired.actor().is_expired(now));

    let next_week = Some(now + TimeDelta::days(7));
    let reactivated = tx.reactivate_user(guest.id, next_week).await.unwrap();
    assert_eq!(reactivated.expires_at, next_week);

    let error = tx.reactivate_user(guest.id, None).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::GuestsMustExpire)),
        "{error:?}"
    );
    let past = Some(now - TimeDelta::seconds(1));
    let error = tx.reactivate_user(guest.id, past).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::ExpiryNotInFuture)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    let actions: Vec<_> = fx
        .store
        .audit_log()
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [AuditAction::Create, AuditAction::Expire, AuditAction::Reactivate]
    );
}

pub async fn residents_cannot_expire<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    let resident = add_user(fx, "daniele", Role::Resident).await;
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.expire_user(resident.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::ResidentsNeverExpire)),
        "{error:?}"
    );
    let with_expiry = NewUser {
        expires_at: Some(start() + TimeDelta::days(1)),
        ..new_user("other", Role::Resident)
    };
    let error = tx.create_user(with_expiry).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Invalid(DomainError::ResidentsNeverExpire)),
        "{error:?}"
    );
}

pub async fn a_dropped_transaction_leaves_no_trace<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.create_user(new_user("daniele", Role::Resident))
        .await
        .unwrap();
    drop(tx);
    assert!(fx.store.users().await.unwrap().is_empty());
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}

pub async fn instants_are_kept_to_the_microsecond<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites> + UserRepo + AuditRepo,
{
    let precise = start() + TimeDelta::days(7) + TimeDelta::nanoseconds(123_456_789);
    let mut tx = fx.store.begin(None).await.unwrap();
    let guest = tx
        .create_user(NewUser {
            expires_at: Some(precise),
            ..new_user("guest", Role::Guest)
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(guest.expires_at, Some(truncate_to_micros(precise)));
    assert_eq!(fx.store.user(guest.id).await.unwrap(), Some(guest.clone()));
    assert_eq!(last_entry(fx).await.record.after, Some(guest.snapshot()));
}

pub async fn writing_an_unknown_user_is_not_found<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites>,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.set_locale(UserId(999), Locale::It).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::NotFound {
                entity: AuditEntity::User,
                id: 999
            }
        ),
        "{error:?}"
    );
}
```

- [ ] **Step 3: Create the SQLite crate's manifest, schema and test harness**

`crates/storage-sqlite/Cargo.toml`:

```toml
[package]
name = "smoke-storage-sqlite"
version = "0.1.0"
description = "SQLite implementation of the Smoke Tracker storage traits"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
chrono.workspace = true
serde_json.workspace = true
smoke-domain.workspace = true
sqlx = { workspace = true, features = ["sqlite-bundled"] }

[dev-dependencies]
smoke-storage-contract.workspace = true
tempfile.workspace = true
tokio.workspace = true

[lints]
workspace = true
```

`crates/storage-sqlite/sqlx.toml`:

```toml
# The query macros check this crate against SQLITE_DATABASE_URL, so the SQLite and Postgres
# crates can be checked against different databases in one workspace.
[common]
database-url-var = "SQLITE_DATABASE_URL"
```

`crates/storage-sqlite/migrations/0001_initial.sql`:

```sql
-- Instants are INTEGER microseconds since the Unix epoch, UTC (decision #46).

CREATE TABLE users (
    id            INTEGER PRIMARY KEY,
    username      TEXT    NOT NULL CONSTRAINT users_username_unique UNIQUE,
    password_hash TEXT,
    role          TEXT    NOT NULL CHECK (role IN ('resident', 'guest', 'viewer')),
    locale        TEXT    NOT NULL CHECK (locale IN ('en', 'it', 'tr')),
    expires_at    INTEGER,
    created_at    INTEGER NOT NULL,
    CHECK (role <> 'resident' OR expires_at IS NULL),
    CHECK (role <> 'guest' OR expires_at IS NOT NULL)
);

CREATE TABLE invites (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users (id),
    purpose    TEXT    NOT NULL CHECK (purpose IN ('setup', 'password_reset')),
    token_hash TEXT    NOT NULL CONSTRAINT invites_token_hash_unique UNIQUE,
    created_by INTEGER NOT NULL REFERENCES users (id),
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    used_at    INTEGER
);

CREATE TABLE rooms (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    i18n_key  TEXT    CONSTRAINT rooms_i18n_key_unique UNIQUE,
    area      TEXT    NOT NULL CHECK (area IN ('inside', 'common')),
    hidden_at INTEGER
);

CREATE TABLE symptoms (
    id        INTEGER PRIMARY KEY,
    name      TEXT    NOT NULL,
    i18n_key  TEXT    CONSTRAINT symptoms_i18n_key_unique UNIQUE,
    hidden_at INTEGER
);

CREATE TABLE events (
    id                        INTEGER PRIMARY KEY,
    owner_id                  INTEGER NOT NULL REFERENCES users (id),
    opened_at                 INTEGER NOT NULL,
    closed_at                 INTEGER,
    close_reason              TEXT    CHECK (close_reason IN ('manual', 'new_event', 'auto_0400', 'backdated')),
    auto_close_notice_seen_at INTEGER,
    recorded_at               INTEGER NOT NULL,
    CHECK ((closed_at IS NULL) = (close_reason IS NULL)),
    CHECK (closed_at IS NULL OR closed_at >= opened_at)
);

-- At most one open event (decision #49).
CREATE UNIQUE INDEX events_one_open ON events ((closed_at IS NULL)) WHERE closed_at IS NULL;
CREATE INDEX events_opened_at ON events (opened_at);

CREATE TABLE smell_reports (
    id          INTEGER PRIMARY KEY,
    event_id    INTEGER NOT NULL REFERENCES events (id),
    user_id     INTEGER NOT NULL REFERENCES users (id),
    observed_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    intensity   INTEGER NOT NULL CHECK (intensity BETWEEN 1 AND 10),
    note        TEXT,
    deleted_at  INTEGER
);

CREATE INDEX smell_reports_event ON smell_reports (event_id, observed_at);

CREATE TABLE smell_report_rooms (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
    room_id         INTEGER NOT NULL REFERENCES rooms (id),
    PRIMARY KEY (smell_report_id, room_id)
);

CREATE TABLE smell_report_symptoms (
    smell_report_id INTEGER NOT NULL REFERENCES smell_reports (id),
    symptom_id      INTEGER NOT NULL REFERENCES symptoms (id),
    PRIMARY KEY (smell_report_id, symptom_id)
);

CREATE TABLE pm25_readings (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users (id),
    event_id    INTEGER REFERENCES events (id),
    observed_at INTEGER NOT NULL,
    recorded_at INTEGER NOT NULL,
    value_ugm3  REAL    NOT NULL CHECK (value_ugm3 >= 0 AND value_ugm3 <= 1000),
    deleted_at  INTEGER
);

CREATE INDEX pm25_readings_event ON pm25_readings (event_id, observed_at);

-- Append-only (decision #48). `id` is assigned by the application: it is part of the hash.
CREATE TABLE audit_log (
    id          INTEGER PRIMARY KEY,
    at          INTEGER NOT NULL,
    actor_id    INTEGER REFERENCES users (id),
    entity      TEXT    NOT NULL,
    entity_id   INTEGER NOT NULL,
    action      TEXT    NOT NULL,
    before_json TEXT,
    after_json  TEXT,
    prev_hash   TEXT    NOT NULL CONSTRAINT audit_log_prev_hash_unique UNIQUE,
    hash        TEXT    NOT NULL CONSTRAINT audit_log_hash_unique UNIQUE
);

CREATE INDEX audit_log_entity ON audit_log (entity, entity_id);

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only');
END;

CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
BEGIN
    SELECT RAISE(ABORT, 'audit_log is append-only');
END;

CREATE TABLE jobs (
    name        TEXT    NOT NULL PRIMARY KEY,
    last_run_at INTEGER NOT NULL
);
```

(The `sessions` table belongs to `tower-sessions`, which creates it in milestone 3.)

`crates/storage-sqlite/tests/contract.rs`:

```rust
use std::sync::Arc;

use smoke_domain::clock::{FakeClock, SystemClock};
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
use smoke_storage_sqlite::SqliteStore;
use sqlx::SqlitePool;
use tempfile::TempDir;

fn database() -> (TempDir, String) {
    let dir = tempfile::tempdir().expect("temp dir");
    let url = format!("sqlite://{}", dir.path().join("test.db").display());
    (dir, url)
}

async fn fixture() -> Option<Fixture<SqliteStore>> {
    let (dir, url) = database();
    let clock = Arc::new(FakeClock::new(start()));
    let store = SqliteStore::connect(&url, clock.clone())
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    let pool = SqlitePool::connect(&url).await.expect("raw connection");
    let raw = raw_runner(move |sql| {
        let pool = pool.clone();
        async move {
            sqlx::raw_sql(sql)
                .execute(&pool)
                .await
                .map(drop)
                .map_err(|e| e.to_string())
        }
    });
    let drop_guard = "DROP TRIGGER audit_log_no_update; DROP TRIGGER audit_log_no_delete;";
    Some(Fixture::new(store, clock, raw, drop_guard, dir))
}

smoke_storage_contract::contract_tests!(fixture);

#[tokio::test]
async fn migrations_create_every_table_and_can_run_again() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock)).await.unwrap();
    store.migrate().await.unwrap();
    store.migrate().await.unwrap();

    let pool = SqlitePool::connect(&url).await.unwrap();
    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite_%' AND name <> '_sqlx_migrations' ORDER BY name",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        tables,
        [
            "audit_log",
            "events",
            "invites",
            "jobs",
            "pm25_readings",
            "rooms",
            "smell_report_rooms",
            "smell_report_symptoms",
            "smell_reports",
            "symptoms",
            "users",
        ]
    );
}

#[tokio::test]
async fn the_database_uses_write_ahead_logging() {
    let (_dir, url) = database();
    let store = SqliteStore::connect(&url, Arc::new(SystemClock)).await.unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    let mode: String = sqlx::query_scalar("PRAGMA journal_mode")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(mode, "wal");
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Both new crates join the workspace automatically (`members = ["crates/*"]`).
Run: `cargo test -p smoke-storage-sqlite`
Expected: compile error — `smoke_storage_sqlite::SqliteStore` does not exist (the crate has no
`src/lib.rs` yet).

- [ ] **Step 5: Implement the SQLite store**

`crates/storage-sqlite/src/lib.rs`:

```rust
//! SQLite implementation of the `smoke-domain` storage traits.
//!
//! SQLite allows one writer at a time, so all write transactions share a single connection: they
//! run one after another and every audit entry links to the one before it. Reads use a separate
//! pool of read-only connections, which WAL mode lets run alongside the writer.

mod audit;
mod convert;
mod error;
mod tx;
mod users;

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use smoke_domain::clock::Clock;
use smoke_domain::repos::{StoreError, StoreResult, Transactional};
use smoke_domain::types::UserId;
use sqlx::pool::PoolConnection;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sqlx::{Sqlite, SqlitePool};

use crate::error::IntoStore;
pub use crate::tx::SqliteTx;

#[derive(Clone)]
pub struct SqliteStore {
    writer: SqlitePool,
    reader: SqlitePool,
    clock: Arc<dyn Clock>,
}

impl SqliteStore {
    /// Opens (and creates if missing) the database at `url`, e.g. `sqlite://data/smoke.db`.
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self> {
        let base = SqliteConnectOptions::from_str(url)
            .db()?
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let writer = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                base.clone()
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal),
            )
            .await
            .db()?;
        let reader = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_lazy_with(base.read_only(true));
        Ok(Self {
            writer,
            reader,
            clock,
        })
    }

    /// Applies the embedded migrations. Safe to run on every start.
    pub async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("./migrations")
            .run(&self.writer)
            .await
            .map_err(|e| StoreError::Backend(Box::new(e)))
    }

    async fn read(&self) -> StoreResult<PoolConnection<Sqlite>> {
        self.reader.acquire().await.db()
    }
}

impl Transactional for SqliteStore {
    type Tx = SqliteTx;

    async fn begin(&self, actor: Option<UserId>) -> StoreResult<SqliteTx> {
        let tx = self.writer.begin().await.db()?;
        Ok(SqliteTx::new(tx, actor, self.clock.clone()))
    }
}
```

`crates/storage-sqlite/src/error.rs`:

```rust
//! Turns sqlx errors into the storage boundary's errors.

use smoke_domain::repos::{Conflict, StoreError, StoreResult};
use sqlx::error::ErrorKind;

pub(crate) trait IntoStore<T> {
    fn db(self) -> StoreResult<T>;
}

impl<T> IntoStore<T> for Result<T, sqlx::Error> {
    fn db(self) -> StoreResult<T> {
        self.map_err(into_store_error)
    }
}

fn into_store_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &error {
        match db.kind() {
            ErrorKind::UniqueViolation => return unique_conflict(db.message()).into(),
            ErrorKind::ForeignKeyViolation => return Conflict::UnknownReference.into(),
            _ => {}
        }
    }
    StoreError::Backend(Box::new(error))
}

/// SQLite names the violated columns or index in the message, e.g.
/// `UNIQUE constraint failed: users.username`.
fn unique_conflict(message: &str) -> Conflict {
    if message.contains("users.username") {
        Conflict::UsernameTaken
    } else if message.contains("events_one_open") {
        Conflict::AnotherEventOpen
    } else if message.contains("audit_log.") {
        Conflict::ChainForked
    } else {
        Conflict::Duplicate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_violations_map_to_their_conflict() {
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: users.username"),
            Conflict::UsernameTaken
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: index 'events_one_open'"),
            Conflict::AnotherEventOpen
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: audit_log.prev_hash"),
            Conflict::ChainForked
        );
        assert_eq!(
            unique_conflict("UNIQUE constraint failed: rooms.i18n_key"),
            Conflict::Duplicate
        );
    }
}
```

(The `events_one_open` message text was verified against SQLite while writing this plan.)

`crates/storage-sqlite/src/convert.rs`:

```rust
//! Database values ↔ domain values. Instants are microseconds since the Unix epoch.

use std::fmt::Display;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::repos::{StoreError, StoreResult};

pub(crate) fn micros(t: DateTime<Utc>) -> i64 {
    t.timestamp_micros()
}

pub(crate) fn instant(value: i64) -> StoreResult<DateTime<Utc>> {
    DateTime::from_timestamp_micros(value)
        .ok_or_else(|| corrupt(format!("timestamp {value} is out of range")))
}

pub(crate) fn opt_instant(value: Option<i64>) -> StoreResult<Option<DateTime<Utc>>> {
    value.map(instant).transpose()
}

pub(crate) fn parse<T: FromStr<Err = DomainError>>(text: &str) -> StoreResult<T> {
    text.parse().map_err(corrupt)
}

pub(crate) fn corrupt(reason: impl Display) -> StoreError {
    StoreError::Corrupt(reason.to_string())
}
```

`crates/storage-sqlite/src/tx.rs`:

```rust
//! The write transaction, and the audit entry every write appends to it.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use smoke_domain::audit::{AuditAction, AuditRecord, ChainedEntry, GENESIS_HASH};
use smoke_domain::clock::Clock;
use smoke_domain::records::Audited;
use smoke_domain::repos::{Commit, StoreResult};
use smoke_domain::types::UserId;
use sqlx::{Sqlite, SqliteConnection, Transaction};

use crate::error::IntoStore;

pub struct SqliteTx {
    tx: Transaction<'static, Sqlite>,
    actor: Option<UserId>,
    clock: Arc<dyn Clock>,
}

impl SqliteTx {
    pub(crate) fn new(
        tx: Transaction<'static, Sqlite>,
        actor: Option<UserId>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { tx, actor, clock }
    }

    pub(crate) fn conn(&mut self) -> &mut SqliteConnection {
        &mut self.tx
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Appends the audit entry for one write, linked to the latest entry. An `update` that
    /// changed nothing is not recorded.
    pub(crate) async fn record<T: Audited>(
        &mut self,
        action: AuditAction,
        before: Option<&T>,
        after: &T,
    ) -> StoreResult<()> {
        let before = before.map(Audited::snapshot);
        let after_value = after.snapshot();
        if action == AuditAction::Update && before.as_ref() == Some(&after_value) {
            return Ok(());
        }
        let head = sqlx::query!("SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1")
            .fetch_optional(self.conn())
            .await
            .db()?;
        let (id, prev_hash) = head.map_or((1, GENESIS_HASH.to_owned()), |h| (h.id + 1, h.hash));
        let entry = ChainedEntry::seal(
            AuditRecord {
                id,
                at_micros: self.now().timestamp_micros(),
                actor_id: self.actor,
                entity: T::ENTITY,
                entity_id: after.audit_id(),
                action,
                before,
                after: Some(after_value),
            },
            &prev_hash,
        );
        let record = &entry.record;
        sqlx::query!(
            "INSERT INTO audit_log \
             (id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            record.id,
            record.at_micros,
            record.actor_id.map(|a| a.0),
            record.entity.as_str(),
            record.entity_id,
            record.action.as_str(),
            record.before.as_ref().map(Value::to_string),
            record.after.as_ref().map(Value::to_string),
            entry.prev_hash,
            entry.hash,
        )
        .execute(&mut *self.tx)
        .await
        .db()?;
        Ok(())
    }
}

impl Commit for SqliteTx {
    async fn commit(self) -> StoreResult<()> {
        self.tx.commit().await.db()
    }
}
```

`crates/storage-sqlite/src/audit.rs`:

```rust
//! Reading the audit log.

use serde_json::Value;
use smoke_domain::audit::{AuditEntity, AuditRecord, ChainedEntry};
use smoke_domain::repos::{AuditRepo, StoreResult};
use smoke_domain::types::UserId;
use sqlx::SqliteConnection;

use crate::SqliteStore;
use crate::convert::{corrupt, parse};
use crate::error::IntoStore;

struct AuditRow {
    id: i64,
    at: i64,
    actor_id: Option<i64>,
    entity: String,
    entity_id: i64,
    action: String,
    before_json: Option<String>,
    after_json: Option<String>,
    prev_hash: String,
    hash: String,
}

impl AuditRow {
    fn into_entry(self) -> StoreResult<ChainedEntry> {
        Ok(ChainedEntry {
            record: AuditRecord {
                id: self.id,
                at_micros: self.at,
                actor_id: self.actor_id.map(UserId),
                entity: parse(&self.entity)?,
                entity_id: self.entity_id,
                action: parse(&self.action)?,
                before: json(self.before_json)?,
                after: json(self.after_json)?,
            },
            prev_hash: self.prev_hash,
            hash: self.hash,
        })
    }
}

fn json(text: Option<String>) -> StoreResult<Option<Value>> {
    text.map(|t| serde_json::from_str(&t).map_err(corrupt))
        .transpose()
}

pub(crate) async fn fetch_entries(conn: &mut SqliteConnection) -> StoreResult<Vec<ChainedEntry>> {
    sqlx::query_as!(
        AuditRow,
        "SELECT id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash \
         FROM audit_log ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(AuditRow::into_entry)
    .collect()
}

impl AuditRepo for SqliteStore {
    async fn audit_log(&self) -> StoreResult<Vec<ChainedEntry>> {
        fetch_entries(&mut *self.read().await?).await
    }

    async fn audit_history(
        &self,
        entity: AuditEntity,
        entity_id: i64,
    ) -> StoreResult<Vec<ChainedEntry>> {
        sqlx::query_as!(
            AuditRow,
            "SELECT id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash \
             FROM audit_log WHERE entity = $1 AND entity_id = $2 ORDER BY id",
            entity.as_str(),
            entity_id,
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(AuditRow::into_entry)
        .collect()
    }
}
```

`crates/storage-sqlite/src/users.rs`:

```rust
//! Users: accounts, passwords, languages and expiry.

use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::permissions::check_expiry;
use smoke_domain::records::{NewUser, PasswordHash, User};
use smoke_domain::repos::{StoreError, StoreResult, UserRepo, UserWrites};
use smoke_domain::types::{Locale, Role, UserId, Username};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct UserRow {
    id: i64,
    username: String,
    password_hash: Option<String>,
    role: String,
    locale: String,
    expires_at: Option<i64>,
    created_at: i64,
}

impl UserRow {
    fn into_user(self) -> StoreResult<User> {
        Ok(User {
            id: UserId(self.id),
            username: Username::new(&self.username).map_err(corrupt)?,
            password_hash: self.password_hash.map(PasswordHash::new),
            role: parse(&self.role)?,
            locale: parse(&self.locale)?,
            expires_at: opt_instant(self.expires_at)?,
            created_at: instant(self.created_at)?,
        })
    }
}

async fn fetch(conn: &mut SqliteConnection, id: UserId) -> StoreResult<Option<User>> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, username, password_hash, role, locale, expires_at, created_at \
         FROM users WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(UserRow::into_user)
    .transpose()
}

async fn require(conn: &mut SqliteConnection, id: UserId) -> StoreResult<User> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::User,
        id: id.0,
    })
}

pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<User>> {
    sqlx::query_as!(
        UserRow,
        "SELECT id, username, password_hash, role, locale, expires_at, created_at \
         FROM users ORDER BY username"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(UserRow::into_user)
    .collect()
}

/// Re-reads the user after an update and audits the change.
async fn audited(tx: &mut SqliteTx, action: AuditAction, before: User) -> StoreResult<User> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl UserRepo for SqliteStore {
    async fn user(&self, id: UserId) -> StoreResult<Option<User>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn user_by_username(&self, username: &Username) -> StoreResult<Option<User>> {
        sqlx::query_as!(
            UserRow,
            "SELECT id, username, password_hash, role, locale, expires_at, created_at \
             FROM users WHERE username = $1",
            username.as_str(),
        )
        .fetch_optional(&mut *self.read().await?)
        .await
        .db()?
        .map(UserRow::into_user)
        .transpose()
    }

    async fn users(&self) -> StoreResult<Vec<User>> {
        fetch_all(&mut *self.read().await?).await
    }
}

impl UserWrites for SqliteTx {
    async fn create_user(&mut self, new: NewUser) -> StoreResult<User> {
        let now = self.now();
        check_expiry(new.role, new.expires_at, now)?;
        let id = sqlx::query_scalar!(
            "INSERT INTO users (username, password_hash, role, locale, expires_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            new.username.as_str(),
            new.password_hash.as_ref().map(PasswordHash::as_str),
            new.role.as_str(),
            new.locale.as_str(),
            new.expires_at.map(micros),
            micros(now),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let user = require(self.conn(), UserId(id)).await?;
        self.record(AuditAction::Create, None, &user).await?;
        Ok(user)
    }

    async fn set_password(&mut self, id: UserId, hash: PasswordHash) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE users SET password_hash = $1 WHERE id = $2",
            hash.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::PasswordChange, before).await
    }

    async fn set_locale(&mut self, id: UserId, locale: Locale) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE users SET locale = $1 WHERE id = $2",
            locale.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn expire_user(&mut self, id: UserId) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        if before.role == Role::Resident {
            return Err(DomainError::ResidentsNeverExpire.into());
        }
        let now = micros(self.now());
        sqlx::query!("UPDATE users SET expires_at = $1 WHERE id = $2", now, id.0)
            .execute(self.conn())
            .await
            .db()?;
        audited(self, AuditAction::Expire, before).await
    }

    async fn reactivate_user(
        &mut self,
        id: UserId,
        expires_at: Option<DateTime<Utc>>,
    ) -> StoreResult<User> {
        let before = require(self.conn(), id).await?;
        check_expiry(before.role, expires_at, self.now())?;
        sqlx::query!(
            "UPDATE users SET expires_at = $1 WHERE id = $2",
            expires_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Reactivate, before).await
    }
}
```

- [ ] **Step 6: Generate the offline query metadata**

Create `scripts/sqlx-prepare.sh` (then `chmod +x scripts/sqlx-prepare.sh`):

```bash
#!/usr/bin/env bash
# Regenerates the committed sqlx query metadata (`.sqlx/`) of the storage crates from freshly
# migrated databases. With --check, fails instead when the committed metadata is out of date.
# Needs sqlx-cli 0.9 (see README).
set -euo pipefail

mode=prepare
if [[ "${1:-}" == "--check" ]]; then mode=check; fi

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export SQLX_OFFLINE=false

prepare() {
  if [[ "$mode" == check ]]; then cargo sqlx prepare --check; else cargo sqlx prepare; fi
}

cd "$root/crates/storage-sqlite"
export SQLITE_DATABASE_URL="sqlite://$work/prepare.db"
export DATABASE_URL="$SQLITE_DATABASE_URL"
sqlx database create
sqlx migrate run
prepare
```

Run: `scripts/sqlx-prepare.sh`
Expected: `crates/storage-sqlite/.sqlx/` contains one `query-<hash>.json` per query.

Check that the files landed in `crates/storage-sqlite/.sqlx/` (the macros look in the crate's own
`.sqlx/` first, then the workspace root's; verified in `sqlx-macros-core` 0.9). If sqlx-cli wrote
them to the workspace root `.sqlx/` instead, add `mv "$root/.sqlx" .sqlx` after `prepare` in the
script (prepare mode only) so each crate keeps its own metadata, and re-run.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p smoke-storage-sqlite && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check`
Expected: 12 contract tests + 2 SQLite tests + 1 unit test pass; the whole workspace passes;
no clippy warnings. If `cargo deny check licenses` rejects a licence, add it to `allow` in
`deny.toml` only if it is a permissive OSI licence (e.g. `CDLA-Permissive-2.0`); for anything
copyleft, stop and ask the user.

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo/config.toml scripts/sqlx-prepare.sh docs/DECISIONS.md deny.toml crates/storage-contract crates/storage-sqlite
git commit -m "feat(storage): SQLite store whose transactions append audit entries

One writer connection serialises write transactions so the hash chain never
forks; read-only connections run alongside in WAL mode. Queries are checked at
compile time against the committed .sqlx metadata (sqlx.toml renames the URL
variable per crate). The contract crate's macro turns one test list into
#[tokio::test]s for each backend.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: SQLite — rooms and symptoms

**Files:**
- Create: `crates/storage-sqlite/src/lists.rs`
- Modify: `crates/storage-sqlite/src/lib.rs` (add `mod lists;`)
- Create: `crates/storage-contract/src/lists.rs`
- Modify: `crates/storage-contract/src/lib.rs` (add `pub mod lists;` and the tests to the macro)
- Regenerate: `crates/storage-sqlite/.sqlx/`

**Interfaces:**
- Consumes: `SqliteTx::{conn, now, record}`, `convert::*`, `IntoStore` (Task 6);
  `seed_starter_lists`, `STARTER_ROOMS`, `STARTER_SYMPTOMS` (Task 5).
- Produces: `impl ListRepo for SqliteStore`, `impl ListWrites for SqliteTx`; crate-internal
  `lists::fetch_rooms(conn) -> StoreResult<Vec<Room>>`, `lists::fetch_symptoms(conn) -> StoreResult<Vec<Symptom>>`
  (used by Task 11).

- [ ] **Step 1: Write the failing contract tests**

`crates/storage-contract/src/lists.rs`:

```rust
//! Rooms and symptoms.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::NewRoom;
use smoke_domain::repos::{AuditRepo, Commit, ListRepo, ListWrites, StoreError, Transactional};
use smoke_domain::starter::{STARTER_ROOMS, STARTER_SYMPTOMS, seed_starter_lists};
use smoke_domain::types::{ListName, RoomArea, SymptomId};

use crate::Fixture;
use crate::fixture::start;

pub async fn seeding_adds_the_starter_lists_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo + AuditRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    seed_starter_lists(&fx.store).await.unwrap();

    let rooms = fx.store.rooms().await.unwrap();
    let keys: Vec<_> = rooms.iter().map(|r| r.i18n_key.as_deref().unwrap()).collect();
    assert_eq!(keys, STARTER_ROOMS.map(|r| r.i18n_key));
    let stairwell = rooms.last().unwrap();
    assert_eq!(stairwell.area, RoomArea::Common);
    assert_eq!(stairwell.name.as_str(), "Stairwell (shared)");
    assert_eq!(fx.store.symptoms().await.unwrap().len(), STARTER_SYMPTOMS.len());

    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 12);
    assert!(log
        .iter()
        .all(|e| e.record.actor_id.is_none() && e.record.action == AuditAction::Create));
}

pub async fn a_custom_room_has_no_translation_key<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let room = tx
        .create_room(NewRoom {
            name: ListName::new("Balcony").unwrap(),
            i18n_key: None,
            area: RoomArea::Common,
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(room.i18n_key, None);
    assert_eq!(room.hidden_at, None);
    assert_eq!(fx.store.rooms().await.unwrap(), [room]);
}

pub async fn hiding_a_room_is_audited_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites> + ListRepo + AuditRepo,
{
    seed_starter_lists(&fx.store).await.unwrap();
    let kitchen = fx.store.rooms().await.unwrap()[1].clone();

    let mut tx = fx.store.begin(None).await.unwrap();
    let hidden = tx.set_room_hidden(kitchen.id, true).await.unwrap();
    assert_eq!(hidden.hidden_at, Some(start()));
    fx.clock.advance(TimeDelta::minutes(5));
    let still_hidden = tx.set_room_hidden(kitchen.id, true).await.unwrap();
    assert_eq!(still_hidden.hidden_at, Some(start()));
    let shown = tx.set_room_hidden(kitchen.id, false).await.unwrap();
    assert_eq!(shown.hidden_at, None);
    tx.commit().await.unwrap();

    assert_eq!(fx.store.rooms().await.unwrap().len(), 6);
    let updates = fx
        .store
        .audit_history(AuditEntity::Room, kitchen.id.0)
        .await
        .unwrap()
        .iter()
        .filter(|e| e.record.action == AuditAction::Update)
        .count();
    assert_eq!(updates, 2);
}

pub async fn hiding_an_unknown_symptom_is_not_found<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: ListWrites>,
{
    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.set_symptom_hidden(SymptomId(42), true).await.unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::NotFound {
                entity: AuditEntity::Symptom,
                id: 42
            }
        ),
        "{error:?}"
    );
}
```

In `crates/storage-contract/src/lib.rs` add `pub mod lists;` and append to the macro list:

```rust
            lists::seeding_adds_the_starter_lists_once,
            lists::a_custom_room_has_no_translation_key,
            lists::hiding_a_room_is_audited_once,
            lists::hiding_an_unknown_symptom_is_not_found,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite`
Expected: compile errors — `SqliteStore: ListRepo` and `SqliteTx: ListWrites` are not satisfied.

- [ ] **Step 3: Implement**

`crates/storage-sqlite/src/lists.rs`:

```rust
//! Rooms and symptoms: the lists a report picks from. Hidden, never deleted.

use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewRoom, NewSymptom, Room, Symptom};
use smoke_domain::repos::{ListRepo, ListWrites, StoreError, StoreResult};
use smoke_domain::types::{ListName, RoomId, SymptomId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct RoomRow {
    id: i64,
    name: String,
    i18n_key: Option<String>,
    area: String,
    hidden_at: Option<i64>,
}

impl RoomRow {
    fn into_room(self) -> StoreResult<Room> {
        Ok(Room {
            id: RoomId(self.id),
            name: ListName::new(&self.name).map_err(corrupt)?,
            i18n_key: self.i18n_key,
            area: parse(&self.area)?,
            hidden_at: opt_instant(self.hidden_at)?,
        })
    }
}

struct SymptomRow {
    id: i64,
    name: String,
    i18n_key: Option<String>,
    hidden_at: Option<i64>,
}

impl SymptomRow {
    fn into_symptom(self) -> StoreResult<Symptom> {
        Ok(Symptom {
            id: SymptomId(self.id),
            name: ListName::new(&self.name).map_err(corrupt)?,
            i18n_key: self.i18n_key,
            hidden_at: opt_instant(self.hidden_at)?,
        })
    }
}

async fn require_room(conn: &mut SqliteConnection, id: RoomId) -> StoreResult<Room> {
    sqlx::query_as!(
        RoomRow,
        "SELECT id, name, i18n_key, area, hidden_at FROM rooms WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .ok_or(StoreError::NotFound {
        entity: AuditEntity::Room,
        id: id.0,
    })?
    .into_room()
}

async fn require_symptom(conn: &mut SqliteConnection, id: SymptomId) -> StoreResult<Symptom> {
    sqlx::query_as!(
        SymptomRow,
        "SELECT id, name, i18n_key, hidden_at FROM symptoms WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .ok_or(StoreError::NotFound {
        entity: AuditEntity::Symptom,
        id: id.0,
    })?
    .into_symptom()
}

pub(crate) async fn fetch_rooms(conn: &mut SqliteConnection) -> StoreResult<Vec<Room>> {
    sqlx::query_as!(
        RoomRow,
        "SELECT id, name, i18n_key, area, hidden_at FROM rooms ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(RoomRow::into_room)
    .collect()
}

pub(crate) async fn fetch_symptoms(conn: &mut SqliteConnection) -> StoreResult<Vec<Symptom>> {
    sqlx::query_as!(
        SymptomRow,
        "SELECT id, name, i18n_key, hidden_at FROM symptoms ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(SymptomRow::into_symptom)
    .collect()
}

impl ListRepo for SqliteStore {
    async fn rooms(&self) -> StoreResult<Vec<Room>> {
        fetch_rooms(&mut *self.read().await?).await
    }

    async fn symptoms(&self) -> StoreResult<Vec<Symptom>> {
        fetch_symptoms(&mut *self.read().await?).await
    }
}

impl ListWrites for SqliteTx {
    async fn create_room(&mut self, new: NewRoom) -> StoreResult<Room> {
        let id = sqlx::query_scalar!(
            "INSERT INTO rooms (name, i18n_key, area) VALUES ($1, $2, $3) RETURNING id",
            new.name.as_str(),
            new.i18n_key,
            new.area.as_str(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let room = require_room(self.conn(), RoomId(id)).await?;
        self.record(AuditAction::Create, None, &room).await?;
        Ok(room)
    }

    async fn set_room_hidden(&mut self, id: RoomId, hidden: bool) -> StoreResult<Room> {
        let before = require_room(self.conn(), id).await?;
        let hidden_at = hidden.then(|| before.hidden_at.unwrap_or_else(|| self.now()));
        sqlx::query!(
            "UPDATE rooms SET hidden_at = $1 WHERE id = $2",
            hidden_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let after = require_room(self.conn(), id).await?;
        self.record(AuditAction::Update, Some(&before), &after).await?;
        Ok(after)
    }

    async fn create_symptom(&mut self, new: NewSymptom) -> StoreResult<Symptom> {
        let id = sqlx::query_scalar!(
            "INSERT INTO symptoms (name, i18n_key) VALUES ($1, $2) RETURNING id",
            new.name.as_str(),
            new.i18n_key,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let symptom = require_symptom(self.conn(), SymptomId(id)).await?;
        self.record(AuditAction::Create, None, &symptom).await?;
        Ok(symptom)
    }

    async fn set_symptom_hidden(&mut self, id: SymptomId, hidden: bool) -> StoreResult<Symptom> {
        let before = require_symptom(self.conn(), id).await?;
        let hidden_at = hidden.then(|| before.hidden_at.unwrap_or_else(|| self.now()));
        sqlx::query!(
            "UPDATE symptoms SET hidden_at = $1 WHERE id = $2",
            hidden_at.map(micros),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        let after = require_symptom(self.conn(), id).await?;
        self.record(AuditAction::Update, Some(&before), &after).await?;
        Ok(after)
    }
}
```

Add `mod lists;` to `lib.rs`.

- [ ] **Step 4: Regenerate metadata and run the tests**

Run: `scripts/sqlx-prepare.sh && cargo test -p smoke-storage-sqlite && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (16 contract tests + 2 SQLite tests + 1 unit test).

- [ ] **Step 5: Commit**

```bash
git add crates/storage-sqlite crates/storage-contract
git commit -m "feat(storage): rooms and symptoms are hidden, never deleted; seeding is audited

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: SQLite — events

**Files:**
- Create: `crates/storage-sqlite/src/events.rs`
- Modify: `crates/storage-sqlite/src/lib.rs` (add `mod events;`)
- Create: `crates/storage-contract/src/events.rs`
- Modify: `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/fixture.rs` (add `add_event`)
- Regenerate: `crates/storage-sqlite/.sqlx/`

**Interfaces:**
- Consumes: Task 6 helpers; `Event`, `Closure`, `NewEvent` (Task 3).
- Produces: `impl EventRepo for SqliteStore`, `impl EventWrites for SqliteTx`;
  `smoke_storage_contract::fixture::add_event<S>(fx, owner: UserId, opened_at: DateTime<Utc>, closed: Option<Closure>) -> Event where S: Transactional<Tx: EventWrites>`.

- [ ] **Step 1: Write the failing contract tests**

Append to `crates/storage-contract/src/fixture.rs` (and extend its imports with
`smoke_domain::records::{Closure, Event, NewEvent}`, `smoke_domain::repos::EventWrites`,
`smoke_domain::types::UserId`):

```rust
pub async fn add_event<S>(
    fx: &Fixture<S>,
    owner: UserId,
    opened_at: DateTime<Utc>,
    closed: Option<Closure>,
) -> Event
where
    S: Transactional<Tx: EventWrites>,
{
    let mut tx = fx.store.begin(Some(owner)).await.expect("begin");
    let event = tx
        .create_event(NewEvent {
            owner_id: owner,
            opened_at,
            closed,
        })
        .await
        .expect("create event");
    tx.commit().await.expect("commit");
    event
}
```

`crates/storage-contract/src/events.rs`:

```rust
//! Events: opening, closing, reopening, and the one-open-event rule.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{Closure, Event, NewEvent};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventRepo, EventWrites, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{CloseReason, EventId, Role};

use crate::Fixture;
use crate::fixture::{add_event, add_user, last_entry, start};

fn closure(at: chrono::DateTime<chrono::Utc>, reason: CloseReason) -> Option<Closure> {
    Some(Closure { at, reason })
}

pub async fn a_new_event_can_be_the_open_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let opened_at = start() - TimeDelta::minutes(5);
    let event = add_event(fx, owner.id, opened_at, None).await;
    assert!(event.is_open());
    assert_eq!(
        (event.owner_id, event.opened_at, event.recorded_at),
        (owner.id, opened_at, start())
    );
    assert_eq!(fx.store.open_event().await.unwrap(), Some(event.clone()));
    assert_eq!(fx.store.event(event.id).await.unwrap(), Some(event.clone()));

    let mut tx = fx.store.begin(None).await.unwrap();
    assert_eq!(tx.open_event().await.unwrap(), Some(event.clone()));
    assert_eq!(tx.event(event.id).await.unwrap(), Some(event));
    drop(tx);

    let entry = last_entry(fx).await.record;
    assert_eq!(
        (entry.entity, entry.action, entry.actor_id),
        (AuditEntity::Event, AuditAction::Create, Some(owner.id))
    );
}

pub async fn only_one_event_can_be_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites>,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    add_event(fx, owner.id, start(), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx
        .create_event(NewEvent {
            owner_id: owner.id,
            opened_at: start(),
            closed: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AnotherEventOpen)),
        "{error:?}"
    );
    drop(tx);

    let yesterday = start() - TimeDelta::days(1);
    let backdated = add_event(fx, owner.id, yesterday, closure(yesterday, CloseReason::Backdated)).await;
    assert!(!backdated.is_open());
}

pub async fn closing_and_reopening_are_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let event = add_event(fx, owner.id, start() - TimeDelta::hours(1), None).await;
    let manual = Closure {
        at: start(),
        reason: CloseReason::Manual,
    };

    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
    let closed = tx.close_event(event.id, manual).await.unwrap();
    assert_eq!(closed.closed, Some(manual));
    let error = tx.close_event(event.id, manual).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotOpen)),
        "{error:?}"
    );
    let reopened = tx.reopen_event(event.id).await.unwrap();
    assert!(reopened.is_open());
    let error = tx.reopen_event(event.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotClosed)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    assert_eq!(fx.store.open_event().await.unwrap(), Some(reopened));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::Event, event.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [AuditAction::Create, AuditAction::Close, AuditAction::Reopen]
    );
}

pub async fn reopening_is_refused_while_another_event_is_open<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let earlier = start() - TimeDelta::hours(3);
    let first = add_event(fx, owner.id, earlier, closure(earlier + TimeDelta::hours(1), CloseReason::Manual)).await;
    add_event(fx, owner.id, start() - TimeDelta::minutes(30), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let error = tx.reopen_event(first.id).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AnotherEventOpen)),
        "{error:?}"
    );
    drop(tx);
    assert!(!fx.store.event(first.id).await.unwrap().unwrap().is_open());
}

pub async fn the_end_of_a_closed_event_can_move<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites>,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let at = start() - TimeDelta::days(1);
    let past = add_event(fx, owner.id, at, closure(at, CloseReason::Backdated)).await;
    let open = add_event(fx, owner.id, start(), None).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    let moved = tx.move_event_end(past.id, at + TimeDelta::hours(1)).await.unwrap();
    assert_eq!(moved.closed, closure(at + TimeDelta::hours(1), CloseReason::Backdated));
    let error = tx.move_event_end(open.id, start()).await.unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::EventNotClosed)),
        "{error:?}"
    );
}

pub async fn the_auto_close_notice_is_marked_seen_once<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + AuditRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let opened = start() - TimeDelta::hours(6);
    let event = add_event(fx, owner.id, opened, closure(opened + TimeDelta::hours(1), CloseReason::Auto0400)).await;

    let mut tx = fx.store.begin(Some(owner.id)).await.unwrap();
    let seen = tx.mark_auto_close_notice_seen(event.id).await.unwrap();
    assert_eq!(seen.auto_close_notice_seen_at, Some(start()));
    fx.clock.advance(TimeDelta::minutes(10));
    let again = tx.mark_auto_close_notice_seen(event.id).await.unwrap();
    assert_eq!(again.auto_close_notice_seen_at, Some(start()));
    tx.commit().await.unwrap();

    let history = fx.store.audit_history(AuditEntity::Event, event.id.0).await.unwrap();
    assert_eq!(history.len(), 2);
}

pub async fn events_between_returns_overlapping_events<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + EventWrites> + EventRepo,
{
    let owner = add_user(fx, "daniele", Role::Resident).await;
    let morning = start() - TimeDelta::hours(12);
    let at = |hours: i64| morning + TimeDelta::hours(hours);
    let first = add_event(fx, owner.id, at(0), closure(at(1), CloseReason::Manual)).await;
    let second = add_event(fx, owner.id, at(2), closure(at(3), CloseReason::Manual)).await;
    let open = add_event(fx, owner.id, at(8), None).await;

    let between = |from, to| async move {
        fx.store
            .events_between(from, to)
            .await
            .unwrap()
            .into_iter()
            .map(|e: Event| e.id)
            .collect::<Vec<EventId>>()
    };
    let half = TimeDelta::minutes(30);
    assert_eq!(between(at(0) + half, at(2) + half).await, [first.id, second.id]);
    assert_eq!(between(at(1), at(2)).await, [first.id]);
    assert!(between(at(4), at(5)).await.is_empty());
    assert_eq!(between(at(10), at(11)).await, [open.id]);
}
```

In `lib.rs` add `pub mod events;` and append to the macro list:

```rust
            events::a_new_event_can_be_the_open_event,
            events::only_one_event_can_be_open,
            events::closing_and_reopening_are_audited,
            events::reopening_is_refused_while_another_event_is_open,
            events::the_end_of_a_closed_event_can_move,
            events::the_auto_close_notice_is_marked_seen_once,
            events::events_between_returns_overlapping_events,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite`
Expected: compile errors — `SqliteStore: EventRepo` / `SqliteTx: EventWrites` not satisfied.

- [ ] **Step 3: Implement**

`crates/storage-sqlite/src/events.rs`:

```rust
//! Events: one smoke incident each. At most one is open at a time.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{Closure, Event, NewEvent};
use smoke_domain::repos::{Conflict, EventRepo, EventWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, UserId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant, parse};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct EventRow {
    id: i64,
    owner_id: i64,
    opened_at: i64,
    closed_at: Option<i64>,
    close_reason: Option<String>,
    auto_close_notice_seen_at: Option<i64>,
    recorded_at: i64,
}

impl EventRow {
    fn into_event(self) -> StoreResult<Event> {
        let closed = match (self.closed_at, self.close_reason) {
            (Some(at), Some(reason)) => Some(Closure {
                at: instant(at)?,
                reason: parse(&reason)?,
            }),
            (None, None) => None,
            _ => {
                return Err(corrupt(format!(
                    "event {}: closed_at and close_reason disagree",
                    self.id
                )));
            }
        };
        Ok(Event {
            id: EventId(self.id),
            owner_id: UserId(self.owner_id),
            opened_at: instant(self.opened_at)?,
            closed,
            auto_close_notice_seen_at: opt_instant(self.auto_close_notice_seen_at)?,
            recorded_at: instant(self.recorded_at)?,
        })
    }
}

async fn fetch(conn: &mut SqliteConnection, id: EventId) -> StoreResult<Option<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(EventRow::into_event)
    .transpose()
}

async fn require(conn: &mut SqliteConnection, id: EventId) -> StoreResult<Event> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::Event,
        id: id.0,
    })
}

async fn fetch_open(conn: &mut SqliteConnection) -> StoreResult<Option<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events WHERE closed_at IS NULL"
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(EventRow::into_event)
    .transpose()
}

async fn audited(tx: &mut SqliteTx, action: AuditAction, before: Event) -> StoreResult<Event> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl EventRepo for SqliteStore {
    async fn event(&self, id: EventId) -> StoreResult<Option<Event>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn open_event(&self) -> StoreResult<Option<Event>> {
        fetch_open(&mut *self.read().await?).await
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> StoreResult<Vec<Event>> {
        sqlx::query_as!(
            EventRow,
            "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
             FROM events WHERE (closed_at IS NULL OR closed_at >= $1) AND opened_at < $2 \
             ORDER BY opened_at, id",
            micros(from),
            micros(to),
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(EventRow::into_event)
        .collect()
    }
}

impl EventWrites for SqliteTx {
    async fn open_event(&mut self) -> StoreResult<Option<Event>> {
        fetch_open(self.conn()).await
    }

    async fn event(&mut self, id: EventId) -> StoreResult<Option<Event>> {
        fetch(self.conn(), id).await
    }

    async fn latest_report_at(&mut self, event: EventId) -> StoreResult<Option<DateTime<Utc>>> {
        let latest = sqlx::query_scalar!(
            "SELECT MAX(observed_at) FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL",
            event.0,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        opt_instant(latest)
    }

    async fn create_event(&mut self, new: NewEvent) -> StoreResult<Event> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO events (owner_id, opened_at, closed_at, close_reason, recorded_at) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
            new.owner_id.0,
            micros(new.opened_at),
            new.closed.map(|c| micros(c.at)),
            new.closed.map(|c| c.reason.as_str()),
            now,
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let event = require(self.conn(), EventId(id)).await?;
        self.record(AuditAction::Create, None, &event).await?;
        Ok(event)
    }

    async fn close_event(&mut self, id: EventId, closure: Closure) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if !before.is_open() {
            return Err(Conflict::EventNotOpen.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = $1, close_reason = $2 WHERE id = $3",
            micros(closure.at),
            closure.reason.as_str(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Close, before).await
    }

    async fn reopen_event(&mut self, id: EventId) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.is_open() {
            return Err(Conflict::EventNotClosed.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = NULL, close_reason = NULL WHERE id = $1",
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Reopen, before).await
    }

    async fn move_event_end(&mut self, id: EventId, closed_at: DateTime<Utc>) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.is_open() {
            return Err(Conflict::EventNotClosed.into());
        }
        sqlx::query!(
            "UPDATE events SET closed_at = $1 WHERE id = $2",
            micros(closed_at),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn mark_auto_close_notice_seen(&mut self, id: EventId) -> StoreResult<Event> {
        let before = require(self.conn(), id).await?;
        if before.auto_close_notice_seen_at.is_some() {
            return Ok(before);
        }
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE events SET auto_close_notice_seen_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }
}
```

Add `mod events;` to `lib.rs`. If sqlx infers `MAX(observed_at)` as non-nullable `i64`, write the
column as `MAX(observed_at) AS "latest?"` to force `Option<i64>` (in the prototype it was already
inferred as `Option<i64>`).

- [ ] **Step 4: Regenerate metadata and run the tests**

Run: `scripts/sqlx-prepare.sh && cargo test -p smoke-storage-sqlite && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (23 contract tests).

- [ ] **Step 5: Commit**

```bash
git add crates/storage-sqlite crates/storage-contract
git commit -m "feat(storage): events with a partial unique index guarding the one-open-event rule

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: SQLite — smell reports

**Files:**
- Create: `crates/storage-sqlite/src/reports.rs`
- Modify: `crates/storage-sqlite/src/lib.rs` (add `mod reports;`)
- Create: `crates/storage-contract/src/reports.rs`
- Modify: `crates/storage-contract/src/lib.rs`, `crates/storage-contract/src/fixture.rs` (add `Scene`, `scene`, `report_values`, `add_report`)
- Regenerate: `crates/storage-sqlite/.sqlx/`

**Interfaces:**
- Consumes: Tasks 6–8.
- Produces: `impl ReportRepo for SqliteStore`, `impl ReportWrites for SqliteTx`;
  fixture helpers `Scene { user: User, event: Event, rooms: Vec<Room>, symptoms: Vec<Symptom> }`,
  `scene<S>(fx) -> Scene where S: Transactional<Tx: UserWrites + ListWrites + EventWrites> + ListRepo`,
  `report_values(intensity: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues`,
  `add_report<S>(fx, scene: &Scene, observed_at, values: ReportValues) -> SmellReport where S: Transactional<Tx: ReportWrites>`.

- [ ] **Step 1: Write the failing contract tests**

Append to `crates/storage-contract/src/fixture.rs` (extend the imports with
`smoke_domain::records::{NewReport, ReportValues, Room, SmellReport, Symptom}`,
`smoke_domain::repos::{ListRepo, ListWrites, ReportWrites}`,
`smoke_domain::starter::seed_starter_lists`,
`smoke_domain::types::{Intensity, RoomId, SymptomId}`):

```rust
/// Seeded lists, a resident, and an event they opened an hour before [`start`].
pub struct Scene {
    pub user: User,
    pub event: Event,
    pub rooms: Vec<Room>,
    pub symptoms: Vec<Symptom>,
}

pub async fn scene<S>(fx: &Fixture<S>) -> Scene
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites> + ListRepo,
{
    seed_starter_lists(&fx.store).await.expect("seed");
    let user = add_user(fx, "daniele", Role::Resident).await;
    let event = add_event(fx, user.id, start() - TimeDelta::hours(1), None).await;
    Scene {
        user,
        event,
        rooms: fx.store.rooms().await.expect("rooms"),
        symptoms: fx.store.symptoms().await.expect("symptoms"),
    }
}

pub fn report_values(intensity: u8, rooms: &[RoomId], symptoms: &[SymptomId]) -> ReportValues {
    ReportValues::new(
        Intensity::new(intensity).expect("valid intensity"),
        rooms.to_vec(),
        symptoms.to_vec(),
        None,
    )
    .expect("valid values")
}

pub async fn add_report<S>(
    fx: &Fixture<S>,
    scene: &Scene,
    observed_at: DateTime<Utc>,
    values: ReportValues,
) -> SmellReport
where
    S: Transactional<Tx: ReportWrites>,
{
    let mut tx = fx.store.begin(Some(scene.user.id)).await.expect("begin");
    let report = tx
        .create_report(NewReport {
            event_id: scene.event.id,
            user_id: scene.user.id,
            observed_at,
            values,
        })
        .await
        .expect("create report");
    tx.commit().await.expect("commit");
    report
}
```

`crates/storage-contract/src/reports.rs`:

```rust
//! Smell reports: creating, correcting, moving and soft-deleting.

use chrono::TimeDelta;
use serde_json::json;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::{Closure, NewReport};
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventWrites, ListRepo, ListWrites, ReportRepo, ReportWrites,
    StoreError, Transactional, UserWrites,
};
use smoke_domain::types::{CloseReason, RoomId};

use crate::Fixture;
use crate::fixture::{add_event, add_report, last_entry, report_values, scene, start};

pub async fn a_report_keeps_its_rooms_and_symptoms<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let observed_at = start() - TimeDelta::minutes(2);
    let report = add_report(
        fx,
        &scene,
        observed_at,
        report_values(6, &[r2, r0, r2], &[scene.symptoms[0].id]),
    )
    .await;

    assert_eq!(report.values.rooms(), [r0, r2]);
    assert_eq!(report.values.symptoms(), [scene.symptoms[0].id]);
    assert_eq!(
        (report.observed_at, report.recorded_at, report.event_id, report.user_id),
        (observed_at, start(), scene.event.id, scene.user.id)
    );
    assert_eq!(fx.store.report(report.id).await.unwrap(), Some(report.clone()));
    assert_eq!(
        fx.store.reports_for_event(scene.event.id).await.unwrap(),
        [report]
    );
}

pub async fn correcting_a_report_records_before_and_after<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let (r0, r2) = (scene.rooms[0].id, scene.rooms[2].id);
    let report = add_report(fx, &scene, start(), report_values(6, &[r0], &[])).await;
    fx.clock.advance(TimeDelta::hours(13));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let corrected = tx
        .correct_report(report.id, report.observed_at, report_values(7, &[r0, r2], &[]))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(corrected.values.intensity().get(), 7);
    assert_eq!(corrected.recorded_at, report.recorded_at);
    let entry = last_entry(fx).await.record;
    assert_eq!(entry.action, AuditAction::Update);
    assert_eq!(entry.at_micros, fx.clock.now().timestamp_micros());
    let before = entry.before.unwrap();
    let after = entry.after.unwrap();
    assert_eq!(before["intensity"], 6);
    assert_eq!(after["intensity"], 7);
    assert_eq!(after["rooms"], json!([r0.0, r2.0]));
}

pub async fn moving_a_report_is_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let yesterday = start() - TimeDelta::days(1);
    let other = add_event(
        fx,
        scene.user.id,
        yesterday,
        Some(Closure {
            at: yesterday,
            reason: CloseReason::Backdated,
        }),
    )
    .await;
    let report = add_report(fx, &scene, start(), report_values(4, &[], &[])).await;

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let moved = tx.move_report(report.id, other.id).await.unwrap();
    assert_eq!(moved.event_id, other.id);
    tx.move_report(report.id, other.id).await.unwrap();
    tx.commit().await.unwrap();

    assert!(fx.store.reports_for_event(scene.event.id).await.unwrap().is_empty());
    assert_eq!(fx.store.reports_for_event(other.id).await.unwrap(), [moved]);
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::SmellReport, report.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(actions, [AuditAction::Create, AuditAction::Move]);
}

pub async fn deleting_a_report_is_soft<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let report = add_report(fx, &scene, start(), report_values(5, &[], &[])).await;
    fx.clock.advance(TimeDelta::minutes(1));

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let deleted = tx.delete_report(report.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    for error in [
        tx.delete_report(report.id).await.unwrap_err(),
        tx.correct_report(report.id, start(), report_values(6, &[], &[]))
            .await
            .unwrap_err(),
        tx.move_report(report.id, scene.event.id).await.unwrap_err(),
    ] {
        assert!(
            matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
            "{error:?}"
        );
    }
    tx.commit().await.unwrap();

    assert_eq!(fx.store.report(report.id).await.unwrap(), Some(deleted));
    assert!(fx.store.reports_for_event(scene.event.id).await.unwrap().is_empty());
    assert_eq!(last_entry(fx).await.record.action, AuditAction::Delete);
}

pub async fn latest_report_time_ignores_deleted_reports<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites> + ListRepo,
{
    let scene = scene(fx).await;
    let earlier = start() - TimeDelta::minutes(30);
    let later = start() - TimeDelta::minutes(10);
    add_report(fx, &scene, earlier, report_values(3, &[], &[])).await;
    let last = add_report(fx, &scene, later, report_values(5, &[], &[])).await;

    let mut tx = fx.store.begin(None).await.unwrap();
    assert_eq!(tx.latest_report_at(scene.event.id).await.unwrap(), Some(later));
    tx.delete_report(last.id).await.unwrap();
    assert_eq!(tx.latest_report_at(scene.event.id).await.unwrap(), Some(earlier));
    let empty = tx
        .create_event(smoke_domain::records::NewEvent {
            owner_id: scene.user.id,
            opened_at: start() - TimeDelta::days(2),
            closed: Some(Closure {
                at: start() - TimeDelta::days(2),
                reason: CloseReason::Backdated,
            }),
        })
        .await
        .unwrap();
    assert_eq!(tx.latest_report_at(empty.id).await.unwrap(), None);
}

pub async fn an_unknown_room_rolls_back_the_report<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReportWrites>
        + ListRepo
        + ReportRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let entries_before = fx.store.audit_log().await.unwrap().len();

    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let error = tx
        .create_report(NewReport {
            event_id: scene.event.id,
            user_id: scene.user.id,
            observed_at: start(),
            values: report_values(5, &[RoomId(999)], &[]),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::UnknownReference)),
        "{error:?}"
    );
    drop(tx);

    assert!(fx.store.reports_for_event(scene.event.id).await.unwrap().is_empty());
    assert_eq!(fx.store.audit_log().await.unwrap().len(), entries_before);
}
```

In `lib.rs` add `pub mod reports;` and append to the macro list:

```rust
            reports::a_report_keeps_its_rooms_and_symptoms,
            reports::correcting_a_report_records_before_and_after,
            reports::moving_a_report_is_audited,
            reports::deleting_a_report_is_soft,
            reports::latest_report_time_ignores_deleted_reports,
            reports::an_unknown_room_rolls_back_the_report,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite`
Expected: compile errors — `SqliteStore: ReportRepo` / `SqliteTx: ReportWrites` not satisfied.

- [ ] **Step 3: Implement**

`crates/storage-sqlite/src/reports.rs`:

```rust
//! Smell reports, with their rooms and symptoms in link tables. Deleted softly.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewReport, ReportValues, SmellReport};
use smoke_domain::repos::{Conflict, ReportRepo, ReportWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, Intensity, ReportId, RoomId, SymptomId, UserId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct ReportRow {
    id: i64,
    event_id: i64,
    user_id: i64,
    observed_at: i64,
    recorded_at: i64,
    intensity: i64,
    note: Option<String>,
    deleted_at: Option<i64>,
}

async fn load(conn: &mut SqliteConnection, row: ReportRow) -> StoreResult<SmellReport> {
    let rooms = sqlx::query_scalar!(
        "SELECT room_id FROM smell_report_rooms WHERE smell_report_id = $1 ORDER BY room_id",
        row.id,
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    let symptoms = sqlx::query_scalar!(
        "SELECT symptom_id FROM smell_report_symptoms WHERE smell_report_id = $1 ORDER BY symptom_id",
        row.id,
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    let intensity = u8::try_from(row.intensity)
        .map_err(corrupt)
        .and_then(|value| Intensity::new(value).map_err(corrupt))?;
    let values = ReportValues::new(
        intensity,
        rooms.into_iter().map(RoomId).collect(),
        symptoms.into_iter().map(SymptomId).collect(),
        row.note.as_deref(),
    )
    .map_err(corrupt)?;
    Ok(SmellReport {
        id: ReportId(row.id),
        event_id: EventId(row.event_id),
        user_id: UserId(row.user_id),
        observed_at: instant(row.observed_at)?,
        recorded_at: instant(row.recorded_at)?,
        values,
        deleted_at: opt_instant(row.deleted_at)?,
    })
}

async fn load_all(conn: &mut SqliteConnection, rows: Vec<ReportRow>) -> StoreResult<Vec<SmellReport>> {
    let mut reports = Vec::with_capacity(rows.len());
    for row in rows {
        reports.push(load(&mut *conn, row).await?);
    }
    Ok(reports)
}

async fn fetch(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<Option<SmellReport>> {
    let row = sqlx::query_as!(
        ReportRow,
        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
         FROM smell_reports WHERE id = $1",
        id.0,
    )
    .fetch_optional(&mut *conn)
    .await
    .db()?;
    match row {
        Some(row) => load(conn, row).await.map(Some),
        None => Ok(None),
    }
}

async fn require(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<SmellReport> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::SmellReport,
        id: id.0,
    })
}

/// Deleted reports cannot be changed.
async fn require_live(conn: &mut SqliteConnection, id: ReportId) -> StoreResult<SmellReport> {
    let report = require(conn, id).await?;
    if report.is_deleted() {
        Err(Conflict::AlreadyDeleted.into())
    } else {
        Ok(report)
    }
}

async fn insert_links(conn: &mut SqliteConnection, id: i64, values: &ReportValues) -> StoreResult<()> {
    for room in values.rooms() {
        sqlx::query!(
            "INSERT INTO smell_report_rooms (smell_report_id, room_id) VALUES ($1, $2)",
            id,
            room.0,
        )
        .execute(&mut *conn)
        .await
        .db()?;
    }
    for symptom in values.symptoms() {
        sqlx::query!(
            "INSERT INTO smell_report_symptoms (smell_report_id, symptom_id) VALUES ($1, $2)",
            id,
            symptom.0,
        )
        .execute(&mut *conn)
        .await
        .db()?;
    }
    Ok(())
}

async fn delete_links(conn: &mut SqliteConnection, id: i64) -> StoreResult<()> {
    sqlx::query!("DELETE FROM smell_report_rooms WHERE smell_report_id = $1", id)
        .execute(&mut *conn)
        .await
        .db()?;
    sqlx::query!("DELETE FROM smell_report_symptoms WHERE smell_report_id = $1", id)
        .execute(&mut *conn)
        .await
        .db()?;
    Ok(())
}

async fn audited(tx: &mut SqliteTx, action: AuditAction, before: SmellReport) -> StoreResult<SmellReport> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl ReportRepo for SqliteStore {
    async fn report(&self, id: ReportId) -> StoreResult<Option<SmellReport>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn reports_for_event(&self, event: EventId) -> StoreResult<Vec<SmellReport>> {
        let mut conn = self.read().await?;
        let rows = sqlx::query_as!(
            ReportRow,
            "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
             FROM smell_reports WHERE event_id = $1 AND deleted_at IS NULL \
             ORDER BY observed_at, id",
            event.0,
        )
        .fetch_all(&mut *conn)
        .await
        .db()?;
        load_all(&mut conn, rows).await
    }
}

impl ReportWrites for SqliteTx {
    async fn report(&mut self, id: ReportId) -> StoreResult<Option<SmellReport>> {
        fetch(self.conn(), id).await
    }

    async fn create_report(&mut self, new: NewReport) -> StoreResult<SmellReport> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO smell_reports (event_id, user_id, observed_at, recorded_at, intensity, note) \
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            new.event_id.0,
            new.user_id.0,
            micros(new.observed_at),
            now,
            i64::from(new.values.intensity().get()),
            new.values.note(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        insert_links(self.conn(), id, &new.values).await?;
        let report = require(self.conn(), ReportId(id)).await?;
        self.record(AuditAction::Create, None, &report).await?;
        Ok(report)
    }

    async fn correct_report(
        &mut self,
        id: ReportId,
        observed_at: DateTime<Utc>,
        values: ReportValues,
    ) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE smell_reports SET observed_at = $1, intensity = $2, note = $3 WHERE id = $4",
            micros(observed_at),
            i64::from(values.intensity().get()),
            values.note(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        delete_links(self.conn(), id.0).await?;
        insert_links(self.conn(), id.0, &values).await?;
        audited(self, AuditAction::Update, before).await
    }

    async fn move_report(&mut self, id: ReportId, to: EventId) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        if before.event_id == to {
            return Ok(before);
        }
        sqlx::query!(
            "UPDATE smell_reports SET event_id = $1 WHERE id = $2",
            to.0,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Move, before).await
    }

    async fn delete_report(&mut self, id: ReportId) -> StoreResult<SmellReport> {
        let before = require_live(self.conn(), id).await?;
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE smell_reports SET deleted_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Delete, before).await
    }
}
```

Add `mod reports;` to `lib.rs`.

- [ ] **Step 4: Regenerate metadata and run the tests**

Run: `scripts/sqlx-prepare.sh && cargo test -p smoke-storage-sqlite && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (29 contract tests).

- [ ] **Step 5: Commit**

```bash
git add crates/storage-sqlite crates/storage-contract
git commit -m "feat(storage): smell reports with link tables, soft delete and audited moves

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: SQLite — PM2.5 readings and job bookkeeping

**Files:**
- Create: `crates/storage-sqlite/src/readings.rs`, `crates/storage-sqlite/src/jobs.rs`
- Modify: `crates/storage-sqlite/src/lib.rs` (add `mod jobs; mod readings;`)
- Create: `crates/storage-contract/src/readings.rs`
- Modify: `crates/storage-contract/src/lib.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`

**Interfaces:**
- Consumes: Tasks 6–9.
- Produces: `impl ReadingRepo + JobRepo for SqliteStore`, `impl ReadingWrites + JobWrites for SqliteTx`.
  After this task `SqliteTx: WriteTx`.

- [ ] **Step 1: Write the failing contract tests**

`crates/storage-contract/src/readings.rs`:

```rust
//! PM2.5 readings, and job bookkeeping.

use chrono::TimeDelta;
use serde_json::json;
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::clock::Clock;
use smoke_domain::records::NewReading;
use smoke_domain::repos::{
    AuditRepo, Commit, Conflict, EventWrites, JobRepo, JobWrites, ListRepo, ListWrites,
    ReadingRepo, ReadingWrites, StoreError, Transactional, UserWrites,
};
use smoke_domain::types::Pm25;

use crate::Fixture;
use crate::fixture::{last_entry, scene, start};

fn pm25(value: f64) -> Pm25 {
    Pm25::new(value).expect("valid PM2.5")
}

pub async fn a_reading_can_stand_alone_or_join_an_event<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo,
{
    let scene = scene(fx).await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let alone = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: None,
            observed_at: start() - TimeDelta::days(1),
            value: pm25(8.0),
        })
        .await
        .unwrap();
    let linked = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: pm25(35.5),
        })
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(alone.event_id, None);
    assert_eq!(linked.recorded_at, start());
    assert_eq!(fx.store.reading(alone.id).await.unwrap(), Some(alone));
    assert_eq!(
        fx.store.readings_for_event(scene.event.id).await.unwrap(),
        [linked]
    );
}

pub async fn correcting_and_deleting_a_reading_are_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let reading = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: pm25(12.5),
        })
        .await
        .unwrap();
    let corrected = tx
        .correct_reading(reading.id, reading.observed_at, pm25(14.0))
        .await
        .unwrap();
    assert_eq!(corrected.value, pm25(14.0));
    fx.clock.advance(TimeDelta::minutes(1));
    let deleted = tx.delete_reading(reading.id).await.unwrap();
    assert_eq!(deleted.deleted_at, Some(fx.clock.now()));
    let error = tx
        .correct_reading(reading.id, start(), pm25(1.0))
        .await
        .unwrap_err();
    assert!(
        matches!(error, StoreError::Conflict(Conflict::AlreadyDeleted)),
        "{error:?}"
    );
    tx.commit().await.unwrap();

    assert!(fx.store.readings_for_event(scene.event.id).await.unwrap().is_empty());
    assert_eq!(fx.store.reading(reading.id).await.unwrap(), Some(deleted));
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::Pm25Reading, reading.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(
        actions,
        [AuditAction::Create, AuditAction::Update, AuditAction::Delete]
    );
}

pub async fn pm25_values_round_trip_exactly<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: UserWrites + ListWrites + EventWrites + ReadingWrites>
        + ListRepo
        + ReadingRepo
        + AuditRepo,
{
    let scene = scene(fx).await;
    for value in [0.0, 0.1, 12.3, 999.99, 1000.0] {
        let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
        let reading = tx
            .create_reading(NewReading {
                user_id: scene.user.id,
                event_id: None,
                observed_at: start(),
                value: pm25(value),
            })
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let stored = fx.store.reading(reading.id).await.unwrap().unwrap();
        assert_eq!(stored.value, pm25(value));
        assert_eq!(
            last_entry(fx).await.record.after.unwrap()["value"],
            json!(value)
        );
    }
}

pub async fn job_runs_are_remembered_but_not_audited<S>(fx: &Fixture<S>)
where
    S: Transactional<Tx: JobWrites> + JobRepo + AuditRepo,
{
    const SWEEP: &str = "auto_close_sweep";
    assert_eq!(fx.store.job_last_run(SWEEP).await.unwrap(), None);
    let mut tx = fx.store.begin(None).await.unwrap();
    tx.record_job_run(SWEEP, start()).await.unwrap();
    tx.record_job_run(SWEEP, start() + TimeDelta::days(1)).await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        fx.store.job_last_run(SWEEP).await.unwrap(),
        Some(start() + TimeDelta::days(1))
    );
    assert!(fx.store.audit_log().await.unwrap().is_empty());
}
```

In `lib.rs` add `pub mod readings;` and append to the macro list:

```rust
            readings::a_reading_can_stand_alone_or_join_an_event,
            readings::correcting_and_deleting_a_reading_are_audited,
            readings::pm25_values_round_trip_exactly,
            readings::job_runs_are_remembered_but_not_audited,
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite`
Expected: compile errors — `ReadingRepo`, `ReadingWrites`, `JobRepo`, `JobWrites` not implemented.

- [ ] **Step 3: Implement**

`crates/storage-sqlite/src/readings.rs`:

```rust
//! PM2.5 readings: hand-entered sensor values, optionally linked to an event. Deleted softly.

use chrono::{DateTime, Utc};
use smoke_domain::audit::{AuditAction, AuditEntity};
use smoke_domain::records::{NewReading, Pm25Reading};
use smoke_domain::repos::{Conflict, ReadingRepo, ReadingWrites, StoreError, StoreResult};
use smoke_domain::types::{EventId, Pm25, ReadingId, UserId};
use sqlx::SqliteConnection;

use crate::convert::{corrupt, instant, micros, opt_instant};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

struct ReadingRow {
    id: i64,
    user_id: i64,
    event_id: Option<i64>,
    observed_at: i64,
    recorded_at: i64,
    value_ugm3: f64,
    deleted_at: Option<i64>,
}

impl ReadingRow {
    fn into_reading(self) -> StoreResult<Pm25Reading> {
        Ok(Pm25Reading {
            id: ReadingId(self.id),
            user_id: UserId(self.user_id),
            event_id: self.event_id.map(EventId),
            observed_at: instant(self.observed_at)?,
            recorded_at: instant(self.recorded_at)?,
            value: Pm25::new(self.value_ugm3).map_err(corrupt)?,
            deleted_at: opt_instant(self.deleted_at)?,
        })
    }
}

async fn fetch(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Option<Pm25Reading>> {
    sqlx::query_as!(
        ReadingRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
         FROM pm25_readings WHERE id = $1",
        id.0,
    )
    .fetch_optional(conn)
    .await
    .db()?
    .map(ReadingRow::into_reading)
    .transpose()
}

async fn require(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Pm25Reading> {
    fetch(conn, id).await?.ok_or(StoreError::NotFound {
        entity: AuditEntity::Pm25Reading,
        id: id.0,
    })
}

async fn require_live(conn: &mut SqliteConnection, id: ReadingId) -> StoreResult<Pm25Reading> {
    let reading = require(conn, id).await?;
    if reading.is_deleted() {
        Err(Conflict::AlreadyDeleted.into())
    } else {
        Ok(reading)
    }
}

async fn audited(tx: &mut SqliteTx, action: AuditAction, before: Pm25Reading) -> StoreResult<Pm25Reading> {
    let after = require(tx.conn(), before.id).await?;
    tx.record(action, Some(&before), &after).await?;
    Ok(after)
}

impl ReadingRepo for SqliteStore {
    async fn reading(&self, id: ReadingId) -> StoreResult<Option<Pm25Reading>> {
        fetch(&mut *self.read().await?, id).await
    }

    async fn readings_for_event(&self, event: EventId) -> StoreResult<Vec<Pm25Reading>> {
        sqlx::query_as!(
            ReadingRow,
            "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
             FROM pm25_readings WHERE event_id = $1 AND deleted_at IS NULL \
             ORDER BY observed_at, id",
            event.0,
        )
        .fetch_all(&mut *self.read().await?)
        .await
        .db()?
        .into_iter()
        .map(ReadingRow::into_reading)
        .collect()
    }
}

impl ReadingWrites for SqliteTx {
    async fn create_reading(&mut self, new: NewReading) -> StoreResult<Pm25Reading> {
        let now = micros(self.now());
        let id = sqlx::query_scalar!(
            "INSERT INTO pm25_readings (user_id, event_id, observed_at, recorded_at, value_ugm3) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
            new.user_id.0,
            new.event_id.map(|e| e.0),
            micros(new.observed_at),
            now,
            new.value.ugm3(),
        )
        .fetch_one(self.conn())
        .await
        .db()?;
        let reading = require(self.conn(), ReadingId(id)).await?;
        self.record(AuditAction::Create, None, &reading).await?;
        Ok(reading)
    }

    async fn correct_reading(
        &mut self,
        id: ReadingId,
        observed_at: DateTime<Utc>,
        value: Pm25,
    ) -> StoreResult<Pm25Reading> {
        let before = require_live(self.conn(), id).await?;
        sqlx::query!(
            "UPDATE pm25_readings SET observed_at = $1, value_ugm3 = $2 WHERE id = $3",
            micros(observed_at),
            value.ugm3(),
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Update, before).await
    }

    async fn delete_reading(&mut self, id: ReadingId) -> StoreResult<Pm25Reading> {
        let before = require_live(self.conn(), id).await?;
        let now = micros(self.now());
        sqlx::query!(
            "UPDATE pm25_readings SET deleted_at = $1 WHERE id = $2",
            now,
            id.0,
        )
        .execute(self.conn())
        .await
        .db()?;
        audited(self, AuditAction::Delete, before).await
    }
}
```

`crates/storage-sqlite/src/jobs.rs`:

```rust
//! When each background job last ran. Bookkeeping, not evidence: not audited.

use chrono::{DateTime, Utc};
use smoke_domain::repos::{JobRepo, JobWrites, StoreResult};

use crate::convert::{micros, opt_instant};
use crate::error::IntoStore;
use crate::{SqliteStore, SqliteTx};

impl JobRepo for SqliteStore {
    async fn job_last_run(&self, name: &str) -> StoreResult<Option<DateTime<Utc>>> {
        let last = sqlx::query_scalar!("SELECT last_run_at FROM jobs WHERE name = $1", name)
            .fetch_optional(&mut *self.read().await?)
            .await
            .db()?;
        opt_instant(last)
    }
}

impl JobWrites for SqliteTx {
    async fn record_job_run(&mut self, name: &str, at: DateTime<Utc>) -> StoreResult<()> {
        sqlx::query!(
            "INSERT INTO jobs (name, last_run_at) VALUES ($1, $2) \
             ON CONFLICT (name) DO UPDATE SET last_run_at = excluded.last_run_at",
            name,
            micros(at),
        )
        .execute(self.conn())
        .await
        .db()?;
        Ok(())
    }
}
```

Add `mod jobs;` and `mod readings;` to `lib.rs`.

- [ ] **Step 4: Regenerate metadata and run the tests**

Run: `scripts/sqlx-prepare.sh && cargo test -p smoke-storage-sqlite && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (33 contract tests).

- [ ] **Step 5: Commit**

```bash
git add crates/storage-sqlite crates/storage-contract
git commit -m "feat(storage): PM2.5 readings and unaudited job bookkeeping (upsert)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 11: SQLite — consistent audit state and `verify-audit` against real tampering

**Files:**
- Modify: `crates/storage-sqlite/src/audit.rs` (add `SnapshotRepo`)
- Modify: `crates/storage-sqlite/src/events.rs`, `reports.rs`, `readings.rs` (add `fetch_all`)
- Create: `crates/storage-contract/src/audit.rs`
- Modify: `crates/storage-contract/src/lib.rs`
- Regenerate: `crates/storage-sqlite/.sqlx/`

**Interfaces:**
- Consumes: everything above; `verify_store`, `AuditState`, `Snapshot` (Task 5).
- Produces: `impl SnapshotRepo for SqliteStore` — after this task `SqliteStore: Store`.
  Crate-internal `events::fetch_all`, `reports::fetch_all`, `readings::fetch_all` (all rows,
  deleted included, ordered by id).

- [ ] **Step 1: Write the failing contract tests**

`crates/storage-contract/src/audit.rs`:

```rust
//! The hash chain and `verify-audit`, including tampering done directly in the database.

use chrono::TimeDelta;
use smoke_domain::audit::{AuditAction, AuditEntity, ChainError, verify_chain};
use smoke_domain::records::{NewReading, NewRoom, Pm25Reading, SmellReport};
use smoke_domain::repos::{
    AuditRepo, Commit, ListRepo, ListWrites, ReadingWrites, ReportWrites, Store, Transactional,
};
use smoke_domain::types::{ListName, Pm25, RoomArea};
use smoke_domain::verify::{SnapshotProblem, verify_store};

use crate::Fixture;
use crate::fixture::{add_report, report_values, scene, start};

struct Populated {
    report: SmellReport,
    reading: Pm25Reading,
}

/// Seeding, a user, an event, a report (intensity 6), a reading and a correction.
async fn populate<S: Store>(fx: &Fixture<S>) -> Populated {
    let scene = scene(fx).await;
    let rooms = [scene.rooms[0].id, scene.rooms[1].id];
    let report = add_report(
        fx,
        &scene,
        start() - TimeDelta::minutes(1),
        report_values(6, &rooms[..1], &[scene.symptoms[0].id]),
    )
    .await;
    let mut tx = fx.store.begin(Some(scene.user.id)).await.unwrap();
    let reading = tx
        .create_reading(NewReading {
            user_id: scene.user.id,
            event_id: Some(scene.event.id),
            observed_at: start(),
            value: Pm25::new(42.0).unwrap(),
        })
        .await
        .unwrap();
    tx.correct_report(report.id, report.observed_at, report_values(6, &rooms, &[]))
        .await
        .unwrap();
    tx.commit().await.unwrap();
    Populated { report, reading }
}

pub async fn every_write_extends_one_unbroken_chain<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    let log = fx.store.audit_log().await.unwrap();
    let ids: Vec<i64> = log.iter().map(|e| e.record.id).collect();
    assert_eq!(ids, (1..=log.len() as i64).collect::<Vec<_>>());
    assert_eq!(verify_chain(&log), Ok(()));

    let report = verify_store(&fx.store).await.unwrap();
    assert!(report.is_ok(), "{report:?}");
    assert_eq!(report.entries, log.len());
    assert_eq!(report.head_hash, log.last().unwrap().hash);
}

pub async fn the_history_of_a_record_lists_its_entries<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let actions: Vec<_> = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .iter()
        .map(|e| e.record.action)
        .collect();
    assert_eq!(actions, [AuditAction::Create, AuditAction::Update]);
}

pub async fn a_direct_record_edit_is_detected<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let latest = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .pop()
        .unwrap();
    fx.raw("UPDATE smell_reports SET intensity = 9").await.unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(report.chain, Ok(()));
    assert_eq!(
        report.problems,
        [SnapshotProblem::Differs {
            entity: AuditEntity::SmellReport,
            entity_id: p.report.id.0,
            audit_id: latest.record.id,
        }]
    );
}

pub async fn a_directly_inserted_record_is_detected<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    fx.raw("INSERT INTO symptoms (name) VALUES ('Dizziness')")
        .await
        .unwrap();
    let inserted = fx
        .store
        .symptoms()
        .await
        .unwrap()
        .into_iter()
        .find(|s| s.name.as_str() == "Dizziness")
        .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.problems,
        [SnapshotProblem::Unaudited {
            entity: AuditEntity::Symptom,
            entity_id: inserted.id.0,
        }]
    );
}

pub async fn a_directly_deleted_record_is_detected<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    fx.raw("DELETE FROM pm25_readings").await.unwrap();
    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.problems,
        [SnapshotProblem::Vanished {
            entity: AuditEntity::Pm25Reading,
            entity_id: p.reading.id.0,
        }]
    );
}

pub async fn the_audit_log_refuses_updates_and_deletes<S: Store>(fx: &Fixture<S>) {
    populate(fx).await;
    assert!(fx.raw("UPDATE audit_log SET hash = 'x'").await.is_err());
    assert!(fx.raw("DELETE FROM audit_log").await.is_err());
    assert!(verify_store(&fx.store).await.unwrap().is_ok());
}

pub async fn an_edited_audit_entry_breaks_the_chain<S: Store>(fx: &Fixture<S>) {
    let p = populate(fx).await;
    let first = fx
        .store
        .audit_history(AuditEntity::SmellReport, p.report.id.0)
        .await
        .unwrap()
        .remove(0);
    fx.drop_audit_guard().await;
    fx.raw(
        r#"UPDATE audit_log SET after_json = replace(after_json, '"intensity":6', '"intensity":9') WHERE entity = 'smell_report'"#,
    )
    .await
    .unwrap();

    let report = verify_store(&fx.store).await.unwrap();
    assert_eq!(
        report.chain,
        Err(ChainError::HashMismatch {
            id: first.record.id
        })
    );
}

pub async fn concurrent_writers_never_fork_the_chain<S>(fx: &Fixture<S>)
where
    S: Store + Clone + 'static,
{
    let tasks: Vec<_> = (0..8)
        .map(|i| {
            let store = fx.store.clone();
            tokio::spawn(async move {
                let mut tx = store.begin(None).await?;
                tx.create_room(NewRoom {
                    name: ListName::new(&format!("Room {i}")).expect("valid name"),
                    i18n_key: None,
                    area: RoomArea::Inside,
                })
                .await?;
                tx.commit().await
            })
        })
        .collect();
    for task in tasks {
        task.await.expect("task panicked").expect("write");
    }
    let log = fx.store.audit_log().await.unwrap();
    assert_eq!(log.len(), 8);
    assert_eq!(verify_chain(&log), Ok(()));
    assert_eq!(fx.store.rooms().await.unwrap().len(), 8);
}
```

`ListRepo`, `ListWrites`, `ReadingWrites`, `ReportWrites`, `Transactional` are imported so their
methods resolve on `S: Store`; if the compiler reports any of them as unused, remove that import.

In `lib.rs` add `pub mod audit;` and append to the macro list:

```rust
            audit::every_write_extends_one_unbroken_chain,
            audit::the_history_of_a_record_lists_its_entries,
            audit::a_direct_record_edit_is_detected,
            audit::a_directly_inserted_record_is_detected,
            audit::a_directly_deleted_record_is_detected,
            audit::the_audit_log_refuses_updates_and_deletes,
            audit::an_edited_audit_entry_breaks_the_chain,
            audit::concurrent_writers_never_fork_the_chain,
```

Add to the bottom of `crates/storage-sqlite/tests/contract.rs`:

```rust
#[test]
fn the_sqlite_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<SqliteStore>();
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-storage-sqlite`
Expected: compile errors — `SqliteStore: SnapshotRepo` (and so `Store`) not satisfied.

- [ ] **Step 3: Implement**

Add to `crates/storage-sqlite/src/events.rs`:

```rust
pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<Event>> {
    sqlx::query_as!(
        EventRow,
        "SELECT id, owner_id, opened_at, closed_at, close_reason, auto_close_notice_seen_at, recorded_at \
         FROM events ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(EventRow::into_event)
    .collect()
}
```

Add to `crates/storage-sqlite/src/reports.rs`:

```rust
pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<SmellReport>> {
    let rows = sqlx::query_as!(
        ReportRow,
        "SELECT id, event_id, user_id, observed_at, recorded_at, intensity, note, deleted_at \
         FROM smell_reports ORDER BY id"
    )
    .fetch_all(&mut *conn)
    .await
    .db()?;
    load_all(conn, rows).await
}
```

Add to `crates/storage-sqlite/src/readings.rs`:

```rust
pub(crate) async fn fetch_all(conn: &mut SqliteConnection) -> StoreResult<Vec<Pm25Reading>> {
    sqlx::query_as!(
        ReadingRow,
        "SELECT id, user_id, event_id, observed_at, recorded_at, value_ugm3, deleted_at \
         FROM pm25_readings ORDER BY id"
    )
    .fetch_all(conn)
    .await
    .db()?
    .into_iter()
    .map(ReadingRow::into_reading)
    .collect()
}
```

Add to `crates/storage-sqlite/src/audit.rs` (extend imports with
`smoke_domain::repos::SnapshotRepo`, `smoke_domain::verify::{AuditState, Snapshot}` and
`crate::{events, lists, readings, reports, users}`):

```rust
impl SnapshotRepo for SqliteStore {
    /// One read transaction, so the log and the records come from the same moment even while
    /// the app keeps writing.
    async fn audit_state(&self) -> StoreResult<AuditState> {
        let mut tx = self.reader.begin().await.db()?;
        let entries = fetch_entries(&mut tx).await?;
        let mut snapshots = Vec::new();
        snapshots.extend(users::fetch_all(&mut tx).await?.iter().map(Snapshot::of));
        snapshots.extend(lists::fetch_rooms(&mut tx).await?.iter().map(Snapshot::of));
        snapshots.extend(lists::fetch_symptoms(&mut tx).await?.iter().map(Snapshot::of));
        snapshots.extend(events::fetch_all(&mut tx).await?.iter().map(Snapshot::of));
        snapshots.extend(reports::fetch_all(&mut tx).await?.iter().map(Snapshot::of));
        snapshots.extend(readings::fetch_all(&mut tx).await?.iter().map(Snapshot::of));
        Ok(AuditState { entries, snapshots })
    }
}
```

- [ ] **Step 4: Regenerate metadata and run the tests**

Run: `scripts/sqlx-prepare.sh && cargo test -p smoke-storage-sqlite && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS (41 contract tests + 3 SQLite tests + 1 unit test).

- [ ] **Step 5: Commit**

```bash
git add crates/storage-sqlite crates/storage-contract
git commit -m "feat(storage): verify-audit catches direct edits, inserts, deletes and log tampering

The audit state is read in one read transaction so a running app cannot cause
false alarms. Tamper tests go around the store with raw SQL, as someone with
database access would; triggers keep the log append-only.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 12: Postgres store — the same contract on a second backend

The Postgres crate mirrors the SQLite crate. Its SQL text is identical; the differences are the
schema's column types, connection setup, the lock that serialises writers, error mapping by
constraint name, and a repeatable-read transaction for the audit state. Feature modules are
**copied** from the SQLite crate with a fixed list of renames (Step 4). The files that differ in
more than names are given in full.

**Files:**
- Modify: `Cargo.toml` (workspace: add `smoke-storage-sqlite` path dependency for the parity test)
- Create: `crates/storage-postgres/Cargo.toml`, `sqlx.toml`, `migrations/0001_initial.sql`,
  `src/lib.rs`, `src/error.rs`, `src/tx.rs`, `src/convert.rs`, `src/users.rs`, `src/lists.rs`,
  `src/events.rs`, `src/reports.rs`, `src/readings.rs`, `src/jobs.rs`, `src/audit.rs`,
  `tests/common/mod.rs`, `tests/contract.rs`, `.sqlx/` (generated)
- Modify: `scripts/sqlx-prepare.sh` (Postgres section)
- Modify: `deny.toml` (rustls brings `webpki-roots`; expect `CDLA-Permissive-2.0`)

**Interfaces:**
- Consumes: all `smoke_domain` traits; the contract suite.
- Produces: `smoke_storage_postgres::PostgresStore` (`Clone`) — `connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self>`, `migrate(&self) -> StoreResult<()>`; implements `Store`. `smoke_storage_postgres::PostgresTx` implements `WriteTx`.

- [ ] **Step 1: Start a local Postgres**

```sh
docker run -d --name smoke-pg -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432 postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
```

- [ ] **Step 2: Write the failing tests**

Add to `[workspace.dependencies]` in the root `Cargo.toml`:

```toml
smoke-storage-sqlite = { path = "crates/storage-sqlite" }
```

`crates/storage-postgres/Cargo.toml`:

```toml
[package]
name = "smoke-storage-postgres"
version = "0.1.0"
description = "Postgres implementation of the Smoke Tracker storage traits (Neon-ready)"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]
chrono.workspace = true
serde_json.workspace = true
smoke-domain.workspace = true
sqlx = { workspace = true, features = ["postgres", "tls-rustls-ring-webpki"] }

[dev-dependencies]
smoke-storage-contract.workspace = true
smoke-storage-sqlite.workspace = true
tempfile.workspace = true
tokio.workspace = true

[lints]
workspace = true
```

`crates/storage-postgres/sqlx.toml`:

```toml
# The query macros check this crate against POSTGRES_DATABASE_URL (see storage-sqlite/sqlx.toml).
[common]
database-url-var = "POSTGRES_DATABASE_URL"
```

`crates/storage-postgres/tests/common/mod.rs`:

```rust
//! A fresh database per test on the server named by `TEST_POSTGRES_URL`.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::{AssertSqlSafe, PgPool};

static NEXT: AtomicU32 = AtomicU32::new(0);

/// The URL of a new, empty database, or `None` outside CI when no server is configured.
/// `TEST_POSTGRES_URL` must end with a database name (e.g. `/postgres`) and have no query string.
pub async fn fresh_database() -> Option<String> {
    let Ok(admin_url) = std::env::var("TEST_POSTGRES_URL") else {
        assert!(
            std::env::var_os("CI").is_none(),
            "TEST_POSTGRES_URL must be set in CI"
        );
        eprintln!("skipped: set TEST_POSTGRES_URL to run the Postgres tests");
        return None;
    };
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after 1970")
        .as_micros();
    let name = format!("smoke_test_{stamp}_{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let admin = PgPool::connect(&admin_url)
        .await
        .expect("connect to TEST_POSTGRES_URL");
    sqlx::raw_sql(AssertSqlSafe(format!("CREATE DATABASE {name}")))
        .execute(&admin)
        .await
        .expect("create a test database");
    admin.close().await;
    let (server, _) = admin_url
        .rsplit_once('/')
        .expect("TEST_POSTGRES_URL ends with /<database>");
    Some(format!("{server}/{name}"))
}
```

`crates/storage-postgres/tests/contract.rs`:

```rust
mod common;

use std::sync::Arc;

use smoke_domain::clock::FakeClock;
use smoke_storage_contract::fixture::{Fixture, raw_runner, start};
use smoke_storage_postgres::PostgresStore;
use sqlx::PgPool;

async fn fixture() -> Option<Fixture<PostgresStore>> {
    let url = common::fresh_database().await?;
    let clock = Arc::new(FakeClock::new(start()));
    let store = PostgresStore::connect(&url, clock.clone())
        .await
        .expect("connect");
    store.migrate().await.expect("migrate");
    let pool = PgPool::connect(&url).await.expect("raw connection");
    let raw = raw_runner(move |sql| {
        let pool = pool.clone();
        async move {
            sqlx::raw_sql(sql)
                .execute(&pool)
                .await
                .map(drop)
                .map_err(|e| e.to_string())
        }
    });
    let drop_guard = "DROP TRIGGER audit_log_append_only ON audit_log";
    Some(Fixture::new(store, clock, raw, drop_guard, ()))
}

smoke_storage_contract::contract_tests!(fixture);

#[test]
fn the_postgres_store_is_a_complete_store() {
    fn assert_store<S: smoke_domain::repos::Store>() {}
    assert_store::<PostgresStore>();
}
```

Run: `cargo test -p smoke-storage-postgres`
Expected: compile error — the crate has no `src/lib.rs`.

- [ ] **Step 3: Write the Postgres schema**

`crates/storage-postgres/migrations/0001_initial.sql`:

```sql
-- Instants are BIGINT microseconds since the Unix epoch, UTC (decision #46).
-- Kept in step with crates/storage-sqlite/migrations; tests/schema_parity.rs compares the columns.

CREATE TABLE users (
    id            BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    username      TEXT   NOT NULL CONSTRAINT users_username_unique UNIQUE,
    password_hash TEXT,
    role          TEXT   NOT NULL CHECK (role IN ('resident', 'guest', 'viewer')),
    locale        TEXT   NOT NULL CHECK (locale IN ('en', 'it', 'tr')),
    expires_at    BIGINT,
    created_at    BIGINT NOT NULL,
    CHECK (role <> 'resident' OR expires_at IS NULL),
    CHECK (role <> 'guest' OR expires_at IS NOT NULL)
);

CREATE TABLE invites (
    id         BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    user_id    BIGINT NOT NULL REFERENCES users (id),
    purpose    TEXT   NOT NULL CHECK (purpose IN ('setup', 'password_reset')),
    token_hash TEXT   NOT NULL CONSTRAINT invites_token_hash_unique UNIQUE,
    created_by BIGINT NOT NULL REFERENCES users (id),
    created_at BIGINT NOT NULL,
    expires_at BIGINT NOT NULL,
    used_at    BIGINT
);

CREATE TABLE rooms (
    id        BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    name      TEXT   NOT NULL,
    i18n_key  TEXT   CONSTRAINT rooms_i18n_key_unique UNIQUE,
    area      TEXT   NOT NULL CHECK (area IN ('inside', 'common')),
    hidden_at BIGINT
);

CREATE TABLE symptoms (
    id        BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    name      TEXT   NOT NULL,
    i18n_key  TEXT   CONSTRAINT symptoms_i18n_key_unique UNIQUE,
    hidden_at BIGINT
);

CREATE TABLE events (
    id                        BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    owner_id                  BIGINT NOT NULL REFERENCES users (id),
    opened_at                 BIGINT NOT NULL,
    closed_at                 BIGINT,
    close_reason              TEXT   CHECK (close_reason IN ('manual', 'new_event', 'auto_0400', 'backdated')),
    auto_close_notice_seen_at BIGINT,
    recorded_at               BIGINT NOT NULL,
    CHECK ((closed_at IS NULL) = (close_reason IS NULL)),
    CHECK (closed_at IS NULL OR closed_at >= opened_at)
);

-- At most one open event (decision #49).
CREATE UNIQUE INDEX events_one_open ON events ((closed_at IS NULL)) WHERE closed_at IS NULL;
CREATE INDEX events_opened_at ON events (opened_at);

CREATE TABLE smell_reports (
    id          BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    event_id    BIGINT NOT NULL REFERENCES events (id),
    user_id     BIGINT NOT NULL REFERENCES users (id),
    observed_at BIGINT NOT NULL,
    recorded_at BIGINT NOT NULL,
    intensity   BIGINT NOT NULL CHECK (intensity BETWEEN 1 AND 10),
    note        TEXT,
    deleted_at  BIGINT
);

CREATE INDEX smell_reports_event ON smell_reports (event_id, observed_at);

CREATE TABLE smell_report_rooms (
    smell_report_id BIGINT NOT NULL REFERENCES smell_reports (id),
    room_id         BIGINT NOT NULL REFERENCES rooms (id),
    PRIMARY KEY (smell_report_id, room_id)
);

CREATE TABLE smell_report_symptoms (
    smell_report_id BIGINT NOT NULL REFERENCES smell_reports (id),
    symptom_id      BIGINT NOT NULL REFERENCES symptoms (id),
    PRIMARY KEY (smell_report_id, symptom_id)
);

CREATE TABLE pm25_readings (
    id          BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    user_id     BIGINT NOT NULL REFERENCES users (id),
    event_id    BIGINT REFERENCES events (id),
    observed_at BIGINT NOT NULL,
    recorded_at BIGINT NOT NULL,
    value_ugm3  DOUBLE PRECISION NOT NULL CHECK (value_ugm3 >= 0 AND value_ugm3 <= 1000),
    deleted_at  BIGINT
);

CREATE INDEX pm25_readings_event ON pm25_readings (event_id, observed_at);

-- Append-only (decision #48). `id` is assigned by the application: it is part of the hash.
CREATE TABLE audit_log (
    id          BIGINT PRIMARY KEY,
    at          BIGINT NOT NULL,
    actor_id    BIGINT REFERENCES users (id),
    entity      TEXT   NOT NULL,
    entity_id   BIGINT NOT NULL,
    action      TEXT   NOT NULL,
    before_json TEXT,
    after_json  TEXT,
    prev_hash   TEXT   NOT NULL CONSTRAINT audit_log_prev_hash_unique UNIQUE,
    hash        TEXT   NOT NULL CONSTRAINT audit_log_hash_unique UNIQUE
);

CREATE INDEX audit_log_entity ON audit_log (entity, entity_id);

CREATE FUNCTION audit_log_append_only() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only';
END;
$$;

CREATE TRIGGER audit_log_append_only BEFORE UPDATE OR DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_log_append_only();

CREATE TABLE jobs (
    name        TEXT   NOT NULL PRIMARY KEY,
    last_run_at BIGINT NOT NULL
);
```

Every integer column is `BIGINT`, so every query decodes to the same Rust types as on SQLite
(`i64`, `Option<i64>`, `f64`, `String`); verified with a prototype while writing this plan.

- [ ] **Step 4: Implement the Postgres store**

`crates/storage-postgres/src/lib.rs`:

```rust
//! Postgres implementation of the `smoke-domain` storage traits, for a later move to Neon
//! (spec §10.2).
//!
//! Every write transaction first locks `audit_log` against other writers (plain reads are not
//! blocked), so transactions append to the hash chain one after another.

mod audit;
mod convert;
mod error;
mod events;
mod jobs;
mod lists;
mod readings;
mod reports;
mod tx;
mod users;

use std::sync::Arc;

use smoke_domain::clock::Clock;
use smoke_domain::repos::{StoreError, StoreResult, Transactional};
use smoke_domain::types::UserId;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres};

use crate::error::IntoStore;
pub use crate::tx::PostgresTx;

#[derive(Clone)]
pub struct PostgresStore {
    pool: PgPool,
    clock: Arc<dyn Clock>,
}

impl PostgresStore {
    /// Connects to `url`, e.g. `postgres://user@host/db?sslmode=require`.
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> StoreResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .db()?;
        Ok(Self { pool, clock })
    }

    /// Applies the embedded migrations. Safe to run on every start.
    pub async fn migrate(&self) -> StoreResult<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| StoreError::Backend(Box::new(e)))
    }

    async fn read(&self) -> StoreResult<PoolConnection<Postgres>> {
        self.pool.acquire().await.db()
    }
}

impl Transactional for PostgresStore {
    type Tx = PostgresTx;

    async fn begin(&self, actor: Option<UserId>) -> StoreResult<PostgresTx> {
        let mut tx = self.pool.begin().await.db()?;
        sqlx::query!("LOCK TABLE audit_log IN EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .db()?;
        Ok(PostgresTx::new(tx, actor, self.clock.clone()))
    }
}
```

`crates/storage-postgres/src/error.rs`:

```rust
//! Turns sqlx errors into the storage boundary's errors.

use smoke_domain::repos::{Conflict, StoreError, StoreResult};
use sqlx::error::ErrorKind;

pub(crate) trait IntoStore<T> {
    fn db(self) -> StoreResult<T>;
}

impl<T> IntoStore<T> for Result<T, sqlx::Error> {
    fn db(self) -> StoreResult<T> {
        self.map_err(into_store_error)
    }
}

fn into_store_error(error: sqlx::Error) -> StoreError {
    if let sqlx::Error::Database(db) = &error {
        match db.kind() {
            ErrorKind::UniqueViolation => return unique_conflict(db.constraint()).into(),
            ErrorKind::ForeignKeyViolation => return Conflict::UnknownReference.into(),
            _ => {}
        }
    }
    StoreError::Backend(Box::new(error))
}

/// Postgres reports the violated constraint or index by name; the migration names them.
fn unique_conflict(constraint: Option<&str>) -> Conflict {
    match constraint {
        Some("users_username_unique") => Conflict::UsernameTaken,
        Some("events_one_open") => Conflict::AnotherEventOpen,
        Some("audit_log_prev_hash_unique" | "audit_log_hash_unique") => Conflict::ChainForked,
        _ => Conflict::Duplicate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_violations_map_to_their_conflict() {
        assert_eq!(unique_conflict(Some("users_username_unique")), Conflict::UsernameTaken);
        assert_eq!(unique_conflict(Some("events_one_open")), Conflict::AnotherEventOpen);
        assert_eq!(unique_conflict(Some("audit_log_prev_hash_unique")), Conflict::ChainForked);
        assert_eq!(unique_conflict(Some("rooms_i18n_key_unique")), Conflict::Duplicate);
        assert_eq!(unique_conflict(None), Conflict::Duplicate);
    }
}
```

(`constraint()` returning `events_one_open` for the partial index was verified against
Postgres 17 while writing this plan.)

`crates/storage-postgres/src/tx.rs`:

```rust
//! The write transaction, and the audit entry every write appends to it.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde_json::Value;
use smoke_domain::audit::{AuditAction, AuditRecord, ChainedEntry, GENESIS_HASH};
use smoke_domain::clock::Clock;
use smoke_domain::records::Audited;
use smoke_domain::repos::{Commit, StoreResult};
use smoke_domain::types::UserId;
use sqlx::{PgConnection, Postgres, Transaction};

use crate::error::IntoStore;

pub struct PostgresTx {
    tx: Transaction<'static, Postgres>,
    actor: Option<UserId>,
    clock: Arc<dyn Clock>,
}

impl PostgresTx {
    pub(crate) fn new(
        tx: Transaction<'static, Postgres>,
        actor: Option<UserId>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self { tx, actor, clock }
    }

    pub(crate) fn conn(&mut self) -> &mut PgConnection {
        &mut self.tx
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    /// Appends the audit entry for one write, linked to the latest entry. An `update` that
    /// changed nothing is not recorded.
    pub(crate) async fn record<T: Audited>(
        &mut self,
        action: AuditAction,
        before: Option<&T>,
        after: &T,
    ) -> StoreResult<()> {
        let before = before.map(Audited::snapshot);
        let after_value = after.snapshot();
        if action == AuditAction::Update && before.as_ref() == Some(&after_value) {
            return Ok(());
        }
        let head = sqlx::query!("SELECT id, hash FROM audit_log ORDER BY id DESC LIMIT 1")
            .fetch_optional(self.conn())
            .await
            .db()?;
        let (id, prev_hash) = head.map_or((1, GENESIS_HASH.to_owned()), |h| (h.id + 1, h.hash));
        let entry = ChainedEntry::seal(
            AuditRecord {
                id,
                at_micros: self.now().timestamp_micros(),
                actor_id: self.actor,
                entity: T::ENTITY,
                entity_id: after.audit_id(),
                action,
                before,
                after: Some(after_value),
            },
            &prev_hash,
        );
        let record = &entry.record;
        sqlx::query!(
            "INSERT INTO audit_log \
             (id, at, actor_id, entity, entity_id, action, before_json, after_json, prev_hash, hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            record.id,
            record.at_micros,
            record.actor_id.map(|a| a.0),
            record.entity.as_str(),
            record.entity_id,
            record.action.as_str(),
            record.before.as_ref().map(Value::to_string),
            record.after.as_ref().map(Value::to_string),
            entry.prev_hash,
            entry.hash,
        )
        .execute(&mut *self.tx)
        .await
        .db()?;
        Ok(())
    }
}

impl Commit for PostgresTx {
    async fn commit(self) -> StoreResult<()> {
        self.tx.commit().await.db()
    }
}
```

**Copied modules.** Copy these files from `crates/storage-sqlite/src/` to
`crates/storage-postgres/src/`: `convert.rs`, `users.rs`, `lists.rs`, `events.rs`, `reports.rs`,
`readings.rs`, `jobs.rs`, `audit.rs`. In each copy apply exactly these renames (whole words):

| SQLite crate | Postgres crate |
|---|---|
| `SqliteConnection` | `PgConnection` |
| `SqliteStore` | `PostgresStore` |
| `SqliteTx` | `PostgresTx` |

Then make one more change, in `audit.rs`'s `audit_state` — Postgres needs the isolation level set
explicitly to read one consistent snapshot. Replace

```rust
        let mut tx = self.reader.begin().await.db()?;
```

with

```rust
        let mut tx = self.pool.begin().await.db()?;
        sqlx::query!("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .execute(&mut *tx)
            .await
            .db()?;
```

and in its doc comment, keep "One read transaction, …" as is. Nothing else changes: no SQL text,
no type. After copying, `grep -rn -i sqlite crates/storage-postgres/src` must print nothing.

- [ ] **Step 5: Generate the Postgres query metadata**

Append to `scripts/sqlx-prepare.sh`:

```bash

cd "$root/crates/storage-postgres"
admin_url="${TEST_POSTGRES_URL:?set TEST_POSTGRES_URL to a Postgres server where databases can be created}"
export POSTGRES_DATABASE_URL="${admin_url%/*}/smoke_prepare_$$"
export DATABASE_URL="$POSTGRES_DATABASE_URL"
trap 'rm -rf "$work"; (cd "$root/crates/storage-postgres" && sqlx database drop -y >/dev/null 2>&1) || true' EXIT
sqlx database create
sqlx migrate run
prepare
```

and update the header comment's last line to:
`# Needs sqlx-cli 0.9 (see README) and TEST_POSTGRES_URL.`

Run: `scripts/sqlx-prepare.sh`
Expected: `crates/storage-postgres/.sqlx/` is created; `crates/storage-sqlite/.sqlx/` is unchanged
(`git status` shows no change there).

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p smoke-storage-postgres && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check`
Expected: the 41 contract tests pass on Postgres, plus the completeness test and the error-mapping
unit test. Then run once with `TEST_POSTGRES_URL` unset: the Postgres contract tests print
`skipped` and pass. Add `CDLA-Permissive-2.0` (from `webpki-roots`) to `deny.toml`'s `allow`
list if `cargo deny check licenses` asks for it; any other new licence: permissive OSI only,
copyleft → stop and ask the user.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock deny.toml scripts/sqlx-prepare.sh crates/storage-postgres
git commit -m "feat(storage): Postgres store passing the same contract suite

Same SQL text as SQLite; writers serialise on LOCK TABLE audit_log IN
EXCLUSIVE MODE, and the audit state is read at REPEATABLE READ. Unique
violations map to conflicts by constraint name.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 13: Schema parity, CI, and documentation

**Files:**
- Create: `crates/storage-postgres/tests/schema_parity.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `.github/rulesets/protect-main.json`
- Modify: `README.md` (Development section)
- Modify: `docs/plans/ROADMAP.md`

**Interfaces:**
- Consumes: `SqliteStore::{connect, migrate}`, `PostgresStore::{connect, migrate}`,
  `tests/common::fresh_database` (Task 12).
- Produces: CI job names `fmt · clippy · test` (unchanged) and `sqlx offline data` (new, required).

- [ ] **Step 1: Write the parity test**

`crates/storage-postgres/tests/schema_parity.rs`:

```rust
//! The SQLite and Postgres migrations must produce the same tables and columns (spec §10.1).

mod common;

use std::collections::BTreeSet;
use std::sync::Arc;

use smoke_domain::clock::SystemClock;
use smoke_storage_postgres::PostgresStore;
use smoke_storage_sqlite::SqliteStore;
use sqlx::{PgPool, SqlitePool};

/// (table, column, type family, nullable)
type Columns = BTreeSet<(String, String, &'static str, bool)>;

fn family(sql_type: &str) -> &'static str {
    match sql_type.to_ascii_lowercase().as_str() {
        "integer" | "bigint" => "integer",
        "text" => "text",
        "real" | "double precision" => "real",
        other => panic!("no parity rule for column type {other:?}"),
    }
}

async fn sqlite_columns() -> Columns {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}", dir.path().join("parity.db").display());
    let store = SqliteStore::connect(&url, Arc::new(SystemClock)).await.unwrap();
    store.migrate().await.unwrap();
    let pool = SqlitePool::connect(&url).await.unwrap();
    let rows: Vec<(String, String, String, i64, i64)> = sqlx::query_as(
        "SELECT m.name, p.name, p.type, p.\"notnull\", p.pk \
         FROM sqlite_master m JOIN pragma_table_info(m.name) p \
         WHERE m.type = 'table' AND m.name NOT LIKE 'sqlite_%' AND m.name <> '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(table, column, ty, not_null, pk)| {
            (table, column, family(&ty), not_null == 0 && pk == 0)
        })
        .collect()
}

async fn postgres_columns(url: &str) -> Columns {
    let store = PostgresStore::connect(url, Arc::new(SystemClock)).await.unwrap();
    store.migrate().await.unwrap();
    let pool = PgPool::connect(url).await.unwrap();
    let rows: Vec<(String, String, String, bool)> = sqlx::query_as(
        "SELECT table_name::text, column_name::text, data_type::text, is_nullable = 'YES' \
         FROM information_schema.columns \
         WHERE table_schema = current_schema() AND table_name <> '_sqlx_migrations'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    rows.into_iter()
        .map(|(table, column, ty, nullable)| (table, column, family(&ty), nullable))
        .collect()
}

#[tokio::test]
async fn sqlite_and_postgres_schemas_have_the_same_columns() {
    let Some(url) = common::fresh_database().await else {
        return;
    };
    let sqlite = sqlite_columns().await;
    let postgres = postgres_columns(&url).await;
    let only_sqlite: Vec<_> = sqlite.difference(&postgres).collect();
    let only_postgres: Vec<_> = postgres.difference(&sqlite).collect();
    assert!(
        only_sqlite.is_empty() && only_postgres.is_empty(),
        "only in SQLite: {only_sqlite:#?}\nonly in Postgres: {only_postgres:#?}"
    );
    assert_eq!(sqlite.len(), 62, "a table went missing from the comparison");
}
```

(62 = users 7 + invites 8 + rooms 5 + symptoms 4 + events 7 + smell_reports 8 + the two link
tables 2 + 2 + pm25_readings 7 + audit_log 10 + jobs 2. SQLite reports `INTEGER PRIMARY KEY`
columns with `notnull = 0`, hence the `pk` check.)

- [ ] **Step 2: Run it to verify it passes, then prove it can fail**

Run: `cargo test -p smoke-storage-postgres --test schema_parity`
Expected: PASS. Then temporarily delete the `note TEXT,` line from the Postgres migration's
`smell_reports`, re-run, and confirm it FAILS listing `("smell_reports", "note", "text", true)`
under "only in SQLite". Restore the line.

- [ ] **Step 3: CI — Postgres service and the metadata check**

Replace `.github/workflows/ci.yml` with:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: -D warnings
  TEST_POSTGRES_URL: postgres://postgres@localhost:5432/postgres

jobs:
  check:
    name: fmt · clippy · test
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_HOST_AUTH_METHOD: trust
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 5s
          --health-retries 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked
      - run: cargo test --workspace --locked
      - name: audit hash ignores serde_json map ordering
        run: cargo test -p smoke-domain --lib --features serde_json/preserve_order audit

  sqlx:
    name: sqlx offline data
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:17-alpine
        env:
          POSTGRES_HOST_AUTH_METHOD: trust
        ports: ["5432:5432"]
        options: >-
          --health-cmd pg_isready
          --health-interval 5s
          --health-timeout 5s
          --health-retries 10
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: install sqlx-cli
        env:
          RUSTFLAGS: ""
        run: cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite,postgres,rustls,sqlx-toml
      - name: committed query metadata matches the queries and migrations
        run: scripts/sqlx-prepare.sh --check
```

(`RUSTFLAGS: ""` on the install step: sqlx-cli's own dependencies must not fail on our
`-D warnings`. `CI` is set by GitHub, so a missing `TEST_POSTGRES_URL` would fail the tests
instead of skipping them.)

- [ ] **Step 4: Require the new check**

In `.github/rulesets/protect-main.json`, add to `required_status_checks`:

```json
          { "context": "sqlx offline data", "integration_id": 15368 },
```

(after the `fmt · clippy · test` entry). **Manual step for the user after merging:** re-import
the ruleset in GitHub → Settings → Rules → Rulesets (or edit the existing ruleset and add the
check), as the repo's ruleset is not applied from the file automatically.

- [ ] **Step 5: Documentation**

In `README.md`, replace the `## Development` section with:

~~~markdown
## Development

Requirements: [rustup](https://rustup.rs), [gitleaks](https://github.com/gitleaks/gitleaks)
(`brew install gitleaks`), Docker, and sqlx-cli:

```sh
git config core.hooksPath .githooks   # once per clone: blocks commits containing secrets
cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite,postgres,rustls,sqlx-toml

# A throwaway local Postgres for the Postgres tests (no password, local only)
docker run -d --name smoke-pg -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432 postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres

cargo test --workspace         # Postgres tests are skipped when TEST_POSTGRES_URL is unset
scripts/sqlx-prepare.sh        # after changing a query or a migration; commit the .sqlx/ changes
```

Building needs no database: queries are checked against the committed `.sqlx/` metadata.
~~~

In `docs/plans/ROADMAP.md`:
- In row 2, set the **Delivers** cell to: `Repository traits in domain; storage-sqlite and storage-postgres crates with migrations; an audit entry inside every write transaction; one storage-contract test suite run on both backends; schema-parity check; verify-audit logic`, and the **Plan** cell to `` `02-storage.md` ✅ ``.
- Replace the paragraph starting `**After milestone 1:**` with:
  `**After milestone 1 (done):** GitHub repo created, branch protection (ruleset) and push protection enabled, push-reminder issue created (#3).`
- Add a paragraph after it: `**Future work:** push reminders — [issue #3](https://github.com/locadani/smoke-tracker/issues/3).`

- [ ] **Step 6: Run everything**

Run: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check && cargo deny check && scripts/sqlx-prepare.sh --check`
Expected: all pass; `--check` reports the metadata is up to date.

- [ ] **Step 7: Commit**

```bash
git add crates/storage-postgres/tests/schema_parity.rs .github README.md docs/plans/ROADMAP.md
git commit -m "ci: Postgres service, schema-parity test and an sqlx offline-metadata check

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 14: Spec wording — **only with the user's explicit approval**

Do not start this task until the user has approved these exact edits in chat. If they approve only
some, apply only those. If they decline, skip the task entirely.

**Files:**
- Modify: `docs/specs/2026-10-13-smoke-tracker-design.md`

- [ ] **Step 1: Apply the approved edits**

1. §6 data model, the `audit_log` line (`prev_hash, hash          -- SHA-256(canonical(entry) || prev_hash)`) becomes:
   `  prev_hash, hash          -- SHA-256(prev_hash || "\n" || canonical(entry))`
   (the code from milestone 1 hashes in this order; decision log rows are unaffected).
2. §6.1, the bullet starting `Each entry's \`hash\` = SHA-256 of a **canonical serialisation**` becomes:
   ``- Each entry's `hash` = SHA-256 of the previous entry's hash, a newline, and a **canonical serialisation** of the entry (independent of the database backend).``
3. §11, `**Expiry** is checked on every request (\`expires_at < now\`); no job is needed.` becomes:
   ``- **Expiry** is checked on every request (`expires_at <= now`: an account is expired from that exact instant); no job is needed.``
4. §14, `**Push reminders.** Tracked as a GitHub issue (text below).` becomes:
   `**Push reminders.** Tracked as GitHub issue #3 (text below).`
5. §10 architecture tree, the `storage/` block becomes:
   ```
   │  ├─ storage-sqlite/    SqliteStore, migrations/, .sqlx/ (decision #45)
   │  ├─ storage-postgres/  PostgresStore, migrations/, .sqlx/, schema-parity test
   │  ├─ storage-contract/  one test suite run against both backends
   ```
   and the dependency line becomes: ``**Dependency direction:** `app → web → domain ← storage-*` (`app` also wires a storage crate).``
6. §10.1, add a bullet: `- Instants are stored as integer microseconds since the Unix epoch in both backends (decision #46).`
7. §13.3, the storage row's "How" cell `contract tests vs SQLite (in-memory) and Postgres` becomes
   `contract tests vs SQLite (temporary file) and Postgres`.

- [ ] **Step 2: Commit**

```bash
git add docs/specs/2026-10-13-smoke-tracker-design.md
git commit -m "docs(spec): align hash order, expiry instant and storage layout with the code

Approved by the user on <date>.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(Replace `<date>` with the approval date.)

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §6 | Every table, UTC instants, soft deletes, hidden list items | 6 (schema), 7–10 |
| §6 | Starter rooms and symptoms | 5, 7 |
| §6.1 | One transaction changes data and appends the audit entry | 6 (`record`), 6–11 |
| §6.1 | Canonical, backend-independent hash | M1 + 6 (`TEXT` JSON, integer instants) |
| §6.1 | `verify-audit`: chain + current record = latest `after_json` | 5, 11 |
| §7 | At most one open event | 8 (index + `AnotherEventOpen`) |
| §7.3–7.5 | Close / reopen / move end / auto-close notice | 8 |
| §7.6–7.7 | PM2.5 linking, corrections, moves | 9, 10 |
| §10.1 | Traits in `domain`, two backends, compile-time-checked queries, offline metadata | 4, 6, 12 |
| §10.1 | Contract tests on SQLite and a Postgres service in CI | 6–13 |
| §10.1 | Schema parity check in CI | 13 |
| §11 | `jobs.last_run_at` | 10 |
| §12 | `thiserror` errors in library crates | 4, 6 |
| §13.3 | Storage: migrations, queries, audit entry per write, soft deletes | 6–11 |

Not in this milestone (by the roadmap): the `verify-audit` CLI command and session store
(milestone 3), the auto-close sweep and logging flow (milestone 4), invites (milestone 5), and
`migrate-to-postgres` (milestone 7). The `invites` table exists now so both schemas stay complete.
