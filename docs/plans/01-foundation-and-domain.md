# Milestone 1: Foundation & Domain Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Cargo workspace with secret protection and CI, plus a fully tested, I/O-free `smoke-domain` crate containing every business rule that doesn't need a database.

**Architecture:** The workspace lives at the repo root, with crates under `crates/`. This
milestone creates only `crates/domain` (package `smoke-domain`). Storage, web and app crates
arrive in later milestones (see `docs/plans/ROADMAP.md`). The domain crate is pure: no
database, no HTTP, no reading the system clock directly. Time comes in through a `Clock` trait
or as function arguments, which makes every rule deterministic to test.

**Tech Stack:** Rust (edition 2024, toolchain via `rust-toolchain.toml`), chrono + chrono-tz,
serde + serde_json, sha2 + hex, thiserror, proptest (tests), GitHub Actions, gitleaks,
cargo-deny, Dependabot.

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md`. Decisions: `docs/DECISIONS.md`.

## Global Constraints

- Personal project. Before every commit, `git config user.email` must print
  `61902944+locadani@users.noreply.github.com` and `git config user.name` must print `locadani`.
  Never put any email address in a tracked file.
- Never commit secrets. `.env` and database files are git-ignored. The gitleaks pre-commit hook
  (Task 2) must be active for every commit after Task 2.
- Never modify `description.md` or `claude.md`. Never `git add` them.
- Every commit message ends with a blank line followed by
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one, e.g.
  `feat(domain): Intensity newtype rejects values outside 1–10`.
- No tutorial-style comments in code. Doc comments explain *what* and *why*; teaching material
  goes in `docs/learning/` in milestone 8.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI
  (`RUSTFLAGS=-D warnings`), `cargo fmt` enforced.
- Time zone default `Europe/Rome`, auto-close hour default `4`. All instants are
  `chrono::DateTime<Utc>`.
- Intensity range 1–10. PM2.5 range 0–1000 µg/m³, finite.
- Backdate threshold 10 minutes; future tolerance 1 minute; large-gap warning above 3 hours.
- Audit hash = SHA-256 over `prev_hash`, `"\n"`, then canonical JSON (sorted keys) of the record.
  Timestamps in the hash are integer **microseconds** since the Unix epoch. The genesis
  `prev_hash` is 64 `0` characters.

## Review Focus

1. **A phone clock running ahead, or a typo, gives an observed time in the future.** Beyond 1
   minute ahead it must be rejected with `DomainError::ObservedInFuture`, not silently accepted
   (test in Task 8).
2. **DST days and DST-edge auto-close hours.** On 2026-03-29 and 2026-10-25 the 04:00 boundary
   must still be exactly 04:00 local. If `AUTO_CLOSE_HOUR` is ever set to a non-existent or
   ambiguous local hour (02:00 in Rome), the boundary must still resolve, not panic (tests in
   Task 7).
3. **A future dependency enables serde_json's `preserve_order` feature.** Cargo unifies
   features, so object key order would silently change and every stored hash would stop
   verifying. A test must pin key-order independence (Task 9).
4. **Timestamp precision across databases.** Postgres keeps microseconds; SQLite text can keep
   nanoseconds. Hashing at nanosecond precision would break the chain after a move to Neon. A
   test must show sub-microsecond differences don't change the hash (Task 9).
5. **Expiry at the exact instant.** `expires_at == now` must count as expired, so a guest can't
   log one extra request (test in Task 6).

---

## File Structure

```
smoke-tracker/
├─ Cargo.toml                 workspace manifest, shared deps and lints
├─ Cargo.lock                 committed (this is an application)
├─ rust-toolchain.toml        stable + rustfmt + clippy
├─ rustfmt.toml
├─ .gitignore
├─ .gitleaks.toml             gitleaks config (default rules)
├─ .githooks/pre-commit       blocks commits containing secrets
├─ deny.toml                  cargo-deny policy
├─ LICENSE                    MIT
├─ README.md                  minimal for now; full README in milestone 6
├─ .github/
│  ├─ dependabot.yml
│  └─ workflows/
│     ├─ ci.yml               fmt, clippy, test
│     └─ security.yml         cargo-deny, gitleaks
└─ crates/domain/
   ├─ Cargo.toml
   └─ src/
      ├─ lib.rs               module list + re-exports
      ├─ error.rs             DomainError
      ├─ types.rs             ID newtypes, Intensity, Pm25, str_enum! enums
      ├─ clock.rs             Clock trait, SystemClock, FakeClock
      ├─ permissions.rs       Actor, Action, can()
      ├─ schedule.rs          DailySchedule (the 04:00 boundary)
      ├─ events.rs            event lifecycle rules
      └─ audit.rs             hash chain
```

Tests live in `#[cfg(test)] mod tests` at the bottom of each source file (the Rust convention
for unit tests).

---

### Task 1: Workspace scaffold

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `.gitignore`, `LICENSE`, `README.md`
- Create: `crates/domain/Cargo.toml`, `crates/domain/src/lib.rs`

**Interfaces:**
- Consumes: nothing
- Produces: workspace dependency names (`chrono`, `chrono-tz`, `serde`, `serde_json`, `sha2`,
  `hex`, `thiserror`, `proptest`) that member crates reference with `.workspace = true`; package
  `smoke-domain`.

- [ ] **Step 1: Verify the git identity**

Run: `git config user.name && git config user.email`
Expected:
```
locadani
61902944+locadani@users.noreply.github.com
```
If this is different, STOP and ask the user. Do not change it yourself.

- [ ] **Step 2: Create `Cargo.toml` (workspace root)**

```toml
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
edition = "2024"
rust-version = "1.85"
license = "MIT"
publish = false
repository = "https://github.com/locadani/smoke-tracker"

[workspace.dependencies]
chrono = { version = "0.4", default-features = false, features = ["clock", "std", "serde"] }
chrono-tz = "0.10"
hex = "0.4"
proptest = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = "0.10"
thiserror = "2"

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
```

- [ ] **Step 3: Create `rust-toolchain.toml` and `rustfmt.toml`**

`rust-toolchain.toml`:
```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
```

`rustfmt.toml`:
```toml
edition = "2024"
```

- [ ] **Step 4: Create `.gitignore`**

```gitignore
# Build output
/target

# Secrets and local config — never commit
.env
.env.*
!.env.example

# Local databases
*.db
*.db-shm
*.db-wal
/data/

# OS / editor
.DS_Store
.idea/
.vscode/
```

- [ ] **Step 5: Create `LICENSE` (MIT)**

```text
MIT License

Copyright (c) 2026 locadani

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 6: Create a minimal `README.md`**

```markdown
# Smoke Tracker

A small, installable web app (PWA) for logging cigarette smoke that drifts into an apartment
from neighbours. It records how strong the smell is, which rooms it reached, any symptoms, and
PM2.5 readings, and builds a detailed, tamper-evident record over time.

Written in Rust (Axum, Askama, htmx, SQLite).

> **Status:** early development. See the [roadmap](docs/plans/ROADMAP.md).

## Development

Requirements: [rustup](https://rustup.rs) and [gitleaks](https://github.com/gitleaks/gitleaks)
(`brew install gitleaks`).

```sh
git config core.hooksPath .githooks   # once per clone: blocks commits containing secrets
cargo test --workspace
```

## Documentation

- [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
- [Decision log](docs/DECISIONS.md)
```

- [ ] **Step 7: Create `crates/domain/Cargo.toml`**

```toml
[package]
name = "smoke-domain"
version = "0.1.0"
description = "Pure domain logic for Smoke Tracker: types, rules and the audit chain"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish.workspace = true
repository.workspace = true

[dependencies]

[dev-dependencies]

[lints]
workspace = true
```

- [ ] **Step 8: Create `crates/domain/src/lib.rs`**

```rust
//! Pure domain logic for Smoke Tracker: types, business rules and the audit chain.
//!
//! This crate performs no I/O. Time is passed in explicitly or through
//! [`clock::Clock`], so every rule is deterministic and fast to test.
```

- [ ] **Step 9: Verify the workspace builds, formats and lints cleanly**

Run: `cargo build --workspace && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`
Expected: all succeed. `cargo test` reports `0 passed` for `smoke-domain`. A `Cargo.lock` file
now exists.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml rustfmt.toml .gitignore LICENSE README.md crates/
git status --short   # must NOT list description.md or claude.md as staged
git commit -m "chore: cargo workspace with smoke-domain crate

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Secret protection (gitleaks pre-commit hook)

**Files:**
- Create: `.gitleaks.toml`, `.githooks/pre-commit`

**Interfaces:**
- Consumes: nothing
- Produces: an active hook (`core.hooksPath=.githooks`) that every later commit passes through.

- [ ] **Step 1: Install gitleaks**

Run: `brew install gitleaks && gitleaks version`
Expected: a version `8.19.0` or newer. The `gitleaks git` subcommand used below needs ≥ 8.19.

- [ ] **Step 2: Create `.gitleaks.toml`**

```toml
title = "smoke-tracker"

[extend]
useDefault = true
```

- [ ] **Step 3: Create `.githooks/pre-commit`**

```sh
#!/bin/sh
# Blocks any commit whose staged changes contain a secret.
# Enable once per clone with: git config core.hooksPath .githooks
set -eu

if ! command -v gitleaks >/dev/null 2>&1; then
    echo "pre-commit: gitleaks is not installed (brew install gitleaks); commit blocked." >&2
    exit 1
fi

exec gitleaks git --pre-commit --staged --redact --no-banner --config .gitleaks.toml
```

Run: `chmod +x .githooks/pre-commit && git config core.hooksPath .githooks`

- [ ] **Step 4: Prove the hook blocks a secret**

The fake token is generated at runtime, so no token-shaped string ever appears in a tracked file:

```bash
printf 'token = "ghp_%s"\n' "$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 36)" > leak-test.txt
git add leak-test.txt
git commit -m "should be blocked"; echo "exit=$?"
```
Expected: gitleaks prints `leaks found: 1` (or similar), `exit=1`, and no commit is created.
Check with `git log --oneline -1`: the last commit is still Task 1's.

- [ ] **Step 5: Clean up the test file**

```bash
git rm --cached -q leak-test.txt && rm leak-test.txt
git status --short   # leak-test.txt must not appear
```

- [ ] **Step 6: Commit**

```bash
git add .gitleaks.toml .githooks/pre-commit
git commit -m "chore(security): gitleaks pre-commit hook blocks secrets before they are committed

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
Expected: the hook runs, prints `no leaks found`, and the commit succeeds.

---

### Task 3: CI and security workflows

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/security.yml`, `.github/dependabot.yml`, `deny.toml`

**Interfaces:**
- Consumes: the workspace from Task 1, `.gitleaks.toml` from Task 2.
- Produces: GitHub Actions jobs named `fmt · clippy · test`, `cargo-deny` and `gitleaks`. Later
  milestones add jobs to these files.

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

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

jobs:
  check:
    name: fmt · clippy · test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --all --check
      - run: cargo clippy --workspace --all-targets --locked
      - run: cargo test --workspace --locked
```

- [ ] **Step 2: Create `.github/workflows/security.yml`**

```yaml
name: Security

on:
  push:
    branches: [main]
  pull_request:
  schedule:
    - cron: "0 6 * * 1" # Mondays 06:00 UTC: catches advisories published after the last push

permissions:
  contents: read

jobs:
  cargo-deny:
    name: cargo-deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2

  gitleaks:
    name: gitleaks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: gitleaks/gitleaks-action@v2
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

- [ ] **Step 3: Create `.github/dependabot.yml`**

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
    groups:
      rust-dependencies:
        patterns: ["*"]
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

- [ ] **Step 4: Create `deny.toml`**

```toml
[graph]
all-features = true

[advisories]
yanked = "deny"

[licenses]
confidence-threshold = 0.8
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"
allow-wildcard-paths = true

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

- [ ] **Step 5: Run cargo-deny and actionlint locally**

Run: `brew install cargo-deny actionlint && cargo deny check && actionlint`
Expected: `cargo deny check` prints `advisories ok, bans ok, licenses ok, sources ok`, and
`actionlint` prints nothing.

If cargo-deny rejects a licence that isn't in the list, check on crates.io that it is a
permissive OSI licence before adding it to `allow`. Never add copyleft licences (GPL, AGPL)
without asking the user.

- [ ] **Step 6: Commit**

```bash
git add .github/ deny.toml
git commit -m "ci: fmt/clippy/test pipeline, cargo-deny + gitleaks security workflow, dependabot

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

The workflows first run when the repo is pushed to GitHub, which is a user step after this
milestone.

---

### Task 4: Domain errors and core types

**Files:**
- Modify: `crates/domain/Cargo.toml` (dependencies)
- Modify: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/error.rs`, `crates/domain/src/types.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `smoke_domain::DomainError` (enum; variants listed in Step 3)
  - ID newtypes `UserId`, `EventId`, `ReportId`, `ReadingId`, `RoomId`, `SymptomId`, each
    `pub struct X(pub i64)`, `Copy + Ord + Hash + Serialize(transparent)`
  - `Intensity` with `Intensity::new(u8) -> Result<Intensity, DomainError>` and `.get() -> u8`
    (serde: integer)
  - `Pm25` with `Pm25::new(f64) -> Result<Pm25, DomainError>` and `.ugm3() -> f64`
    (serde: number)
  - `Role { Resident, Guest, Viewer }`, `RoomArea { Inside, Common }`,
    `CloseReason { Manual, NewEvent, Auto0400, Backdated }`, `Locale { En, It, Tr }`. Each has
    `.as_str()`, `FromStr`, `Display`, an `ALL` slice, and serde as its string form.
  - Macro `str_enum!` defined in `types.rs`. Task 9 re-exports it as `crate::types::str_enum`.

- [ ] **Step 1: Add dependencies to `crates/domain/Cargo.toml`**

Replace the empty `[dependencies]` and `[dev-dependencies]` sections with:
```toml
[dependencies]
serde.workspace = true
thiserror.workspace = true

[dev-dependencies]
serde_json.workspace = true
```

- [ ] **Step 2: Write the failing tests: create `crates/domain/src/types.rs` with tests only**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::DomainError;

    #[test]
    fn intensity_accepts_the_full_range() {
        for value in 1..=10 {
            assert_eq!(Intensity::new(value).unwrap().get(), value);
        }
    }

    #[test]
    fn intensity_rejects_values_outside_one_to_ten() {
        assert_eq!(Intensity::new(0), Err(DomainError::InvalidIntensity(0)));
        assert_eq!(Intensity::new(11), Err(DomainError::InvalidIntensity(11)));
    }

    #[test]
    fn intensity_cannot_be_deserialised_out_of_range() {
        assert!(serde_json::from_str::<Intensity>("11").is_err());
        assert_eq!(serde_json::from_str::<Intensity>("7").unwrap().get(), 7);
        assert_eq!(serde_json::to_string(&Intensity::new(7).unwrap()).unwrap(), "7");
    }

    #[test]
    fn pm25_accepts_zero_to_one_thousand() {
        assert_eq!(Pm25::new(0.0).unwrap().ugm3(), 0.0);
        assert_eq!(Pm25::new(42.5).unwrap().ugm3(), 42.5);
        assert_eq!(Pm25::new(1000.0).unwrap().ugm3(), 1000.0);
    }

    #[test]
    fn pm25_rejects_negative_too_large_and_non_finite() {
        for bad in [-0.1, 1000.1, f64::NAN, f64::INFINITY] {
            assert!(Pm25::new(bad).is_err(), "{bad} should be rejected");
        }
        assert!(serde_json::from_str::<Pm25>("-5").is_err());
    }

    #[test]
    fn string_enums_round_trip_through_text_and_serde() {
        for role in Role::ALL {
            assert_eq!(role.as_str().parse::<Role>().unwrap(), *role);
            let json = serde_json::to_string(role).unwrap();
            assert_eq!(serde_json::from_str::<Role>(&json).unwrap(), *role);
        }
        for reason in CloseReason::ALL {
            assert_eq!(reason.as_str().parse::<CloseReason>().unwrap(), *reason);
        }
        for area in RoomArea::ALL {
            assert_eq!(area.as_str().parse::<RoomArea>().unwrap(), *area);
        }
        for locale in Locale::ALL {
            assert_eq!(locale.as_str().parse::<Locale>().unwrap(), *locale);
        }
    }

    #[test]
    fn string_enums_use_the_stored_spelling() {
        assert_eq!(CloseReason::Auto0400.as_str(), "auto_0400");
        assert_eq!(serde_json::to_string(&CloseReason::NewEvent).unwrap(), "\"new_event\"");
        assert_eq!(Role::Guest.to_string(), "guest");
    }

    #[test]
    fn unknown_strings_are_rejected_with_the_type_name() {
        assert_eq!(
            "admin".parse::<Role>(),
            Err(DomainError::UnknownValue { kind: "Role", value: "admin".into() })
        );
        assert!(serde_json::from_str::<Role>("\"admin\"").is_err());
    }

    #[test]
    fn ids_serialise_as_plain_numbers() {
        assert_eq!(serde_json::to_string(&UserId(7)).unwrap(), "7");
        assert_eq!(EventId(3).to_string(), "3");
    }
}
```

Add the modules to `crates/domain/src/lib.rs` (below the existing doc comment):
```rust
pub mod error;
pub mod types;

pub use error::DomainError;
```

Create `crates/domain/src/error.rs` as an empty file for now.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain`
Expected: compilation FAILS, with errors like `cannot find type 'Intensity' in this scope` and
`unresolved import 'crate::DomainError'`.

- [ ] **Step 4: Implement `crates/domain/src/error.rs`**

```rust
use thiserror::Error;

/// Every way a domain rule can reject input.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DomainError {
    #[error("intensity must be between 1 and 10, got {0}")]
    InvalidIntensity(u8),

    #[error("PM2.5 must be a number between 0 and 1000 µg/m³, got {0}")]
    InvalidPm25(f64),

    #[error("unknown {kind}: {value:?}")]
    UnknownValue { kind: &'static str, value: String },

    #[error("hour must be between 0 and 23, got {0}")]
    InvalidHour(u32),

    #[error("the observed time is in the future")]
    ObservedInFuture,

    #[error("an event cannot close before its last report")]
    CloseBeforeLastReport,

    #[error("the close time is in the future")]
    CloseInFuture,
}
```

- [ ] **Step 5: Implement the types: put this at the top of `crates/domain/src/types.rs`, above the tests**

```rust
//! Core value types. Constructors validate, so an invalid value can never exist.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::DomainError;

macro_rules! id_type {
    ($($(#[$meta:meta])* $name:ident),+ $(,)?) => {$(
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    )+};
}

id_type!(
    /// Identifies a user account.
    UserId,
    /// Identifies a smoke event (one incident).
    EventId,
    /// Identifies a smell report.
    ReportId,
    /// Identifies a PM2.5 reading.
    ReadingId,
    /// Identifies a room.
    RoomId,
    /// Identifies a symptom.
    SymptomId,
);

/// Defines a field-less enum stored as text: `as_str`, `FromStr`, `Display`, serde, and `ALL`.
macro_rules! str_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($(#[$vmeta:meta])* $variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($(#[$vmeta])* $variant),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The stable text form, used in the database and in serde.
            pub fn as_str(self) -> &'static str {
                match self {
                    $($name::$variant => $text),+
                }
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::DomainError;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok($name::$variant),)+
                    _ => Err($crate::DomainError::UnknownValue {
                        kind: stringify!($name),
                        value: s.to_owned(),
                    }),
                }
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = <String as ::serde::Deserialize>::deserialize(deserializer)?;
                text.parse().map_err(::serde::de::Error::custom)
            }
        }
    };
}

str_enum!(
    /// What a user is allowed to do; see `permissions`.
    Role {
        Resident => "resident",
        Guest => "guest",
        Viewer => "viewer",
    }
);

str_enum!(
    /// Whether a room is inside the apartment or a shared common area.
    RoomArea {
        Inside => "inside",
        Common => "common",
    }
);

str_enum!(
    /// Why an event was closed.
    CloseReason {
        /// A user closed it.
        Manual => "manual",
        /// Someone started a new event while this one was open.
        NewEvent => "new_event",
        /// The nightly 04:00 sweep closed it.
        Auto0400 => "auto_0400",
        /// It was created in the past, already over.
        Backdated => "backdated",
    }
);

str_enum!(
    /// Interface language.
    Locale {
        En => "en",
        It => "it",
        Tr => "tr",
    }
);

/// How strong a smell is, on the personal 1–10 scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Intensity(u8);

impl Intensity {
    pub const MIN: u8 = 1;
    pub const MAX: u8 = 10;

    pub fn new(value: u8) -> Result<Self, DomainError> {
        if (Self::MIN..=Self::MAX).contains(&value) {
            Ok(Self(value))
        } else {
            Err(DomainError::InvalidIntensity(value))
        }
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Intensity {
    type Error = DomainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Intensity> for u8 {
    fn from(intensity: Intensity) -> Self {
        intensity.0
    }
}

/// A PM2.5 concentration in µg/m³.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Pm25(f64);

impl Pm25 {
    pub const MAX: f64 = 1000.0;

    pub fn new(ugm3: f64) -> Result<Self, DomainError> {
        if ugm3.is_finite() && (0.0..=Self::MAX).contains(&ugm3) {
            Ok(Self(ugm3))
        } else {
            Err(DomainError::InvalidPm25(ugm3))
        }
    }

    pub fn ugm3(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Pm25 {
    type Error = DomainError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Pm25> for f64 {
    fn from(pm25: Pm25) -> Self {
        pm25.0
    }
}
```

The macro is not re-exported yet. Task 9 adds `pub(crate) use str_enum;` once another module
needs it, because an unused re-export would fail clippy with `-D warnings`.

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: 9 tests pass and clippy reports no warnings.

- [ ] **Step 7: Commit**

```bash
git add crates/domain Cargo.lock
git commit -m "feat(domain): validated newtypes (Intensity, Pm25, IDs) and str_enum! macro

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Clock

**Files:**
- Modify: `crates/domain/Cargo.toml`, `crates/domain/src/lib.rs`
- Create: `crates/domain/src/clock.rs`

**Interfaces:**
- Consumes: nothing
- Produces:
  - `trait Clock: Send + Sync { fn now(&self) -> DateTime<Utc>; }`
  - `SystemClock` (unit struct)
  - `FakeClock::new(DateTime<Utc>)`, `.set(DateTime<Utc>)`, `.advance(TimeDelta)`

  Later milestones hold the clock as `Arc<dyn Clock>`.

- [ ] **Step 1: Add chrono to `crates/domain/Cargo.toml` `[dependencies]`**

```toml
chrono.workspace = true
```

- [ ] **Step 2: Write the failing tests: create `crates/domain/src/clock.rs`**

```rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{DateTime, TimeDelta, Utc};

    use super::*;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn fake_clock_returns_the_time_it_was_given() {
        let clock = FakeClock::new(t("2026-01-15T10:00:00Z"));
        assert_eq!(clock.now(), t("2026-01-15T10:00:00Z"));
    }

    #[test]
    fn fake_clock_can_be_set_and_advanced() {
        let clock = FakeClock::new(t("2026-01-15T10:00:00Z"));
        clock.advance(TimeDelta::minutes(90));
        assert_eq!(clock.now(), t("2026-01-15T11:30:00Z"));
        clock.set(t("2026-03-29T01:59:00Z"));
        assert_eq!(clock.now(), t("2026-03-29T01:59:00Z"));
    }

    #[test]
    fn clocks_are_usable_as_shared_trait_objects() {
        let fake = Arc::new(FakeClock::new(t("2026-01-15T10:00:00Z")));
        let shared: Arc<dyn Clock> = fake.clone();
        fake.advance(TimeDelta::seconds(1));
        assert_eq!(shared.now(), t("2026-01-15T10:00:01Z"));

        let system: Arc<dyn Clock> = Arc::new(SystemClock);
        assert!(system.now() > t("2026-01-01T00:00:00Z"));
    }
}
```

Add `pub mod clock;` to `lib.rs` (keep modules alphabetical).

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain clock`
Expected: compilation FAILS: `cannot find type 'FakeClock'`.

- [ ] **Step 4: Implement: put this at the top of `clock.rs`**

```rust
//! The source of "now". Production uses [`SystemClock`]; tests use [`FakeClock`].

use std::sync::Mutex;

use chrono::{DateTime, TimeDelta, Utc};

/// Anything that can tell the current time.
pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

/// The real wall clock.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// A clock that only moves when told to.
#[derive(Debug)]
pub struct FakeClock {
    now: Mutex<DateTime<Utc>>,
}

impl FakeClock {
    pub fn new(start: DateTime<Utc>) -> Self {
        Self { now: Mutex::new(start) }
    }

    pub fn set(&self, to: DateTime<Utc>) {
        *self.now.lock().expect("clock mutex poisoned") = to;
    }

    pub fn advance(&self, by: TimeDelta) {
        *self.now.lock().expect("clock mutex poisoned") += by;
    }
}

impl Clock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock mutex poisoned")
    }
}
```

Remove the now-duplicate `use chrono::{DateTime, TimeDelta, Utc};` from the test module if
clippy flags it. `use super::*;` already brings those names in.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all tests pass (12 total); no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/domain Cargo.lock
git commit -m "feat(domain): Clock trait with system and fake implementations (time injection)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Permissions

**Files:**
- Modify: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/permissions.rs`

**Interfaces:**
- Consumes: `UserId`, `Role` (Task 4)
- Produces:
  - `struct Actor { pub id: UserId, pub role: Role, pub expires_at: Option<DateTime<Utc>> }`
    with `.is_expired(now) -> bool` and `.can_sign_in(now) -> bool`
  - `enum Action { LogEntry, ViewAll, ViewOwn, EditEntry { owner: UserId }, ManageEvent { owner: UserId }, ManageUsers, EditLists }`
    (`Copy`). `EditEntry` covers correct and move; `ManageEvent` covers close and reopen.
  - `fn can(actor: &Actor, action: Action, now: DateTime<Utc>) -> bool`

- [ ] **Step 1: Write the failing tests: create `crates/domain/src/permissions.rs`**

```rust
#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};

    use super::*;

    const ME: UserId = UserId(1);
    const OTHER: UserId = UserId(2);

    const ACTIONS: [Action; 9] = [
        Action::LogEntry,
        Action::ViewAll,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::EditEntry { owner: OTHER },
        Action::ManageEvent { owner: ME },
        Action::ManageEvent { owner: OTHER },
        Action::ManageUsers,
        Action::EditLists,
    ];

    fn now() -> DateTime<Utc> {
        "2026-06-01T12:00:00Z".parse().unwrap()
    }

    fn actor(role: Role, expires_at: Option<DateTime<Utc>>) -> Actor {
        Actor { id: ME, role, expires_at }
    }

    fn active() -> Option<DateTime<Utc>> {
        Some(now() + TimeDelta::days(1))
    }

    fn expired() -> Option<DateTime<Utc>> {
        Some(now() - TimeDelta::days(1))
    }

    /// The spec §4 permissions table, cell by cell. Columns follow `ACTIONS`:
    /// log, view all, view own, edit own, edit other, manage own event, manage other event,
    /// manage users, edit lists.
    #[test]
    fn permission_matrix_matches_the_spec() {
        const T: bool = true;
        const F: bool = false;
        let rows = [
            ("resident", actor(Role::Resident, None), [T, T, T, T, T, T, T, T, T]),
            ("active guest", actor(Role::Guest, active()), [T, F, T, T, F, T, F, F, F]),
            ("expired guest", actor(Role::Guest, expired()), [F, F, T, F, F, F, F, F, F]),
            ("active viewer", actor(Role::Viewer, active()), [F, T, T, F, F, F, F, F, F]),
            ("viewer without expiry", actor(Role::Viewer, None), [F, T, T, F, F, F, F, F, F]),
            ("expired viewer", actor(Role::Viewer, expired()), [F, F, F, F, F, F, F, F, F]),
        ];
        for (label, actor, expected) in rows {
            for (action, want) in ACTIONS.iter().zip(expected) {
                assert_eq!(can(&actor, *action, now()), want, "{label} / {action:?}");
            }
        }
    }

    #[test]
    fn expiry_at_the_exact_instant_counts_as_expired() {
        let guest = actor(Role::Guest, Some(now()));
        assert!(guest.is_expired(now()));
        assert!(!can(&guest, Action::LogEntry, now()));
    }

    #[test]
    fn residents_never_expire() {
        assert!(!actor(Role::Resident, None).is_expired(now()));
    }

    #[test]
    fn expired_guests_can_still_sign_in_but_expired_viewers_cannot() {
        assert!(actor(Role::Guest, expired()).can_sign_in(now()));
        assert!(!actor(Role::Viewer, expired()).can_sign_in(now()));
        assert!(actor(Role::Viewer, active()).can_sign_in(now()));
        assert!(actor(Role::Resident, None).can_sign_in(now()));
    }
}
```

Add `pub mod permissions;` to `lib.rs` (alphabetical).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain permissions`
Expected: compilation FAILS: `cannot find type 'Actor'`.

- [ ] **Step 3: Implement: put this at the top of `permissions.rs`**

```rust
//! Who may do what. This is the spec §4 table, expressed as one function.

use chrono::{DateTime, Utc};

use crate::types::{Role, UserId};

/// The signed-in user, as far as permissions are concerned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: UserId,
    pub role: Role,
    pub expires_at: Option<DateTime<Utc>>,
}

impl Actor {
    /// Expired once `now` reaches `expires_at`. Residents have no expiry.
    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|expires_at| expires_at <= now)
    }

    /// Expired guests keep read access to their own data; expired viewers lose all access.
    pub fn can_sign_in(&self, now: DateTime<Utc>) -> bool {
        !(self.role == Role::Viewer && self.is_expired(now))
    }
}

/// Something a user might try to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Log a smell report or PM2.5 reading.
    LogEntry,
    /// See everyone's data.
    ViewAll,
    /// See one's own data.
    ViewOwn,
    /// Correct or move an entry owned by `owner`.
    EditEntry { owner: UserId },
    /// Close or reopen an event owned by `owner`.
    ManageEvent { owner: UserId },
    /// Invite, expire, reactivate users or reset their passwords.
    ManageUsers,
    /// Edit the room and symptom lists.
    EditLists,
}

pub fn can(actor: &Actor, action: Action, now: DateTime<Utc>) -> bool {
    let active = !actor.is_expired(now);
    match actor.role {
        Role::Resident => true,
        Role::Guest => match action {
            Action::ViewOwn => true,
            Action::LogEntry => active,
            Action::EditEntry { owner } | Action::ManageEvent { owner } => {
                active && owner == actor.id
            }
            Action::ViewAll | Action::ManageUsers | Action::EditLists => false,
        },
        Role::Viewer => active && matches!(action, Action::ViewAll | Action::ViewOwn),
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all pass (16 total); no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/domain
git commit -m "feat(domain): permissions as one exhaustive match, tested cell by cell against the spec

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Daily schedule (the 04:00 boundary)

**Files:**
- Modify: `crates/domain/Cargo.toml`, `crates/domain/src/lib.rs`
- Create: `crates/domain/src/schedule.rs`

**Interfaces:**
- Consumes: `DomainError::InvalidHour` (Task 4)
- Produces:
  - `DailySchedule::new(tz: chrono_tz::Tz, hour: u32) -> Result<DailySchedule, DomainError>`
  - `.tz() -> Tz`, `.hour() -> u32`
  - `.last_boundary(now: DateTime<Utc>) -> DateTime<Utc>`: the most recent local `hour:00` at or
    before `now`
  - `.boundary_passed_since(since: DateTime<Utc>, now: DateTime<Utc>) -> bool`: true if a
    boundary lies in `(since, now]`. This is the auto-close sweep's "is it due?" check.

- [ ] **Step 1: Add chrono-tz to `crates/domain/Cargo.toml` `[dependencies]`**

```toml
chrono-tz.workspace = true
```

- [ ] **Step 2: Write the failing tests: create `crates/domain/src/schedule.rs`**

Reference facts for Europe/Rome: winter is UTC+1, summer UTC+2. DST starts 2026-03-29 at 02:00
local (clocks jump to 03:00) and ends 2026-10-25 at 03:00 local (clocks fall back to 02:00).

```rust
#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use chrono_tz::Europe::Rome;

    use super::*;
    use crate::DomainError;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn rome_at(hour: u32) -> DailySchedule {
        DailySchedule::new(Rome, hour).unwrap()
    }

    #[test]
    fn rejects_hours_outside_the_day() {
        assert_eq!(DailySchedule::new(Rome, 24), Err(DomainError::InvalidHour(24)));
        assert!(DailySchedule::new(Rome, 23).is_ok());
    }

    #[test]
    fn winter_boundary_is_four_local_three_utc() {
        assert_eq!(rome_at(4).last_boundary(t("2026-01-15T10:00:00Z")), t("2026-01-15T03:00:00Z"));
    }

    #[test]
    fn before_todays_boundary_returns_yesterdays() {
        assert_eq!(rome_at(4).last_boundary(t("2026-01-15T02:59:00Z")), t("2026-01-14T03:00:00Z"));
    }

    #[test]
    fn exactly_at_the_boundary_returns_it() {
        assert_eq!(rome_at(4).last_boundary(t("2026-01-15T03:00:00Z")), t("2026-01-15T03:00:00Z"));
    }

    #[test]
    fn summer_boundary_is_four_local_two_utc() {
        assert_eq!(rome_at(4).last_boundary(t("2026-07-01T12:00:00Z")), t("2026-07-01T02:00:00Z"));
    }

    #[test]
    fn spring_forward_day_still_closes_at_four_local() {
        assert_eq!(rome_at(4).last_boundary(t("2026-03-29T08:00:00Z")), t("2026-03-29T02:00:00Z"));
    }

    #[test]
    fn fall_back_day_still_closes_at_four_local() {
        assert_eq!(rome_at(4).last_boundary(t("2026-10-25T12:00:00Z")), t("2026-10-25T03:00:00Z"));
    }

    #[test]
    fn a_skipped_local_hour_resolves_to_the_end_of_the_gap() {
        // 02:00 does not exist in Rome on 2026-03-29; the boundary becomes 03:00 CEST.
        assert_eq!(rome_at(2).last_boundary(t("2026-03-29T08:00:00Z")), t("2026-03-29T01:00:00Z"));
    }

    #[test]
    fn a_repeated_local_hour_resolves_to_its_first_occurrence() {
        // 02:00 happens twice in Rome on 2026-10-25 (CEST, then CET); take the first.
        assert_eq!(rome_at(2).last_boundary(t("2026-10-25T12:00:00Z")), t("2026-10-25T00:00:00Z"));
    }

    #[test]
    fn boundary_passed_since_detects_a_crossing() {
        let s = rome_at(4);
        assert!(s.boundary_passed_since(t("2026-01-14T23:00:00Z"), t("2026-01-15T08:00:00Z")));
        assert!(s.boundary_passed_since(t("2026-01-14T23:00:00Z"), t("2026-01-15T03:00:00Z")));
    }

    #[test]
    fn boundary_passed_since_is_false_without_a_crossing() {
        let s = rome_at(4);
        // The sweep ran exactly at the boundary: not due again until tomorrow.
        assert!(!s.boundary_passed_since(t("2026-01-15T03:00:00Z"), t("2026-01-15T20:00:00Z")));
        assert!(!s.boundary_passed_since(t("2026-01-15T04:00:00Z"), t("2026-01-15T05:00:00Z")));
    }

    #[test]
    fn catch_up_after_sleeping_through_several_nights_is_still_one_sweep() {
        let s = rome_at(4);
        assert!(s.boundary_passed_since(t("2026-01-10T12:00:00Z"), t("2026-01-15T08:00:00Z")));
    }
}
```

Add `pub mod schedule;` to `lib.rs` (alphabetical).

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain schedule`
Expected: compilation FAILS: `cannot find type 'DailySchedule'`.

- [ ] **Step 4: Implement: put this at the top of `schedule.rs`**

```rust
//! A time of day that recurs in a local time zone, such as the 04:00 auto-close.

use chrono::{DateTime, LocalResult, NaiveDate, TimeDelta, TimeZone, Utc};
use chrono_tz::Tz;

use crate::DomainError;

/// "Every day at `hour`:00 local time in `tz`."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailySchedule {
    tz: Tz,
    hour: u32,
}

impl DailySchedule {
    pub fn new(tz: Tz, hour: u32) -> Result<Self, DomainError> {
        if hour < 24 {
            Ok(Self { tz, hour })
        } else {
            Err(DomainError::InvalidHour(hour))
        }
    }

    pub fn tz(&self) -> Tz {
        self.tz
    }

    pub fn hour(&self) -> u32 {
        self.hour
    }

    /// The most recent boundary at or before `now`.
    pub fn last_boundary(&self, now: DateTime<Utc>) -> DateTime<Utc> {
        let today = now.with_timezone(&self.tz).date_naive();
        let candidate = self.boundary_on(today);
        if candidate <= now {
            candidate
        } else {
            let yesterday = today.pred_opt().expect("date within chrono's range");
            self.boundary_on(yesterday)
        }
    }

    /// Whether at least one boundary lies in `(since, now]`.
    pub fn boundary_passed_since(&self, since: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        self.last_boundary(now) > since
    }

    /// The boundary instant on a local calendar date. A local time skipped by DST resolves to
    /// the end of the gap; a repeated one resolves to its first occurrence.
    fn boundary_on(&self, date: NaiveDate) -> DateTime<Utc> {
        let local = date.and_hms_opt(self.hour, 0, 0).expect("hour validated in new()");
        let instant = match self.tz.from_local_datetime(&local) {
            LocalResult::Single(t) => t,
            LocalResult::Ambiguous(earliest, _) => earliest,
            LocalResult::None => self
                .tz
                .from_local_datetime(&(local + TimeDelta::hours(1)))
                .earliest()
                .expect("DST gaps last at most one hour"),
        };
        instant.with_timezone(&Utc)
    }
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all pass (28 total); no warnings.

If `a_skipped_local_hour_resolves_to_the_end_of_the_gap` fails, print
`Rome.from_local_datetime(...)` for 2026-03-29 02:00 and 03:00 to see what chrono-tz returns.
Do not loosen the assertion: the expected instant is 01:00 UTC.

- [ ] **Step 6: Commit**

```bash
git add crates/domain Cargo.lock
git commit -m "feat(domain): DailySchedule finds the local 04:00 boundary, DST-safe

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Event lifecycle rules

**Files:**
- Modify: `crates/domain/src/lib.rs`
- Create: `crates/domain/src/events.rs`

**Interfaces:**
- Consumes: `DailySchedule::boundary_passed_since` (Task 7), `EventId` and `DomainError`
  (Task 4)
- Produces:
  - `fn backdate_threshold() -> TimeDelta` (10 min), `fn future_tolerance() -> TimeDelta`
    (1 min), `fn large_gap() -> TimeDelta` (3 h)
  - `enum ObservedTime { Now, Backdated }`
  - `fn classify_observed(observed_at: DateTime<Utc>, now: DateTime<Utc>) -> Result<ObservedTime, DomainError>`
  - `struct EventSummary { pub id: EventId, pub opened_at: DateTime<Utc> }` (`Copy`)
  - `enum LogPrompt { Direct, SameSmell(EventSummary), WhichEvent, AutoClosed(EventSummary) }`
  - `fn log_prompt(observed: ObservedTime, open_event: Option<EventSummary>, unseen_auto_closed: Option<EventSummary>) -> LogPrompt`
  - `enum NewEventState { Open, ClosedBackdated }`
  - `fn new_event_state(observed_at: DateTime<Utc>, now: DateTime<Utc>, schedule: &DailySchedule, another_event_open: bool) -> NewEventState`
  - `fn validate_manual_close(closed_at: DateTime<Utc>, last_report_at: DateTime<Utc>, now: DateTime<Utc>) -> Result<(), DomainError>`
  - `fn large_gap_indices(observed_times: &[DateTime<Utc>]) -> Vec<usize>`: index `i` means the
    gap between `i-1` and `i` is too large. Input must be sorted ascending.

- [ ] **Step 1: Write the failing tests: create `crates/domain/src/events.rs`**

```rust
#[cfg(test)]
mod tests {
    use chrono::{DateTime, TimeDelta, Utc};
    use chrono_tz::Europe::Rome;

    use super::*;
    use crate::DomainError;

    fn t(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    fn now() -> DateTime<Utc> {
        t("2026-01-15T20:00:00Z")
    }

    fn summary(id: i64) -> EventSummary {
        EventSummary { id: EventId(id), opened_at: t("2026-01-15T18:05:00Z") }
    }

    fn schedule() -> DailySchedule {
        DailySchedule::new(Rome, 4).unwrap()
    }

    // --- classify_observed ---

    #[test]
    fn recent_times_count_as_now() {
        assert_eq!(classify_observed(now(), now()), Ok(ObservedTime::Now));
        assert_eq!(classify_observed(now() - TimeDelta::minutes(9), now()), Ok(ObservedTime::Now));
        assert_eq!(classify_observed(now() - TimeDelta::minutes(10), now()), Ok(ObservedTime::Now));
    }

    #[test]
    fn older_times_count_as_backdated() {
        assert_eq!(
            classify_observed(now() - TimeDelta::minutes(11), now()),
            Ok(ObservedTime::Backdated)
        );
        assert_eq!(
            classify_observed(now() - TimeDelta::days(400), now()),
            Ok(ObservedTime::Backdated)
        );
    }

    #[test]
    fn slightly_fast_phone_clocks_are_tolerated() {
        assert_eq!(classify_observed(now() + TimeDelta::seconds(30), now()), Ok(ObservedTime::Now));
    }

    #[test]
    fn future_times_are_rejected() {
        assert_eq!(
            classify_observed(now() + TimeDelta::minutes(2), now()),
            Err(DomainError::ObservedInFuture)
        );
    }

    // --- log_prompt ---

    #[test]
    fn backdated_logging_always_asks_which_event() {
        assert_eq!(log_prompt(ObservedTime::Backdated, Some(summary(1)), Some(summary(2))), LogPrompt::WhichEvent);
        assert_eq!(log_prompt(ObservedTime::Backdated, None, None), LogPrompt::WhichEvent);
    }

    #[test]
    fn an_open_event_asks_same_smell_and_wins_over_a_notice() {
        assert_eq!(log_prompt(ObservedTime::Now, Some(summary(1)), None), LogPrompt::SameSmell(summary(1)));
        assert_eq!(
            log_prompt(ObservedTime::Now, Some(summary(1)), Some(summary(2))),
            LogPrompt::SameSmell(summary(1))
        );
    }

    #[test]
    fn an_unseen_auto_close_is_offered_when_nothing_is_open() {
        assert_eq!(log_prompt(ObservedTime::Now, None, Some(summary(2))), LogPrompt::AutoClosed(summary(2)));
    }

    #[test]
    fn nothing_open_and_nothing_pending_logs_directly() {
        assert_eq!(log_prompt(ObservedTime::Now, None, None), LogPrompt::Direct);
    }

    // --- new_event_state ---

    #[test]
    fn a_new_event_logged_now_is_open() {
        assert_eq!(new_event_state(now(), now(), &schedule(), false), NewEventState::Open);
    }

    #[test]
    fn a_new_event_is_closed_if_another_is_already_open() {
        assert_eq!(
            new_event_state(t("2026-01-15T10:00:00Z"), now(), &schedule(), true),
            NewEventState::ClosedBackdated
        );
    }

    #[test]
    fn a_backdated_event_stays_open_if_no_four_am_has_passed() {
        // Observed 02:00 local, now 03:30 local, same night.
        assert_eq!(
            new_event_state(t("2026-01-15T01:00:00Z"), t("2026-01-15T02:30:00Z"), &schedule(), false),
            NewEventState::Open
        );
    }

    #[test]
    fn a_backdated_event_across_four_am_is_created_closed() {
        assert_eq!(
            new_event_state(t("2026-01-14T20:00:00Z"), t("2026-01-15T08:00:00Z"), &schedule(), false),
            NewEventState::ClosedBackdated
        );
    }

    // --- validate_manual_close ---

    #[test]
    fn closing_at_or_after_the_last_report_and_not_in_the_future_is_valid() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(validate_manual_close(last, last, now()), Ok(()));
        assert_eq!(validate_manual_close(now(), last, now()), Ok(()));
    }

    #[test]
    fn closing_before_the_last_report_is_rejected() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(last - TimeDelta::minutes(1), last, now()),
            Err(DomainError::CloseBeforeLastReport)
        );
    }

    #[test]
    fn closing_in_the_future_is_rejected() {
        let last = t("2026-01-15T19:00:00Z");
        assert_eq!(
            validate_manual_close(now() + TimeDelta::minutes(5), last, now()),
            Err(DomainError::CloseInFuture)
        );
    }

    // --- large_gap_indices ---

    #[test]
    fn no_reports_or_one_report_has_no_gaps() {
        assert!(large_gap_indices(&[]).is_empty());
        assert!(large_gap_indices(&[now()]).is_empty());
    }

    #[test]
    fn gaps_over_three_hours_are_flagged_at_the_later_report() {
        let t0 = t("2026-01-15T09:00:00Z");
        let times = [t0, t0 + TimeDelta::hours(1), t0 + TimeDelta::hours(5), t0 + TimeDelta::minutes(330)];
        assert_eq!(large_gap_indices(&times), vec![2]);
    }

    #[test]
    fn a_gap_of_exactly_three_hours_is_not_flagged() {
        let t0 = t("2026-01-15T09:00:00Z");
        assert!(large_gap_indices(&[t0, t0 + TimeDelta::hours(3)]).is_empty());
    }
}
```

Add `pub mod events;` to `lib.rs` (alphabetical).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain events`
Expected: compilation FAILS: `cannot find function 'classify_observed'`.

- [ ] **Step 3: Implement: put this at the top of `events.rs`**

```rust
//! Event lifecycle rules (spec §7) that need no database.

use chrono::{DateTime, TimeDelta, Utc};

use crate::DomainError;
use crate::schedule::DailySchedule;
use crate::types::EventId;

/// Reports observed longer ago than this count as backdated.
pub fn backdate_threshold() -> TimeDelta {
    TimeDelta::minutes(10)
}

/// Phone clocks drift; times this far ahead of the server still count as valid.
pub fn future_tolerance() -> TimeDelta {
    TimeDelta::minutes(1)
}

/// Consecutive reports in one event further apart than this get a warning.
pub fn large_gap() -> TimeDelta {
    TimeDelta::hours(3)
}

/// Whether a report describes the present or the past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedTime {
    Now,
    Backdated,
}

pub fn classify_observed(
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<ObservedTime, DomainError> {
    if observed_at > now + future_tolerance() {
        Err(DomainError::ObservedInFuture)
    } else if observed_at < now - backdate_threshold() {
        Ok(ObservedTime::Backdated)
    } else {
        Ok(ObservedTime::Now)
    }
}

/// The minimum an event prompt needs to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSummary {
    pub id: EventId,
    pub opened_at: DateTime<Utc>,
}

/// The question to ask before saving a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPrompt {
    /// Nothing to ask: the report starts a new event.
    Direct,
    /// "Same smell still going?" about the open event.
    SameSmell(EventSummary),
    /// "Which event was this?" for a backdated report.
    WhichEvent,
    /// "This event was auto-closed — was it still going?"
    AutoClosed(EventSummary),
}

/// Picks the prompt. A backdated report always asks which event; otherwise an open event takes
/// priority over a pending auto-close notice, which the UI then shows only as a banner.
pub fn log_prompt(
    observed: ObservedTime,
    open_event: Option<EventSummary>,
    unseen_auto_closed: Option<EventSummary>,
) -> LogPrompt {
    match (observed, open_event, unseen_auto_closed) {
        (ObservedTime::Backdated, _, _) => LogPrompt::WhichEvent,
        (ObservedTime::Now, Some(open), _) => LogPrompt::SameSmell(open),
        (ObservedTime::Now, None, Some(closed)) => LogPrompt::AutoClosed(closed),
        (ObservedTime::Now, None, None) => LogPrompt::Direct,
    }
}

/// Whether a newly created event starts open or already closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewEventState {
    Open,
    ClosedBackdated,
}

/// At most one event is open at a time, and nothing stays open across a 04:00 boundary.
pub fn new_event_state(
    observed_at: DateTime<Utc>,
    now: DateTime<Utc>,
    schedule: &DailySchedule,
    another_event_open: bool,
) -> NewEventState {
    if another_event_open || schedule.boundary_passed_since(observed_at, now) {
        NewEventState::ClosedBackdated
    } else {
        NewEventState::Open
    }
}

pub fn validate_manual_close(
    closed_at: DateTime<Utc>,
    last_report_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), DomainError> {
    if closed_at < last_report_at {
        Err(DomainError::CloseBeforeLastReport)
    } else if closed_at > now + future_tolerance() {
        Err(DomainError::CloseInFuture)
    } else {
        Ok(())
    }
}

/// Indices of reports that follow a gap longer than [`large_gap`]. `observed_times` must be
/// sorted ascending.
pub fn large_gap_indices(observed_times: &[DateTime<Utc>]) -> Vec<usize> {
    observed_times
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[1] - pair[0] > large_gap())
        .map(|(i, _)| i + 1)
        .collect()
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo fmt --all && cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all pass (46 total); no warnings. `cargo fmt` re-wraps the long test lines; that's
expected.

- [ ] **Step 5: Commit**

```bash
git add crates/domain
git commit -m "feat(domain): event lifecycle rules as pure functions over enums (prompt, backdating, close, gaps)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Tamper-evident audit hash chain

**Files:**
- Modify: `crates/domain/Cargo.toml`, `crates/domain/src/lib.rs`
- Create: `crates/domain/src/audit.rs`

**Files (additional):**
- Modify: `crates/domain/src/types.rs` (re-export `str_enum!` for crate-internal use)

**Interfaces:**
- Consumes: `str_enum!` and `UserId` (Task 4)
- Produces:
  - `enum AuditAction { Create, Update, Delete, Close, Reopen, Move, Invite, Expire, Reactivate, PasswordChange }`
    (`str_enum!`; text e.g. `"password_change"`)
  - `const GENESIS_HASH: &str` (64 × `'0'`)
  - `struct AuditRecord { pub id: i64, pub at_micros: i64, pub actor_id: Option<UserId>, pub entity: String, pub entity_id: i64, pub action: AuditAction, pub before: Option<serde_json::Value>, pub after: Option<serde_json::Value> }`
  - `fn compute_hash(record: &AuditRecord, prev_hash: &str) -> String` (64 lowercase hex chars)
  - `struct ChainedEntry { pub record: AuditRecord, pub prev_hash: String, pub hash: String }`
    with `ChainedEntry::seal(record, prev_hash: &str) -> ChainedEntry`
  - `enum ChainError { BrokenLink { id: i64 }, HashMismatch { id: i64 } }`
  - `fn verify_chain(entries: &[ChainedEntry]) -> Result<(), ChainError>` (entries ordered by id)

  Milestone 2 stores these columns and calls `seal` and `verify_chain`.

- [ ] **Step 1: Add dependencies to `crates/domain/Cargo.toml`**

Move `serde_json` from `[dev-dependencies]` to `[dependencies]`, and add `sha2`, `hex` and
`proptest`:
```toml
[dependencies]
chrono.workspace = true
chrono-tz.workspace = true
hex.workspace = true
serde.workspace = true
serde_json.workspace = true
sha2.workspace = true
thiserror.workspace = true

[dev-dependencies]
proptest.workspace = true
```

- [ ] **Step 2: Write the failing tests: create `crates/domain/src/audit.rs`**

```rust
#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use proptest::prelude::*;
    use proptest::sample::Index;
    use serde_json::{Map, Value, json};

    use super::*;

    fn record(id: i64, after: Value) -> AuditRecord {
        AuditRecord {
            id,
            at_micros: 1_768_500_000_000_000 + id,
            actor_id: Some(UserId(1)),
            entity: "smell_report".into(),
            entity_id: 12,
            action: AuditAction::Create,
            before: None,
            after: Some(after),
        }
    }

    fn build_chain(n: usize) -> Vec<ChainedEntry> {
        let mut chain: Vec<ChainedEntry> = Vec::with_capacity(n);
        for i in 0..n {
            let prev = chain.last().map_or(GENESIS_HASH, |e| e.hash.as_str()).to_owned();
            let id = i as i64 + 1;
            chain.push(ChainedEntry::seal(record(id, json!({ "intensity": (i % 10) + 1 })), &prev));
        }
        chain
    }

    #[test]
    fn genesis_hash_is_sixty_four_zeros() {
        assert_eq!(GENESIS_HASH.len(), 64);
        assert!(GENESIS_HASH.chars().all(|c| c == '0'));
    }

    #[test]
    fn hash_is_deterministic_lowercase_hex() {
        let r = record(1, json!({ "intensity": 6 }));
        let a = compute_hash(&r, GENESIS_HASH);
        assert_eq!(a, compute_hash(&r, GENESIS_HASH));
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn hash_depends_on_the_previous_hash() {
        let r = record(1, json!({ "intensity": 6 }));
        assert_ne!(compute_hash(&r, GENESIS_HASH), compute_hash(&r, &"f".repeat(64)));
    }

    /// Guards against serde_json's `preserve_order` feature being switched on by any
    /// dependency (Cargo unifies features): key order must never affect the hash.
    #[test]
    fn json_key_order_does_not_change_the_hash() {
        let mut za = Map::new();
        za.insert("z".into(), json!(1));
        za.insert("a".into(), json!(2));
        let mut az = Map::new();
        az.insert("a".into(), json!(2));
        az.insert("z".into(), json!(1));
        assert_eq!(
            compute_hash(&record(1, Value::Object(za)), GENESIS_HASH),
            compute_hash(&record(1, Value::Object(az)), GENESIS_HASH)
        );
    }

    /// Postgres keeps microseconds; anything finer must not reach the hash.
    #[test]
    fn sub_microsecond_precision_does_not_change_the_hash() {
        let precise: DateTime<Utc> = "2026-01-15T20:00:00.123456789Z".parse().unwrap();
        let as_stored_by_postgres: DateTime<Utc> = "2026-01-15T20:00:00.123456Z".parse().unwrap();
        let mut a = record(1, json!({}));
        let mut b = record(1, json!({}));
        a.at_micros = precise.timestamp_micros();
        b.at_micros = as_stored_by_postgres.timestamp_micros();
        assert_eq!(compute_hash(&a, GENESIS_HASH), compute_hash(&b, GENESIS_HASH));
    }

    #[test]
    fn an_empty_chain_and_a_valid_chain_verify() {
        assert_eq!(verify_chain(&[]), Ok(()));
        assert_eq!(verify_chain(&build_chain(5)), Ok(()));
    }

    #[test]
    fn editing_an_entry_is_reported_at_that_entry() {
        let mut chain = build_chain(3);
        chain[0].record.after = Some(json!({ "intensity": 9 }));
        assert_eq!(verify_chain(&chain), Err(ChainError::HashMismatch { id: 1 }));
    }

    #[test]
    fn re_hashing_an_edited_entry_breaks_the_next_link() {
        let mut chain = build_chain(3);
        chain[0].record.after = Some(json!({ "intensity": 9 }));
        chain[0].hash = compute_hash(&chain[0].record, &chain[0].prev_hash);
        assert_eq!(verify_chain(&chain), Err(ChainError::BrokenLink { id: 2 }));
    }

    #[test]
    fn deleting_an_entry_breaks_the_chain() {
        let mut chain = build_chain(3);
        chain.remove(1);
        assert_eq!(verify_chain(&chain), Err(ChainError::BrokenLink { id: 3 }));
    }

    #[test]
    fn audit_actions_use_snake_case_text() {
        assert_eq!(AuditAction::PasswordChange.as_str(), "password_change");
        assert_eq!("reopen".parse::<AuditAction>(), Ok(AuditAction::Reopen));
    }

    proptest! {
        #[test]
        fn any_change_to_any_entry_is_detected(
            n in 1usize..20,
            pick in any::<Index>(),
            new_entity_id in any::<i64>(),
        ) {
            let mut chain = build_chain(n);
            let k = pick.index(n);
            prop_assume!(chain[k].record.entity_id != new_entity_id);
            chain[k].record.entity_id = new_entity_id;
            prop_assert_eq!(
                verify_chain(&chain),
                Err(ChainError::HashMismatch { id: chain[k].record.id })
            );
        }
    }
}
```

Add `pub mod audit;` to `lib.rs` (alphabetical).

In `crates/domain/src/types.rs`, directly after the closing `}` of `macro_rules! str_enum`, add:
```rust
pub(crate) use str_enum;
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain audit`
Expected: compilation FAILS: `cannot find type 'AuditRecord'`.

- [ ] **Step 4: Implement: put this at the top of `audit.rs`**

```rust
//! The tamper-evident audit log: each entry's hash covers its content and the previous hash,
//! so changing or removing any entry breaks every link after it.

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::types::{UserId, str_enum};

str_enum!(
    /// What happened to an entity.
    AuditAction {
        Create => "create",
        Update => "update",
        Delete => "delete",
        Close => "close",
        Reopen => "reopen",
        Move => "move",
        Invite => "invite",
        Expire => "expire",
        Reactivate => "reactivate",
        PasswordChange => "password_change",
    }
);

/// The `prev_hash` of the first entry.
pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One audit entry's content: everything the hash covers.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AuditRecord {
    pub id: i64,
    /// Microseconds since the Unix epoch: the finest precision both SQLite and Postgres keep.
    pub at_micros: i64,
    pub actor_id: Option<UserId>,
    pub entity: String,
    pub entity_id: i64,
    pub action: AuditAction,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

/// SHA-256 over `prev_hash`, a newline, and the record as canonical JSON, as lowercase hex.
///
/// Converting to [`Value`] first sorts every object's keys (serde_json's default map is
/// ordered), so the bytes depend only on the data, never on the database or field order.
pub fn compute_hash(record: &AuditRecord, prev_hash: &str) -> String {
    let canonical = serde_json::to_value(record).expect("AuditRecord always serialises");
    let bytes = serde_json::to_vec(&canonical).expect("a Value always serialises");
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"\n");
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

/// A record together with its place in the chain.
#[derive(Debug, Clone, PartialEq)]
pub struct ChainedEntry {
    pub record: AuditRecord,
    pub prev_hash: String,
    pub hash: String,
}

impl ChainedEntry {
    /// Computes the hash and links the record after `prev_hash`.
    pub fn seal(record: AuditRecord, prev_hash: &str) -> Self {
        let hash = compute_hash(&record, prev_hash);
        Self { record, prev_hash: prev_hash.to_owned(), hash }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChainError {
    #[error("audit entry {id}: its prev_hash does not match the previous entry's hash")]
    BrokenLink { id: i64 },
    #[error("audit entry {id}: its content does not match its hash")]
    HashMismatch { id: i64 },
}

/// Checks the whole chain from the genesis hash and reports the first problem found.
pub fn verify_chain(entries: &[ChainedEntry]) -> Result<(), ChainError> {
    let mut expected_prev = GENESIS_HASH;
    for entry in entries {
        let id = entry.record.id;
        if entry.prev_hash != expected_prev {
            return Err(ChainError::BrokenLink { id });
        }
        if compute_hash(&entry.record, &entry.prev_hash) != entry.hash {
            return Err(ChainError::HashMismatch { id });
        }
        expected_prev = &entry.hash;
    }
    Ok(())
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo fmt --all && cargo test -p smoke-domain && cargo clippy -p smoke-domain --all-targets -- -D warnings`
Expected: all pass (57 total, including the proptest run of 256 random cases); no warnings.

- [ ] **Step 6: Commit**

```bash
git add crates/domain Cargo.lock
git commit -m "feat(domain): SHA-256 hash-chained audit log with property-tested tamper detection

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Milestone verification

**Files:** none created. Read-only checks, plus one roadmap update.

- [ ] **Step 1: Run everything CI will run**

Run: `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets --locked && cargo fmt --all --check && cargo test --workspace --locked && cargo deny check && actionlint`
Expected: every command succeeds.

- [ ] **Step 2: Check that no personal data or secrets are tracked**

Run: `git ls-files | xargs grep -nIoE '[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[a-z]{2,}' | grep -vE 'users\.noreply\.github\.com|noreply@anthropic\.com|git@github\.com' ; gitleaks git --no-banner --redact`
Expected: `grep` prints nothing (the only addresses allowed in tracked files are the GitHub noreply address, the Co-Authored-By trailer address and the `git@github.com` remote), and gitleaks prints `no leaks found`.

- [ ] **Step 3: Check commit authorship**

Run: `git log --format='%an <%ae>' | sort -u`
Expected: exactly one line, `locadani <61902944+locadani@users.noreply.github.com>`.

- [ ] **Step 4: Mark milestone 1 done in `docs/plans/ROADMAP.md` and commit**

In the Milestone 1 row, change the plan cell from `` `01-foundation-and-domain.md` `` to
`` `01-foundation-and-domain.md` ✅ ``.

```bash
git add docs/plans/ROADMAP.md
git commit -m "docs: milestone 1 complete

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 5: Hand over to the user**

Tell the user the milestone is done and list their GitHub steps:
1. Create the public repo `locadani/smoke-tracker` (no README, licence or .gitignore, since
   they already exist locally).
2. Run `git remote add origin git@github.com:locadani/smoke-tracker.git && git push -u origin main`.
3. Enable: Settings → Code security → **Secret scanning** and **Push protection**, and **Dependabot alerts**.
4. Settings → Branches → protect `main`: require a PR and require the `fmt · clippy · test`,
   `cargo-deny` and `gitleaks` checks to pass.
5. Create the push-reminder issue (text in spec §14).
6. GitHub → Settings → Emails: tick **Block command line pushes that expose my email**.
