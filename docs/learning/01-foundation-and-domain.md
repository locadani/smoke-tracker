# 01 — Foundation and domain

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 1 (newtypes), 2 (permissions), 3 (macro), 4 (audit chain). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn

- How a workspace, modules and privacy enforce layers; how a newtype makes invalid values impossible.
- How `enum` and `match` turn a permissions table into code the compiler checks.
- How `macro_rules!` removes repeated code, and when not to use it.
- How `thiserror` and `anyhow` split the work of error handling between libraries and the app.
- Why time is a dependency, and how a hash-chained log uses slices, iterators and `Result`.

## The map

This note covers the Cargo workspace and the pure `domain` crate, which holds the rules and does
no input or output, so its tests run in milliseconds. The later notes build on it.

- `Cargo.toml`: the root file that declares the workspace, shared versions and lints.
- `crates/domain/Cargo.toml` and `crates/domain/src/lib.rs`: the domain crate, with no database or web dependency.
- `crates/domain/src/types.rs`: value types, the id types and the macros that build them.
- `crates/domain/src/permissions.rs`: who may do what (spec §4).
- `crates/domain/src/error.rs` and `crates/domain/src/clock.rs`: the error enum and the source of "now".
- `crates/domain/src/schedule.rs`: a daily time in a local zone, such as the 04:00 auto-close.
- `crates/domain/src/audit.rs`: the hash-chained audit log (spec §6).
- `crates/app/src/main.rs`: the binary's entry point, where errors become an exit code.

## 1. A Cargo workspace splits the app into crates

**The idea.** A crate is the unit the Rust compiler builds: a library or a program. A workspace is
a set of crates in one repository that share one lockfile and one output folder. Our app has a web
layer, two database layers and a program that must agree on the same rules, so it is six crates.

**In our code.**

- `Cargo.toml` (root): `[workspace]` with `members = ["crates/*"]`, then `[workspace.package]`
  (edition, licence and so on, written once), `[workspace.dependencies]` (every external
  version written once) and `[workspace.lints.rust]` with `unsafe_code = "forbid"`.
- `crates/domain/Cargo.toml`: each dependency is written as `chrono.workspace = true`, meaning
  "use the version the root declared". `[lints] workspace = true` opts the crate into the root's
  lint settings.
- `crates/web/Cargo.toml`: it depends on `smoke-domain.workspace = true`. The SQLite storage crate
  appears only under `[dev-dependencies]`, so the web crate's tests can use a real database but
  its shipped code cannot.
- The direction of the arrows: `smoke-tracker` (the app crate) depends on `smoke-web` and on the
  two storage crates, and all of those depend on `smoke-domain`. The domain depends on none of
  them.

**Why this way.** One lockfile means one version of each dependency in the whole app. The `domain`
crate has no database or web dependency, so its tests need no setup. Cargo refuses a cycle between
crates, so a mistake such as "domain uses the web layer" is a compile error, not a habit that
slips in during review (decisions #22 and #45).

Alternatives: one crate with modules is faster to start, but the layers are then only a
convention that nothing enforces. Separate repositories would give each layer its own version
numbers and releases, which is pure cost for one developer. A single crate is the better choice
when a project is small or when the layers are not yet clear.

**Read more.**

- [The Rust Book: Cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
- [Cargo reference: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)

## 2. Modules and privacy

**The idea.** A module is a named group of items inside a crate. Everything in it is private to it
and its child modules unless you share it with `pub` (everyone), `pub(crate)` (this crate) or
`pub use` (re-export under a shorter path). Privacy lets a type make a promise: if the field is
private, only code in that module can build the value.

**In our code.**

- `crates/domain/src/lib.rs`: a list of `pub mod` lines, and `pub use error::DomainError` so
  callers write `smoke_domain::DomainError` instead of the longer path.
- `crates/domain/src/types.rs::str_enum`: ends with `pub(crate) use str_enum;`. A `macro_rules!`
  macro is normally visible only below its definition. That line turns it into an item with a
  path and a visibility, so `audit.rs` can use it but other crates cannot.
- `crates/storage-sqlite/src/lib.rs`: private `mod` lines and one `pub use crate::tx::SqliteTx`;
  the outside world sees only what the crate exposes.
- `crates/web/src/lib.rs`: almost every module is private; the public surface is `router`,
  `serve`, `AppState` and a couple of re-exports.

**Why this way.** Section 3 depends on it: "only this module can build a `Level`" is true only
because the field is private. A small public surface also lets a crate refactor inside.

Alternatives: everything `pub` is easier to write and lets anyone break an invariant; one big
file stops scaling at a few hundred lines.

**Read more.**

- [The Rust Book: modules, scope and privacy](https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html)
- [The Rust Reference: visibility and privacy](https://doc.rust-lang.org/reference/visibility-and-privacy.html)

## 3. Newtypes that cannot be invalid

**The idea.** A newtype is a struct with one field that gives a plain value, such as a number, its
own type. When the field is private and the only way to build the type is a function that checks
the value, every value of that type is valid. The check happens once, at the door; code deeper in
the app takes a `Level` and never checks again.

**In our code.**

- `crates/domain/src/types.rs::Level`: a private `u8`, `Level::new` that returns a `Result`,
  the constant `Level::CLEAN`, `TryFrom<u8>` and `From<Level> for u8`. The attribute
  `#[serde(try_from = "u8", into = "u8")]` makes serde read a number, then call `TryFrom`, so a
  JSON `11` is refused too.
- `crates/domain/src/types.rs::Username`: trims, lowercases, then checks length and characters
  (decision #51). `crates/domain/src/types.rs::ListName` trims and counts characters.
  Both derive `Serialize` only, not `Deserialize`, so there is no back door around `new`.
- `crates/domain/src/types.rs::NewPassword`: counts `chars()`, not bytes, so a password of
  ten Turkish letters is ten characters even though it takes more than ten bytes.
  `Username` can use `len()` because it only allows ASCII, where one character is one byte.
- `crates/domain/src/types.rs::Pm25`: uses `is_finite` so NaN ("not a number", a floating-point
  value equal to nothing) is refused.
- Ids such as `UserId(pub i64)` are plain wrappers with no rule, so a `UserId` cannot be passed
  where an `EventId` is expected.

**Why this way.** Everything inside the app can trust a `Level`; a plain `u8` checked at each use
means one forgotten check is a bug. A `Result` lets the page show a message.

Alternatives: a validation crate adds a dependency for a four-line `if`, and pays off only with
dozens of rules. An `assert!` in the constructor would panic on a typed-in value instead of
telling the user.

**Read more.**

- [The Rust Book: defining structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)
- [std: `TryFrom`](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [serde: container attributes](https://serde.rs/container-attrs.html)
- [Parse, don't validate](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/),
  an article on the same idea in another language

**Try it.** [Exercise 1](../../exercises/src/ex01_newtypes.rs) builds three such types.

## 4. Derives, and a Debug that does not leak

**The idea.** A `derive` attribute asks the compiler to write a standard trait implementation for
you. `Debug` is the trait behind `{:?}`, used in logs and in test failures. The derived version
prints every field, which is wrong for a secret.

**In our code.**

- `crates/domain/src/types.rs::Level`: a long derive list. `Copy` is allowed because the value
  is one byte. `Ord` is needed so `Iterator::max` works in
  `crates/domain/src/records.rs::ReportValues::intensity`, and `Hash` lets a `Level` be a map key.
- `crates/domain/src/types.rs::NewPassword` writes its own `Debug` and prints
  `NewPassword(<redacted>)`. `crates/domain/src/types.rs::TokenHash` prints only the first eight
  characters of the hash (slicing by bytes is safe there because hex is ASCII).
  `crates/domain/src/records.rs::PasswordHash` redacts the same way. A stray `{:?}` in a log line
  cannot print a secret.

**Why this way.** For a secret, the safe default is to print nothing, and the hand-written
implementation is six lines. A test in the same file checks the printed text.

Alternative: a wrapper from a crate such as `secrecy`, which also wipes memory on drop. That
pays off with many secrets. Here there are three.

**Read more.**

- [std: `Debug`](https://doc.rust-lang.org/std/fmt/trait.Debug.html)
- [std: `Display`](https://doc.rust-lang.org/std/fmt/trait.Display.html)

## 5. Enums and match: permissions

**The idea.** A Rust `enum` is exactly one of a fixed list of variants, and a variant can carry
data. A `match` takes such a value apart, and the compiler refuses a `match` that forgets a
variant (exhaustiveness). "Did I handle every case?" becomes a question for the compiler.

**In our code.**

- `crates/domain/src/permissions.rs::Action`: variants that carry data, such as
  `EditEntry { owner: UserId }`.
- `crates/domain/src/permissions.rs::can`: an outer `match` on the role, an inner one on the
  action. There is no catch-all `_` arm on the role, so adding a fourth role stops the build here.
  The guest arm lists even the denied actions by name for the same reason. The
  pattern `Action::EditEntry { owner } | Action::ManageEvent { owner }` binds the same name in
  both alternatives. The viewer arm uses `matches!` instead, which is shorter and defaults to
  "no" for any new action.
- `crates/domain/src/permissions.rs::check_expiry`: matches a tuple `(role, expires_at)` with a
  guard, `(_, Some(at)) if at <= now`; the first matching arm wins.
- `crates/domain/src/permissions.rs::Actor`: `is_expired` uses `Option::is_some_and`, which is
  `false` for `None`, so a resident with no expiry is never expired.

**Why this way.** The permissions table in spec §4 is fixed, so code the compiler checks beats a
table nobody can check.

Alternatives: a table in the database is more flexible but gives no compiler help; it would
win if residents could define their own roles. A bag of boolean flags per user loses the link
between a flag and the action it allows.

**Read more.**

- [The Rust Book: defining an enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)
- [The Rust Book: the `match` control flow construct](https://doc.rust-lang.org/book/ch06-02-match.html)
- [std: `Option`](https://doc.rust-lang.org/std/option/enum.Option.html)

**Try it.** [Exercise 2](../../exercises/src/ex02_permissions.rs) writes `can` and `can_see`.

## 6. Macros: str_enum! and id_type!

**The idea.** A declarative macro, written with `macro_rules!`, is a pattern that the compiler
expands into code before it checks types. You use one when ordinary functions and generics cannot
do the job, for example when you must generate new type names. They are harder to read and to
debug than functions, so a plain function or a generic is the better tool whenever it can do the
job.

**In our code.**

- `crates/domain/src/types.rs::id_type`: one macro, seven id types. The part
  `$(#[$meta:meta])*` accepts any attributes in front of a name, and doc comments are attributes,
  so each id keeps its own comment. `$(...),+` repeats for each name.
- `crates/domain/src/types.rs::str_enum`: an enum stored as text. For each one it writes `ALL`,
  `as_str`, `FromStr`, `Display` and the serde implementations. Inside, `stringify!` turns the
  type name into text for the error message, and `$crate::DomainError` names a type in the
  defining crate, so the macro works from any module. Paths start with `::std` and `::serde` so a
  local name cannot shadow them.
- Used for `Role`, `RoomArea`, `CloseReason`, `InvitePurpose` and `Locale` (all in
  `crates/domain/src/types.rs`) and for `AuditAction` and `AuditEntity` in
  `crates/domain/src/audit.rs`: seven enums.

**Why this way.** The same forty lines for seven enums would drift apart. The macro keeps the text
form in one place.

Alternatives: a derive crate such as `strum` is one more dependency, and the stored text must
match the database `CHECK` lists exactly, which an explicit literal next to each variant makes
visible. A hand-written implementation per enum is plain and greppable, and the better choice
for one or two enums.

**Read more.**

- [The Rust Reference: macros by example](https://doc.rust-lang.org/reference/macros-by-example.html)
- [Rust by Example: macro_rules!](https://doc.rust-lang.org/rust-by-example/macros.html)
- [The Rust Book: macros](https://doc.rust-lang.org/book/ch20-05-macros.html)

**Try it.** [Exercise 3](../../exercises/src/ex03_text_enum.rs) writes a small version of
`str_enum!`.

## 7. Errors: thiserror for libraries, anyhow for the app

**The idea.** In Rust an operation that can fail returns `Result<T, E>`, and `E` is part of the
function's promise. A library should return a precise enum so the caller can `match` on what went
wrong. An application's `main` usually only prints the problem and stops.

**In our code.**

- `crates/domain/src/error.rs::DomainError`: `#[derive(Error)]` from `thiserror` writes the
  `Display` and `Error` implementations from the `#[error("...")]` text. `{0}` is the first field
  of the variant. `UnknownValue { kind, value }` shows named fields.
- `crates/domain/src/logbook.rs::LogError`: a variant with `#[error(transparent)]` and `#[from]`
  forwards its message to the inner error and generates a `From` implementation, so the `?`
  operator converts a `DomainError` or a `StoreError` into a `LogError` on its own.
- `crates/app/src/lib.rs::run` returns `anyhow::Result`. In `crates/app/src/main.rs` the error is
  printed with `{error:#}`, which prints the whole chain of causes on one line, and the process
  exits with code 2.

**Why this way.** `domain` is a library, so it hands out enums callers can match on. The binary is
the end of the line; nobody matches on its errors.

Alternatives: `Box<dyn Error>` everywhere hides the cases from callers. Hand-written `Display` and
`From` implementations are what `thiserror` generates; the web crate's `AppError` does it by
hand, and Note 03 explains why.

**Read more.**

- [The Rust Book: recoverable errors with `Result`](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [docs.rs: thiserror](https://docs.rs/thiserror)
- [docs.rs: anyhow](https://docs.rs/anyhow)
- [std: the `Error` trait](https://doc.rust-lang.org/std/error/trait.Error.html)

## 8. Time is a dependency: Clock

**The idea.** A function that calls `Utc::now()` itself gives a different answer every time, so
you cannot test it at 03:59 or across a daylight saving change. If the time is handed in, a test
can hand in any time it likes.

**In our code.**

- `crates/domain/src/clock.rs::Clock`: a trait with one method, `now`. Its supertraits
  `Send + Sync` say that every implementation can be shared between threads, which a web server
  needs.
- `crates/domain/src/clock.rs::SystemClock` reads the real time;
  `crates/domain/src/clock.rs::FakeClock` holds a `Mutex`, so `set` and `advance` work through
  `&self` (interior mutability: a change behind a shared reference that the lock makes safe).
- `crates/domain/src/clock.rs::truncate_to_micros` cuts instants to whole microseconds, which is
  what the database stores (decision #46), so SQLite and Postgres hold identical data.
- A use: `crates/web/src/state.rs::AppState` holds an `Arc<dyn Clock>`, so a test with a
  `FakeClock` controls time for every handler.

**Why this way.** Most domain functions take `now` as a plain argument, for example
`can(actor, action, now)`. The `Clock` trait is for the places that must fetch the time
themselves, such as a request handler.

Alternatives: a mocking crate generates fakes, but our trait has one method, so a hand-written
fake is shorter.

**Read more.**

- [The Rust Book: traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [std: `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html)
- [docs.rs: chrono](https://docs.rs/chrono)

## 9. Time zones and daylight saving

**The idea.** An instant is one moment in time; a local time is what a wall clock in some place
shows. Twice a year they stop matching one to one: when clocks jump forward a local time does not
exist, and when they go back one exists twice. A "day" can last 23 or 25 hours.

**In our code.**

- `crates/domain/src/schedule.rs::DailySchedule`: "every day at this hour in this zone".
  `last_boundary` finds the latest 04:00 at or before a moment, `boundary_passed_since` asks
  whether one lies between two moments, and `day_span`, `month_span` and `year_span` give the
  start and end of a local day, month or year.
- The private `at_local` matches on `chrono::LocalResult`. A single answer is used as is. A
  repeated local time resolves to its first occurrence. A skipped one resolves to the end of the
  gap.
- Everything is stored and passed around as `DateTime<Utc>`. The zone is only used at the edges
  to answer "which local day is this?" (decision #46).

**Why this way.** Store UTC, convert for display and for local-day questions. The 04:00 auto-close
is a local-time rule, so it has to know the zone.

Alternatives: storing local times is ambiguous twice a year; treating every day as 24 hours is
wrong twice a year. The tests in the same file cover both transitions.

**Read more.**

- [docs.rs: chrono](https://docs.rs/chrono)
- [docs.rs: chrono-tz](https://docs.rs/chrono-tz)

## 10. The audit chain: slices, iterators and Result

**The idea.** A hash function turns any data into a short fingerprint, and a tiny change gives a
completely different one. If every audit entry stores the fingerprint of its content and of the
previous entry, the entries form a chain. Editing or deleting one entry breaks every link after
it, and anyone with the data can check the chain.

**In our code.**

- `crates/domain/src/audit.rs::AuditRecord` is the content the hash covers;
  `crates/domain/src/audit.rs::compute_hash` is SHA-256 over the previous hash, a newline and the
  record as canonical JSON.
- `crates/domain/src/audit.rs::write_canonical`: writes the JSON by hand with sorted keys. The
  default `serde_json` map is sorted, but a Cargo feature (`preserve_order`) that another crate
  turns on changes it for the whole build. The CI step "audit hash ignores serde_json map
  ordering" in `.github/workflows/ci.yml` runs the audit tests with that feature on, to prove the
  bytes do not change.
- `crates/domain/src/audit.rs::ChainedEntry::seal`: computes the hash and links a record.
- `crates/domain/src/audit.rs::verify_chain`: a `for` loop over a slice, `&[ChainedEntry]`,
  carrying `expected_prev`, a borrowed `&str` that always points at the previous entry's hash.
  It returns the first `ChainError`, or `Ok(())`.
- `crates/domain/src/audit.rs::GENESIS_HASH`: 64 zeros, the first entry's `prev_hash`.
- `crates/domain/src/records.rs::SmellReport::upgrade_snapshot` returns a `Cow`, "clone on
  write": an unchanged snapshot is returned as a borrow, and only an old-shaped one is copied
  and rewritten (decision #104).

**Why this way.** Anyone with the data can check the log, with no trusted server (decision #25);
every write audits the record it re-read (decision #55).

Alternatives: an `updated_at` column can be forged. A database trigger alone is invisible to the
application and differs between SQLite and Postgres. A signed log needs a key to protect, and is
better when the checker must not trust the database owner.

**Read more.**

- [docs.rs: sha2](https://docs.rs/sha2)
- [docs.rs: serde_json](https://docs.rs/serde_json)
- [std: `Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)

**Try it.** [Exercise 4](../../exercises/src/ex04_audit_chain.rs) builds and verifies a chain.

## 11. Tests next to the code, and property tests

**The idea.** A unit test is a function marked `#[test]`. It normally sits in the file it tests,
in a module marked `#[cfg(test)]` that is compiled only for `cargo test`; as a child module it
can see its parent's private items. A property test states a rule that must hold for any input,
and the tool generates many inputs to try to break it.

**In our code.**

- `crates/domain/src/permissions.rs`: `#[cfg(test)] mod tests` with a `const ACTIONS` array
  and loops over every role and every action, so the whole table is checked cell by cell.
- `crates/domain/src/audit.rs`: a `proptest!` block builds arbitrary chains, edits one entry
  anywhere, and requires `verify_chain` to find it.
- `crates/web/tests/common/mod.rs::TestApp`: the web crate's integration tests (Note 03).

**Why this way.** A change and its test sit in one diff. Example tests pin down the cases we
thought of; property tests find the others.

Alternatives: only example tests miss edge cases. A `tests/` folder for everything cannot reach
private functions, because an integration test is a separate crate that sees only the public API.

**Read more.**

- [The Rust Book: how to write tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html)
- [The Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [The Proptest book](https://proptest-rs.github.io/proptest/intro.html)

## 12. Formatting, lints and the safety net

**The idea.** Some mistakes are cheaper to catch with a tool than with attention: a formatter
ends layout arguments, a linter flags code that compiles but is probably wrong, and a secret
scanner stops a password from entering the history.

**In our code.**

- `rustfmt.toml`: sets the edition. `rust-toolchain.toml` pins the stable channel with the
  `rustfmt` and `clippy` components.
- `Cargo.toml` (`[workspace.lints]`): `unsafe_code = "forbid"` refuses `unsafe` everywhere,
  and, unlike `deny`, it cannot be switched off by an `allow` in a file. Clippy's `all` group is
  set to `warn` with `priority = -1`, so that any single lint can still be overridden.
- `.github/workflows/ci.yml`, job `check`: `cargo fmt --all --check`, clippy with
  `RUSTFLAGS: -D warnings` (every warning fails the build), and
  `cargo test --workspace --locked` (`--locked` fails if the lockfile would change).
- `deny.toml`: licence and advisory rules, run by `.github/workflows/security.yml`.
- `.githooks/pre-commit` runs gitleaks with `.gitleaks.toml`, and a commit that contains a
  secret is refused (decision #37).

**Why this way.** These checks cost nothing and run on every change (decision #36).

Alternatives: review alone misses things when tired; a paid scanner adds cost, and the free tools
cover the same ground here.

**Read more.**

- [Cargo reference: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html), which
  describes the `[workspace.lints]` table
- [The Clippy lint list](https://rust-lang.github.io/rust-clippy/master/index.html)

## Check yourself

1. Why does `Level::new` return a `Result` instead of panicking?
2. What would break if the field of `Level` were `pub`? What would still be checked, and what
   would not?
3. Why does `can` have no `_ =>` arm on the role, and what happens to the build when a fourth
   role is added?
4. What does `$crate` do inside `str_enum!`, and why is it written that way?
5. What would happen to the old audit entries if `write_canonical` sorted keys differently from
   before?
6. Why does `FakeClock` hold a `Mutex`, and why are `Send + Sync` written on the `Clock` trait?

## Where next

Next: [02 — Storage](02-storage.md).
Exercises for this note: [1, newtypes](../../exercises/src/ex01_newtypes.rs),
[2, permissions](../../exercises/src/ex02_permissions.rs),
[3, the macro](../../exercises/src/ex03_text_enum.rs) and
[4, the audit chain](../../exercises/src/ex04_audit_chain.rs).
