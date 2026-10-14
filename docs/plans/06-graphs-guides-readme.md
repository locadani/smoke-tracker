# Milestone 6: Graphs, guides & README — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Everyone signed in gets a Graphs screen (intensity per person with PM2.5, events per
day or week, a room × hour heatmap, symptom frequency, inside vs common areas) over what they
may see; Settings → Guides gains *How to log*; the README becomes the project's front page with
the spec's tables; CI gains CodeQL and a coverage report. The Postgres tests stop leaving one
database per test behind.

**Architecture:** Same layering as milestones 3–5. `smoke-domain` gets a pure module
`graphs` (a `Period` of one local month or year, and plain functions from reports to series)
and `DailySchedule::year_span`. `smoke-web` adds `/graphs` (one route module, one template, a
vendored Chart.js and a small `static/graphs.js` that reads each chart's JSON from a
`data-chart` attribute), `/guide/logging`, and moves month parsing into `time.rs`. Storage is
untouched: **no migration, no new query, no `.sqlx` change**. CI gains `codeql.yml` and a
`coverage` job in `ci.yml`.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, askama 0.16, fluent-templates 0.15,
chrono + chrono-tz, serde_json, sqlx 0.9 (tests only here). **No new crate.** One vendored
JavaScript library: **Chart.js 4.5.1** (MIT, npm `chart.js`, `dist/chart.umd.min.js`, SHA-256
`48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a`). CI: `github/codeql-action`
v4, `taiki-e/install-action` v2.87.28 (commit-pinned), `cargo-llvm-cov`, `actions/upload-artifact` v7.

**Spec:** `docs/specs/2026-10-13-smoke-tracker-design.md` §4 (graphs: residents and viewers all
data, guests own data; the table is published in the README), §5 (How to log, rules of thumb,
intensity guide; shown in-app under Settings → Guides and in the README), §9 screen 5 (the five
graphs, Chart.js self-hosted, guests own data), §9 screen 7 (Guides), §12 (CSP `'self'` only),
§13.1 (`ci.yml` coverage with `cargo-llvm-cov`; `codeql.yml` "kept only if Rust analysis is
stable"). Decisions: `docs/DECISIONS.md` rows **#123–#130**, each appended by the task that
makes it. Previous plans: `docs/plans/05-people-and-pwa.md`, `docs/plans/04b-room-levels.md`.

**Provenance:** every code block in this plan was compiled and tested before the plan was
written, as one commit per task on the branch **`m6-proto`** (worktree
`/Users/daniele/Documents/Personal/smoke-tracker-m6proto`, from `main` at `a39bbaa`; commits
`dcfd028`, `78815bb`, `74447c7`, `da4584a`, `47aef00`, `a25cea6`, `a5eef4b` = Tasks 1–7). At
every task boundary `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D
warnings` and `cargo test --workspace` passed; with `TEST_POSTGRES_URL` set after Tasks 1 and 7
(no other task touches storage). `actionlint` passed on both workflows after Task 6. The
rendered Graphs page was checked once in headless Chrome under the production CSP (all charts
drawn, no violation). Test totals with `TEST_POSTGRES_URL` set: 566 at `a39bbaa`, then 567, 577,
586, 589, 593, 593, **594** at the head. Executors may consult that branch (e.g. `git -C
…/smoke-tracker-m6proto show 74447c7`); this plan is the source of truth. **Not verified
locally:** the `coverage` and `codeql` jobs themselves (no `cargo-llvm-cov` or CodeQL CLI on
the machine); the first CI run of the pull request verifies them.

## Decisions for the user to confirm

Defaults were chosen so nothing blocks; each is a decision row and easy to change.

1. **What the graphs count** (decision #123): one local **month per day** or one local **year
   per week** (Monday–Sunday); an event counts once, on the day of its earliest visible report
   (the summary card's rule, #94); the heatmap cell is the room's **average** level at that
   hour (clean checks pull it down); inside vs common shows each area's **highest** level per
   day or week; symptoms count reports.
2. **The heatmap is an HTML table**, coloured by CSS classes, not a Chart.js chart (decision
   #126): no second library (`chartjs-chart-matrix`), no inline styles, readable without
   JavaScript.
3. **Chart.js 4.5.1 is vendored** (decision #124): about 200 kB more in the binary and image;
   loaded only by the Graphs page.
4. **Guides stay at `/guide/rating` and `/guide/logging`**, listed under Settings → Guides
   (decision #127); the popup's "Full guide →" link is unchanged.
5. **CodeQL is not a required check** and **coverage has no threshold** (decisions #128,
   #129). After the merge you can add them to the ruleset if you want them to block merges.
6. **37 new translation keys**: 37 Italian and 37 Turkish strings (74) are drafts for native
   review (Task 3: 12, Task 4: 8, Task 5: 17). The English *How to log* texts are the spec's
   words; the Italian and Turkish ones follow the existing "tu"/"sen" tone of the rating guide.
7. **No database migration and no new query**: nothing changes on the live Neon database.

## Questions for the user

Sources (spec, decisions, code) do not settle these; the plan takes the smallest option and
keeps the question open.

1. **A "No smell" check for the whole home** (no rooms listed, spec §7.1) is a 0 on the
   intensity line, but it is **left out of the room × hour heatmap and the inside vs common
   chart**, because it names no room. Should it count as every *inside* room at 0 (the
   stairwell is a common area, not "home")? Changing it is one function each in
   `smoke_domain::graphs` (`room_hours`, `area_levels`) plus their tests.
2. **The heatmap is a table, not Chart.js.** Spec §9 lists it under "Graphs (Chart.js,
   self-hosted)". The wording proposal is in `.superpowers/M6-spec-proposals.md` (not part of
   this plan's commits); the plan keeps the table either way.

## Global Constraints

- Personal project. Before every commit, `git config user.name` must print `locadani` and
  `git config user.email` must print locadani's GitHub noreply address (the repository's local
  config already holds it). Never put any email address in a tracked file (the README included).
- Never commit secrets. The gitleaks pre-commit hook (`git config core.hooksPath .githooks`) must
  be active. No passwords in connection strings: local and CI Postgres use trust authentication.
  No workflow may need a new secret or token (`GITHUB_TOKEN` permissions only).
- Never modify `description.md` or `CLAUDE.md`. **Never edit
  `docs/specs/2026-10-13-smoke-tracker-design.md`** without the user's explicit approval; no task
  touches it.
- Every commit message ends with a blank line followed by exactly
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Commit subjects use Conventional Commits and name the Rust concept where there is one.
- No tutorial-style comments in code. Doc comments explain *what* and *why*.
- Workspace-wide: `unsafe_code = "forbid"`, clippy warnings are errors in CI, `cargo fmt`
  enforced.
- `smoke-web` never sees SQL and depends on a storage crate only as a dev-dependency. The graph
  arithmetic lives in `smoke_domain::graphs` (pure, no I/O); the web layer only fetches,
  filters with `can_see` and turns series into Chart.js JSON.
- **No migration and no query change in this milestone**; `0001`–`0004` and every `.sqlx/`
  file stay untouched, `schema_parity` keeps passing. Production is live on Neon with real data.
- Free tier and a small footprint. **No new crate**; `Cargo.lock` does not change. The only new
  third-party code is the vendored `crates/web/static/chart-4.5.1.umd.min.js`, byte for byte the
  npm release (SHA-256 above, pinned by `the_vendored_libraries_are_the_published_releases`).
- Content-Security-Policy is `'self'` only. No template may contain inline `<script>` code,
  `<style>`, `style=` attributes, `on*=` handlers, `hx-on` or `javascript:` URLs. The only
  scripts are the vendored htmx and Chart.js, `static/app.js`, `static/graphs.js` and
  `static/sw.js`. Chart data travels in `data-chart` attributes (escaped by Askama), never in a
  script block. Every new page is added to `no_page_needs_inline_scripts_styles_or_eval`.
- Every "now" goes through `smoke_domain::clock::Clock`; local days, hours, months and years go
  through `DailySchedule` and `APP_TIMEZONE`, also in the browser (`Intl.DateTimeFormat` with
  the zone the page sends, never the browser's own).
- Guests see graphs of **their own** reports and readings only and never another person's
  name (spec §4, decision #92); everything shown passes `can_see`.
- Every new translation key exists in `en`, `it` and `tr` (the existing test fails otherwise);
  keys passed through variables are listed in the module's `KEYS`/`keys()` for
  `every_key_the_pages_use_is_translated`. Italian and Turkish texts are drafts for native review.
- The README's intensity and *How to log* tables equal the English translation texts
  (`the_readme_tables_match_the_english_guides`); edit both sides together.
- Production runs on Cloud Run with **max-instances = 1** and `CLIENT_IP_HEADER=X-Forwarded-For`.
- Postgres tests need `TEST_POSTGRES_URL`; locally the container `smoke-pg` listens on **port
  55432** (`postgres://postgres@localhost:55432/postgres`). From Task 1 on, each test drops its
  own database; databases left by earlier runs stay until someone drops them (the planner did
  not: 620 `smoke_test_%` databases were on the server at planning time). If tests fail with
  `could not resize shared memory segment … No space left on device`, run `docker restart
  smoke-pg` and wait for `docker exec smoke-pg pg_isready`.
- `docs/DECISIONS.md` is append-only; this plan adds rows **#123–#130**.
- Disk: all builds share one `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target`;
  never create another target folder, never run `cargo clean` or `rm -rf` on build folders (the
  user does that). Check `df -h /System/Volumes/Data` before big builds.

## Review Focus

1. **A guest learning about other people through the graphs**: another person's name, report,
   reading or event in the JSON, the heatmap or a count. Expected: only the guest's own data,
   and a line saying so. Tests: `guests_see_graphs_of_their_own_reports_only` (Task 3, extended
   in Task 4 with the heatmap row).
2. **A chart that needs an inline script or style, or a CDN** (Chart.js config in a script
   block, a `style=` colour on a heatmap cell, a library fetched from elsewhere). Expected:
   every page passes the CSP scan; the only new scripts are `/static/…`. Tests:
   `no_page_needs_inline_scripts_styles_or_eval` (extended in Tasks 3 and 5),
   `the_vendored_libraries_are_the_published_releases`,
   `every_file_in_the_static_folder_is_served` (Task 3).
3. **Times and days in the wrong zone**: a report at 23:30 UTC counted on the UTC day, an hour
   of the heatmap off by one in summer, axis labels in the browser's zone, weeks of a year
   starting on Sunday or missing the days before the first Monday. Expected: local days, local
   hours, Monday weeks, `Europe/Rome` handed to the browser. Tests:
   `an_event_counts_once_on_the_local_day_of_its_first_report`,
   `the_heatmap_averages_levels_per_room_and_local_hour_counting_clean_checks`,
   `a_year_is_counted_per_week_from_monday` (Task 2);
   `residents_see_one_line_per_person_and_pm25_on_its_own_axis`,
   `a_year_counts_events_per_week_and_links_back_to_a_month` (Task 3).
4. **A hand-typed address** (`?month=2026-13`, `?year=99999`, `?year=1999`, both parameters)
   making the page fail or overflow chrono. Expected: the current month, status 200. Tests:
   `the_address_picks_a_month_or_a_year_and_falls_back_to_this_month` (Task 3, unit),
   `an_unusable_address_shows_this_month_and_an_empty_one_says_so` (Task 3),
   `months_in_addresses_are_read_within_the_supported_years` (Task 3, unit).
5. **The README and the app drifting apart** (an edited intensity description or rule in one
   place only). Expected: a failing unit test. Test:
   `the_readme_tables_match_the_english_guides` (Task 7).

## Out of scope (recorded so nobody hunts for it)

- **History shows raw event ids** ("event 3 → 5", deferred since M4): it lives on the report
  page, which this milestone does not touch. Still deferred.
- **Deferred M5 minors** (`smoke-tracker-m5/.superpowers/sdd/05-people-and-pwa/final-review.md`,
  "can wait"): none touches a file this milestone changes (`people.rs`, `expiry.rs`, `sw.js`,
  `app.js`, `accept.rs`, the invite storage), so none is taken. Earlier minors also stay open:
  no `Retry-After` on 429, the 500/panic page untested, base images pinned by tag not digest,
  the Trivy PR build uncached (`security.yml` is not touched).
- **ADRs** (`docs/adr/`, spec §15): the decision log carries the reasoning; the learning pack
  (milestone 8) is the place for longer write-ups.
- Data export, push reminders, offline logging (spec §14).

## File Structure

```
smoke-tracker/
├─ .gitattributes                         vendored *.min.js left out of language statistics
├─ .github/workflows/ci.yml               + coverage job (cargo-llvm-cov)
├─ .github/workflows/codeql.yml           NEW  Rust, actions, javascript-typescript
├─ README.md                              rewritten (front page, tables, architecture, CI)
├─ docs/DECISIONS.md, plans/ROADMAP.md    + rows #123–#130; roadmap links this plan
└─ crates/
   ├─ domain/src/
   │  ├─ graphs.rs     NEW  Period, intensity_lines, events_per_bucket, room_hours,
   │  │                     symptom_counts, area_levels
   │  ├─ schedule.rs   year_span
   │  └─ lib.rs        pub mod graphs
   ├─ storage-postgres/tests/
   │  ├─ common/mod.rs TestDatabase: dropped with its handle
   │  └─ contract.rs, schema_parity.rs   hold the handle
   └─ web/
      ├─ src/routes/graphs.rs    NEW  /graphs
      ├─ src/routes/guide.rs     /guide/logging, situations(), rules(), README test
      ├─ src/time.rs             SUPPORTED_YEARS, parse_month, day_month (from events.rs)
      ├─ src/{lib,assets,i18n}.rs, src/routes/{mod,events}.rs
      ├─ templates/              graphs.html, guide_logging.html (NEW); base.html (scripts
      │                          block, Graphs link), guide.html, settings.html
      ├─ static/                 chart-4.5.1.umd.min.js, graphs.js (NEW); app.css
      ├─ locales/{en,it,tr}/main.ftl   +37 keys each
      └─ tests/                  graphs.rs, guides.rs (NEW); security.rs, common/mod.rs
```

**Dependency direction** is unchanged: `smoke-tracker → smoke-web → smoke-domain ← storage-*`.

**Why these seven tasks:** test infrastructure first, so every later Postgres run stops
growing the server (Task 1); the arithmetic before any page (Task 2); the Graphs screen in two
reviewable halves, the Chart.js plumbing with two charts, then the remaining three graphs
(Tasks 3–4); the guide (Task 5); CI (Task 6) before the README that describes it (Task 7).

---


### Task 1: Postgres tests drop the database they create

Carry-over (handoff): every Postgres test creates `smoke_test_<pid>_<micros>_<n>` and never
drops it, so the local `smoke-pg` server collects thousands of databases until it runs out of
shared memory. `fresh_database()` now returns a `TestDatabase` handle whose `Drop` runs
`DROP DATABASE IF EXISTS … WITH (FORCE)` on its own thread and current-thread Tokio runtime (the
test's runtime may be the one dropping it and cannot block on itself; a failure is printed,
never a panic inside `drop`). The contract fixture passes the handle as its `keep_alive`, which
`Fixture` drops last, after the store's pools; `WITH (FORCE)` closes any connection still open.
Test-only: no production code changes, no decision row.

**Files:**
- Modify: `crates/storage-postgres/tests/common/mod.rs`, `crates/storage-postgres/tests/contract.rs`, `crates/storage-postgres/tests/schema_parity.rs`

**Interfaces:**
- Consumes: `smoke_storage_contract::fixture::Fixture::new(store, clock, raw, drop_guard,
  keep_alive)` (milestone 2), whose fields drop in declaration order (`_keep_alive` last).
- Produces (test helper, `crates/storage-postgres/tests/common/mod.rs`):
  `pub struct TestDatabase { pub url: String, .. }` with `impl Drop`, and
  `pub async fn fresh_database() -> Option<TestDatabase>` (was `Option<String>`).
  Test `a_test_database_is_dropped_with_its_handle`.

- [ ] **Step 1: Write the failing tests**

The whole change is test code; the new test is in the diff below together with the helper it needs.

````diff
diff --git a/crates/storage-postgres/tests/contract.rs b/crates/storage-postgres/tests/contract.rs
index 4893d5e..0042914 100644
--- a/crates/storage-postgres/tests/contract.rs
+++ b/crates/storage-postgres/tests/contract.rs
@@ -9,13 +9,15 @@ use smoke_storage_postgres::PostgresStore;
 use sqlx::PgPool;
 
 async fn fixture() -> Option<Fixture<PostgresStore>> {
-    let url = common::fresh_database().await?;
+    let database = common::fresh_database().await?;
     let clock = Arc::new(FakeClock::new(start()));
-    let store = PostgresStore::connect(&url, clock.clone())
+    let store = PostgresStore::connect(&database.url, clock.clone())
         .await
         .expect("connect");
     store.migrate().await.expect("migrate");
-    let pool = PgPool::connect(&url).await.expect("raw connection");
+    let pool = PgPool::connect(&database.url)
+        .await
+        .expect("raw connection");
     let raw = raw_runner(move |sql| {
         let pool = pool.clone();
         async move {
@@ -28,7 +30,7 @@ async fn fixture() -> Option<Fixture<PostgresStore>> {
     });
     let drop_guard = "DROP TRIGGER audit_log_append_only ON audit_log; \
          DROP TRIGGER audit_log_no_truncate ON audit_log;";
-    Some(Fixture::new(store, clock, raw, drop_guard, ()))
+    Some(Fixture::new(store, clock, raw, drop_guard, database))
 }
 
 smoke_storage_contract::contract_tests!(fixture);
@@ -39,24 +41,46 @@ fn the_postgres_store_is_a_complete_store() {
     assert_store::<PostgresStore>();
 }
 
-/// A fresh database migrated up to `version`, with `rows` written by hand.
-async fn database_at(version: i64, rows: &'static str) -> Option<(String, PgPool)> {
-    let url = common::fresh_database().await?;
-    let pool = PgPool::connect(&url).await.unwrap();
+#[tokio::test]
+async fn a_test_database_is_dropped_with_its_handle() {
+    let Some(database) = common::fresh_database().await else {
+        return;
+    };
+    let still_connected = PgPool::connect(&database.url).await.unwrap();
+    let (_, name) = database.url.rsplit_once('/').unwrap();
+    let name = name.to_owned();
+    drop(database);
+    let admin = PgPool::connect(&std::env::var("TEST_POSTGRES_URL").unwrap())
+        .await
+        .unwrap();
+    let left: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_database WHERE datname = $1")
+        .bind(&name)
+        .fetch_one(&admin)
+        .await
+        .unwrap();
+    assert_eq!(left, 0, "{name} is still on the server");
+    drop(still_connected);
+}
+
+/// A fresh database migrated up to `version`, with `rows` written by hand. The database lives
+/// as long as the returned handle.
+async fn database_at(version: i64, rows: &'static str) -> Option<(common::TestDatabase, PgPool)> {
+    let database = common::fresh_database().await?;
+    let pool = PgPool::connect(&database.url).await.unwrap();
     sqlx::migrate!("./migrations")
         .run_to(version, &pool)
         .await
         .unwrap();
     sqlx::raw_sql(rows).execute(&pool).await.unwrap();
-    Some((url, pool))
+    Some((database, pool))
 }
 
 #[tokio::test]
 async fn migration_0003_gives_each_room_its_reports_intensity() {
-    let Some((url, _pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
+    let Some((database, _pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
         return;
     };
-    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
+    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
         .await
         .unwrap();
     store.migrate().await.unwrap();
@@ -65,14 +89,14 @@ async fn migration_0003_gives_each_room_its_reports_intensity() {
 
 #[tokio::test]
 async fn migration_0003_refuses_a_report_without_rooms_and_changes_nothing() {
-    let Some((url, pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
+    let Some((database, pool)) = database_at(2, migrations::ROWS_BEFORE_0003).await else {
         return;
     };
     sqlx::raw_sql(migrations::REPORT_WITHOUT_ROOMS)
         .execute(&pool)
         .await
         .unwrap();
-    let store = PostgresStore::connect(&url, Arc::new(SystemClock))
+    let store = PostgresStore::connect(&database.url, Arc::new(SystemClock))
         .await
         .unwrap();
     let error = store.migrate().await.unwrap_err();
diff --git a/crates/storage-postgres/tests/schema_parity.rs b/crates/storage-postgres/tests/schema_parity.rs
index f6be56c..8f54f7a 100644
--- a/crates/storage-postgres/tests/schema_parity.rs
+++ b/crates/storage-postgres/tests/schema_parity.rs
@@ -66,11 +66,11 @@ async fn postgres_columns(url: &str) -> Columns {
 
 #[tokio::test]
 async fn sqlite_and_postgres_schemas_have_the_same_columns() {
-    let Some(url) = common::fresh_database().await else {
+    let Some(database) = common::fresh_database().await else {
         return;
     };
     let sqlite = sqlite_columns().await;
-    let postgres = postgres_columns(&url).await;
+    let postgres = postgres_columns(&database.url).await;
     let only_sqlite: Vec<_> = sqlite.difference(&postgres).collect();
     let only_postgres: Vec<_> = postgres.difference(&sqlite).collect();
     assert!(
````

- [ ] **Step 2: Run the tests to verify they fail**

Run: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test -p smoke-storage-postgres`
Expected (before Step 3, with only the new test added): the crate fails to compile —
`fresh_database()` returns `Option<String>`, which has no field `url`.

- [ ] **Step 3: Implement**

````diff
diff --git a/crates/storage-postgres/tests/common/mod.rs b/crates/storage-postgres/tests/common/mod.rs
index c44de3e..f1b6ee3 100644
--- a/crates/storage-postgres/tests/common/mod.rs
+++ b/crates/storage-postgres/tests/common/mod.rs
@@ -1,15 +1,54 @@
-//! A fresh database per test on the server named by `TEST_POSTGRES_URL`.
+//! A fresh database per test on the server named by `TEST_POSTGRES_URL`, dropped again when the
+//! test is done with it.
 
 use std::sync::atomic::{AtomicU32, Ordering};
 use std::time::{SystemTime, UNIX_EPOCH};
 
-use sqlx::{AssertSqlSafe, PgPool};
+use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool};
 
 static NEXT: AtomicU32 = AtomicU32::new(0);
 
-/// The URL of a new, empty database, or `None` outside CI when no server is configured.
+/// A database that lives as long as this value. Dropping it drops the database and closes any
+/// connection still open to it, so the server does not keep one database per test ever run.
+pub struct TestDatabase {
+    pub url: String,
+    admin_url: String,
+    name: String,
+}
+
+impl Drop for TestDatabase {
+    /// Runs on a thread with its own runtime: the test's runtime may be the one dropping this
+    /// value and cannot wait on itself. A failure is reported, never a panic inside `drop`.
+    fn drop(&mut self) {
+        let admin_url = self.admin_url.clone();
+        let sql = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name);
+        let dropped = std::thread::spawn(move || -> Result<(), String> {
+            let runtime = tokio::runtime::Builder::new_current_thread()
+                .enable_all()
+                .build()
+                .map_err(|e| e.to_string())?;
+            runtime.block_on(async {
+                let mut admin = PgConnection::connect(&admin_url)
+                    .await
+                    .map_err(|e| e.to_string())?;
+                sqlx::raw_sql(AssertSqlSafe(sql))
+                    .execute(&mut admin)
+                    .await
+                    .map_err(|e| e.to_string())?;
+                admin.close().await.map_err(|e| e.to_string())
+            })
+        })
+        .join()
+        .unwrap_or_else(|_| Err("the thread dropping it panicked".to_owned()));
+        if let Err(error) = dropped {
+            eprintln!("could not drop test database {}: {error}", self.name);
+        }
+    }
+}
+
+/// A new, empty database, or `None` outside CI when no server is configured.
 /// `TEST_POSTGRES_URL` must end with a database name (e.g. `/postgres`) and have no query string.
-pub async fn fresh_database() -> Option<String> {
+pub async fn fresh_database() -> Option<TestDatabase> {
     let Ok(admin_url) = std::env::var("TEST_POSTGRES_URL") else {
         assert!(
             std::env::var_os("CI").is_none(),
@@ -38,5 +77,9 @@ pub async fn fresh_database() -> Option<String> {
     let (server, _) = admin_url
         .rsplit_once('/')
         .expect("TEST_POSTGRES_URL ends with /<database>");
-    Some(format!("{server}/{name}"))
+    Some(TestDatabase {
+        url: format!("{server}/{name}"),
+        admin_url,
+        name,
+    })
 }
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Then check that nothing leaks: note `docker exec smoke-pg psql -U postgres -tAc "select count(*)
from pg_database where datname like 'smoke_test_%'"` before and after a run of
`cargo test -p smoke-storage-postgres`; the number must not grow (the planner saw 620 before
and after 127 tests). With `TEST_POSTGRES_URL` set: **567** tests in the workspace.

- [ ] **Step 5: Commit**

```bash
git add crates/storage-postgres/tests/common/mod.rs \
  crates/storage-postgres/tests/contract.rs \
  crates/storage-postgres/tests/schema_parity.rs
git commit -F - <<'EOF'
test(storage-postgres): drop each test database when its handle goes out of scope

The Postgres tests created one database per test and never dropped it, so a local server
collected thousands of them until it ran out of shared memory. fresh_database now returns a
TestDatabase whose Drop impl removes the database (WITH (FORCE), on its own thread and
runtime); the contract fixture keeps it alive for the length of each test.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 2: Graph series in `smoke_domain::graphs`

Spec §9 screen 5 names five graphs; this task gives them their arithmetic as pure functions
(decision #123), so the web layer only fetches, filters by permission and draws. A `Period` is
one local month (buckets: days) or one local year (buckets: Monday-to-Sunday weeks, the first
named by the Monday before 1 January when the year starts mid-week). Every function takes
reports the caller has already limited to the period and to what the viewer may see.
`DailySchedule` gains `year_span`, the year analogue of `month_span`.

**Files:**
- Create: `crates/domain/src/graphs.rs`
- Modify: `crates/domain/src/lib.rs`, `crates/domain/src/schedule.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `DailySchedule::{month_span, local_date, tz}`, `SmellReport`, `ReportValues::{rooms,
  symptoms, intensity}`, `Room::area`, `Level`.
- Produces (`smoke_domain::graphs`):
  - `pub enum Period { Month(NaiveDate), Year(NaiveDate) }` (any date inside the month/year) with
    `span(&self, &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>)`, `first_day() -> NaiveDate`,
    `previous() -> Period`, `next() -> Period`, `buckets() -> Vec<NaiveDate>`,
    `bucket_of(NaiveDate) -> NaiveDate`.
  - `pub struct PersonLine { pub user: UserId, pub points: Vec<(DateTime<Utc>, Level)> }`;
    `pub fn intensity_lines(&[SmellReport]) -> Vec<PersonLine>` (by user id, points by time).
  - `pub fn events_per_bucket(&[SmellReport], Period, &DailySchedule) -> Vec<(NaiveDate, usize)>`.
  - `pub struct LevelTally { pub checks: u32, pub total: u32 }` with `mean() -> Option<f64>`;
    `pub struct RoomHours { pub room: RoomId, pub hours: [LevelTally; 24] }`;
    `pub fn room_hours(&[SmellReport], &DailySchedule) -> Vec<RoomHours>`.
  - `pub fn symptom_counts(&[SmellReport]) -> Vec<(SymptomId, usize)>` (most frequent first).
  - `pub struct AreaLevels { pub inside: Option<Level>, pub common: Option<Level> }`;
    `pub fn area_levels(&[SmellReport], &[Room], Period, &DailySchedule) -> Vec<(NaiveDate, AreaLevels)>`.
- `DailySchedule::year_span(&self, date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>)`.

- [ ] **Step 1: Write the failing tests**

All tests of this task are unit tests inside `crates/domain/src/graphs.rs` and
`crates/domain/src/schedule.rs`, so they are part of the Step 3 diff: write the `#[cfg(test)]`
modules first (`a_month_is_counted_per_day`, `a_year_is_counted_per_week_from_monday`,
`periods_step_back_and_forward_from_their_first_day`, `each_person_gets_a_line_in_time_order`,
`an_event_counts_once_on_the_local_day_of_its_first_report`, `a_year_counts_events_per_week`,
`the_heatmap_averages_levels_per_room_and_local_hour_counting_clean_checks`,
`symptoms_are_counted_per_report_most_frequent_first`,
`each_area_keeps_its_highest_level_per_bucket`,
`a_local_year_runs_from_new_years_midnight_to_the_next`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-domain --lib`
Expected: compile errors — `graphs` is not a module of `smoke_domain` and `year_span` does not exist.

- [ ] **Step 3: Implement**

````diff
diff --git a/crates/domain/src/graphs.rs b/crates/domain/src/graphs.rs
new file mode 100644
index 0000000..ee956e1
--- /dev/null
+++ b/crates/domain/src/graphs.rs
@@ -0,0 +1,466 @@
+//! The numbers behind the Graphs screen (spec §9 screen 5). Every function takes the reports the
+//! caller has already limited to one [`Period`] and to what the viewer may see, so guests get
+//! graphs of their own data only (spec §4).
+
+use std::collections::BTreeMap;
+
+use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Timelike, Utc};
+
+use crate::records::{Room, SmellReport};
+use crate::schedule::DailySchedule;
+use crate::types::{EventId, Level, RoomArea, RoomId, SymptomId, UserId};
+
+/// The stretch of local calendar time the graphs cover.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Period {
+    /// The local calendar month that contains this date, counted per day.
+    Month(NaiveDate),
+    /// The local calendar year that contains this date, counted per week (Monday to Sunday).
+    Year(NaiveDate),
+}
+
+impl Period {
+    /// `[start, end)` in UTC.
+    pub fn span(&self, schedule: &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>) {
+        match *self {
+            Self::Month(date) => schedule.month_span(date),
+            Self::Year(date) => schedule.year_span(date),
+        }
+    }
+
+    /// The first local day of the period.
+    pub fn first_day(&self) -> NaiveDate {
+        match *self {
+            Self::Month(date) => date.with_day(1).expect("every month has a first day"),
+            Self::Year(date) => date.with_ordinal(1).expect("every year has a first day"),
+        }
+    }
+
+    /// The period before this one.
+    pub fn previous(&self) -> Self {
+        let first = self.first_day();
+        match self {
+            Self::Month(_) => Self::Month(first - Months::new(1)),
+            Self::Year(_) => Self::Year(first - Months::new(12)),
+        }
+    }
+
+    /// The period after this one.
+    pub fn next(&self) -> Self {
+        self.after(self.first_day())
+    }
+
+    fn after(&self, first: NaiveDate) -> Self {
+        match self {
+            Self::Month(_) => Self::Month(first + Months::new(1)),
+            Self::Year(_) => Self::Year(first + Months::new(12)),
+        }
+    }
+
+    /// The first day of every bucket, in order: each day of a month; each week touching a year,
+    /// named by its Monday (the first may fall in December of the year before).
+    pub fn buckets(&self) -> Vec<NaiveDate> {
+        let first = self.first_day();
+        let end = self.after(first).first_day();
+        let step = match self {
+            Self::Month(_) => Days::new(1),
+            Self::Year(_) => Days::new(7),
+        };
+        std::iter::successors(Some(self.bucket_of(first)), |&day| Some(day + step))
+            .take_while(|&day| day < end)
+            .collect()
+    }
+
+    /// The first day of the bucket that holds `date`.
+    pub fn bucket_of(&self, date: NaiveDate) -> NaiveDate {
+        match self {
+            Self::Month(_) => date,
+            Self::Year(_) => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
+        }
+    }
+}
+
+/// One person's reports as (observed time, intensity), in time order.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct PersonLine {
+    pub user: UserId,
+    pub points: Vec<(DateTime<Utc>, Level)>,
+}
+
+/// Intensity over time with one line per person (spec §9: "a report's highest room level, one
+/// line per person"), ordered by user id. A no-smell report is a point at 0.
+pub fn intensity_lines(reports: &[SmellReport]) -> Vec<PersonLine> {
+    let mut lines: BTreeMap<UserId, Vec<(DateTime<Utc>, Level)>> = BTreeMap::new();
+    for report in reports {
+        lines
+            .entry(report.user_id)
+            .or_default()
+            .push((report.observed_at, report.values.intensity()));
+    }
+    lines
+        .into_iter()
+        .map(|(user, mut points)| {
+            points.sort_by_key(|&(at, _)| at);
+            PersonLine { user, points }
+        })
+        .collect()
+}
+
+/// The number of events in each bucket of `period`. An event counts once, in the bucket of
+/// its earliest report among `reports`, the rule of the summary card (decision #94); a no-smell
+/// check outside any event is not an event.
+pub fn events_per_bucket(
+    reports: &[SmellReport],
+    period: Period,
+    schedule: &DailySchedule,
+) -> Vec<(NaiveDate, usize)> {
+    let mut first_seen: BTreeMap<EventId, DateTime<Utc>> = BTreeMap::new();
+    for report in reports {
+        if let Some(event) = report.event_id {
+            first_seen
+                .entry(event)
+                .and_modify(|at| *at = (*at).min(report.observed_at))
+                .or_insert(report.observed_at);
+        }
+    }
+    let mut counts = zeroed(period, 0);
+    for at in first_seen.into_values() {
+        if let Some(count) = counts.get_mut(&period.bucket_of(schedule.local_date(at))) {
+            *count += 1;
+        }
+    }
+    counts.into_iter().collect()
+}
+
+/// The levels one room was given at one hour of the day.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub struct LevelTally {
+    /// Reports that checked the room at that hour, a clean check (0) included.
+    pub checks: u32,
+    /// The sum of their levels.
+    pub total: u32,
+}
+
+impl LevelTally {
+    /// The average level, or `None` when nobody checked the room at that hour.
+    pub fn mean(&self) -> Option<f64> {
+        (self.checks > 0).then(|| f64::from(self.total) / f64::from(self.checks))
+    }
+}
+
+/// One row of the room × hour-of-day heatmap: the room's tallies at local hours 0–23.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct RoomHours {
+    pub room: RoomId,
+    pub hours: [LevelTally; 24],
+}
+
+/// The room × hour-of-day heatmap of room levels, where 0 counts as checked and clean (spec
+/// §9). Rooms that no report names are left out; the rest are ordered by id. A no-smell
+/// report that names no room adds nothing: it says nothing about any one room.
+pub fn room_hours(reports: &[SmellReport], schedule: &DailySchedule) -> Vec<RoomHours> {
+    let mut rows: BTreeMap<RoomId, [LevelTally; 24]> = BTreeMap::new();
+    for report in reports {
+        let hour = report.observed_at.with_timezone(&schedule.tz()).hour() as usize;
+        for room in report.values.rooms() {
+            let tally = &mut rows.entry(room.room).or_default()[hour];
+            tally.checks += 1;
+            tally.total += u32::from(room.level.get());
+        }
+    }
+    rows.into_iter()
+        .map(|(room, hours)| RoomHours { room, hours })
+        .collect()
+}
+
+/// How many reports name each symptom, most frequent first, then by id. Symptoms nobody
+/// reported are left out.
+pub fn symptom_counts(reports: &[SmellReport]) -> Vec<(SymptomId, usize)> {
+    let mut counts: BTreeMap<SymptomId, usize> = BTreeMap::new();
+    for report in reports {
+        for &symptom in report.values.symptoms() {
+            *counts.entry(symptom).or_default() += 1;
+        }
+    }
+    let mut counts: Vec<_> = counts.into_iter().collect();
+    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
+    counts
+}
+
+/// The highest level reported inside the home and in common areas within one bucket; `None`
+/// where no room of that area was checked.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub struct AreaLevels {
+    pub inside: Option<Level>,
+    pub common: Option<Level>,
+}
+
+/// Inside vs common areas, bucket by bucket: each area's highest room level, the same rule as
+/// a report's intensity (spec §6). `rooms` gives each room's area; a room missing from it is
+/// skipped.
+pub fn area_levels(
+    reports: &[SmellReport],
+    rooms: &[Room],
+    period: Period,
+    schedule: &DailySchedule,
+) -> Vec<(NaiveDate, AreaLevels)> {
+    let areas: BTreeMap<RoomId, RoomArea> = rooms.iter().map(|r| (r.id, r.area)).collect();
+    let mut buckets = zeroed(period, AreaLevels::default());
+    for report in reports {
+        let bucket = period.bucket_of(schedule.local_date(report.observed_at));
+        let Some(levels) = buckets.get_mut(&bucket) else {
+            continue;
+        };
+        for room in report.values.rooms() {
+            let highest = match areas.get(&room.room) {
+                Some(RoomArea::Inside) => &mut levels.inside,
+                Some(RoomArea::Common) => &mut levels.common,
+                None => continue,
+            };
+            *highest = (*highest).max(Some(room.level));
+        }
+    }
+    buckets.into_iter().collect()
+}
+
+/// Every bucket of `period`, each holding `empty`.
+fn zeroed<T: Copy>(period: Period, empty: T) -> BTreeMap<NaiveDate, T> {
+    period.buckets().into_iter().map(|b| (b, empty)).collect()
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::records::{ReportValues, RoomLevel};
+    use crate::types::{ListName, ReportId};
+
+    fn t(s: &str) -> DateTime<Utc> {
+        s.parse().unwrap()
+    }
+
+    fn date(s: &str) -> NaiveDate {
+        s.parse().unwrap()
+    }
+
+    fn rome() -> DailySchedule {
+        DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap()
+    }
+
+    /// Event 0 stands for no event: a no-smell check.
+    fn report(
+        event: i64,
+        user: i64,
+        at: &str,
+        rooms: &[(i64, u8)],
+        symptoms: &[i64],
+    ) -> SmellReport {
+        let at = t(at);
+        SmellReport {
+            id: ReportId(1),
+            event_id: (event != 0).then_some(EventId(event)),
+            user_id: UserId(user),
+            observed_at: at,
+            recorded_at: at,
+            values: ReportValues::new(
+                rooms
+                    .iter()
+                    .map(|&(room, level)| RoomLevel {
+                        room: RoomId(room),
+                        level: Level::new(level).unwrap(),
+                    })
+                    .collect(),
+                symptoms.iter().map(|&s| SymptomId(s)).collect(),
+                None,
+            )
+            .unwrap(),
+            deleted_at: None,
+        }
+    }
+
+    fn room(id: i64, area: RoomArea) -> Room {
+        Room {
+            id: RoomId(id),
+            name: ListName::new("Room").unwrap(),
+            i18n_key: None,
+            area,
+            hidden_at: None,
+        }
+    }
+
+    fn level(value: u8) -> Level {
+        Level::new(value).unwrap()
+    }
+
+    #[test]
+    fn a_month_is_counted_per_day() {
+        let february = Period::Month(date("2026-02-14"));
+        let days = february.buckets();
+        assert_eq!(days.len(), 28);
+        assert_eq!(days[0], date("2026-02-01"));
+        assert_eq!(days[27], date("2026-02-28"));
+        assert_eq!(february.bucket_of(date("2026-02-14")), date("2026-02-14"));
+        assert_eq!(
+            february.span(&rome()),
+            (t("2026-01-31T23:00:00Z"), t("2026-02-28T23:00:00Z"))
+        );
+    }
+
+    #[test]
+    fn a_year_is_counted_per_week_from_monday() {
+        // 1 January 2026 is a Thursday: the first week starts on Monday 29 December 2025.
+        let year = Period::Year(date("2026-06-01"));
+        let weeks = year.buckets();
+        assert_eq!(weeks[0], date("2025-12-29"));
+        assert_eq!(weeks[1], date("2026-01-05"));
+        assert_eq!(*weeks.last().unwrap(), date("2026-12-28"));
+        assert_eq!(weeks.len(), 53);
+        assert_eq!(year.bucket_of(date("2026-01-01")), date("2025-12-29"));
+        assert_eq!(year.bucket_of(date("2026-01-11")), date("2026-01-05"));
+    }
+
+    #[test]
+    fn periods_step_back_and_forward_from_their_first_day() {
+        let january = Period::Month(date("2026-01-31"));
+        assert_eq!(january.previous(), Period::Month(date("2025-12-01")));
+        assert_eq!(january.next(), Period::Month(date("2026-02-01")));
+        let year = Period::Year(date("2026-07-15"));
+        assert_eq!(year.previous(), Period::Year(date("2025-01-01")));
+        assert_eq!(year.next().first_day(), date("2027-01-01"));
+    }
+
+    #[test]
+    fn each_person_gets_a_line_in_time_order() {
+        let reports = [
+            report(1, 2, "2026-01-10T21:00:00Z", &[(1, 6)], &[]),
+            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4), (3, 7)], &[]),
+            report(1, 2, "2026-01-10T20:30:00Z", &[(1, 5)], &[]),
+            report(0, 1, "2026-01-11T09:00:00Z", &[], &[]),
+        ];
+        assert_eq!(
+            intensity_lines(&reports),
+            [
+                PersonLine {
+                    user: UserId(1),
+                    points: vec![
+                        (t("2026-01-10T20:00:00Z"), level(7)),
+                        (t("2026-01-11T09:00:00Z"), level(0)),
+                    ],
+                },
+                PersonLine {
+                    user: UserId(2),
+                    points: vec![
+                        (t("2026-01-10T20:30:00Z"), level(5)),
+                        (t("2026-01-10T21:00:00Z"), level(6)),
+                    ],
+                },
+            ]
+        );
+    }
+
+    #[test]
+    fn an_event_counts_once_on_the_local_day_of_its_first_report() {
+        let january = Period::Month(date("2026-01-01"));
+        let reports = [
+            // 00:30 on 11 January in Rome.
+            report(1, 1, "2026-01-10T23:30:00Z", &[(1, 4)], &[]),
+            report(1, 2, "2026-01-11T08:00:00Z", &[(1, 6)], &[]),
+            report(2, 1, "2026-01-11T20:00:00Z", &[(1, 3)], &[]),
+            report(0, 1, "2026-01-12T09:00:00Z", &[], &[]),
+        ];
+        let counts: BTreeMap<_, _> = events_per_bucket(&reports, january, &rome())
+            .into_iter()
+            .collect();
+        assert_eq!(counts.len(), 31);
+        assert_eq!(counts[&date("2026-01-10")], 0);
+        assert_eq!(counts[&date("2026-01-11")], 2);
+        assert_eq!(counts[&date("2026-01-12")], 0);
+        assert_eq!(counts.values().sum::<usize>(), 2);
+    }
+
+    #[test]
+    fn a_year_counts_events_per_week() {
+        let year = Period::Year(date("2026-01-01"));
+        let reports = [
+            report(1, 1, "2026-01-05T20:00:00Z", &[(1, 4)], &[]),
+            report(2, 1, "2026-01-11T20:00:00Z", &[(1, 4)], &[]),
+            report(3, 1, "2026-01-12T20:00:00Z", &[(1, 4)], &[]),
+        ];
+        let counts = events_per_bucket(&reports, year, &rome());
+        assert_eq!(counts[1], (date("2026-01-05"), 2));
+        assert_eq!(counts[2], (date("2026-01-12"), 1));
+    }
+
+    #[test]
+    fn the_heatmap_averages_levels_per_room_and_local_hour_counting_clean_checks() {
+        let reports = [
+            // 21:xx in Rome (UTC+1).
+            report(1, 1, "2026-01-10T20:05:00Z", &[(1, 6), (3, 0)], &[]),
+            report(1, 2, "2026-01-11T20:50:00Z", &[(1, 3)], &[]),
+            // In summer Rome is UTC+2: still 21:xx local.
+            report(2, 1, "2026-07-01T19:10:00Z", &[(1, 0)], &[]),
+            // No rooms named: nothing to place.
+            report(0, 1, "2026-01-12T09:00:00Z", &[], &[]),
+        ];
+        let rows = room_hours(&reports, &rome());
+        assert_eq!(
+            rows.iter().map(|r| r.room).collect::<Vec<_>>(),
+            [RoomId(1), RoomId(3)]
+        );
+        assert_eq!(
+            rows[0].hours[21],
+            LevelTally {
+                checks: 3,
+                total: 9
+            }
+        );
+        assert_eq!(rows[0].hours[21].mean(), Some(3.0));
+        assert_eq!(rows[1].hours[21].mean(), Some(0.0));
+        assert_eq!(rows[0].hours[20].mean(), None);
+        assert_eq!(rows[0].hours.iter().map(|h| h.checks).sum::<u32>(), 3);
+    }
+
+    #[test]
+    fn symptoms_are_counted_per_report_most_frequent_first() {
+        let reports = [
+            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4)], &[2, 5]),
+            report(1, 1, "2026-01-10T21:00:00Z", &[(1, 4)], &[5]),
+            report(1, 2, "2026-01-10T22:00:00Z", &[(1, 4)], &[1]),
+            report(1, 2, "2026-01-10T23:00:00Z", &[(1, 4)], &[]),
+        ];
+        assert_eq!(
+            symptom_counts(&reports),
+            [(SymptomId(5), 2), (SymptomId(1), 1), (SymptomId(2), 1)]
+        );
+    }
+
+    #[test]
+    fn each_area_keeps_its_highest_level_per_bucket() {
+        let rooms = [room(1, RoomArea::Inside), room(6, RoomArea::Common)];
+        let january = Period::Month(date("2026-01-01"));
+        let reports = [
+            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4), (6, 2)], &[]),
+            report(1, 2, "2026-01-10T21:00:00Z", &[(1, 7)], &[]),
+            report(0, 1, "2026-01-11T09:00:00Z", &[(6, 0)], &[]),
+            // A room the list does not know is skipped.
+            report(2, 1, "2026-01-12T09:00:00Z", &[(9, 8)], &[]),
+        ];
+        let levels: BTreeMap<_, _> = area_levels(&reports, &rooms, january, &rome())
+            .into_iter()
+            .collect();
+        assert_eq!(
+            levels[&date("2026-01-10")],
+            AreaLevels {
+                inside: Some(level(7)),
+                common: Some(level(2)),
+            }
+        );
+        assert_eq!(
+            levels[&date("2026-01-11")],
+            AreaLevels {
+                inside: None,
+                common: Some(level(0)),
+            }
+        );
+        assert_eq!(levels[&date("2026-01-12")], AreaLevels::default());
+    }
+}
diff --git a/crates/domain/src/lib.rs b/crates/domain/src/lib.rs
index b9f821c..317ec8a 100644
--- a/crates/domain/src/lib.rs
+++ b/crates/domain/src/lib.rs
@@ -7,6 +7,7 @@ pub mod audit;
 pub mod clock;
 pub mod error;
 pub mod events;
+pub mod graphs;
 pub mod history;
 pub mod lists;
 pub mod logbook;
diff --git a/crates/domain/src/schedule.rs b/crates/domain/src/schedule.rs
index 8089946..fd4c87d 100644
--- a/crates/domain/src/schedule.rs
+++ b/crates/domain/src/schedule.rs
@@ -66,6 +66,15 @@ impl DailySchedule {
         (self.at_local(first, 0), self.at_local(next, 0))
     }
 
+    /// `[start, end)` of the local calendar year that contains `date`.
+    pub fn year_span(&self, date: NaiveDate) -> (DateTime<Utc>, DateTime<Utc>) {
+        let first = date.with_ordinal(1).expect("every year has a first day");
+        let next = first
+            .checked_add_months(Months::new(12))
+            .expect("date within chrono's range");
+        (self.at_local(first, 0), self.at_local(next, 0))
+    }
+
     /// The boundary instant on a local calendar date.
     fn boundary_on(&self, date: NaiveDate) -> DateTime<Utc> {
         self.at_local(date, self.hour)
@@ -249,6 +258,14 @@ mod tests {
         );
     }
 
+    #[test]
+    fn a_local_year_runs_from_new_years_midnight_to_the_next() {
+        assert_eq!(
+            rome_at(4).year_span(date("2026-07-15")),
+            (t("2025-12-31T23:00:00Z"), t("2026-12-31T23:00:00Z"))
+        );
+    }
+
     #[test]
     fn catch_up_after_sleeping_through_several_nights_is_still_one_sweep() {
         let s = rome_at(4);
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 43539e2..c747d63 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -129,3 +129,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 120 | 2026-10-14 | Invites | Opening a link asks for the **password twice and a language** (preselected from the page's language); saving uses up the link, ends the account's sessions and **signs this device in**, unless the account may not sign in (an expired viewer goes to the sign-in page); a new account starts in the **inviter's language** until then | Ask for the username too; email confirmation | Spec §8.3 step 3; the username is fixed by the resident and shown on the page |
 | 121 | 2026-10-14 | PWA | The **manifest** (`/static/manifest.webmanifest`, scope and start URL `/`, a "New smell report" shortcut to the Log screen) lists PNG icons of 192 and 512 px (the 512 also as maskable) and an SVG, all drawn from `static/icon.svg` with `rsvg-convert`; the **service worker** is served at `/sw.js` and goes to the network first: static assets fall back to their cached copy and pages to a cached **`/offline`** page; posts are never touched. The header's **Install app** link opens the browser's prompt where there is one and otherwise the **`/install`** page (button plus the iPhone steps, with a small Share and Add-to-Home-Screen icon beside the two steps); CSS hides it in standalone mode; a first setup lands on `/install?welcome` | Cache-first assets; an install banner; a separate offline logging queue | Spec §9.1 and §8.3 step 4 with no inline script; a deploy is never hidden behind an old cache (assets are already revalidated with ETags, #65); offline logging is out of scope for v1 (§14) |
 | 122 | 2026-10-14 | Invites | The app cuts `/invite/` paths from its own request span, but **Cloud Run's platform request log and a browser `Referer` could still carry the token**; this is **accepted** because a link is single-use and valid 5 minutes, and mitigated by `Referrer-Policy: no-referrer` on every response under `/invite/` (the global `same-origin` policy stays elsewhere) | Read the token from a URL fragment with script; a proxy that strips paths from the platform log | The platform log is readable only by the project owner and a used or expired link is dead; a fragment would need inline-free script for little gain; amends the logging claim in #119 |
+| 123 | 2026-10-14 | Graphs | The graphs' numbers come from **`smoke_domain::graphs`** (pure) over the reports the viewer may see, for **one local month counted per day or one local year counted per week** (Monday to Sunday): **intensity** = one line per person through each report's intensity (a no-smell report at 0); **events** = each event once, on the local day of its earliest visible report (the summary card's rule, #94); **room × hour** = each room's **average** level per local hour, clean checks (0) included, a report naming no room left out; **symptoms** = how many reports name each; **inside vs common** = each area's **highest** room level per day or week | A rolling 30/90/365 days; months as a year's buckets; the heatmap's highest level; symptoms counted per day | Spec §9 screen 5 names the graphs, not their arithmetic; a month matches the Events list (#100) and bounds every query, a year needs coarser buckets; an average lets "checked and clean" pull a cell down, the highest level is how a report's intensity is defined (§6) |
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Expected: the ten new unit tests pass. **577** tests in the workspace (Postgres not needed:
storage is untouched).

- [ ] **Step 5: Commit**

```bash
git add crates/domain/src/graphs.rs \
  crates/domain/src/lib.rs \
  crates/domain/src/schedule.rs \
  docs/DECISIONS.md
git commit -F - <<'EOF'
feat(graphs): pure series for the Graphs screen in smoke_domain::graphs

A Period enum (one local month counted per day, or one local year counted per Monday-to-
Sunday week) and plain functions over the reports a viewer may see: one intensity line per
person, events per bucket, the room-by-hour heatmap of average levels, symptom counts and
the highest level inside versus in common areas. DailySchedule gains year_span.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 3: Graphs screen: vendored Chart.js, intensity and events

Spec §9 screen 5, first half. `/graphs` (in the menu for everyone signed in) shows one local
month (`?month=YYYY-MM`, the current one by default) or one year (`?year=YYYY`), with
previous/next links and a link to the other kind of period (decision #125). Intensity over time
has one line per person, named by username, on a 0–10 axis and PM2.5 readings on a right-hand
axis; events are counted per day (month) or per week (year). Guests see their own reports and
readings only, and a line saying so; a period with nothing logged says so instead of drawing
empty charts.

Chart.js 4.5.1 is vendored (decision #124) and loaded only by this page through a new
`{% block scripts %}` in `base.html`. Each chart's configuration is built in Rust with
`serde_json::json!` and written into its canvas's `data-chart` attribute (Askama escapes it);
`static/graphs.js` parses it and adds the only thing JSON cannot carry: time-axis labels,
formatted with `Intl.DateTimeFormat` in `APP_TIMEZONE` (the page sends `{"zone", "locale"}` in a
`time` key, `en-GB`/`it-IT`/`tr-TR` so dates are day first, decision #95). Chart.js 4 sets
canvas sizes through the CSSOM, which the CSP allows. `.gitattributes` marks vendored
`*.min.js` as `linguist-vendored` so GitHub's language bar shows a Rust project.

Month parsing (`parse_month`, `SUPPORTED_YEARS`) moves from `events.rs` to `time.rs`, shared by
both screens; `time::day_month` labels a year's weeks.

**Files:**
- Create: `crates/web/src/routes/graphs.rs`, `crates/web/static/chart-4.5.1.umd.min.js`, `crates/web/static/graphs.js`, `crates/web/templates/graphs.html`, `crates/web/tests/graphs.rs`
- Modify: `.gitattributes`, `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/assets.rs`, `crates/web/src/i18n.rs`, `crates/web/src/lib.rs`, `crates/web/src/routes/events.rs`, `crates/web/src/routes/mod.rs`, `crates/web/src/time.rs`, `crates/web/static/app.css`, `crates/web/templates/base.html`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `smoke_domain::graphs` (Task 2); `ReportRepo::reports_between`,
  `ReadingRepo::readings_between`, `UserRepo::users`; `can_see`, `can(…, Action::ViewAll, …)`;
  `SignedIn::of`; `page`.
- Produces (web):
  - Route `GET /graphs` → `routes::graphs::show` (signed-in only); template `graphs.html`.
  - `time::{SUPPORTED_YEARS, parse_month, day_month}`.
  - `base.html`: `{% block scripts %}{% endblock %}` after `app.js`; menu link `/graphs`.
  - `static/graphs.js`; `ASSETS` gains `chart-4.5.1.umd.min.js` and `graphs.js`.
  - `routes::graphs::KEYS` (keys passed through variables) for the translation test.
  - Chart JSON contract, relied on by Task 4 and by `graphs.js`: `{"type", "data", "options",
    "time"?}`; the intensity chart's datasets use `yAxisID` `level`/`pm25`, its x axis is
    milliseconds since the epoch.
  - Translation keys (12): `nav-graphs`, `graphs-title`, `graphs-month`, `graphs-year`,
    `graphs-own-only`, `graphs-none`, `graphs-needs-js`, `graphs-intensity`, `graphs-pm25`,
    `graphs-events-day`, `graphs-events-week`, `graphs-events-label`.

- [ ] **Step 1: Write the failing tests**

The integration tests below. The unit tests in `crates/web/src/routes/graphs.rs`,
`crates/web/src/time.rs` and `crates/web/src/assets.rs` are part of the Step 3 diff.

````diff
diff --git a/crates/web/tests/graphs.rs b/crates/web/tests/graphs.rs
new file mode 100644
index 0000000..43f1234
--- /dev/null
+++ b/crates/web/tests/graphs.rs
@@ -0,0 +1,203 @@
+//! The Graphs screen (spec §9 screen 5): what each chart is given, and who sees what (spec §4).
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use serde_json::Value;
+use smoke_domain::logbook::EventChoice;
+use smoke_domain::types::{Locale, Role};
+
+/// The Chart.js configurations on a page, in page order, read back from their `data-chart`
+/// attributes.
+fn charts(html: &str) -> Vec<Value> {
+    html.match_indices(r#"data-chart=""#)
+        .map(|(start, attribute)| {
+            let rest = &html[start + attribute.len()..];
+            let escaped = &rest[..rest.find('"').unwrap()];
+            let json = escaped
+                .replace("&#34;", "\"")
+                .replace("&#39;", "'")
+                .replace("&lt;", "<")
+                .replace("&gt;", ">")
+                .replace("&amp;", "&");
+            serde_json::from_str(&json).unwrap_or_else(|e| panic!("{e}: {json}"))
+        })
+        .collect()
+}
+
+fn labels(chart: &Value) -> Vec<&str> {
+    chart["data"]["datasets"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .map(|d| d["label"].as_str().unwrap())
+        .collect()
+}
+
+/// Two residents and a guest, each with a report in January 2026; one PM2.5 reading each from
+/// a resident and the guest.
+async fn household() -> (TestApp, String, String) {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    let first = app
+        .log(
+            &me,
+            "2026-01-10T18:00:00Z",
+            3,
+            &[],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let event = first.report.event_id.unwrap();
+    app.log(
+        &partner,
+        "2026-01-10T19:00:00Z",
+        7,
+        &[],
+        EventChoice::Join(event),
+    )
+    .await;
+    app.log(
+        &guest,
+        "2026-01-12T07:30:00Z",
+        5,
+        &[],
+        EventChoice::New { replacing: None },
+    )
+    .await;
+    app.log_pm25(&me, "2026-01-10T18:30:00Z", 35.5).await;
+    app.log_pm25(&guest, "2026-01-12T07:35:00Z", 12.0).await;
+    let resident = app.sign_in("daniele").await;
+    let guest = app.sign_in("guest").await;
+    (app, resident, guest)
+}
+
+#[tokio::test]
+async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
+    let (app, resident, _) = household().await;
+    let html = text(app.send(get_with("/graphs?month=2026-01", &resident)).await).await;
+    assert!(html.contains(r#"<a href="/graphs">Graphs</a>"#), "{html}");
+    assert!(html.contains("<strong>01/2026</strong>"), "{html}");
+    assert!(!html.contains("only your own"), "{html}");
+    let intensity = &charts(&html)[0];
+    assert_eq!(
+        labels(intensity),
+        ["daniele", "partner", "guest", "PM2.5 (µg/m³)"]
+    );
+    let datasets = &intensity["data"]["datasets"];
+    assert_eq!(datasets[0]["yAxisID"], "level");
+    assert_eq!(datasets[1]["data"][0]["y"], 7);
+    assert_eq!(
+        datasets[1]["data"][0]["x"],
+        t("2026-01-10T19:00:00Z").timestamp_millis()
+    );
+    assert_eq!(datasets[3]["yAxisID"], "pm25");
+    assert_eq!(datasets[3]["data"][0]["y"], 35.5);
+    assert_eq!(datasets[3]["data"].as_array().unwrap().len(), 2);
+    let scales = &intensity["options"]["scales"];
+    assert_eq!(scales["level"]["max"], 10);
+    assert_eq!(scales["pm25"]["position"], "right");
+    assert_eq!(
+        scales["x"]["min"],
+        t("2025-12-31T23:00:00Z").timestamp_millis()
+    );
+    assert_eq!(intensity["time"]["zone"], "Europe/Rome");
+    assert_eq!(intensity["time"]["locale"], "en-GB");
+}
+
+#[tokio::test]
+async fn guests_see_graphs_of_their_own_reports_only() {
+    let (app, _, guest) = household().await;
+    let html = text(app.send(get_with("/graphs?month=2026-01", &guest)).await).await;
+    assert!(
+        html.contains("These graphs show only your own reports and readings."),
+        "{html}"
+    );
+    assert!(!html.contains("daniele"), "{html}");
+    assert!(!html.contains("partner"), "{html}");
+    let charts = charts(&html);
+    assert_eq!(labels(&charts[0]), ["guest", "PM2.5 (µg/m³)"]);
+    assert_eq!(charts[0]["data"]["datasets"][1]["data"][0]["y"], 12.0);
+    let events: Vec<u64> = charts[1]["data"]["datasets"][0]["data"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .map(|n| n.as_u64().unwrap())
+        .collect();
+    assert_eq!(events.iter().sum::<u64>(), 1, "only the guest's own event");
+    assert_eq!(events[11], 1, "on 12 January");
+}
+
+#[tokio::test]
+async fn a_month_counts_events_per_day() {
+    let (app, resident, _) = household().await;
+    let html = text(app.send(get_with("/graphs?month=2026-01", &resident)).await).await;
+    assert!(html.contains("<h2>Events per day</h2>"), "{html}");
+    let events = &charts(&html)[1];
+    let labels = events["data"]["labels"].as_array().unwrap();
+    assert_eq!(labels.len(), 31);
+    assert_eq!(labels[0], "1");
+    let counts = &events["data"]["datasets"][0]["data"];
+    assert_eq!(counts[9], 1, "10 January: one event, two people");
+    assert_eq!(counts[11], 1, "12 January");
+    assert_eq!(
+        counts
+            .as_array()
+            .unwrap()
+            .iter()
+            .filter(|n| **n != 0)
+            .count(),
+        2
+    );
+}
+
+#[tokio::test]
+async fn a_year_counts_events_per_week_and_links_back_to_a_month() {
+    let (app, resident, _) = household().await;
+    let html = text(app.send(get_with("/graphs?year=2026", &resident)).await).await;
+    assert!(html.contains("<strong>2026</strong>"), "{html}");
+    assert!(html.contains(r#"href="/graphs?year=2025""#), "{html}");
+    assert!(html.contains(r#"href="/graphs?year=2027""#), "{html}");
+    assert!(
+        html.contains(r#"<a href="/graphs?month=2026-01">Show one month</a>"#),
+        "{html}"
+    );
+    assert!(html.contains("<h2>Events per week</h2>"), "{html}");
+    let events = &charts(&html)[1];
+    let labels = events["data"]["labels"].as_array().unwrap();
+    assert_eq!(labels.len(), 53);
+    assert_eq!(labels[0], "29/12");
+    assert_eq!(labels[1], "05/01");
+    let counts = &events["data"]["datasets"][0]["data"];
+    assert_eq!(counts[1], 1, "the week of 10 January");
+    assert_eq!(counts[2], 1, "the week of 12 January");
+}
+
+#[tokio::test]
+async fn an_unusable_address_shows_this_month_and_an_empty_one_says_so() {
+    let (app, resident, _) = household().await;
+    app.at("2026-03-05T12:00:00Z");
+    for path in ["/graphs", "/graphs?month=2026-13", "/graphs?year=1999"] {
+        let response = app.send(get_with(path, &resident)).await;
+        assert_eq!(response.status(), StatusCode::OK, "{path}");
+        let html = text(response).await;
+        assert!(html.contains("<strong>03/2026</strong>"), "{path}: {html}");
+        assert!(
+            html.contains("Nothing was logged in this period."),
+            "{path}: {html}"
+        );
+        assert!(charts(&html).is_empty(), "{path}");
+    }
+}
+
+#[tokio::test]
+async fn the_graphs_need_a_sign_in() {
+    let app = TestApp::new().await;
+    let response = app.send(get("/graphs")).await;
+    assert_eq!(response.status(), StatusCode::SEE_OTHER);
+    assert_eq!(location(&response), Some("/login"));
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index 94d93e7..eba4649 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -234,6 +234,8 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         .await,
         text(app.send(get_with("/install?welcome", &cookie)).await).await,
         text(app.send(get("/offline")).await).await,
+        text(app.send(get_with("/graphs", &cookie)).await).await,
+        text(app.send(get_with("/graphs?year=2026", &cookie)).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
````

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web`
Expected: `tests/graphs.rs` fails (every `/graphs` request answers 404, so no `data-chart`
attribute is found and `labels`/`charts` indexing panics; `the_graphs_need_a_sign_in` gets 404,
not a redirect). The security scan still passes on the 404 pages until Step 3.

- [ ] **Step 3: Implement**

First the vendored library. Download the npm release into a new, empty temporary directory,
check the archive against the registry's integrity value, and copy the minified UMD build
unchanged (do not reformat it; a test pins its SHA-256):

```sh
dir=$(mktemp -d) && cd "$dir"
curl -sL -o chart.tgz https://registry.npmjs.org/chart.js/-/chart.js-4.5.1.tgz
openssl dgst -sha512 -binary chart.tgz | base64
# must print GIjfiT9dbmHRiYi6Nl2yFCq7kkwdkp1W/lp2J99rX0yo9tgJGn3lKQATztIjb5tVtevcBtIdICNWqlq5+E8/Pw==
tar xzf chart.tgz
cp package/dist/chart.umd.min.js <repo>/crates/web/static/chart-4.5.1.umd.min.js
shasum -a 256 <repo>/crates/web/static/chart-4.5.1.umd.min.js
# must print 48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a
```

(Or copy the prototype's file: `git checkout m6-proto -- crates/web/static/chart-4.5.1.umd.min.js`.) Then:

````diff
diff --git a/.gitattributes b/.gitattributes
index e01197f..ff75186 100644
--- a/.gitattributes
+++ b/.gitattributes
@@ -1,3 +1,6 @@
 # Machine-generated files: GitHub collapses them in pull-request diffs.
 Cargo.lock linguist-generated=true
 crates/*/.sqlx/** linguist-generated=true
+
+# Third-party libraries, vendored byte for byte: left out of the language statistics.
+crates/web/static/*.min.js linguist-vendored=true
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 26bd036..2e947b0 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Like a smoke-filled room. You must leave.
 
 nav-log = Log
 nav-events = Events
+nav-graphs = Graphs
 nav-people = People
 nav-settings = Settings
 
@@ -257,6 +258,18 @@ offline-title = No connection
 offline-text = Smoke Tracker needs the internet to save what you log. Try again when you are back online.
 offline-retry = Try again
 
+graphs-title = Graphs
+graphs-month = Show one month
+graphs-year = Show the whole year
+graphs-own-only = These graphs show only your own reports and readings.
+graphs-none = Nothing was logged in this period.
+graphs-needs-js = This graph needs JavaScript.
+graphs-intensity = Smell intensity over time
+graphs-pm25 = PM2.5 (µg/m³)
+graphs-events-day = Events per day
+graphs-events-week = Events per week
+graphs-events-label = Events
+
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
 error-forbidden-title = Request blocked
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 2795481..84d27dd 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
 nav-log = Registra
 nav-events = Eventi
+nav-graphs = Grafici
 nav-people = Persone
 nav-settings = Impostazioni
 
@@ -257,6 +258,18 @@ offline-title = Nessuna connessione
 offline-text = Smoke Tracker ha bisogno di internet per salvare ciò che registri. Riprova quando sei di nuovo online.
 offline-retry = Riprova
 
+graphs-title = Grafici
+graphs-month = Mostra un mese
+graphs-year = Mostra l'intero anno
+graphs-own-only = Questi grafici mostrano solo le tue segnalazioni e misurazioni.
+graphs-none = Nessuna registrazione in questo periodo.
+graphs-needs-js = Questo grafico richiede JavaScript.
+graphs-intensity = Intensità dell'odore nel tempo
+graphs-pm25 = PM2.5 (µg/m³)
+graphs-events-day = Eventi al giorno
+graphs-events-week = Eventi alla settimana
+graphs-events-label = Eventi
+
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
 error-forbidden-title = Richiesta bloccata
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 33cfe6e..862d508 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -118,6 +118,7 @@ intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
 nav-log = Kaydet
 nav-events = Olaylar
+nav-graphs = Grafikler
 nav-people = Kişiler
 nav-settings = Ayarlar
 
@@ -257,6 +258,18 @@ offline-title = Bağlantı yok
 offline-text = Smoke Tracker'ın kaydettiklerinizi saklamak için internete ihtiyacı var. Yeniden çevrimiçi olduğunuzda tekrar deneyin.
 offline-retry = Tekrar dene
 
+graphs-title = Grafikler
+graphs-month = Tek bir ayı göster
+graphs-year = Tüm yılı göster
+graphs-own-only = Bu grafikler yalnızca kendi bildirimlerinizi ve ölçümlerinizi gösterir.
+graphs-none = Bu dönemde hiçbir kayıt yok.
+graphs-needs-js = Bu grafik için JavaScript gerekir.
+graphs-intensity = Zaman içinde koku yoğunluğu
+graphs-pm25 = PM2.5 (µg/m³)
+graphs-events-day = Günlük olay sayısı
+graphs-events-week = Haftalık olay sayısı
+graphs-events-label = Olaylar
+
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
 error-forbidden-title = İstek engellendi
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 57bc1b2..cc936c0 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -28,7 +28,9 @@ macro_rules! asset {
 const ASSETS: &[Asset] = &[
     asset!("app.css", "text/css; charset=utf-8"),
     asset!("app.js", "text/javascript; charset=utf-8"),
+    asset!("chart-4.5.1.umd.min.js", "text/javascript; charset=utf-8"),
     asset!("favicon.svg", "image/svg+xml"),
+    asset!("graphs.js", "text/javascript; charset=utf-8"),
     asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
     asset!("icon-180.png", "image/png"),
     asset!("icon-192.png", "image/png"),
@@ -85,6 +87,10 @@ mod tests {
     /// The published `dist/htmx.min.js` of htmx.org 2.0.11 on npm.
     const HTMX_SHA256: &str = "d6fdc75f204e6bdefa99b69bf1e6d4ac69b8a364f77929f45c13476b4000f717";
 
+    /// The published `dist/chart.umd.min.js` of chart.js 4.5.1 on npm (MIT licence).
+    const CHART_JS_SHA256: &str =
+        "48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a";
+
     #[test]
     fn every_file_in_the_static_folder_is_served() {
         let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
@@ -141,8 +147,14 @@ mod tests {
     }
 
     #[test]
-    fn the_vendored_htmx_is_the_published_release() {
-        let htmx = ASSETS.iter().find(|a| a.path.starts_with("htmx-")).unwrap();
-        assert_eq!(hex::encode(Sha256::digest(htmx.body)), HTMX_SHA256);
+    fn the_vendored_libraries_are_the_published_releases() {
+        for (prefix, sha256) in [("htmx-", HTMX_SHA256), ("chart-", CHART_JS_SHA256)] {
+            let library = ASSETS.iter().find(|a| a.path.starts_with(prefix)).unwrap();
+            assert_eq!(
+                hex::encode(Sha256::digest(library.body)),
+                sha256,
+                "{prefix}"
+            );
+        }
     }
 }
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index abaa2fd..2a49d0b 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -127,7 +127,7 @@ mod tests {
 
     use super::*;
     use crate::routes::login::Notice;
-    use crate::routes::{self, entries, guide, lists, people, reports, settings};
+    use crate::routes::{self, entries, graphs, guide, lists, people, reports, settings};
     use crate::{error, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
@@ -221,6 +221,7 @@ mod tests {
         used.extend(settings::keys().into_iter().map(str::to_owned));
         used.extend(lists::KEYS.map(str::to_owned));
         used.extend(people::keys().into_iter().map(str::to_owned));
+        used.extend(graphs::KEYS.map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 11660bb..85cd877 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -50,6 +50,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
         .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
         .route("/events", get(routes::events::list::<S>))
         .route("/events/{id}", get(routes::events::show::<S>))
+        .route("/graphs", get(routes::graphs::show::<S>))
         .route("/events/{id}/close", post(routes::events::close::<S>))
         .route("/events/{id}/reopen", post(routes::events::reopen::<S>))
         .route("/reports/{id}", get(routes::reports::show::<S>))
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index bea716a..3a71bab 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -90,9 +90,6 @@ fn day(days: &mut BTreeMap<NaiveDate, Day>, tr: Tr, date: NaiveDate) -> &mut Day
     })
 }
 
-/// The years the month list accepts, well inside chrono's range so month arithmetic cannot overflow.
-const SUPPORTED_YEARS: std::ops::RangeInclusive<i32> = 2000..=9999;
-
 pub(crate) async fn list<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
@@ -106,8 +103,8 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
     let actor = user.actor();
     let first = query
         .month
-        .and_then(|m| NaiveDate::parse_from_str(&format!("{m}-01"), "%Y-%m-%d").ok())
-        .filter(|first| SUPPORTED_YEARS.contains(&first.year()))
+        .as_deref()
+        .and_then(time::parse_month)
         .unwrap_or_else(|| schedule.local_date(now));
     let (from, to) = schedule.month_span(first);
     let events = state.store.events_between(from, to).await?;
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
new file mode 100644
index 0000000..36385be
--- /dev/null
+++ b/crates/web/src/routes/graphs.rs
@@ -0,0 +1,335 @@
+//! The Graphs screen (spec §9 screen 5). Each chart is a `<canvas>` whose Chart.js
+//! configuration travels as JSON in its `data-chart` attribute; `static/graphs.js` draws it, so
+//! no page carries an inline script. The numbers come from `smoke_domain::graphs`, over what the
+//! viewer may see: guests get graphs of their own data only (spec §4).
+
+use std::collections::HashMap;
+
+use askama::Template;
+use axum::extract::{Query, State};
+use axum::http::StatusCode;
+use axum::response::Response;
+use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
+use chrono_tz::Tz;
+use serde::Deserialize;
+use serde_json::{Value, json};
+use smoke_domain::graphs::{Period, PersonLine, events_per_bucket, intensity_lines};
+use smoke_domain::permissions::{Action, can, can_see};
+use smoke_domain::records::Pm25Reading;
+use smoke_domain::repos::Store;
+use smoke_domain::types::{Locale, UserId};
+
+use crate::auth::CurrentUser;
+use crate::error::{AppError, page};
+use crate::header::SignedIn;
+use crate::i18n::Tr;
+use crate::state::AppState;
+use crate::time;
+
+#[derive(Deserialize)]
+pub(crate) struct GraphsQuery {
+    /// `YYYY-MM`: that month, counted per day.
+    month: Option<String>,
+    /// `YYYY`: that year, counted per week. Wins over `month`.
+    year: Option<String>,
+}
+
+/// The period an address asks for; the current local month when it asks for none we accept.
+fn period(query: &GraphsQuery, today: NaiveDate) -> Period {
+    let year = query
+        .year
+        .as_deref()
+        .and_then(|y| y.parse::<i32>().ok())
+        .filter(|y| time::SUPPORTED_YEARS.contains(y))
+        .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
+    if let Some(first) = year {
+        return Period::Year(first);
+    }
+    let month = query.month.as_deref().and_then(time::parse_month);
+    Period::Month(month.unwrap_or(today))
+}
+
+/// The query string that asks for `period`.
+fn address(period: Period) -> String {
+    match period {
+        Period::Month(_) => format!("month={}", period.first_day().format("%Y-%m")),
+        Period::Year(_) => format!("year={}", period.first_day().year()),
+    }
+}
+
+/// The other kind of period: a month's year, or a year's current month (January when the year
+/// is not this one).
+fn switched(period: Period, today: NaiveDate) -> Period {
+    match period {
+        Period::Month(date) => Period::Year(date),
+        Period::Year(date) if date.year() == today.year() => Period::Month(today),
+        Period::Year(date) => Period::Month(date),
+    }
+}
+
+/// The language tag `Intl.DateTimeFormat` gets, chosen so that dates come out day first as on
+/// every other page (decision #95).
+fn intl_locale(locale: Locale) -> &'static str {
+    match locale {
+        Locale::En => "en-GB",
+        Locale::It => "it-IT",
+        Locale::Tr => "tr-TR",
+    }
+}
+
+const DAY_MS: i64 = 24 * 60 * 60 * 1000;
+
+/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
+/// on the right (spec §9). The x axis is milliseconds since the epoch.
+fn intensity_chart(
+    tr: Tr,
+    lines: &[PersonLine],
+    names: &HashMap<UserId, String>,
+    readings: &[Pm25Reading],
+    period: Period,
+    (from, to): (DateTime<Utc>, DateTime<Utc>),
+    tz: Tz,
+) -> Value {
+    let mut datasets: Vec<Value> = lines
+        .iter()
+        .map(|line| {
+            let points: Vec<Value> = line
+                .points
+                .iter()
+                .map(|(at, level)| json!({ "x": at.timestamp_millis(), "y": level.get() }))
+                .collect();
+            json!({
+                "label": names.get(&line.user).map_or("", String::as_str),
+                "data": points,
+                "yAxisID": "level",
+            })
+        })
+        .collect();
+    if !readings.is_empty() {
+        let points: Vec<Value> = readings
+            .iter()
+            .map(|r| json!({ "x": r.observed_at.timestamp_millis(), "y": r.value.ugm3() }))
+            .collect();
+        datasets.push(json!({
+            "label": tr.t("graphs-pm25"),
+            "data": points,
+            "yAxisID": "pm25",
+            "borderDash": [6, 4],
+        }));
+    }
+    let step = match period {
+        Period::Month(_) => json!({ "stepSize": DAY_MS }),
+        Period::Year(_) => json!({}),
+    };
+    json!({
+        "type": "line",
+        "data": { "datasets": datasets },
+        "options": {
+            "scales": {
+                "x": {
+                    "type": "linear",
+                    "min": from.timestamp_millis(),
+                    "max": (to - TimeDelta::milliseconds(1)).timestamp_millis(),
+                    "ticks": step,
+                },
+                "level": {
+                    "type": "linear",
+                    "position": "left",
+                    "min": 0,
+                    "max": 10,
+                    "title": { "display": true, "text": tr.t("rating-level") },
+                },
+                "pm25": {
+                    "type": "linear",
+                    "position": "right",
+                    "display": "auto",
+                    "beginAtZero": true,
+                    "grid": { "drawOnChartArea": false },
+                    "title": { "display": true, "text": tr.t("graphs-pm25") },
+                },
+            },
+        },
+        "time": { "zone": tz.name(), "locale": intl_locale(tr.0) },
+    })
+}
+
+/// The label under each bucket: the day of the month, or a week's Monday as day and month.
+fn bucket_labels(tr: Tr, period: Period, buckets: &[NaiveDate]) -> Vec<String> {
+    buckets
+        .iter()
+        .map(|&day| match period {
+            Period::Month(_) => day.day().to_string(),
+            Period::Year(_) => time::day_month(tr.0, day),
+        })
+        .collect()
+}
+
+/// A bar chart of whole numbers, one bar per bucket and no legend.
+fn count_chart(tr: Tr, labels: Vec<String>, counts: Vec<usize>) -> Value {
+    json!({
+        "type": "bar",
+        "data": {
+            "labels": labels,
+            "datasets": [{ "label": tr.t("graphs-events-label"), "data": counts }],
+        },
+        "options": {
+            "scales": { "y": { "beginAtZero": true, "ticks": { "precision": 0 } } },
+            "plugins": { "legend": { "display": false } },
+        },
+    })
+}
+
+#[derive(Template)]
+#[template(path = "graphs.html")]
+struct GraphsPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    label: String,
+    previous: String,
+    next: String,
+    other: String,
+    other_key: &'static str,
+    /// Guests see only their own data; the page says so.
+    own_only: bool,
+    empty: bool,
+    intensity: String,
+    events_key: &'static str,
+    events: String,
+}
+
+pub(crate) async fn show<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    Query(query): Query<GraphsQuery>,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let schedule = &state.config.schedule;
+    let today = schedule.local_date(now);
+    let actor = user.actor();
+    let period = period(&query, today);
+    let span = period.span(schedule);
+
+    let reports: Vec<_> = state
+        .store
+        .reports_between(span.0, span.1)
+        .await?
+        .into_iter()
+        .filter(|r| can_see(&actor, r.user_id, now))
+        .collect();
+    let readings: Vec<_> = state
+        .store
+        .readings_between(span.0, span.1)
+        .await?
+        .into_iter()
+        .filter(|r| can_see(&actor, r.user_id, now))
+        .collect();
+    let names: HashMap<UserId, String> = state
+        .store
+        .users()
+        .await?
+        .into_iter()
+        .map(|u| (u.id, u.username.to_string()))
+        .collect();
+
+    let buckets = period.buckets();
+    let labels = bucket_labels(tr, period, &buckets);
+    let events = events_per_bucket(&reports, period, schedule)
+        .into_iter()
+        .map(|(_, count)| count)
+        .collect();
+    let lines = intensity_lines(&reports);
+    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule.tz());
+
+    let (label, events_key, other_key) = match period {
+        Period::Month(_) => (
+            period.first_day().format("%m/%Y").to_string(),
+            "graphs-events-day",
+            "graphs-year",
+        ),
+        Period::Year(_) => (
+            period.first_day().year().to_string(),
+            "graphs-events-week",
+            "graphs-month",
+        ),
+    };
+    page(
+        StatusCode::OK,
+        &GraphsPage {
+            tr,
+            signed_in: SignedIn::of(&user, now),
+            label,
+            previous: address(period.previous()),
+            next: address(period.next()),
+            other: address(switched(period, today)),
+            other_key,
+            own_only: !can(&actor, Action::ViewAll, now),
+            empty: reports.is_empty() && readings.is_empty(),
+            intensity: intensity.to_string(),
+            events_key,
+            events: count_chart(tr, labels, events).to_string(),
+        },
+    )
+}
+
+/// Every key passed to the template through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) const KEYS: [&str; 4] = [
+    "graphs-events-day",
+    "graphs-events-week",
+    "graphs-year",
+    "graphs-month",
+];
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn date(s: &str) -> NaiveDate {
+        s.parse().unwrap()
+    }
+
+    fn query(month: Option<&str>, year: Option<&str>) -> GraphsQuery {
+        GraphsQuery {
+            month: month.map(str::to_owned),
+            year: year.map(str::to_owned),
+        }
+    }
+
+    #[test]
+    fn the_address_picks_a_month_or_a_year_and_falls_back_to_this_month() {
+        let today = date("2026-10-14");
+        assert_eq!(
+            period(&query(Some("2026-01"), None), today),
+            Period::Month(date("2026-01-01"))
+        );
+        assert_eq!(
+            period(&query(Some("2026-01"), Some("2025")), today),
+            Period::Year(date("2025-01-01"))
+        );
+        for (month, year) in [(Some("2026-13"), None), (None, Some("99999")), (None, None)] {
+            assert_eq!(
+                period(&query(month, year), today),
+                Period::Month(today),
+                "{month:?} {year:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn switching_keeps_the_year_and_prefers_this_month() {
+        let today = date("2026-10-14");
+        assert_eq!(
+            address(switched(Period::Month(date("2025-03-01")), today)),
+            "year=2025"
+        );
+        assert_eq!(
+            address(switched(Period::Year(date("2026-01-01")), today)),
+            "month=2026-10"
+        );
+        assert_eq!(
+            address(switched(Period::Year(date("2024-01-01")), today)),
+            "month=2024-01"
+        );
+    }
+}
diff --git a/crates/web/src/routes/mod.rs b/crates/web/src/routes/mod.rs
index 0aa1ed3..cc1ecdc 100644
--- a/crates/web/src/routes/mod.rs
+++ b/crates/web/src/routes/mod.rs
@@ -3,6 +3,7 @@
 pub(crate) mod accept;
 pub(crate) mod entries;
 pub(crate) mod events;
+pub(crate) mod graphs;
 pub(crate) mod guide;
 pub(crate) mod install;
 pub(crate) mod lang;
diff --git a/crates/web/src/time.rs b/crates/web/src/time.rs
index 5fc91e6..f4e5a3e 100644
--- a/crates/web/src/time.rs
+++ b/crates/web/src/time.rs
@@ -1,7 +1,9 @@
 //! Times as people read and type them: in the configured time zone (`APP_TIMEZONE`), never the
 //! browser's, so every phone shows the same times.
 
-use chrono::{DateTime, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
+use std::ops::RangeInclusive;
+
+use chrono::{DateTime, Datelike, LocalResult, NaiveDate, NaiveDateTime, TimeZone, Utc};
 use chrono_tz::Tz;
 use smoke_domain::types::Locale;
 use thiserror::Error;
@@ -27,6 +29,17 @@ impl TimeInputError {
     }
 }
 
+/// The years a page accepts in its address, well inside chrono's range so that month and year
+/// arithmetic cannot overflow.
+pub(crate) const SUPPORTED_YEARS: RangeInclusive<i32> = 2000..=9999;
+
+/// The first day of a `YYYY-MM` month, as `?month=` carries it; `None` when it is not one.
+pub(crate) fn parse_month(value: &str) -> Option<NaiveDate> {
+    NaiveDate::parse_from_str(&format!("{value}-01"), "%Y-%m-%d")
+        .ok()
+        .filter(|first| SUPPORTED_YEARS.contains(&first.year()))
+}
+
 /// A calendar date as the language writes it with digits.
 pub(crate) fn date(locale: Locale, date: NaiveDate) -> String {
     let pattern = match locale {
@@ -36,6 +49,15 @@ pub(crate) fn date(locale: Locale, date: NaiveDate) -> String {
     date.format(pattern).to_string()
 }
 
+/// A day and month without the year: `15/01`, `15.01` in Turkish.
+pub(crate) fn day_month(locale: Locale, date: NaiveDate) -> String {
+    let pattern = match locale {
+        Locale::En | Locale::It => "%d/%m",
+        Locale::Tr => "%d.%m",
+    };
+    date.format(pattern).to_string()
+}
+
 /// `19:05`, in the local time zone.
 pub(crate) fn clock(at: DateTime<Utc>, tz: Tz) -> String {
     at.with_timezone(&tz).format("%H:%M").to_string()
@@ -96,6 +118,18 @@ mod tests {
         );
     }
 
+    #[test]
+    fn months_in_addresses_are_read_within_the_supported_years() {
+        assert_eq!(parse_month("2026-02"), "2026-02-01".parse().ok());
+        for bad in ["2026-13", "2026", "", "1999-12", "10000-01", "2026-02-01"] {
+            assert_eq!(parse_month(bad), None, "{bad}");
+        }
+        assert_eq!(
+            day_month(Locale::Tr, "2026-01-05".parse().unwrap()),
+            "05.01"
+        );
+    }
+
     #[test]
     fn today_shows_only_the_time() {
         let at = t("2026-01-15T18:05:00Z");
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 75cc3da..4161711 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -367,3 +367,8 @@ button.current {
   height: 1.25em;
   vertical-align: text-bottom;
 }
+
+.chart {
+  position: relative;
+  height: 18rem;
+}
diff --git a/crates/web/static/graphs.js b/crates/web/static/graphs.js
new file mode 100644
index 0000000..3f9d37f
--- /dev/null
+++ b/crates/web/static/graphs.js
@@ -0,0 +1,32 @@
+// Draws the charts of the Graphs screen (spec §9 screen 5) with the vendored Chart.js. The
+// server writes each chart's configuration as JSON into its canvas's data-chart attribute; this
+// file adds only what JSON cannot carry: labels for the time axis, written in the app's time
+// zone (decision #95) rather than the browser's.
+Chart.defaults.animation = false;
+Chart.defaults.maintainAspectRatio = false;
+
+function formatter(time, withClock) {
+  const clock = withClock ? { hour: "2-digit", minute: "2-digit", hourCycle: "h23" } : {};
+  return new Intl.DateTimeFormat(time.locale, {
+    timeZone: time.zone,
+    day: "2-digit",
+    month: "2-digit",
+    ...clock,
+  });
+}
+
+for (const canvas of document.querySelectorAll("canvas[data-chart]")) {
+  const config = JSON.parse(canvas.dataset.chart);
+  const time = config.time;
+  delete config.time;
+  if (time) {
+    const day = formatter(time, false);
+    const moment = formatter(time, true);
+    config.options.scales.x.ticks = { ...config.options.scales.x.ticks, callback: (value) => day.format(value) };
+    config.options.plugins = {
+      ...config.options.plugins,
+      tooltip: { callbacks: { title: (items) => moment.format(items[0].parsed.x) } },
+    };
+  }
+  new Chart(canvas, config);
+}
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index af0dbc7..779fa22 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -12,6 +12,7 @@
   <link rel="stylesheet" href="/static/app.css">
   <script src="/static/htmx-2.0.11.min.js" defer></script>
   <script src="/static/app.js" defer></script>
+  {% block scripts %}{% endblock %}
 </head>
 <body>
   <header class="bar">
@@ -21,6 +22,7 @@
     <nav class="menu">
       <a href="/">{{ tr.t("nav-log") }}</a>
       <a href="/events">{{ tr.t("nav-events") }}</a>
+      <a href="/graphs">{{ tr.t("nav-graphs") }}</a>
       {% if me.people %}<a href="/people">{{ tr.t("nav-people") }}</a>{% endif %}
       <a href="/settings">{{ tr.t("nav-settings") }}</a>
     </nav>
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
new file mode 100644
index 0000000..3ef674c
--- /dev/null
+++ b/crates/web/templates/graphs.html
@@ -0,0 +1,34 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("graphs-title") }}{% endblock %}
+{% block scripts %}
+  <script src="/static/chart-4.5.1.umd.min.js" defer></script>
+  <script src="/static/graphs.js" defer></script>
+{% endblock %}
+{% block content %}
+<h1>{{ tr.t("graphs-title") }}</h1>
+<nav class="months">
+  <a href="/graphs?{{ previous }}">{{ tr.t("events-previous") }}</a>
+  <strong>{{ label }}</strong>
+  <a href="/graphs?{{ next }}">{{ tr.t("events-next") }}</a>
+</nav>
+<p><a href="/graphs?{{ other }}">{{ tr.t(other_key) }}</a></p>
+{% if own_only %}
+<p class="hint">{{ tr.t("graphs-own-only") }}</p>
+{% endif %}
+{% if empty %}
+<p>{{ tr.t("graphs-none") }}</p>
+{% else %}
+<section>
+  <h2>{{ tr.t("graphs-intensity") }}</h2>
+  <div class="chart">
+    <canvas data-chart="{{ intensity }}" role="img" aria-label="{{ tr.t("graphs-intensity") }}">{{ tr.t("graphs-needs-js") }}</canvas>
+  </div>
+</section>
+<section>
+  <h2>{{ tr.t(events_key) }}</h2>
+  <div class="chart">
+    <canvas data-chart="{{ events }}" role="img" aria-label="{{ tr.t(events_key) }}">{{ tr.t("graphs-needs-js") }}</canvas>
+  </div>
+</section>
+{% endif %}
+{% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index c747d63..78015be 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -130,3 +130,5 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 121 | 2026-10-14 | PWA | The **manifest** (`/static/manifest.webmanifest`, scope and start URL `/`, a "New smell report" shortcut to the Log screen) lists PNG icons of 192 and 512 px (the 512 also as maskable) and an SVG, all drawn from `static/icon.svg` with `rsvg-convert`; the **service worker** is served at `/sw.js` and goes to the network first: static assets fall back to their cached copy and pages to a cached **`/offline`** page; posts are never touched. The header's **Install app** link opens the browser's prompt where there is one and otherwise the **`/install`** page (button plus the iPhone steps, with a small Share and Add-to-Home-Screen icon beside the two steps); CSS hides it in standalone mode; a first setup lands on `/install?welcome` | Cache-first assets; an install banner; a separate offline logging queue | Spec §9.1 and §8.3 step 4 with no inline script; a deploy is never hidden behind an old cache (assets are already revalidated with ETags, #65); offline logging is out of scope for v1 (§14) |
 | 122 | 2026-10-14 | Invites | The app cuts `/invite/` paths from its own request span, but **Cloud Run's platform request log and a browser `Referer` could still carry the token**; this is **accepted** because a link is single-use and valid 5 minutes, and mitigated by `Referrer-Policy: no-referrer` on every response under `/invite/` (the global `same-origin` policy stays elsewhere) | Read the token from a URL fragment with script; a proxy that strips paths from the platform log | The platform log is readable only by the project owner and a used or expired link is dead; a fragment would need inline-free script for little gain; amends the logging claim in #119 |
 | 123 | 2026-10-14 | Graphs | The graphs' numbers come from **`smoke_domain::graphs`** (pure) over the reports the viewer may see, for **one local month counted per day or one local year counted per week** (Monday to Sunday): **intensity** = one line per person through each report's intensity (a no-smell report at 0); **events** = each event once, on the local day of its earliest visible report (the summary card's rule, #94); **room × hour** = each room's **average** level per local hour, clean checks (0) included, a report naming no room left out; **symptoms** = how many reports name each; **inside vs common** = each area's **highest** room level per day or week | A rolling 30/90/365 days; months as a year's buckets; the heatmap's highest level; symptoms counted per day | Spec §9 screen 5 names the graphs, not their arithmetic; a month matches the Events list (#100) and bounds every query, a year needs coarser buckets; an average lets "checked and clean" pull a cell down, the highest level is how a report's intensity is defined (§6) |
+| 124 | 2026-10-14 | Graphs | **Chart.js 4.5.1 is vendored** as `static/chart-4.5.1.umd.min.js` (the npm release byte for byte, **MIT**, SHA-256 pinned by a test, marked `linguist-vendored`) and loaded **only by the Graphs page**; each chart's configuration is **JSON in its canvas's `data-chart` attribute**, which `static/graphs.js` hands to Chart.js, adding only the time-axis labels, written in `APP_TIMEZONE` with `Intl` (no date adapter) | A CDN; a JSON endpoint per chart; `<script type="application/json">` blocks; `chartjs-adapter-date-fns`; SVG drawn by the server | The CSP stays `'self'` with no exception (Chart.js 4 sets styles through the CSSOM, which the CSP allows); one request per page; the numbers are built and tested in Rust and escaped by Askama; about 200 kB more binary, served with an ETag like the other assets |
+| 125 | 2026-10-14 | UI | **Graphs** (`/graphs`, in the menu for everyone signed in) show **one month** (`?month=YYYY-MM`, the current one by default) **or one year** (`?year=YYYY`), with previous and next links and a link to the other kind of period; intensity has **one line per person, named by username**, and PM2.5 on a right-hand axis; **guests** see their own data and a line saying so; a period with nothing logged says so instead of drawing empty charts | Any date range; a rolling window | The same navigation as the Events list (#100); bounded queries; spec §4 gives guests graphs of their own data |
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Expected: every test passes, including the six tests of `tests/graphs.rs`,
`the_vendored_libraries_are_the_published_releases`, `every_file_in_the_static_folder_is_served`
and `no_page_needs_inline_scripts_styles_or_eval` (now also `/graphs` and `/graphs?year=2026`).
**586** tests in the workspace. No test runs a browser (spec §13.3): put in the pull request's
checklist "open Graphs on a phone: the charts draw, the axis shows local dates, the browser
console shows no CSP error".

- [ ] **Step 5: Commit**

```bash
git add .gitattributes \
  crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/assets.rs \
  crates/web/src/i18n.rs \
  crates/web/src/lib.rs \
  crates/web/src/routes/events.rs \
  crates/web/src/routes/graphs.rs \
  crates/web/src/routes/mod.rs \
  crates/web/src/time.rs \
  crates/web/static/app.css \
  crates/web/static/chart-4.5.1.umd.min.js \
  crates/web/static/graphs.js \
  crates/web/templates/base.html \
  crates/web/templates/graphs.html \
  crates/web/tests/graphs.rs \
  crates/web/tests/security.rs \
  docs/DECISIONS.md
git commit -F - <<'EOF'
feat(graphs): Graphs screen with vendored Chart.js, intensity and events charts

/graphs shows one local month or one year: intensity over time with one line per person and
PM2.5 on a second axis, and events per day or per week. Each chart's configuration is JSON in
its canvas's data-chart attribute, drawn by static/graphs.js with the vendored Chart.js 4.5.1
(SHA-256 pinned), so the 'self'-only CSP needs no exception. Guests see their own data only.
Month parsing moves to time.rs, shared with the Events list.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 4: Graphs screen: room × hour heatmap, symptoms, inside vs common

Spec §9 screen 5, second half (decision #126). The **room × hour-of-day heatmap** is a
table drawn by the server: one row per room a report named (shown name, hidden rooms included),
one column per local hour 0–23, each cell the room's average level rounded to a whole number and
coloured by a CSS class `heat-0` … `heat-10` (the accent colour at rising strength, white text
from 6); empty where nobody checked; it scrolls sideways on a phone. **Symptoms** are horizontal
bars (`indexAxis: "y"`), most frequent first, named as shown elsewhere (`labels::symptom`);
**inside vs common** are paired bars of each area's highest level per day or week on the 0–10
axis (`null` where the area was not checked). Empty sections say so. The test harness can now
log symptoms (`log_values` is public and takes them; `symptom(key)` finds a starter symptom).
The header bar wraps on narrow phones (the menu gained a link in Task 3).

**Files:**
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/routes/graphs.rs`, `crates/web/static/app.css`, `crates/web/templates/graphs.html`, `crates/web/tests/common/mod.rs`, `crates/web/tests/graphs.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: Task 2 (`room_hours`, `symptom_counts`, `area_levels`), Task 3 (`graphs.rs`,
  `graphs.html`, the chart JSON contract), `labels::{room, symptom}`, `ListRepo::{rooms,
  symptoms}`.
- Produces: `count_chart(label, labels, counts, across: bool)` (replaces Task 3's
  `count_chart(tr, labels, counts)`), `symptom_chart`, `area_chart`, `heat_rows`; the template
  sections in this order: intensity, events, rooms table, symptoms (only when any), areas.
  Test helpers `TestApp::log_values(user, at, rooms, symptoms, choice)` and
  `TestApp::symptom(key)`. Translation keys (8): `graphs-rooms`, `graphs-rooms-hint`,
  `graphs-rooms-none`, `graphs-room`, `graphs-symptoms-label`, `graphs-symptoms-none`,
  `graphs-areas`, `graphs-areas-hint` (headings reuse `log-symptoms`, `log-inside`, `log-common`,
  `rating-level`).

- [ ] **Step 1: Write the failing tests**

````diff
diff --git a/crates/web/tests/common/mod.rs b/crates/web/tests/common/mod.rs
index 442059c..4e6e5b3 100644
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -20,7 +20,7 @@ use smoke_domain::repos::{Commit, ListRepo, Transactional, UserWrites};
 use smoke_domain::schedule::DailySchedule;
 use smoke_domain::starter::seed_starter_lists;
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{Level, Pm25, RoomId};
+use smoke_domain::types::{Level, Pm25, RoomId, SymptomId};
 use smoke_domain::types::{Locale, NewPassword, Role, Username};
 use smoke_storage_sqlite::SqliteStore;
 use smoke_web::config::{BaseUrl, WebConfig};
@@ -103,6 +103,16 @@ impl TestApp {
             .id
     }
 
+    /// The id of a starter symptom, e.g. `symptom("symptom-cough")`.
+    pub async fn symptom(&self, key: &str) -> SymptomId {
+        let symptoms = self.store.symptoms().await.unwrap();
+        symptoms
+            .iter()
+            .find(|s| s.i18n_key.as_deref() == Some(key))
+            .unwrap()
+            .id
+    }
+
     /// Logs a report through the domain, as the Log form would, at `at` (RFC 3339): every room
     /// in `rooms` at `level`, or the living room alone when `rooms` is empty.
     pub async fn log(
@@ -122,25 +132,27 @@ impl TestApp {
             .into_iter()
             .map(|room| RoomLevel { room, level })
             .collect();
-        self.log_values(user, at, rooms, choice).await
+        self.log_values(user, at, rooms, &[], choice).await
     }
 
     /// Logs "No smell" for the whole home through the domain, at `at` (RFC 3339).
     pub async fn no_smell(&self, user: &User, at: &str) -> Logged {
         let choice = EventChoice::New { replacing: None };
-        self.log_values(user, at, vec![], choice).await
+        self.log_values(user, at, vec![], &[], choice).await
     }
 
-    async fn log_values(
+    /// Logs a report with these room levels and symptoms through the domain, at `at`.
+    pub async fn log_values(
         &self,
         user: &User,
         at: &str,
         rooms: Vec<RoomLevel>,
+        symptoms: &[SymptomId],
         choice: EventChoice,
     ) -> Logged {
         let entry = SmellEntry {
             observed_at: t(at),
-            values: ReportValues::new(rooms, vec![], None).unwrap(),
+            values: ReportValues::new(rooms, symptoms.to_vec(), None).unwrap(),
             pm25: None,
         };
         let mut tx = self.store.begin(Some(user.id)).await.unwrap();
diff --git a/crates/web/tests/graphs.rs b/crates/web/tests/graphs.rs
index 43f1234..b3d3904 100644
--- a/crates/web/tests/graphs.rs
+++ b/crates/web/tests/graphs.rs
@@ -6,7 +6,8 @@ use axum::http::StatusCode;
 use common::*;
 use serde_json::Value;
 use smoke_domain::logbook::EventChoice;
-use smoke_domain::types::{Locale, Role};
+use smoke_domain::records::RoomLevel;
+use smoke_domain::types::{Level, Locale, Role};
 
 /// The Chart.js configurations on a page, in page order, read back from their `data-chart`
 /// attributes.
@@ -35,6 +36,17 @@ fn labels(chart: &Value) -> Vec<&str> {
         .collect()
 }
 
+/// The heatmap row of `room`: a cell per local hour, coloured and filled where `levels` has one.
+fn heat_row(room: &str, levels: &[(usize, u8)]) -> String {
+    let cells: String = (0..24)
+        .map(|hour| match levels.iter().find(|(h, _)| *h == hour) {
+            Some((_, level)) => format!(r#"<td class="heat-{level}">{level}</td>"#),
+            None => "<td></td>".to_owned(),
+        })
+        .collect();
+    format!(r#"<tr><th scope="row">{room}</th>{cells}</tr>"#)
+}
+
 /// Two residents and a guest, each with a report in January 2026; one PM2.5 reading each from
 /// a resident and the guest.
 async fn household() -> (TestApp, String, String) {
@@ -130,6 +142,10 @@ async fn guests_see_graphs_of_their_own_reports_only() {
         .collect();
     assert_eq!(events.iter().sum::<u64>(), 1, "only the guest's own event");
     assert_eq!(events[11], 1, "on 12 January");
+    assert!(
+        html.contains(&heat_row("Living room", &[(8, 5)])),
+        "only the guest's own report, at 08:30 in Rome: {html}"
+    );
 }
 
 #[tokio::test]
@@ -201,3 +217,121 @@ async fn the_graphs_need_a_sign_in() {
     assert_eq!(response.status(), StatusCode::SEE_OTHER);
     assert_eq!(location(&response), Some("/login"));
 }
+
+#[tokio::test]
+async fn the_room_table_averages_each_rooms_level_by_local_hour() {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let living = app.room("room-living-room").await;
+    let bedroom = app.room("room-bedroom").await;
+    let at = |room, level| RoomLevel {
+        room,
+        level: Level::new(level).unwrap(),
+    };
+    let first = app
+        .log(
+            &me,
+            "2026-01-10T18:00:00Z",
+            6,
+            &[living],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let join = EventChoice::Join(first.report.event_id.unwrap());
+    app.log_values(
+        &me,
+        "2026-01-10T18:40:00Z",
+        vec![at(living, 3), at(bedroom, 0)],
+        &[],
+        join,
+    )
+    .await;
+    app.no_smell(&me, "2026-01-11T09:00:00Z").await;
+    let cookie = app.sign_in("daniele").await;
+
+    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    assert!(html.contains("<h2>Rooms by hour of day</h2>"), "{html}");
+    let living_row = heat_row("Living room", &[(19, 5)]);
+    let bedroom_row = heat_row("Bedroom", &[(19, 0)]);
+    assert!(
+        html.contains(&living_row),
+        "(6 + 3) / 2 rounds to 5: {html}"
+    );
+    assert!(html.contains(&bedroom_row), "a clean check is a 0: {html}");
+    assert!(html.find(&living_row) < html.find(&bedroom_row));
+    assert_eq!(html.matches(r#"<th scope="row">"#).count(), 2, "{html}");
+}
+
+#[tokio::test]
+async fn symptoms_and_areas_are_compared() {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
+    let living = app.room("room-living-room").await;
+    let stairwell = app.room("room-stairwell").await;
+    let cough = app.symptom("symptom-cough").await;
+    let headache = app.symptom("symptom-headache").await;
+    let at = |room, level| RoomLevel {
+        room,
+        level: Level::new(level).unwrap(),
+    };
+    let first = app
+        .log_values(
+            &me,
+            "2026-01-10T18:00:00Z",
+            vec![at(living, 4), at(stairwell, 2)],
+            &[cough, headache],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let join = EventChoice::Join(first.report.event_id.unwrap());
+    app.log_values(
+        &partner,
+        "2026-01-10T19:00:00Z",
+        vec![at(living, 7)],
+        &[cough],
+        join,
+    )
+    .await;
+    let cookie = app.sign_in("daniele").await;
+
+    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    let charts = charts(&html);
+    let symptoms = &charts[2];
+    assert_eq!(symptoms["options"]["indexAxis"], "y");
+    assert_eq!(
+        symptoms["data"]["labels"],
+        serde_json::json!(["Cough", "Headache"])
+    );
+    assert_eq!(
+        symptoms["data"]["datasets"][0]["data"],
+        serde_json::json!([2, 1])
+    );
+    let areas = &charts[3];
+    assert_eq!(labels(areas), ["Inside", "Common areas"]);
+    assert_eq!(areas["data"]["datasets"][0]["data"][9], 7);
+    assert_eq!(areas["data"]["datasets"][1]["data"][9], 2);
+    assert_eq!(areas["data"]["datasets"][0]["data"][10], Value::Null);
+    assert_eq!(areas["options"]["scales"]["y"]["max"], 10);
+}
+
+#[tokio::test]
+async fn a_period_without_symptoms_or_rooms_says_so() {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    app.no_smell(&me, "2026-01-11T09:00:00Z").await;
+    let cookie = app.sign_in("daniele").await;
+    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    assert!(
+        html.contains("No room was checked in this period."),
+        "{html}"
+    );
+    assert!(
+        html.contains("No symptoms were reported in this period."),
+        "{html}"
+    );
+    assert_eq!(charts(&html).len(), 3, "intensity, events and areas");
+}
````

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test graphs`
Expected: the three new tests fail (no heatmap rows, only two charts on the page), and
`guests_see_graphs_of_their_own_reports_only` fails on its new heatmap assertion.

- [ ] **Step 3: Implement**

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 2e947b0..dec98cd 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -269,6 +269,14 @@ graphs-pm25 = PM2.5 (µg/m³)
 graphs-events-day = Events per day
 graphs-events-week = Events per week
 graphs-events-label = Events
+graphs-rooms = Rooms by hour of day
+graphs-rooms-hint = The average level each room was given at each hour, 0 meaning checked and clean. An empty cell means nobody checked that room at that hour.
+graphs-rooms-none = No room was checked in this period.
+graphs-room = Room
+graphs-symptoms-label = Reports
+graphs-symptoms-none = No symptoms were reported in this period.
+graphs-areas = Inside vs common areas
+graphs-areas-hint = Each bar is the highest level reported in that day or week.
 
 error-not-found-title = Page not found
 error-not-found = This page does not exist.
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 84d27dd..92795ca 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -269,6 +269,14 @@ graphs-pm25 = PM2.5 (µg/m³)
 graphs-events-day = Eventi al giorno
 graphs-events-week = Eventi alla settimana
 graphs-events-label = Eventi
+graphs-rooms = Stanze per ora del giorno
+graphs-rooms-hint = Il livello medio dato a ogni stanza in ogni ora; 0 significa controllata e pulita. Una cella vuota significa che nessuno ha controllato quella stanza a quell'ora.
+graphs-rooms-none = Nessuna stanza è stata controllata in questo periodo.
+graphs-room = Stanza
+graphs-symptoms-label = Segnalazioni
+graphs-symptoms-none = Nessun sintomo segnalato in questo periodo.
+graphs-areas = In casa e parti comuni a confronto
+graphs-areas-hint = Ogni barra è il livello più alto segnalato in quel giorno o in quella settimana.
 
 error-not-found-title = Pagina non trovata
 error-not-found = Questa pagina non esiste.
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 862d508..b331acb 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -269,6 +269,14 @@ graphs-pm25 = PM2.5 (µg/m³)
 graphs-events-day = Günlük olay sayısı
 graphs-events-week = Haftalık olay sayısı
 graphs-events-label = Olaylar
+graphs-rooms = Günün saatlerine göre odalar
+graphs-rooms-hint = Her odaya her saatte verilen ortalama seviye; 0 kontrol edildi ve temiz demektir. Boş bir hücre, o saatte o odayı kimsenin kontrol etmediği anlamına gelir.
+graphs-rooms-none = Bu dönemde hiçbir oda kontrol edilmedi.
+graphs-room = Oda
+graphs-symptoms-label = Bildirimler
+graphs-symptoms-none = Bu dönemde hiçbir belirti bildirilmedi.
+graphs-areas = Ev içi ve ortak alanlar karşılaştırması
+graphs-areas-hint = Her çubuk, o gün veya hafta bildirilen en yüksek seviyedir.
 
 error-not-found-title = Sayfa bulunamadı
 error-not-found = Bu sayfa mevcut değil.
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
index 36385be..30f79e8 100644
--- a/crates/web/src/routes/graphs.rs
+++ b/crates/web/src/routes/graphs.rs
@@ -1,7 +1,8 @@
 //! The Graphs screen (spec §9 screen 5). Each chart is a `<canvas>` whose Chart.js
 //! configuration travels as JSON in its `data-chart` attribute; `static/graphs.js` draws it, so
-//! no page carries an inline script. The numbers come from `smoke_domain::graphs`, over what the
-//! viewer may see: guests get graphs of their own data only (spec §4).
+//! no page carries an inline script. The room × hour heatmap is a plain table, coloured by CSS
+//! classes. The numbers come from `smoke_domain::graphs`, over what the viewer may see: guests
+//! get graphs of their own data only (spec §4).
 
 use std::collections::HashMap;
 
@@ -13,9 +14,12 @@ use chrono::{DateTime, Datelike, NaiveDate, TimeDelta, Utc};
 use chrono_tz::Tz;
 use serde::Deserialize;
 use serde_json::{Value, json};
-use smoke_domain::graphs::{Period, PersonLine, events_per_bucket, intensity_lines};
+use smoke_domain::graphs::{
+    AreaLevels, Period, PersonLine, RoomHours, area_levels, events_per_bucket, intensity_lines,
+    room_hours, symptom_counts,
+};
 use smoke_domain::permissions::{Action, can, can_see};
-use smoke_domain::records::Pm25Reading;
+use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
 use smoke_domain::repos::Store;
 use smoke_domain::types::{Locale, UserId};
 
@@ -23,6 +27,7 @@ use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::header::SignedIn;
 use crate::i18n::Tr;
+use crate::labels;
 use crate::state::AppState;
 use crate::time;
 
@@ -164,21 +169,96 @@ fn bucket_labels(tr: Tr, period: Period, buckets: &[NaiveDate]) -> Vec<String> {
         .collect()
 }
 
-/// A bar chart of whole numbers, one bar per bucket and no legend.
-fn count_chart(tr: Tr, labels: Vec<String>, counts: Vec<usize>) -> Value {
+/// A bar chart of whole numbers with no legend; `across` lays the bars along the y axis, for
+/// long labels such as symptom names.
+fn count_chart(label: String, labels: Vec<String>, counts: Vec<usize>, across: bool) -> Value {
+    let (index, value) = if across { ("y", "x") } else { ("x", "y") };
     json!({
         "type": "bar",
         "data": {
             "labels": labels,
-            "datasets": [{ "label": tr.t("graphs-events-label"), "data": counts }],
+            "datasets": [{ "label": label, "data": counts }],
         },
         "options": {
-            "scales": { "y": { "beginAtZero": true, "ticks": { "precision": 0 } } },
+            "indexAxis": index,
+            "scales": { value: { "beginAtZero": true, "ticks": { "precision": 0 } } },
             "plugins": { "legend": { "display": false } },
         },
     })
 }
 
+/// How often each symptom was reported, most frequent first, by its shown name.
+fn symptom_chart(tr: Tr, reports: &[SmellReport], symptoms: &[Symptom]) -> Option<Value> {
+    let counts = symptom_counts(reports);
+    if counts.is_empty() {
+        return None;
+    }
+    let (names, counts) = counts
+        .into_iter()
+        .filter_map(|(id, count)| {
+            let symptom = symptoms.iter().find(|s| s.id == id)?;
+            Some((labels::symptom(tr, symptom), count))
+        })
+        .unzip();
+    Some(count_chart(
+        tr.t("graphs-symptoms-label"),
+        names,
+        counts,
+        true,
+    ))
+}
+
+/// Inside the home against common areas: each area's highest level per bucket, side by side.
+fn area_chart(tr: Tr, labels: Vec<String>, levels: &[(NaiveDate, AreaLevels)]) -> Value {
+    let series = |pick: fn(&AreaLevels) -> Option<u8>| -> Vec<Option<u8>> {
+        levels.iter().map(|(_, l)| pick(l)).collect()
+    };
+    json!({
+        "type": "bar",
+        "data": {
+            "labels": labels,
+            "datasets": [
+                { "label": tr.t("log-inside"), "data": series(|l| l.inside.map(|v| v.get())) },
+                { "label": tr.t("log-common"), "data": series(|l| l.common.map(|v| v.get())) },
+            ],
+        },
+        "options": {
+            "scales": {
+                "y": {
+                    "min": 0,
+                    "max": 10,
+                    "title": { "display": true, "text": tr.t("rating-level") },
+                },
+            },
+        },
+    })
+}
+
+/// One cell of the heatmap: the room's average level at that hour, rounded, which also picks
+/// the colour (`heat-0` to `heat-10`). `None` where nobody checked the room.
+type HeatCell = Option<u8>;
+
+struct HeatRow {
+    room: String,
+    cells: Vec<HeatCell>,
+}
+
+fn heat_rows(tr: Tr, rows: Vec<RoomHours>, rooms: &[Room]) -> Vec<HeatRow> {
+    rows.into_iter()
+        .filter_map(|row| {
+            let room = rooms.iter().find(|r| r.id == row.room)?;
+            Some(HeatRow {
+                room: labels::room(tr, room),
+                cells: row
+                    .hours
+                    .iter()
+                    .map(|tally| tally.mean().map(|mean| mean.round() as u8))
+                    .collect(),
+            })
+        })
+        .collect()
+}
+
 #[derive(Template)]
 #[template(path = "graphs.html")]
 struct GraphsPage {
@@ -195,6 +275,10 @@ struct GraphsPage {
     intensity: String,
     events_key: &'static str,
     events: String,
+    hours: std::ops::Range<u8>,
+    heatmap: Vec<HeatRow>,
+    symptoms: Option<String>,
+    areas: String,
 }
 
 pub(crate) async fn show<S: Store + Clone + 'static>(
@@ -238,6 +322,9 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
         .into_iter()
         .map(|(_, count)| count)
         .collect();
+    let rooms = state.store.rooms().await?;
+    let symptoms = state.store.symptoms().await?;
+    let areas = area_levels(&reports, &rooms, period, schedule);
     let lines = intensity_lines(&reports);
     let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule.tz());
 
@@ -267,7 +354,12 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
             empty: reports.is_empty() && readings.is_empty(),
             intensity: intensity.to_string(),
             events_key,
-            events: count_chart(tr, labels, events).to_string(),
+            events: count_chart(tr.t("graphs-events-label"), labels.clone(), events, false)
+                .to_string(),
+            hours: 0..24,
+            heatmap: heat_rows(tr, room_hours(&reports, schedule), &rooms),
+            symptoms: symptom_chart(tr, &reports, &symptoms).map(|chart| chart.to_string()),
+            areas: area_chart(tr, labels, &areas).to_string(),
         },
     )
 }
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index 4161711..d1bc594 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -14,6 +14,7 @@ body {
 
 .bar {
   display: flex;
+  flex-wrap: wrap;
   align-items: center;
   justify-content: space-between;
   padding: var(--gap) 0;
@@ -372,3 +373,73 @@ button.current {
   position: relative;
   height: 18rem;
 }
+
+.scroll {
+  overflow-x: auto;
+}
+
+table.heatmap {
+  border-collapse: collapse;
+  font-size: 0.75rem;
+}
+
+table.heatmap th,
+table.heatmap td {
+  min-width: 1.25rem;
+  padding: 0.25rem 0.125rem;
+  text-align: center;
+}
+
+table.heatmap tbody th {
+  text-align: left;
+  white-space: nowrap;
+}
+
+.heat-0 {
+  background: color-mix(in srgb, var(--accent) 6%, transparent);
+}
+
+.heat-1 {
+  background: color-mix(in srgb, var(--accent) 12%, transparent);
+}
+
+.heat-2 {
+  background: color-mix(in srgb, var(--accent) 20%, transparent);
+}
+
+.heat-3 {
+  background: color-mix(in srgb, var(--accent) 28%, transparent);
+}
+
+.heat-4 {
+  background: color-mix(in srgb, var(--accent) 36%, transparent);
+}
+
+.heat-5 {
+  background: color-mix(in srgb, var(--accent) 44%, transparent);
+}
+
+.heat-6 {
+  background: color-mix(in srgb, var(--accent) 54%, transparent);
+  color: #fff;
+}
+
+.heat-7 {
+  background: color-mix(in srgb, var(--accent) 64%, transparent);
+  color: #fff;
+}
+
+.heat-8 {
+  background: color-mix(in srgb, var(--accent) 76%, transparent);
+  color: #fff;
+}
+
+.heat-9 {
+  background: color-mix(in srgb, var(--accent) 88%, transparent);
+  color: #fff;
+}
+
+.heat-10 {
+  background: color-mix(in srgb, var(--accent) 100%, transparent);
+  color: #fff;
+}
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
index 3ef674c..205a1f9 100644
--- a/crates/web/templates/graphs.html
+++ b/crates/web/templates/graphs.html
@@ -30,5 +30,42 @@
     <canvas data-chart="{{ events }}" role="img" aria-label="{{ tr.t(events_key) }}">{{ tr.t("graphs-needs-js") }}</canvas>
   </div>
 </section>
+<section>
+  <h2>{{ tr.t("graphs-rooms") }}</h2>
+  {% if heatmap.is_empty() %}
+  <p>{{ tr.t("graphs-rooms-none") }}</p>
+  {% else %}
+  <p class="hint">{{ tr.t("graphs-rooms-hint") }}</p>
+  <div class="scroll">
+    <table class="heatmap">
+      <thead>
+        <tr><th scope="col">{{ tr.t("graphs-room") }}</th>{% for hour in hours.clone() %}<th scope="col">{{ hour }}</th>{% endfor %}</tr>
+      </thead>
+      <tbody>
+        {% for row in heatmap %}
+        <tr><th scope="row">{{ row.room }}</th>{% for cell in row.cells %}{% if let Some(level) = cell %}<td class="heat-{{ level }}">{{ level }}</td>{% else %}<td></td>{% endif %}{% endfor %}</tr>
+        {% endfor %}
+      </tbody>
+    </table>
+  </div>
+  {% endif %}
+</section>
+<section>
+  <h2>{{ tr.t("log-symptoms") }}</h2>
+  {% if let Some(symptoms) = symptoms %}
+  <div class="chart">
+    <canvas data-chart="{{ symptoms }}" role="img" aria-label="{{ tr.t("log-symptoms") }}">{{ tr.t("graphs-needs-js") }}</canvas>
+  </div>
+  {% else %}
+  <p>{{ tr.t("graphs-symptoms-none") }}</p>
+  {% endif %}
+</section>
+<section>
+  <h2>{{ tr.t("graphs-areas") }}</h2>
+  <p class="hint">{{ tr.t("graphs-areas-hint") }}</p>
+  <div class="chart">
+    <canvas data-chart="{{ areas }}" role="img" aria-label="{{ tr.t("graphs-areas") }}">{{ tr.t("graphs-needs-js") }}</canvas>
+  </div>
+</section>
 {% endif %}
 {% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 78015be..8ff346e 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -132,3 +132,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 123 | 2026-10-14 | Graphs | The graphs' numbers come from **`smoke_domain::graphs`** (pure) over the reports the viewer may see, for **one local month counted per day or one local year counted per week** (Monday to Sunday): **intensity** = one line per person through each report's intensity (a no-smell report at 0); **events** = each event once, on the local day of its earliest visible report (the summary card's rule, #94); **room × hour** = each room's **average** level per local hour, clean checks (0) included, a report naming no room left out; **symptoms** = how many reports name each; **inside vs common** = each area's **highest** room level per day or week | A rolling 30/90/365 days; months as a year's buckets; the heatmap's highest level; symptoms counted per day | Spec §9 screen 5 names the graphs, not their arithmetic; a month matches the Events list (#100) and bounds every query, a year needs coarser buckets; an average lets "checked and clean" pull a cell down, the highest level is how a report's intensity is defined (§6) |
 | 124 | 2026-10-14 | Graphs | **Chart.js 4.5.1 is vendored** as `static/chart-4.5.1.umd.min.js` (the npm release byte for byte, **MIT**, SHA-256 pinned by a test, marked `linguist-vendored`) and loaded **only by the Graphs page**; each chart's configuration is **JSON in its canvas's `data-chart` attribute**, which `static/graphs.js` hands to Chart.js, adding only the time-axis labels, written in `APP_TIMEZONE` with `Intl` (no date adapter) | A CDN; a JSON endpoint per chart; `<script type="application/json">` blocks; `chartjs-adapter-date-fns`; SVG drawn by the server | The CSP stays `'self'` with no exception (Chart.js 4 sets styles through the CSSOM, which the CSP allows); one request per page; the numbers are built and tested in Rust and escaped by Askama; about 200 kB more binary, served with an ETag like the other assets |
 | 125 | 2026-10-14 | UI | **Graphs** (`/graphs`, in the menu for everyone signed in) show **one month** (`?month=YYYY-MM`, the current one by default) **or one year** (`?year=YYYY`), with previous and next links and a link to the other kind of period; intensity has **one line per person, named by username**, and PM2.5 on a right-hand axis; **guests** see their own data and a line saying so; a period with nothing logged says so instead of drawing empty charts | Any date range; a rolling window | The same navigation as the Events list (#100); bounded queries; spec §4 gives guests graphs of their own data |
+| 126 | 2026-10-14 | Graphs | The **room × hour heatmap is an HTML table** drawn by the server: one row per room a report named, 24 local hours, each cell the rounded average level with a CSS class `heat-0`…`heat-10` (the accent colour at rising strength), empty where nobody checked; the **symptoms** are horizontal bars, most frequent first; **inside vs common** are paired bars on the 0–10 axis | The `chartjs-chart-matrix` plugin; a bubble chart; inline `style` colours | No second library; works without JavaScript and reads as a table to screen readers; classes keep the CSP free of inline styles; the header now wraps on narrow phones because the menu gained Graphs |
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Expected: the nine tests of `tests/graphs.rs` pass, and the CSP scan still passes with the
heatmap's classes. **589** tests in the workspace.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/routes/graphs.rs \
  crates/web/static/app.css \
  crates/web/templates/graphs.html \
  crates/web/tests/common/mod.rs \
  crates/web/tests/graphs.rs \
  docs/DECISIONS.md
git commit -F - <<'EOF'
feat(graphs): room-by-hour heatmap table, symptom frequency, inside vs common areas

The Graphs screen gains the last three graphs of spec §9 screen 5: a server-drawn table of
each room's average level by local hour, coloured by CSS classes so the CSP needs no inline
style; horizontal bars of how many reports name each symptom; and paired bars of the highest
level inside the home and in common areas per day or week. The test harness can log
symptoms, and the header wraps on narrow phones.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 5: Settings → Guides: How to log

Spec §5 and §9 screen 7 (decision #127). `/guide/logging` shows the "What happened /
Correct action" table, the five rules of thumb and the note on different sensitivities, all as
translation entries; the auto-close rule names the configured `AUTO_CLOSE_HOUR` (`{ $hour }`,
"04:00" by default). Settings → Guides lists both guides; the two guide pages link to each
other. The English texts are the spec's words (the README repeats them in Task 7). Every
signed-in role can open the guides.

**Files:**
- Create: `crates/web/templates/guide_logging.html`, `crates/web/tests/guides.rs`
- Modify: `crates/web/locales/en/main.ftl`, `crates/web/locales/it/main.ftl`, `crates/web/locales/tr/main.ftl`, `crates/web/src/lib.rs`, `crates/web/src/routes/guide.rs`, `crates/web/static/app.css`, `crates/web/templates/guide.html`, `crates/web/templates/settings.html`, `crates/web/tests/security.rs`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: `DailySchedule::hour`, `Tr::t1`, the rating guide (`guide::levels`, decision #97).
- Produces: route `GET /guide/logging` → `routes::guide::logging`; template
  `guide_logging.html`; `pub(crate) fn situations(Tr) -> Vec<Situation>` (`happened`, `action`)
  and `pub(crate) fn rules(Tr, auto_close_hour: u32) -> Vec<String>` (used by Task 7's README
  test); `guide::keys()` also lists the new keys. Translation keys (17): `guide-log-title`,
  `guide-log-happened`, `guide-log-action`, `guide-log-{mistake,changed,gone}` and their
  `-action`, `guide-log-rules`, `guide-log-rule-1` … `-5`, `guide-log-sensitivity-title`,
  `guide-log-sensitivity`.

- [ ] **Step 1: Write the failing tests**

The integration tests below. The unit test `the_auto_close_rule_names_the_configured_hour`
is part of the Step 3 diff (`crates/web/src/routes/guide.rs`).

````diff
diff --git a/crates/web/tests/guides.rs b/crates/web/tests/guides.rs
new file mode 100644
index 0000000..65b7646
--- /dev/null
+++ b/crates/web/tests/guides.rs
@@ -0,0 +1,60 @@
+//! Settings → Guides: *How to rate* and *How to log* (spec §5).
+
+mod common;
+
+use axum::http::StatusCode;
+use common::*;
+use smoke_domain::types::{Locale, Role};
+
+#[tokio::test]
+async fn the_logging_guide_has_the_table_and_rules_of_thumb() {
+    let app = TestApp::new().await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let cookie = app.sign_in("guest").await;
+    let response = app.send(get_with("/guide/logging", &cookie)).await;
+    assert_eq!(response.status(), StatusCode::OK);
+    let html = text(response).await;
+    assert!(html.contains("<h1>How to log</h1>"), "{html}");
+    assert!(
+        html.contains(
+            "<tr><td>The smell went away completely and came back later</td>\
+             <td>Start a new event</td></tr>"
+        ),
+        "{html}"
+    );
+    assert_eq!(html.matches("<li>").count(), 5, "{html}");
+    assert!(
+        html.contains("Forgetting is fine (04:00 auto-close)"),
+        "the configured hour: {html}"
+    );
+    assert!(html.contains(r#"<a href="/guide/rating">How to rate</a>"#));
+}
+
+#[tokio::test]
+async fn settings_and_the_rating_guide_lead_to_both_guides() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let settings = text(app.send(get_with("/settings", &cookie)).await).await;
+    for link in [
+        r#"<a href="/guide/rating">How to rate</a>"#,
+        r#"<a href="/guide/logging">How to log</a>"#,
+    ] {
+        assert!(settings.contains(link), "{link}: {settings}");
+    }
+    let rating = text(app.send(get_with("/guide/rating", &cookie)).await).await;
+    assert!(
+        rating.contains(r#"<a href="/guide/logging">How to log</a>"#),
+        "{rating}"
+    );
+}
+
+#[tokio::test]
+async fn the_guides_are_translated() {
+    let app = TestApp::new().await;
+    app.add_user("partner", Role::Resident, Locale::It).await;
+    let cookie = app.sign_in("partner").await;
+    let html = text(app.send(get_with("/guide/logging", &cookie)).await).await;
+    assert!(html.contains("<h1>Come registrare</h1>"), "{html}");
+    assert!(html.contains("chiusura automatica alle 04:00"), "{html}");
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index eba4649..e505c79 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -236,6 +236,7 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         text(app.send(get("/offline")).await).await,
         text(app.send(get_with("/graphs", &cookie)).await).await,
         text(app.send(get_with("/graphs?year=2026", &cookie)).await).await,
+        text(app.send(get_with("/guide/logging", &cookie)).await).await,
         text(app.send(get("/missing")).await).await,
         text(app.attempt("daniele", "not the password").await).await,
     ];
````

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --test guides`
Expected: all three tests fail (`/guide/logging` answers 404; Settings has no link to it).

- [ ] **Step 3: Implement**

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index dec98cd..d447c99 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -116,6 +116,24 @@ intensity-9-desc = Like standing next to someone smoking.
 intensity-10-name = Unbearable
 intensity-10-desc = Like a smoke-filled room. You must leave.
 
+guide-log-title = How to log
+guide-log-happened = What happened
+guide-log-action = Correct action
+guide-log-mistake = I made a mistake (wrong number, forgot a room, wrong symptom)
+guide-log-mistake-action = Correct that report (edit; recorded in history)
+guide-log-changed = The situation changed (stronger/weaker, a room started or stopped, symptoms started)
+guide-log-changed-action = Add a new report to the same event
+guide-log-gone = The smell went away completely and came back later
+guide-log-gone-action = Start a new event
+guide-log-rules = Rules of thumb
+guide-log-rule-1 = Never edit to show a change; add a report. Editing changes the past; adding records the present.
+guide-log-rule-2 = Add a report when a room's level changes by 2 or more, a room starts or clears (log it at 0), or a new symptom appears. Each report lists only the rooms you checked: a room you leave out is "not checked", never "clean".
+guide-log-rule-3 = Unsure whether it's the same event? If the smell was completely gone at some point, it's a new one.
+guide-log-rule-4 = Close the event when the smell is gone. Forgetting is fine ({ $hour } auto-close), but closing gives accurate durations.
+guide-log-rule-5 = Nothing smells? Log No smell. It keeps a record that you checked.
+guide-log-sensitivity-title = Different sensitivities
+guide-log-sensitivity = Each person rates on their own 1–10 scale. Overlapping reports with different values are kept as-is. Graphs show one line per person; the PM2.5 reading is the objective reference.
+
 nav-log = Log
 nav-events = Events
 nav-graphs = Graphs
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 92795ca..e138d57 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -116,6 +116,24 @@ intensity-9-desc = Come stare accanto a qualcuno che fuma.
 intensity-10-name = Insopportabile
 intensity-10-desc = Come una stanza piena di fumo. Devi uscire.
 
+guide-log-title = Come registrare
+guide-log-happened = Cosa è successo
+guide-log-action = Cosa fare
+guide-log-mistake = Ho sbagliato (numero sbagliato, stanza dimenticata, sintomo sbagliato)
+guide-log-mistake-action = Correggi quella segnalazione (modifica; resta nella cronologia)
+guide-log-changed = La situazione è cambiata (più forte/più debole, una stanza è iniziata o finita, sono iniziati sintomi)
+guide-log-changed-action = Aggiungi una nuova segnalazione allo stesso evento
+guide-log-gone = L'odore è sparito del tutto ed è tornato più tardi
+guide-log-gone-action = Inizia un nuovo evento
+guide-log-rules = Regole pratiche
+guide-log-rule-1 = Non modificare mai per mostrare un cambiamento: aggiungi una segnalazione. Modificare cambia il passato; aggiungere registra il presente.
+guide-log-rule-2 = Aggiungi una segnalazione quando il livello di una stanza cambia di 2 o più, quando in una stanza l'odore inizia o sparisce (registrala a 0) o quando compare un nuovo sintomo. Ogni segnalazione elenca solo le stanze che hai controllato: una stanza che non indichi è "non controllata", mai "pulita".
+guide-log-rule-3 = Non sai se è lo stesso evento? Se a un certo punto l'odore era sparito del tutto, è un evento nuovo.
+guide-log-rule-4 = Chiudi l'evento quando l'odore è sparito. Se te ne dimentichi va bene (chiusura automatica alle { $hour }), ma chiudendolo le durate sono precise.
+guide-log-rule-5 = Non senti odore? Registra Nessun odore. Resta traccia che hai controllato.
+guide-log-sensitivity-title = Sensibilità diverse
+guide-log-sensitivity = Ognuno valuta sulla propria scala da 1 a 10. Le segnalazioni sovrapposte con valori diversi restano così come sono. I grafici mostrano una linea per persona; la misura del PM2.5 è il riferimento oggettivo.
+
 nav-log = Registra
 nav-events = Eventi
 nav-graphs = Grafici
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index b331acb..7a30f7d 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -116,6 +116,24 @@ intensity-9-desc = Sigara içen birinin yanında durmak gibi.
 intensity-10-name = Dayanılmaz
 intensity-10-desc = Dumanla dolu bir oda gibi. Çıkmak zorundasın.
 
+guide-log-title = Nasıl kaydedilir
+guide-log-happened = Ne oldu
+guide-log-action = Doğru adım
+guide-log-mistake = Bir hata yaptım (yanlış sayı, unutulan oda, yanlış belirti)
+guide-log-mistake-action = O bildirimi düzelt (düzenleme; geçmişte kayıtlı kalır)
+guide-log-changed = Durum değişti (daha güçlü/daha zayıf, bir odada başladı ya da bitti, belirtiler başladı)
+guide-log-changed-action = Aynı olaya yeni bir bildirim ekle
+guide-log-gone = Koku tamamen geçti ve daha sonra geri geldi
+guide-log-gone-action = Yeni bir olay başlat
+guide-log-rules = Temel kurallar
+guide-log-rule-1 = Bir değişikliği göstermek için asla düzenleme yapma; bildirim ekle. Düzenlemek geçmişi değiştirir; eklemek şimdiyi kaydeder.
+guide-log-rule-2 = Bir odanın seviyesi 2 veya daha fazla değiştiğinde, bir odada koku başladığında ya da geçtiğinde (0 olarak kaydet) veya yeni bir belirti ortaya çıktığında bildirim ekle. Her bildirim yalnızca kontrol ettiğin odaları listeler: eklemediğin bir oda "kontrol edilmedi" demektir, asla "temiz" değil.
+guide-log-rule-3 = Aynı olay olup olmadığından emin değil misin? Koku bir noktada tamamen geçtiyse, bu yeni bir olaydır.
+guide-log-rule-4 = Koku geçince olayı kapat. Unutman sorun değil ({ $hour } otomatik kapanış), ama kapatmak süreleri doğru tutar.
+guide-log-rule-5 = Koku yok mu? Koku yok olarak kaydet. Kontrol ettiğinin kaydı kalır.
+guide-log-sensitivity-title = Farklı hassasiyetler
+guide-log-sensitivity = Herkes kendi 1–10 ölçeğine göre puanlar. Farklı değerler içeren çakışan bildirimler olduğu gibi saklanır. Grafikler kişi başına bir çizgi gösterir; PM2.5 ölçümü nesnel referanstır.
+
 nav-log = Kaydet
 nav-events = Olaylar
 nav-graphs = Grafikler
diff --git a/crates/web/src/lib.rs b/crates/web/src/lib.rs
index 85cd877..92decb9 100644
--- a/crates/web/src/lib.rs
+++ b/crates/web/src/lib.rs
@@ -46,6 +46,7 @@ pub fn router<S: Store + Clone + 'static>(state: AppState<S>) -> Router {
     let signed_in = Router::new()
         .route("/", get(routes::log::show::<S>))
         .route("/guide/rating", get(routes::guide::rating::<S>))
+        .route("/guide/logging", get(routes::guide::logging::<S>))
         .route("/log", post(routes::entries::submit::<S>))
         .route("/notices/dismiss", post(routes::notices::dismiss::<S>))
         .route("/events", get(routes::events::list::<S>))
diff --git a/crates/web/src/routes/guide.rs b/crates/web/src/routes/guide.rs
index 4173eb3..4741171 100644
--- a/crates/web/src/routes/guide.rs
+++ b/crates/web/src/routes/guide.rs
@@ -1,5 +1,6 @@
-//! The rating guide (spec §5): a compact version in a popup on the Log screen, and this full
-//! page. The texts live in the translation files so they are easy to edit.
+//! The guides of spec §5, under Settings → Guides: *How to rate* (also as a compact popup on the
+//! Log screen) and *How to log*. The texts live in the translation files so they are easy to
+//! edit, and the README repeats the English ones (a test keeps them equal).
 
 use askama::Template;
 use axum::extract::State;
@@ -31,12 +32,46 @@ pub(crate) fn levels(tr: Tr) -> Vec<GuideRow> {
         .collect()
 }
 
-/// Every key [`levels`] looks up, for the translation tests.
+/// The situations of the "How to log" table, in the spec's order.
+const SITUATIONS: [&str; 3] = ["mistake", "changed", "gone"];
+
+/// The rules of thumb, numbered from 1.
+const RULES: u8 = 5;
+
+/// One row of the "How to log" table: what happened, and what to do about it.
+pub(crate) struct Situation {
+    pub(crate) happened: String,
+    pub(crate) action: String,
+}
+
+pub(crate) fn situations(tr: Tr) -> Vec<Situation> {
+    SITUATIONS
+        .iter()
+        .map(|s| Situation {
+            happened: tr.t(&format!("guide-log-{s}")),
+            action: tr.t(&format!("guide-log-{s}-action")),
+        })
+        .collect()
+}
+
+/// The rules of thumb. The auto-close rule names the configured hour (`AUTO_CLOSE_HOUR`).
+pub(crate) fn rules(tr: Tr, auto_close_hour: u32) -> Vec<String> {
+    let hour = format!("{auto_close_hour:02}:00");
+    (1..=RULES)
+        .map(|n| tr.t1(&format!("guide-log-rule-{n}"), "hour", &hour))
+        .collect()
+}
+
+/// Every key [`levels`], [`situations`] and [`rules`] look up, for the translation tests.
 #[cfg(test)]
 pub(crate) fn keys() -> Vec<String> {
-    (Level::MIN..=Level::MAX)
-        .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")])
-        .collect()
+    let levels = (Level::MIN..=Level::MAX)
+        .flat_map(|v| [format!("intensity-{v}-name"), format!("intensity-{v}-desc")]);
+    let situations = SITUATIONS
+        .iter()
+        .flat_map(|s| [format!("guide-log-{s}"), format!("guide-log-{s}-action")]);
+    let rules = (1..=RULES).map(|n| format!("guide-log-rule-{n}"));
+    levels.chain(situations).chain(rules).collect()
 }
 
 #[derive(Template)]
@@ -61,3 +96,42 @@ pub(crate) async fn rating<S: Store + Clone + 'static>(
         },
     )
 }
+
+#[derive(Template)]
+#[template(path = "guide_logging.html")]
+struct LoggingPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    situations: Vec<Situation>,
+    rules: Vec<String>,
+}
+
+pub(crate) async fn logging<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+) -> Result<Response, AppError> {
+    page(
+        StatusCode::OK,
+        &LoggingPage {
+            tr,
+            signed_in: SignedIn::of(&user, state.clock.now()),
+            situations: situations(tr),
+            rules: rules(tr, state.config.schedule.hour()),
+        },
+    )
+}
+
+#[cfg(test)]
+mod tests {
+    use smoke_domain::types::Locale;
+
+    use super::*;
+
+    #[test]
+    fn the_auto_close_rule_names_the_configured_hour() {
+        let rules = rules(Tr(Locale::En), 5);
+        assert_eq!(rules.len(), 5);
+        assert!(rules[3].contains("(05:00 auto-close)"), "{}", rules[3]);
+    }
+}
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
index d1bc594..74e731e 100644
--- a/crates/web/static/app.css
+++ b/crates/web/static/app.css
@@ -443,3 +443,7 @@ table.heatmap tbody th {
   background: color-mix(in srgb, var(--accent) 100%, transparent);
   color: #fff;
 }
+
+.rules li {
+  margin: 0.5rem 0;
+}
diff --git a/crates/web/templates/guide.html b/crates/web/templates/guide.html
index 9fa5e74..9bc81a8 100644
--- a/crates/web/templates/guide.html
+++ b/crates/web/templates/guide.html
@@ -13,5 +13,5 @@
   </tbody>
 </table>
 <p>{{ tr.t("rating-tip") }}</p>
-<p><a href="/">{{ tr.t("error-back-home") }}</a></p>
+<p><a href="/guide/logging">{{ tr.t("guide-log-title") }}</a> · <a href="/">{{ tr.t("error-back-home") }}</a></p>
 {% endblock %}
diff --git a/crates/web/templates/guide_logging.html b/crates/web/templates/guide_logging.html
new file mode 100644
index 0000000..276c334
--- /dev/null
+++ b/crates/web/templates/guide_logging.html
@@ -0,0 +1,24 @@
+{% extends "base.html" %}
+{% block title %}{{ tr.t("guide-log-title") }}{% endblock %}
+{% block content %}
+<h1>{{ tr.t("guide-log-title") }}</h1>
+<table class="guide">
+  <thead>
+    <tr><th scope="col">{{ tr.t("guide-log-happened") }}</th><th scope="col">{{ tr.t("guide-log-action") }}</th></tr>
+  </thead>
+  <tbody>
+    {% for situation in situations %}
+    <tr><td>{{ situation.happened }}</td><td>{{ situation.action }}</td></tr>
+    {% endfor %}
+  </tbody>
+</table>
+<h2>{{ tr.t("guide-log-rules") }}</h2>
+<ol class="rules">
+  {% for rule in rules %}
+  <li>{{ rule }}</li>
+  {% endfor %}
+</ol>
+<h2>{{ tr.t("guide-log-sensitivity-title") }}</h2>
+<p>{{ tr.t("guide-log-sensitivity") }}</p>
+<p><a href="/guide/rating">{{ tr.t("rating-title") }}</a> · <a href="/">{{ tr.t("error-back-home") }}</a></p>
+{% endblock %}
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
index a54fcdc..a5f321c 100644
--- a/crates/web/templates/settings.html
+++ b/crates/web/templates/settings.html
@@ -74,6 +74,7 @@
   <h2>{{ tr.t("settings-guides") }}</h2>
   <ul class="rows">
     <li><a href="/guide/rating">{{ tr.t("rating-title") }}</a></li>
+    <li><a href="/guide/logging">{{ tr.t("guide-log-title") }}</a></li>
   </ul>
 </section>
 {% endblock %}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 8ff346e..d1d72f3 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -133,3 +133,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 124 | 2026-10-14 | Graphs | **Chart.js 4.5.1 is vendored** as `static/chart-4.5.1.umd.min.js` (the npm release byte for byte, **MIT**, SHA-256 pinned by a test, marked `linguist-vendored`) and loaded **only by the Graphs page**; each chart's configuration is **JSON in its canvas's `data-chart` attribute**, which `static/graphs.js` hands to Chart.js, adding only the time-axis labels, written in `APP_TIMEZONE` with `Intl` (no date adapter) | A CDN; a JSON endpoint per chart; `<script type="application/json">` blocks; `chartjs-adapter-date-fns`; SVG drawn by the server | The CSP stays `'self'` with no exception (Chart.js 4 sets styles through the CSSOM, which the CSP allows); one request per page; the numbers are built and tested in Rust and escaped by Askama; about 200 kB more binary, served with an ETag like the other assets |
 | 125 | 2026-10-14 | UI | **Graphs** (`/graphs`, in the menu for everyone signed in) show **one month** (`?month=YYYY-MM`, the current one by default) **or one year** (`?year=YYYY`), with previous and next links and a link to the other kind of period; intensity has **one line per person, named by username**, and PM2.5 on a right-hand axis; **guests** see their own data and a line saying so; a period with nothing logged says so instead of drawing empty charts | Any date range; a rolling window | The same navigation as the Events list (#100); bounded queries; spec §4 gives guests graphs of their own data |
 | 126 | 2026-10-14 | Graphs | The **room × hour heatmap is an HTML table** drawn by the server: one row per room a report named, 24 local hours, each cell the rounded average level with a CSS class `heat-0`…`heat-10` (the accent colour at rising strength), empty where nobody checked; the **symptoms** are horizontal bars, most frequent first; **inside vs common** are paired bars on the 0–10 axis | The `chartjs-chart-matrix` plugin; a bubble chart; inline `style` colours | No second library; works without JavaScript and reads as a table to screen readers; classes keep the CSP free of inline styles; the header now wraps on narrow phones because the menu gained Graphs |
+| 127 | 2026-10-14 | UI | **Settings → Guides** lists *How to rate* (`/guide/rating`, unchanged since #97) and the new ***How to log*** (`/guide/logging`): the spec §5 table, the five rules of thumb (the auto-close one names the configured `AUTO_CLOSE_HOUR`) and the note on different sensitivities; the two guides link to each other, and every signed-in role can open them | Move the pages to `/settings/guides/…`; one combined guide page | Settings already links the guides (M5), so "under Settings" needs no new address and the popup's "Full guide →" link keeps working; the texts are translation entries, easy to edit (spec §5) |
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Expected: every test passes, including `tests/guides.rs` and the CSP scan (now also
`/guide/logging`). **593** tests in the workspace.

- [ ] **Step 5: Commit**

```bash
git add crates/web/locales/en/main.ftl \
  crates/web/locales/it/main.ftl \
  crates/web/locales/tr/main.ftl \
  crates/web/src/lib.rs \
  crates/web/src/routes/guide.rs \
  crates/web/static/app.css \
  crates/web/templates/guide.html \
  crates/web/templates/guide_logging.html \
  crates/web/templates/settings.html \
  crates/web/tests/guides.rs \
  crates/web/tests/security.rs \
  docs/DECISIONS.md
git commit -F - <<'EOF'
feat(guides): How to log page under Settings → Guides

The spec §5 usage rules become a page: the what-happened / correct-action table, the five
rules of thumb (the auto-close rule names the configured hour) and the note on different
sensitivities, translated like the rating guide. Settings lists both guides and the two
pages link to each other.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 6: CI: CodeQL and coverage

Spec §13.1 (decisions #128, #129). **`codeql.yml`** analyses Rust, the workflow files
(`actions`) and the project's own JavaScript (`javascript-typescript`) on pushes to `main`, pull
requests and weekly, all with `build-mode: none`, ignoring the vendored `*.min.js`. CodeQL now
lists Rust (editions 2021 and 2024) as supported, not as a preview, which is the spec's
condition for keeping it. **Coverage** is a new job in `ci.yml`: `cargo llvm-cov --workspace
--locked --no-report` against SQLite and a Postgres service, then the summary into
`$GITHUB_STEP_SUMMARY` and `lcov.info` as a 14-day artifact. Both are free for a public
repository and use no secret. The third-party installer action is pinned to a commit like Trivy
is; GitHub's own actions use their major tag like the rest of the repository. Neither job is
added to the ruleset's required checks (`.github/rulesets/` is unchanged).

**Files:**
- Create: `.github/workflows/codeql.yml`
- Modify: `.github/workflows/ci.yml`, `docs/DECISIONS.md`

**Interfaces:**
- Consumes: the `ci.yml` Postgres service block and `TEST_POSTGRES_URL` env (milestone 2).
- Produces: workflow `CodeQL` with jobs `codeql (rust)`, `codeql (actions)`,
  `codeql (javascript-typescript)`; job `coverage` in `CI`; artifact `coverage-lcov`.

- [ ] **Step 1: Write the failing tests**

No Rust test: workflows are checked by `actionlint` (Step 2/4) and by their first run on the
pull request.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `actionlint .github/workflows/ci.yml .github/workflows/codeql.yml` (install with
`brew install actionlint` if missing)
Expected before Step 3: `codeql.yml` does not exist (`no such file`).

- [ ] **Step 3: Implement**

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index bfcc923..48b57e1 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -40,6 +40,49 @@ jobs:
       - name: audit hash ignores serde_json map ordering
         run: cargo test -p smoke-domain --lib --features serde_json/preserve_order audit
 
+  coverage:
+    name: coverage
+    runs-on: ubuntu-latest
+    services:
+      postgres:
+        image: postgres:17-alpine
+        env:
+          POSTGRES_HOST_AUTH_METHOD: trust
+        ports: ["5432:5432"]
+        options: >-
+          --health-cmd pg_isready
+          --health-interval 5s
+          --health-timeout 5s
+          --health-retries 10
+    steps:
+      - uses: actions/checkout@v7
+      - uses: dtolnay/rust-toolchain@stable
+        with:
+          components: llvm-tools-preview
+      - uses: Swatinem/rust-cache@v2
+      - name: install cargo-llvm-cov
+        # Pinned to a commit, not a tag: a third-party action's tags can be moved.
+        uses: taiki-e/install-action@1168f6810a3cb30a3514bf5756a3b7f0422c6df8 # v2.87.28
+        with:
+          tool: cargo-llvm-cov
+      - name: run the tests with coverage (SQLite and Postgres)
+        run: cargo llvm-cov --workspace --locked --no-report
+      - name: line coverage in the job summary
+        run: |
+          {
+            echo '### Line coverage'
+            echo '```'
+            cargo llvm-cov report --summary-only
+            echo '```'
+          } >> "$GITHUB_STEP_SUMMARY"
+      - name: lcov report
+        run: cargo llvm-cov report --lcov --output-path lcov.info
+      - uses: actions/upload-artifact@v7
+        with:
+          name: coverage-lcov
+          path: lcov.info
+          retention-days: 14
+
   sqlx:
     name: sqlx offline data
     runs-on: ubuntu-latest
diff --git a/.github/workflows/codeql.yml b/.github/workflows/codeql.yml
new file mode 100644
index 0000000..ca088fe
--- /dev/null
+++ b/.github/workflows/codeql.yml
@@ -0,0 +1,37 @@
+name: CodeQL
+
+on:
+  push:
+    branches: [main]
+  pull_request:
+  schedule:
+    - cron: "0 6 * * 3" # Wednesdays 06:00 UTC: new queries find old code too
+
+permissions:
+  contents: read
+
+jobs:
+  analyze:
+    name: codeql (${{ matrix.language }})
+    runs-on: ubuntu-latest
+    permissions:
+      contents: read
+      security-events: write
+    strategy:
+      fail-fast: false
+      matrix:
+        # rust: the app; actions: these workflows; javascript-typescript: static/*.js.
+        language: [rust, actions, javascript-typescript]
+    steps:
+      - uses: actions/checkout@v7
+      - uses: github/codeql-action/init@v4
+        with:
+          languages: ${{ matrix.language }}
+          build-mode: none
+          # Vendored third-party libraries (htmx, Chart.js) are not this project's code.
+          config: |
+            paths-ignore:
+              - crates/web/static/*.min.js
+      - uses: github/codeql-action/analyze@v4
+        with:
+          category: /language:${{ matrix.language }}
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index d1d72f3..2d2b62b 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -134,3 +134,5 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 125 | 2026-10-14 | UI | **Graphs** (`/graphs`, in the menu for everyone signed in) show **one month** (`?month=YYYY-MM`, the current one by default) **or one year** (`?year=YYYY`), with previous and next links and a link to the other kind of period; intensity has **one line per person, named by username**, and PM2.5 on a right-hand axis; **guests** see their own data and a line saying so; a period with nothing logged says so instead of drawing empty charts | Any date range; a rolling window | The same navigation as the Events list (#100); bounded queries; spec §4 gives guests graphs of their own data |
 | 126 | 2026-10-14 | Graphs | The **room × hour heatmap is an HTML table** drawn by the server: one row per room a report named, 24 local hours, each cell the rounded average level with a CSS class `heat-0`…`heat-10` (the accent colour at rising strength), empty where nobody checked; the **symptoms** are horizontal bars, most frequent first; **inside vs common** are paired bars on the 0–10 axis | The `chartjs-chart-matrix` plugin; a bubble chart; inline `style` colours | No second library; works without JavaScript and reads as a table to screen readers; classes keep the CSP free of inline styles; the header now wraps on narrow phones because the menu gained Graphs |
 | 127 | 2026-10-14 | UI | **Settings → Guides** lists *How to rate* (`/guide/rating`, unchanged since #97) and the new ***How to log*** (`/guide/logging`): the spec §5 table, the five rules of thumb (the auto-close one names the configured `AUTO_CLOSE_HOUR`) and the note on different sensitivities; the two guides link to each other, and every signed-in role can open them | Move the pages to `/settings/guides/…`; one combined guide page | Settings already links the guides (M5), so "under Settings" needs no new address and the popup's "Full guide →" link keeps working; the texts are translation entries, easy to edit (spec §5) |
+| 128 | 2026-10-14 | CI | **`codeql.yml`** runs CodeQL on push to `main`, pull requests and weekly for **Rust, the workflows (`actions`) and our own JavaScript**, all with `build-mode: none`; the vendored `*.min.js` files are ignored; results go to the repository's code-scanning alerts. It is **not a required check** yet | GitHub's "default setup" (no file in the repository); Rust only; Semgrep | Spec §13.1 keeps CodeQL "only if Rust analysis is stable": CodeQL lists Rust editions 2021 and 2024 as supported, no longer as a preview; free for public repositories; a file is versioned and reviewed like the other workflows. Default setup must stay off, or GitHub refuses the uploads |
+| 129 | 2026-10-14 | CI | **Coverage** is a separate `ci.yml` job: **`cargo llvm-cov`** runs the whole workspace on SQLite and a Postgres service, writes the line-coverage summary into the job summary and keeps `lcov.info` as a **14-day artifact**; no threshold, no badge, no upload to a coverage service | Codecov or Coveralls; `cargo-tarpaulin`; coverage inside the test job | Free and tokenless (no secret to protect); llvm-cov is exact on stable Rust; a separate job keeps the required `fmt · clippy · test` check unchanged, and a threshold would only invite tests written for the number |
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
Then `actionlint .github/workflows/ci.yml .github/workflows/codeql.yml` prints nothing.
**593** tests (unchanged). Put in the pull request's checklist: "the `coverage` job shows a
line-coverage table in its summary and an `lcov` artifact; the three CodeQL jobs finish; in
Settings → Code security, CodeQL *default setup* is **off** (otherwise GitHub rejects the
uploads of an advanced workflow)".

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml \
  .github/workflows/codeql.yml \
  docs/DECISIONS.md
git commit -F - <<'EOF'
ci: CodeQL for Rust, workflows and JavaScript; coverage with cargo-llvm-cov

codeql.yml analyses the Rust code, the workflow files and the project's own JavaScript on
pushes to main, pull requests and weekly (vendored libraries ignored). A new coverage job in
ci.yml runs the whole test suite under cargo-llvm-cov against SQLite and Postgres, puts the
line-coverage summary in the job summary and keeps lcov.info as an artifact. Both are free
for a public repository and need no token.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 7: README as the project's front page

Spec §4 ("This table is also published in the README") and §5 ("Shown in-app and in the
README"); handoff: a portfolio README (decision #130). The README keeps its *Development*,
*Running it* and *Hosting* sections word for word (plus one `fmt`/`clippy` line) and gains:
status and workflow badges (GitHub's own, free), *What it does*, *How to log* with the rules of
thumb, the *Intensity guide*, *Who can do what* (the §4 table), *Architecture* (a Mermaid crate
diagram, a crate table and the main design choices with a link to the decision log), *CI/CD and
security*, *Third-party code in the binary* (htmx 2.0.11 0BSD, Chart.js 4.5.1 MIT),
*Documentation* and *Licence*. It contains no email address and no secret. A unit test reads
the README (`include_str!`, test-only) and checks every intensity row, every *How to log* row
and the rating tip against the English translations. The roadmap's row 6 links this plan, as
milestone 5's last task did.

**Files:**
- Modify: `README.md`, `crates/web/src/routes/guide.rs`, `docs/DECISIONS.md`, `docs/plans/ROADMAP.md`

**Interfaces:**
- Consumes: `guide::{levels, situations}` (Tasks 5 and earlier), `Tr::t("rating-tip")`.
- Produces: `README.md`; unit test `the_readme_tables_match_the_english_guides`;
  `docs/plans/ROADMAP.md` row 6 Plan cell `06-graphs-guides-readme.md`.

- [ ] **Step 1: Write the failing tests**

The test is a unit test in `crates/web/src/routes/guide.rs`, part of the Step 3 diff; write it
first and run it against the old README.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p smoke-web --lib guide`
Expected (test added, README not yet rewritten): `the_readme_tables_match_the_english_guides`
fails with `README lacks | 0 | Clean | You checked: no smoke smell at all. |`.

- [ ] **Step 3: Implement**

````diff
diff --git a/README.md b/README.md
index 87fa16a..5aa48a4 100644
--- a/README.md
+++ b/README.md
@@ -1,12 +1,135 @@
 # Smoke Tracker
 
+[![CI](https://github.com/locadani/smoke-tracker/actions/workflows/ci.yml/badge.svg)](https://github.com/locadani/smoke-tracker/actions/workflows/ci.yml)
+[![Security](https://github.com/locadani/smoke-tracker/actions/workflows/security.yml/badge.svg)](https://github.com/locadani/smoke-tracker/actions/workflows/security.yml)
+[![CodeQL](https://github.com/locadani/smoke-tracker/actions/workflows/codeql.yml/badge.svg)](https://github.com/locadani/smoke-tracker/actions/workflows/codeql.yml)
+
 A small, installable web app (PWA) for logging cigarette smoke that drifts into an apartment
 from neighbours. It records how strong the smell is, which rooms it reached, any symptoms, and
-PM2.5 readings, and builds a detailed, tamper-evident record over time.
+PM2.5 readings, and builds a detailed, tamper-evident record over time that can support a
+formal complaint.
+
+Written in Rust (Axum, Askama, htmx), with SQLite locally and Postgres in production. It runs
+for free on Google Cloud Run and Neon.
+
+> **Status:** in use. Every feature of the [design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
+> is built; see the [roadmap](docs/plans/ROADMAP.md).
+
+## What it does
+
+- **Log in about five seconds** from the phone's home screen: tap the rooms, give each a level
+  from 0 to 10, tick any symptoms, add a PM2.5 reading or a note. **No smell** records a clean
+  check with one tap.
+- **Events** group the reports of one smoke incident from start to end. The app asks "Same
+  smell still going?" when one is open, closes forgotten events at 04:00, and allows logging
+  in the past.
+- **Tamper-evident history:** every change is stored with its before and after, in an audit log
+  where each entry hashes the previous one. `verify-audit` recomputes the chain and checks every
+  record against it.
+- **Graphs:** intensity over time with one line per person and PM2.5 as the objective
+  reference, events per day or week, a room × hour-of-day heatmap, symptom frequency, and
+  inside versus common areas.
+- **People:** residents invite guests (who expire) and viewers (read-only, e.g. a lawyer) with
+  a one-time QR code; there is no email and no self-registration.
+- English, Italian and Turkish; installable on Android and iPhone; an offline page.
+
+## How to log
+
+Also in the app, under Settings → Guides.
+
+| What happened | Correct action |
+|---|---|
+| I made a mistake (wrong number, forgot a room, wrong symptom) | Correct that report (edit; recorded in history) |
+| The situation changed (stronger/weaker, a room started or stopped, symptoms started) | Add a new report to the same event |
+| The smell went away completely and came back later | Start a new event |
+
+**Rules of thumb**
+
+1. **Never edit to show a change; add a report.** Editing changes the past; adding records the present.
+2. Add a report when a room's level changes by **2 or more**, a **room starts or clears** (log it
+   at 0), or a **new symptom** appears. Each report lists only the rooms you checked: a room you
+   leave out is "not checked", never "clean".
+3. Unsure whether it's the same event? If the smell was **completely gone** at some point, it's a new one.
+4. **Close the event when the smell is gone.** Forgetting is fine (04:00 auto-close), but closing gives accurate durations.
+5. **Nothing smells? Log *No smell*.** It keeps a record that you checked.
+
+Each person rates on their own 1–10 scale. Overlapping reports with different values are kept
+as-is. Graphs show one line per person; the PM2.5 reading is the objective reference.
+
+### Intensity guide
+
+Smell only; symptoms are recorded separately.
+
+| Level | Name | What it's like |
+|---|---|---|
+| 0 | Clean | You checked: no smoke smell at all. |
+| 1 | Barely there | Only if you focus. Unsure it's smoke. |
+| 2 | Faint | Noticeable entering the room, then fades. |
+| 3 | Light | Clearly smoke, but weak. Easy to ignore. |
+| 4 | Noticeable | Constantly present, mildly annoying. |
+| 5 | Moderate | Obvious all the time. You'd mention it. |
+| 6 | Strong | Hard to ignore; want to open a window or leave the room. |
+| 7 | Very strong | Can't stay comfortably; smell builds up. |
+| 8 | Heavy | Gets into clothes/bedding; lingers after you leave. |
+| 9 | Very heavy | Like standing next to someone smoking. |
+| 10 | Unbearable | Like a smoke-filled room. You must leave. |
+
+Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.
+
+## Who can do what
+
+| Action | Resident | Guest (active) | Guest (expired) | Viewer |
+|---|---|---|---|---|
+| Log smell / PM2.5 | ✅ | ✅ | ❌ | ❌ |
+| See data | all | own | own | all |
+| Correct / move entries | all | own | ❌ | ❌ |
+| Close / reopen events | ✅ | events they own | ❌ | ❌ |
+| Invite / expire / reactivate / reset password | ✅ | ❌ | ❌ | ❌ |
+| Edit room & symptom lists | ✅ | ❌ | ❌ | ❌ |
+| See graphs | all data | own data | own data | all data |
 
-Written in Rust (Axum, Askama, htmx), with SQLite locally and Postgres in production.
+- **Residents** never expire. **Guests** always have an expiry date, after which they can only
+  read their own data. **Viewers** may have one, after which they cannot sign in.
+- A resident can reactivate an expired account with a new expiry date; the old password still works.
+- Ending someone's access signs them out everywhere.
 
-> **Status:** early development. See the [roadmap](docs/plans/ROADMAP.md).
+## Architecture
+
+A Cargo workspace of layered crates. The domain is pure Rust with no I/O; the web layer never
+sees SQL, only the repository traits the domain defines; both storage backends pass one shared
+test suite.
+
+```mermaid
+flowchart LR
+    app[smoke-tracker<br/>binary and CLI] --> web[smoke-web<br/>routes, pages, assets]
+    app --> sqlite[smoke-storage-sqlite]
+    app --> postgres[smoke-storage-postgres]
+    web --> domain[smoke-domain<br/>rules, no I/O]
+    sqlite --> domain
+    postgres --> domain
+    contract[smoke-storage-contract<br/>shared test suite] -.tests.-> sqlite
+    contract -.tests.-> postgres
+```
+
+| Crate | What it holds |
+|---|---|
+| [`crates/domain`](crates/domain) | Types that make invalid states unrepresentable (`Level` 0–10, `Pm25`, ids), the permissions table as one function, the event rules, the logging and account use cases, the graphs' arithmetic, the hash-chained audit log, a `Clock` trait, and the repository traits |
+| [`crates/storage-sqlite`](crates/storage-sqlite), [`crates/storage-postgres`](crates/storage-postgres) | The two backends: migrations kept in step (a test compares the schemas) and queries checked at compile time by `sqlx`; every write transaction appends its audit entry |
+| [`crates/storage-contract`](crates/storage-contract) | One behaviour suite written against the traits and run on both backends |
+| [`crates/web`](crates/web) | Axum routes (one module per screen), Askama templates, Fluent translations, and the static files compiled into the binary |
+| [`crates/app`](crates/app) | The `smoke-tracker` binary: configuration from the environment, wiring, the 04:00 auto-close timer, and the CLI (`serve`, `migrate`, `create-resident`, `verify-audit`) |
+
+Some choices, each recorded with its alternatives in the [decision log](docs/DECISIONS.md):
+
+- **Server-rendered pages** with plain forms; the little JavaScript there is (a dialog, the
+  install prompt, the charts) is progressive enhancement.
+- A **strict Content-Security-Policy** (`'self'` only, no inline scripts or styles): the charts
+  read their data from `data-` attributes, the heatmap is a table coloured by CSS classes, and
+  QR codes are inline SVG. A test scans the app's pages for anything the policy would block.
+- **Server-side sessions** (revocable at once), Argon2id passwords, a sign-in rate limit, and an
+  Origin check on every form post.
+- All times are stored in UTC and shown in one configured time zone, so every phone shows the
+  same times; tests drive the 04:00 rule across daylight-saving changes with a fake clock.
 
 ## Development
 
@@ -23,6 +146,7 @@ export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
 
 cargo test --workspace         # Postgres tests are skipped when TEST_POSTGRES_URL is unset
 scripts/sqlx-prepare.sh        # after changing a query or a migration; commit the .sqlx/ changes
+cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
 ```
 
 Building needs no database: queries are checked against the committed `.sqlx/` metadata.
@@ -60,6 +184,22 @@ everyone out together. The front end appends the real client as the header's las
 which is the one the app trusts. With `RUST_LOG=info,smoke_web=debug` each sign-in attempt
 logs the address it resolved, to check this on the first deploy.
 
+## CI/CD and security
+
+Everything runs on GitHub Actions, on free tiers, with no token stored in the repository.
+Pull requests into `main` need the checks marked *required* to pass.
+
+| Workflow | When | What |
+|---|---|---|
+| [`ci.yml`](.github/workflows/ci.yml) | push to `main`, pull requests | `fmt`, `clippy -D warnings`, the whole test suite on SQLite and a Postgres service (required); the committed `sqlx` query data (required); line coverage with `cargo-llvm-cov`, shown in the job summary and kept as an `lcov` artifact |
+| [`security.yml`](.github/workflows/security.yml) | push, pull requests, weekly | `cargo-deny` (RustSec advisories, licences, sources), `gitleaks`, a Trivy scan of the container image (all required) |
+| [`codeql.yml`](.github/workflows/codeql.yml) | push, pull requests, weekly | CodeQL for the Rust code, the workflows and the project's own JavaScript |
+| [`deploy.yml`](.github/workflows/deploy.yml) | push to `main` | Builds, scans and deploys the image to Cloud Run |
+| [`infra.yml`](.github/workflows/infra.yml) | changes under `infra/` | Terraform format, validation, tests and plan |
+
+Dependabot keeps Cargo crates, actions, Docker images and Terraform providers up to date; a
+gitleaks pre-commit hook and GitHub push protection stop secrets before they leave a laptop.
+
 ## Hosting
 
 The app runs on Google Cloud Run, with its data in a Neon Postgres database. Both are on free
@@ -116,7 +256,24 @@ No secret is ever stored in this repository, in a Docker image, or in GitHub.
 The Google Cloud side is Terraform: see [`infra/`](infra/README.md) for what is automatic, what is
 manual and why, and how changes are applied.
 
+## Third-party code in the binary
+
+Two JavaScript libraries are vendored byte for byte under `crates/web/static/`; a test pins
+each file's SHA-256 to the published release.
+
+| Library | Version | Licence | Used for |
+|---|---|---|---|
+| [htmx](https://htmx.org) | 2.0.11 | 0BSD | Loaded on every page |
+| [Chart.js](https://www.chartjs.org) | 4.5.1 | MIT | The Graphs screen |
+
 ## Documentation
 
-- [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
-- [Decision log](docs/DECISIONS.md)
+- [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md) and
+  [hosting spec](docs/specs/2026-10-13-hosting-infra-design.md)
+- [Decision log](docs/DECISIONS.md): every product and technical decision, with the alternatives
+- [Roadmap](docs/plans/ROADMAP.md) and the plan of each milestone
+- [Infrastructure](infra/README.md)
+
+## Licence
+
+[MIT](LICENSE)
diff --git a/crates/web/src/routes/guide.rs b/crates/web/src/routes/guide.rs
index 4741171..f96ed30 100644
--- a/crates/web/src/routes/guide.rs
+++ b/crates/web/src/routes/guide.rs
@@ -128,6 +128,26 @@ mod tests {
 
     use super::*;
 
+    /// The README repeats the English guides (spec §5); this keeps the two from drifting apart.
+    #[test]
+    fn the_readme_tables_match_the_english_guides() {
+        let readme = include_str!("../../../../README.md");
+        let tr = Tr(Locale::En);
+        for level in levels(tr) {
+            let row = format!(
+                "| {} | {} | {} |",
+                level.value, level.name, level.description
+            );
+            assert!(readme.contains(&row), "README lacks {row}");
+        }
+        for situation in situations(tr) {
+            let row = format!("| {} | {} |", situation.happened, situation.action);
+            assert!(readme.contains(&row), "README lacks {row}");
+        }
+        let tip = tr.t("rating-tip");
+        assert!(readme.contains(&tip), "README lacks {tip}");
+    }
+
     #[test]
     fn the_auto_close_rule_names_the_configured_hour() {
         let rules = rules(Tr(Locale::En), 5);
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 2d2b62b..486e7fa 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -136,3 +136,4 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 127 | 2026-10-14 | UI | **Settings → Guides** lists *How to rate* (`/guide/rating`, unchanged since #97) and the new ***How to log*** (`/guide/logging`): the spec §5 table, the five rules of thumb (the auto-close one names the configured `AUTO_CLOSE_HOUR`) and the note on different sensitivities; the two guides link to each other, and every signed-in role can open them | Move the pages to `/settings/guides/…`; one combined guide page | Settings already links the guides (M5), so "under Settings" needs no new address and the popup's "Full guide →" link keeps working; the texts are translation entries, easy to edit (spec §5) |
 | 128 | 2026-10-14 | CI | **`codeql.yml`** runs CodeQL on push to `main`, pull requests and weekly for **Rust, the workflows (`actions`) and our own JavaScript**, all with `build-mode: none`; the vendored `*.min.js` files are ignored; results go to the repository's code-scanning alerts. It is **not a required check** yet | GitHub's "default setup" (no file in the repository); Rust only; Semgrep | Spec §13.1 keeps CodeQL "only if Rust analysis is stable": CodeQL lists Rust editions 2021 and 2024 as supported, no longer as a preview; free for public repositories; a file is versioned and reviewed like the other workflows. Default setup must stay off, or GitHub refuses the uploads |
 | 129 | 2026-10-14 | CI | **Coverage** is a separate `ci.yml` job: **`cargo llvm-cov`** runs the whole workspace on SQLite and a Postgres service, writes the line-coverage summary into the job summary and keeps `lcov.info` as a **14-day artifact**; no threshold, no badge, no upload to a coverage service | Codecov or Coveralls; `cargo-tarpaulin`; coverage inside the test job | Free and tokenless (no secret to protect); llvm-cov is exact on stable Rust; a separate job keeps the required `fmt · clippy · test` check unchanged, and a threshold would only invite tests written for the number |
+| 130 | 2026-10-14 | Docs | The **README** is the project's front page: what the app does, the spec §5 *How to log* rules and intensity guide, the §4 permissions table, the crate architecture, how to build, test and run it, the CI and security workflows, hosting, and the vendored libraries with their licences; a unit test checks that its intensity and *How to log* tables **match the English guide texts** | Link to the spec instead of copying the tables; generate the README from the translations | Spec §4 and §5 ask for the tables in the README; the test catches an edit to one side only, without a generator to maintain |
diff --git a/docs/plans/ROADMAP.md b/docs/plans/ROADMAP.md
index 642990d..06dd35b 100644
--- a/docs/plans/ROADMAP.md
+++ b/docs/plans/ROADMAP.md
@@ -12,7 +12,7 @@ guesses.
 | 3 | **Web foundation & auth** | Axum server, config, CLI (`serve`, `migrate`, `create-resident`, `verify-audit`), sessions, login + rate limit, security headers, Origin check, i18n (en/it/tr), base layout, embedded assets, Dockerfile, release workflow, Trivy | §8.1, §8.2, §8.4, §9.2, §10.3–10.5, §12, §13.1 | `03-web-and-auth.md` ✅ |
 | 4 | **Logging flow** | Log screen, every prompt (same smell / which event / auto-close notice), close/reopen, PM2.5, events list and detail, report detail (correct, move, history), large-gap warning, auto-close sweep, summary card, "How to rate" popup | §5, §7, §9 (1–4), §11 | `04-logging-flow.md`, `04b-room-levels.md` |
 | 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | `05-people-and-pwa.md` |
-| 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | — |
+| 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | `06-graphs-guides-readme.md` |
 | 7 | **Neon readiness** | ~~`migrate-to-postgres` command and its CI test~~ Not needed: production starts on Postgres (decision #77) | §10.2 | — |
 | 8 | **Learning pack** | `docs/learning/` notes per milestone, optional exercises with tests, walkthroughs | Decision #43 | — |
 
````

- [ ] **Step 4: Run the checks to verify they pass**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace
```
Expected: every test passes. With `TEST_POSTGRES_URL` set: **594** tests. Read the rendered
README once on the branch's GitHub page (Mermaid diagrams and tables) when the pull request is
open; check that it shows no email address.

- [ ] **Step 5: Commit**

```bash
git add README.md \
  crates/web/src/routes/guide.rs \
  docs/DECISIONS.md \
  docs/plans/ROADMAP.md
git commit -F - <<'EOF'
docs(readme): portfolio README with the guides, permissions and architecture

The README now explains what the app does, repeats the How to log rules, the intensity guide
and the permissions table of the spec, shows the crate architecture, lists the CI and
security workflows and the vendored libraries with their licences, and keeps the hosting
section. A unit test keeps its guide tables equal to the English translations. The roadmap
links this milestone's plan.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

## Development

Local loop (one shared build folder for every worktree):

```sh
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
export TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
actionlint .github/workflows/*.yml
```

Trying the Graphs screen by hand: serve with `APP_BASE_URL=http://localhost:8080`, log a few
reports (some backdated, with symptoms and a stairwell level), then open Graphs, switch to the
year and back, and sign in as a guest in a private window to see only the guest's data. The
browser console must show no Content-Security-Policy error.

After the merge: nothing to migrate. Optional, in GitHub: make sure CodeQL *default setup* is
off (Settings → Code security), and add `coverage` or the CodeQL checks to the `main` ruleset
if they should block merges.

---

## Spec coverage

| Spec | Requirement | Task |
|---|---|---|
| §4 | See graphs: residents and viewers all data, guests (active or expired) own data | 3, 4 |
| §4 | The permissions table is published in the README | 7 |
| §5 | How to log table and rules of thumb, shown in-app (Settings → Guides) and in the README | 5, 7 |
| §5 | Intensity guide in the README; guide texts in translation entries | 7 (in-app since M4) |
| §5 | Different sensitivities: one line per person, PM2.5 the objective reference | 3, 5 |
| §9 screen 5 | Graphs with Chart.js, self-hosted | 3 |
| §9 screen 5 | Intensity over time (a report's highest room level), one line per person, PM2.5 on a secondary axis | 2, 3 |
| §9 screen 5 | Events per day/week | 2, 3 |
| §9 screen 5 | Room × hour-of-day heatmap of room levels, 0 counting as checked and clean | 2, 4 |
| §9 screen 5 | Symptom frequency | 2, 4 |
| §9 screen 5 | Inside vs common-area comparison | 2, 4 |
| §9 screen 7 | Settings → Guides: How to rate, How to log | 5 |
| §9.2 | Every new text in en/it/tr; it/tr drafts for native review | 3, 4, 5 |
| §12 | CSP `'self'` only, no CDN | 3, 4, 5 |
| §13.1 | `ci.yml`: coverage (`cargo-llvm-cov`) | 6 |
| §13.1 | `codeql.yml`: push, PR, weekly; kept only if Rust analysis is stable | 6 |
| §13.3 | Storage tests vs Postgres (test databases cleaned up) | 1 |
