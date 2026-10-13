# Roadmap

The design spec (`docs/specs/2026-10-13-smoke-tracker-design.md`) is built in milestones. Each
milestone has its own plan in this folder and ends with working, tested software. A plan is
written only when the previous milestone is done, so it can build on real code instead of
guesses.

| # | Milestone | Delivers | Spec sections | Plan |
|---|---|---|---|---|
| 1 | **Foundation & domain core** | Cargo workspace, secret protection, CI + security workflows, and the pure `domain` crate: types, clock, permissions, 04:00 schedule, event rules, hash-chained audit | §2, §4, §6.1, §7, §10, §11, §12 (secrets), §13.1 | `01-foundation-and-domain.md` ✅ |
| 2 | **Storage** | Repository traits implemented for SQLite **and** Postgres, migrations, audit entry inside every write transaction, contract tests on both backends, schema-parity check, `verify-audit` logic | §6, §6.1, §10.1, §13.3 | — |
| 3 | **Web foundation & auth** | Axum server, config, CLI (`serve`, `migrate`, `create-resident`, `verify-audit`), sessions, login + rate limit, security headers, Origin check, i18n (en/it/tr), base layout, embedded assets, Dockerfile, release workflow, Trivy | §8.1, §8.2, §8.4, §9.2, §10.3–10.5, §12, §13.1 | — |
| 4 | **Logging flow** | Log screen, every prompt (same smell / which event / auto-close notice), close/reopen, PM2.5, events list and detail, report detail (correct, move, history), large-gap warning, auto-close sweep, summary card, "How to rate" popup | §5, §7, §9 (1–4), §11 | — |
| 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | — |
| 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | — |
| 7 | **Neon readiness** | `migrate-to-postgres` command and its CI test | §10.2 | — |
| 8 | **Learning pack** | `docs/learning/` notes per milestone, optional exercises with tests, walkthroughs | Decision #43 | — |

**After milestone 1:** create the GitHub repo, push, enable branch protection and push
protection, and create the push-reminder issue (text in spec §14).

**Still open:** choosing the hosting provider and adding the deploy job. This is needed before
going live, not for any milestone above.
