# 02 — Storage

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 5 (a repository trait), 6 (transactions and Drop), 7 (error types and the ? operator). Reading time: about
> 50 minutes, plus the exercises.

## What you will learn

- How a trait lets the domain say what it needs while two database crates provide it.
- How native `async fn` in traits works, and why the trait spells out `impl Future + Send`.
- How an associated type and a method that takes `self` shape a transaction.
- How ownership and `Drop` undo a half-finished write without any code on the error path.
- How `Result`, `?` and `From` keep one backend's errors out of the domain.
- How sqlx checks SQL at build time, and how one test suite checks two databases.
- How a generic function (`record<T: Audited>`) serves every kind of record.

## The map

This note covers how the app talks to its database. The `domain` crate declares what storage must
do as traits, and two crates implement them. A third crate holds the tests both must pass.

- `crates/domain/src/repos.rs`: every storage trait, the error types and the `Store` bundle.
- `crates/storage-sqlite/src/lib.rs`: `SqliteStore`, used for local work and tests.
- `crates/storage-sqlite/src/tx.rs`: `SqliteTx`, the write transaction, and the audit entry it appends.
- `crates/storage-sqlite/src/error.rs` and `crates/storage-sqlite/src/convert.rs`: error mapping and value conversions.
- `crates/storage-postgres/src/lib.rs`: `PostgresStore`, the production backend (decision #77).
- `crates/storage-contract/src/lib.rs`: the shared test suite and the macro that runs it.
- `crates/storage-sqlite/migrations/` and `crates/storage-postgres/migrations/`: the schema.

## 1. Traits as the storage boundary

**The idea.** The rules of the app should not depend on which database holds the data. A trait is
a list of methods a type promises to have. The domain crate declares small traits; the database
crates implement them; the domain calls them through generics, so it never names SQLite or
Postgres. A function that is generic over `T: SomeTrait` is compiled once per concrete type
(static dispatch), with no lookup at run time.

**In our code.**

- `crates/domain/src/repos.rs::UserRepo` and `crates/domain/src/repos.rs::EventRepo` are read
  traits. `crates/domain/src/repos.rs::EventWrites` holds the writes that run inside a transaction.
- `crates/domain/src/repos.rs::Store` has a blanket impl: `impl<T> Store for T where T: ...`. Any
  type that implements all the small traits is a `Store` automatically, with no `impl` written.
- `crates/domain/src/logbook.rs::log_smell` is generic: `T: EventWrites + ReportWrites +
  ReadingWrites`. `crates/domain/src/sweep.rs::auto_close` asks for `S: Transactional<Tx:
  EventWrites + JobWrites> + JobRepo`: only what it uses.
- `crates/storage-sqlite/src/lib.rs::SqliteStore` and
  `crates/storage-postgres/src/lib.rs::PostgresStore` are the two implementations.

**Why this way.** The domain states what it needs and the database crates provide it. Many small
traits mean a function asks only for what it uses (the interface segregation idea), and a test
can implement one small trait. Generics were chosen over `dyn` (decisions #53 and #67): the
compiler builds one copy per backend, there is no vtable call and the async methods need no
boxing. The price is more compiled code and generic parameters that spread (`AppState<S>`,
`router<S>`).

Alternatives: `Box<dyn Store>` is simple to pass around, but a trait with async methods is not
object-safe without boxing, and every call becomes dynamic. One big trait would force every fake
to implement everything. An ORM (a library that maps rows to objects) hides the SQL that the
audit chain must control. `dyn` is the better choice when the type is only known at run time.

**Read more.**

- [The Rust Book: generic types, traits and lifetimes](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [The Rust Book: traits](https://doc.rust-lang.org/book/ch10-02-traits.html)
- [The Rust Book: trait objects](https://doc.rust-lang.org/book/ch18-02-trait-objects.html)

**Try it.** [Exercise 5](../../exercises/src/ex05_event_repo.rs) writes a small repository trait,
an in-memory store and a generic function over it.

## 2. Native `async fn` in traits

**The idea.** An `async fn` returns a future: a value that does the work later, when awaited.
Since Rust 1.75 a trait can declare `async fn` directly. Before that, the `async-trait` crate
rewrote each method to return a boxed future (a heap allocation per call).

**In our code.**

- `crates/domain/src/repos.rs::Transactional`: `begin` returns `impl Future<Output =
  StoreResult<Self::Tx>> + Send`. The `+ Send` is written out, so callers may hand the future to
  another thread, as the web server does.
- `crates/storage-sqlite/src/lib.rs::SqliteStore`: the impl writes `async fn begin`. That is
  allowed because the future it returns is `Send`.
- `crates/web/src/sessions.rs::DbSessions` implements tower-sessions' `SessionStore`, a trait
  from another crate that is declared with `async-trait`. The impl therefore carries
  `#[async_trait]` too: the one place it remains (decisions #53 and #56).

**Why this way.** A plain `async fn` in a trait does not promise `Send`, and generic code that
spawns the future needs that promise; `impl Future + Send` states it once, in the trait. No macro
and no boxing remain in our own traits.

Alternative: `async-trait` everywhere works on old compilers and when you need `dyn Trait`, at
the cost of one allocation per call. It is still needed when a foreign trait uses it, as here.

**Read more.**

- [Rust blog: async fn and return-position impl Trait in traits](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html)
- [The Rust Book: async and await](https://doc.rust-lang.org/book/ch17-00-async-await.html)
- [std: the `Future` trait](https://doc.rust-lang.org/std/future/trait.Future.html)
- [The Async Book](https://rust-lang.github.io/async-book/)

## 3. Associated types and the transaction object

**The idea.** An associated type is a type name that belongs to a trait and is chosen by each
implementor. Here every backend names its own transaction type, while generic code only says
"whatever transaction this store gives me". A transaction is a group of writes that all happen or
none does.

**In our code.**

- `crates/domain/src/repos.rs::Transactional`: `type Tx: Commit;` and
  `begin(&self, actor: Option<UserId>)`. The actor is who the audit entries will name.
- `crates/domain/src/repos.rs::Commit`: `fn commit(self)` takes `self` by value. After the call
  the transaction is gone, so a committed transaction cannot be used again. The compiler enforces it.
- `crates/domain/src/repos.rs::WriteTx` bundles the write traits (supertraits). In
  `crates/domain/src/repos.rs::Store`, `Transactional<Tx: WriteTx>` is an associated type bound:
  "the store's transaction can do every write".
- `crates/storage-sqlite/src/tx.rs::SqliteTx` and `crates/storage-postgres/src/tx.rs::PostgresTx`
  are the concrete types.
- Every write method lives on the transaction, not on the store, because every write appends its
  audit entry in the same transaction (decision #47).

**Why this way.** The associated type lets each backend have its own transaction while the
generic code stays backend-free. Putting writes on the transaction makes it hard to write
without the audit entry.

Alternative: a generic parameter on the trait, `Transactional<Tx>`, would let one store have
several transaction types, which we never want. That choice would suit a type that really can
produce more than one kind of the thing.

**Read more.**

- [The Rust Book: advanced traits](https://doc.rust-lang.org/book/ch20-02-advanced-traits.html),
  where associated types are explained
- [std: the `Send` trait](https://doc.rust-lang.org/std/marker/trait.Send.html)

## 4. Ownership at work: a transaction that rolls back when dropped

**The idea.** In Rust a value has one owner, and when the owner goes out of scope the value is
dropped: its `Drop` code runs. A value can therefore own a resource and release it automatically.
This is called RAII (Resource Acquisition Is Initialization). A transaction that rolls back in
`Drop` can never be left open by mistake.

**In our code.**

- `crates/domain/src/repos.rs::Transactional`: the module doc says that dropping a transaction
  without `commit` rolls it back.
- `crates/web/src/routes/entries.rs::submit`: on a refused entry it calls `drop(tx)` and shows the
  form again; only on success does it call `tx.commit().await?`.
- `crates/storage-sqlite/src/tx.rs::SqliteTx` holds a `sqlx::Transaction<'static, Sqlite>`, and
  that type's own `Drop` performs the rollback.
- Every write method takes `&mut self`, so two writes cannot overlap on one transaction.

**Why this way.** Tying "undo" to leaving a scope covers every path: an early `return`, a `?`
and a panic. In `submit` the explicit `drop(tx)` is not what makes the rollback happen (the end of
the function would do it); it frees the single write lock before the form is built. The
`begin` doc comment warns that a second `begin` on the same task waits for the first.

Alternatives: an explicit `rollback()` call is easy to forget on one error path. A closure style,
`store.transaction(|tx| ...)`, is also safe, but nests awkwardly with `async` code.

**Read more.**

- [The Rust Book: the `Drop` trait](https://doc.rust-lang.org/book/ch15-03-drop.html)
- [The Rust Book: references and borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [std: the `Drop` trait](https://doc.rust-lang.org/std/ops/trait.Drop.html)

**Try it.** [Exercise 6](../../exercises/src/ex06_transaction.rs) builds a ledger whose
transaction rolls back when dropped.

## 5. Errors: `Result`, `?` and `From`

**The idea.** A function that can fail returns `Result<T, E>`. The `?` operator returns early
with the error, and on the way it converts the error with `From::from`, so a function can pass on
an error of a different type if a conversion exists. The domain defines one error type for
storage, so callers never see a database library's error.

**In our code.**

- `crates/domain/src/repos.rs::StoreError`: `NotFound`, `Conflict`, `Invalid`, `Corrupt` and
  `Backend(Box<dyn Error + Send + Sync>)`, the last one marked `#[source]` to keep the cause.
  `#[from]` on `Conflict` and `Invalid` writes the `From` impls.
- `crates/domain/src/repos.rs::Conflict`: the data-rule failures a caller can act on, such as a
  taken username.
- `crates/storage-sqlite/src/error.rs::IntoStore` is an extension trait: it adds `.db()` to
  `Result<T, sqlx::Error>`. `crates/storage-sqlite/src/error.rs::into_store_error` maps a unique,
  foreign key or check violation to a `Conflict`.
- `crates/storage-sqlite/src/error.rs::unique_conflict` decides by the message text. The Postgres
  twin in `crates/storage-postgres/src/error.rs` uses the constraint name instead.
- `crates/storage-sqlite/src/convert.rs::corrupt` builds a `Corrupt` error.
- `crates/web/src/error.rs::AppError` has its own `From<StoreError>`, which is why `submit` can
  write `?` on a store call.

**Why this way.** Callers must not depend on `sqlx::Error`: it differs between backends and would
leak the database into the domain. A conflict the user can fix ("that username is taken") is a
different thing from a broken disk, and an enum lets the caller `match` on it.

Alternatives: returning `sqlx::Error` leaks the backend. `anyhow::Error` in a library gives
callers nothing to match on (it suits the final application, as in note 01). Matching on message
text everywhere is fragile; we do it once, in `unique_conflict`, and the weakness is named there:
the text is SQLite's own and could change.

**Read more.**

- [The Rust Book: recoverable errors with `Result`](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [The Rust Book: to panic or not to panic](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html)
- [std: the `From` trait](https://doc.rust-lang.org/std/convert/trait.From.html)
- [std: the `Error` trait](https://doc.rust-lang.org/std/error/trait.Error.html)

**Try it.** [Exercise 7](../../exercises/src/ex07_store_errors.rs) writes a small error type with
`From` impls and uses `?` across them.

## 6. SQL checked at compile time with sqlx

**The idea.** SQL is normally a string that is only checked when it runs. sqlx's `query!` macros
check the string while compiling: they learn from the database whether the SQL is valid and what
type each column has, and they fail the build if the code disagrees.

**In our code.**

- `crates/storage-sqlite/src/audit.rs::fetch_entries` uses `sqlx::query_as!` with a row struct
  and a fixed query. `crates/storage-sqlite/src/tx.rs::SqliteTx` uses `sqlx::query!` in
  `record`.
- Asking a live database on every build would be slow, so the answers are saved as JSON in
  `crates/storage-sqlite/.sqlx/` and `crates/storage-postgres/.sqlx/`. `.cargo/config.toml` sets
  `SQLX_OFFLINE = "true"`, so a normal build needs no database.
- `scripts/sqlx-prepare.sh` regenerates those files from freshly migrated databases. The CI job
  `sqlx` in `.github/workflows/ci.yml` runs it with `--check`, so stale files fail the build.
- `crates/storage-sqlite/sqlx.toml` names the environment variable that holds this crate's
  database URL, so two crates can check against two databases (decisions #45 and #52).

**Why this way.** A typo or a wrong column type becomes a compile error, not a failure in
production. The price is the saved data that must be committed and kept current.

Alternatives: `sqlx::query` with a runtime string gives no build-time check. An ORM such as Diesel
or SeaORM is a larger API to learn and a model layer that the audit chain would have to bypass.
Using `rusqlite` and `tokio-postgres` by hand means learning two APIs.

**Read more.**

- [sqlx on docs.rs](https://docs.rs/sqlx)
- [sqlx: the `query!` macro](https://docs.rs/sqlx/latest/sqlx/macro.query.html)

## 7. Migrations, and two databases that must agree

**The idea.** A migration is a numbered SQL file that moves the schema one step forward. The
database remembers which ones ran, so each runs once and in order. The schema is code: it is
versioned, reviewed and tested like the rest.

**In our code.**

- `crates/storage-sqlite/migrations/0001_initial.sql` uses `CHECK` constraints and the partial
  unique index `events_one_open`, which lets only one event be open at a time (decision #49).
- `crates/storage-sqlite/migrations/0004_checks.sql`: SQLite cannot drop `NOT NULL` from a
  column, so the migration rebuilds three tables by copying, dropping and renaming (decision
  #109). The Postgres twins sit in `crates/storage-postgres/migrations/`.
- `crates/storage-sqlite/src/lib.rs::SqliteStore::migrate`: `sqlx::migrate!("./migrations")`
  embeds the files in the binary, so a deployed app needs no loose SQL files.
- `crates/storage-postgres/tests/schema_parity.rs::family` and the test after it compare the
  column names, type families and nullability that the two migration sets produce. It does not
  compare constraints or indexes: those are covered by the contract tests.

**Why this way.** Two dialects drift unless something compares them, and a parity test is the
cheapest way to catch "I changed one backend and forgot the other".

Alternatives: SQL kept by hand in a README is never checked. One migration set with conditional
SQL would not work, because the dialects differ too much (see 0004).

**Read more.**

- [sqlx on docs.rs](https://docs.rs/sqlx), which documents the `migrate!` macro
- [SQLite: transactions](https://www.sqlite.org/lang_transaction.html)

## 8. One contract, two backends: a test macro

**The idea.** Both backends must behave the same, so the same tests should run against both. The
tests are written once, as plain async functions that take a `Fixture`. Each backend crate
generates one `#[tokio::test]` per function with a macro.

**In our code.**

- `crates/storage-contract/src/lib.rs::contract_tests` is a `#[macro_export] macro_rules!`. An
  internal `@each` rule expands one test per listed function, calling `$crate::$module::$test`.
- `crates/storage-contract/src/fixture.rs::Fixture` is a fresh migrated database, a `FakeClock`
  and a raw SQL runner (`crates/storage-contract/src/fixture.rs::raw_runner`), so a test can tamper
  with the data the way an attacker with database access could.
- `crates/storage-sqlite/tests/contract.rs::fixture` and `crates/storage-postgres/tests/contract.rs`
  each end by calling `smoke_storage_contract::contract_tests!(fixture)`.
- `crates/storage-postgres/tests/common/mod.rs::fresh_database` returns `None`, which skips the
  test, when `TEST_POSTGRES_URL` is unset on a developer machine. When `CI` is set it panics, so CI
  can never skip the Postgres tests silently (decisions #40 and #45).

**Why this way.** "SQLite and Postgres agree" is checked, not hoped for.

Alternatives: copying the tests per backend lets them drift. A trait-object test harness is
awkward because of the async methods (section 2). Testing only SQLite and trusting Postgres is
exactly the risk decision #40 exists to remove.

**Read more.**

- [The Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [The Rust Reference: macros by example](https://doc.rust-lang.org/reference/macros-by-example.html)

## 9. Writers and readers: SQLite pools and Postgres locks

**The idea.** Every audit entry stores the hash of the one before it, so two writers working at
once would both link to the same previous entry and split the chain. Writes must therefore run
one at a time. Each database gets there its own way.

**In our code.**

- `crates/storage-sqlite/src/lib.rs::SqliteStore::connect` opens one writer connection
  (`max_connections(1)`) in WAL mode, plus a separate read-only pool of four. WAL lets readers
  work while the writer writes. In-memory databases are refused: writer and readers must share a
  file.
- `crates/storage-sqlite/src/lib.rs::SqliteStore` `begin` runs `BEGIN IMMEDIATE`, taking the
  write lock at the start. A second process, such as the CLI, then waits instead of failing
  late.
- `crates/storage-postgres/src/lib.rs::PostgresStore` `begin` sets `SET LOCAL lock_timeout` and
  then `LOCK TABLE audit_log IN EXCLUSIVE MODE`, which blocks other writers but not readers.
- `crates/storage-contract/src/audit.rs::concurrent_writers_never_fork_the_chain` starts eight
  writers at once and verifies the chain (decision #48).

**Why this way.** Serialising at the database also protects against a second process, which a
Rust lock cannot.

Alternatives: one global `Mutex` in Rust does not stop the CLI from writing at the same time.
Optimistic retry on a unique-index failure would work; the `ChainForked` conflict exists as the
backstop, but waiting is simpler than retrying.

**Read more.**

- [SQLite: write-ahead logging](https://www.sqlite.org/wal.html)
- [SQLite: transactions](https://www.sqlite.org/lang_transaction.html)
- [PostgreSQL: the LOCK command](https://www.postgresql.org/docs/current/sql-lock.html)

## 10. A generic helper: `record<T: Audited>`

**The idea.** A generic function with a trait bound works for every type that implements the
trait. Here one function audits every kind of record, and the trait tells it what it needs to
know about each one.

**In our code.**

- `crates/domain/src/records.rs::Audited` has an associated `const ENTITY`, a required
  `audit_id` and a provided `snapshot`, which serialises the record to JSON.
- `crates/storage-sqlite/src/tx.rs::SqliteTx` `record` is generic over `T: Audited`. It uses
  `T::ENTITY` and the snapshots; an `update` that changed nothing writes no entry.
- The verifying side is the same idea: `crates/domain/src/verify.rs::Snapshot` has `Snapshot::of`
  with the same bound, and `crates/domain/src/verify.rs::verify_store` compares the log to the
  stored records.

**Why this way.** Adding a record type means implementing a few small items, and the audit code
stays unchanged.

Alternative: an enum of all records and a `match` means every new record edits one central
function. The enum is the better choice when the set of kinds is closed and small.

**Read more.**

- [The Rust Book: generic data types](https://doc.rust-lang.org/book/ch10-01-syntax.html)

## 11. Small conversions that keep the database honest

**The idea.** The database has loose types (any integer, any text); the domain has strict ones.
The boundary is where bad data is caught. Our code refuses to read corrupt data rather than show
it.

**In our code.**

- `crates/storage-sqlite/src/convert.rs::micros`, `crates/storage-sqlite/src/convert.rs::instant`
  and `crates/storage-sqlite/src/convert.rs::opt_instant` turn instants into integer
  microseconds and back (decision #46). `opt_instant` is `Option::map` followed by `transpose`,
  which turns an `Option<Result>` into a `Result<Option>`.
- `crates/storage-sqlite/src/convert.rs::parse` is generic over `FromStr<Err = DomainError>`.
- `crates/storage-sqlite/src/convert.rs::canonical` fails when a stored value is not already in
  its normalised form, which means someone edited the data by hand.
- `crates/storage-sqlite/src/reports.rs::assemble` uses `u8::try_from(level).map_err(corrupt)?`
  and collects an iterator of `Result`s into one `Result`.
- `crates/storage-sqlite/src/audit.rs::json` uses `transpose` the same way.

**Why this way.** Failing loudly on a bad row is better than showing wrong data.

Alternative: `as` casts never fail, so an out-of-range number silently becomes a wrong one.

**Read more.**

- [std: the `TryFrom` trait](https://doc.rust-lang.org/std/convert/trait.TryFrom.html)
- [std: the `Iterator` trait](https://doc.rust-lang.org/std/iter/trait.Iterator.html)

## Check yourself

1. Why does `Commit::commit` take `self` and not `&mut self`?
2. What does `drop(tx)` do in `submit`, and what happens if that line is removed and the
   function returns an error?
3. Why do the storage traits return `impl Future + Send` instead of using `async fn` directly?
4. Where would a third backend plug in, and what would the contract tests ask of it?
5. Why does the SQLite `unique_conflict` match on text, and what would break if SQLite changed
   its message?
6. What does the blanket `impl<T> Store for T` buy, and what would you write without it?

## Where next

Next: [03 — Web and auth](03-web-and-auth.md).
Exercises for this note: [5, a repository trait](../../exercises/src/ex05_event_repo.rs),
[6, a transaction that rolls back](../../exercises/src/ex06_transaction.rs) and
[7, store errors](../../exercises/src/ex07_store_errors.rs).
