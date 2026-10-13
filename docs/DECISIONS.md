# Decision Log

Every product and technical decision, newest at the bottom. Each one records what else was
considered and why this option won. Changing a decision means adding a new row that supersedes
the old one; rows are never rewritten. Larger technical decisions also get an ADR in
`docs/adr/`. The full design is in `docs/specs/2026-10-13-smoke-tracker-design.md`.

| # | Date | Area | Decision | Alternatives considered | Why |
|---|---|---|---|---|---|
| 1 | 2026-10-13 | Platform | Mobile-first **PWA** (installable web app) | Native iOS/Android app | Free (no $99/yr Apple fee), all-Rust, one codebase, guests join by just opening a QR link |
| 2 | 2026-10-13 | Stack | **Axum + Askama + htmx** | Leptos (Rust→WASM) | Teaches transferable backend Rust; tiny binary; no JS build step; less framework friction while learning |
| 3 | 2026-10-13 | Sensor | PM2.5 is **its own record type**, entered by hand | Field on smell report | Can be logged alone; ready for automatic sensor input later |
| 4 | 2026-10-13 | Reports | Export (CSV, later PDF) is **future work**; data model ready for it | Build now | YAGNI for v1 |
| 5 | 2026-10-13 | Evidence | Records are **tamper-evident**: full edit history, server-set recorded time | Simple editing | Data may support a formal complaint |
| 6 | 2026-10-13 | Logging | **Backdating allowed with no limit**; both observed and recorded times stored and shown | 24h limit; no backdating | Real life needs flexibility; the visible gap keeps late entries honest |
| 7 | 2026-10-13 | Events | **Every** new signal with an open event asks "continuation or new?", however old the event is | Only ask within a time window | Smell can last from 19:00 until bedtime |
| 8 | 2026-10-13 | Events | **Auto-close all open events at 04:00**, end time = last report | Close after N hours of inactivity; conditional rules | Simple and predictable |
| 9 | 2026-10-13 | Events | After an auto-close, the next log shows a notice offering **"it was a continuation"** (reopen) | Silent auto-close | Recovers incidents that genuinely ran past 04:00 |
| 10 | 2026-10-13 | Events | The auto-close notice goes to the **event owner** (whoever started it), plus residents when the owner is a guest | All users; residents only | The owner knows the context; residents oversee guest events |
| 11 | 2026-10-13 | Roles | Active guests see **only their own** entries, history, and graphs | See everything; see their stay period | Privacy; one consistent rule for active and expired guests |
| 12 | 2026-10-13 | Events | Guests also get the "same smell?" prompt, but see **only the event start time** | Auto-join; no grouping | Guests can say it's a new event without seeing others' data |
| 13 | 2026-10-13 | Events | Choosing "new event" closes the previous one at **its last report** | Close at "now" | Durations are not inflated by forgotten events |
| 14 | 2026-10-13 | Events | **Large-gap warning** (>3h between reports) + residents can **move** reports between events | — | Safety net for wrongly joined reports |
| 15 | 2026-10-13 | Invites | Expiry chosen per invite (quick picks, default 7 days); reactivation asks for a new date | Fixed duration; no expiry | Guest stays vary |
| 16 | 2026-10-13 | Invites | QR invite links expire **5 minutes** after generation and are single-use | 48 hours | Shown in person; minimal exposure |
| 17 | 2026-10-13 | Lists | Rooms and symptoms are **editable lists**; removed items are hidden, never deleted | Hard-coded; free text | Fast taps; clean data for graphs; old records stay intact |
| 18 | 2026-10-13 | Motivation | v1: **fast quick-log screen**, **summary card**, **home-screen shortcut** | Push reminders now | Reduce friction first; push is the most complex PWA feature |
| 19 | 2026-10-13 | Motivation | **Push reminders** deferred and tracked as a GitHub issue | — | See #18 |
| 20 | 2026-10-13 | Logging | In-app **1–10 intensity guide** (smell only) | No guide | Keeps ratings consistent between people |
| 21 | 2026-10-13 | i18n | **English (default), Italian, Turkish**; Fluent; locale in user profile (DB) + cookie before login | localStorage | Server-rendered pages can't read localStorage (would flicker) |
| 22 | 2026-10-13 | Structure | **Cargo workspace with layered crates** (domain / storage / web / app) | Single crate; full hexagonal | The compiler enforces boundaries; good for portfolio and learning without ceremony |
| 23 | 2026-10-13 | Roles | New **viewer** role: read-only, sees **all** data, optional expiry | Guest with expiry = creation + 1s | Explicit, readable, correct visibility |
| 24 | 2026-10-13 | Auth | **Server-side sessions in the DB** (tower-sessions), `__Host-` HttpOnly/Secure/SameSite=Lax cookie, 30 days | JWT | Immediate revocation on guest expiry; standard for server-rendered apps |
| 25 | 2026-10-13 | Evidence | **Hash-chained audit log** + `verify-audit` + hash pinned in backups/exports | Plain audit log | Detects edits made outside the app |
| 26 | 2026-10-13 | Logging | **Usage rules**: correct = edit; situation changed = new report in the same event; smell gone and back = new event | Leave it to users | Consistent data |
| 27 | 2026-10-13 | Lists | Added **entrance hall** (inside) and **stairwell (shared)** (common); rooms have an `area` attribute | — | Smoke in common areas is evidence of its source |
| 28 | 2026-10-13 | Onboarding | After first login, suggest **installing to the home screen**; "Install app" always in the header | — | Makes the shortcut (#18) actually used |
| 29 | 2026-10-13 | Bootstrap | First resident is created via the **`create-resident` CLI** | Default admin; first visitor becomes admin | No credentials in code or repo |
| 30 | 2026-10-13 | Docs | Permissions table, usage rules, and intensity guide go in the **README** | — | Visible to portfolio readers and users |
| 31 | 2026-10-13 | Guests | Guests can correct/move only their own reports, and close/reopen only events they own | — | Least privilege |
| 32 | 2026-10-13 | Auth | Password reset via a resident-generated **5-minute QR link**; no email | Email reset | No email infrastructure, free |
| 33 | 2026-10-13 | UI | **Rooms = toggle chips**, **symptoms = checkboxes** | Both chips; both checkboxes | User preference |
| 34 | 2026-10-13 | Jobs | Auto-close implemented as a **catch-up sweep** (startup, before logging, timer) | Timer only | Free hosts may sleep at 04:00 |
| 35 | 2026-10-13 | Repo | **Public** GitHub repo | Private | Portfolio; free push protection, CodeQL, GHCR |
| 36 | 2026-10-13 | Security | cargo-deny + gitleaks + Trivy + CodeQL + Dependabot; **Snyk optional** | Snyk as primary | Snyk's Rust/Cargo support is limited; RustSec is the Rust standard |
| 37 | 2026-10-13 | Secrets | gitleaks **pre-commit hook** + GitHub push protection + CI scan | CI scan only | CI catches secrets after they're pushed, which is too late |
| 38 | 2026-10-13 | Database | **SQLite** as primary DB | Postgres on VM; managed Postgres (Neon/Supabase) | Tiny data, zero resources, no maintenance, data under own control |
| 39 | 2026-10-13 | Hosting | **Deferred**; must be free with a persistent disk (Oracle Always Free / GCP e2-micro candidates); Litestream → Cloudflare R2 backups | Home server (none available) | No always-on machine at home |
| 40 | 2026-10-13 | Database | **Neon-ready**: repository traits, SQLite + Postgres backends, contract tests on both in CI, `migrate-to-postgres` command | SQLite only; ORM (SeaORM) | Moving to Neon is one command; keeps compile-time-checked SQL |
| 41 | 2026-10-13 | Process | Personal project: commits authored by the personal GitHub account; personal email never in tracked files | — | Keep personal and work identities separate |
| 42 | 2026-10-13 | UI | ⓘ *How to rate* opens a **popup** first (compact table + tip), with a link to the **full guide page** | Straight to a page | Quick reference without losing the half-filled form |
| 43 | 2026-10-13 | Learning | Build the app first; a final **learning pack** milestone adds `docs/learning/` notes per milestone, optional exercises with tests, and walkthroughs. Commit messages describe concepts | Teach during each milestone; tutorial comments in code | User wants the app done first; teaching material stays out of the portfolio code |
| 44 | 2026-10-13 | Repo | `main` protected by a **ruleset kept in git** (`.github/rulesets/protect-main.json`, imported in GitHub settings): PR required (0 approvals, solo project), merge/rebase only (no squash), required checks `fmt · clippy · test` + `cargo-deny` + `gitleaks` on an up-to-date branch, no force-push, no deletion, no bypass | Classic branch protection clicked in the UI | Configuration is reviewable and versioned like code; squash disabled to keep the concept-level history |
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
