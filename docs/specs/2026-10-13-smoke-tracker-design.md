# Smoke Tracker — Design Spec

- **Date:** 2026-10-13
- **Status:** Approved (2026-10-13)
- **Source requirements:** `description.md`, `claude.md` (not modified)

---

## 1. Purpose

Neighbours' cigarette smoke regularly enters the apartment. The residents want a **detailed,
credible log** of these incursions that could support a formal complaint (building
administrator, municipality, ASL/ARPA, lawyer). The hard part is staying motivated to log, so
the app must make logging take **about 5 seconds on a phone**.

**Success criteria**

1. A smell report can be logged in ~5 seconds from the phone home screen.
2. Months of data produce a report that is detailed (rooms, intensity, symptoms, PM2.5,
   durations) and **tamper-evident** (every change recorded, history verifiable).
3. Zero hosting cost; small footprint.
4. Portfolio-quality Rust codebase that also serves as a Rust learning project.

**Non-goals for v1** (see §14): data export, push reminders, automatic sensor ingestion,
offline logging, browser end-to-end tests, choosing the hosting provider.

> *Superseded: hosting is chosen, see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*

---

## 2. Constraints

| Constraint | Consequence |
|---|---|
| Written in Rust | Axum + Askama + htmx; SQLite via sqlx |
| No money | Free tiers only; single small binary/container; SQLite (no DB server) |
| Tokens never pushed | gitleaks pre-commit hook, GitHub push protection, gitleaks in CI, `.env` git-ignored |
| CI/CD on GitHub | GitHub Actions; public repo |
| Free-tier security plugins | cargo-deny, gitleaks, Trivy, CodeQL, Dependabot (Snyk optional) |
| Portfolio + learning | Cargo workspace with layered crates; idiomatic, well-tested code; ADRs |
| Must be able to move to Neon later | Repository traits, SQLite + Postgres backends, contract tests, migration command |

---

## 3. Glossary

| Term | Meaning |
|---|---|
| **Resident** | The two people living in the apartment. Full access, manage other users. |
| **Guest** | Invited person who stays temporarily. Logs data; sees only own data; expires. |
| **Viewer** | Read-only person (e.g. lawyer, administrator). Sees all data; cannot write. |
| **Smell report** | One person's observation at one moment: the rooms they checked, each with a level 0–10 (0 = checked, no smell), plus symptoms and a note. |
| **PM2.5 reading** | A sensor value in µg/m³, entered by hand. Independent record type. |
| **Event** | One smoke incident, grouping smell reports (and PM2.5 readings) from start to end. |
| **Event owner** | The user who logged the event's first smell report. |
| **Observed time** | When the user says the thing happened (user-chosen, backdating allowed). |
| **Recorded time** | When the server stored it (server-set, immutable). |

---

## 4. Roles and permissions

| Action | Resident | Guest (active) | Guest (expired) | Viewer |
|---|---|---|---|---|
| Log smell / PM2.5 | ✅ | ✅ | ❌ | ❌ |
| See data | all | own | own | all |
| Correct / move entries | all | own | ❌ | ❌ |
| Close / reopen events | ✅ | events they own | ❌ | ❌ |
| Invite / expire / reactivate / reset password | ✅ | ❌ | ❌ | ❌ |
| Edit room & symptom lists | ✅ | ❌ | ❌ | ❌ |
| See graphs | all data | own data | own data | all data |

- **Residents** never expire.
- **Guests** always have an expiry date. After it, they become read-only on their own data.
- **Viewers** have an optional expiry. After it, they cannot log in at all.
- Expired accounts can be **reactivated** by a resident with a new expiry date. The old password
  still works.
- On expiry/deactivation, all of the user's sessions are deleted.

This table is also published in the README.

---

## 5. Usage rules ("How to log")

Shown in-app (Settings → Guides) and in the README.

| What happened | Correct action |
|---|---|
| I made a mistake (wrong number, forgot a room, wrong symptom) | **Correct** that report (edit; recorded in history) |
| The situation changed (stronger/weaker, a room started or stopped, symptoms started) | **Add a new report** to the **same event** |
| The smell went away completely and came back later | **Start a new event** |

**Rules of thumb**

1. **Never edit to show a change; add a report.** Editing changes the past; adding records the present.
2. Add a report when a room's level changes by **2 or more**, a **room starts or clears** (log it
   at 0), or a **new symptom** appears. Each report lists only the rooms you checked: a room you
   leave out is "not checked", never "clean".
3. Unsure whether it's the same event? If the smell was **completely gone** at some point, it's a new one.
4. **Close the event when the smell is gone.** Forgetting is fine (04:00 auto-close), but closing gives accurate durations.
5. **Nothing smells? Log *No smell*.** It keeps a record that you checked.

**UI support:** with an event open, the main button reads **"Update — situation changed"**. The
edit action lives only on the report detail page, labelled **"Correct a mistake"**.

### Intensity guide (smell only; symptoms are recorded separately)

| | Level | What it's like |
|---|---|---|
| 0 | Clean | You checked: no smoke smell at all. |
| 1 | Barely there | Only if you focus. Unsure it's smoke. |
| 2 | Faint | Noticeable entering the room, then fades. |
| 3 | Light | Clearly smoke, but weak. Easy to ignore. |
| 4 | Noticeable | Constantly present, mildly annoying. |
| 5 | Moderate | Obvious all the time. You'd mention it. |
| 6 | Strong | Hard to ignore; want to open a window or leave the room. |
| 7 | Very strong | Can't stay comfortably; smell builds up. |
| 8 | Heavy | Gets into clothes/bedding; lingers after you leave. |
| 9 | Very heavy | Like standing next to someone smoking. |
| 10 | Unbearable | Like a smoke-filled room. You must leave. |

Tip: *Rate what you smell right now, not how annoyed you are. Between two numbers, pick the lower.*

The guide text lives in its own template/translation entries so it can be edited easily.

**How the guide opens:**
1. Tapping ⓘ *How to rate* on the Log screen opens a **popup** over the form, so you don't lose
   what you've entered. It contains the compact 1–10 table and the tip.
2. A **"Full guide →"** link in the popup opens the **full guide page** (Settings → Guides).

The popup is a native `<dialog>` element with no extra JavaScript library. It closes with ✕,
by tapping outside it, or with the back gesture.

**Different sensitivities:** each person rates on their own 1–10 scale. Overlapping reports with
different values are kept as-is. Graphs show one line per person; the PM2.5 reading is the
objective reference.

---

## 6. Data model

All timestamps are stored in **UTC** and displayed in the configured time zone
(default `Europe/Rome`). Nothing is hard-deleted: removable records have `deleted_at` and every
removal is audited.

```
users
  id, username (unique), password_hash (Argon2id; NULL until invite accepted)
  role: resident | guest | viewer
  locale: en | it | tr
  expires_at?              -- guests: required; viewers: optional; residents: NULL
  created_at

invites                    -- one-time links: account setup and password reset
  id, user_id, purpose: setup | password_reset
  token_hash               -- SHA-256 of a 32-byte random token; plaintext never stored
  created_by, created_at, expires_at (= created_at + 5 min), used_at?

sessions                   -- managed by tower-sessions (SQLite / Postgres store)

rooms
  id, name, i18n_key?      -- starter items translated via key; custom items shown as typed
  area: inside | common
  hidden_at?               -- hidden instead of deleted

symptoms
  id, name, i18n_key?, hidden_at?

events
  id, owner_id
  opened_at                -- = first smell report's observed_at
  closed_at?
  close_reason?: manual | new_event | auto_0400 | backdated
  auto_close_notice_seen_at?
  recorded_at

log_entries                -- one per press of Log (decision #147)
  id, user_id
  event_id?                -- no event = standalone (a no-smell check, or a reading outside events)
  observed_at, recorded_at
  note?
  deleted_at?
smell_reports
  id, log_entry_id (unique)
smell_report_rooms         (smell_report_id, room_id, level 0–10)   -- at least one room per report,
                                                                    -- except a no-smell report (no rooms = whole home)
smell_report_symptoms      (smell_report_id, symptom_id)

pm25_readings
  id, log_entry_id (unique)
  value_ugm3

audit_log                  -- append-only
  id, at, actor_id
  entity, entity_id
  action: create | update | delete | close | reopen | move | invite | expire | reactivate | password_change
  before_json?, after_json?
  prev_hash, hash          -- SHA-256(prev_hash || "\n" || canonical(entry))

jobs
  name, last_run_at        -- e.g. auto_close_sweep
```

A log entry owns a smell report, a PM2.5 reading, or both, and never neither. Who, when,
which event and the note live on the entry, so a report and the reading typed with it move
and are corrected together.

**Starter rooms:** living room, kitchen, bedroom, bathroom, entrance hall (*inside*);
stairwell (shared) (*common*).

**Starter symptoms:** cough, headache, sore throat, eye irritation, nausea, difficulty sleeping.

A report's **intensity** is its highest room level. An event's intensity is the highest intensity
among its reports.

### 6.1 Tamper-evident audit log (hash chain)

- Every write goes through **one transaction** that changes the data **and** appends an audit
  entry. Changing data without recording it is impossible through the app. Housekeeping deletions
  of expired sessions and expired, unused invites are not audited (decision #112).
- Each entry's `hash` = SHA-256 of the previous entry's hash, a newline, and a **canonical
  serialisation** of the entry (independent of the database backend).
- **`verify-audit`** command:
  1. Recomputes the chain and reports the first broken entry.
  2. Checks that each current log entry, with its report and reading, equals the `after_json`
     of its latest `log_entry` audit entry. This catches direct edits that bypass the audit log.
     Older `smell_report` and `pm25_reading` entries (before migration 0005) stay in the chain.
- **Pinning:** backups (and future exports) include the latest hash, so recomputing the whole
  chain after tampering is also detectable.
- Corrections made in the app are normal and visible. A log with a report shows an "edited"
  badge and a History view (e.g. *"09:15 next day, edited by Daniele: living room 6 → 7, added bedroom 3"*).

---

## 7. Event lifecycle rules

**Invariant:** at most **one open event** at a time (one apartment, one smoke source).

### 7.1 Logging "now" with no open event
A new event is created, owned by the logger.

A **no-smell report** (rooms all at 0, or *No smell* with no rooms, meaning the whole home) never
opens an event. It is saved as a **check** outside any event, a record that someone looked and
found nothing. The same holds for a backdated one that would otherwise need a new event.

A check can be corrected (its rooms, symptoms, note, time), but never to a level above 0: the form
says to log a smell instead. A check can be moved into an event.

### 7.2 Logging "now" with an open event
Prompt: **"Same smell still going?"**
- **[Yes, update]:** the report joins the open event.
- **[No, it had stopped: new event]:** the open event is closed with `close_reason = new_event`
  and `closed_at` = its last smell report's `observed_at`. A new event is created.

A no-smell report joins the open event without the prompt (it records that the smell has gone,
in the rooms it lists or, with no rooms, everywhere); it does not close the event.

**Guests** see the same prompt, but only the event's **start time**: *"A smell event is ongoing
(started 19:05). Is this the same smell?"* They never see who logged it, intensities, rooms, or
symptoms.

### 7.3 Closing
- **Manual close:** the user picks the end time (default now, ≥ last report's observed time).
  Allowed for residents, and for guests on events they own.
- **New-event close:** see 7.2.
- **Auto-close:** every day at **04:00** (local), all open events are closed with
  `close_reason = auto_0400` and `closed_at` = last smell report's `observed_at`.
- An event's end is never later than its last report unless a user closed it manually.

### 7.4 Auto-close notice
When the next user logs and an event was auto-closed whose notice hasn't been seen, the app shows
*"The event from [date time] was auto-closed."*
- **[It was still going: continuation]:** reopens the event (audited as `reopen`) and attaches the new report.
- **[New event]:** leaves it closed and starts a new one.

The notice is shown to the **event owner**, and also to **residents** when the owner is a guest.
If several notices are pending, only the most recent one is offered; the rest are marked seen.
If an event is currently open, the "same smell?" prompt takes priority and the notice is shown
only as a dismissable banner.

### 7.5 Backdated logging
- Observed time can be set to any past time (no limit). Recorded time is always the server time,
  and both are displayed.
- With a past time, the prompt becomes **"Which event was this?"**. It lists the events on that day
  (guests see start times only) plus **New event**.
- A backdated **new event** becomes the open event only if no event is open and no 04:00 has
  passed since its observed time. Otherwise it is created closed (`close_reason = backdated`,
  `closed_at` = report time).
- If a backdated report falls after its event's end, the end moves to the report time (audited).

### 7.6 PM2.5 readings
- They can be logged together with a smell report (one form, one transaction, **one log
  entry**) or alone (**Log** with only PM2.5 filled in). A reading logged alone can carry a note.
- A reading is linked automatically to the event whose span contains its observed time, or to the
  open event. Otherwise it is standalone.
- PM2.5 readings never open, extend, or close events.

### 7.7 Corrections and moves
- **Correct a mistake:** edits a report's values and its log's time and note. Audited
  (before/after). Re-timing a report re-times the reading logged with it. A log with only a
  PM2.5 reading cannot be corrected yet.
- **Move to another event** (residents; guests for their own reports): moves a report, and the
  reading logged with it, to an existing event or a new one. Audited.
- The event page shows a **large-gap warning** when consecutive reports are more than 3 hours apart.
  This makes it easy to spot a guest report that joined a forgotten event. The warning offers
  **Move to a new event** for the report after the gap (same rules as Move).
- Every log has its own page at `/entries/{id}` (time, author, PM2.5, rooms, symptoms, note);
  `/reports/{id}` redirects there.

---

## 8. Authentication, invites, sessions

### 8.1 Login
- Username + password. Passwords are hashed with **Argon2id**. Length 10–128 characters, no
  composition rules.
- Generic error message ("Invalid username or password").
- Rate limiting: 5 failed attempts → that username is locked for 15 minutes, plus a coarse per-IP
  limit. The limiter is in memory (single instance).
- No self-registration.

### 8.2 Bootstrap
The first resident is created on the server with `smoke-tracker create-resident <username>`, which
prompts for the password. There are no default credentials anywhere.

### 8.3 Invites and password reset
1. A resident chooses the username, role, and expiry:
   - guest: quick picks 1 day / weekend / 1 week (default) / 2 weeks / custom
   - viewer: optional
   - resident: none
2. The app shows a QR code for `{APP_BASE_URL}/invite/{token}`. The link is valid for
   **5 minutes** and works **once**.
3. The invitee sets a password and language, and is logged in.
4. After first login: **"Add Smoke Tracker to your home screen"**:
   - Chromium/Android: native install button (`beforeinstallprompt`)
   - iOS Safari: illustrated steps (Share → Add to Home Screen)

**Password reset** uses the same mechanism (`purpose = password_reset`). A resident generates the
QR code. There is no email.

### 8.4 Sessions
- Server-side sessions via **tower-sessions**, stored by our own session store in the database
  matching `DATABASE_URL` (only a hash of the session id is stored).
- Cookie: `__Host-` prefix, `HttpOnly`, `Secure`, `SameSite=Lax`. On `http://localhost`
  (development only) the cookie is named `session` and is not `Secure`, because browsers refuse
  `Secure` cookies there.
- 30-day lifetime, renewed on activity. Deleted on logout, expiry, deactivation, or password change.
  On password change every session is deleted; the device that made the change is given a new
  session so it stays signed in (decision #116).
- JWTs were rejected because they cannot be revoked immediately (see ADR).

---

## 9. Screens (mobile-first PWA)

1. **Log (home screen; the icon opens here)**
   - Event status bar: since when, the number of reports, and the highest level reported in the
     event with its room
   - **Rooms as toggle rows**, grouped *Inside / Common*, pre-selected from your last report in
     the open event. Each selected room shows its own level row: **0** (clean) and **1–10**, with
     ⓘ *How to rate* (popup, then a link to the full page; see §5). Every selected room needs a
     level; at least one room is required, except for *No smell*.
   - **Symptoms as checkboxes**
   - Optional PM2.5 field
   - Time ("now", with a *change* link)
   - Optional note
   - Main button: **Log** / **Update: situation changed**. It logs a smell report when rooms are
     selected (a clean check when all are 0) and a PM2.5 reading alone when only PM2.5 is filled
     in; symptoms or a note without a room is an error. **Close event** is in the event status bar.
   - **No smell**: one tap records the whole home as clean; with rooms selected (all at 0) it
     records those rooms. Guests can use it too.
2. **Summary card** on the home screen, e.g. *"This month: 14 events · 41 reports · 9 days with
   bedroom smoke · 6 clean checks."*
3. **Events:** list grouped by day, with that day's no-smell checks. The event detail shows a timeline of reports and PM2.5
   readings, the large-gap warning, and Close / Reopen.
4. **Report detail:** values, observed and recorded times, *Correct a mistake*, *Move to another
   event*, *History*.
5. **Graphs** (Chart.js, self-hosted; the room × hour-of-day heatmap is an HTML table coloured by
   CSS classes, decision #126):
   - intensity over time (a report's highest room level), one line per person, with PM2.5 on a
     secondary axis
   - events per day/week
   - room × hour-of-day heatmap of room levels (0 counts as checked and clean)
   - symptom frequency
   - inside vs common-area comparison

   Filters for all graphs, kept in the URL: dates (quick picks or from–to), area or rooms, person
   who reported (with search; not shown to guests), time of day, minimum level, hide clean checks.
   The intensity graph can show both series, intensity only or PM2.5 only, and, when no person is
   filtered, a **By person | Everyone** switch: one line per person, or one line with every
   report.

   Graphs cover any period of local days up to two years: per day up to 62 days, longer periods
   per Monday-to-Sunday week (decision #137). The heatmap shows each room's average level per
   local hour; inside vs common shows each area's highest level; an event counts once, on the day
   of its earliest report the viewer may see (decision #123).

   Guests see their own data only.
6. **People** (residents): users with role and expiry; Invite, Expire, Reactivate, Reset password
   (QR code).
7. **Settings:**
   - language
   - password change
   - room and symptom lists (residents)
   - Guides: *How to rate*, *How to log*
   - **Install app** (hidden in standalone mode)
8. **Header:** app name and an avatar menu (signed-in name, Log out).

### 9.1 PWA
- Web app manifest:
  - name, icons, `display: standalone`, `start_url` = Log screen
  - `shortcuts` entry "New smell report" (Android long-press)
- A minimal service worker caches static assets and an offline page. There is no offline logging
  in v1.

### 9.2 Internationalisation
- Languages: English (default), Italian, Turkish. Uses **Fluent** (`fluent-templates`), one
  `.ftl` file per language.
- Locale storage:
  - logged in: `users.locale`
  - before login: a `lang` cookie
- localStorage is not used because pages are server-rendered.
- Starter rooms and symptoms are translated via `i18n_key`. User-added ones are shown as typed.
- The Turkish (and Italian) strings are drafted by Claude and need review by a native speaker.

---

## 10. Architecture

> *Superseded: production runs on Postgres from the start (decision #77); SQLite stays for local use and tests. See docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*

Cargo workspace:

```
smoke-tracker/
├─ crates/
│  ├─ domain/     pure logic, no I/O
│  │   types.rs        Level(0..=10), Pm25, UserId, Role, … (invalid states unrepresentable)
│  │   permissions.rs  can(user, action) -> bool  (the §4 table as code)
│  │   events.rs       lifecycle rules (§7)
│  │   audit.rs        canonical serialisation, hash chain, verification
│  │   clock.rs        Clock trait (real + fake for tests)
│  │   repos.rs        repository traits (the storage boundary)
│  ├─ storage-sqlite/    SqliteStore, migrations/, .sqlx/ (decision #45)
│  ├─ storage-postgres/  PostgresStore, migrations/, .sqlx/, schema-parity test
│  │   migrate.rs        SQLite → Postgres data migration (milestone 7)
│  ├─ storage-contract/  one test suite run against both backends
│  ├─ web/        Axum routes (one module per feature), Askama templates, htmx partials,
│  │              i18n (.ftl), static assets (htmx, Chart.js, manifest, SW, icons)
│  └─ app/        binary: config, wiring, scheduler, CLI (clap)
├─ docs/          specs/, adr/
├─ .github/       workflows/, dependabot.yml, ISSUE_TEMPLATE/
├─ Dockerfile, .env.example, deny.toml, rust-toolchain.toml, .gitleaks.toml
└─ README.md
```

**Dependency direction:** `app → web → domain ← storage-*` (`app` also wires a storage crate). `web`
never sees SQL; it uses the repository traits only.

### 10.1 Storage abstraction (Neon-ready)
- Repository traits (`EventRepo`, `ReportRepo`, `UserRepo`, `AuditRepo`, …) live in `domain`.
- `SqliteStore` and `PostgresStore` both implement them, with queries checked at compile time by
  `sqlx::query!` (offline metadata committed for CI).
- The backend is chosen at runtime from `DATABASE_URL` (`sqlite://…` or `postgres://…`).
- **Contract tests:** the storage test suite is written once against the traits and runs against
  SQLite and a Postgres service container in CI.
- **Schema parity:** `crates/storage-sqlite/migrations` and `crates/storage-postgres/migrations` are kept in step. A CI check
  compares the resulting tables and columns.
- Instants are stored as integer microseconds since the Unix epoch in both backends (decision #46).
- Portable conventions: timestamps stored identically in both backends; no backend-specific
  features in domain-visible behaviour.

### 10.2 `migrate-to-postgres` command

> *Superseded: not needed, production starts on Postgres (decision #77). See docs/specs/2026-10-13-hosting-infra-design.md.*

```
smoke-tracker migrate-to-postgres --from sqlite://data.db --to postgres://…
```
1. Applies the Postgres migrations to the target (which must be empty).
2. Copies all tables in dependency order inside one transaction, preserving IDs, then resets
   sequences.
3. Verifies:
   - row counts per table
   - the audit hash chain recomputed on Postgres
   - the final hash printed for comparison with the last pinned hash
4. Never modifies the SQLite source. Rollback = switch `DATABASE_URL` back.

The command has its own CI test: seed SQLite, migrate, verify.

### 10.3 Single binary
- Templates, translations, and static assets are embedded in the executable.
- Static musl build on a distroless base image, roughly 15–20 MB.

### 10.4 CLI
`serve`, `migrate`, `create-resident`, `verify-audit`, `migrate-to-postgres`.

### 10.5 Configuration (environment only; `.env.example` committed, `.env` ignored)
`DATABASE_URL`, `APP_BASE_URL`, `APP_TIMEZONE` (default `Europe/Rome`), `AUTO_CLOSE_HOUR`
(default `4`), `RUST_LOG`, `BIND_ADDR`, and optionally `CLIENT_IP_HEADER` (the header a trusted
reverse proxy writes the client address into).

---

## 11. Background jobs and time

- **Auto-close sweep:**
  - `jobs.last_run_at` records the last sweep.
  - If a 04:00 (local) boundary has passed since then, every open event is closed (§7.3).
  - It runs at startup, before every logging request, and from an in-process timer.
  - Free hosts may sleep the app, so the result is identical whether the sweep runs at 04:00 or
    catches up later.
- **Expiry** is checked on every request (`expires_at <= now`: an account is expired from that exact instant); no job is needed.
- **Cleanup:** expired sessions are removed hourly through tower-sessions' `ExpiredDeletion`; expired invites are removed daily.
- **Time:** stored in UTC, with `chrono-tz` for local conversion. 04:00 exists exactly once a day
  in Europe/Rome (DST shifts happen at 02:00/03:00).
- **Clock trait:** all "now" calls go through `Clock`, so tests use a fake clock.

---

## 12. Error handling and security

**Errors**
- Library crates use `thiserror` enums; the binary uses `anyhow`.
- Validation errors re-render the form with a translated message (htmx partial).
- Unexpected errors are logged with `tracing`, including a request ID. The user sees *"Something
  went wrong (ref: …)"*.

**Security**
- Askama auto-escaping (XSS).
- CSRF: `SameSite=Lax` plus an Origin / `Sec-Fetch-Site` check on every state-changing request.
- Strict Content-Security-Policy (`'self'` only; no CDNs), HSTS, `X-Content-Type-Options`,
  `Referrer-Policy`, `frame-ancestors 'none'`.
- Argon2id passwords, hashed invite tokens, rate-limited login, generic auth errors.

**Secrets**
- gitleaks pre-commit hook (local), GitHub push protection, gitleaks in CI.
- Deploy credentials only in GitHub Actions Secrets.
- Personal data (e.g. commit email) never in tracked files.

> *Superseded: deploys log in with Workload Identity Federation, no stored key; see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*

---

## 13. CI/CD, tooling, deployment, testing

### 13.1 GitHub Actions (public repo)
| Workflow | Trigger | Jobs |
|---|---|---|
| `ci.yml` | push, PR | fmt check, clippy `-D warnings`, tests (SQLite + Postgres service), sqlx offline check, schema-parity check, build, coverage (`cargo-llvm-cov`) |
| `security.yml` | push, PR, weekly | cargo-deny (RustSec advisories, licences, bans, sources), gitleaks, Trivy image scan |
| `codeql.yml` | push, PR, weekly | CodeQL (kept only if Rust analysis is stable) |
| `release.yml` | push to `main`, tags | build and push the Docker image to GHCR; deploy job added once hosting is chosen |

> *Superseded: `release.yml` and GHCR are replaced by `deploy.yml` and Artifact Registry; see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*

- **Dependabot:** Cargo, GitHub Actions, Docker (weekly).
- **Branch protection on `main`:** PRs required, CI must pass.
- **Caching:** `Swatinem/rust-cache`.
- **Snyk:** optional (free tier; mainly for container scanning, since its Cargo support is
  limited). Needs a `SNYK_TOKEN` secret.

### 13.2 Hosting and backups

> *Superseded: hosting is Cloud Run + Neon, with a nightly `pg_dump` instead of Litestream → R2; see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*
- **Hosting:** deferred decision. Requirement: free and a persistent disk. Candidates are Oracle
  Cloud Always Free and Google Cloud e2-micro. Free-tier terms are re-checked at decision time.
  Moving to Neon (§10.2) widens the options to stateless hosts.
- **Backups:** Litestream streams the SQLite database to Cloudflare R2 (free tier). The latest
  audit hash is recorded with backups.

### 13.3 Testing
| Layer | What | How |
|---|---|---|
| domain | permissions table (every cell), lifecycle rules, `Level` bounds, hash chain tamper detection | unit tests + proptest |
| storage | migrations, queries, audit entry per write, soft deletes | contract tests vs SQLite (temporary file) and Postgres |
| migration | SQLite → Postgres copy + verification | seeded end-to-end test in CI |
| web | login and rate limit, guest isolation, expired guest can't write, viewer can't write, 5-minute invite expiry, Origin check | in-memory requests to the Axum router (`tower::ServiceExt::oneshot`) |
| i18n | every key present in en/it/tr | single test |
| jobs | sweep at 04:00 vs catch-up at 08:00; DST days | fake clock |

The workflow is test-first (TDD). Browser E2E tests are out of scope for v1.

---

## 14. Out of scope for v1 / future work

- **Export** (CSV first, then PDF), including the pinned audit hash. The data model already supports it.
- **Push reminders.** Tracked as GitHub issue #3 (text below).
- Automatic PM2.5 sensor ingestion. PM2.5 is already its own record type.
- Offline logging (service worker queue).
- Browser E2E tests (Playwright).
- Hosting choice and deploy job.

> *Superseded: done, see docs/specs/2026-10-13-hosting-infra-design.md and decisions #76–#85.*

### GitHub issue to create: push reminders

> **Title:** Push reminders for open events
>
> **Description:** Send a Web Push notification when an event is open and the user hasn't added a
> report for a while (e.g. 1 hour): *"Smell still there? Update or close the event."*
>
> **Notes**
> - Requires a service worker push handler, VAPID keys (stored as secrets, never committed), a
>   `push_subscriptions` table, and a scheduler.
> - iOS: only works when the PWA is installed to the home screen (iOS 16.4+).
> - Per-user opt-in and quiet hours.
> - Must work when the free host sleeps (consider an external free cron trigger).
>
> **Labels:** enhancement

---

## 15. Planned ADRs (`docs/adr/`)

1. Axum + Askama + htmx PWA instead of a native app
2. Cargo workspace with layered crates
3. SQLite as primary DB, with a Postgres-ready storage boundary
4. Server-side sessions instead of JWT
5. Hash-chained audit log for tamper evidence
6. Fluent for i18n; locale in DB + cookie
7. cargo-deny + Trivy instead of relying on Snyk for Rust
