# Milestone 7b: Look & feel — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every screen gets the approved "Soft cards" look in light and dark (tokens, cards,
avatar menu, bottom tab bar, icons), built the industry-standard way the look-and-feel spec §9
asks for (CSS cascade layers in small files joined at compile time, Askama macros, one icon
sprite, one ES module per behaviour, no duplication, no inline style or script), plus the few
behaviour changes of spec §6: one Log button, event banner counts, graph filters with
per-person lines, and "Move to a new event" from the large-gap warning. CI gains Biome and an
accessibility job (Playwright + axe-core, light and dark).

**Architecture:** Same layering as milestones 3–6. `smoke-web` gains `crates/web/css/` (one
file per layer or component, joined by a `stylesheet!` macro in `assets.rs` into the one
`/static/app.css`), `templates/components.html` and `templates/report_fields.html` (macros),
`static/icons.svg`, `static/js/` (modules), `header::Tab`, `forms::QueryPairs` and
`routes/graphs/{mod,charts,filters}.rs`. `smoke-domain` changes only in `graphs.rs`: `Period`
becomes any run of days and `Filter`/`DayPart` narrow reports and readings (pure). Storage is
untouched: **no migration, no new query, no `.sqlx` change**. CI: a `biome` job and an
`accessibility` job in `ci.yml`; `tests/e2e/` holds the Playwright suite.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8, askama 0.16 (macros, `{% call %}`),
fluent-templates 0.15, chrono + chrono-tz, serde_json. **One new crate, dev-only:**
`scraper 0.27.0` (`default-features = false`; ISC; its Servo dependencies `cssparser` and
`selectors` are MPL-2.0 and never reach the binary; `cargo deny check licenses` passes
unchanged because the configuration does not check dev-dependencies). Biome **2.5.15**
(standalone binary, `biomejs/setup-biome` v2.7.1 pinned by commit). Node 24 in CI only for
`tests/e2e`: `@playwright/test` **1.63.0**, `@axe-core/playwright` **4.13.0** (lock file
committed, `node_modules` never).

**Spec:** `docs/specs/2026-10-14-look-and-feel-design.md` (binding: §2 scope, §3 tokens, §4
screens, §5 Log, §6 behaviour changes, §7 graphs, §9 code standards, §10 testing) and the main
spec `docs/specs/2026-10-13-smoke-tracker-design.md` (§7.7, §9 screens 1, 5, §9.1 as amended
on 2026-10-14). Research: `docs/research/2026-10-14-frontend-standards.md`. Decisions:
`docs/DECISIONS.md` rows **#131–#140**, appended by Task 20. Canvas mockups:
`.superpowers/canvas/*.dc.html` (git-ignored; their inline styles are mockup-only).

**Provenance:** every code block in this plan was compiled and tested before the plan was
written, as one commit per task on the branch **`m7b-proto`** (worktree
`/Users/daniele/Documents/Personal/smoke-tracker-m7bproto`, from `c4e6258`); each task names
its **proto commit** (Task 5 has two: the second is a fix found by the browser seed). At every
commit `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings` and
`cargo test --workspace` passed; at the head also with
`TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres`, `biome ci .` and the
Playwright suite (20 checks, light and dark, all passing). `actionlint` passed on `ci.yml`.
Every screen was looked at in headless Chromium at 390 px in light and dark. Test totals
(SQLite): 595 at `c4e6258`, then 598, 606, 607, 611, 612, 615, 615, 615, 617, 617, 617, 617,
617, 625, 626, 634, 634, 634, 634, **634** after Tasks 1–20. Executors may consult the branch
(`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show <commit>`); **this plan
is the source of truth**. Code is given as unified diffs against the state the previous task
leaves; apply the test hunks first. **Not verified locally:** the two new CI jobs themselves
(the first pull-request run verifies them).

## Decisions for the user to confirm

Defaults were chosen so nothing blocks; each is a decision row (Task 20) and easy to change.

1. **Log button and pre-ticked rooms** (#131): rooms ticked *without any level* count as no
   rooms beside a PM2.5 value, so a reading typed during an event (whose rooms the form ticks
   for you) is saved as a reading instead of "pick a level". Spec wording proposal in
   `.superpowers/M7b-spec-proposals.md`.
2. **Banner counts** (#132): only the reports the viewer may see are counted; a tie goes to the
   most recent report's room; no "highest level" while everything is 0.
3. **Graph filters** (#133): any period up to **two years**, per day up to **62 days**; a room
   or area filter **keeps only the chosen rooms of each report** (a whole-home check drops out);
   "Hide clean checks" drops every report at 0 after that narrowing; the month/year switch
   and its `?month=`/`?year=` addresses are gone (old bookmarks show this month).
4. **Everyone line and colours** (#134): the Everyone line exists only for those who see
   everyone and while no person is chosen; four line colours (`accent`, `series-2…4`) repeat
   after four people.
5. **Move to a new event** (#136) redirects to the moved report's page, as the move route does
   today.
6. **Month and date labels stay numeric** ("10/2026", "Event of 14/10/2026"); the canvas shows
   "October 2026". Month names need 36 new strings; proposal in the spec-proposals file.
7. **A collapsible card's action keeps its word when open** ("Change ⌃"); the canvas switches
   to "Hide". Only the chevron turns.
8. **One new dev-dependency** (`scraper`) and **Node in one CI job** (#139); neither reaches the
   binary or the image (`tests` is in `.dockerignore`).
9. **About 75 new translation keys** in en/it/tr; the Italian and Turkish ones are drafts for
   native review. Removed keys: `log-pm25-only`, `log-error-pm25-missing`, `event-none`,
   `event-open-since`, `summary-this-month`, `summary-events`, `summary-bedroom-days`,
   `summary-checks`, `log-pm25`, `log-time-change`, `graphs-month`, `graphs-year`.
10. **No database migration and no new query**: nothing changes on the live Neon database.

## Questions for the user

1. Should the two new CI jobs (`biome`, `accessibility`) become **required checks** in the
   ruleset after the first green run?
2. Is a two-year cap on custom graph periods right, or should it be longer?

## Global Constraints

- Personal project. Before every commit, `git config user.name` must print `locadani` and
  `git config user.email` must print `61902944+locadani@users.noreply.github.com`. Never put an
  email address in a tracked file.
- Every commit message ends with a blank line followed by exactly
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`; check with
  `git log -1 --format='%an%n%B'`. Commit subjects use Conventional Commits.
- Never commit secrets; the gitleaks pre-commit hook (`git config core.hooksPath .githooks`)
  must be active. No workflow may need a new secret.
- Never modify `description.md`, `CLAUDE.md` or either spec. A needed spec change goes into
  `.superpowers/M7b-spec-proposals.md` (not committed) and the task keeps the default.
- Never push, merge, touch `main`, force-push or run `gh`.
- **No database or storage change**: `0001`–`0004` migrations and every `.sqlx/` file stay
  untouched; no new storage query. Rust changes only where look-and-feel §6/§7 allow them (the
  Log route and form reading, the banner, the gap button, graph filters in the route and in
  pure `smoke_domain::graphs`) plus the template plumbing they need (page structs, `Tab`,
  `QueryPairs`, translated labels).
- **Code standards (spec §9, user rule: very well written, no duplication):** every visual
  pattern exists once (one CSS file per component, one macro per markup pattern, one JS module
  per behaviour, one sprite); **no `style=` attribute, `<style>` element, inline `<script>`
  body or `on*=` handler** in any template (tested); **colours only as tokens in
  `crates/web/css/20-tokens.css`** (tested); every CSS rule inside a declared layer (tested);
  **iOS 16 floor: no CSS nesting, no `light-dark()`**; logical properties; macros take
  translated text, never `tr`; no tutorial comments (doc comments say what and why).
- CSP stays `'self'` only. Scripts: the vendored htmx and Chart.js, `static/js/*.js` modules,
  `static/sw.js`. Every new page stays in `no_page_needs_inline_scripts_styles_or_eval_and_every_icon_exists`.
- Every page works without JavaScript except the charts (as today): disclosure via
  `<details>`, sheets via `<dialog>` with a full-page fallback, native radios, range and select.
- Touch targets ≥ 44 px (`--target`), text contrast ≥ 4.5:1 in both themes, control borders
  ≥ 3:1 (`--control-border`).
- Every new translation key exists in `en`, `it` and `tr`; keys passed through variables are
  listed in the module's `keys()`/`KEYS` for `every_key_the_pages_use_is_translated`.
- Guests see only their own data (spec §4): banner counts, graph filters (no person filter)
  and the Everyone line respect `can_see`.
- Builds share **one** `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target`;
  never `cargo clean`, never `rm -rf` a build folder; check `df -h /System/Volumes/Data` before
  each task (stop under 12 GB). Postgres tests: `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres`
  (container `smoke-pg`; `docker start smoke-pg` and wait for `docker exec smoke-pg pg_isready`).
- Node only inside `tests/e2e` (`npm ci` there); never install anything globally. Biome
  locally: `npx --yes @biomejs/biome@2.5.15 ci .` run from a scratch folder against the repo,
  or the standalone binary; CI uses the binary.
- `docs/DECISIONS.md` is append-only; this plan adds rows **#131–#140** (Task 20).

## Review Focus

1. **No duplication, no inline style or script, tokens only.** Reviewers grep the diff for
   `style=`, `<style`, `<script>` with a body, `#[0-9a-f]` outside `20-tokens.css`, repeated
   markup that should be a macro, repeated CSS that should be one component, repeated JS.
   Tests: `no_template_has_inline_styles_or_scripts`, `colours_are_written_only_as_tokens`,
   `every_rule_sits_in_a_layer`, `the_stylesheet_joins_every_css_file_in_name_order` (Tasks 1–2).
2. **The Log button never saves the wrong thing.** Every row of the §6.1 table, plus pre-ticked
   rooms beside PM2.5. Tests: `the_log_button_decides_from_what_is_filled_in` (unit),
   `the_log_button_saves_what_the_form_holds` (route) (Task 5).
3. **Guests learn nothing new.** Banner counts only visible reports; graphs never offer a person
   filter to guests and ignore `?by=`; no Everyone line for guests. Tests:
   `guests_see_the_open_event_only_if_they_own_it_or_reported_in_it` (Task 6),
   `guests_get_no_person_filter_and_cannot_pick_someone_else`,
   `guests_see_graphs_of_their_own_reports_only` (Tasks 16–17).
4. **The gap button moves only what the viewer may move.** Tests:
   `the_gap_warning_moves_the_report_after_it_to_a_new_event`,
   `the_gap_warning_offers_no_move_without_permission_or_for_a_check` (Task 9).
5. **Filtering is pure and correct.** Narrowing, area vs rooms, the night across midnight,
   minimum level and hidden checks after narrowing, events counted when any report survives.
   Tests in `crates/domain/src/graphs.rs` (Task 14) and `each_filter_narrows_the_graphs`
   (Task 16).
6. **No 400 on a repeated parameter.** Tests: `a_repeated_month_takes_the_first_one` (Task 15),
   `a_repeated_filter_takes_the_first_value` (Task 16).
7. **Accessibility in both themes.** `tests/e2e/specs/accessibility.spec.js` (Task 19) must stay
   at zero violations; reviewers run it when a template or CSS file changes.
8. **No storage change.** `git diff c4e6258 -- crates/storage-* crates/*/migrations` is empty.

## Out of scope (recorded so nobody hunts for it)

Look-and-feel spec §11 (long lists grouped, "Other" symptom, moving a report and everything
after it, a new app icon); month names; a manual theme switch; visual-regression baselines;
Lighthouse; stylelint; djLint. The language buttons on the invite page still lead to `/login`
after a change (as before this milestone; `/lang` is not in §6).

## File Structure

```
crates/web/css/                      NEW  stylesheet sources, joined in name order
  00-layers.css 10-reset.css 20-tokens.css 30-base.css 40-layout.css
  50-icon.css 51-card.css 52-button.css 53-chip.css 54-field.css 55-notice.css
  56-badge.css 57-row.css 60-appbar.css 61-menu.css 62-tabbar.css 63-install.css
  64-sheet.css 65-stats.css 66-room.css 67-switcher.css 68-timeline.css 69-facts.css
  70-items.css 71-people.css 72-guide.css 73-chart.css 90-utilities.css
crates/web/static/app.css            DELETED (served from css/ by assets.rs)
crates/web/static/icons.svg          NEW  the one icon sprite
crates/web/static/js/                NEW  app.js (entry) dialog.js date-picks.js install.js
                                          menu.js range-output.js service-worker.js
                                          graphs.js series-switch.js person-filter.js
crates/web/static/app.js             DELETED (split into js/)
crates/web/static/graphs.js          MOVED to js/graphs.js
crates/web/templates/components.html NEW  macros: icon card collapsible chip field post_button
                                          notice row back languages account_menu tabbar expiry series
crates/web/templates/report_fields.html  REWRITTEN as macros (room rooms symptoms rating_sheet)
crates/web/templates/{room_level,rating_dialog,list_item,expiry_fields}.html  DELETED
crates/web/src/header.rs             Tab, TabLink, initial(), SignedIn::of(user, now, tab)
crates/web/src/i18n.rs               Tr::t2, Tr::languages, LanguageChoice
crates/web/src/forms.rs              QueryPairs, Pairs::contains
crates/web/src/routes/graphs/        mod.rs (handler) charts.rs filters.rs
crates/domain/src/graphs.rs          Period (any days), Granularity, DayPart, Filter
crates/web/tests/common/page.rs      NEW  scraper helper
crates/web/tests/{components,shell}.rs   NEW
biome.json                           NEW
tests/e2e/                           NEW  package.json package-lock.json playwright.config.js
                                          setup/app.js setup/seed.js specs/accessibility.spec.js .gitignore
.github/workflows/ci.yml             + biome, accessibility jobs
.github/dependabot.yml               + npm (tests/e2e)
.dockerignore                        + tests
docs/DECISIONS.md, docs/plans/ROADMAP.md, README.md   Task 20
```

## Rulings made while planning

- **Ruling:** pre-ticked rooms without a level count as no rooms beside PM2.5 (cost if wrong:
  one condition in `entries::read` and two test rows).
- **Ruling:** numeric month/date labels kept (cost if wrong: 36 keys and two formatters).
- **Ruling:** filters narrow each report to the chosen rooms; whole-home checks drop out under
  a room/area filter (cost if wrong: `Filter::reports` and four domain tests).
- **Ruling:** 2-year cap, 62-day daily threshold (cost if wrong: two constants).
- **Ruling:** bucket labels are always day/month (`01/10`), also for one month (cost if wrong:
  `bucket_labels`).
- **Ruling:** `QueryPairs` replaces typed `Query` on every page with a query, not only `/events`
  and `/graphs` (cost if wrong: none; same behaviour for valid addresses).
- **Ruling:** the stats tiles are a `<dl>` (label `dt` under number `dd` via `column-reverse`).
- **Ruling:** the Install card's `.install` hides itself in standalone mode by CSS
  (`63-install.css`), as the header link did.
- **Ruling:** the e2e job creates the resident through `create-resident` under a Python
  pseudo-terminal (the command reads `/dev/tty`); no Rust change for test seeding.


### Task 1: Stylesheet in cascade layers, design tokens, joined at compile time

**Proto commit:** `ed01931` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show ed01931`)

Spec §3.1, §9 (CSS). `assets.rs` gets a `stylesheet!` macro that `concat!`s `crates/web/css/*.css` in name order into the one `/static/app.css`; `static/app.css` is deleted. Tests: the file list equals the macro list in name order, the layer statement comes first, every file is one layer block, and no colour literal appears outside `20-tokens.css`. `theme-color` and the manifest take the new accent. Pages look plain until their task restyles them; that is expected.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/00-layers.css
A	crates/web/css/10-reset.css
A	crates/web/css/20-tokens.css
A	crates/web/css/30-base.css
A	crates/web/css/40-layout.css
A	crates/web/css/90-utilities.css
D	crates/web/static/app.css
M	crates/web/src/assets.rs
M	crates/web/static/manifest.webmanifest
M	crates/web/templates/base.html
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/src/assets.rs (tests module)`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **598 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
style(web): CSS cascade layers and design tokens, joined at compile time with concat!

The stylesheet is now several files under crates/web/css/ (layers, reset, tokens, base,
layout, utilities), joined by a stylesheet! macro into the one /static/app.css. Tests check
the file list and order, that every rule sits in a layer and that colours appear only as
tokens. The old single app.css is gone; theme-color and the manifest use the new accent.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `ed01931`):

````diff
diff --git a/crates/web/css/00-layers.css b/crates/web/css/00-layers.css
new file mode 100644
index 0000000..d8a9b4a
--- /dev/null
+++ b/crates/web/css/00-layers.css
@@ -0,0 +1,4 @@
+/* Smoke Tracker stylesheet. The files in crates/web/css/ are joined in name order into the one
+   /static/app.css (assets.rs). Every rule sits in one of these layers, so a later layer wins
+   whatever the selectors' specificity. */
+@layer reset, tokens, base, layout, components, utilities;
diff --git a/crates/web/css/10-reset.css b/crates/web/css/10-reset.css
new file mode 100644
index 0000000..7d52c97
--- /dev/null
+++ b/crates/web/css/10-reset.css
@@ -0,0 +1,71 @@
+@layer reset {
+  *,
+  *::before,
+  *::after {
+    box-sizing: border-box;
+  }
+
+  html {
+    -webkit-text-size-adjust: 100%;
+    text-size-adjust: 100%;
+  }
+
+  body,
+  h1,
+  h2,
+  h3,
+  p,
+  figure,
+  dl,
+  dd,
+  ol,
+  ul {
+    margin: 0;
+  }
+
+  ol,
+  ul {
+    padding: 0;
+  }
+
+  fieldset {
+    margin: 0;
+    padding: 0;
+    border: 0;
+    min-inline-size: 0;
+  }
+
+  legend {
+    padding: 0;
+  }
+
+  img,
+  svg {
+    display: block;
+  }
+
+  input,
+  button,
+  select,
+  textarea {
+    font: inherit;
+    color: inherit;
+  }
+
+  table {
+    border-collapse: collapse;
+  }
+
+  [hidden] {
+    display: none !important;
+  }
+
+  @media (prefers-reduced-motion: reduce) {
+    *,
+    *::before,
+    *::after {
+      animation-duration: 0.01ms !important;
+      transition-duration: 0.01ms !important;
+    }
+  }
+}
diff --git a/crates/web/css/20-tokens.css b/crates/web/css/20-tokens.css
new file mode 100644
index 0000000..7a4240e
--- /dev/null
+++ b/crates/web/css/20-tokens.css
@@ -0,0 +1,103 @@
+/* Design tokens (look-and-feel spec §3.1). The only file with colour values: every other file
+   uses these custom properties. Dark values follow the system setting. */
+@layer tokens {
+  :root {
+    color-scheme: light dark;
+
+    --bg: #f2f3f0;
+    --card: #ffffff;
+    --field: #ffffff;
+    --text: #1b1f1d;
+    --muted: #5c6560;
+    --line: #dfe3df;
+    --accent: #2f6b5e;
+    --accent-soft: #e2eeea;
+    --on-accent: #ffffff;
+    --warn-bg: #fbefd9;
+    --warn-text: #6a4710;
+    --amber: #b7791f;
+    --danger: #a33a2a;
+    --control-border: #7f8984;
+    --shadow: 0 8px 24px rgb(0 0 0 / 12%);
+    --backdrop: rgb(0 0 0 / 45%);
+
+    /* Chart lines after the first person's (which uses --accent). */
+    --series-2: #3a5ba0;
+    --series-3: #8a4f9e;
+    --series-4: #4d7a1f;
+
+    /* The heatmap ramp: dark text on 0–5, --on-accent text on 6–10. */
+    --heat-0: #e2eeea;
+    --heat-1: #d1e1dc;
+    --heat-2: #c1d4ce;
+    --heat-3: #b0c8c0;
+    --heat-4: #a0bbb2;
+    --heat-5: #8faea4;
+    --heat-6: #4f776c;
+    --heat-7: #41685e;
+    --heat-8: #335a50;
+    --heat-9: #254c41;
+    --heat-10: #173d33;
+
+    --font: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
+    --text-title: 1.875rem;
+    --text-brand: 1.375rem;
+    --text-card: 1.125rem;
+    --text-label: 0.875rem;
+    --text-small: 0.8125rem;
+    --text-tiny: 0.75rem;
+
+    --radius-card: 20px;
+    --radius-button: 16px;
+    --radius-field: 12px;
+    --radius-pill: 999px;
+
+    --space-1: 0.25rem;
+    --space-2: 0.5rem;
+    --space-3: 0.75rem;
+    --space-4: 1rem;
+    --space-5: 1.5rem;
+    --gutter: 1rem;
+    --column: 32rem;
+    --target: 44px;
+    --field-height: 48px;
+    --tabbar-height: 4rem;
+  }
+
+  @media (prefers-color-scheme: dark) {
+    :root {
+      --bg: #111513;
+      --card: #1b201e;
+      --field: #151917;
+      --text: #e7ece9;
+      --muted: #a2aca6;
+      --line: #2d3431;
+      --accent: #74c2ac;
+      --accent-soft: #21332e;
+      --on-accent: #0c1311;
+      --warn-bg: #332812;
+      --warn-text: #f2d9a2;
+      --amber: #e0a84a;
+      --danger: #f09a8a;
+      --control-border: #6f7a74;
+      --shadow: 0 8px 24px rgb(0 0 0 / 50%);
+      --backdrop: rgb(0 0 0 / 60%);
+
+      --series-2: #8fb0f0;
+      --series-3: #d3a2e3;
+      --series-4: #b8d98a;
+
+      --heat-0: #21332e;
+      --heat-1: #273c37;
+      --heat-2: #2d4640;
+      --heat-3: #344f48;
+      --heat-4: #3a5951;
+      --heat-5: #40625a;
+      --heat-6: #86b5a7;
+      --heat-7: #98c3b6;
+      --heat-8: #aad1c6;
+      --heat-9: #bddfd5;
+      --heat-10: #cfede4;
+    }
+  }
+}
diff --git a/crates/web/css/30-base.css b/crates/web/css/30-base.css
new file mode 100644
index 0000000..e9b78d4
--- /dev/null
+++ b/crates/web/css/30-base.css
@@ -0,0 +1,86 @@
+/* Elements without classes: type, links, form controls and focus. */
+@layer base {
+  body {
+    font-family: var(--font);
+    font-size: 1rem;
+    line-height: 1.45;
+    background: var(--bg);
+    color: var(--text);
+  }
+
+  h1 {
+    font-size: var(--text-title);
+    line-height: 1.15;
+    font-weight: 700;
+    letter-spacing: -0.01em;
+  }
+
+  h2 {
+    font-size: var(--text-card);
+    line-height: 1.3;
+    font-weight: 650;
+  }
+
+  a {
+    color: var(--accent);
+    text-underline-offset: 0.15em;
+  }
+
+  strong {
+    font-weight: 650;
+  }
+
+  label,
+  legend {
+    font-size: var(--text-label);
+    font-weight: 600;
+  }
+
+  input:not([type="checkbox"], [type="radio"], [type="range"]),
+  select,
+  textarea {
+    inline-size: 100%;
+    min-block-size: var(--field-height);
+    padding-inline: var(--space-3);
+    border: 1px solid var(--control-border);
+    border-radius: var(--radius-field);
+    background: var(--field);
+    font-size: 1rem;
+    font-weight: 400;
+  }
+
+  textarea {
+    padding-block: var(--space-3);
+    resize: vertical;
+  }
+
+  input[type="checkbox"],
+  input[type="radio"],
+  input[type="range"] {
+    accent-color: var(--accent);
+  }
+
+  button {
+    cursor: pointer;
+  }
+
+  :focus-visible {
+    outline: 2px solid var(--accent);
+    outline-offset: 2px;
+  }
+
+  table {
+    inline-size: 100%;
+  }
+
+  th,
+  td {
+    text-align: start;
+    vertical-align: top;
+  }
+
+  code {
+    font-size: var(--text-tiny);
+    overflow-wrap: anywhere;
+  }
+}
diff --git a/crates/web/css/40-layout.css b/crates/web/css/40-layout.css
new file mode 100644
index 0000000..164897a
--- /dev/null
+++ b/crates/web/css/40-layout.css
@@ -0,0 +1,52 @@
+/* The page column and the composition classes that space their children. */
+@layer layout {
+  html {
+    scroll-padding-block-end: calc(var(--tabbar-height) + env(safe-area-inset-bottom));
+  }
+
+  body {
+    min-block-size: 100vh;
+    display: flex;
+    flex-direction: column;
+  }
+
+  main {
+    inline-size: 100%;
+    max-inline-size: var(--column);
+    margin-inline: auto;
+    padding: var(--space-2) var(--gutter) var(--space-5);
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-4);
+    flex-grow: 1;
+  }
+
+  body:has(> .tabbar) main {
+    padding-block-end: calc(var(--tabbar-height) + env(safe-area-inset-bottom) + var(--space-5));
+  }
+
+  .stack {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-3);
+  }
+
+  .cluster {
+    display: flex;
+    flex-wrap: wrap;
+    align-items: center;
+    gap: var(--space-2);
+  }
+
+  .split {
+    display: flex;
+    align-items: center;
+    justify-content: space-between;
+    gap: var(--space-3);
+  }
+
+  .center {
+    justify-content: center;
+    text-align: center;
+  }
+}
diff --git a/crates/web/css/90-utilities.css b/crates/web/css/90-utilities.css
new file mode 100644
index 0000000..2416643
--- /dev/null
+++ b/crates/web/css/90-utilities.css
@@ -0,0 +1,14 @@
+@layer utilities {
+  .visually-hidden {
+    position: absolute;
+    inline-size: 1px;
+    block-size: 1px;
+    overflow: hidden;
+    clip-path: inset(50%);
+    white-space: nowrap;
+  }
+
+  .muted {
+    color: var(--muted);
+  }
+}
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index cc936c0..b9efeaa 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -1,5 +1,6 @@
 //! Static files, compiled into the binary (spec §10.3). Each is listed here explicitly; a test
-//! checks the list matches the `static/` folder.
+//! checks the list matches the `static/` folder. The stylesheet is the exception: it is written
+//! as several files under `css/` and joined at compile time into the one `/static/app.css`.
 
 use std::sync::LazyLock;
 
@@ -25,8 +26,33 @@ macro_rules! asset {
     };
 }
 
+/// Joins the stylesheet sources, in the order given, into one `&str`, and lists their names.
+macro_rules! stylesheet {
+    ($($file:literal),+ $(,)?) => {
+        (
+            concat!($(include_str!(concat!("../css/", $file))),+),
+            &[$($file),+],
+        )
+    };
+}
+
+/// The served stylesheet and its sources, in cascade order: `00-layers.css` declares the
+/// layers, so it comes first.
+const STYLESHEET: (&str, &[&str]) = stylesheet!(
+    "00-layers.css",
+    "10-reset.css",
+    "20-tokens.css",
+    "30-base.css",
+    "40-layout.css",
+    "90-utilities.css",
+);
+
 const ASSETS: &[Asset] = &[
-    asset!("app.css", "text/css; charset=utf-8"),
+    Asset {
+        path: "app.css",
+        content_type: "text/css; charset=utf-8",
+        body: STYLESHEET.0.as_bytes(),
+    },
     asset!("app.js", "text/javascript; charset=utf-8"),
     asset!("chart-4.5.1.umd.min.js", "text/javascript; charset=utf-8"),
     asset!("favicon.svg", "image/svg+xml"),
@@ -91,17 +117,111 @@ mod tests {
     const CHART_JS_SHA256: &str =
         "48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a";
 
-    #[test]
-    fn every_file_in_the_static_folder_is_served() {
-        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
-        let on_disk: BTreeSet<String> = std::fs::read_dir(dir)
+    /// The names of the files in `dir` (relative to the crate), sorted.
+    fn files_in(dir: &str) -> Vec<String> {
+        let path = format!("{}/{dir}", env!("CARGO_MANIFEST_DIR"));
+        let mut names: Vec<String> = std::fs::read_dir(path)
             .unwrap()
             .map(|entry| entry.unwrap().file_name().into_string().unwrap())
             .collect();
-        let listed: BTreeSet<String> = ASSETS.iter().map(|a| a.path.to_owned()).collect();
+        names.sort();
+        names
+    }
+
+    #[test]
+    fn every_file_in_the_static_folder_is_served() {
+        let on_disk: BTreeSet<String> = files_in("static").into_iter().collect();
+        let listed: BTreeSet<String> = ASSETS
+            .iter()
+            .map(|a| a.path.to_owned())
+            .filter(|path| path != "app.css")
+            .collect();
         assert_eq!(on_disk, listed);
     }
 
+    #[test]
+    fn the_stylesheet_joins_every_css_file_in_name_order() {
+        assert_eq!(files_in("css"), STYLESHEET.1);
+        assert!(
+            STYLESHEET
+                .0
+                .contains("@layer reset, tokens, base, layout, components, utilities;"),
+            "the layer order is declared"
+        );
+        let first_rule = STYLESHEET.0.find('{').unwrap();
+        let declaration = STYLESHEET.0.find("@layer reset,").unwrap();
+        assert!(declaration < first_rule, "the layers are declared first");
+    }
+
+    /// The source of each stylesheet file, by name.
+    fn css_sources() -> impl Iterator<Item = (&'static str, String)> {
+        STYLESHEET.1.iter().map(|&name| {
+            let path = format!("{}/css/{name}", env!("CARGO_MANIFEST_DIR"));
+            (name, std::fs::read_to_string(path).unwrap())
+        })
+    }
+
+    #[test]
+    fn every_rule_sits_in_a_layer() {
+        for (name, source) in css_sources().skip(1) {
+            let code = without_comments(&source);
+            let code = code.trim_start();
+            assert!(code.starts_with("@layer "), "{name} starts outside a layer");
+            let body_end = matching_brace(code, code.find('{').unwrap());
+            assert!(
+                code[body_end + 1..].trim().is_empty(),
+                "{name} has rules after its layer block"
+            );
+        }
+    }
+
+    #[test]
+    fn colours_are_written_only_as_tokens() {
+        for (name, source) in css_sources().filter(|(name, _)| *name != "20-tokens.css") {
+            let code = without_comments(&source);
+            for function in ["rgb(", "rgba(", "hsl(", "hwb(", "lab(", "lch(", "oklch("] {
+                assert!(
+                    !code.contains(function),
+                    "{name} has a raw colour: {function}"
+                );
+            }
+            let hex = code
+                .match_indices('#')
+                .find(|(at, _)| code[at + 1..].starts_with(|c: char| c.is_ascii_hexdigit()));
+            assert_eq!(hex, None, "{name} has a hex colour");
+        }
+    }
+
+    fn without_comments(css: &str) -> String {
+        let mut out = String::new();
+        let mut rest = css;
+        while let Some(start) = rest.find("/*") {
+            out.push_str(&rest[..start]);
+            let end = rest[start..].find("*/").expect("a closed comment");
+            rest = &rest[start + end + 2..];
+        }
+        out.push_str(rest);
+        out
+    }
+
+    /// The index of the `}` that closes the `{` at `open`.
+    fn matching_brace(code: &str, open: usize) -> usize {
+        let mut depth = 0;
+        for (index, byte) in code.bytes().enumerate().skip(open) {
+            match byte {
+                b'{' => depth += 1,
+                b'}' => {
+                    depth -= 1;
+                    if depth == 0 {
+                        return index;
+                    }
+                }
+                _ => {}
+            }
+        }
+        panic!("an unclosed block");
+    }
+
     fn asset(path: &str) -> &'static Asset {
         ASSETS.iter().find(|a| a.path == path).unwrap()
     }
diff --git a/crates/web/static/app.css b/crates/web/static/app.css
deleted file mode 100644
index 74e731e..0000000
--- a/crates/web/static/app.css
+++ /dev/null
@@ -1,449 +0,0 @@
-:root {
-  color-scheme: light dark;
-  --gap: 1rem;
-  --accent: #b4532a;
-  font-family: system-ui, sans-serif;
-  line-height: 1.5;
-}
-
-body {
-  margin: 0 auto;
-  max-width: 32rem;
-  padding: 0 var(--gap);
-}
-
-.bar {
-  display: flex;
-  flex-wrap: wrap;
-  align-items: center;
-  justify-content: space-between;
-  padding: var(--gap) 0;
-}
-
-.brand {
-  font-weight: 700;
-}
-
-.stack {
-  display: flex;
-  flex-direction: column;
-  gap: var(--gap);
-}
-
-.stack label {
-  display: flex;
-  flex-direction: column;
-  gap: 0.25rem;
-}
-
-input,
-button {
-  font: inherit;
-  padding: 0.75rem;
-  border-radius: 0.5rem;
-}
-
-button {
-  border: none;
-  background: var(--accent);
-  color: #fff;
-}
-
-button.link {
-  background: none;
-  color: inherit;
-  padding: 0.25rem;
-  text-decoration: underline;
-}
-
-.notice {
-  padding: 0.75rem;
-  border-left: 0.25rem solid var(--accent);
-}
-
-footer {
-  margin-top: calc(var(--gap) * 3);
-  font-size: 0.875rem;
-}
-
-.who {
-  display: flex;
-  align-items: center;
-  gap: 0.5rem;
-  font-size: 0.875rem;
-}
-
-.status,
-.summary {
-  margin: 0.5rem 0;
-}
-
-.summary {
-  font-size: 0.875rem;
-}
-
-fieldset {
-  border: none;
-  margin: 0;
-  padding: 0;
-}
-
-legend {
-  font-weight: 600;
-  margin-bottom: 0.5rem;
-}
-
-.rooms {
-  display: flex;
-  flex-wrap: wrap;
-  gap: 0.5rem;
-}
-
-.rooms legend,
-.rooms .group {
-  flex-basis: 100%;
-  margin: 0;
-}
-
-.room:has(input[name="room"]:checked) {
-  flex-basis: 100%;
-  margin: 0;
-}
-
-.levels {
-  display: grid;
-  grid-template-columns: repeat(6, 1fr);
-  gap: 0.5rem;
-  margin-top: 0.5rem;
-}
-
-.levels legend {
-  grid-column: 1 / -1;
-  font-weight: 400;
-  margin: 0;
-}
-
-.room:not(:has(input[name="room"]:checked)) .levels {
-  display: none;
-}
-
-.chip {
-  position: relative;
-}
-
-.chip input {
-  position: absolute;
-  opacity: 0;
-}
-
-.chip span {
-  display: block;
-  padding: 0.6rem 0.9rem;
-  border: 1px solid currentColor;
-  border-radius: 999px;
-  text-align: center;
-}
-
-.chip input:checked + span {
-  background: var(--accent);
-  border-color: var(--accent);
-  color: #fff;
-}
-
-.chip input:focus-visible + span {
-  outline: 2px solid var(--accent);
-  outline-offset: 2px;
-}
-
-.checks label {
-  display: block;
-  padding: 0.25rem 0;
-}
-
-.info {
-  font-weight: 400;
-  margin-left: 0.5rem;
-}
-
-textarea {
-  font: inherit;
-  padding: 0.75rem;
-  border-radius: 0.5rem;
-}
-
-button.secondary {
-  background: none;
-  color: inherit;
-  border: 1px solid currentColor;
-}
-
-dialog {
-  padding: 0;
-  border: none;
-  border-radius: 0.75rem;
-  max-width: 28rem;
-}
-
-dialog::backdrop {
-  background: rgb(0 0 0 / 50%);
-}
-
-.dialog-body {
-  padding: var(--gap);
-}
-
-.dialog-close {
-  float: right;
-}
-
-table.guide {
-  border-collapse: collapse;
-  width: 100%;
-}
-
-table.guide th,
-table.guide td {
-  text-align: left;
-  vertical-align: top;
-  padding: 0.35rem 0.5rem;
-  border-bottom: 1px solid rgb(128 128 128 / 30%);
-}
-
-.menu {
-  display: flex;
-  gap: var(--gap);
-}
-
-.status {
-  display: flex;
-  align-items: center;
-  justify-content: space-between;
-  gap: var(--gap);
-}
-
-.months {
-  display: flex;
-  justify-content: space-between;
-  align-items: baseline;
-}
-
-.rows,
-.timeline {
-  padding-left: 1.25rem;
-}
-
-.timeline .at {
-  font-variant-numeric: tabular-nums;
-  margin-right: 0.5rem;
-}
-
-.timeline .gap {
-  list-style: none;
-  margin: 0.5rem 0 0.5rem -1.25rem;
-  padding: 0.5rem 0.75rem;
-  border-left: 0.25rem solid var(--accent);
-}
-
-.by {
-  opacity: 0.7;
-}
-
-.badge {
-  font-size: 0.75rem;
-  font-weight: 400;
-  padding: 0.1rem 0.5rem;
-  border: 1px solid currentColor;
-  border-radius: 999px;
-  vertical-align: middle;
-}
-
-.facts {
-  display: grid;
-  grid-template-columns: max-content 1fr;
-  gap: 0.25rem var(--gap);
-}
-
-.facts dt {
-  font-weight: 600;
-}
-
-.facts dd {
-  margin: 0;
-}
-
-select {
-  font: inherit;
-  padding: 0.75rem;
-  border-radius: 0.5rem;
-}
-
-.choices {
-  display: flex;
-  flex-wrap: wrap;
-  gap: 0.5rem;
-}
-
-button.current {
-  background: var(--accent);
-  color: #fff;
-}
-
-.hint {
-  font-size: 0.875rem;
-  margin: 0;
-  opacity: 0.8;
-}
-
-.items {
-  list-style: none;
-  padding: 0;
-}
-
-.item {
-  display: flex;
-  align-items: center;
-  justify-content: space-between;
-  gap: var(--gap);
-}
-
-.people {
-  list-style: none;
-  padding: 0;
-}
-
-.people li {
-  padding: 0.5rem 0;
-  border-bottom: 1px solid rgb(128 128 128 / 30%);
-}
-
-.expiry input[type="date"] {
-  font: inherit;
-  padding: 0.5rem;
-  border-radius: 0.5rem;
-}
-
-.invite:has(input[name="role"]:checked) .expiry {
-  display: none;
-}
-
-.invite:has(input[name="role"][value="guest"]:checked) .expiry[data-role="guest"],
-.invite:has(input[name="role"][value="viewer"]:checked) .expiry[data-role="viewer"] {
-  display: flex;
-}
-
-.qr-code {
-  margin: 0;
-  max-width: 18rem;
-}
-
-.qr {
-  display: block;
-  width: 100%;
-  height: auto;
-}
-
-.link-text {
-  overflow-wrap: anywhere;
-  font-size: 0.75rem;
-}
-
-.install {
-  margin-left: auto;
-  margin-right: var(--gap);
-  font-size: 0.875rem;
-}
-
-@media (display-mode: standalone) {
-  .install {
-    display: none;
-  }
-}
-
-.steps li {
-  margin: 0.5rem 0;
-}
-
-.glyph {
-  width: 1.25em;
-  height: 1.25em;
-  vertical-align: text-bottom;
-}
-
-.chart {
-  position: relative;
-  height: 18rem;
-}
-
-.scroll {
-  overflow-x: auto;
-}
-
-table.heatmap {
-  border-collapse: collapse;
-  font-size: 0.75rem;
-}
-
-table.heatmap th,
-table.heatmap td {
-  min-width: 1.25rem;
-  padding: 0.25rem 0.125rem;
-  text-align: center;
-}
-
-table.heatmap tbody th {
-  text-align: left;
-  white-space: nowrap;
-}
-
-.heat-0 {
-  background: color-mix(in srgb, var(--accent) 6%, transparent);
-}
-
-.heat-1 {
-  background: color-mix(in srgb, var(--accent) 12%, transparent);
-}
-
-.heat-2 {
-  background: color-mix(in srgb, var(--accent) 20%, transparent);
-}
-
-.heat-3 {
-  background: color-mix(in srgb, var(--accent) 28%, transparent);
-}
-
-.heat-4 {
-  background: color-mix(in srgb, var(--accent) 36%, transparent);
-}
-
-.heat-5 {
-  background: color-mix(in srgb, var(--accent) 44%, transparent);
-}
-
-.heat-6 {
-  background: color-mix(in srgb, var(--accent) 54%, transparent);
-  color: #fff;
-}
-
-.heat-7 {
-  background: color-mix(in srgb, var(--accent) 64%, transparent);
-  color: #fff;
-}
-
-.heat-8 {
-  background: color-mix(in srgb, var(--accent) 76%, transparent);
-  color: #fff;
-}
-
-.heat-9 {
-  background: color-mix(in srgb, var(--accent) 88%, transparent);
-  color: #fff;
-}
-
-.heat-10 {
-  background: color-mix(in srgb, var(--accent) 100%, transparent);
-  color: #fff;
-}
-
-.rules li {
-  margin: 0.5rem 0;
-}
diff --git a/crates/web/static/manifest.webmanifest b/crates/web/static/manifest.webmanifest
index 42d2a64..4c54f30 100644
--- a/crates/web/static/manifest.webmanifest
+++ b/crates/web/static/manifest.webmanifest
@@ -6,8 +6,8 @@
   "start_url": "/",
   "scope": "/",
   "display": "standalone",
-  "background_color": "#ffffff",
-  "theme_color": "#b4532a",
+  "background_color": "#f2f3f0",
+  "theme_color": "#2f6b5e",
   "icons": [
     { "src": "/static/icon-192.png", "sizes": "192x192", "type": "image/png" },
     { "src": "/static/icon-512.png", "sizes": "512x512", "type": "image/png" },
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index 779fa22..d1f359e 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -2,10 +2,10 @@
 <html lang="{{ tr.lang() }}">
 <head>
   <meta charset="utf-8">
-  <meta name="viewport" content="width=device-width, initial-scale=1">
+  <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
   <meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>
   <title>{% block title %}{% endblock %} · {{ tr.t("app-name") }}</title>
-  <meta name="theme-color" content="#b4532a">
+  <meta name="theme-color" content="#2f6b5e">
   <link rel="icon" href="/static/favicon.svg" type="image/svg+xml">
   <link rel="apple-touch-icon" href="/static/icon-180.png">
   <link rel="manifest" href="/static/manifest.webmanifest">
````

### Task 2: Component macros, icon sprite, scraper helper and the template scan

**Proto commit:** `efb11ea` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show efb11ea`)

Spec §3.3, §9 (templates, icons), §10. `templates/components.html` (first macros), `static/icons.svg`, component CSS files 50–57, `scraper` as a workspace dev-dependency (`Cargo.toml`, `crates/web/Cargo.toml`, `Cargo.lock`), `tests/common/page.rs`, `tests/components.rs` (renders every macro through an inline Askama template), and `no_template_has_inline_styles_or_scripts` in `tests/security.rs`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/50-icon.css
A	crates/web/css/51-card.css
A	crates/web/css/52-button.css
A	crates/web/css/53-chip.css
A	crates/web/css/54-field.css
A	crates/web/css/55-notice.css
A	crates/web/css/56-badge.css
A	crates/web/css/57-row.css
A	crates/web/static/icons.svg
A	crates/web/templates/components.html
A	crates/web/tests/common/page.rs
A	crates/web/tests/components.rs
M	Cargo.lock
M	Cargo.toml
M	crates/web/Cargo.toml
M	crates/web/src/assets.rs
M	crates/web/tests/common/mod.rs
M	crates/web/tests/security.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/components.rs`, `crates/web/tests/security.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **606 passed** (SQLite). Run `cargo fetch` once for scraper; `Cargo.lock` changes (dev-only crates).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(web): Askama component macros, an SVG icon sprite and scraper-based HTML assertions

templates/components.html holds one macro per repeated markup pattern (card, collapsible
card, chip, field, one-button form, notice, list row, back link, language buttons, icon),
styled by one CSS file per component. static/icons.svg is the one icon sprite. Tests render
every macro through an inline template and query the result with scraper (tests/common/page.rs);
a scan refuses inline styles, scripts and handlers in any template.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `efb11ea`):

````diff
diff --git a/Cargo.toml b/Cargo.toml
index 0c4108b..4ec7467 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -26,6 +26,7 @@ http-body-util = "0.1.5"
 proptest = "1"
 qrcodegen = "1.8.0"
 rpassword = "7.5.4"
+scraper = { version = "0.27.0", default-features = false }
 serde = { version = "1", features = ["derive"] }
 serde_json = { version = "1", features = ["float_roundtrip"] }
 sha2 = "0.11"
diff --git a/crates/web/Cargo.toml b/crates/web/Cargo.toml
index 6391236..782bc1e 100644
--- a/crates/web/Cargo.toml
+++ b/crates/web/Cargo.toml
@@ -32,6 +32,7 @@ tracing.workspace = true
 [dev-dependencies]
 fluent-syntax.workspace = true
 http-body-util.workspace = true
+scraper.workspace = true
 smoke-storage-sqlite.workspace = true
 sqlx = { workspace = true, features = ["sqlite-bundled"] }
 tempfile.workspace = true
diff --git a/crates/web/css/50-icon.css b/crates/web/css/50-icon.css
new file mode 100644
index 0000000..1d61127
--- /dev/null
+++ b/crates/web/css/50-icon.css
@@ -0,0 +1,7 @@
+@layer components {
+  .icon {
+    inline-size: 1.25em;
+    block-size: 1.25em;
+    flex-shrink: 0;
+  }
+}
diff --git a/crates/web/css/51-card.css b/crates/web/css/51-card.css
new file mode 100644
index 0000000..08cb0e9
--- /dev/null
+++ b/crates/web/css/51-card.css
@@ -0,0 +1,46 @@
+/* .card, and .collapsible: a card built on <details> whose summary is the whole tap target. */
+@layer components {
+  .card {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-3);
+    padding: var(--space-4);
+    border-radius: var(--radius-card);
+    background: var(--card);
+  }
+
+  .collapsible {
+    padding-block: var(--space-1);
+  }
+
+  .collapsible > summary {
+    display: flex;
+    align-items: center;
+    justify-content: space-between;
+    gap: var(--space-3);
+    min-block-size: 3.25rem;
+    list-style: none;
+    cursor: pointer;
+  }
+
+  .collapsible > summary::-webkit-details-marker {
+    display: none;
+  }
+
+  .collapsible .action {
+    display: inline-flex;
+    align-items: center;
+    gap: var(--space-1);
+    color: var(--accent);
+    font-size: 0.9375rem;
+    font-weight: 600;
+  }
+
+  .collapsible[open] .action .icon {
+    transform: rotate(180deg);
+  }
+
+  .collapsible[open] > .stack {
+    padding-block-end: var(--space-3);
+  }
+}
diff --git a/crates/web/css/52-button.css b/crates/web/css/52-button.css
new file mode 100644
index 0000000..9900573
--- /dev/null
+++ b/crates/web/css/52-button.css
@@ -0,0 +1,58 @@
+/* .button: primary by default; data-variant="secondary", "danger" or "quiet". */
+@layer components {
+  .button {
+    display: inline-flex;
+    align-items: center;
+    justify-content: center;
+    gap: var(--space-2);
+    min-block-size: var(--target);
+    padding: var(--space-2) var(--space-4);
+    border: 1px solid var(--accent);
+    border-radius: var(--radius-button);
+    background: var(--accent);
+    color: var(--on-accent);
+    font-size: 1rem;
+    font-weight: 650;
+    text-align: center;
+    text-decoration: none;
+  }
+
+  .button[data-variant="secondary"] {
+    border-color: var(--control-border);
+    background: var(--card);
+    color: var(--accent);
+  }
+
+  .button[data-variant="danger"] {
+    border-color: var(--danger);
+    background: transparent;
+    color: var(--danger);
+  }
+
+  .button[data-variant="quiet"] {
+    border-color: transparent;
+    background: transparent;
+    color: var(--accent);
+    padding-inline: var(--space-2);
+  }
+
+  /* Buttons that share a row: the main one takes three parts in five (look-and-feel spec §5). */
+  .actions {
+    display: grid;
+    grid-template-columns: 2fr 3fr;
+    gap: var(--space-3);
+  }
+
+  .actions .button {
+    min-block-size: 3.25rem;
+  }
+
+  .back {
+    display: inline-flex;
+    align-items: center;
+    gap: var(--space-1);
+    min-block-size: var(--target);
+    font-weight: 600;
+    text-decoration: none;
+  }
+}
diff --git a/crates/web/css/53-chip.css b/crates/web/css/53-chip.css
new file mode 100644
index 0000000..4b3a60a
--- /dev/null
+++ b/crates/web/css/53-chip.css
@@ -0,0 +1,68 @@
+/* .chip: a native checkbox or radio drawn as a pill. .segmented: a row of choices in one frame,
+   from radios (chips) or submit buttons (the current one marked aria-current). */
+@layer components {
+  .chip {
+    position: relative;
+    display: inline-flex;
+    font-size: 0.9375rem;
+    font-weight: 550;
+  }
+
+  .chip > input {
+    position: absolute;
+    inset: 0;
+    margin: 0;
+    opacity: 0;
+  }
+
+  .chip > span {
+    display: inline-flex;
+    align-items: center;
+    justify-content: center;
+    inline-size: 100%;
+    min-block-size: var(--target);
+    padding-inline: var(--space-4);
+    border: 1px solid var(--line);
+    border-radius: var(--radius-pill);
+  }
+
+  .chip > input:checked + span {
+    border-color: var(--accent);
+    background: var(--accent-soft);
+    color: var(--accent);
+  }
+
+  .chip > input:focus-visible + span {
+    outline: 2px solid var(--accent);
+    outline-offset: 2px;
+  }
+
+  .segmented {
+    display: flex;
+    gap: var(--space-1);
+    padding: var(--space-1);
+    border: 1px solid var(--line);
+    border-radius: var(--radius-field);
+    background: var(--bg);
+  }
+
+  .segmented > * {
+    flex: 1 1 0;
+  }
+
+  .segmented > button,
+  .segmented > .chip > span {
+    min-block-size: var(--target);
+    border: 0;
+    border-radius: 10px;
+    background: transparent;
+    font-weight: 600;
+  }
+
+  .segmented > [aria-current="true"],
+  .segmented > .chip > input:checked + span {
+    background: var(--card);
+    color: var(--accent);
+    box-shadow: var(--shadow);
+  }
+}
diff --git a/crates/web/css/54-field.css b/crates/web/css/54-field.css
new file mode 100644
index 0000000..72e1eae
--- /dev/null
+++ b/crates/web/css/54-field.css
@@ -0,0 +1,29 @@
+/* .field: a label above its control. .inline-field: label and a short control on one line. */
+@layer components {
+  .field {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-1);
+  }
+
+  .field > span {
+    color: var(--muted);
+  }
+
+  .inline-field {
+    display: flex;
+    align-items: center;
+    justify-content: space-between;
+    gap: var(--space-3);
+  }
+
+  .inline-field input {
+    inline-size: 7rem;
+    text-align: end;
+  }
+
+  .hint {
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+}
diff --git a/crates/web/css/55-notice.css b/crates/web/css/55-notice.css
new file mode 100644
index 0000000..24bc7bf
--- /dev/null
+++ b/crates/web/css/55-notice.css
@@ -0,0 +1,34 @@
+/* .notice: a confirmation (role="status") or an error (role="alert") above a form.
+   .banner: the amber strip of something going on, with its action. */
+@layer components {
+  .notice {
+    padding: var(--space-3) var(--space-4);
+    border-radius: var(--radius-button);
+    background: var(--accent-soft);
+  }
+
+  .notice[role="alert"],
+  .banner {
+    background: var(--warn-bg);
+    color: var(--warn-text);
+  }
+
+  .banner {
+    display: flex;
+    align-items: center;
+    gap: var(--space-3);
+    padding: var(--space-3) var(--space-4);
+    border-radius: var(--radius-button);
+  }
+
+  .banner > :first-child {
+    flex-grow: 1;
+  }
+
+  .banner .button {
+    border-color: var(--warn-text);
+    background: transparent;
+    color: var(--warn-text);
+    font-size: var(--text-label);
+  }
+}
diff --git a/crates/web/css/56-badge.css b/crates/web/css/56-badge.css
new file mode 100644
index 0000000..9d8a426
--- /dev/null
+++ b/crates/web/css/56-badge.css
@@ -0,0 +1,40 @@
+/* .badge: the square in front of a list line. .tag: a small pill after a name.
+   data-tone="warn" or "muted" changes either. */
+@layer components {
+  .badge {
+    display: inline-flex;
+    align-items: center;
+    justify-content: center;
+    flex-shrink: 0;
+    inline-size: 2.5rem;
+    block-size: 2.5rem;
+    border-radius: var(--radius-field);
+    background: var(--accent-soft);
+    color: var(--accent);
+    font-size: var(--text-label);
+    font-weight: 700;
+  }
+
+  .tag {
+    display: inline-block;
+    padding: 0.125rem var(--space-2);
+    border-radius: var(--radius-pill);
+    background: var(--accent-soft);
+    color: var(--accent);
+    font-size: var(--text-small);
+    font-weight: 600;
+    vertical-align: middle;
+  }
+
+  .badge[data-tone="warn"],
+  .tag[data-tone="warn"] {
+    background: var(--warn-bg);
+    color: var(--warn-text);
+  }
+
+  .badge[data-tone="muted"],
+  .tag[data-tone="muted"] {
+    background: var(--bg);
+    color: var(--muted);
+  }
+}
diff --git a/crates/web/css/57-row.css b/crates/web/css/57-row.css
new file mode 100644
index 0000000..0a9f4b5
--- /dev/null
+++ b/crates/web/css/57-row.css
@@ -0,0 +1,36 @@
+/* .row: one line of a list card, divided from the next by a hairline. */
+@layer components {
+  .row {
+    display: flex;
+    align-items: center;
+    gap: var(--space-3);
+    min-block-size: 3.5rem;
+    padding-block: var(--space-2);
+    color: inherit;
+    text-decoration: none;
+  }
+
+  .row + .row {
+    border-block-start: 1px solid var(--line);
+  }
+
+  .row-text {
+    display: flex;
+    flex-direction: column;
+    flex-grow: 1;
+    min-inline-size: 0;
+  }
+
+  .row-text > strong {
+    font-weight: 600;
+  }
+
+  .row-text > span {
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+
+  .row > .icon:last-child {
+    color: var(--muted);
+  }
+}
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index b9efeaa..4784a3f 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -44,6 +44,14 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "20-tokens.css",
     "30-base.css",
     "40-layout.css",
+    "50-icon.css",
+    "51-card.css",
+    "52-button.css",
+    "53-chip.css",
+    "54-field.css",
+    "55-notice.css",
+    "56-badge.css",
+    "57-row.css",
     "90-utilities.css",
 );
 
@@ -62,6 +70,7 @@ const ASSETS: &[Asset] = &[
     asset!("icon-192.png", "image/png"),
     asset!("icon-512.png", "image/png"),
     asset!("icon.svg", "image/svg+xml"),
+    asset!("icons.svg", "image/svg+xml"),
     asset!("manifest.webmanifest", "application/manifest+json"),
     asset!("sw.js", "text/javascript; charset=utf-8"),
 ];
diff --git a/crates/web/static/icons.svg b/crates/web/static/icons.svg
new file mode 100644
index 0000000..6d98b1d
--- /dev/null
+++ b/crates/web/static/icons.svg
@@ -0,0 +1,19 @@
+<svg xmlns="http://www.w3.org/2000/svg">
+  <symbol id="log" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 5v14M5 12h14"/></symbol>
+  <symbol id="events" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 6h12M8 12h12M8 18h12M4 6h.01M4 12h.01M4 18h.01"/></symbol>
+  <symbol id="graphs" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/></symbol>
+  <symbol id="people" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 20v-1a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v1M9 11a4 4 0 1 0 0-8 4 4 0 0 0 0 8M22 20v-1a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8"/></symbol>
+  <symbol id="settings" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 7h10M18 7h2M4 17h4M12 17h8M14 4v6M8 14v6"/></symbol>
+  <symbol id="chevron-down" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6"/></symbol>
+  <symbol id="chevron-left" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 6l-6 6 6 6"/></symbol>
+  <symbol id="chevron-right" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 6l6 6-6 6"/></symbol>
+  <symbol id="info" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="9"/><path d="M12 11v6M12 7.5v.5"/></symbol>
+  <symbol id="check" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></symbol>
+  <symbol id="close" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6L6 18"/></symbol>
+  <symbol id="logout" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 4h4v16h-4M10 8l-4 4 4 4M6 12h10"/></symbol>
+  <symbol id="filter" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6h16M7 12h10M10 18h4"/></symbol>
+  <symbol id="search" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="6"/><path d="M20 20l-4.5-4.5"/></symbol>
+  <symbol id="install" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4v11M7 10l5 5 5-5M5 20h14"/></symbol>
+  <symbol id="share" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 15V3m-4 4 4-4 4 4M6 10H5v11h14V10h-1"/></symbol>
+  <symbol id="add-square" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><rect x="4" y="4" width="16" height="16" rx="3"/><path d="M12 8v8m-4-4h8"/></symbol>
+</svg>
diff --git a/crates/web/templates/components.html b/crates/web/templates/components.html
new file mode 100644
index 0000000..d08eade
--- /dev/null
+++ b/crates/web/templates/components.html
@@ -0,0 +1,69 @@
+{# Every repeated piece of markup, as Askama macros (look-and-feel spec §9). Import with
+   {% import "components.html" as c %}. Macros take translated text, never the translator. #}
+
+{# A decorative icon from the sprite; the text beside it, or the control's aria-label, names it. #}
+{% macro icon(name) %}<svg class="icon" aria-hidden="true" focusable="false"><use href="/static/icons.svg#{{ name }}"></use></svg>{% endmacro %}
+
+{# A card with an optional heading; the body is the call block. #}
+{% macro card(title) %}
+<section class="card">
+  {% if !title.is_empty() %}<h2>{{ title }}</h2>{% endif %}
+  {{ caller() }}
+</section>
+{% endmacro %}
+
+{# A card that opens and closes: the title, a muted hint and an accent action ("Change", "Add").
+   `open` keeps it open when the form comes back with a value in it. #}
+{% macro collapsible(title, hint, action, open) %}
+<details class="card collapsible"{% if open %} open{% endif %}>
+  <summary>
+    <span><strong>{{ title }}</strong>{% if !hint.is_empty() %} <span class="muted">· {{ hint }}</span>{% endif %}</span>
+    <span class="action">{{ action }}{% call icon("chevron-down") %}{% endcall %}</span>
+  </summary>
+  <div class="stack">{{ caller() }}</div>
+</details>
+{% endmacro %}
+
+{# A checkbox or radio drawn as a pill; the input stays native, visually hidden. #}
+{% macro chip(kind, name, value, label, checked) %}
+<label class="chip"><input type="{{ kind }}" name="{{ name }}" value="{{ value }}"{% if checked %} checked{% endif %}><span>{{ label }}</span></label>
+{% endmacro %}
+
+{# A label above its control; the control is the call block. #}
+{% macro field(label) %}
+<label class="field"><span>{{ label }}</span>{{ caller() }}</label>
+{% endmacro %}
+
+{# A one-button form that posts to `action`. #}
+{% macro post_button(action, label, variant) %}
+<form method="post" action="{{ action }}"><button type="submit" class="button" data-variant="{{ variant }}">{{ label }}</button></form>
+{% endmacro %}
+
+{# A message above a form: `alert` for what went wrong, otherwise a confirmation. #}
+{% macro notice(text, alert) %}
+<p class="notice" role="{% if alert %}alert{% else %}status{% endif %}">{{ text }}</p>
+{% endmacro %}
+
+{# A line of a list card, linked when `href` is set. The call block is what leads the line,
+   such as a badge; it may be empty. #}
+{% macro row(href, title, detail) %}
+{% if href.is_empty() %}<div class="row">{% else %}<a class="row" href="{{ href }}">{% endif %}
+  {{ caller() }}
+  <span class="row-text"><strong>{{ title }}</strong>{% if !detail.is_empty() %}<span>{{ detail }}</span>{% endif %}</span>
+{% if href.is_empty() %}</div>{% else %}{% call icon("chevron-right") %}{% endcall %}</a>{% endif %}
+{% endmacro %}
+
+{# The link back to the page above, e.g. "‹ Events". #}
+{% macro back(href, label) %}
+<a class="back" href="{{ href }}">{% call icon("chevron-left") %}{% endcall %}{{ label }}</a>
+{% endmacro %}
+
+{# The three languages as buttons that post to /lang; `return_to` names the page to come back to. #}
+{% macro languages(choices, label, return_to) %}
+<form method="post" action="/lang" class="segmented" aria-label="{{ label }}">
+  {% if !return_to.is_empty() %}<input type="hidden" name="return_to" value="{{ return_to }}">{% endif %}
+  {% for language in choices %}
+  <button type="submit" name="lang" value="{{ language.value }}" lang="{{ language.value }}"{% if language.current %} aria-current="true"{% endif %}>{{ language.name }}</button>
+  {% endfor %}
+</form>
+{% endmacro %}
diff --git a/crates/web/tests/common/mod.rs b/crates/web/tests/common/mod.rs
index 4e6e5b3..44e28a8 100644
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -2,6 +2,8 @@
 
 #![allow(dead_code)]
 
+pub mod page;
+
 use std::net::{IpAddr, Ipv4Addr, SocketAddr};
 use std::sync::Arc;
 
diff --git a/crates/web/tests/common/page.rs b/crates/web/tests/common/page.rs
new file mode 100644
index 0000000..2587f18
--- /dev/null
+++ b/crates/web/tests/common/page.rs
@@ -0,0 +1,61 @@
+//! A rendered page, queried with CSS selectors, so tests assert what the page means rather than
+//! how its markup happens to be spelled.
+
+use scraper::{ElementRef, Html, Selector};
+
+pub struct Page(Html);
+
+fn selector(css: &str) -> Selector {
+    Selector::parse(css).unwrap_or_else(|error| panic!("bad selector {css}: {error}"))
+}
+
+/// An element's text with runs of whitespace collapsed to one space.
+fn text_of(element: ElementRef) -> String {
+    element
+        .text()
+        .collect::<Vec<_>>()
+        .join(" ")
+        .split_whitespace()
+        .collect::<Vec<_>>()
+        .join(" ")
+}
+
+impl Page {
+    pub fn parse(html: &str) -> Self {
+        Self(Html::parse_document(html))
+    }
+
+    pub fn exists(&self, css: &str) -> bool {
+        self.0.select(&selector(css)).next().is_some()
+    }
+
+    pub fn count(&self, css: &str) -> usize {
+        self.0.select(&selector(css)).count()
+    }
+
+    /// The text of every match, in document order.
+    pub fn texts(&self, css: &str) -> Vec<String> {
+        self.0.select(&selector(css)).map(text_of).collect()
+    }
+
+    /// The text of the first match; panics when nothing matches.
+    pub fn text(&self, css: &str) -> String {
+        self.texts(css)
+            .into_iter()
+            .next()
+            .unwrap_or_else(|| panic!("nothing matches {css}"))
+    }
+
+    /// The attribute `name` of every match that has it.
+    pub fn attrs(&self, css: &str, name: &str) -> Vec<String> {
+        self.0
+            .select(&selector(css))
+            .filter_map(|element| element.value().attr(name).map(str::to_owned))
+            .collect()
+    }
+
+    /// The attribute `name` of the first match.
+    pub fn attr(&self, css: &str, name: &str) -> Option<String> {
+        self.attrs(css, name).into_iter().next()
+    }
+}
diff --git a/crates/web/tests/components.rs b/crates/web/tests/components.rs
new file mode 100644
index 0000000..a9b68c7
--- /dev/null
+++ b/crates/web/tests/components.rs
@@ -0,0 +1,135 @@
+//! The Askama macros of `templates/components.html`, rendered on their own.
+
+mod common;
+
+use askama::Template;
+use common::page::Page;
+
+struct Language {
+    value: &'static str,
+    name: &'static str,
+    current: bool,
+}
+
+#[derive(Template)]
+#[template(
+    ext = "html",
+    source = r#"{% import "components.html" as c %}
+{% call c::card("Rooms") %}<p>body</p>{% endcall %}
+{% call c::collapsible("Time", "now", "Change", open) %}<input name="observed_at">{% endcall %}
+{% call c::chip("checkbox", "symptom", 3, "Cough", true) %}{% endcall %}
+{% call c::chip("radio", "area", "inside", "Inside", false) %}{% endcall %}
+{% call c::field("Username") %}<input name="username">{% endcall %}
+{% call c::post_button("/events/1/reopen", "Reopen", "secondary") %}{% endcall %}
+{% call c::notice("Saved.", false) %}{% endcall %}
+{% call c::notice("Pick a room.", true) %}{% endcall %}
+{% call c::row("/events/4", "21:40 – in progress", "3 reports") %}<span class="badge">6</span>{% endcall %}
+{% call c::row("", "21:00", "") %}{% endcall %}
+{% call c::back("/events", "Events") %}{% endcall %}
+{% call c::languages(languages, "Language", "settings") %}{% endcall %}"#
+)]
+struct Gallery {
+    open: bool,
+    languages: Vec<Language>,
+}
+
+fn gallery(open: bool) -> Page {
+    let languages = vec![
+        Language {
+            value: "en",
+            name: "English",
+            current: true,
+        },
+        Language {
+            value: "it",
+            name: "Italiano",
+            current: false,
+        },
+    ];
+    Page::parse(&Gallery { open, languages }.render().unwrap())
+}
+
+#[test]
+fn a_card_has_its_title_and_body() {
+    let page = gallery(false);
+    assert_eq!(page.text("section.card > h2"), "Rooms");
+    assert!(page.exists("section.card > p"));
+}
+
+#[test]
+fn a_collapsible_card_is_a_details_element_open_on_request() {
+    assert!(gallery(false).exists("details.collapsible:not([open]) input[name=observed_at]"));
+    let open = gallery(true);
+    assert!(open.exists("details.collapsible[open]"));
+    assert_eq!(
+        open.text("details.collapsible > summary"),
+        "Time · now Change"
+    );
+    assert!(open.exists("summary .action use[href='/static/icons.svg#chevron-down']"));
+}
+
+#[test]
+fn chips_keep_the_native_input() {
+    let page = gallery(false);
+    assert!(page.exists("label.chip > input[type=checkbox][name=symptom][value='3'][checked]"));
+    assert!(page.exists("label.chip > input[type=radio][name=area][value=inside]:not([checked])"));
+    assert_eq!(page.texts("label.chip > span"), ["Cough", "Inside"]);
+}
+
+#[test]
+fn fields_buttons_and_notices() {
+    let page = gallery(false);
+    assert_eq!(page.text("label.field > span"), "Username");
+    assert!(page.exists("label.field > input[name=username]"));
+    assert_eq!(
+        page.text(
+            "form[method=post][action='/events/1/reopen'] > button.button[data-variant=secondary]"
+        ),
+        "Reopen"
+    );
+    assert_eq!(page.text("p.notice[role=status]"), "Saved.");
+    assert_eq!(page.text("p.notice[role=alert]"), "Pick a room.");
+}
+
+#[test]
+fn rows_link_with_a_chevron_only_when_they_have_an_address() {
+    let page = gallery(false);
+    assert_eq!(
+        page.text("a.row[href='/events/4'] .row-text > strong"),
+        "21:40 – in progress"
+    );
+    assert_eq!(page.text("a.row .row-text > span"), "3 reports");
+    assert!(page.exists("a.row > .badge + .row-text + svg.icon"));
+    assert!(page.exists("div.row .row-text > strong"));
+    assert!(!page.exists("div.row svg"));
+    assert!(!page.exists("div.row .row-text > span"));
+}
+
+#[test]
+fn the_back_link_and_the_language_buttons() {
+    let page = gallery(false);
+    assert_eq!(page.text("a.back[href='/events']"), "Events");
+    assert!(page.exists("form.segmented[action='/lang'] > input[name=return_to][value=settings]"));
+    assert_eq!(
+        page.attr("button[name=lang][aria-current=true]", "value")
+            .as_deref(),
+        Some("en")
+    );
+    assert_eq!(page.count("button[name=lang]"), 2);
+}
+
+#[test]
+fn icons_are_decorative_and_exist_in_the_sprite() {
+    let sprite = include_str!("../static/icons.svg");
+    let page = gallery(true);
+    let icons = page.attrs("svg.icon > use", "href");
+    assert!(!icons.is_empty());
+    for href in icons {
+        let id = href.strip_prefix("/static/icons.svg#").expect("the sprite");
+        assert!(sprite.contains(&format!("<symbol id=\"{id}\"")), "{id}");
+    }
+    assert_eq!(
+        page.count("svg.icon"),
+        page.count("svg.icon[aria-hidden=true][focusable=false]")
+    );
+}
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index e505c79..e38dba3 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -273,6 +273,39 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
     }
 }
 
+/// Every template's source, by file name.
+fn templates() -> Vec<(String, String)> {
+    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/templates");
+    std::fs::read_dir(dir)
+        .unwrap()
+        .map(|entry| {
+            let path = entry.unwrap().path();
+            let name = path.file_name().unwrap().to_string_lossy().into_owned();
+            (name, std::fs::read_to_string(&path).unwrap())
+        })
+        .collect()
+}
+
+#[test]
+fn no_template_has_inline_styles_or_scripts() {
+    let templates = templates();
+    assert!(templates.iter().any(|(name, _)| name == "base.html"));
+    for (name, source) in templates {
+        for forbidden in ["<style", " style=", "javascript:", " hx-on"] {
+            assert!(!source.contains(forbidden), "{forbidden} in {name}");
+        }
+        assert_eq!(event_handler_attribute(&source), None, "{name}");
+        for script in source.match_indices("<script").map(|(i, _)| &source[i..]) {
+            let tag = &script[..script.find('>').unwrap()];
+            assert!(tag.contains(" src=\"/static/"), "{name}: {tag}");
+            assert!(
+                script[tag.len()..].starts_with("></script>"),
+                "{name}: a script body"
+            );
+        }
+    }
+}
+
 #[tokio::test]
 async fn oversized_form_posts_are_refused() {
     let app = TestApp::new().await;
````

`Cargo.lock` changes come from `cargo fetch`/`cargo build`; do not edit it by hand.

### Task 3: JavaScript as ES modules, one file per behaviour

**Proto commit:** `1cac53a` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 1cac53a`)

Spec §9 (JavaScript). `static/app.js` splits into `static/js/{app,dialog,date-picks,install,service-worker}.js`; `graphs.js` moves to `static/js/`; `base.html` loads `<script src="/static/js/app.js" type="module">` (src first: the CSP test checks the prefix). The expiry script becomes the generic `[data-date-picks]` behaviour. `sw.js` precaches the modules (cache `smoke-tracker-v2`); `the_service_worker_keeps_the_entry_module_and_its_imports` keeps them in step; the static-folder test walks subfolders.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/static/js/app.js
A	crates/web/static/js/date-picks.js
A	crates/web/static/js/dialog.js
A	crates/web/static/js/install.js
A	crates/web/static/js/service-worker.js
D	crates/web/static/app.js
M	crates/web/src/assets.rs
M	crates/web/src/routes/graphs.rs
M	crates/web/static/sw.js
M	crates/web/templates/base.html
M	crates/web/templates/expiry_fields.html
M	crates/web/templates/graphs.html
M	crates/web/tests/log.rs
R100	crates/web/static/graphs.js	crates/web/static/js/graphs.js
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/src/assets.rs (tests)`, `crates/web/tests/log.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **607 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
refactor(web): JavaScript as ES modules, one file per behaviour

static/js/app.js is the module entry of every page and imports one small module per
behaviour (dialog, date picks, install prompt, service worker); graphs.js moves next to them.
The expiry fields' script becomes the generic data-date-picks behaviour. The service worker
precaches the modules, and a test keeps its list in step with the entry's imports; the
static-folder test walks subfolders.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `1cac53a`):

````diff
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 4784a3f..0c99b5e 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -61,16 +61,20 @@ const ASSETS: &[Asset] = &[
         content_type: "text/css; charset=utf-8",
         body: STYLESHEET.0.as_bytes(),
     },
-    asset!("app.js", "text/javascript; charset=utf-8"),
     asset!("chart-4.5.1.umd.min.js", "text/javascript; charset=utf-8"),
     asset!("favicon.svg", "image/svg+xml"),
-    asset!("graphs.js", "text/javascript; charset=utf-8"),
     asset!("htmx-2.0.11.min.js", "text/javascript; charset=utf-8"),
     asset!("icon-180.png", "image/png"),
     asset!("icon-192.png", "image/png"),
     asset!("icon-512.png", "image/png"),
     asset!("icon.svg", "image/svg+xml"),
     asset!("icons.svg", "image/svg+xml"),
+    asset!("js/app.js", "text/javascript; charset=utf-8"),
+    asset!("js/date-picks.js", "text/javascript; charset=utf-8"),
+    asset!("js/dialog.js", "text/javascript; charset=utf-8"),
+    asset!("js/graphs.js", "text/javascript; charset=utf-8"),
+    asset!("js/install.js", "text/javascript; charset=utf-8"),
+    asset!("js/service-worker.js", "text/javascript; charset=utf-8"),
     asset!("manifest.webmanifest", "application/manifest+json"),
     asset!("sw.js", "text/javascript; charset=utf-8"),
 ];
@@ -126,15 +130,24 @@ mod tests {
     const CHART_JS_SHA256: &str =
         "48444a82d4edcb5bec0f1965faacdde18d9c17db3063d042abada2f705c9f54a";
 
-    /// The names of the files in `dir` (relative to the crate), sorted.
+    /// The paths of the files under `dir` (relative to the crate), relative to `dir`, sorted.
     fn files_in(dir: &str) -> Vec<String> {
-        let path = format!("{}/{dir}", env!("CARGO_MANIFEST_DIR"));
-        let mut names: Vec<String> = std::fs::read_dir(path)
-            .unwrap()
-            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
-            .collect();
-        names.sort();
-        names
+        fn walk(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<String>) {
+            for entry in std::fs::read_dir(dir).unwrap() {
+                let path = entry.unwrap().path();
+                if path.is_dir() {
+                    walk(root, &path, out);
+                } else {
+                    let relative = path.strip_prefix(root).unwrap();
+                    out.push(relative.to_str().unwrap().replace('\\', "/"));
+                }
+            }
+        }
+        let root = std::path::PathBuf::from(format!("{}/{dir}", env!("CARGO_MANIFEST_DIR")));
+        let mut paths = Vec::new();
+        walk(&root, &root, &mut paths);
+        paths.sort();
+        paths
     }
 
     #[test]
@@ -148,6 +161,37 @@ mod tests {
         assert_eq!(on_disk, listed);
     }
 
+    /// The modules `js/app.js` imports, as `/static/js/…` paths.
+    fn entry_imports() -> Vec<String> {
+        let entry = std::str::from_utf8(asset("js/app.js").body).unwrap();
+        entry
+            .lines()
+            .filter_map(|line| line.strip_prefix("import \"./"))
+            .map(|rest| format!("/static/js/{}", rest.trim_end_matches("\";")))
+            .collect()
+    }
+
+    #[test]
+    fn the_service_worker_keeps_the_entry_module_and_its_imports() {
+        let worker = std::str::from_utf8(asset("sw.js").body).unwrap();
+        let imports = entry_imports();
+        assert!(imports.len() >= 4, "{imports:?}");
+        for path in imports
+            .iter()
+            .map(String::as_str)
+            .chain(["/static/js/app.js"])
+        {
+            assert!(
+                ASSETS.iter().any(|a| format!("/static/{}", a.path) == path),
+                "{path}"
+            );
+            assert!(
+                worker.contains(&format!("\"{path}\"")),
+                "sw.js lacks {path}"
+            );
+        }
+    }
+
     #[test]
     fn the_stylesheet_joins_every_css_file_in_name_order() {
         assert_eq!(files_in("css"), STYLESHEET.1);
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
index 47df3f8..c5b61ab 100644
--- a/crates/web/src/routes/graphs.rs
+++ b/crates/web/src/routes/graphs.rs
@@ -1,5 +1,5 @@
 //! The Graphs screen (spec §9 screen 5). Each chart is a `<canvas>` whose Chart.js
-//! configuration travels as JSON in its `data-chart` attribute; `static/graphs.js` draws it, so
+//! configuration travels as JSON in its `data-chart` attribute; `static/js/graphs.js` draws it, so
 //! no page carries an inline script. The room × hour heatmap is a plain table, coloured by CSS
 //! classes. The numbers come from `smoke_domain::graphs`, over what the viewer may see: guests
 //! get graphs of their own data only (spec §4).
diff --git a/crates/web/static/app.js b/crates/web/static/app.js
deleted file mode 100644
index 7582c09..0000000
--- a/crates/web/static/app.js
+++ /dev/null
@@ -1,70 +0,0 @@
-// Progressive enhancement, loaded by every page.
-//
-// - Links with a data-dialog attribute open that <dialog> instead of navigating (without
-//   JavaScript they open the full page), and tapping a dialog's backdrop closes it. The close
-//   button is a method="dialog" form and needs no script.
-// - In the expiry fields, typing a date picks "at the end of this day", and picking any other
-//   option clears the date.
-// - The service worker is registered (spec §9.1).
-// - Where the browser offers its own install prompt (Chromium, Android), every data-install
-//   element opens it; a hidden install button is shown. Elsewhere the Install link leads to the
-//   page with the steps (spec §8.3).
-let installPrompt = null;
-
-document.addEventListener("click", (event) => {
-  const installer = event.target.closest("[data-install]");
-  if (installer && installPrompt) {
-    event.preventDefault();
-    installPrompt.prompt();
-    installPrompt = null;
-    return;
-  }
-  const opener = event.target.closest("[data-dialog]");
-  if (opener) {
-    const dialog = document.getElementById(opener.dataset.dialog);
-    if (dialog instanceof HTMLDialogElement) {
-      event.preventDefault();
-      dialog.showModal();
-    }
-    return;
-  }
-  if (event.target instanceof HTMLDialogElement) {
-    event.target.close();
-  }
-});
-
-document.addEventListener("input", (event) => {
-  const field = event.target;
-  if (field instanceof HTMLInputElement && field.type === "date" && field.value) {
-    const radio = field.closest(".expiry")?.querySelector('input[type="radio"][value="date"]');
-    if (radio) radio.checked = true;
-  }
-});
-
-document.addEventListener("change", (event) => {
-  const radio = event.target;
-  if (radio instanceof HTMLInputElement && radio.type === "radio" && radio.value !== "date") {
-    const fieldset = radio.closest(".expiry");
-    const date = fieldset?.querySelector('input[type="date"]');
-    if (date) date.value = "";
-  }
-});
-
-window.addEventListener("beforeinstallprompt", (event) => {
-  event.preventDefault();
-  installPrompt = event;
-  for (const button of document.querySelectorAll("button[data-install]")) {
-    button.hidden = false;
-  }
-});
-
-window.addEventListener("appinstalled", () => {
-  installPrompt = null;
-  for (const element of document.querySelectorAll("[data-install]")) {
-    element.hidden = true;
-  }
-});
-
-if ("serviceWorker" in navigator) {
-  navigator.serviceWorker.register("/sw.js");
-}
diff --git a/crates/web/static/js/app.js b/crates/web/static/js/app.js
new file mode 100644
index 0000000..cd59d41
--- /dev/null
+++ b/crates/web/static/js/app.js
@@ -0,0 +1,6 @@
+// The entry module of every page. Each import is one progressive enhancement; every page works
+// without them.
+import "./dialog.js";
+import "./date-picks.js";
+import "./install.js";
+import "./service-worker.js";
diff --git a/crates/web/static/js/date-picks.js b/crates/web/static/js/date-picks.js
new file mode 100644
index 0000000..a2dfb6b
--- /dev/null
+++ b/crates/web/static/js/date-picks.js
@@ -0,0 +1,24 @@
+// A fieldset with data-date-picks offers quick picks (radio buttons) and date fields. Typing a
+// date checks the radio whose value is "date", or clears the picks when there is none; choosing
+// a pick clears the dates, so the form never sends both. Without JavaScript the server decides.
+document.addEventListener("input", (event) => {
+  const field = event.target;
+  const picks = field.closest?.("[data-date-picks]");
+  if (!picks || !(field instanceof HTMLInputElement) || field.type !== "date" || !field.value) {
+    return;
+  }
+  const dated = picks.querySelector('input[type="radio"][value="date"]');
+  for (const radio of picks.querySelectorAll('input[type="radio"]')) {
+    radio.checked = radio === dated;
+  }
+});
+
+document.addEventListener("change", (event) => {
+  const radio = event.target;
+  const picks = radio.closest?.("[data-date-picks]");
+  if (picks && radio instanceof HTMLInputElement && radio.type === "radio" && radio.value !== "date") {
+    for (const date of picks.querySelectorAll('input[type="date"]')) {
+      date.value = "";
+    }
+  }
+});
diff --git a/crates/web/static/js/dialog.js b/crates/web/static/js/dialog.js
new file mode 100644
index 0000000..5c824cb
--- /dev/null
+++ b/crates/web/static/js/dialog.js
@@ -0,0 +1,16 @@
+// A link with data-dialog="<id>" opens that <dialog> in place of the page it leads to, and a tap
+// on the backdrop closes it. The close button is a method="dialog" form and needs no script.
+document.addEventListener("click", (event) => {
+  const opener = event.target.closest("[data-dialog]");
+  if (opener) {
+    const dialog = document.getElementById(opener.dataset.dialog);
+    if (dialog instanceof HTMLDialogElement) {
+      event.preventDefault();
+      dialog.showModal();
+    }
+    return;
+  }
+  if (event.target instanceof HTMLDialogElement) {
+    event.target.close();
+  }
+});
diff --git a/crates/web/static/graphs.js b/crates/web/static/js/graphs.js
similarity index 100%
rename from crates/web/static/graphs.js
rename to crates/web/static/js/graphs.js
diff --git a/crates/web/static/js/install.js b/crates/web/static/js/install.js
new file mode 100644
index 0000000..de323f1
--- /dev/null
+++ b/crates/web/static/js/install.js
@@ -0,0 +1,28 @@
+// Where the browser has its own install prompt (Chromium, Android), every data-install element
+// opens it and hidden install buttons appear. Elsewhere data-install links lead to /install, the
+// page with the steps (spec §8.3).
+let installPrompt = null;
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
+document.addEventListener("click", (event) => {
+  const installer = event.target.closest("[data-install]");
+  if (installer && installPrompt) {
+    event.preventDefault();
+    installPrompt.prompt();
+    installPrompt = null;
+  }
+});
diff --git a/crates/web/static/js/service-worker.js b/crates/web/static/js/service-worker.js
new file mode 100644
index 0000000..6e30747
--- /dev/null
+++ b/crates/web/static/js/service-worker.js
@@ -0,0 +1,4 @@
+// Registers the service worker (spec §9.1), which keeps the static files and an offline page.
+if ("serviceWorker" in navigator) {
+  navigator.serviceWorker.register("/sw.js");
+}
diff --git a/crates/web/static/sw.js b/crates/web/static/sw.js
index 12d71eb..f6b6469 100644
--- a/crates/web/static/sw.js
+++ b/crates/web/static/sw.js
@@ -2,11 +2,16 @@
 // static assets and an offline page. Pages and form posts always go to the network: there is
 // no offline logging. Assets are fetched first and the cached copy is the fallback, so a new
 // deploy is never hidden behind an old cache.
-const CACHE = "smoke-tracker-v1";
+const CACHE = "smoke-tracker-v2";
 const PRECACHE = [
   "/offline",
   "/static/app.css",
-  "/static/app.js",
+  "/static/icons.svg",
+  "/static/js/app.js",
+  "/static/js/date-picks.js",
+  "/static/js/dialog.js",
+  "/static/js/install.js",
+  "/static/js/service-worker.js",
   "/static/htmx-2.0.11.min.js",
   "/static/favicon.svg",
   "/static/icon-192.png",
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index d1f359e..3d0380b 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -11,7 +11,7 @@
   <link rel="manifest" href="/static/manifest.webmanifest">
   <link rel="stylesheet" href="/static/app.css">
   <script src="/static/htmx-2.0.11.min.js" defer></script>
-  <script src="/static/app.js" defer></script>
+  <script src="/static/js/app.js" type="module"></script>
   {% block scripts %}{% endblock %}
 </head>
 <body>
diff --git a/crates/web/templates/expiry_fields.html b/crates/web/templates/expiry_fields.html
index 7f83334..a086b5e 100644
--- a/crates/web/templates/expiry_fields.html
+++ b/crates/web/templates/expiry_fields.html
@@ -1,5 +1,5 @@
 {# When an account stops working: quick picks or a day (spec §8.3). Expects `expiry`. #}
-<fieldset class="expiry choices" data-role="{{ expiry.role }}">
+<fieldset class="expiry choices" data-date-picks data-role="{{ expiry.role }}">
   <legend>{{ tr.t("expiry-legend") }}</legend>
   {% for pick in expiry.picks %}
   <label class="chip"><input type="radio" name="{{ expiry.pick_name }}" value="{{ pick.value }}"{% if pick.checked %} checked{% endif %}><span>{{ tr.t(pick.key) }}</span></label>
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
index 205a1f9..8dc6292 100644
--- a/crates/web/templates/graphs.html
+++ b/crates/web/templates/graphs.html
@@ -2,7 +2,7 @@
 {% block title %}{{ tr.t("graphs-title") }}{% endblock %}
 {% block scripts %}
   <script src="/static/chart-4.5.1.umd.min.js" defer></script>
-  <script src="/static/graphs.js" defer></script>
+  <script src="/static/js/graphs.js" type="module"></script>
 {% endblock %}
 {% block content %}
 <h1>{{ tr.t("graphs-title") }}</h1>
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index e2f30b5..68e7e33 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::logbook::EventChoice;
 use smoke_domain::repos::{Commit, EventRepo, ListWrites, Transactional, UserWrites};
@@ -271,7 +272,10 @@ async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
         html.contains("Tra due numeri, scegli il più basso."),
         "{html}"
     );
-    assert!(html.contains(r#"<script src="/static/app.js" defer></script>"#));
+    assert!(
+        Page::parse(&html).exists(r#"script[type=module][src="/static/js/app.js"]"#),
+        "{html}"
+    );
 
     let guide = app.send(get_with("/guide/rating", &cookie)).await;
     assert_eq!(guide.status(), StatusCode::OK);
@@ -285,7 +289,7 @@ async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
         guide.contains("Come una stanza piena di fumo. Devi uscire."),
         "{guide}"
     );
-    let script = app.send(get("/static/app.js")).await;
+    let script = app.send(get("/static/js/dialog.js")).await;
     assert_eq!(script.status(), StatusCode::OK);
     assert!(text(script).await.contains("showModal()"));
     assert_eq!(
````

### Task 4: App shell: avatar menu, bottom tab bar, Install in Settings

**Proto commit:** `f151ec5` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show f151ec5`)

Spec §3.3, §6.5. `header::Tab` and `SignedIn::of(user, now, tab)` (every call site names its tab), `SignedIn::tabs(&tr)`; `Tr::languages()` replaces the two copies of the language list (settings, accept); macros `account_menu` and `tabbar`; `js/menu.js`; CSS 60–63; the Install card at the top of Settings; signed-out pages end with the language buttons. New keys: `account-menu`, `nav-label`, `install-card-hint`. Tests: `tests/shell.rs`, `settings_offers_the_install_card_instead_of_the_header`, updated people/settings/pwa/graphs assertions, and the all-pages security test now checks every `<use href>` exists in the sprite.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/60-appbar.css
A	crates/web/css/61-menu.css
A	crates/web/css/62-tabbar.css
A	crates/web/css/63-install.css
A	crates/web/static/js/menu.js
A	crates/web/tests/shell.rs
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/assets.rs
M	crates/web/src/header.rs
M	crates/web/src/i18n.rs
M	crates/web/src/routes/accept.rs
M	crates/web/src/routes/entries.rs
M	crates/web/src/routes/events.rs
M	crates/web/src/routes/graphs.rs
M	crates/web/src/routes/guide.rs
M	crates/web/src/routes/install.rs
M	crates/web/src/routes/log.rs
M	crates/web/src/routes/people/links.rs
M	crates/web/src/routes/people/mod.rs
M	crates/web/src/routes/reports.rs
M	crates/web/src/routes/settings.rs
M	crates/web/static/js/app.js
M	crates/web/static/sw.js
M	crates/web/templates/accept.html
M	crates/web/templates/base.html
M	crates/web/templates/components.html
M	crates/web/templates/settings.html
M	crates/web/tests/graphs.rs
M	crates/web/tests/people.rs
M	crates/web/tests/pwa.rs
M	crates/web/tests/security.rs
M	crates/web/tests/settings.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/shell.rs`, `crates/web/tests/pwa.rs`, `crates/web/tests/security.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **611 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(web): app shell with an avatar menu, a bottom tab bar and Install in Settings

base.html draws the header (app name, avatar) and, when signed in, the bottom tab bar from
SignedIn::tabs (Tab enum: link, icon, label; People for residents only; the current screen
marked aria-current). The avatar menu is a <details> with who is signed in and Sign out;
js/menu.js closes it on an outside tap or Escape. Signed-out pages end with the language
buttons. The header's Install link becomes the Install card at the top of Settings (amends
decision #121). Tr::languages replaces the two copies of the language list.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `f151ec5`):

````diff
diff --git a/crates/web/css/60-appbar.css b/crates/web/css/60-appbar.css
new file mode 100644
index 0000000..d743739
--- /dev/null
+++ b/crates/web/css/60-appbar.css
@@ -0,0 +1,34 @@
+/* The header (app name and avatar) and, on signed-out pages, the footer with the languages. */
+@layer components {
+  .appbar {
+    display: flex;
+    align-items: center;
+    justify-content: space-between;
+    inline-size: 100%;
+    max-inline-size: var(--column);
+    margin-inline: auto;
+    padding: calc(var(--space-4) + env(safe-area-inset-top)) var(--gutter) var(--space-1);
+  }
+
+  .brand {
+    font-size: var(--text-brand);
+    font-weight: 750;
+    letter-spacing: -0.01em;
+  }
+
+  .footer {
+    inline-size: 100%;
+    max-inline-size: var(--column);
+    margin-inline: auto;
+    padding: var(--space-4) var(--gutter) calc(var(--space-5) + env(safe-area-inset-bottom));
+  }
+
+  .footer .segmented {
+    border: 0;
+    background: transparent;
+  }
+
+  .footer .segmented > [aria-current="true"] {
+    box-shadow: none;
+  }
+}
diff --git a/crates/web/css/61-menu.css b/crates/web/css/61-menu.css
new file mode 100644
index 0000000..9e15034
--- /dev/null
+++ b/crates/web/css/61-menu.css
@@ -0,0 +1,57 @@
+/* .menu: the avatar and the small panel it opens (a <details>). */
+@layer components {
+  .menu {
+    position: relative;
+  }
+
+  .avatar {
+    display: grid;
+    place-items: center;
+    inline-size: var(--target);
+    block-size: var(--target);
+    border-radius: var(--radius-pill);
+    background: var(--accent-soft);
+    color: var(--accent);
+    font-size: var(--text-card);
+    font-weight: 700;
+    list-style: none;
+    cursor: pointer;
+  }
+
+  .avatar::-webkit-details-marker {
+    display: none;
+  }
+
+  .menu-panel {
+    position: absolute;
+    inset-inline-end: 0;
+    inset-block-start: calc(var(--target) + var(--space-2));
+    z-index: 2;
+    inline-size: 14rem;
+    padding: var(--space-2);
+    border: 1px solid var(--line);
+    border-radius: var(--radius-button);
+    background: var(--card);
+    box-shadow: var(--shadow);
+  }
+
+  .menu-panel > p {
+    padding: var(--space-2) var(--space-2) var(--space-3);
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+
+  .menu-item {
+    display: flex;
+    align-items: center;
+    gap: var(--space-2);
+    inline-size: 100%;
+    min-block-size: var(--target);
+    padding-inline: var(--space-2);
+    border: 0;
+    border-radius: var(--radius-field);
+    background: transparent;
+    font-weight: 600;
+    text-align: start;
+  }
+}
diff --git a/crates/web/css/62-tabbar.css b/crates/web/css/62-tabbar.css
new file mode 100644
index 0000000..2da3a42
--- /dev/null
+++ b/crates/web/css/62-tabbar.css
@@ -0,0 +1,38 @@
+/* .tabbar: the bottom navigation, fixed, clear of the home indicator (safe area). */
+@layer components {
+  .tabbar {
+    position: fixed;
+    inset-inline: 0;
+    inset-block-end: 0;
+    z-index: 1;
+    display: grid;
+    grid-auto-columns: 1fr;
+    grid-auto-flow: column;
+    padding: var(--space-1) var(--space-2) calc(var(--space-1) + env(safe-area-inset-bottom));
+    border-block-start: 1px solid var(--line);
+    background: var(--card);
+  }
+
+  .tabbar > a {
+    display: flex;
+    flex-direction: column;
+    align-items: center;
+    justify-content: center;
+    gap: 0.1875rem;
+    min-block-size: 3rem;
+    color: var(--muted);
+    font-size: var(--text-tiny);
+    font-weight: 500;
+    text-decoration: none;
+  }
+
+  .tabbar > a[aria-current="page"] {
+    color: var(--accent);
+    font-weight: 650;
+  }
+
+  .tabbar .icon {
+    inline-size: 1.5rem;
+    block-size: 1.5rem;
+  }
+}
diff --git a/crates/web/css/63-install.css b/crates/web/css/63-install.css
new file mode 100644
index 0000000..1521a34
--- /dev/null
+++ b/crates/web/css/63-install.css
@@ -0,0 +1,8 @@
+/* .install: the Install card of Settings, pointless once the app runs installed. */
+@layer components {
+  @media (display-mode: standalone) {
+    .install {
+      display: none;
+    }
+  }
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index d447c99..bb14981 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -11,6 +11,7 @@ login-locked = Too many attempts. Try again in 15 minutes.
 login-expired = This account has expired. Ask a resident to reactivate it.
 signed-in-as = Signed in as { $username }.
 logout = Sign out
+account-menu = Account: { $username }
 
 log-title = Log a smell
 event-none = No smell event in progress.
@@ -139,6 +140,7 @@ nav-events = Events
 nav-graphs = Graphs
 nav-people = People
 nav-settings = Settings
+nav-label = Main menu
 
 events-title = Events
 events-none = No events this month.
@@ -262,6 +264,7 @@ expiry-date = at the end of this day:
 expiry-never = never
 
 install-app = Install app
+install-card-hint = Open it from your home screen, like any app.
 install-title = Add Smoke Tracker to your home screen
 install-welcome = Your account is ready.
 install-intro = It then opens like an app, straight on the Log screen, so logging takes a few seconds.
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index e138d57..65fbff0 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -11,6 +11,7 @@ login-locked = Troppi tentativi. Riprova tra 15 minuti.
 login-expired = Questo account è scaduto. Chiedi a un residente di riattivarlo.
 signed-in-as = Accesso effettuato come { $username }.
 logout = Esci
+account-menu = Account: { $username }
 
 log-title = Registra un odore
 event-none = Nessun evento di fumo in corso.
@@ -139,6 +140,7 @@ nav-events = Eventi
 nav-graphs = Grafici
 nav-people = Persone
 nav-settings = Impostazioni
+nav-label = Menu principale
 
 events-title = Eventi
 events-none = Nessun evento questo mese.
@@ -262,6 +264,7 @@ expiry-date = alla fine di questo giorno:
 expiry-never = mai
 
 install-app = Installa l'app
+install-card-hint = Aprila dalla schermata Home, come ogni app.
 install-title = Aggiungi Smoke Tracker alla schermata Home
 install-welcome = Il tuo account è pronto.
 install-intro = Si aprirà come un'app, direttamente sulla schermata Registra: registrare richiede pochi secondi.
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 7a30f7d..2253580 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -11,6 +11,7 @@ login-locked = Çok fazla deneme. 15 dakika sonra tekrar deneyin.
 login-expired = Bu hesabın süresi doldu. Yeniden etkinleştirmesi için bir ev sakinine başvurun.
 signed-in-as = { $username } olarak giriş yapıldı.
 logout = Çıkış yap
+account-menu = Hesap: { $username }
 
 log-title = Koku kaydet
 event-none = Devam eden bir koku olayı yok.
@@ -139,6 +140,7 @@ nav-events = Olaylar
 nav-graphs = Grafikler
 nav-people = Kişiler
 nav-settings = Ayarlar
+nav-label = Ana menü
 
 events-title = Olaylar
 events-none = Bu ay olay yok.
@@ -262,6 +264,7 @@ expiry-date = bu günün sonunda:
 expiry-never = hiçbir zaman
 
 install-app = Uygulamayı yükle
+install-card-hint = Her uygulama gibi ana ekrandan açın.
 install-title = Smoke Tracker'ı ana ekranınıza ekleyin
 install-welcome = Hesabınız hazır.
 install-intro = Böylece doğrudan Kaydet ekranında bir uygulama gibi açılır ve kayıt birkaç saniye sürer.
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 0c99b5e..730e63a 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -52,6 +52,10 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "55-notice.css",
     "56-badge.css",
     "57-row.css",
+    "60-appbar.css",
+    "61-menu.css",
+    "62-tabbar.css",
+    "63-install.css",
     "90-utilities.css",
 );
 
@@ -74,6 +78,7 @@ const ASSETS: &[Asset] = &[
     asset!("js/dialog.js", "text/javascript; charset=utf-8"),
     asset!("js/graphs.js", "text/javascript; charset=utf-8"),
     asset!("js/install.js", "text/javascript; charset=utf-8"),
+    asset!("js/menu.js", "text/javascript; charset=utf-8"),
     asset!("js/service-worker.js", "text/javascript; charset=utf-8"),
     asset!("manifest.webmanifest", "application/manifest+json"),
     asset!("sw.js", "text/javascript; charset=utf-8"),
diff --git a/crates/web/src/header.rs b/crates/web/src/header.rs
index 4a836a4..277b8c6 100644
--- a/crates/web/src/header.rs
+++ b/crates/web/src/header.rs
@@ -1,20 +1,113 @@
-//! What the page header shows for the signed-in person (spec §9 screen 8).
+//! The page frame for the signed-in person (look-and-feel spec §3.3): the avatar menu in the
+//! header and the bottom tab bar, with the current tab marked.
 
 use chrono::{DateTime, Utc};
 use smoke_domain::permissions::{Action, can};
 use smoke_domain::records::User;
 
+use crate::i18n::Tr;
+
+/// The screens of the tab bar.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub(crate) enum Tab {
+    Log,
+    Events,
+    Graphs,
+    People,
+    Settings,
+}
+
+impl Tab {
+    pub(crate) const ALL: [Tab; 5] = [
+        Tab::Log,
+        Tab::Events,
+        Tab::Graphs,
+        Tab::People,
+        Tab::Settings,
+    ];
+
+    fn href(self) -> &'static str {
+        match self {
+            Self::Log => "/",
+            Self::Events => "/events",
+            Self::Graphs => "/graphs",
+            Self::People => "/people",
+            Self::Settings => "/settings",
+        }
+    }
+
+    /// The symbol in `static/icons.svg`.
+    pub(crate) fn icon(self) -> &'static str {
+        match self {
+            Self::Log => "log",
+            Self::Events => "events",
+            Self::Graphs => "graphs",
+            Self::People => "people",
+            Self::Settings => "settings",
+        }
+    }
+
+    fn label_key(self) -> &'static str {
+        match self {
+            Self::Log => "nav-log",
+            Self::Events => "nav-events",
+            Self::Graphs => "nav-graphs",
+            Self::People => "nav-people",
+            Self::Settings => "nav-settings",
+        }
+    }
+}
+
+/// One link of the tab bar.
+pub(crate) struct TabLink {
+    pub(crate) href: &'static str,
+    pub(crate) icon: &'static str,
+    pub(crate) label: String,
+    pub(crate) current: bool,
+}
+
 pub(crate) struct SignedIn {
     pub(crate) username: String,
-    /// The menu links to People (residents).
-    pub(crate) people: bool,
+    /// The avatar: the username's first letter, upper case.
+    pub(crate) initial: String,
+    /// The tab bar links to People (residents).
+    people: bool,
+    /// The tab this page belongs to.
+    tab: Tab,
 }
 
 impl SignedIn {
-    pub(crate) fn of(user: &User, now: DateTime<Utc>) -> Option<Self> {
+    pub(crate) fn of(user: &User, now: DateTime<Utc>, tab: Tab) -> Option<Self> {
+        let username = user.username.to_string();
         Some(Self {
-            username: user.username.to_string(),
+            initial: username
+                .chars()
+                .next()
+                .into_iter()
+                .flat_map(char::to_uppercase)
+                .collect(),
+            username,
             people: can(&user.actor(), Action::ManageUsers, now),
+            tab,
         })
     }
+
+    pub(crate) fn tabs(&self, tr: &Tr) -> Vec<TabLink> {
+        Tab::ALL
+            .into_iter()
+            .filter(|&tab| tab != Tab::People || self.people)
+            .map(|tab| TabLink {
+                href: tab.href(),
+                icon: tab.icon(),
+                label: tr.t(tab.label_key()),
+                current: tab == self.tab,
+            })
+            .collect()
+    }
+}
+
+/// Every key the tab bar looks up, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    Tab::ALL.iter().map(|tab| tab.label_key()).collect()
 }
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index 2a49d0b..f1c78ae 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -31,6 +31,21 @@ fn language(locale: Locale) -> &'static LanguageIdentifier {
     }
 }
 
+/// The languages, each named in itself.
+const LANGUAGES: [(Locale, &str); 3] = [
+    (Locale::En, "English"),
+    (Locale::It, "Italiano"),
+    (Locale::Tr, "Türkçe"),
+];
+
+/// A language as the language buttons and radios offer it.
+pub(crate) struct LanguageChoice {
+    pub(crate) value: &'static str,
+    pub(crate) name: &'static str,
+    /// The language the page is in.
+    pub(crate) current: bool,
+}
+
 /// Translates for one language. Templates call `tr.t("app-name")`; a key missing from a language
 /// falls back to English.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
@@ -41,6 +56,18 @@ impl Tr {
         self.0.as_str()
     }
 
+    /// Every language, the page's own marked current.
+    pub(crate) fn languages(&self) -> Vec<LanguageChoice> {
+        LANGUAGES
+            .iter()
+            .map(|&(locale, name)| LanguageChoice {
+                value: locale.as_str(),
+                name,
+                current: locale == self.0,
+            })
+            .collect()
+    }
+
     pub(crate) fn t(&self, key: &str) -> String {
         LOCALES.lookup(language(self.0), key)
     }
@@ -128,7 +155,7 @@ mod tests {
     use super::*;
     use crate::routes::login::Notice;
     use crate::routes::{self, entries, graphs, guide, lists, people, reports, settings};
-    use crate::{error, labels};
+    use crate::{error, header, labels};
 
     const DIR: &str = env!("CARGO_MANIFEST_DIR");
 
@@ -212,6 +239,7 @@ mod tests {
             "the source scan found nothing"
         );
         used.extend(error::KEYS.map(str::to_owned));
+        used.extend(header::keys().into_iter().map(str::to_owned));
         used.extend(Notice::ALL.map(|notice| notice.key().to_owned()));
         used.extend(guide::keys());
         used.extend(labels::keys().into_iter().map(str::to_owned));
diff --git a/crates/web/src/routes/accept.rs b/crates/web/src/routes/accept.rs
index 7809892..b349b66 100644
--- a/crates/web/src/routes/accept.rs
+++ b/crates/web/src/routes/accept.rs
@@ -21,7 +21,6 @@ use crate::error::{AppError, page};
 use crate::header::SignedIn;
 use crate::i18n::Tr;
 use crate::routes::people_refusal;
-use crate::routes::settings::{LANGUAGES, LanguageChoice};
 use crate::state::AppState;
 
 #[derive(Template)]
@@ -31,7 +30,6 @@ struct AcceptPage {
     signed_in: Option<SignedIn>,
     username: String,
     setup: bool,
-    languages: Vec<LanguageChoice>,
     error: Option<&'static str>,
 }
 
@@ -70,14 +68,6 @@ fn accept_page(
             signed_in: None,
             username,
             setup: invite.purpose == InvitePurpose::Setup,
-            languages: LANGUAGES
-                .iter()
-                .map(|&(locale, name)| LanguageChoice {
-                    value: locale.as_str(),
-                    name,
-                    current: locale == tr.0,
-                })
-                .collect(),
             error,
         },
     )
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index 49e60e9..2541d68 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -22,7 +22,7 @@ use smoke_domain::types::{EventId, Level, Pm25, Role, RoomId, SymptomId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::log::{Draft, log_page, offered};
 use crate::routes::{notices, refusal};
@@ -435,7 +435,7 @@ async fn ask<S: Store>(
         StatusCode::OK,
         &PromptPage {
             tr,
-            signed_in: SignedIn::of(user, now),
+            signed_in: SignedIn::of(user, now, Tab::Log),
             title,
             detail,
             hidden: form.without(&["choice", "asked"]),
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index 3a71bab..d2339d4 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -21,7 +21,7 @@ use smoke_domain::types::{EventId, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::labels;
 use crate::routes::entries::read_time;
@@ -182,7 +182,7 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
         StatusCode::OK,
         &ListPage {
             tr,
-            signed_in: SignedIn::of(&user, now),
+            signed_in: SignedIn::of(&user, now, Tab::Events),
             month: first_of_month.format("%m/%Y").to_string(),
             previous: month_param(first_of_month - Months::new(1)),
             next: month_param(first_of_month + Months::new(1)),
@@ -341,7 +341,7 @@ async fn detail<S: Store>(
     };
     Ok(DetailPage {
         tr,
-        signed_in: SignedIn::of(user, now),
+        signed_in: SignedIn::of(user, now, Tab::Events),
         id: id.0,
         date: time::date(tr.0, state.config.schedule.local_date(event.opened_at)),
         span: span(tr, &event, tz),
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
index c5b61ab..dc4fbbd 100644
--- a/crates/web/src/routes/graphs.rs
+++ b/crates/web/src/routes/graphs.rs
@@ -25,7 +25,7 @@ use smoke_domain::types::{Locale, UserId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::labels;
 use crate::state::AppState;
@@ -355,7 +355,7 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
         StatusCode::OK,
         &GraphsPage {
             tr,
-            signed_in: SignedIn::of(&user, now),
+            signed_in: SignedIn::of(&user, now, Tab::Graphs),
             label,
             previous: address(period.previous()),
             next: address(period.next()),
diff --git a/crates/web/src/routes/guide.rs b/crates/web/src/routes/guide.rs
index f96ed30..11ce6d6 100644
--- a/crates/web/src/routes/guide.rs
+++ b/crates/web/src/routes/guide.rs
@@ -11,7 +11,7 @@ use smoke_domain::types::Level;
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::state::AppState;
 
@@ -91,7 +91,7 @@ pub(crate) async fn rating<S: Store + Clone + 'static>(
         StatusCode::OK,
         &GuidePage {
             tr,
-            signed_in: SignedIn::of(&user, state.clock.now()),
+            signed_in: SignedIn::of(&user, state.clock.now(), Tab::Settings),
             levels: levels(tr),
         },
     )
@@ -115,7 +115,7 @@ pub(crate) async fn logging<S: Store + Clone + 'static>(
         StatusCode::OK,
         &LoggingPage {
             tr,
-            signed_in: SignedIn::of(&user, state.clock.now()),
+            signed_in: SignedIn::of(&user, state.clock.now(), Tab::Settings),
             situations: situations(tr),
             rules: rules(tr, state.config.schedule.hour()),
         },
diff --git a/crates/web/src/routes/install.rs b/crates/web/src/routes/install.rs
index 7bdec28..0fdcb74 100644
--- a/crates/web/src/routes/install.rs
+++ b/crates/web/src/routes/install.rs
@@ -10,7 +10,7 @@ use smoke_domain::repos::Store;
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::state::AppState;
 
@@ -40,7 +40,7 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
         StatusCode::OK,
         &InstallPage {
             tr,
-            signed_in: user.and_then(|CurrentUser(user)| SignedIn::of(&user, now)),
+            signed_in: user.and_then(|CurrentUser(user)| SignedIn::of(&user, now, Tab::Settings)),
             welcome: query.welcome.is_some(),
         },
     )
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index f1b712b..6af18c1 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -17,7 +17,7 @@ use smoke_domain::types::{Level, RoomArea, RoomId, SymptomId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::entries::level_field;
 use crate::routes::guide::{GuideRow, levels};
@@ -293,7 +293,7 @@ pub(crate) async fn log_page<S: Store>(
 
     Ok(LogPage {
         tr,
-        signed_in: SignedIn::of(user, now),
+        signed_in: SignedIn::of(user, now, Tab::Log),
         close_event: shown
             .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
             .map(|e| e.id.0),
diff --git a/crates/web/src/routes/people/links.rs b/crates/web/src/routes/people/links.rs
index 189d884..91895f9 100644
--- a/crates/web/src/routes/people/links.rs
+++ b/crates/web/src/routes/people/links.rs
@@ -16,7 +16,7 @@ use smoke_domain::types::{InvitePurpose, Role, TokenHash, UserId, Username};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::people::expiry::{self, ExpiryDraft, ExpiryFields};
 use crate::routes::people::{require_manager, role_key};
@@ -85,7 +85,7 @@ fn invite_page(
         status,
         &InvitePage {
             tr,
-            signed_in: SignedIn::of(user, now),
+            signed_in: SignedIn::of(user, now, Tab::People),
             error,
             username: draft.username,
             roles: ROLES
@@ -140,7 +140,7 @@ fn link_page<S>(
         StatusCode::OK,
         &LinkPage {
             tr,
-            signed_in: SignedIn::of(user, now),
+            signed_in: SignedIn::of(user, now, Tab::People),
             username: person.username.to_string(),
             setup: link.purpose == InvitePurpose::Setup,
             qr: qr::svg(&url),
diff --git a/crates/web/src/routes/people/mod.rs b/crates/web/src/routes/people/mod.rs
index 2a4e420..6569289 100644
--- a/crates/web/src/routes/people/mod.rs
+++ b/crates/web/src/routes/people/mod.rs
@@ -20,7 +20,7 @@ use smoke_domain::types::{Role, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::people::expiry::{ExpiryDraft, ExpiryFields};
 use crate::routes::people_refusal;
@@ -135,7 +135,7 @@ async fn people_page<S: Store>(
         status_code,
         &PeoplePage {
             tr,
-            signed_in: SignedIn::of(user, now),
+            signed_in: SignedIn::of(user, now, Tab::People),
             saved,
             error,
             people,
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index 0853837..45f9963 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -17,7 +17,7 @@ use smoke_domain::types::{EventId, ReportId, SymptomId, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::entries::{Smell, joinable_events, read_time, read_values};
 use crate::routes::guide::{GuideRow, levels};
@@ -252,7 +252,7 @@ async fn report_page<S: Store>(
     let intensity = report.values.intensity().get();
     Ok(ReportPage {
         tr,
-        signed_in: SignedIn::of(user, now),
+        signed_in: SignedIn::of(user, now, Tab::Events),
         id: id.0,
         event: event.map(|event| EventLink {
             id: event.id.0,
@@ -333,7 +333,7 @@ async fn correct_page<S: Store>(
     let keep = (room_ids.as_slice(), report.values.symptoms());
     Ok(CorrectPage {
         tr,
-        signed_in: SignedIn::of(user, state.clock.now()),
+        signed_in: SignedIn::of(user, state.clock.now(), Tab::Events),
         id: report.id.0,
         levels: levels(tr),
         check: report.event_id.is_none(),
diff --git a/crates/web/src/routes/settings.rs b/crates/web/src/routes/settings.rs
index 1a66c82..c38f4d5 100644
--- a/crates/web/src/routes/settings.rs
+++ b/crates/web/src/routes/settings.rs
@@ -11,32 +11,19 @@ use smoke_domain::people::change_password;
 use smoke_domain::permissions::{Action, can};
 use smoke_domain::records::User;
 use smoke_domain::repos::{Commit, Store};
-use smoke_domain::types::{Locale, NewPassword};
+use smoke_domain::types::NewPassword;
 use tower_sessions::Session;
 
 use crate::auth::client_ip::ClientIp;
 use crate::auth::limiter::LoginLimiter;
 use crate::auth::{self, CurrentUser, password};
 use crate::error::{AppError, page};
-use crate::header::SignedIn;
+use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::lists::{self, ListsView};
 use crate::routes::people_refusal;
 use crate::state::AppState;
 
-/// The languages, each named in itself.
-pub(crate) const LANGUAGES: [(Locale, &str); 3] = [
-    (Locale::En, "English"),
-    (Locale::It, "Italiano"),
-    (Locale::Tr, "Türkçe"),
-];
-
-pub(crate) struct LanguageChoice {
-    pub(crate) value: &'static str,
-    pub(crate) name: &'static str,
-    pub(crate) current: bool,
-}
-
 /// The messages above the page's sections: what was just saved, or why a form was refused.
 #[derive(Debug, Clone, Copy, Default)]
 pub(crate) struct Notices {
@@ -52,7 +39,6 @@ struct SettingsPage {
     signed_in: Option<SignedIn>,
     /// For password managers, which file the new password under this name.
     username: String,
-    languages: Vec<LanguageChoice>,
     notices: Notices,
     /// `None` for people who may not edit the lists.
     lists: Option<ListsView>,
@@ -75,16 +61,8 @@ pub(crate) async fn settings_page<S: Store>(
         status,
         &SettingsPage {
             tr,
-            signed_in: SignedIn::of(user, now),
+            signed_in: SignedIn::of(user, now, Tab::Settings),
             username: user.username.to_string(),
-            languages: LANGUAGES
-                .iter()
-                .map(|&(locale, name)| LanguageChoice {
-                    value: locale.as_str(),
-                    name,
-                    current: locale == user.locale,
-                })
-                .collect(),
             notices,
             lists,
         },
diff --git a/crates/web/static/js/app.js b/crates/web/static/js/app.js
index cd59d41..d448e66 100644
--- a/crates/web/static/js/app.js
+++ b/crates/web/static/js/app.js
@@ -3,4 +3,5 @@
 import "./dialog.js";
 import "./date-picks.js";
 import "./install.js";
+import "./menu.js";
 import "./service-worker.js";
diff --git a/crates/web/static/js/menu.js b/crates/web/static/js/menu.js
new file mode 100644
index 0000000..c8bace3
--- /dev/null
+++ b/crates/web/static/js/menu.js
@@ -0,0 +1,21 @@
+// A <details data-menu> closes when the person taps outside it or presses Escape, and Escape
+// gives the focus back to its summary. Without JavaScript it opens and closes from its summary.
+const openMenus = () => document.querySelectorAll("details[data-menu][open]");
+
+document.addEventListener("click", (event) => {
+  for (const menu of openMenus()) {
+    if (!menu.contains(event.target)) {
+      menu.open = false;
+    }
+  }
+});
+
+document.addEventListener("keydown", (event) => {
+  if (event.key !== "Escape") {
+    return;
+  }
+  for (const menu of openMenus()) {
+    menu.open = false;
+    menu.querySelector("summary")?.focus();
+  }
+});
diff --git a/crates/web/static/sw.js b/crates/web/static/sw.js
index f6b6469..c33902e 100644
--- a/crates/web/static/sw.js
+++ b/crates/web/static/sw.js
@@ -11,6 +11,7 @@ const PRECACHE = [
   "/static/js/date-picks.js",
   "/static/js/dialog.js",
   "/static/js/install.js",
+  "/static/js/menu.js",
   "/static/js/service-worker.js",
   "/static/htmx-2.0.11.min.js",
   "/static/favicon.svg",
diff --git a/crates/web/templates/accept.html b/crates/web/templates/accept.html
index 48abbd4..f1c53a6 100644
--- a/crates/web/templates/accept.html
+++ b/crates/web/templates/accept.html
@@ -17,7 +17,7 @@
   <p class="hint">{{ tr.t("accept-password-hint") }}</p>
   <fieldset class="choices">
     <legend>{{ tr.t("language") }}</legend>
-    {% for language in languages %}
+    {% for language in tr.languages() %}
     <label class="chip"><input type="radio" name="lang" value="{{ language.value }}"{% if language.current %} checked{% endif %}><span lang="{{ language.value }}">{{ language.name }}</span></label>
     {% endfor %}
   </fieldset>
diff --git a/crates/web/templates/base.html b/crates/web/templates/base.html
index 3d0380b..7364014 100644
--- a/crates/web/templates/base.html
+++ b/crates/web/templates/base.html
@@ -1,3 +1,4 @@
+{% import "components.html" as c %}
 <!doctype html>
 <html lang="{{ tr.lang() }}">
 <head>
@@ -15,33 +16,23 @@
   {% block scripts %}{% endblock %}
 </head>
 <body>
-  <header class="bar">
+  {% block header %}
+  <header class="appbar">
     <span class="brand">{{ tr.t("app-name") }}</span>
-    <a href="/install" class="install" data-install>{{ tr.t("install-app") }}</a>
     {% if let Some(me) = signed_in %}
-    <nav class="menu">
-      <a href="/">{{ tr.t("nav-log") }}</a>
-      <a href="/events">{{ tr.t("nav-events") }}</a>
-      <a href="/graphs">{{ tr.t("nav-graphs") }}</a>
-      {% if me.people %}<a href="/people">{{ tr.t("nav-people") }}</a>{% endif %}
-      <a href="/settings">{{ tr.t("nav-settings") }}</a>
-    </nav>
-    <form method="post" action="/logout" class="who">
-      <span>{{ tr.t1("signed-in-as", "username", me.username) }}</span>
-      <button type="submit" class="link">{{ tr.t("logout") }}</button>
-    </form>
+    {% call c::account_menu(me.initial, tr.t1("account-menu", "username", me.username), tr.t1("signed-in-as", "username", me.username), tr.t("logout")) %}{% endcall %}
     {% endif %}
   </header>
+  {% endblock %}
   <main>
     {% block content %}{% endblock %}
   </main>
-  <footer>
-    <form method="post" action="/lang" class="languages">
-      <span>{{ tr.t("language") }}:</span>
-      <button type="submit" name="lang" value="en" lang="en" class="link">English</button>
-      <button type="submit" name="lang" value="it" lang="it" class="link">Italiano</button>
-      <button type="submit" name="lang" value="tr" lang="tr" class="link">Türkçe</button>
-    </form>
+  {% if let Some(me) = signed_in %}
+  {% call c::tabbar(me.tabs(tr), tr.t("nav-label")) %}{% endcall %}
+  {% else %}
+  <footer class="footer">
+    {% call c::languages(tr.languages(), tr.t("language"), "") %}{% endcall %}
   </footer>
+  {% endif %}
 </body>
 </html>
diff --git a/crates/web/templates/components.html b/crates/web/templates/components.html
index d08eade..1d2fde6 100644
--- a/crates/web/templates/components.html
+++ b/crates/web/templates/components.html
@@ -67,3 +67,26 @@
   {% endfor %}
 </form>
 {% endmacro %}
+
+{# The avatar in the header and its menu: who is signed in, and Sign out. A <details>, so it
+   opens without JavaScript; js/menu.js closes it on an outside tap or Escape. #}
+{% macro account_menu(initial, label, signed_in, logout) %}
+<details class="menu" data-menu>
+  <summary class="avatar" aria-label="{{ label }}">{{ initial }}</summary>
+  <div class="menu-panel">
+    <p>{{ signed_in }}</p>
+    <form method="post" action="/logout">
+      <button type="submit" class="menu-item">{% call icon("logout") %}{% endcall %}{{ logout }}</button>
+    </form>
+  </div>
+</details>
+{% endmacro %}
+
+{# The bottom tab bar; the current page's link is marked aria-current. #}
+{% macro tabbar(tabs, label) %}
+<nav class="tabbar" aria-label="{{ label }}">
+  {% for tab in tabs %}
+  <a href="{{ tab.href }}"{% if tab.current %} aria-current="page"{% endif %}>{% call icon(tab.icon) %}{% endcall %}<span>{{ tab.label }}</span></a>
+  {% endfor %}
+</nav>
+{% endmacro %}
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
index a5f321c..d2990d2 100644
--- a/crates/web/templates/settings.html
+++ b/crates/web/templates/settings.html
@@ -1,19 +1,20 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("settings-title") }}{% endblock %}
 {% block content %}
 <h1>{{ tr.t("settings-title") }}</h1>
 {% if let Some(saved) = notices.saved %}
 <p class="notice" role="status">{{ tr.t(saved) }}</p>
 {% endif %}
-<section>
-  <h2>{{ tr.t("language") }}</h2>
-  <form method="post" action="/lang" class="choices">
-    <input type="hidden" name="return_to" value="settings">
-    {% for language in languages %}
-    <button type="submit" name="lang" value="{{ language.value }}" lang="{{ language.value }}"{% if language.current %} class="current" aria-pressed="true"{% else %} class="secondary" aria-pressed="false"{% endif %}>{{ language.name }}</button>
-    {% endfor %}
-  </form>
+<section class="card install">
+  <div class="split">
+    <span class="row-text"><strong>{{ tr.t("install-app") }}</strong><span>{{ tr.t("install-card-hint") }}</span></span>
+    <a href="/install" class="button" data-install>{{ tr.t("install-button") }}</a>
+  </div>
 </section>
+{% call c::card(tr.t("language")) %}
+{% call c::languages(tr.languages(), tr.t("language"), "settings") %}{% endcall %}
+{% endcall %}
 <section>
   <h2>{{ tr.t("settings-password") }}</h2>
   {% if let Some(error) = notices.password_error %}
diff --git a/crates/web/tests/graphs.rs b/crates/web/tests/graphs.rs
index b3d3904..59ffe90 100644
--- a/crates/web/tests/graphs.rs
+++ b/crates/web/tests/graphs.rs
@@ -92,7 +92,10 @@ async fn household() -> (TestApp, String, String) {
 async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
     let (app, resident, _) = household().await;
     let html = text(app.send(get_with("/graphs?month=2026-01", &resident)).await).await;
-    assert!(html.contains(r#"<a href="/graphs">Graphs</a>"#), "{html}");
+    assert!(
+        html.contains(r#"<a href="/graphs" aria-current="page">"#),
+        "{html}"
+    );
     assert!(html.contains("<strong>01/2026</strong>"), "{html}");
     assert!(!html.contains("only your own"), "{html}");
     let intensity = &charts(&html)[0];
diff --git a/crates/web/tests/people.rs b/crates/web/tests/people.rs
index 0b5201a..3204d7e 100644
--- a/crates/web/tests/people.rs
+++ b/crates/web/tests/people.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::clock::Clock;
 use smoke_domain::repos::UserRepo;
@@ -16,7 +17,10 @@ async fn only_residents_reach_people_and_see_it_in_the_menu() {
     app.add_user("lawyer", Role::Viewer, Locale::En).await;
     let resident = app.sign_in("daniele").await;
     let home = text(app.send(get_with("/", &resident)).await).await;
-    assert!(home.contains(r#"<a href="/people">People</a>"#), "{home}");
+    assert_eq!(
+        Page::parse(&home).text(r#"nav.tabbar a[href="/people"]"#),
+        "People"
+    );
     for name in ["guest", "lawyer"] {
         let cookie = app.sign_in(name).await;
         let home = text(app.send(get_with("/", &cookie)).await).await;
diff --git a/crates/web/tests/pwa.rs b/crates/web/tests/pwa.rs
index bf6f594..6fbc7c7 100644
--- a/crates/web/tests/pwa.rs
+++ b/crates/web/tests/pwa.rs
@@ -3,11 +3,12 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::types::{Locale, Role};
 
 #[tokio::test]
-async fn every_page_links_the_manifest_and_the_install_page() {
+async fn every_page_links_the_manifest() {
     let app = TestApp::new().await;
     for html in [
         text(app.send(get("/login")).await).await,
@@ -21,10 +22,6 @@ async fn every_page_links_the_manifest_and_the_install_page() {
             html.contains(r#"<link rel="apple-touch-icon" href="/static/icon-180.png">"#),
             "{html}"
         );
-        assert!(
-            html.contains(r#"<a href="/install" class="install" data-install>Install app</a>"#),
-            "{html}"
-        );
     }
     let manifest = app.send(get("/static/manifest.webmanifest")).await;
     assert_eq!(
@@ -33,6 +30,28 @@ async fn every_page_links_the_manifest_and_the_install_page() {
     );
 }
 
+#[tokio::test]
+async fn settings_offers_the_install_card_instead_of_the_header() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let home = Page::parse(&text(app.send(get_with("/", &cookie)).await).await);
+    assert!(
+        !home.exists("[data-install]"),
+        "no install link outside Settings"
+    );
+    let settings = Page::parse(&text(app.send(get_with("/settings", &cookie)).await).await);
+    assert_eq!(settings.text(".install strong"), "Install app");
+    assert_eq!(
+        settings
+            .attr(".install a.button[data-install]", "href")
+            .as_deref(),
+        Some("/install")
+    );
+    let first_card = settings.attrs("main > section", "class");
+    assert_eq!(first_card.first().map(String::as_str), Some("card install"));
+}
+
 #[tokio::test]
 async fn the_service_worker_is_served_at_the_root() {
     let app = TestApp::new().await;
@@ -85,7 +104,7 @@ async fn the_install_page_offers_the_button_and_the_iphone_steps() {
     let html = text(app.send(get_with("/install?welcome", &cookie)).await).await;
     assert!(html.contains("Hesabınız hazır."), "{html}");
     assert!(
-        html.contains(r#"<a href="/settings">"#),
+        Page::parse(&html).exists(r#"nav.tabbar a[href="/settings"][aria-current=page]"#),
         "signed in: {html}"
     );
 }
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index e38dba3..ab2acec 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -5,6 +5,7 @@ mod common;
 use axum::body::Body;
 use axum::http::header::CONTENT_TYPE;
 use axum::http::{Request, StatusCode};
+use common::page::Page;
 use common::*;
 use smoke_domain::types::{Locale, Role};
 
@@ -186,7 +187,7 @@ fn the_event_handler_scan_finds_attributes_and_not_words() {
 }
 
 #[tokio::test]
-async fn no_page_needs_inline_scripts_styles_or_eval() {
+async fn no_page_needs_inline_scripts_styles_or_eval_and_every_icon_exists() {
     let app = TestApp::new().await;
     app.at("2026-01-15T20:00:00Z");
     app.add_user("daniele", Role::Resident, Locale::En).await;
@@ -250,6 +251,7 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         "{accept_page}"
     );
     let pages = pages.into_iter().chain([accept_page]);
+    let sprite = include_str!("../static/icons.svg");
     for html in pages {
         assert!(
             html.contains(r#"<meta name="htmx-config" content='{"includeIndicatorStyles":false,"allowEval":false,"allowScriptTags":false}'>"#),
@@ -270,6 +272,10 @@ async fn no_page_needs_inline_scripts_styles_or_eval() {
         for script in html.match_indices("<script").map(|(i, _)| &html[i..]) {
             assert!(script.starts_with("<script src=\"/static/"), "{script}");
         }
+        for icon in Page::parse(&html).attrs("svg use", "href") {
+            let id = icon.strip_prefix("/static/icons.svg#").expect("the sprite");
+            assert!(sprite.contains(&format!("<symbol id=\"{id}\"")), "{id}");
+        }
     }
 }
 
diff --git a/crates/web/tests/settings.rs b/crates/web/tests/settings.rs
index 2556ef2..a2be555 100644
--- a/crates/web/tests/settings.rs
+++ b/crates/web/tests/settings.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::repos::{ListRepo, UserRepo};
 use smoke_domain::types::{Locale, Role, RoomArea};
@@ -19,17 +20,17 @@ async fn the_settings_page_shows_the_language_and_password_forms() {
     app.add_user("guest", Role::Guest, Locale::It).await;
     let cookie = app.sign_in("guest").await;
     let html = text(app.send(get_with("/settings", &cookie)).await).await;
-    assert!(html.contains("<h1>Impostazioni</h1>"), "{html}");
-    assert!(
-        html.contains(
-            r#"value="it" lang="it" class="current" aria-pressed="true">Italiano</button>"#
-        ),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(page.text("h1"), "Impostazioni");
+    assert_eq!(
+        page.text(r#"form[action="/lang"] button[aria-current=true]"#),
+        "Italiano"
     );
-    assert!(html.contains(r#"action="/settings/password""#), "{html}");
-    assert!(
-        html.contains(r#"<a href="/settings">Impostazioni</a>"#),
-        "{html}"
+    assert_eq!(page.count(r#"form[action="/lang"] button[name=lang]"#), 3);
+    assert!(page.exists(r#"form[action="/settings/password"]"#));
+    assert_eq!(
+        page.text(r#"nav.tabbar a[aria-current=page]"#),
+        "Impostazioni"
     );
 }
 
diff --git a/crates/web/tests/shell.rs b/crates/web/tests/shell.rs
new file mode 100644
index 0000000..04edad7
--- /dev/null
+++ b/crates/web/tests/shell.rs
@@ -0,0 +1,91 @@
+//! The frame around every page (look-and-feel spec §3.3, §6.5): the header with the avatar
+//! menu and the tab bar when signed in; the language buttons at the bottom when signed out.
+
+mod common;
+
+use common::page::Page;
+use common::*;
+use smoke_domain::types::{Locale, Role};
+
+async fn page(app: &TestApp, path: &str, cookie: &str) -> Page {
+    Page::parse(&text(app.send(get_with(path, cookie)).await).await)
+}
+
+#[tokio::test]
+async fn the_avatar_menu_names_the_account_and_signs_out() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let home = page(&app, "/", &cookie).await;
+    assert_eq!(home.text("header .brand"), "Smoke Tracker");
+    assert_eq!(home.text("details.menu[data-menu] > summary.avatar"), "D");
+    assert_eq!(
+        home.attr("summary.avatar", "aria-label").as_deref(),
+        Some("Account: daniele")
+    );
+    assert_eq!(home.text(".menu-panel > p"), "Signed in as daniele.");
+    assert_eq!(
+        home.text(r#".menu-panel form[method=post][action="/logout"] button"#),
+        "Sign out"
+    );
+    assert!(!home.exists("footer"), "no language buttons when signed in");
+}
+
+#[tokio::test]
+async fn the_tab_bar_marks_the_current_screen_and_shows_people_to_residents_only() {
+    let app = TestApp::new().await;
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    app.add_user("guest", Role::Guest, Locale::En).await;
+    let resident = app.sign_in("daniele").await;
+    for (path, current) in [
+        ("/", "Log"),
+        ("/events", "Events"),
+        ("/graphs", "Graphs"),
+        ("/people", "People"),
+        ("/settings", "Settings"),
+        ("/guide/rating", "Settings"),
+    ] {
+        let page = page(&app, path, &resident).await;
+        assert_eq!(
+            page.texts("nav.tabbar > a"),
+            ["Log", "Events", "Graphs", "People", "Settings"],
+            "{path}"
+        );
+        assert_eq!(
+            page.texts("nav.tabbar > a[aria-current=page]"),
+            [current],
+            "{path}"
+        );
+        assert_eq!(
+            page.count("nav.tabbar svg.icon[aria-hidden=true]"),
+            5,
+            "{path}"
+        );
+    }
+    let guest = app.sign_in("guest").await;
+    let page = page(&app, "/", &guest).await;
+    assert_eq!(
+        page.attrs("nav.tabbar > a", "href"),
+        ["/", "/events", "/graphs", "/settings"]
+    );
+}
+
+#[tokio::test]
+async fn signed_out_pages_end_with_the_language_buttons_and_have_no_tab_bar() {
+    let app = TestApp::new().await;
+    for path in ["/login", "/offline", "/install"] {
+        let page = Page::parse(&text(app.send(get(path)).await).await);
+        assert!(!page.exists("nav.tabbar"), "{path}");
+        assert!(!page.exists("details.menu"), "{path}");
+        assert_eq!(
+            page.attrs(r#"footer form[action="/lang"] button[name=lang]"#, "value"),
+            ["en", "it", "tr"],
+            "{path}"
+        );
+        assert_eq!(
+            page.text("footer button[aria-current=true]"),
+            "English",
+            "{path}"
+        );
+    }
+}
````

### Task 5: One Log button that decides from the form

**Proto commit:** `4ef1bbf` + `7ae8ced` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 4ef1bbf`)

Spec §6.1. `entries::read` decides Log vs reading vs error; the form has two buttons (No smell secondary 2/5, Log primary 3/5, `name=action value=log`); `log-pm25-only` and `log-error-pm25-missing` go, `log-error-empty` comes; `log-submit` becomes "Log". The second proto commit adds the pre-ticked-rooms rule (decision #131): apply both diffs in this task. Tests: unit `the_log_button_decides_from_what_is_filled_in`, route `the_log_button_saves_what_the_form_holds`; existing tests post `action=log`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/routes/entries.rs
M	crates/web/templates/log.html
M	crates/web/tests/common/mod.rs
M	crates/web/tests/log.rs
M	crates/web/tests/logging.rs
M	crates/web/tests/security.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/src/routes/entries.rs (tests)`, `crates/web/tests/logging.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **612 passed** (SQLite).
- [ ] **Step 5: Commit** as one commit (`git add` the files listed above) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(log): one Log button that decides from what the form holds

Log replaces Log smell and Log PM2.5 only (look-and-feel spec §6.1): rooms make a smell
report, or a clean check when all are at 0, with PM2.5 saved alongside; without rooms a PM2.5
value alone is a reading, symptoms or a note still need a room, and an empty form says what to
fill in. No smell is unchanged. Unit and route tests cover every row of the table.

The Log screen ticks the rooms of the person's last report in the open event, so a PM2.5
value typed on its own came back as "pick a level". Rooms ticked with no level now let a
PM2.5 value through as a reading, unless symptoms or a note are filled in; alone they still
ask for a level. Found by the browser seed of the accessibility job.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `4ef1bbf`):

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index bb14981..bf4e576 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -28,9 +28,8 @@ log-time = Time
 log-time-now = now
 log-time-change = change
 log-note = Note (optional)
-log-submit = Log smell
+log-submit = Log
 log-update = Update: situation changed
-log-pm25-only = Log PM2.5 only
 log-no-smell = No smell
 log-read-only = You can look at the data but not log.
 
@@ -39,9 +38,9 @@ log-saved-pm25 = PM2.5 reading saved.
 log-saved-no-smell = Saved: no smell.
 log-error-rooms = Pick at least one room.
 log-error-level = Pick a level from 0 to 10 for each selected room.
-log-error-no-smell-level = No smell records the selected rooms at 0. Remove the levels above 0, or use Log smell.
+log-error-no-smell-level = No smell records the selected rooms at 0. Remove the levels above 0, or use Log.
 log-error-pm25 = PM2.5 must be a number from 0 to 1000.
-log-error-pm25-missing = Enter a PM2.5 value to log it on its own.
+log-error-empty = Pick a room, or enter a PM2.5 value.
 log-error-time = Enter a valid date and time.
 log-error-time-skipped = That time does not exist: the clocks moved forward.
 log-error-note = The note can be at most 2000 characters.
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 65fbff0..bf786da 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -28,9 +28,8 @@ log-time = Ora
 log-time-now = adesso
 log-time-change = cambia
 log-note = Nota (facoltativa)
-log-submit = Registra odore
+log-submit = Registra
 log-update = Aggiorna: la situazione è cambiata
-log-pm25-only = Registra solo PM2.5
 log-no-smell = Nessun odore
 log-read-only = Puoi consultare i dati ma non registrare.
 
@@ -39,9 +38,9 @@ log-saved-pm25 = Lettura PM2.5 salvata.
 log-saved-no-smell = Salvato: nessun odore.
 log-error-rooms = Scegli almeno una stanza.
 log-error-level = Scegli un livello da 0 a 10 per ogni stanza selezionata.
-log-error-no-smell-level = Nessun odore registra le stanze selezionate a 0. Togli i livelli sopra 0, oppure usa Registra odore.
+log-error-no-smell-level = Nessun odore registra le stanze selezionate a 0. Togli i livelli sopra 0, oppure usa Registra.
 log-error-pm25 = Il PM2.5 deve essere un numero da 0 a 1000.
-log-error-pm25-missing = Inserisci un valore di PM2.5 per registrarlo da solo.
+log-error-empty = Scegli una stanza o inserisci un valore di PM2.5.
 log-error-time = Inserisci una data e un'ora valide.
 log-error-time-skipped = Quell'ora non esiste: è scattata l'ora legale.
 log-error-note = La nota può avere al massimo 2000 caratteri.
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 2253580..1c91099 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -28,9 +28,8 @@ log-time = Saat
 log-time-now = şimdi
 log-time-change = değiştir
 log-note = Not (isteğe bağlı)
-log-submit = Kokuyu kaydet
+log-submit = Kaydet
 log-update = Güncelle: durum değişti
-log-pm25-only = Yalnızca PM2.5 kaydet
 log-no-smell = Koku yok
 log-read-only = Verilere bakabilirsin ama kayıt yapamazsın.
 
@@ -39,9 +38,9 @@ log-saved-pm25 = PM2.5 ölçümü kaydedildi.
 log-saved-no-smell = Kaydedildi: koku yok.
 log-error-rooms = En az bir oda seç.
 log-error-level = Seçtiğin her oda için 0 ile 10 arasında bir seviye seç.
-log-error-no-smell-level = Koku yok, seçili odaları 0 olarak kaydeder. 0'dan büyük seviyeleri kaldır ya da Kokuyu kaydet'i kullan.
+log-error-no-smell-level = Koku yok, seçili odaları 0 olarak kaydeder. 0'dan büyük seviyeleri kaldır ya da Kaydet'i kullan.
 log-error-pm25 = PM2.5, 0 ile 1000 arasında bir sayı olmalı.
-log-error-pm25-missing = Tek başına kaydetmek için bir PM2.5 değeri gir.
+log-error-empty = Bir oda seç ya da bir PM2.5 değeri gir.
 log-error-time = Geçerli bir tarih ve saat gir.
 log-error-time-skipped = Bu saat yok: saatler ileri alındı.
 log-error-note = Not en fazla 2000 karakter olabilir.
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index 2541d68..f670049 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -35,7 +35,7 @@ pub(crate) const FORM_ERRORS: [&str; 9] = [
     "log-error-time",
     "log-error-time-skipped",
     "log-error-pm25",
-    "log-error-pm25-missing",
+    "log-error-empty",
     "log-error-rooms",
     "log-error-level",
     "log-error-no-smell-level",
@@ -72,7 +72,7 @@ pub(crate) fn level_field(form: &Pairs, room: RoomId) -> Option<u8> {
 /// How a form's rooms and levels are read.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub(crate) enum Smell {
-    /// "Log smell": at least one room, each with a level.
+    /// "Log" with rooms: each room with a level; all at 0 is a clean check.
     Levels,
     /// "No smell": the selected rooms at 0, or the whole home when none is selected.
     None,
@@ -138,6 +138,11 @@ pub(crate) fn read_values(
 
 /// Reads and validates the Log form. `Err` is the translation key of what to fix. Only rooms
 /// and symptoms the form offers (not hidden) are accepted.
+///
+/// "No smell" has its own button. The one Log button decides from what is filled in (look-and-
+/// feel spec §6.1): rooms make a smell report, or a clean check when every level is 0, with the
+/// PM2.5 value saved alongside; without rooms a PM2.5 value alone is a reading, while symptoms
+/// or a note still need a room.
 fn read(
     form: &Pairs,
     rooms: &[Room],
@@ -160,14 +165,18 @@ fn read(
                 .ok_or("log-error-pm25")
         })
         .transpose()?;
-    if form.get("action") == Some("pm25") {
-        let value = pm25.ok_or("log-error-pm25-missing")?;
-        return Ok(Entry::Pm25Only { observed_at, value });
+    let no_smell = form.get("action") == Some("no-smell");
+    let no_rooms = form.ids("room").ok_or("log-error-form")?.is_empty();
+    if no_rooms && !no_smell {
+        let symptoms = form.ids("symptom").ok_or("log-error-form")?;
+        let described = form.get("note").is_some() || !symptoms.is_empty();
+        return match pm25 {
+            _ if described => Err("log-error-rooms"),
+            Some(value) => Ok(Entry::Pm25Only { observed_at, value }),
+            None => Err("log-error-empty"),
+        };
     }
-    let smell = match form.get("action") {
-        Some("no-smell") => Smell::None,
-        _ => Smell::Levels,
-    };
+    let smell = if no_smell { Smell::None } else { Smell::Levels };
     let (offered_rooms, offered_symptoms) = offered(rooms, symptoms, (&[], &[]));
     let values = read_values(form, &offered_rooms, &offered_symptoms, smell)?;
     Ok(Entry::Smell(SmellEntry {
@@ -501,7 +510,7 @@ mod tests {
             "unselected rooms are ignored"
         );
         for (pairs, key) in [
-            (&[("level-1", "6")][..], "log-error-rooms"),
+            (&[("level-1", "6")][..], "log-error-empty"),
             (&[("room", "1")][..], "log-error-level"),
             (&[("room", "1"), ("level-1", "11")][..], "log-error-level"),
             (&[("room", "2"), ("level-2", "6")][..], "log-error-form"),
@@ -543,20 +552,58 @@ mod tests {
         );
     }
 
+    /// The Log button's decision table (look-and-feel spec §6.1), row by row.
     #[test]
-    fn pm25_only_ignores_the_smell_fields_but_needs_a_value() {
+    fn the_log_button_decides_from_what_is_filled_in() {
+        let Ok(Entry::Smell(smell)) = read_form(&[
+            ("action", "log"),
+            ("room", "1"),
+            ("level-1", "6"),
+            ("pm25", "40"),
+        ]) else {
+            panic!("rooms with a level above 0: a smell report");
+        };
+        assert_eq!(smell.values.intensity().get(), 6);
+        assert_eq!(smell.pm25.map(Pm25::ugm3), Some(40.0), "PM2.5 alongside");
+
+        let Ok(Entry::Smell(check)) =
+            read_form(&[("action", "log"), ("room", "1"), ("level-1", "0")])
+        else {
+            panic!("rooms all at 0: a clean check");
+        };
+        assert!(check.values.is_no_smell());
+        assert_eq!(check.values.room_ids(), [RoomId(1)]);
+
         let Ok(Entry::Pm25Only { observed_at, value }) = read_form(&[
-            ("action", "pm25"),
+            ("action", "log"),
             ("pm25", "12,5"),
             ("observed_at", "2026-01-15T20:30"),
         ]) else {
-            panic!("a valid reading");
+            panic!("no rooms, only PM2.5: a reading");
         };
         assert_eq!(observed_at, now() - TimeDelta::minutes(30));
         assert_eq!(value.ugm3(), 12.5);
+
+        for pairs in [
+            &[("action", "log"), ("symptom", "1")][..],
+            &[("action", "log"), ("note", "acrid")][..],
+            &[("action", "log"), ("note", "acrid"), ("pm25", "12")][..],
+        ] {
+            assert_eq!(read_form(pairs).err(), Some("log-error-rooms"), "{pairs:?}");
+        }
+        assert_eq!(
+            read_form(&[("action", "log"), ("note", "  ")]).err(),
+            Some("log-error-empty"),
+            "a blank note is no note"
+        );
+        assert_eq!(
+            read_form(&[("action", "log")]).err(),
+            Some("log-error-empty")
+        );
         assert_eq!(
-            read_form(&[("action", "pm25")]).err(),
-            Some("log-error-pm25-missing")
+            read_form(&[]).err(),
+            Some("log-error-empty"),
+            "no action: Log"
         );
     }
 
diff --git a/crates/web/templates/log.html b/crates/web/templates/log.html
index 11bac56..588bf5b 100644
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -43,9 +43,10 @@
   <label>{{ tr.t("log-note") }}
     <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
   </label>
-  <button type="submit" name="action" value="smell">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
-  <button type="submit" name="action" value="no-smell" formnovalidate class="secondary">{{ tr.t("log-no-smell") }}</button>
-  <button type="submit" name="action" value="pm25" formnovalidate class="secondary">{{ tr.t("log-pm25-only") }}</button>
+  <div class="actions">
+    <button type="submit" name="action" value="no-smell" formnovalidate class="button" data-variant="secondary">{{ tr.t("log-no-smell") }}</button>
+    <button type="submit" name="action" value="log" class="button">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
+  </div>
 </form>
 {% include "rating_dialog.html" %}
 {% else %}
diff --git a/crates/web/tests/common/mod.rs b/crates/web/tests/common/mod.rs
index 44e28a8..b367584 100644
--- a/crates/web/tests/common/mod.rs
+++ b/crates/web/tests/common/mod.rs
@@ -172,7 +172,7 @@ impl TestApp {
         logged
     }
 
-    /// Logs a PM2.5 reading on its own, as "Log PM2.5 only" would.
+    /// Logs a PM2.5 reading on its own, as the Log button does with only a PM2.5 value.
     pub async fn log_pm25(&self, user: &User, at: &str, value: f64) -> Pm25Reading {
         let mut tx = self.store.begin(Some(user.id)).await.unwrap();
         let reading = log_reading(
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index 68e7e33..819b299 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -39,12 +39,19 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
     assert!(inside < bedroom && bedroom < common && common < stairwell);
     assert!(!html.contains("Kitchen"), "hidden rooms are not offered");
     assert!(html.contains(r#"name="symptom""#) && html.contains("Eye irritation"));
-    assert!(html.contains(">Log smell</button>"), "{html}");
-    assert!(html.contains(">Log PM2.5 only</button>"), "{html}");
-    assert!(
-        html.contains(r#"value="no-smell" formnovalidate class="secondary">No smell</button>"#),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.texts(r#"form[action="/log"] .actions button[name=action]"#),
+        ["No smell", "Log"]
+    );
+    assert_eq!(
+        page.attrs(
+            r#"form[action="/log"] .actions button[name=action]"#,
+            "value"
+        ),
+        ["no-smell", "log"]
     );
+    assert!(page.exists("button[value=no-smell][formnovalidate][data-variant=secondary]"));
     assert!(html.contains(" · 0 clean checks"), "{html}");
 }
 
@@ -137,7 +144,7 @@ async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
     ] {
         assert!(!html.contains(hidden), "{hidden}: {html}");
     }
-    assert!(html.contains(">Log smell</button>"), "{html}");
+    assert!(html.contains(">Log</button>"), "{html}");
 
     app.log(
         &guest,
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index 013de7a..a1d2692 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -23,7 +23,7 @@ async fn logging_with_nothing_open_saves_at_once_and_says_so() {
     let cookie = app.sign_in("daniele").await;
 
     let body = format!(
-        "room={0}&level-{0}=6&note=from+the+balcony&action=smell",
+        "room={0}&level-{0}=6&note=from+the+balcony&action=log",
         bedroom.0
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -59,7 +59,7 @@ async fn with_an_event_open_the_same_smell_question_comes_first() {
     let question = app
         .send(post_form(
             "/log",
-            "room=1&level-1=7&action=smell",
+            "room=1&level-1=7&action=log",
             Some(&cookie),
         ))
         .await;
@@ -73,7 +73,7 @@ async fn with_an_event_open_the_same_smell_question_comes_first() {
     assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));
 
     let body = format!(
-        "room=1&level-1=7&action=smell&asked={}&choice=join-{id}",
+        "room=1&level-1=7&action=log&asked={}&choice=join-{id}",
         asked(&html)
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -97,7 +97,7 @@ async fn answering_no_closes_the_open_event_at_its_last_report() {
         .await;
     let cookie = app.sign_in("daniele").await;
     let body = format!(
-        "room=1&level-1=3&action=smell&asked=same-smell-{}&choice=new",
+        "room=1&level-1=3&action=log&asked=same-smell-{}&choice=new",
         first.report.event_id.unwrap()
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
@@ -150,7 +150,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     let html = text(
         app.send(post_form(
             "/log",
-            "room=1&level-1=2&action=smell",
+            "room=1&level-1=2&action=log",
             Some(&cookie),
         ))
         .await,
@@ -167,7 +167,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     let past = text(
         app.send(post_form(
             "/log",
-            "room=1&level-1=2&action=smell&observed_at=2026-01-15T11:30",
+            "room=1&level-1=2&action=log&observed_at=2026-01-15T11:30",
             Some(&cookie),
         ))
         .await,
@@ -209,7 +209,7 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
     let question = text(
         app.send(post_form(
             "/log",
-            "room=1&level-1=7&action=smell",
+            "room=1&level-1=7&action=log",
             Some(&cookie),
         ))
         .await,
@@ -229,7 +229,7 @@ async fn a_stale_answer_gets_the_current_question_instead_of_being_applied() {
         .await;
 
     let body = format!(
-        "room=1&level-1=7&action=smell&asked={}&choice=join-{}",
+        "room=1&level-1=7&action=log&asked={}&choice=join-{}",
         asked(&question),
         first.report.event_id.unwrap()
     );
@@ -281,7 +281,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
     let html = text(
         app.send(post_form(
             "/log",
-            "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00",
+            "room=1&level-1=5&action=log&observed_at=2026-01-15T12:00",
             Some(&cookie),
         ))
         .await,
@@ -295,7 +295,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
     assert!(html.contains(r#"value="new" class="secondary">New event</button>"#));
 
     let body = format!(
-        "room=1&level-1=5&action=smell&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
+        "room=1&level-1=5&action=log&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
     );
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=report"));
@@ -325,7 +325,7 @@ async fn after_an_auto_close_the_next_log_offers_a_continuation() {
     let html = text(
         app.send(post_form(
             "/log",
-            "room=1&level-1=5&action=smell",
+            "room=1&level-1=5&action=log",
             Some(&cookie),
         ))
         .await,
@@ -336,7 +336,7 @@ async fn after_an_auto_close_the_next_log_offers_a_continuation() {
         "{html}"
     );
     assert_eq!(asked(&html), format!("auto-closed-{id}"));
-    let body = format!("room=1&level-1=5&action=smell&asked=auto-closed-{id}&choice=continue-{id}");
+    let body = format!("room=1&level-1=5&action=log&asked=auto-closed-{id}&choice=continue-{id}");
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=report"));
     let event = app.store.event(id).await.unwrap().unwrap();
@@ -406,7 +406,7 @@ async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
         .await;
     let cookie = app.sign_in("daniele").await;
     let response = app
-        .send(post_form("/log", "pm25=35.5&action=pm25", Some(&cookie)))
+        .send(post_form("/log", "pm25=35.5&action=log", Some(&cookie)))
         .await;
     assert_eq!(location(&response), Some("/?saved=pm25"));
     let readings = app
@@ -418,7 +418,7 @@ async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
     assert_eq!(readings[0].value.ugm3(), 35.5);
 
     let with_smell = format!(
-        "room=1&level-1=6&pm25=40&action=smell&asked=same-smell-{0}&choice=join-{0}",
+        "room=1&level-1=6&pm25=40&action=log&asked=same-smell-{0}&choice=join-{0}",
         open.report.event_id.unwrap()
     );
     app.send(post_form("/log", &with_smell, Some(&cookie)))
@@ -433,13 +433,67 @@ async fn pm25_alone_needs_no_room_and_joins_the_open_event() {
     );
 }
 
+/// The Log button's decision table (look-and-feel spec §6.1) through the route.
+#[tokio::test]
+async fn the_log_button_saves_what_the_form_holds() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let bedroom = app.room("room-bedroom").await;
+    let cough = app.symptom("symptom-cough").await;
+    let cookie = app.sign_in("daniele").await;
+    let post = async |body: String| app.send(post_form("/log", &body, Some(&cookie))).await;
+
+    for (body, message) in [
+        (
+            format!("symptom={}&action=log", cough.0),
+            "Pick at least one room.",
+        ),
+        (
+            "note=acrid&pm25=12&action=log".to_owned(),
+            "Pick at least one room.",
+        ),
+        (
+            "action=log".to_owned(),
+            "Pick a room, or enter a PM2.5 value.",
+        ),
+    ] {
+        let response = post(body.clone()).await;
+        assert_eq!(
+            response.status(),
+            StatusCode::UNPROCESSABLE_ENTITY,
+            "{body}"
+        );
+        assert!(text(response).await.contains(message), "{body}");
+    }
+
+    let reading = post("pm25=21&action=log".to_owned()).await;
+    assert_eq!(location(&reading), Some("/?saved=pm25"));
+    let clean = post(format!("room={0}&level-{0}=0&action=log", bedroom.0)).await;
+    assert_eq!(location(&clean), Some("/?saved=no-smell"));
+    assert_eq!(
+        app.store.open_event().await.unwrap(),
+        None,
+        "a check opens no event"
+    );
+    let smell = post(format!(
+        "room={0}&level-{0}=4&pm25=30&action=log",
+        bedroom.0
+    ))
+    .await;
+    assert_eq!(location(&smell), Some("/?saved=report"));
+    let event = app.store.open_event().await.unwrap().expect("an event");
+    let readings = app.store.readings_for_event(event.id).await.unwrap();
+    assert_eq!(readings.len(), 1, "PM2.5 saved alongside the report");
+}
+
 #[tokio::test]
 async fn invalid_input_shows_the_form_again_with_what_was_typed() {
     let app = TestApp::new().await;
     app.add_user("daniele", Role::Resident, Locale::En).await;
     let bedroom = app.room("room-bedroom").await;
     let cookie = app.sign_in("daniele").await;
-    let body = format!("room={}&note=kept+as+typed&action=smell", bedroom.0);
+    let body = format!("room={}&note=kept+as+typed&action=log", bedroom.0);
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
     let html = text(response).await;
@@ -453,12 +507,12 @@ async fn invalid_input_shows_the_form_again_with_what_was_typed() {
         "{html}"
     );
     let missing = app
-        .send(post_form("/log", "action=pm25", Some(&cookie)))
+        .send(post_form("/log", "action=log", Some(&cookie)))
         .await;
     assert!(
         text(missing)
             .await
-            .contains("Enter a PM2.5 value to log it on its own.")
+            .contains("Pick a room, or enter a PM2.5 value.")
     );
     assert_eq!(app.store.open_event().await.unwrap(), None);
 }
@@ -471,7 +525,7 @@ async fn viewers_cannot_log() {
     let response = app
         .send(post_form(
             "/log",
-            "room=1&level-1=5&action=smell",
+            "room=1&level-1=5&action=log",
             Some(&cookie),
         ))
         .await;
@@ -518,7 +572,7 @@ async fn no_smell_with_an_event_open_joins_it_without_the_question() {
     let cookie = app.sign_in("guest").await;
     let bedroom = app.room("room-bedroom").await;
 
-    let body = format!("room={0}&level-{0}=0&action=smell", bedroom.0);
+    let body = format!("room={0}&level-{0}=0&action=log", bedroom.0);
     let response = app.send(post_form("/log", &body, Some(&cookie))).await;
     assert_eq!(location(&response), Some("/?saved=no-smell"));
     let open = app.store.open_event().await.unwrap().unwrap();
diff --git a/crates/web/tests/security.rs b/crates/web/tests/security.rs
index ab2acec..b5f1cac 100644
--- a/crates/web/tests/security.rs
+++ b/crates/web/tests/security.rs
@@ -196,14 +196,14 @@ async fn no_page_needs_inline_scripts_styles_or_eval_and_every_icon_exists() {
         .await;
     app.send(post_form(
         "/log",
-        "room=1&level-1=4&action=smell",
+        "room=1&level-1=4&action=log",
         Some(&cookie),
     ))
     .await;
     let question = app
         .send(post_form(
             "/log",
-            "room=1&level-1=5&action=smell",
+            "room=1&level-1=5&action=log",
             Some(&cookie),
         ))
         .await;
````

**Code** (proto `7ae8ced`):

````diff
diff --git a/crates/web/src/routes/entries.rs b/crates/web/src/routes/entries.rs
index f670049..50ee815 100644
--- a/crates/web/src/routes/entries.rs
+++ b/crates/web/src/routes/entries.rs
@@ -142,7 +142,8 @@ pub(crate) fn read_values(
 /// "No smell" has its own button. The one Log button decides from what is filled in (look-and-
 /// feel spec §6.1): rooms make a smell report, or a clean check when every level is 0, with the
 /// PM2.5 value saved alongside; without rooms a PM2.5 value alone is a reading, while symptoms
-/// or a note still need a room.
+/// or a note still need a room. Rooms ticked without any level count as no rooms next to a
+/// PM2.5 value: the Log screen ticks the rooms of the person's last report in the open event.
 fn read(
     form: &Pairs,
     rooms: &[Room],
@@ -166,15 +167,19 @@ fn read(
         })
         .transpose()?;
     let no_smell = form.get("action") == Some("no-smell");
-    let no_rooms = form.ids("room").ok_or("log-error-form")?.is_empty();
-    if no_rooms && !no_smell {
+    let picked = form.ids("room").ok_or("log-error-form")?;
+    let unrated = picked
+        .iter()
+        .all(|&room| level_field(form, RoomId(room)).is_none());
+    if unrated && !no_smell {
         let symptoms = form.ids("symptom").ok_or("log-error-form")?;
         let described = form.get("note").is_some() || !symptoms.is_empty();
-        return match pm25 {
-            _ if described => Err("log-error-rooms"),
-            Some(value) => Ok(Entry::Pm25Only { observed_at, value }),
-            None => Err("log-error-empty"),
-        };
+        match pm25 {
+            Some(value) if !described => return Ok(Entry::Pm25Only { observed_at, value }),
+            _ if !picked.is_empty() => {}
+            _ if described => return Err("log-error-rooms"),
+            _ => return Err("log-error-empty"),
+        }
     }
     let smell = if no_smell { Smell::None } else { Smell::Levels };
     let (offered_rooms, offered_symptoms) = offered(rooms, symptoms, (&[], &[]));
@@ -605,6 +610,22 @@ mod tests {
             Some("log-error-empty"),
             "no action: Log"
         );
+        let Ok(Entry::Pm25Only { .. }) =
+            read_form(&[("action", "log"), ("room", "1"), ("pm25", "18")])
+        else {
+            panic!("a ticked room without a level beside PM2.5: a reading");
+        };
+        for pairs in [
+            &[("action", "log"), ("room", "1")][..],
+            &[
+                ("action", "log"),
+                ("room", "1"),
+                ("pm25", "18"),
+                ("note", "acrid"),
+            ][..],
+        ] {
+            assert_eq!(read_form(pairs).err(), Some("log-error-level"), "{pairs:?}");
+        }
     }
 
     #[test]
````

### Task 6: Event banner with report count and the highest level's room

**Proto commit:** `f1ac0c4` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show f1ac0c4`)

Spec §5 item 2, §6.2. `log::peak` (pure, unit-tested tie rule), `Banner { since, detail }` replaces `Status`; `Tr::t2` (two arguments; the key scan learns `tr.t2(`). Keys `event-banner-title/-since/-peak` replace `event-open-since` and `event-none` (no banner without an open event).

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/i18n.rs
M	crates/web/src/routes/log.rs
M	crates/web/templates/log.html
M	crates/web/tests/log.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/src/routes/log.rs (tests)`, `crates/web/tests/log.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **615 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(log): the event banner counts the reports and names the highest level and its room

The Log screen's amber banner (look-and-feel spec §6.2) shows when the open event started,
how many of its reports the viewer may see and the highest room level among them with that
room; on a tie the most recently reported room wins. A pure peak function is unit-tested;
Tr::t2 translates a message with two arguments. Without an open event there is no banner.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `f1ac0c4`):

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index bf4e576..b5e1262 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -14,8 +14,9 @@ logout = Sign out
 account-menu = Account: { $username }
 
 log-title = Log a smell
-event-none = No smell event in progress.
-event-open-since = Smell event in progress since { $time }.
+event-banner-title = Smell event in progress
+event-banner-since = since { $time }
+event-banner-peak = highest level { $level } ({ $room })
 log-intensity = Intensity
 log-how-to-rate = How to rate
 log-rooms = Rooms
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index bf786da..8ec643f 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -14,8 +14,9 @@ logout = Esci
 account-menu = Account: { $username }
 
 log-title = Registra un odore
-event-none = Nessun evento di fumo in corso.
-event-open-since = Evento di fumo in corso dalle { $time }.
+event-banner-title = Evento di odore in corso
+event-banner-since = dalle { $time }
+event-banner-peak = livello massimo { $level } ({ $room })
 log-intensity = Intensità
 log-how-to-rate = Come valutare
 log-rooms = Stanze
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 1c91099..e92408f 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -14,8 +14,9 @@ logout = Çıkış yap
 account-menu = Hesap: { $username }
 
 log-title = Koku kaydet
-event-none = Devam eden bir koku olayı yok.
-event-open-since = { $time } saatinden beri süren bir koku olayı var.
+event-banner-title = Koku olayı sürüyor
+event-banner-since = { $time } itibarıyla
+event-banner-peak = en yüksek seviye { $level } ({ $room })
 log-intensity = Yoğunluk
 log-how-to-rate = Nasıl puanlanır
 log-rooms = Odalar
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index f1c78ae..f79c5a0 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -74,10 +74,24 @@ impl Tr {
 
     /// A message with one argument, e.g. `tr.t1("signed-in-as", "username", name)`.
     pub(crate) fn t1(&self, key: &str, name: &str, value: &str) -> String {
-        let args: HashMap<Cow<'static, str>, FluentValue> = HashMap::from([(
-            Cow::Owned(name.to_owned()),
-            FluentValue::from(value.to_owned()),
-        )]);
+        self.with_args(key, &[(name, value)])
+    }
+
+    /// A message with two arguments, each a `(name, value)` pair.
+    pub(crate) fn t2(&self, key: &str, first: (&str, &str), second: (&str, &str)) -> String {
+        self.with_args(key, &[first, second])
+    }
+
+    fn with_args(&self, key: &str, args: &[(&str, &str)]) -> String {
+        let args: HashMap<Cow<'static, str>, FluentValue> = args
+            .iter()
+            .map(|&(name, value)| {
+                (
+                    Cow::Owned(name.to_owned()),
+                    FluentValue::from(value.to_owned()),
+                )
+            })
+            .collect();
         LOCALES.lookup_with_args(language(self.0), key, &args)
     }
 
@@ -174,7 +188,7 @@ mod tests {
             .collect()
     }
 
-    /// Every string literal passed as the key to `tr.t`, `tr.t1` or `tr.n` in the templates
+    /// Every string literal passed as the key to `tr.t`, `tr.t1`, `tr.t2` or `tr.n` in the templates
     /// and the Rust sources.
     fn literal_keys() -> BTreeSet<String> {
         fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
@@ -194,7 +208,7 @@ mod tests {
         let mut keys = BTreeSet::new();
         for path in paths {
             let text = fs::read_to_string(path).unwrap();
-            for call in ["tr.t(\"", "tr.t1(\"", "tr.n(\""] {
+            for call in ["tr.t(\"", "tr.t1(\"", "tr.t2(\"", "tr.n(\""] {
                 for (start, _) in text.match_indices(call) {
                     let rest = &text[start + call.len()..];
                     keys.insert(rest[..rest.find('"').unwrap()].to_owned());
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index 6af18c1..a37663b 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -5,9 +5,10 @@ use axum::extract::{Query, State};
 use axum::http::StatusCode;
 use axum::response::Response;
 use chrono::{DateTime, Utc};
+use chrono_tz::Tz;
 use serde::Deserialize;
 use smoke_domain::permissions::{Action, can, can_see};
-use smoke_domain::records::{Room, Symptom, User};
+use smoke_domain::records::{Event, Room, RoomLevel, SmellReport, Symptom, User};
 use smoke_domain::repos::Store;
 use smoke_domain::starter::BEDROOM_KEY;
 use smoke_domain::summary::{MonthSummary, summarise};
@@ -138,14 +139,48 @@ pub(crate) fn fields(
     }
 }
 
-/// What the Log screen says about the open event.
-pub(crate) enum Status {
-    /// No event is open.
-    None,
-    /// The viewer may see the open event, which started at this time.
-    Open(String),
-    /// An event is open but the viewer may not know about it, so the screen says nothing.
-    Hidden,
+/// The banner of the open event (look-and-feel spec §5, §6.2), for those who may open it.
+pub(crate) struct Banner {
+    /// When the event started.
+    since: String,
+    /// "3 reports · highest level 6 (Bedroom)", over the reports the viewer may see.
+    detail: String,
+}
+
+/// The highest level any room reached in `reports`, with that room. On a tie the room of the
+/// most recent report wins (by observed time, then recorded time), and within one report the
+/// later room. `None` while every level is 0.
+pub(crate) fn peak<'a>(reports: impl IntoIterator<Item = &'a SmellReport>) -> Option<RoomLevel> {
+    reports
+        .into_iter()
+        .flat_map(|report| {
+            let when = (report.observed_at, report.recorded_at);
+            report.values.rooms().iter().map(move |&room| (room, when))
+        })
+        .filter(|(room, _)| !room.level.is_clean())
+        .max_by_key(|&(room, when)| (room.level, when))
+        .map(|(room, _)| room)
+}
+
+fn banner(
+    tr: Tr,
+    event: &Event,
+    visible: &[&SmellReport],
+    rooms: &[Room],
+    now: DateTime<Utc>,
+    tz: Tz,
+) -> Banner {
+    let mut detail = tr.n("summary-reports", &visible.len());
+    if let Some(top) = peak(visible.iter().copied()) {
+        let room = labels::room_by_id(tr, rooms, top.room);
+        let level = top.level.get().to_string();
+        detail.push_str(" · ");
+        detail.push_str(&tr.t2("event-banner-peak", ("level", &level), ("room", &room)));
+    }
+    Banner {
+        since: time::since(tr.0, event.opened_at, now, tz),
+        detail,
+    }
 }
 
 /// The ids a submitted form may name: every visible room and symptom, plus those in `keep` (a
@@ -184,7 +219,8 @@ pub(crate) struct FormView {
 pub(crate) struct LogPage {
     tr: Tr,
     signed_in: Option<SignedIn>,
-    status: Status,
+    /// `None` when no event is open, or the viewer may not know about it.
+    banner: Option<Banner>,
     /// The open event, when the user may close it.
     close_event: Option<i64>,
     /// "The event from … was auto-closed", shown while another event is open (spec §7.4).
@@ -297,11 +333,13 @@ pub(crate) async fn log_page<S: Store>(
         close_event: shown
             .filter(|e| can(&actor, Action::ManageEvent { owner: e.owner_id }, now))
             .map(|e| e.id.0),
-        status: match (&open, shown) {
-            (None, _) => Status::None,
-            (Some(_), Some(e)) => Status::Open(time::since(tr.0, e.opened_at, now, schedule.tz())),
-            (Some(_), None) => Status::Hidden,
-        },
+        banner: shown.map(|event| {
+            let visible: Vec<&SmellReport> = open_reports
+                .iter()
+                .filter(|r| can_see(&actor, r.user_id, now))
+                .collect();
+            banner(tr, event, &visible, &rooms, now, schedule.tz())
+        }),
         notice,
         saved,
         summary,
@@ -309,3 +347,85 @@ pub(crate) async fn log_page<S: Store>(
         form,
     })
 }
+
+#[cfg(test)]
+mod tests {
+    use smoke_domain::records::ReportValues;
+    use smoke_domain::types::{EventId, Level, ReportId, UserId};
+
+    use super::*;
+
+    fn report(observed: &str, recorded: &str, rooms: &[(i64, u8)]) -> SmellReport {
+        SmellReport {
+            id: ReportId(1),
+            event_id: Some(EventId(1)),
+            user_id: UserId(1),
+            observed_at: observed.parse().unwrap(),
+            recorded_at: recorded.parse().unwrap(),
+            values: ReportValues::new(
+                rooms
+                    .iter()
+                    .map(|&(room, level)| RoomLevel {
+                        room: RoomId(room),
+                        level: Level::new(level).unwrap(),
+                    })
+                    .collect(),
+                vec![],
+                None,
+            )
+            .unwrap(),
+            deleted_at: None,
+        }
+    }
+
+    fn top(reports: &[SmellReport]) -> Option<(i64, u8)> {
+        peak(reports).map(|room| (room.room.0, room.level.get()))
+    }
+
+    #[test]
+    fn the_peak_is_the_highest_room_level_of_the_event() {
+        let reports = [
+            report(
+                "2026-01-15T18:00:00Z",
+                "2026-01-15T18:00:00Z",
+                &[(1, 3), (2, 6)],
+            ),
+            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(3, 4)]),
+        ];
+        assert_eq!(top(&reports), Some((2, 6)));
+    }
+
+    #[test]
+    fn on_a_tie_the_most_recently_reported_room_wins() {
+        let reports = [
+            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(2, 6)]),
+            report("2026-01-15T18:00:00Z", "2026-01-15T20:00:00Z", &[(1, 6)]),
+        ];
+        assert_eq!(
+            top(&reports),
+            Some((2, 6)),
+            "by observed time, not list order"
+        );
+        let same_time = [
+            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(1, 6)]),
+            report("2026-01-15T19:00:00Z", "2026-01-15T19:30:00Z", &[(3, 6)]),
+        ];
+        assert_eq!(top(&same_time), Some((3, 6)), "then by recorded time");
+        let one_report = [report(
+            "2026-01-15T19:00:00Z",
+            "2026-01-15T19:00:00Z",
+            &[(1, 6), (4, 6)],
+        )];
+        assert_eq!(top(&one_report), Some((4, 6)), "then the later room");
+    }
+
+    #[test]
+    fn an_event_at_zero_or_without_rooms_has_no_peak() {
+        let reports = [
+            report("2026-01-15T19:00:00Z", "2026-01-15T19:00:00Z", &[(1, 0)]),
+            report("2026-01-15T19:10:00Z", "2026-01-15T19:10:00Z", &[]),
+        ];
+        assert_eq!(top(&reports), None);
+        assert_eq!(top(&[]), None);
+    }
+}
diff --git a/crates/web/templates/log.html b/crates/web/templates/log.html
index 588bf5b..b43ce31 100644
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -11,19 +11,17 @@
   <button type="submit" class="link">{{ tr.t("notice-dismiss") }}</button>
 </form>
 {% endif %}
-<div class="status">
-  {% match status %}
-  {% when Status::Open with (since) %}<p>{{ tr.t1("event-open-since", "time", since) }}</p>
-  {% when Status::None %}<p>{{ tr.t("event-none") }}</p>
-  {% when Status::Hidden %}
-  {% endmatch %}
+{% if let Some(banner) = banner %}
+<section class="banner">
+  <p><strong>{{ tr.t("event-banner-title") }}</strong> {{ tr.t1("event-banner-since", "time", banner.since) }}<br>{{ banner.detail }}</p>
   {% if let Some(id) = close_event %}
   <form method="post" action="/events/{{ id }}/close">
     <input type="hidden" name="return_to" value="log">
-    <button type="submit" class="secondary">{{ tr.t("event-close") }}</button>
+    <button type="submit" class="button">{{ tr.t("event-close") }}</button>
   </form>
   {% endif %}
-</div>
+</section>
+{% endif %}
 <p class="summary">
   {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %} · {{ tr.n("summary-checks", summary.checks) }}
 </p>
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index 819b299..ded99e3 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -23,7 +23,10 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
     assert_eq!(response.status(), StatusCode::OK);
     let html = text(response).await;
     assert!(html.contains("<h1>Log a smell</h1>"), "{html}");
-    assert!(html.contains("No smell event in progress."), "{html}");
+    assert!(
+        !Page::parse(&html).exists(".banner"),
+        "no event, no banner: {html}"
+    );
     assert!(
         html.contains(r#"<form method="post" action="/log""#),
         "{html}"
@@ -80,10 +83,15 @@ async fn with_an_event_open_the_form_offers_an_update_with_my_last_rooms() {
     let cookie = app.sign_in("daniele").await;
 
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(
-        html.contains("Smell event in progress since 19:05."),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.text(".banner > p"),
+        "Smell event in progress since 19:05 2 reports · highest level 6 (Kitchen)"
     );
+    assert!(page.exists(&format!(
+        r#".banner form[action="/events/{}/close"] button"#,
+        first.report.event_id.unwrap().0
+    )));
     assert!(
         html.contains(">Update: situation changed</button>"),
         "{html}"
@@ -137,13 +145,10 @@ async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
 
     let cookie = app.sign_in("guest").await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    for hidden in [
-        "since 19:05",
-        "Update: situation changed",
-        "No smell event in progress.",
-    ] {
+    for hidden in ["since 19:05", "Update: situation changed"] {
         assert!(!html.contains(hidden), "{hidden}: {html}");
     }
+    assert!(!Page::parse(&html).exists(".banner"), "{html}");
     assert!(html.contains(">Log</button>"), "{html}");
 
     app.log(
@@ -155,9 +160,10 @@ async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
     )
     .await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(
-        html.contains("Smell event in progress since 19:05."),
-        "{html}"
+    assert_eq!(
+        Page::parse(&html).text(".banner > p"),
+        "Smell event in progress since 19:05 1 report · highest level 3 (Living room)",
+        "the guest's own report only"
     );
     assert!(
         html.contains(">Update: situation changed</button>"),
@@ -172,10 +178,9 @@ async fn guests_see_the_open_event_only_if_they_own_it_or_reported_in_it() {
     let cookie = app.sign_in("stranger").await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
     assert!(!html.contains("since 19:05"), "{html}");
-    assert!(!html.contains("No smell event in progress."), "{html}");
     let cookie = app.sign_in("guest").await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(html.contains("since 19:05."), "{html}");
+    assert!(html.contains("since 19:05"), "{html}");
     assert!(!html.contains(r#"action="/log""#), "{html}");
 }
 
@@ -246,7 +251,7 @@ async fn opening_the_log_screen_runs_the_auto_close_sweep() {
     app.at("2026-01-16T07:00:00Z");
 
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(html.contains("No smell event in progress."), "{html}");
+    assert!(!Page::parse(&html).exists(".banner"), "auto-closed: {html}");
     let event = app
         .store
         .event(logged.report.event_id.unwrap())
````

### Task 7: Log screen and correction form as cards

**Proto commit:** `6e4b0db` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 6e4b0db`)

Spec §5. `report_fields.html` becomes macros (`room`, `rooms`, `symptoms`, `rating_sheet`); `room_level.html` and `rating_dialog.html` are deleted. `RoomChoice { room, legend, picked }` and `Fields.levels` (the guide rows) let each level carry its name and description, shown when checked by CSS (`.level { display: contents }`, text on grid row 3). Month tiles are a `<dl>` with plural label keys `tile-*`. CSS 64–66. Keys: `log-level-hint`, `optional`, `action-add`, `summary-caption`, `tile-*`; `log-note` becomes "Note".

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/64-sheet.css
A	crates/web/css/65-stats.css
A	crates/web/css/66-room.css
D	crates/web/templates/rating_dialog.html
D	crates/web/templates/room_level.html
M	crates/web/css/51-card.css
M	crates/web/css/54-field.css
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/assets.rs
M	crates/web/src/i18n.rs
M	crates/web/src/routes/log.rs
M	crates/web/src/routes/reports.rs
M	crates/web/templates/correct.html
M	crates/web/templates/log.html
M	crates/web/templates/report_fields.html
M	crates/web/tests/log.rs
M	crates/web/tests/logging.rs
M	crates/web/tests/reports.rs
M	crates/web/tests/settings.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/log.rs`, `crates/web/tests/logging.rs`, `crates/web/tests/reports.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **615 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(log): the Log screen as soft cards, with a level grid that names the picked level

Log a smell is now the banner, four month tiles, and cards: Time and Note fold away
(<details>), Rooms lists one line per room with a check square that opens its 6 + 5 level
grid, where each level carries its name and description, shown once picked (CSS :has, no
script); Symptoms are chips; PM2.5 sits on one line. The correction form shares the cards
through report_fields.html macros, and How to rate opens as a bottom sheet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `6e4b0db`):

````diff
diff --git a/crates/web/css/51-card.css b/crates/web/css/51-card.css
index 08cb0e9..fb6f278 100644
--- a/crates/web/css/51-card.css
+++ b/crates/web/css/51-card.css
@@ -10,6 +10,7 @@
   }
 
   .collapsible {
+    display: block;
     padding-block: var(--space-1);
   }
 
@@ -40,7 +41,7 @@
     transform: rotate(180deg);
   }
 
-  .collapsible[open] > .stack {
+  .collapsible > .stack {
     padding-block-end: var(--space-3);
   }
 }
diff --git a/crates/web/css/54-field.css b/crates/web/css/54-field.css
index 72e1eae..a1fec02 100644
--- a/crates/web/css/54-field.css
+++ b/crates/web/css/54-field.css
@@ -15,6 +15,8 @@
     align-items: center;
     justify-content: space-between;
     gap: var(--space-3);
+    font-size: 1rem;
+    font-weight: 400;
   }
 
   .inline-field input {
diff --git a/crates/web/css/64-sheet.css b/crates/web/css/64-sheet.css
new file mode 100644
index 0000000..7993957
--- /dev/null
+++ b/crates/web/css/64-sheet.css
@@ -0,0 +1,61 @@
+/* .sheet: a <dialog> that rises from the bottom of the screen over a dimmed page. */
+@layer components {
+  .sheet {
+    inline-size: 100%;
+    max-inline-size: var(--column);
+    max-block-size: 85vh;
+    margin: auto auto 0;
+    padding: var(--space-3) var(--space-4) calc(var(--space-5) + env(safe-area-inset-bottom));
+    border: 0;
+    border-radius: var(--radius-card) var(--radius-card) 0 0;
+    background: var(--card);
+    color: var(--text);
+  }
+
+  .sheet[open] {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-3);
+  }
+
+  .sheet::before {
+    content: "";
+    align-self: center;
+    inline-size: 2.5rem;
+    block-size: 0.3125rem;
+    border-radius: var(--radius-pill);
+    background: var(--line);
+  }
+
+  .sheet::backdrop {
+    background: var(--backdrop);
+  }
+
+  .scale th,
+  .scale td {
+    padding-block: var(--space-1);
+    vertical-align: middle;
+  }
+
+  .scale th {
+    inline-size: 3rem;
+  }
+
+  .scale .badge {
+    inline-size: 2rem;
+    block-size: 2rem;
+  }
+
+  @media (prefers-reduced-motion: no-preference) {
+    .sheet[open] {
+      animation: rise 0.2s ease-out;
+    }
+
+    @keyframes rise {
+      from {
+        transform: translateY(2rem);
+        opacity: 0;
+      }
+    }
+  }
+}
diff --git a/crates/web/css/65-stats.css b/crates/web/css/65-stats.css
new file mode 100644
index 0000000..a796313
--- /dev/null
+++ b/crates/web/css/65-stats.css
@@ -0,0 +1,36 @@
+/* .stats: the month's numbers as small tiles, number above its label. */
+@layer components {
+  .stats {
+    display: grid;
+    grid-auto-columns: minmax(0, 1fr);
+    grid-auto-flow: column;
+    gap: var(--space-2);
+  }
+
+  .stats > div {
+    display: flex;
+    flex-direction: column-reverse;
+    padding: var(--space-3) var(--space-2);
+    border-radius: var(--radius-field);
+    background: var(--card);
+    text-align: center;
+  }
+
+  .stats dd {
+    font-size: var(--text-brand);
+    font-weight: 700;
+  }
+
+  .stats dt {
+    color: var(--muted);
+    font-size: var(--text-tiny);
+    line-height: 1.25;
+  }
+
+  .stats-caption {
+    margin-block-start: calc(-1 * var(--space-2));
+    padding-inline: var(--space-1);
+    color: var(--muted);
+    font-size: var(--text-small);
+  }
+}
diff --git a/crates/web/css/66-room.css b/crates/web/css/66-room.css
new file mode 100644
index 0000000..c0cdc27
--- /dev/null
+++ b/crates/web/css/66-room.css
@@ -0,0 +1,151 @@
+/* The Rooms card of the Log form: one line per room with a check square; a selected room is
+   highlighted and opens its level grid 0–10 (6 + 5), where each level carries its name and
+   description, shown under the grid once picked. */
+@layer components {
+  .rooms {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-2);
+  }
+
+  .group {
+    margin-block-start: var(--space-2);
+    color: var(--muted);
+    font-size: var(--text-small);
+    font-weight: 600;
+    letter-spacing: 0.06em;
+    text-transform: uppercase;
+  }
+
+  .info {
+    display: inline-flex;
+    align-items: center;
+    gap: var(--space-1);
+    min-block-size: var(--target);
+    font-size: var(--text-label);
+    font-weight: 600;
+    text-decoration: none;
+  }
+
+  .room {
+    border: 1px solid var(--line);
+    border-radius: 14px;
+  }
+
+  .room:has(> .room-check input:checked) {
+    border-color: var(--accent);
+    background: var(--accent-soft);
+  }
+
+  .room-check {
+    display: flex;
+    align-items: center;
+    gap: var(--space-3);
+    min-block-size: var(--field-height);
+    padding-inline: var(--space-3);
+    font-size: 1rem;
+    font-weight: 550;
+    cursor: pointer;
+  }
+
+  .room-check > input,
+  .level > input {
+    position: absolute;
+    opacity: 0;
+    pointer-events: none;
+  }
+
+  .box {
+    display: grid;
+    place-items: center;
+    inline-size: 1.5rem;
+    block-size: 1.5rem;
+    border: 2px solid var(--control-border);
+    border-radius: 8px;
+    color: transparent;
+  }
+
+  .room-check > input:checked + .box {
+    border-color: var(--accent);
+    background: var(--accent);
+    color: var(--on-accent);
+  }
+
+  .room-check > input:focus-visible + .box,
+  .level > input:focus-visible + span {
+    outline: 2px solid var(--accent);
+    outline-offset: 2px;
+  }
+
+  .box .icon {
+    inline-size: 0.875rem;
+    block-size: 0.875rem;
+  }
+
+  .levels {
+    display: grid;
+    grid-template-columns: repeat(6, minmax(0, 1fr));
+    gap: 0.375rem;
+    padding: 0 var(--space-3) var(--space-3);
+  }
+
+  .room:not(:has(> .room-check input:checked)) > .levels {
+    display: none;
+  }
+
+  /* The label is no box of its own: its number is a grid cell, its text the line below. */
+  .level {
+    display: contents;
+  }
+
+  .level > span:first-of-type {
+    display: grid;
+    place-items: center;
+    min-block-size: var(--target);
+    border: 1px solid var(--line);
+    border-radius: 10px;
+    background: var(--card);
+    font-size: 0.9375rem;
+    font-weight: 650;
+    cursor: pointer;
+  }
+
+  .level > input:checked + span {
+    border-color: var(--accent);
+    background: var(--accent);
+    color: var(--on-accent);
+  }
+
+  .level-text,
+  .level-hint {
+    grid-column: 1 / -1;
+    grid-row: 3;
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+
+  .level-text {
+    display: none;
+  }
+
+  .level-text > strong {
+    color: var(--text);
+  }
+
+  .level > input:checked ~ .level-text {
+    display: block;
+  }
+
+  .levels:has(input:checked) > .level-hint {
+    display: none;
+  }
+
+  .unit {
+    display: inline-flex;
+    align-items: center;
+    gap: var(--space-2);
+    color: var(--muted);
+    font-size: var(--text-label);
+    white-space: nowrap;
+  }
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index b5e1262..ab6444c 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -24,11 +24,10 @@ log-level-in = Level in { $room }
 log-inside = Inside
 log-common = Common areas
 log-symptoms = Symptoms
-log-pm25 = PM2.5 (µg/m³, optional)
 log-time = Time
 log-time-now = now
-log-time-change = change
-log-note = Note (optional)
+log-time-change = Change
+log-note = Note
 log-submit = Log
 log-update = Update: situation changed
 log-no-smell = No smell
@@ -70,22 +69,29 @@ refused-future = The time cannot be in the future.
 refused-close-before-last-report = The end cannot be before the last report.
 refused-close-in-future = The end cannot be in the future.
 
-summary-this-month = This month:
-summary-events = { $count ->
-    [one] 1 event
-   *[other] { $count } events
-}
 summary-reports = { $count ->
     [one] 1 report
    *[other] { $count } reports
 }
-summary-bedroom-days = { $count ->
-    [one] 1 day with bedroom smoke
-   *[other] { $count } days with bedroom smoke
+log-level-hint = Pick a level from 0 to 10.
+optional = optional
+action-add = Add
+summary-caption = This month
+tile-events = { $count ->
+    [one] event
+   *[other] events
+}
+tile-reports = { $count ->
+    [one] report
+   *[other] reports
+}
+tile-bedroom-days = { $count ->
+    [one] bedroom day
+   *[other] bedroom days
 }
-summary-checks = { $count ->
-    [one] 1 clean check
-   *[other] { $count } clean checks
+tile-checks = { $count ->
+    [one] clean check
+   *[other] clean checks
 }
 
 rating-title = How to rate
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 8ec643f..8bd08d1 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -24,11 +24,10 @@ log-level-in = Livello in { $room }
 log-inside = In casa
 log-common = Parti comuni
 log-symptoms = Sintomi
-log-pm25 = PM2.5 (µg/m³, facoltativo)
 log-time = Ora
 log-time-now = adesso
-log-time-change = cambia
-log-note = Nota (facoltativa)
+log-time-change = Cambia
+log-note = Nota
 log-submit = Registra
 log-update = Aggiorna: la situazione è cambiata
 log-no-smell = Nessun odore
@@ -70,22 +69,29 @@ refused-future = L'ora non può essere nel futuro.
 refused-close-before-last-report = La fine non può essere prima dell'ultima segnalazione.
 refused-close-in-future = La fine non può essere nel futuro.
 
-summary-this-month = Questo mese:
-summary-events = { $count ->
-    [one] 1 evento
-   *[other] { $count } eventi
-}
 summary-reports = { $count ->
     [one] 1 segnalazione
    *[other] { $count } segnalazioni
 }
-summary-bedroom-days = { $count ->
-    [one] 1 giorno con fumo in camera da letto
-   *[other] { $count } giorni con fumo in camera da letto
+log-level-hint = Scegli un livello da 0 a 10.
+optional = facoltativo
+action-add = Aggiungi
+summary-caption = Questo mese
+tile-events = { $count ->
+    [one] evento
+   *[other] eventi
+}
+tile-reports = { $count ->
+    [one] segnalazione
+   *[other] segnalazioni
+}
+tile-bedroom-days = { $count ->
+    [one] giorno con fumo in camera
+   *[other] giorni con fumo in camera
 }
-summary-checks = { $count ->
-    [one] 1 controllo pulito
-   *[other] { $count } controlli puliti
+tile-checks = { $count ->
+    [one] controllo pulito
+   *[other] controlli puliti
 }
 
 rating-title = Come valutare
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index e92408f..e903bd2 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -24,11 +24,10 @@ log-level-in = { $room } seviyesi
 log-inside = Ev içi
 log-common = Ortak alanlar
 log-symptoms = Belirtiler
-log-pm25 = PM2.5 (µg/m³, isteğe bağlı)
 log-time = Saat
 log-time-now = şimdi
-log-time-change = değiştir
-log-note = Not (isteğe bağlı)
+log-time-change = Değiştir
+log-note = Not
 log-submit = Kaydet
 log-update = Güncelle: durum değişti
 log-no-smell = Koku yok
@@ -70,22 +69,29 @@ refused-future = Saat gelecekte olamaz.
 refused-close-before-last-report = Bitiş, son bildirimden önce olamaz.
 refused-close-in-future = Bitiş gelecekte olamaz.
 
-summary-this-month = Bu ay:
-summary-events = { $count ->
-    [one] 1 olay
-   *[other] { $count } olay
-}
 summary-reports = { $count ->
     [one] 1 bildirim
    *[other] { $count } bildirim
 }
-summary-bedroom-days = { $count ->
-    [one] yatak odasında duman olan 1 gün
-   *[other] yatak odasında duman olan { $count } gün
+log-level-hint = 0 ile 10 arasında bir seviye seç.
+optional = isteğe bağlı
+action-add = Ekle
+summary-caption = Bu ay
+tile-events = { $count ->
+    [one] olay
+   *[other] olay
+}
+tile-reports = { $count ->
+    [one] bildirim
+   *[other] bildirim
+}
+tile-bedroom-days = { $count ->
+    [one] yatak odasında dumanlı gün
+   *[other] yatak odasında dumanlı gün
 }
-summary-checks = { $count ->
-    [one] 1 temiz kontrol
-   *[other] { $count } temiz kontrol
+tile-checks = { $count ->
+    [one] temiz kontrol
+   *[other] temiz kontrol
 }
 
 rating-title = Nasıl puanlanır
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 730e63a..2a29f47 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -56,6 +56,9 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "61-menu.css",
     "62-tabbar.css",
     "63-install.css",
+    "64-sheet.css",
+    "65-stats.css",
+    "66-room.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index f79c5a0..e2eb4c7 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -95,7 +95,7 @@ impl Tr {
         LOCALES.lookup_with_args(language(self.0), key, &args)
     }
 
-    /// A message that depends on a number, e.g. `tr.n("summary-events", &3)`: Fluent picks the
+    /// A message that depends on a number, e.g. `tr.n("summary-reports", &3)`: Fluent picks the
     /// language's plural form from `$count`. Takes a reference because templates pass one.
     pub(crate) fn n(&self, key: &str, count: &usize) -> String {
         let args: HashMap<Cow<'static, str>, FluentValue> =
@@ -336,10 +336,10 @@ mod tests {
 
     #[test]
     fn counts_pick_the_plural_form_of_each_language() {
-        assert_eq!(Tr(Locale::En).n("summary-events", &1), "1 event");
-        assert_eq!(Tr(Locale::En).n("summary-events", &14), "14 events");
+        assert_eq!(Tr(Locale::En).n("tile-events", &1), "event");
+        assert_eq!(Tr(Locale::En).n("tile-events", &14), "events");
         assert_eq!(Tr(Locale::It).n("summary-reports", &1), "1 segnalazione");
         assert_eq!(Tr(Locale::It).n("summary-reports", &41), "41 segnalazioni");
-        assert_eq!(Tr(Locale::Tr).n("summary-events", &14), "14 olay");
+        assert_eq!(Tr(Locale::Tr).n("summary-reports", &14), "14 bildirim");
     }
 }
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index a37663b..768bcb2 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -13,7 +13,7 @@ use smoke_domain::repos::Store;
 use smoke_domain::starter::BEDROOM_KEY;
 use smoke_domain::summary::{MonthSummary, summarise};
 use smoke_domain::sweep::auto_close;
-use smoke_domain::types::{Level, RoomArea, RoomId, SymptomId};
+use smoke_domain::types::{RoomArea, RoomId, SymptomId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
@@ -74,10 +74,18 @@ pub(crate) struct Choice {
     pub(crate) checked: bool,
 }
 
-/// A room chip and the row of levels it shows while selected.
+/// A room's line and the level grid it opens while selected.
 pub(crate) struct RoomChoice {
     pub(crate) room: Choice,
-    pub(crate) levels: Vec<Choice>,
+    /// "Level in Bedroom", for screen readers.
+    pub(crate) legend: String,
+    pub(crate) picked: Option<u8>,
+}
+
+impl RoomChoice {
+    pub(crate) fn is_picked(&self, level: &u8) -> bool {
+        self.picked == Some(*level)
+    }
 }
 
 /// The room, level and symptom fields shared by the Log form and the correction form
@@ -86,6 +94,8 @@ pub(crate) struct Fields {
     pub(crate) inside: Vec<RoomChoice>,
     pub(crate) common: Vec<RoomChoice>,
     pub(crate) symptoms: Vec<Choice>,
+    /// The levels 0–10 with their names and descriptions, offered for each room.
+    pub(crate) levels: Vec<GuideRow>,
 }
 
 /// The choices for `draft`. Hidden rooms and symptoms are offered only if listed in `keep`
@@ -102,24 +112,19 @@ pub(crate) fn fields(
             .iter()
             .filter(|r| r.area == area && (r.hidden_at.is_none() || keep.0.contains(&r.id)))
             .map(|r| {
-                let picked = draft
-                    .levels
-                    .iter()
-                    .find(|(room, _)| *room == r.id)
-                    .map(|&(_, level)| level);
+                let label = labels::room(tr, r);
                 RoomChoice {
+                    legend: tr.t1("log-level-in", "room", &label),
+                    picked: draft
+                        .levels
+                        .iter()
+                        .find(|(room, _)| *room == r.id)
+                        .map(|&(_, level)| level),
                     room: Choice {
                         value: r.id.0,
-                        label: labels::room(tr, r),
+                        label,
                         checked: draft.rooms.contains(&r.id),
                     },
-                    levels: (Level::MIN..=Level::MAX)
-                        .map(|value| Choice {
-                            value: value.into(),
-                            label: value.to_string(),
-                            checked: picked == Some(value),
-                        })
-                        .collect(),
                 }
             })
             .collect()
@@ -136,6 +141,7 @@ pub(crate) fn fields(
                 checked: draft.symptoms.contains(&s.id),
             })
             .collect(),
+        levels: levels(tr),
     }
 }
 
@@ -214,6 +220,17 @@ pub(crate) struct FormView {
     error: Option<&'static str>,
 }
 
+impl FormView {
+    /// "now" beside "Time" while no time is typed.
+    fn time_hint(&self, tr: &Tr) -> String {
+        if self.observed_at.is_empty() {
+            tr.t("log-time-now")
+        } else {
+            String::new()
+        }
+    }
+}
+
 #[derive(Template)]
 #[template(path = "log.html")]
 pub(crate) struct LogPage {
@@ -228,7 +245,6 @@ pub(crate) struct LogPage {
     /// A translation key confirming what was just saved.
     saved: Option<&'static str>,
     summary: MonthSummary,
-    levels: Vec<GuideRow>,
     /// `None` for people who may not log (viewers, expired guests).
     form: Option<FormView>,
 }
@@ -343,7 +359,6 @@ pub(crate) async fn log_page<S: Store>(
         notice,
         saved,
         summary,
-        levels: levels(tr),
         form,
     })
 }
diff --git a/crates/web/src/routes/reports.rs b/crates/web/src/routes/reports.rs
index 45f9963..8aa7dbf 100644
--- a/crates/web/src/routes/reports.rs
+++ b/crates/web/src/routes/reports.rs
@@ -20,7 +20,6 @@ use crate::forms::Pairs;
 use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::entries::{Smell, joinable_events, read_time, read_values};
-use crate::routes::guide::{GuideRow, levels};
 use crate::routes::log::{Draft, Fields, fields, offered};
 use crate::routes::refusal;
 use crate::state::AppState;
@@ -290,7 +289,6 @@ struct CorrectPage {
     tr: Tr,
     signed_in: Option<SignedIn>,
     id: i64,
-    levels: Vec<GuideRow>,
     /// A no-smell check: its levels stay at 0.
     check: bool,
     form: CorrectForm,
@@ -335,7 +333,6 @@ async fn correct_page<S: Store>(
         tr,
         signed_in: SignedIn::of(user, state.clock.now(), Tab::Events),
         id: report.id.0,
-        levels: levels(tr),
         check: report.event_id.is_none(),
         form: CorrectForm {
             fields: fields(tr, &rooms, &symptoms, &draft, keep),
diff --git a/crates/web/templates/correct.html b/crates/web/templates/correct.html
index 02a0ed7..8519973 100644
--- a/crates/web/templates/correct.html
+++ b/crates/web/templates/correct.html
@@ -1,24 +1,25 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
+{% import "report_fields.html" as f %}
 {% block title %}{{ tr.t("report-correct") }}{% endblock %}
 {% block content %}
+{% call c::back("/reports/{}"|format(id), tr.t("report-title")) %}{% endcall %}
 <h1>{{ tr.t("report-correct") }}</h1>
-<p>{{ tr.t("correct-hint") }}</p>
-{% if check %}
-<p>{{ tr.t("correct-check-hint") }}</p>
-{% endif %}
-{% if let Some(error) = form.error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
+<p class="muted">{{ tr.t("correct-hint") }}{% if check %} {{ tr.t("correct-check-hint") }}{% endif %}</p>
+{% if let Some(error) = form.error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
 <form method="post" action="/reports/{{ id }}/correct" class="stack">
-  {% include "report_fields.html" %}
-  <label>{{ tr.t("correct-time") }}
-    <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" required>
-  </label>
-  <label>{{ tr.t("report-note") }}
-    <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
-  </label>
-  <button type="submit">{{ tr.t("correct-submit") }}</button>
+  {% call c::card("") %}
+  {% call c::field(tr.t("correct-time")) %}<input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" required>{% endcall %}
+  {% endcall %}
+  {% call f::rooms(form.fields, tr.t("log-rooms"), tr.t("log-how-to-rate"), tr.t("log-inside"), tr.t("log-common"), tr.t("log-level-hint")) %}{% endcall %}
+  {% call f::symptoms(form.fields, tr.t("log-symptoms")) %}{% endcall %}
+  {% call c::collapsible(tr.t("log-note"), tr.t("optional"), tr.t("action-add"), !form.note.is_empty()) %}
+  <textarea name="note" rows="3" maxlength="2000" aria-label="{{ tr.t("log-note") }}">{{ form.note }}</textarea>
+  {% endcall %}
+  <div class="actions">
+    <a href="/reports/{{ id }}" class="button" data-variant="secondary">{{ tr.t("prompt-cancel") }}</a>
+    <button type="submit" class="button">{{ tr.t("correct-submit") }}</button>
+  </div>
 </form>
-<p><a href="/reports/{{ id }}">{{ tr.t("prompt-cancel") }}</a></p>
-{% include "rating_dialog.html" %}
+{% call f::rating_sheet(form.fields.levels, tr.t("rating-title"), tr.t("close"), tr.t("rating-tip"), tr.t("rating-full-guide")) %}{% endcall %}
 {% endblock %}
diff --git a/crates/web/templates/log.html b/crates/web/templates/log.html
index b43ce31..fe69a5f 100644
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -1,14 +1,14 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
+{% import "report_fields.html" as f %}
 {% block title %}{{ tr.t("log-title") }}{% endblock %}
 {% block content %}
 <h1>{{ tr.t("log-title") }}</h1>
-{% if let Some(saved) = saved %}
-<p class="notice" role="status">{{ tr.t(saved) }}</p>
-{% endif %}
+{% if let Some(saved) = saved %}{% call c::notice(tr.t(saved), false) %}{% endcall %}{% endif %}
 {% if let Some(notice) = notice %}
-<form method="post" action="/notices/dismiss" class="notice banner">
+<form method="post" action="/notices/dismiss" class="banner">
   <span>{{ notice }}</span>
-  <button type="submit" class="link">{{ tr.t("notice-dismiss") }}</button>
+  <button type="submit" class="button">{{ tr.t("notice-dismiss") }}</button>
 </form>
 {% endif %}
 {% if let Some(banner) = banner %}
@@ -22,32 +22,37 @@
   {% endif %}
 </section>
 {% endif %}
-<p class="summary">
-  {{ tr.t("summary-this-month") }} {{ tr.n("summary-events", summary.events) }} · {{ tr.n("summary-reports", summary.reports) }}{% if let Some(days) = summary.bedroom_days %} · {{ tr.n("summary-bedroom-days", days) }}{% endif %} · {{ tr.n("summary-checks", summary.checks) }}
-</p>
+<dl class="stats">
+  <div><dt>{{ tr.n("tile-events", summary.events) }}</dt><dd>{{ summary.events }}</dd></div>
+  <div><dt>{{ tr.n("tile-reports", summary.reports) }}</dt><dd>{{ summary.reports }}</dd></div>
+  {% if let Some(days) = summary.bedroom_days %}<div><dt>{{ tr.n("tile-bedroom-days", days) }}</dt><dd>{{ days }}</dd></div>{% endif %}
+  <div><dt>{{ tr.n("tile-checks", summary.checks) }}</dt><dd>{{ summary.checks }}</dd></div>
+</dl>
+<p class="stats-caption">{{ tr.t("summary-caption") }}</p>
 {% if let Some(form) = form %}
-{% if let Some(error) = form.error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
-<form method="post" action="/log" class="stack log">
-  {% include "report_fields.html" %}
-  <label>{{ tr.t("log-pm25") }}
-    <input type="number" name="pm25" min="0" max="1000" step="0.1" inputmode="decimal" value="{{ form.pm25 }}">
-  </label>
-  <details class="when"{% if !form.observed_at.is_empty() %} open{% endif %}>
-    <summary>{{ tr.t("log-time") }}: {{ tr.t("log-time-now") }} · {{ tr.t("log-time-change") }}</summary>
-    <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" aria-label="{{ tr.t("log-time") }}">
-  </details>
-  <label>{{ tr.t("log-note") }}
-    <textarea name="note" rows="2" maxlength="2000">{{ form.note }}</textarea>
-  </label>
+{% if let Some(error) = form.error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+<form method="post" action="/log" class="stack">
+  {% call c::collapsible(tr.t("log-time"), form.time_hint(tr), tr.t("log-time-change"), !form.observed_at.is_empty()) %}
+  <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" aria-label="{{ tr.t("log-time") }}">
+  {% endcall %}
+  {% call f::rooms(form.fields, tr.t("log-rooms"), tr.t("log-how-to-rate"), tr.t("log-inside"), tr.t("log-common"), tr.t("log-level-hint")) %}{% endcall %}
+  {% call f::symptoms(form.fields, tr.t("log-symptoms")) %}{% endcall %}
+  <section class="card">
+    <label class="inline-field">
+      <span><strong>PM2.5</strong> <span class="muted">· {{ tr.t("optional") }}</span></span>
+      <span class="unit"><input type="number" name="pm25" min="0" max="1000" step="0.1" inputmode="decimal" value="{{ form.pm25 }}"> µg/m³</span>
+    </label>
+  </section>
+  {% call c::collapsible(tr.t("log-note"), tr.t("optional"), tr.t("action-add"), !form.note.is_empty()) %}
+  <textarea name="note" rows="3" maxlength="2000" aria-label="{{ tr.t("log-note") }}">{{ form.note }}</textarea>
+  {% endcall %}
   <div class="actions">
     <button type="submit" name="action" value="no-smell" formnovalidate class="button" data-variant="secondary">{{ tr.t("log-no-smell") }}</button>
     <button type="submit" name="action" value="log" class="button">{% if form.update %}{{ tr.t("log-update") }}{% else %}{{ tr.t("log-submit") }}{% endif %}</button>
   </div>
 </form>
-{% include "rating_dialog.html" %}
+{% call f::rating_sheet(form.fields.levels, tr.t("rating-title"), tr.t("close"), tr.t("rating-tip"), tr.t("rating-full-guide")) %}{% endcall %}
 {% else %}
-<p>{{ tr.t("log-read-only") }}</p>
+<p class="card">{{ tr.t("log-read-only") }}</p>
 {% endif %}
 {% endblock %}
diff --git a/crates/web/templates/rating_dialog.html b/crates/web/templates/rating_dialog.html
deleted file mode 100644
index faaf877..0000000
--- a/crates/web/templates/rating_dialog.html
+++ /dev/null
@@ -1,16 +0,0 @@
-{# The compact intensity guide that "How to rate" opens (spec §5). #}
-<dialog id="rating-guide" aria-labelledby="rating-guide-title">
-  <div class="dialog-body">
-    <form method="dialog" class="dialog-close">
-      <button type="submit" class="link" aria-label="{{ tr.t("close") }}">✕</button>
-    </form>
-    <h2 id="rating-guide-title">{{ tr.t("rating-title") }}</h2>
-    <table class="guide compact">
-      {% for level in levels %}
-      <tr><th scope="row">{{ level.value }}</th><td>{{ level.name }}</td></tr>
-      {% endfor %}
-    </table>
-    <p>{{ tr.t("rating-tip") }}</p>
-    <p><a href="/guide/rating">{{ tr.t("rating-full-guide") }}</a></p>
-  </div>
-</dialog>
diff --git a/crates/web/templates/report_fields.html b/crates/web/templates/report_fields.html
index da20452..1664442 100644
--- a/crates/web/templates/report_fields.html
+++ b/crates/web/templates/report_fields.html
@@ -1,22 +1,65 @@
-{# The room, level and symptom fields of the Log and correction forms. #}
-<fieldset class="rooms">
-  <legend>{{ tr.t("log-rooms") }}
-    <a href="/guide/rating" data-dialog="rating-guide" class="info">ⓘ {{ tr.t("log-how-to-rate") }}</a>
-  </legend>
-  <p class="group">{{ tr.t("log-inside") }}</p>
-  {% for choice in form.fields.inside %}
-  {% include "room_level.html" %}
-  {% endfor %}
-  {% if !form.fields.common.is_empty() %}
-  <p class="group">{{ tr.t("log-common") }}</p>
-  {% for choice in form.fields.common %}
-  {% include "room_level.html" %}
-  {% endfor %}
-  {% endif %}
-</fieldset>
-<fieldset class="checks">
-  <legend>{{ tr.t("log-symptoms") }}</legend>
-  {% for symptom in form.fields.symptoms %}
-  <label><input type="checkbox" name="symptom" value="{{ symptom.value }}"{% if symptom.checked %} checked{% endif %}> {{ symptom.label }}</label>
-  {% endfor %}
-</fieldset>
+{# The cards the Log form and the correction form share (spec §9 screens 1 and 4). Import with
+   {% import "report_fields.html" as f %}. #}
+{% import "components.html" as c %}
+
+{# One room: its line with a check square, and the level grid 0–10 it opens while selected.
+   Each level carries its name and description, shown under the grid once picked (CSS only). #}
+{% macro room(choice, levels, hint) %}
+<div class="room">
+  <label class="room-check"><input type="checkbox" name="room" value="{{ choice.room.value }}"{% if choice.room.checked %} checked{% endif %}><span class="box">{% call c::icon("check") %}{% endcall %}</span>{{ choice.room.label }}</label>
+  <fieldset class="levels">
+    <legend class="visually-hidden">{{ choice.legend }}</legend>
+    {% for level in levels.iter() %}
+    <label class="level"><input type="radio" name="level-{{ choice.room.value }}" value="{{ level.value }}"{% if choice.is_picked(level.value) %} checked{% endif %}><span>{{ level.value }}</span><span class="level-text"><strong>{{ level.name }}.</strong> {{ level.description }}</span></label>
+    {% endfor %}
+    <span class="level-hint">{{ hint }}</span>
+  </fieldset>
+</div>
+{% endmacro %}
+
+{# The Rooms card: "How to rate", then the rooms grouped Inside and Common areas. #}
+{% macro rooms(fields, title, how_to_rate, inside, common, hint) %}
+<section class="card">
+  <div class="split">
+    <h2>{{ title }}</h2>
+    <a href="/guide/rating" data-dialog="rating-guide" class="info">{% call c::icon("info") %}{% endcall %}{{ how_to_rate }}</a>
+  </div>
+  <fieldset class="rooms">
+    <legend class="visually-hidden">{{ title }}</legend>
+    <p class="group">{{ inside }}</p>
+    {% for choice in fields.inside %}{% call room(choice, fields.levels, hint) %}{% endcall %}{% endfor %}
+    {% if !fields.common.is_empty() %}
+    <p class="group">{{ common }}</p>
+    {% for choice in fields.common %}{% call room(choice, fields.levels, hint) %}{% endcall %}{% endfor %}
+    {% endif %}
+  </fieldset>
+</section>
+{% endmacro %}
+
+{# The Symptoms card: one chip each. #}
+{% macro symptoms(fields, title) %}
+<section class="card">
+  <h2 id="symptoms-title">{{ title }}</h2>
+  <fieldset class="cluster" aria-labelledby="symptoms-title">
+    {% for symptom in fields.symptoms %}{% call c::chip("checkbox", "symptom", symptom.value, symptom.label, symptom.checked) %}{% endcall %}{% endfor %}
+  </fieldset>
+</section>
+{% endmacro %}
+
+{# The compact intensity guide "How to rate" opens, as a bottom sheet (spec §5). Without
+   JavaScript the link leads to the full guide instead. #}
+{% macro rating_sheet(levels, title, close, tip, full_guide) %}
+<dialog id="rating-guide" class="sheet" aria-labelledby="rating-guide-title">
+  <div class="split">
+    <h2 id="rating-guide-title">{{ title }}</h2>
+    <form method="dialog"><button type="submit" class="button" data-variant="quiet" aria-label="{{ close }}" autofocus>{% call c::icon("close") %}{% endcall %}</button></form>
+  </div>
+  <table class="scale">
+    {% for level in levels.iter() %}
+    <tr><th scope="row"><span class="badge">{{ level.value }}</span></th><td>{{ level.name }}</td></tr>
+    {% endfor %}
+  </table>
+  <p class="muted">{{ tip }}</p>
+  <a href="/guide/rating">{{ full_guide }}</a>
+</dialog>
+{% endmacro %}
diff --git a/crates/web/templates/room_level.html b/crates/web/templates/room_level.html
deleted file mode 100644
index a4174d3..0000000
--- a/crates/web/templates/room_level.html
+++ /dev/null
@@ -1,10 +0,0 @@
-{# One room chip and its level row 0–10, shown while the room is selected (spec §9, screen 1). #}
-<div class="room">
-  <label class="chip"><input type="checkbox" name="room" value="{{ choice.room.value }}"{% if choice.room.checked %} checked{% endif %}><span>{{ choice.room.label }}</span></label>
-  <fieldset class="levels">
-    <legend>{{ tr.t1("log-level-in", "room", choice.room.label) }}</legend>
-    {% for level in choice.levels %}
-    <label class="chip"><input type="radio" name="level-{{ choice.room.value }}" value="{{ level.value }}"{% if level.checked %} checked{% endif %}><span>{{ level.label }}</span></label>
-    {% endfor %}
-  </fieldset>
-</div>
diff --git a/crates/web/tests/log.rs b/crates/web/tests/log.rs
index ded99e3..d76faab 100644
--- a/crates/web/tests/log.rs
+++ b/crates/web/tests/log.rs
@@ -31,17 +31,42 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
         html.contains(r#"<form method="post" action="/log""#),
         "{html}"
     );
+    let page = Page::parse(&html);
     let bedroom_id = app.room("room-bedroom").await;
-    let level_row = format!(r#"name="level-{bedroom_id}""#);
-    assert_eq!(html.matches(&level_row).count(), 11, "levels 0 to 10");
-    assert!(html.contains("<legend>Level in Bedroom</legend>"), "{html}");
-    let inside = html.find("Inside").unwrap();
-    let common = html.find("Common areas").unwrap();
-    let bedroom = html.find("<span>Bedroom</span>").unwrap();
-    let stairwell = html.find("<span>Stairwell (shared)</span>").unwrap();
-    assert!(inside < bedroom && bedroom < common && common < stairwell);
-    assert!(!html.contains("Kitchen"), "hidden rooms are not offered");
-    assert!(html.contains(r#"name="symptom""#) && html.contains("Eye irritation"));
+    let bedroom = format!(r#".room:has(input[name=room][value="{bedroom_id}"])"#);
+    assert_eq!(
+        page.attrs(&format!("{bedroom} .levels input[type=radio]"), "value"),
+        (0..=10).map(|level| level.to_string()).collect::<Vec<_>>(),
+        "levels 0 to 10"
+    );
+    assert_eq!(page.text(&format!("{bedroom} legend")), "Level in Bedroom");
+    assert_eq!(
+        page.text(&format!(
+            "{bedroom} .level:has(input[value='3']) .level-text"
+        )),
+        "Light. Clearly smoke, but weak. Easy to ignore."
+    );
+    assert_eq!(
+        page.text(&format!("{bedroom} .level-hint")),
+        "Pick a level from 0 to 10."
+    );
+    assert_eq!(page.texts(".rooms > .group"), ["Inside", "Common areas"]);
+    let rooms = page.texts(".rooms .room-check");
+    assert_eq!(rooms.first().map(String::as_str), Some("Living room"));
+    assert_eq!(rooms.last().map(String::as_str), Some("Stairwell (shared)"));
+    assert!(
+        !rooms.iter().any(|room| room == "Kitchen"),
+        "hidden rooms are not offered"
+    );
+    assert!(
+        page.texts("label.chip > input[name=symptom] + span")
+            .contains(&"Eye irritation".to_owned())
+    );
+    assert!(
+        page.exists("details.collapsible:not([open]) input[type=datetime-local][name=observed_at]")
+    );
+    assert!(page.exists("details.collapsible:not([open]) textarea[name=note]"));
+    assert!(page.exists("input[type=number][name=pm25]"));
     let page = Page::parse(&html);
     assert_eq!(
         page.texts(r#"form[action="/log"] .actions button[name=action]"#),
@@ -55,7 +80,14 @@ async fn the_form_offers_visible_rooms_by_area_and_the_symptoms() {
         ["no-smell", "log"]
     );
     assert!(page.exists("button[value=no-smell][formnovalidate][data-variant=secondary]"));
-    assert!(html.contains(" · 0 clean checks"), "{html}");
+    assert_eq!(
+        page.texts(".stats dt").last().map(String::as_str),
+        Some("clean checks")
+    );
+    assert_eq!(
+        page.texts(".stats dd").last().map(String::as_str),
+        Some("0")
+    );
 }
 
 #[tokio::test]
@@ -96,14 +128,12 @@ async fn with_an_event_open_the_form_offers_an_update_with_my_last_rooms() {
         html.contains(">Update: situation changed</button>"),
         "{html}"
     );
-    assert!(
-        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
-        "my last rooms are pre-selected: {html}"
-    );
-    assert!(
-        html.contains(&format!(r#"value="{}"><span>Kitchen"#, kitchen.0)),
-        "not the partner's: {html}"
+    assert_eq!(
+        page.texts(".room-check:has(input[name=room][checked])"),
+        ["Bedroom"],
+        "my last rooms are pre-selected, not the partner's Kitchen"
     );
+    assert!(page.exists(&format!(r#"input[name=room][value="{}"]"#, kitchen.0)));
 }
 
 #[tokio::test]
@@ -220,16 +250,32 @@ async fn the_summary_counts_this_month_and_guests_count_only_their_own() {
     .await;
 
     let cookie = app.sign_in("daniele").await;
-    let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(
-        html.contains("This month: 2 events · 3 reports · 2 days with bedroom smoke"),
-        "{html}"
+    let tiles = async |cookie: &str| {
+        let page = Page::parse(&text(app.send(get_with("/", cookie)).await).await);
+        assert_eq!(page.text(".stats-caption"), "This month");
+        page.texts(".stats dd")
+            .into_iter()
+            .zip(page.texts(".stats dt"))
+            .collect::<Vec<_>>()
+    };
+    let pair = |n: &str, label: &str| (n.to_owned(), label.to_owned());
+    assert_eq!(
+        tiles(&cookie).await,
+        [
+            pair("2", "events"),
+            pair("3", "reports"),
+            pair("2", "bedroom days"),
+            pair("0", "clean checks")
+        ]
     );
     let cookie = app.sign_in("guest").await;
-    let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(
-        html.contains("This month: 2 events · 2 reports · 1 day with bedroom smoke"),
-        "{html}"
+    assert_eq!(
+        tiles(&cookie).await[..3],
+        [
+            pair("2", "events"),
+            pair("2", "reports"),
+            pair("1", "bedroom day")
+        ]
     );
 }
 
@@ -269,25 +315,28 @@ async fn how_to_rate_opens_a_dialog_with_a_link_to_the_full_guide() {
     app.add_user("daniele", Role::Resident, Locale::It).await;
     let cookie = app.sign_in("daniele").await;
     let html = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(
-        html.contains(r#"<a href="/guide/rating" data-dialog="rating-guide""#),
-        "{html}"
-    );
-    assert!(html.contains(r#"<dialog id="rating-guide""#), "{html}");
-    assert!(html.contains(r#"<form method="dialog""#), "{html}");
-    assert!(
-        html.contains(r#"<tr><th scope="row">0</th><td>Pulito</td></tr>"#),
-        "{html}"
+    let page = Page::parse(&html);
+    assert!(page.exists(r#"a[href="/guide/rating"][data-dialog=rating-guide]"#));
+    let sheet = "dialog#rating-guide.sheet[aria-labelledby=rating-guide-title]";
+    assert_eq!(
+        page.text(&format!("{sheet} h2#rating-guide-title")),
+        "Come valutare"
     );
-    assert!(html.contains("<td>Appena percepibile</td>"), "{html}");
-    assert!(
-        html.contains("Tra due numeri, scegli il più basso."),
-        "{html}"
+    assert!(page.exists(&format!(
+        "{sheet} form[method=dialog] button[aria-label=Chiudi]"
+    )));
+    assert_eq!(page.count(&format!("{sheet} tr")), 11);
+    assert_eq!(page.text(&format!("{sheet} tr td")), "Pulito");
+    assert_eq!(
+        page.texts(&format!("{sheet} tr td"))[1],
+        "Appena percepibile"
     );
-    assert!(
-        Page::parse(&html).exists(r#"script[type=module][src="/static/js/app.js"]"#),
-        "{html}"
+    assert_eq!(
+        page.text(&format!("{sheet} p")),
+        "Valuta quello che senti adesso, non quanto sei infastidito. Tra due numeri, scegli il più basso."
     );
+    assert!(page.exists(&format!(r#"{sheet} a[href="/guide/rating"]"#)));
+    assert!(page.exists(r#"script[type=module][src="/static/js/app.js"]"#));
 
     let guide = app.send(get_with("/guide/rating", &cookie)).await;
     assert_eq!(guide.status(), StatusCode::OK);
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index a1d2692..6672507 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::logbook::EventChoice;
 use smoke_domain::repos::{EventRepo, ReadingRepo, ReportRepo};
@@ -501,10 +502,14 @@ async fn invalid_input_shows_the_form_again_with_what_was_typed() {
         html.contains("Pick a level from 0 to 10 for each selected room."),
         "{html}"
     );
-    assert!(html.contains(">kept as typed</textarea>"), "{html}");
-    assert!(
-        html.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.text("details.collapsible[open] textarea[name=note]"),
+        "kept as typed"
+    );
+    assert_eq!(
+        page.texts(".room-check:has(input[name=room][checked])"),
+        ["Bedroom"]
     );
     let missing = app
         .send(post_form("/log", "action=log", Some(&cookie)))
@@ -551,7 +556,15 @@ async fn no_smell_with_nothing_open_saves_a_check_without_asking() {
     assert!(check.values.rooms().is_empty(), "the whole home");
     let html = text(app.send(get_with("/?saved=no-smell", &cookie)).await).await;
     assert!(html.contains("Saved: no smell."), "{html}");
-    assert!(html.contains(" · 1 clean check"), "{html}");
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.texts(".stats dt").last().map(String::as_str),
+        Some("clean check")
+    );
+    assert_eq!(
+        page.texts(".stats dd").last().map(String::as_str),
+        Some("1")
+    );
 }
 
 #[tokio::test]
diff --git a/crates/web/tests/reports.rs b/crates/web/tests/reports.rs
index 03f490e..d880e2a 100644
--- a/crates/web/tests/reports.rs
+++ b/crates/web/tests/reports.rs
@@ -4,6 +4,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::logbook::EventChoice;
 use smoke_domain::repos::{EventRepo, ReportRepo};
@@ -76,13 +77,14 @@ async fn correcting_a_mistake_is_recorded_and_badged() {
             .await,
     )
     .await;
-    assert!(
-        form.contains(&format!(r#"name="level-{}" value="6" checked>"#, bedroom.0)),
-        "{form}"
-    );
-    assert!(
-        form.contains(&format!(r#"value="{}" checked><span>Bedroom"#, bedroom.0)),
-        "{form}"
+    let page = Page::parse(&form);
+    assert!(page.exists(&format!(
+        r#"input[name="level-{}"][value="6"][checked]"#,
+        bedroom.0
+    )));
+    assert_eq!(
+        page.texts(".room-check:has(input[name=room][checked])"),
+        ["Bedroom"]
     );
     assert!(
         form.contains(r#"name="observed_at" value="2026-01-15T19:05""#),
diff --git a/crates/web/tests/settings.rs b/crates/web/tests/settings.rs
index a2be555..c7f1f37 100644
--- a/crates/web/tests/settings.rs
+++ b/crates/web/tests/settings.rs
@@ -178,7 +178,12 @@ async fn residents_add_rooms_and_symptoms_and_hide_them_from_the_forms() {
         ("Balcony", RoomArea::Common, None)
     );
     let log = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(log.contains("<span>Balcony</span>"), "{log}");
+    assert!(
+        Page::parse(&log)
+            .texts(".room-check")
+            .contains(&"Balcony".to_owned()),
+        "{log}"
+    );
     assert!(log.contains("Dizziness"), "{log}");
 
     let hide = format!("/settings/rooms/{}/hidden", balcony.id);
@@ -199,7 +204,12 @@ async fn residents_add_rooms_and_symptoms_and_hide_them_from_the_forms() {
     app.send(post_form(&hide, "hidden=false", Some(&cookie)))
         .await;
     let log = text(app.send(get_with("/", &cookie)).await).await;
-    assert!(log.contains("<span>Balcony</span>"), "{log}");
+    assert!(
+        Page::parse(&log)
+            .texts(".room-check")
+            .contains(&"Balcony".to_owned()),
+        "{log}"
+    );
 }
 
 #[tokio::test]
````

### Task 8: Events list and event page

**Proto commit:** `960dce3` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 960dce3`)

Spec §4 (Events, Event detail). Month switcher card with icon links (`events-previous/next` become aria-labels), one card of `row`s per day with badges; `EventRow { detail, badge }`; event page: back link, timeline card (CSS dots, hollow for readings), Close card, Reopen as `post_button`. CSS 67–68. Keys `events-month`, `event-timeline`, `event-close-title`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/67-switcher.css
A	crates/web/css/68-timeline.css
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/assets.rs
M	crates/web/src/routes/events.rs
M	crates/web/templates/event.html
M	crates/web/templates/events.html
M	crates/web/tests/events.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/events.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **615 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(events): Events list and event page as cards with badges and a dotted timeline

The month switcher is a card with icon links; each day is a card of list rows led by a
badge (amber with the highest level for an event, green 0 for a clean check, PM for a
reading). An event's page has a back link, its timeline as dots on a line (hollow for PM2.5),
the large-gap warning inside it, and a Close card. Tests query the pages with scraper.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `960dce3`):

````diff
diff --git a/crates/web/css/67-switcher.css b/crates/web/css/67-switcher.css
new file mode 100644
index 0000000..d4ac741
--- /dev/null
+++ b/crates/web/css/67-switcher.css
@@ -0,0 +1,31 @@
+/* .switcher: "‹ October 2026 ›", the period with links to the one before and after. */
+@layer components {
+  .switcher {
+    flex-direction: row;
+    align-items: center;
+    justify-content: space-between;
+    padding: var(--space-1);
+  }
+
+  .switcher > a {
+    display: grid;
+    place-items: center;
+    inline-size: var(--target);
+    block-size: var(--target);
+    border-radius: var(--radius-field);
+  }
+
+  .day {
+    padding-inline: var(--space-1);
+    color: var(--muted);
+    font-size: var(--text-small);
+    font-weight: 600;
+    letter-spacing: 0.06em;
+    text-transform: uppercase;
+  }
+
+  .rows {
+    gap: 0;
+    padding-block: var(--space-1);
+  }
+}
diff --git a/crates/web/css/68-timeline.css b/crates/web/css/68-timeline.css
new file mode 100644
index 0000000..47f0d3b
--- /dev/null
+++ b/crates/web/css/68-timeline.css
@@ -0,0 +1,74 @@
+/* .timeline: an event's reports and readings down a line of dots, times on the left; PM2.5
+   readings are hollow dots; a .gap note marks a long silence. */
+@layer components {
+  .timeline {
+    list-style: none;
+  }
+
+  .timeline > li {
+    position: relative;
+    display: grid;
+    grid-template-columns: 3.25rem 1fr;
+    gap: var(--space-3);
+    padding-block: var(--space-2);
+  }
+
+  .timeline > li:not(.gap)::before {
+    content: "";
+    position: absolute;
+    inset-block: 0;
+    inset-inline-start: calc(3.25rem + var(--space-1));
+    border-inline-start: 2px solid var(--line);
+  }
+
+  .timeline > li:not(.gap)::after {
+    content: "";
+    position: absolute;
+    inset-block-start: calc(var(--space-2) + 0.4rem);
+    inset-inline-start: calc(3.25rem + 0.0625rem);
+    inline-size: 0.75rem;
+    block-size: 0.75rem;
+    border: 2px solid var(--accent);
+    border-radius: var(--radius-pill);
+    background: var(--accent);
+  }
+
+  .timeline > li[data-kind="reading"]::after {
+    background: var(--card);
+  }
+
+  .timeline .at {
+    color: var(--muted);
+    font-variant-numeric: tabular-nums;
+  }
+
+  .timeline .entry {
+    display: flex;
+    flex-direction: column;
+    padding-inline-start: var(--space-4);
+  }
+
+  .timeline .entry > .muted {
+    font-size: var(--text-label);
+  }
+
+  .timeline > .gap {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-2);
+    margin-block: var(--space-1);
+    padding: var(--space-3);
+    border-radius: var(--radius-field);
+    background: var(--warn-bg);
+    color: var(--warn-text);
+    font-size: var(--text-label);
+  }
+
+  .gap .button {
+    align-self: flex-start;
+    border-color: var(--warn-text);
+    background: transparent;
+    color: var(--warn-text);
+    font-size: var(--text-label);
+  }
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index ab6444c..9cd2ee5 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -151,12 +151,15 @@ nav-label = Main menu
 events-title = Events
 events-none = No events this month.
 events-check = no smell (check)
-events-previous = ← Previous
-events-next = Next →
+events-previous = Previous month
+events-next = Next month
+events-month = Month
 events-in-progress = in progress
 events-max-intensity = max { $level }
 event-title = Event of { $date }
 event-started-by = Started by { $username }
+event-timeline = Timeline
+event-close-title = Close this event
 event-gap = More than 3 hours since the previous report: is this still the same event?
 event-intensity = intensity { $level }
 event-pm25 = PM2.5 { $value } µg/m³
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 8bd08d1..5fb89b4 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -151,12 +151,15 @@ nav-label = Menu principale
 events-title = Eventi
 events-none = Nessun evento questo mese.
 events-check = nessun odore (controllo)
-events-previous = ← Precedente
-events-next = Successivo →
+events-previous = Mese precedente
+events-next = Mese successivo
+events-month = Mese
 events-in-progress = in corso
 events-max-intensity = max { $level }
 event-title = Evento del { $date }
 event-started-by = Iniziato da { $username }
+event-timeline = Cronologia
+event-close-title = Chiudi questo evento
 event-gap = Più di 3 ore dalla segnalazione precedente: è ancora lo stesso evento?
 event-intensity = intensità { $level }
 event-pm25 = PM2.5 { $value } µg/m³
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index e903bd2..2fc796f 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -151,12 +151,15 @@ nav-label = Ana menü
 events-title = Olaylar
 events-none = Bu ay olay yok.
 events-check = koku yok (kontrol)
-events-previous = ← Önceki
-events-next = Sonraki →
+events-previous = Önceki ay
+events-next = Sonraki ay
+events-month = Ay
 events-in-progress = sürüyor
 events-max-intensity = en fazla { $level }
 event-title = { $date } tarihli olay
 event-started-by = Başlatan: { $username }
+event-timeline = Zaman çizelgesi
+event-close-title = Bu olayı kapat
 event-gap = Önceki bildirimden bu yana 3 saatten fazla geçti: hâlâ aynı olay mı?
 event-intensity = yoğunluk { $level }
 event-pm25 = PM2.5 { $value } µg/m³
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 2a29f47..c85205d 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -59,6 +59,8 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "64-sheet.css",
     "65-stats.css",
     "66-room.css",
+    "67-switcher.css",
+    "68-timeline.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index d2339d4..a359c0a 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -38,8 +38,10 @@ pub(crate) struct ListQuery {
 struct EventRow {
     id: i64,
     span: String,
-    reports: usize,
-    max_intensity: Option<String>,
+    /// "3 reports · max 6".
+    detail: String,
+    /// The highest intensity among the reports, or a dash when there is none to show.
+    badge: String,
 }
 
 struct ReadingRow {
@@ -143,15 +145,17 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
         if local < schedule.local_date(from) {
             continue;
         }
+        let max = visible.iter().map(|r| r.values.intensity().get()).max();
+        let mut detail = tr.n("summary-reports", &visible.len());
+        if let Some(max) = max {
+            detail.push_str(" · ");
+            detail.push_str(&tr.t1("events-max-intensity", "level", &max.to_string()));
+        }
         day(&mut days, tr, local).events.push(EventRow {
             id: event.id.0,
             span: span(tr, event, tz),
-            reports: visible.len(),
-            max_intensity: visible
-                .iter()
-                .map(|r| r.values.intensity().get())
-                .max()
-                .map(|max| max.to_string()),
+            detail,
+            badge: max.map_or_else(|| "–".to_owned(), |max| max.to_string()),
         });
     }
     for check in checks.iter().rev() {
diff --git a/crates/web/templates/event.html b/crates/web/templates/event.html
index 4b742ce..1d36758 100644
--- a/crates/web/templates/event.html
+++ b/crates/web/templates/event.html
@@ -1,38 +1,33 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t1("event-title", "date", date) }}{% endblock %}
 {% block content %}
-<h1>{{ tr.t1("event-title", "date", date) }}</h1>
-<p>{{ span }}{% if let Some(reason) = reason %} · {{ tr.t(reason) }}{% endif %}</p>
-{% if let Some(owner) = owner %}
-<p>{{ tr.t1("event-started-by", "username", owner) }}</p>
-{% endif %}
-{% if let Some(error) = error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
+{% call c::back("/events", tr.t("events-title")) %}{% endcall %}
+<div>
+  <h1>{{ tr.t1("event-title", "date", date) }}</h1>
+  <p class="muted">{{ span }}{% if let Some(reason) = reason %} · {{ tr.t(reason) }}{% endif %}{% if let Some(owner) = owner %} · {{ tr.t1("event-started-by", "username", owner) }}{% endif %}</p>
+</div>
+{% if let Some(error) = error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+{% call c::card(tr.t("event-timeline")) %}
 <ol class="timeline">
   {% for entry in entries %}
   {% if entry.gap_before %}
-  <li class="gap" role="note">{{ tr.t("event-gap") }}</li>
+  <li class="gap" role="note"><span>{{ tr.t("event-gap") }}</span></li>
   {% endif %}
-  <li>
+  <li{% if entry.report.is_none() %} data-kind="reading"{% endif %}>
     <span class="at">{{ entry.at }}</span>
-    {% if let Some(report) = entry.report %}<a href="/reports/{{ report }}">{{ entry.text }}</a>{% else %}{{ entry.text }}{% endif %}
-    {% if let Some(by) = entry.by %}<span class="by">({{ by }})</span>{% endif %}
+    <span class="entry">{% if let Some(report) = entry.report %}<a href="/reports/{{ report }}">{{ entry.text }}</a>{% else %}{{ entry.text }}{% endif %}{% if let Some(by) = entry.by %}<span class="muted">{{ by }}</span>{% endif %}</span>
   </li>
   {% endfor %}
 </ol>
+{% endcall %}
 {% if let Some(close_at) = close_at %}
+{% call c::card(tr.t("event-close-title")) %}
 <form method="post" action="/events/{{ id }}/close" class="stack">
-  <label>{{ tr.t("event-close-at") }}
-    <input type="datetime-local" name="closed_at" value="{{ close_at }}" required>
-  </label>
-  <button type="submit">{{ tr.t("event-close") }}</button>
-</form>
-{% endif %}
-{% if can_reopen %}
-<form method="post" action="/events/{{ id }}/reopen">
-  <button type="submit" class="secondary">{{ tr.t("event-reopen") }}</button>
+  {% call c::field(tr.t("event-close-at")) %}<input type="datetime-local" name="closed_at" value="{{ close_at }}" required>{% endcall %}
+  <button type="submit" class="button">{{ tr.t("event-close") }}</button>
 </form>
+{% endcall %}
 {% endif %}
-<p><a href="/events">{{ tr.t("events-title") }}</a></p>
+{% if can_reopen %}{% call c::post_button("/events/{}/reopen"|format(id), tr.t("event-reopen"), "secondary") %}{% endcall %}{% endif %}
 {% endblock %}
diff --git a/crates/web/templates/events.html b/crates/web/templates/events.html
index e0fc607..f737bed 100644
--- a/crates/web/templates/events.html
+++ b/crates/web/templates/events.html
@@ -1,28 +1,29 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("events-title") }}{% endblock %}
 {% block content %}
 <h1>{{ tr.t("events-title") }}</h1>
-<nav class="months">
-  <a href="/events?month={{ previous }}">{{ tr.t("events-previous") }}</a>
+<nav class="card switcher" aria-label="{{ tr.t("events-month") }}">
+  <a href="/events?month={{ previous }}" aria-label="{{ tr.t("events-previous") }}">{% call c::icon("chevron-left") %}{% endcall %}</a>
   <strong>{{ month }}</strong>
-  <a href="/events?month={{ next }}">{{ tr.t("events-next") }}</a>
+  <a href="/events?month={{ next }}" aria-label="{{ tr.t("events-next") }}">{% call c::icon("chevron-right") %}{% endcall %}</a>
 </nav>
 {% for day in days %}
-<section>
-  <h2>{{ day.date }}</h2>
-  <ul class="rows">
+<section class="stack">
+  <h2 class="day">{{ day.date }}</h2>
+  <div class="card rows">
     {% for event in day.events %}
-    <li><a href="/events/{{ event.id }}">{{ event.span }}</a> · {{ tr.n("summary-reports", event.reports) }}{% if let Some(max) = event.max_intensity %} · {{ tr.t1("events-max-intensity", "level", max) }}{% endif %}</li>
+    {% call c::row("/events/{}"|format(event.id), event.span, event.detail) %}<span class="badge" data-tone="warn">{{ event.badge }}</span>{% endcall %}
     {% endfor %}
     {% for check in day.checks %}
-    <li><a href="/reports/{{ check.id }}">{{ check.at }} · {{ tr.t("events-check") }}</a></li>
+    {% call c::row("/reports/{}"|format(check.id), check.at, tr.t("events-check")) %}<span class="badge">0</span>{% endcall %}
     {% endfor %}
     {% for reading in day.readings %}
-    <li>{{ reading.at }} · {{ tr.t1("event-pm25", "value", reading.value) }}</li>
+    {% call c::row("", reading.at, tr.t1("event-pm25", "value", reading.value)) %}<span class="badge" data-tone="muted">PM</span>{% endcall %}
     {% endfor %}
-  </ul>
+  </div>
 </section>
 {% else %}
-<p>{{ tr.t("events-none") }}</p>
+<p class="card muted">{{ tr.t("events-none") }}</p>
 {% endfor %}
 {% endblock %}
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index 225b2db..b53a3e8 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -4,6 +4,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::logbook::EventChoice;
 use smoke_domain::repos::EventRepo;
@@ -38,38 +39,42 @@ async fn the_list_groups_a_months_events_by_day_newest_first() {
     app.log_pm25(&me, "2026-01-11T13:00:00Z", 12.5).await;
     let cookie = app.sign_in("daniele").await;
 
-    let html = text(app.send(get_with("/events?month=2026-01", &cookie)).await).await;
-    assert!(html.contains("<strong>01/2026</strong>"), "{html}");
-    assert!(html.contains(r#"href="/events?month=2025-12""#), "{html}");
-    assert!(html.contains(r#"href="/events?month=2026-02""#), "{html}");
-    let day12 = html.find("<h2>12/01/2026</h2>").unwrap();
-    let day11 = html.find("<h2>11/01/2026</h2>").unwrap();
-    let day10 = html.find("<h2>10/01/2026</h2>").unwrap();
-    assert!(day12 < day11 && day11 < day10, "newest first: {html}");
-    assert!(
-        html.contains(&format!(
-            r#"<a href="/events/{}">19:00–20:00</a> · 2 reports · max 7"#,
-            first.report.event_id.unwrap()
-        )),
-        "{html}"
+    let page = Page::parse(&text(app.send(get_with("/events?month=2026-01", &cookie)).await).await);
+    assert_eq!(page.text("nav.switcher strong"), "01/2026");
+    assert_eq!(
+        page.attrs("nav.switcher a", "href"),
+        ["/events?month=2025-12", "/events?month=2026-02"]
+    );
+    assert_eq!(
+        page.attrs("nav.switcher a", "aria-label"),
+        ["Previous month", "Next month"]
     );
+    assert_eq!(
+        page.texts("h2.day"),
+        ["12/01/2026", "11/01/2026", "10/01/2026"],
+        "newest first"
+    );
+    let event = |id| format!(r#"a.row[href="/events/{id}"]"#);
+    let first = event(first.report.event_id.unwrap());
+    assert_eq!(page.text(&format!("{first} .badge[data-tone=warn]")), "7");
+    assert_eq!(
+        page.text(&format!("{first} .row-text")),
+        "19:00–20:00 2 reports · max 7"
+    );
+    let second = event(second.report.event_id.unwrap());
     assert!(
-        html.contains(&format!(
-            r#"<a href="/events/{}">08:30–"#,
-            second.report.event_id.unwrap()
-        )),
-        "{html}"
+        page.text(&format!("{second} .row-text strong"))
+            .starts_with("08:30–")
     );
-    assert!(html.contains("14:00 · PM2.5 12.5 µg/m³"), "{html}");
+    assert_eq!(page.text("div.row .badge[data-tone=muted]"), "PM");
+    assert_eq!(page.text("div.row .row-text"), "14:00 PM2.5 12.5 µg/m³");
 
     let empty = text(app.send(get_with("/events?month=2025-11", &cookie)).await).await;
     assert!(empty.contains("No events this month."), "{empty}");
-    let default = text(app.send(get_with("/events?month=junk", &cookie)).await).await;
-    assert!(default.contains("<strong>01/2026</strong>"), "{default}");
-    for far in ["262142-12", "-262143-01", "0001-01"] {
+    for far in ["junk", "262142-12", "-262143-01", "0001-01"] {
         let uri = format!("/events?month={far}");
-        let html = text(app.send(get_with(&uri, &cookie)).await).await;
-        assert!(html.contains("<strong>01/2026</strong>"), "{far}: {html}");
+        let page = Page::parse(&text(app.send(get_with(&uri, &cookie)).await).await);
+        assert_eq!(page.text("nav.switcher strong"), "01/2026", "{far}");
     }
 }
 
@@ -106,12 +111,14 @@ async fn guests_list_only_events_they_own_or_reported_in() {
         !html.contains(&format!("/events/{}", private.report.event_id.unwrap())),
         "{html}"
     );
-    assert!(
-        html.contains(&format!(
-            r#"<a href="/events/{}">19:00–19:30</a> · 1 report · max 4"#,
-            shared.report.event_id.unwrap()
-        )),
-        "only the guest's own report counts: {html}"
+    let row = format!(
+        r#"a.row[href="/events/{}"] .row-text"#,
+        shared.report.event_id.unwrap()
+    );
+    assert_eq!(
+        Page::parse(&html).text(&row),
+        "19:00–19:30 1 report · max 4",
+        "only the guest's own report counts"
     );
 }
 
@@ -147,25 +154,48 @@ async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
     let cookie = app.sign_in("daniele").await;
 
     let html = text(app.send(get_with(&format!("/events/{id}"), &cookie)).await).await;
-    assert!(html.contains("<h1>Event of 15/01/2026</h1>"), "{html}");
-    assert!(html.contains("Started by daniele"), "{html}");
+    let page = Page::parse(&html);
+    assert_eq!(page.text("h1"), "Event of 15/01/2026");
     assert!(
-        html.contains(&format!(
-            r#"<a href="/reports/{}">intensity 3 · Bedroom 3</a>"#,
+        page.text("h1 + p").ends_with("Started by daniele"),
+        "{html}"
+    );
+    assert_eq!(page.attr("a.back", "href").as_deref(), Some("/events"));
+    assert_eq!(
+        page.text(&format!(
+            r#".timeline a[href="/reports/{}"]"#,
             first.report.id
         )),
-        "{html}"
+        "intensity 3 · Bedroom 3"
     );
-    assert!(html.contains("<span class=\"by\">(guest)</span>"), "{html}");
-    let reading = html.find("PM2.5 30 µg/m³").unwrap();
-    let gap = html
-        .find("More than 3 hours since the previous report")
-        .unwrap();
-    let after_gap = html
-        .find(&format!(r#"<a href="/reports/{}">"#, late.report.id))
+    assert_eq!(
+        page.texts(".timeline .entry > .muted"),
+        ["daniele", "guest", "daniele", "daniele"]
+    );
+    assert_eq!(
+        page.texts(".timeline > li .at"),
+        ["10:00", "11:00", "11:30", "15:00"],
+        "reports and readings in time order"
+    );
+    assert_eq!(
+        page.text(".timeline > li[data-kind=reading] .entry"),
+        "PM2.5 30 µg/m³ daniele"
+    );
+    let items = page.attrs(".timeline > li", "class");
+    assert_eq!(items, ["gap"], "one gap warning");
+    let order: Vec<String> = page.texts(".timeline > li");
+    let gap = order
+        .iter()
+        .position(|item| item.starts_with("More than 3 hours"))
         .unwrap();
-    assert!(reading < gap && gap < after_gap, "{html}");
-    assert_eq!(html.matches("More than 3 hours").count(), 1);
+    assert!(
+        order[gap - 1].contains("PM2.5 30") && order[gap + 1].starts_with("15:00"),
+        "{order:?}"
+    );
+    assert!(page.exists(&format!(
+        r#".timeline a[href="/reports/{}"]"#,
+        late.report.id
+    )));
 }
 
 #[tokio::test]
@@ -467,12 +497,14 @@ async fn the_list_shows_each_days_checks_to_those_who_may_see_them() {
 
     let resident = app.sign_in("daniele").await;
     let html = text(app.send(get_with("/events?month=2026-01", &resident)).await).await;
-    for (check, at) in [(&mine, "09:00"), (&theirs, "09:00")] {
-        let row = format!(
-            r#"<a href="/reports/{}">{at} · no smell (check)</a>"#,
-            check.report.id
+    let page = Page::parse(&html);
+    for check in [&mine, &theirs] {
+        let row = format!(r#"a.row[href="/reports/{}"]"#, check.report.id);
+        assert_eq!(
+            page.text(&format!("{row} .row-text")),
+            "09:00 no smell (check)"
         );
-        assert!(html.contains(&row), "{html}");
+        assert_eq!(page.text(&format!("{row} .badge")), "0");
     }
     assert!(!html.contains("No events this month."), "{html}");
 
````

### Task 9: "Move to a new event" from the large-gap warning

**Proto commit:** `3f4536b` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 3f4536b`)

Spec §6.4. `Entry.gap: Option<Gap { movable }>` — the report after the gap, when `EditEntry` allows it and it is not a no-smell report; the button posts `target=new` to `/reports/{id}/move`. Key `event-gap-move`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/routes/events.rs
M	crates/web/templates/event.html
M	crates/web/tests/events.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/events.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **617 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(events): the large-gap warning offers to move the report after it to a new event

Look-and-feel spec §6.4: the warning gets a "Move 14:45 to a new event" button that posts
target=new to the existing /reports/{id}/move route, for the report after the gap only.
It is shown only to those allowed to move that report and never for a no-smell report,
which cannot start an event. Route tests cover the move, a viewer and a check.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `3f4536b`):

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 9cd2ee5..4e169bf 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -161,6 +161,7 @@ event-started-by = Started by { $username }
 event-timeline = Timeline
 event-close-title = Close this event
 event-gap = More than 3 hours since the previous report: is this still the same event?
+event-gap-move = Move { $time } to a new event
 event-intensity = intensity { $level }
 event-pm25 = PM2.5 { $value } µg/m³
 event-close = Close event
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 5fb89b4..49b10d7 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -161,6 +161,7 @@ event-started-by = Iniziato da { $username }
 event-timeline = Cronologia
 event-close-title = Chiudi questo evento
 event-gap = Più di 3 ore dalla segnalazione precedente: è ancora lo stesso evento?
+event-gap-move = Sposta { $time } in un nuovo evento
 event-intensity = intensità { $level }
 event-pm25 = PM2.5 { $value } µg/m³
 event-close = Chiudi evento
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 2fc796f..2bf50cb 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -161,6 +161,7 @@ event-started-by = Başlatan: { $username }
 event-timeline = Zaman çizelgesi
 event-close-title = Bu olayı kapat
 event-gap = Önceki bildirimden bu yana 3 saatten fazla geçti: hâlâ aynı olay mı?
+event-gap-move = { $time } kaydını yeni bir olaya taşı
 event-intensity = yoğunluk { $level }
 event-pm25 = PM2.5 { $value } µg/m³
 event-close = Olayı kapat
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index a359c0a..63b6fc6 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -204,7 +204,14 @@ struct Entry {
     /// Shown to people who see everyone's data.
     by: Option<String>,
     /// More than three hours after the previous report (spec §7.7).
-    gap_before: bool,
+    gap: Option<Gap>,
+}
+
+/// The large-gap warning before a report.
+struct Gap {
+    /// The report after the gap, when the viewer may move it to a new event (look-and-feel
+    /// spec §6.4). A no-smell report cannot start an event, so it is never offered.
+    movable: Option<i64>,
 }
 
 #[derive(Template)]
@@ -314,7 +321,17 @@ async fn detail<S: Store>(
                     .collect::<Vec<_>>()
                     .join(" · "),
                 by: names.get(&report.user_id).cloned(),
-                gap_before: gaps.contains(&index),
+                gap: gaps.contains(&index).then(|| Gap {
+                    movable: (!report.values.is_no_smell()
+                        && can(
+                            &actor,
+                            Action::EditEntry {
+                                owner: report.user_id,
+                            },
+                            now,
+                        ))
+                    .then_some(report.id.0),
+                }),
             },
         ));
     }
@@ -326,7 +343,7 @@ async fn detail<S: Store>(
                 report: None,
                 text: tr.t1("event-pm25", "value", &reading.value.ugm3().to_string()),
                 by: names.get(&reading.user_id).cloned(),
-                gap_before: false,
+                gap: None,
             },
         ));
     }
diff --git a/crates/web/templates/event.html b/crates/web/templates/event.html
index 1d36758..05bc403 100644
--- a/crates/web/templates/event.html
+++ b/crates/web/templates/event.html
@@ -11,8 +11,16 @@
 {% call c::card(tr.t("event-timeline")) %}
 <ol class="timeline">
   {% for entry in entries %}
-  {% if entry.gap_before %}
-  <li class="gap" role="note"><span>{{ tr.t("event-gap") }}</span></li>
+  {% if let Some(gap) = entry.gap %}
+  <li class="gap" role="note">
+    <span>{{ tr.t("event-gap") }}</span>
+    {% if let Some(report) = gap.movable %}
+    <form method="post" action="/reports/{{ report }}/move">
+      <input type="hidden" name="target" value="new">
+      <button type="submit" class="button">{{ tr.t1("event-gap-move", "time", entry.at) }}</button>
+    </form>
+    {% endif %}
+  </li>
   {% endif %}
   <li{% if entry.report.is_none() %} data-kind="reading"{% endif %}>
     <span class="at">{{ entry.at }}</span>
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index b53a3e8..852fa19 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -7,7 +7,7 @@ use axum::http::StatusCode;
 use common::page::Page;
 use common::*;
 use smoke_domain::logbook::EventChoice;
-use smoke_domain::repos::EventRepo;
+use smoke_domain::repos::{EventRepo, ReportRepo};
 use smoke_domain::types::{CloseReason, Locale, Role};
 
 #[tokio::test]
@@ -198,6 +198,142 @@ async fn the_event_page_shows_the_timeline_and_warns_about_large_gaps() {
     )));
 }
 
+/// Look-and-feel spec §6.4: the warning offers to move the report after the gap, through the
+/// existing move route, to those allowed to move it.
+#[tokio::test]
+async fn the_gap_warning_moves_the_report_after_it_to_a_new_event() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    let first = app
+        .log(
+            &me,
+            "2026-01-15T09:00:00Z",
+            3,
+            &[],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let id = first.report.event_id.unwrap();
+    app.log(
+        &guest,
+        "2026-01-15T09:30:00Z",
+        4,
+        &[],
+        EventChoice::Join(id),
+    )
+    .await;
+    let late = app
+        .log(&me, "2026-01-15T13:45:00Z", 6, &[], EventChoice::Join(id))
+        .await;
+    let path = format!("/events/{id}");
+
+    let resident = app.sign_in("daniele").await;
+    let page = Page::parse(&text(app.send(get_with(&path, &resident)).await).await);
+    let form = format!(
+        r#".gap form[method=post][action="/reports/{}/move"]"#,
+        late.report.id
+    );
+    assert!(page.exists(&format!(
+        "{form} input[type=hidden][name=target][value=new]"
+    )));
+    assert_eq!(
+        page.text(&format!("{form} button")),
+        "Move 14:45 to a new event"
+    );
+
+    let guest = app.sign_in("guest").await;
+    let page = Page::parse(&text(app.send(get_with(&path, &guest)).await).await);
+    assert!(
+        !page.exists(".gap"),
+        "the guest sees only their own report: no gap"
+    );
+
+    let moved = app
+        .send(post_form(
+            &format!("/reports/{}/move", late.report.id),
+            "target=new",
+            Some(&resident),
+        ))
+        .await;
+    assert_eq!(
+        location(&moved),
+        Some(format!("/reports/{}", late.report.id).as_str())
+    );
+    let report = app.store.report(late.report.id).await.unwrap().unwrap();
+    assert_ne!(report.event_id, Some(id), "a new event");
+    let page = Page::parse(&text(app.send(get_with(&path, &resident)).await).await);
+    assert!(!page.exists(".gap"), "the gap is gone with the report");
+}
+
+#[tokio::test]
+async fn the_gap_warning_offers_no_move_without_permission_or_for_a_check() {
+    let app = TestApp::new().await;
+    app.at("2026-01-15T20:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let guest = app.add_user("guest", Role::Guest, Locale::En).await;
+    app.add_user("lawyer", Role::Viewer, Locale::En).await;
+    let first = app
+        .log(
+            &guest,
+            "2026-01-15T09:00:00Z",
+            3,
+            &[],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let id = first.report.event_id.unwrap();
+    app.log(
+        &guest,
+        "2026-01-15T13:00:00Z",
+        5,
+        &[],
+        EventChoice::Join(id),
+    )
+    .await;
+    let other = app
+        .log(
+            &me,
+            "2026-01-15T14:00:00Z",
+            2,
+            &[],
+            EventChoice::New {
+                replacing: Some(id),
+            },
+        )
+        .await;
+    let other_id = other.report.event_id.unwrap();
+    app.log_values(
+        &me,
+        "2026-01-15T18:00:00Z",
+        vec![],
+        &[],
+        EventChoice::Join(other_id),
+    )
+    .await;
+    let path = format!("/events/{id}");
+
+    let viewer = app.sign_in("lawyer").await;
+    let page = Page::parse(&text(app.send(get_with(&path, &viewer)).await).await);
+    assert!(page.exists(".gap"), "viewers see the warning");
+    assert!(!page.exists(".gap form"), "but may not move reports");
+
+    let resident = app.sign_in("daniele").await;
+    let page = Page::parse(
+        &text(
+            app.send(get_with(&format!("/events/{other_id}"), &resident))
+                .await,
+        )
+        .await,
+    );
+    assert!(page.exists(".gap"), "a check after a gap");
+    assert!(
+        !page.exists(".gap form"),
+        "a no-smell report cannot start an event"
+    );
+}
+
 #[tokio::test]
 async fn guests_see_only_their_own_entries_and_not_other_peoples_events() {
     let app = TestApp::new().await;
````

### Task 10: Report page and event prompts

**Proto commit:** `e7eed83` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show e7eed83`)

Spec §4 (Report detail, Prompts). Back link to the event, tags, facts as label/value lines, Correct a mistake full width, Move card, History card; prompts as one card with full-width answer buttons. CSS 69.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/69-facts.css
M	crates/web/src/assets.rs
M	crates/web/templates/prompt.html
M	crates/web/templates/report.html
M	crates/web/tests/logging.rs
M	crates/web/tests/reports.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/logging.rs`, `crates/web/tests/reports.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **617 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(reports): report page and event prompts as cards

A report's page has a back link to its event, the edited and deleted tags, its facts as
label and value lines, Correct a mistake as a full-width secondary button, the Move card and
the History card. The event questions (same smell, which event, auto-close) are one card
with their answers as full-width buttons.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `e7eed83`):

````diff
diff --git a/crates/web/css/69-facts.css b/crates/web/css/69-facts.css
new file mode 100644
index 0000000..e98704e
--- /dev/null
+++ b/crates/web/css/69-facts.css
@@ -0,0 +1,34 @@
+/* .facts: a record's label and value on one line each. .history: its audit trail. */
+@layer components {
+  .facts > div {
+    display: grid;
+    grid-template-columns: 7.5rem 1fr;
+    gap: var(--space-3);
+    padding-block: var(--space-2);
+  }
+
+  .facts > div + div {
+    border-block-start: 1px solid var(--line);
+  }
+
+  .facts dt {
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+
+  .history {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-2);
+    list-style: none;
+  }
+
+  .history .muted {
+    display: block;
+    font-size: var(--text-label);
+  }
+
+  .card h1 {
+    font-size: var(--text-brand);
+  }
+}
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index c85205d..e9cbcef 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -61,6 +61,7 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "66-room.css",
     "67-switcher.css",
     "68-timeline.css",
+    "69-facts.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/templates/prompt.html b/crates/web/templates/prompt.html
index 07023cc..105b390 100644
--- a/crates/web/templates/prompt.html
+++ b/crates/web/templates/prompt.html
@@ -1,18 +1,18 @@
 {% extends "base.html" %}
 {% block title %}{{ title }}{% endblock %}
 {% block content %}
-<h1>{{ title }}</h1>
-{% if let Some(detail) = detail %}
-<p>{{ detail }}</p>
-{% endif %}
-<form method="post" action="/log" class="stack">
-  {% for (name, value) in hidden %}
-  <input type="hidden" name="{{ name }}" value="{{ value }}">
-  {% endfor %}
-  <input type="hidden" name="asked" value="{{ asked }}">
-  {% for answer in answers %}
-  <button type="submit" name="choice" value="{{ answer.choice }}"{% if !answer.primary %} class="secondary"{% endif %}>{{ answer.label }}</button>
-  {% endfor %}
-</form>
-<p><a href="/">{{ tr.t("prompt-cancel") }}</a></p>
+<section class="card">
+  <h1>{{ title }}</h1>
+  {% if let Some(detail) = detail %}<p class="muted">{{ detail }}</p>{% endif %}
+  <form method="post" action="/log" class="stack">
+    {% for (name, value) in hidden %}
+    <input type="hidden" name="{{ name }}" value="{{ value }}">
+    {% endfor %}
+    <input type="hidden" name="asked" value="{{ asked }}">
+    {% for answer in answers %}
+    <button type="submit" name="choice" value="{{ answer.choice }}" class="button"{% if !answer.primary %} data-variant="secondary"{% endif %}>{{ answer.label }}</button>
+    {% endfor %}
+  </form>
+  <a href="/" class="button" data-variant="quiet">{{ tr.t("prompt-cancel") }}</a>
+</section>
 {% endblock %}
diff --git a/crates/web/templates/report.html b/crates/web/templates/report.html
index d4f6388..bcf7c93 100644
--- a/crates/web/templates/report.html
+++ b/crates/web/templates/report.html
@@ -1,51 +1,46 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("report-title") }}{% endblock %}
 {% block content %}
-<h1>{{ tr.t("report-title") }}{% if edited %} <span class="badge">{{ tr.t("report-edited") }}</span>{% endif %}{% if deleted %} <span class="badge">{{ tr.t("report-deleted") }}</span>{% endif %}</h1>
-{% if let Some(error) = error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
+{% if let Some(event) = event %}
+{% call c::back("/events/{}"|format(event.id), tr.t1("event-title", "date", event.date)) %}{% endcall %}
+{% else %}
+{% call c::back("/events", tr.t("events-title")) %}{% endcall %}
 {% endif %}
-<dl class="facts">
-  <dt>{{ tr.t("report-event") }}</dt>
-  <dd>{% if let Some(event) = event %}<a href="/events/{{ event.id }}">{{ tr.t1("event-title", "date", event.date) }}</a>{% else %}{{ tr.t("report-no-event") }}{% endif %}</dd>
-  <dt>{{ tr.t("report-observed") }}</dt>
-  <dd>{{ observed }}</dd>
-  <dt>{{ tr.t("report-recorded") }}</dt>
-  <dd>{{ recorded }}</dd>
-  <dt>{{ tr.t("log-intensity") }}</dt>
-  <dd>{{ intensity }}</dd>
-  <dt>{{ tr.t("log-rooms") }}</dt>
-  <dd>{{ rooms }}</dd>
-  <dt>{{ tr.t("log-symptoms") }}</dt>
-  <dd>{{ symptoms }}</dd>
-  {% if let Some(note) = note %}
-  <dt>{{ tr.t("report-note") }}</dt>
-  <dd>{{ note }}</dd>
-  {% endif %}
-  {% if let Some(by) = by %}
-  <dt>{{ tr.t("report-by") }}</dt>
-  <dd>{{ by }}</dd>
-  {% endif %}
-</dl>
+<h1>{{ tr.t("report-title") }}{% if edited %} <span class="tag">{{ tr.t("report-edited") }}</span>{% endif %}{% if deleted %} <span class="tag" data-tone="warn">{{ tr.t("report-deleted") }}</span>{% endif %}</h1>
+{% if let Some(error) = error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+<section class="card">
+  <dl class="facts">
+    <div><dt>{{ tr.t("report-event") }}</dt><dd>{% if let Some(event) = event %}<a href="/events/{{ event.id }}">{{ tr.t1("event-title", "date", event.date) }}</a>{% else %}{{ tr.t("report-no-event") }}{% endif %}</dd></div>
+    <div><dt>{{ tr.t("report-observed") }}</dt><dd>{{ observed }}</dd></div>
+    <div><dt>{{ tr.t("report-recorded") }}</dt><dd>{{ recorded }}</dd></div>
+    <div><dt>{{ tr.t("log-intensity") }}</dt><dd>{{ intensity }}</dd></div>
+    <div><dt>{{ tr.t("log-rooms") }}</dt><dd>{{ rooms }}</dd></div>
+    <div><dt>{{ tr.t("log-symptoms") }}</dt><dd>{{ symptoms }}</dd></div>
+    {% if let Some(note) = note %}<div><dt>{{ tr.t("report-note") }}</dt><dd>{{ note }}</dd></div>{% endif %}
+    {% if let Some(by) = by %}<div><dt>{{ tr.t("report-by") }}</dt><dd>{{ by }}</dd></div>{% endif %}
+  </dl>
+</section>
 {% if editable %}
-<p><a href="/reports/{{ id }}/correct">{{ tr.t("report-correct") }}</a></p>
+<a href="/reports/{{ id }}/correct" class="button" data-variant="secondary">{{ tr.t("report-correct") }}</a>
 {% if !move_options.is_empty() %}
-<form method="post" action="/reports/{{ id }}/move" class="stack">
-  <label>{{ tr.t("report-move") }}
-    <select name="target">
-      {% for option in move_options %}
-      <option value="{{ option.value }}">{{ option.label }}</option>
-      {% endfor %}
-    </select>
-  </label>
-  <button type="submit" class="secondary">{{ tr.t("report-move-submit") }}</button>
+{% call c::card(tr.t("report-move")) %}
+<form method="post" action="/reports/{{ id }}/move" class="split">
+  <select name="target" aria-label="{{ tr.t("report-move") }}">
+    {% for option in move_options %}
+    <option value="{{ option.value }}">{{ option.label }}</option>
+    {% endfor %}
+  </select>
+  <button type="submit" class="button">{{ tr.t("report-move-submit") }}</button>
 </form>
+{% endcall %}
 {% endif %}
 {% endif %}
-<h2>{{ tr.t("report-history") }}</h2>
+{% call c::card(tr.t("report-history")) %}
 <ol class="history">
   {% for line in history %}
-  <li>{{ line.at }}{% if let Some(by) = line.by %} · {{ by }}{% endif %} · {{ tr.t(line.action) }}{% if !line.changes.is_empty() %}: {{ line.changes.join(", ") }}{% endif %}</li>
+  <li><span class="muted">{{ line.at }}{% if let Some(by) = line.by %} · {{ by }}{% endif %}</span> {{ tr.t(line.action) }}{% if !line.changes.is_empty() %}: {{ line.changes.join(", ") }}{% endif %}</li>
   {% endfor %}
 </ol>
+{% endcall %}
 {% endblock %}
diff --git a/crates/web/tests/logging.rs b/crates/web/tests/logging.rs
index 6672507..cb82b98 100644
--- a/crates/web/tests/logging.rs
+++ b/crates/web/tests/logging.rs
@@ -10,6 +10,25 @@ use smoke_domain::repos::{EventRepo, ReadingRepo, ReportRepo};
 use smoke_domain::types::{CloseReason, EventId, Locale, ReportId, Role};
 
 /// The value of the hidden `asked` field on a question page.
+/// The prompt's answers: the value each button posts, its label, and whether it is the
+/// primary one.
+fn answers(html: &str) -> Vec<(String, String, bool)> {
+    let page = Page::parse(html);
+    let buttons = "form[action='/log'] button[name=choice]";
+    let values = page.attrs(buttons, "value");
+    let labels = page.texts(buttons);
+    values
+        .into_iter()
+        .zip(labels)
+        .enumerate()
+        .map(|(index, (value, label))| {
+            let selector = format!("{buttons}:nth-of-type({})", index + 1);
+            let primary = !page.exists(&format!("{selector}[data-variant=secondary]"));
+            (value, label, primary)
+        })
+        .collect()
+}
+
 fn asked(html: &str) -> String {
     let start = html.find(r#"name="asked" value=""#).expect("a question") + 20;
     html[start..start + html[start..].find('"').unwrap()].to_owned()
@@ -70,8 +89,17 @@ async fn with_an_event_open_the_same_smell_question_comes_first() {
     assert!(html.contains("(started 19:05)"), "{html}");
     assert!(html.contains(r#"<input type="hidden" name="level-1" value="7">"#));
     let id = first.report.event_id.unwrap();
-    assert!(html.contains(&format!(r#"value="join-{id}">Yes, update</button>"#)));
-    assert!(html.contains(r#"value="new" class="secondary">No, it had stopped: new event"#));
+    assert_eq!(
+        answers(&html),
+        [
+            (format!("join-{id}"), "Yes, update".to_owned(), true),
+            (
+                "new".to_owned(),
+                "No, it had stopped: new event".to_owned(),
+                false
+            ),
+        ]
+    );
 
     let body = format!(
         "room=1&level-1=7&action=log&asked={}&choice=join-{id}",
@@ -177,9 +205,7 @@ async fn a_guest_sees_only_the_start_time_of_someone_elses_event() {
     assert!(past.contains("<h1>Which event was this?</h1>"), "{past}");
     let earlier = closed_earlier.report.event_id.unwrap();
     assert!(
-        past.contains(&format!(
-            r#"value="join-{earlier}" class="secondary">Started 11:00</button>"#
-        )),
+        answers(&past).contains(&(format!("join-{earlier}"), "Started 11:00".to_owned(), false)),
         "{past}"
     );
     assert!(
@@ -293,7 +319,7 @@ async fn a_past_report_after_the_end_of_the_chosen_event_moves_its_end() {
         !html.contains("Started 21:00"),
         "not started by then: {html}"
     );
-    assert!(html.contains(r#"value="new" class="secondary">New event</button>"#));
+    assert!(answers(&html).contains(&("new".to_owned(), "New event".to_owned(), false)));
 
     let body = format!(
         "room=1&level-1=5&action=log&observed_at=2026-01-15T12:00&asked=which-event&choice=join-{id}"
diff --git a/crates/web/tests/reports.rs b/crates/web/tests/reports.rs
index d880e2a..4b9640a 100644
--- a/crates/web/tests/reports.rs
+++ b/crates/web/tests/reports.rs
@@ -43,9 +43,9 @@ async fn a_report_shows_both_times_its_values_and_its_history() {
     assert!(html.contains("<dd>6 · Strong</dd>"), "{html}");
     assert!(html.contains("<dd>Bedroom 6</dd>"), "{html}");
     assert!(html.contains("<dd>daniele</dd>"), "{html}");
-    assert!(
-        html.contains("<li>15/01/2026 21:00 · daniele · logged</li>"),
-        "{html}"
+    assert_eq!(
+        Page::parse(&html).texts(".history li"),
+        ["15/01/2026 21:00 · daniele logged"]
     );
     assert!(!html.contains("edited"), "{html}");
     assert!(html.contains(">Correct a mistake</a>"), "{html}");
@@ -106,15 +106,11 @@ async fn correcting_a_mistake_is_recorded_and_badged() {
         .await;
     assert_eq!(location(&response), Some(&*format!("/reports/{id}")));
     let html = text(app.send(get_with(&format!("/reports/{id}"), &cookie)).await).await;
-    assert!(
-        html.contains(r#"<span class="badge">edited</span>"#),
-        "{html}"
-    );
-    assert!(
-        html.contains(
-            "<li>16/01/2026 09:15 · daniele · corrected: Bedroom 6 → 7, added Kitchen 3</li>"
-        ),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(page.text("h1 .tag"), "edited");
+    assert_eq!(
+        page.texts(".history li").last().map(String::as_str),
+        Some("16/01/2026 09:15 · daniele corrected: Bedroom 6 → 7, added Kitchen 3")
     );
 }
 
````

### Task 11: Settings

**Proto commit:** `24f48fb` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 24f48fb`)

Spec §4 (Settings). Password, Rooms and Symptoms as collapsible cards (open on their error), hidden items struck through with Show again, the room area as a segmented pair, Guides as rows. `ListItem.detail` is the translated area; the list line is a macro of `settings.html`; `list_item.html` is deleted. CSS 70; keys `settings-change` (later renamed, Task 16), `settings-room-area`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/70-items.css
D	crates/web/templates/list_item.html
M	crates/web/css/57-row.css
M	crates/web/css/67-switcher.css
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/assets.rs
M	crates/web/src/routes/lists.rs
M	crates/web/templates/settings.html
M	crates/web/tests/guides.rs
M	crates/web/tests/settings.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/guides.rs`, `crates/web/tests/settings.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **617 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(settings): Settings as cards: Install, Language, and Password, Rooms and Symptoms that fold

The language buttons form a segmented control; Password, Rooms and Symptoms are
collapsible cards that open on their own error; a hidden room or symptom is struck through
with Show again; the room area is a segmented pair of radios; Guides lists its two links as
rows. The list line is a macro of the page; ListItem carries its area already translated.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `24f48fb`):

````diff
diff --git a/crates/web/css/57-row.css b/crates/web/css/57-row.css
index 0a9f4b5..3fdd64e 100644
--- a/crates/web/css/57-row.css
+++ b/crates/web/css/57-row.css
@@ -1,5 +1,14 @@
-/* .row: one line of a list card, divided from the next by a hairline. */
+/* .rows: a card of list lines. .row: one line, divided from the next by a hairline. */
 @layer components {
+  .rows {
+    gap: 0;
+    padding-block: var(--space-1);
+  }
+
+  .rows > h2 {
+    padding-block: var(--space-2);
+  }
+
   .row {
     display: flex;
     align-items: center;
diff --git a/crates/web/css/67-switcher.css b/crates/web/css/67-switcher.css
index d4ac741..978478b 100644
--- a/crates/web/css/67-switcher.css
+++ b/crates/web/css/67-switcher.css
@@ -23,9 +23,4 @@
     letter-spacing: 0.06em;
     text-transform: uppercase;
   }
-
-  .rows {
-    gap: 0;
-    padding-block: var(--space-1);
-  }
 }
diff --git a/crates/web/css/70-items.css b/crates/web/css/70-items.css
new file mode 100644
index 0000000..20a5e7d
--- /dev/null
+++ b/crates/web/css/70-items.css
@@ -0,0 +1,19 @@
+/* .items: the editable lists of Settings, one form per line. */
+@layer components {
+  .items {
+    list-style: none;
+  }
+
+  .items > li + li {
+    border-block-start: 1px solid var(--line);
+  }
+
+  .items .row {
+    min-block-size: var(--target);
+    padding-block: var(--space-1);
+  }
+
+  .items s {
+    color: var(--muted);
+  }
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 4e169bf..03eebb7 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -215,6 +215,8 @@ settings-password-wrong = The current password is not correct.
 settings-saved-language = Language saved.
 settings-saved-password = Password changed. Your other devices are signed out.
 settings-guides = Guides
+settings-change = Change
+settings-room-area = Area
 settings-lists-hint = Hidden items are no longer offered on the forms; reports that name them keep them.
 settings-list-hidden = hidden
 settings-list-hide = Hide
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 49b10d7..af069ac 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -215,6 +215,8 @@ settings-password-wrong = La password attuale non è corretta.
 settings-saved-language = Lingua salvata.
 settings-saved-password = Password cambiata. Gli altri dispositivi sono stati disconnessi.
 settings-guides = Guide
+settings-change = Cambia
+settings-room-area = Zona
 settings-lists-hint = Le voci nascoste non sono più proposte nei moduli; le segnalazioni che le citano le mantengono.
 settings-list-hidden = nascosto
 settings-list-hide = Nascondi
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index 2bf50cb..dda048c 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -215,6 +215,8 @@ settings-password-wrong = Mevcut şifre doğru değil.
 settings-saved-language = Dil kaydedildi.
 settings-saved-password = Şifre değiştirildi. Diğer cihazlarınızda oturum kapatıldı.
 settings-guides = Kılavuzlar
+settings-change = Değiştir
+settings-room-area = Alan
 settings-lists-hint = Gizlenen öğeler artık formlarda sunulmaz; onları içeren kayıtlar korunur.
 settings-list-hidden = gizli
 settings-list-hide = Gizle
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index e9cbcef..8ec596b 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -62,6 +62,7 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "67-switcher.css",
     "68-timeline.css",
     "69-facts.css",
+    "70-items.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/src/routes/lists.rs b/crates/web/src/routes/lists.rs
index c53e0a8..0ee3b05 100644
--- a/crates/web/src/routes/lists.rs
+++ b/crates/web/src/routes/lists.rs
@@ -23,8 +23,8 @@ pub(crate) struct ListItem {
     pub(crate) label: String,
     /// Where its Hide / Show button posts.
     pub(crate) action: String,
-    /// The translation key of a room's area; `None` for symptoms.
-    pub(crate) area: Option<&'static str>,
+    /// A room's area; empty for symptoms.
+    pub(crate) detail: String,
     pub(crate) hidden: bool,
 }
 
@@ -50,7 +50,7 @@ pub(crate) async fn view<S: Store>(state: &AppState<S>, tr: Tr) -> Result<ListsV
             .map(|room| ListItem {
                 action: format!("/settings/rooms/{}/hidden", room.id),
                 label: labels::room(tr, room),
-                area: Some(area_key(room.area)),
+                detail: tr.t(area_key(room.area)),
                 hidden: room.hidden_at.is_some(),
             })
             .collect(),
@@ -59,7 +59,7 @@ pub(crate) async fn view<S: Store>(state: &AppState<S>, tr: Tr) -> Result<ListsV
             .map(|symptom| ListItem {
                 action: format!("/settings/symptoms/{}/hidden", symptom.id),
                 label: labels::symptom(tr, symptom),
-                area: None,
+                detail: String::new(),
                 hidden: symptom.hidden_at.is_some(),
             })
             .collect(),
diff --git a/crates/web/templates/list_item.html b/crates/web/templates/list_item.html
deleted file mode 100644
index ee8f1b7..0000000
--- a/crates/web/templates/list_item.html
+++ /dev/null
@@ -1,13 +0,0 @@
-{# A room or symptom in Settings, with the button that hides or shows it again. Expects `item`. #}
-<li>
-  <form method="post" action="{{ item.action }}" class="item">
-    <span>{{ item.label }}{% if let Some(area) = item.area %} · {{ tr.t(area) }}{% endif %}{% if item.hidden %} · <em>{{ tr.t("settings-list-hidden") }}</em>{% endif %}</span>
-    {% if item.hidden %}
-    <input type="hidden" name="hidden" value="false">
-    <button type="submit" class="link">{{ tr.t("settings-list-show") }}</button>
-    {% else %}
-    <input type="hidden" name="hidden" value="true">
-    <button type="submit" class="link">{{ tr.t("settings-list-hide") }}</button>
-    {% endif %}
-  </form>
-</li>
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
index d2990d2..425c752 100644
--- a/crates/web/templates/settings.html
+++ b/crates/web/templates/settings.html
@@ -1,11 +1,21 @@
 {% extends "base.html" %}
 {% import "components.html" as c %}
 {% block title %}{{ tr.t("settings-title") }}{% endblock %}
+
+{# A room or symptom of the lists, with the button that hides it or shows it again. #}
+{% macro list_item(item, hidden, hide, show) %}
+<li>
+  <form method="post" action="{{ item.action }}" class="row">
+    <span class="row-text">{% if item.hidden %}<s>{{ item.label }}</s><span class="visually-hidden">{{ hidden }}</span>{% else %}<strong>{{ item.label }}</strong>{% endif %}{% if !item.detail.is_empty() %}<span>{{ item.detail }}</span>{% endif %}</span>
+    <input type="hidden" name="hidden" value="{{ !item.hidden }}">
+    <button type="submit" class="button" data-variant="quiet">{% if item.hidden %}{{ show }}{% else %}{{ hide }}{% endif %}</button>
+  </form>
+</li>
+{% endmacro %}
+
 {% block content %}
 <h1>{{ tr.t("settings-title") }}</h1>
-{% if let Some(saved) = notices.saved %}
-<p class="notice" role="status">{{ tr.t(saved) }}</p>
-{% endif %}
+{% if let Some(saved) = notices.saved %}{% call c::notice(tr.t(saved), false) %}{% endcall %}{% endif %}
 <section class="card install">
   <div class="split">
     <span class="row-text"><strong>{{ tr.t("install-app") }}</strong><span>{{ tr.t("install-card-hint") }}</span></span>
@@ -15,67 +25,49 @@
 {% call c::card(tr.t("language")) %}
 {% call c::languages(tr.languages(), tr.t("language"), "settings") %}{% endcall %}
 {% endcall %}
-<section>
-  <h2>{{ tr.t("settings-password") }}</h2>
-  {% if let Some(error) = notices.password_error %}
-  <p class="notice" role="alert">{{ tr.t(error) }}</p>
-  {% endif %}
-  <form method="post" action="/settings/password" class="stack">
-    <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
-    <label>{{ tr.t("settings-password-current") }}
-      <input type="password" name="current" autocomplete="current-password" maxlength="128" required>
-    </label>
-    <label>{{ tr.t("settings-password-new") }}
-      <input type="password" name="new" autocomplete="new-password" minlength="10" maxlength="128" required>
-    </label>
-    <label>{{ tr.t("settings-password-repeat") }}
-      <input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>
-    </label>
-    <p class="hint">{{ tr.t("settings-password-hint") }}</p>
-    <button type="submit">{{ tr.t("settings-password-submit") }}</button>
-  </form>
-</section>
+{% call c::collapsible(tr.t("settings-password"), "", tr.t("settings-change"), notices.password_error.is_some()) %}
+{% if let Some(error) = notices.password_error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+<form method="post" action="/settings/password" class="stack">
+  <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
+  {% call c::field(tr.t("settings-password-current")) %}<input type="password" name="current" autocomplete="current-password" maxlength="128" required>{% endcall %}
+  {% call c::field(tr.t("settings-password-new")) %}<input type="password" name="new" autocomplete="new-password" minlength="10" maxlength="128" required>{% endcall %}
+  {% call c::field(tr.t("settings-password-repeat")) %}<input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>{% endcall %}
+  <p class="hint">{{ tr.t("settings-password-hint") }}</p>
+  <button type="submit" class="button">{{ tr.t("settings-password-submit") }}</button>
+</form>
+{% endcall %}
 {% if let Some(lists) = lists %}
-<section id="lists">
-  <h2>{{ tr.t("log-rooms") }}</h2>
-  <p class="hint">{{ tr.t("settings-lists-hint") }}</p>
-  {% if let Some(error) = notices.list_error %}
-  <p class="notice" role="alert">{{ tr.t(error) }}</p>
-  {% endif %}
-  <ul class="items">
-    {% for item in lists.rooms %}
-    {% include "list_item.html" %}
-    {% endfor %}
-  </ul>
-  <form method="post" action="/settings/rooms" class="stack">
-    <label>{{ tr.t("settings-room-add") }}
-      <input name="name" maxlength="60" required>
-    </label>
-    <fieldset class="choices">
-      <label><input type="radio" name="area" value="inside" checked> {{ tr.t("log-inside") }}</label>
-      <label><input type="radio" name="area" value="common"> {{ tr.t("log-common") }}</label>
-    </fieldset>
-    <button type="submit" class="secondary">{{ tr.t("settings-list-add") }}</button>
-  </form>
-  <h2>{{ tr.t("log-symptoms") }}</h2>
-  <ul class="items">
-    {% for item in lists.symptoms %}
-    {% include "list_item.html" %}
-    {% endfor %}
-  </ul>
-  <form method="post" action="/settings/symptoms" class="stack">
-    <label>{{ tr.t("settings-symptom-add") }}
-      <input name="name" maxlength="60" required>
-    </label>
-    <button type="submit" class="secondary">{{ tr.t("settings-list-add") }}</button>
-  </form>
-</section>
+{% call c::collapsible(tr.t("log-rooms"), "", tr.t("settings-change"), notices.list_error.is_some()) %}
+{% if let Some(error) = notices.list_error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+<p class="hint">{{ tr.t("settings-lists-hint") }}</p>
+<ul class="items">
+  {% for item in lists.rooms %}{% call list_item(item, tr.t("settings-list-hidden"), tr.t("settings-list-hide"), tr.t("settings-list-show")) %}{% endcall %}{% endfor %}
+</ul>
+<form method="post" action="/settings/rooms" class="stack">
+  <div class="split">
+    <input name="name" maxlength="60" required aria-label="{{ tr.t("settings-room-add") }}" placeholder="{{ tr.t("settings-room-add") }}">
+    <button type="submit" class="button" data-variant="secondary">{{ tr.t("settings-list-add") }}</button>
+  </div>
+  <fieldset class="segmented">
+    <legend class="visually-hidden">{{ tr.t("settings-room-area") }}</legend>
+    {% call c::chip("radio", "area", "inside", tr.t("log-inside"), true) %}{% endcall %}
+    {% call c::chip("radio", "area", "common", tr.t("log-common"), false) %}{% endcall %}
+  </fieldset>
+</form>
+{% endcall %}
+{% call c::collapsible(tr.t("log-symptoms"), "", tr.t("settings-change"), notices.list_error.is_some()) %}
+<ul class="items">
+  {% for item in lists.symptoms %}{% call list_item(item, tr.t("settings-list-hidden"), tr.t("settings-list-hide"), tr.t("settings-list-show")) %}{% endcall %}{% endfor %}
+</ul>
+<form method="post" action="/settings/symptoms" class="split">
+  <input name="name" maxlength="60" required aria-label="{{ tr.t("settings-symptom-add") }}" placeholder="{{ tr.t("settings-symptom-add") }}">
+  <button type="submit" class="button" data-variant="secondary">{{ tr.t("settings-list-add") }}</button>
+</form>
+{% endcall %}
 {% endif %}
-<section>
+<section class="card rows">
   <h2>{{ tr.t("settings-guides") }}</h2>
-  <ul class="rows">
-    <li><a href="/guide/rating">{{ tr.t("rating-title") }}</a></li>
-    <li><a href="/guide/logging">{{ tr.t("guide-log-title") }}</a></li>
-  </ul>
+  {% call c::row("/guide/rating", tr.t("rating-title"), "") %}{% endcall %}
+  {% call c::row("/guide/logging", tr.t("guide-log-title"), "") %}{% endcall %}
 </section>
 {% endblock %}
diff --git a/crates/web/tests/guides.rs b/crates/web/tests/guides.rs
index 65b7646..58a5a9a 100644
--- a/crates/web/tests/guides.rs
+++ b/crates/web/tests/guides.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use smoke_domain::types::{Locale, Role};
 
@@ -35,13 +36,15 @@ async fn settings_and_the_rating_guide_lead_to_both_guides() {
     let app = TestApp::new().await;
     app.add_user("daniele", Role::Resident, Locale::En).await;
     let cookie = app.sign_in("daniele").await;
-    let settings = text(app.send(get_with("/settings", &cookie)).await).await;
-    for link in [
-        r#"<a href="/guide/rating">How to rate</a>"#,
-        r#"<a href="/guide/logging">How to log</a>"#,
-    ] {
-        assert!(settings.contains(link), "{link}: {settings}");
-    }
+    let settings = Page::parse(&text(app.send(get_with("/settings", &cookie)).await).await);
+    assert_eq!(
+        settings.attrs("section.rows a.row", "href"),
+        ["/guide/rating", "/guide/logging"]
+    );
+    assert_eq!(
+        settings.texts("section.rows a.row"),
+        ["How to rate", "How to log"]
+    );
     let rating = text(app.send(get_with("/guide/rating", &cookie)).await).await;
     assert!(
         rating.contains(r#"<a href="/guide/logging">How to log</a>"#),
diff --git a/crates/web/tests/settings.rs b/crates/web/tests/settings.rs
index c7f1f37..9a471d7 100644
--- a/crates/web/tests/settings.rs
+++ b/crates/web/tests/settings.rs
@@ -194,12 +194,16 @@ async fn residents_add_rooms_and_symptoms_and_hide_them_from_the_forms() {
     let log = text(app.send(get_with("/", &cookie)).await).await;
     assert!(!log.contains("Balcony"), "{log}");
     let settings = text(app.send(get_with("/settings?saved=lists", &cookie)).await).await;
-    assert!(settings.contains("Lists saved."), "{settings}");
-    assert!(
-        settings.contains("Balcony · Common areas · <em>hidden</em>"),
-        "{settings}"
+    let page = Page::parse(&settings);
+    assert_eq!(page.text(".notice[role=status]"), "Lists saved.");
+    let balcony_row = format!(r#".items form[action="{hide}"]"#);
+    assert_eq!(page.text(&format!("{balcony_row} s")), "Balcony");
+    assert_eq!(
+        page.text(&format!("{balcony_row} .row-text")),
+        "Balcony hidden Common areas"
     );
-    assert!(settings.contains(">Show again</button>"), "{settings}");
+    assert_eq!(page.text(&format!("{balcony_row} button")), "Show again");
+    assert!(page.exists(&format!("{balcony_row} input[name=hidden][value=false]")));
 
     app.send(post_form(&hide, "hidden=false", Some(&cookie)))
         .await;
````

### Task 12: People, invite, QR link and account setup

**Proto commit:** `f102c9b` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show f102c9b`)

Spec §4 (People and the single-card pages). `header::initial` shared by the header and People; `PersonRow { initial, tone }`; `expiry::fields(tr, …)` translates its labels and the `expiry` macro replaces `expiry_fields.html`; `.avatar` becomes a shared style. CSS 71.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/71-people.css
D	crates/web/templates/expiry_fields.html
M	crates/web/css/56-badge.css
M	crates/web/css/61-menu.css
M	crates/web/src/assets.rs
M	crates/web/src/header.rs
M	crates/web/src/routes/people/expiry.rs
M	crates/web/src/routes/people/links.rs
M	crates/web/src/routes/people/mod.rs
M	crates/web/templates/accept.html
M	crates/web/templates/components.html
M	crates/web/templates/invite.html
M	crates/web/templates/link.html
M	crates/web/templates/people.html
M	crates/web/tests/people.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/people.rs`, `crates/web/src/routes/people/expiry.rs (tests)`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **617 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(people): People, invite, QR link and account setup as cards

One card per person: avatar, name, status line and role tag (resident green, guest amber,
viewer grey), outlined actions with End access now in red, and the reactivation fields of an
expired account. The expiry fields become a components.html macro (labels translated in
expiry::fields); the invite, QR and setup pages are single cards; header::initial gives the
avatar letter to the header and the People list.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `f102c9b`):

````diff
diff --git a/crates/web/css/56-badge.css b/crates/web/css/56-badge.css
index 9d8a426..2f267a9 100644
--- a/crates/web/css/56-badge.css
+++ b/crates/web/css/56-badge.css
@@ -1,6 +1,19 @@
-/* .badge: the square in front of a list line. .tag: a small pill after a name.
-   data-tone="warn" or "muted" changes either. */
+/* .badge: the square in front of a list line. .tag: a small pill after a name. .avatar: a
+   person's initial in a circle. data-tone="warn" or "muted" changes a badge or a tag. */
 @layer components {
+  .avatar {
+    display: grid;
+    place-items: center;
+    flex-shrink: 0;
+    inline-size: var(--target);
+    block-size: var(--target);
+    border-radius: var(--radius-pill);
+    background: var(--accent-soft);
+    color: var(--accent);
+    font-size: var(--text-card);
+    font-weight: 700;
+  }
+
   .badge {
     display: inline-flex;
     align-items: center;
diff --git a/crates/web/css/61-menu.css b/crates/web/css/61-menu.css
index 9e15034..e8e6507 100644
--- a/crates/web/css/61-menu.css
+++ b/crates/web/css/61-menu.css
@@ -1,24 +1,15 @@
-/* .menu: the avatar and the small panel it opens (a <details>). */
+/* .menu: the avatar in the header and the small panel it opens (a <details>). */
 @layer components {
   .menu {
     position: relative;
   }
 
-  .avatar {
-    display: grid;
-    place-items: center;
-    inline-size: var(--target);
-    block-size: var(--target);
-    border-radius: var(--radius-pill);
-    background: var(--accent-soft);
-    color: var(--accent);
-    font-size: var(--text-card);
-    font-weight: 700;
+  summary.avatar {
     list-style: none;
     cursor: pointer;
   }
 
-  .avatar::-webkit-details-marker {
+  summary.avatar::-webkit-details-marker {
     display: none;
   }
 
diff --git a/crates/web/css/71-people.css b/crates/web/css/71-people.css
new file mode 100644
index 0000000..1110b4f
--- /dev/null
+++ b/crates/web/css/71-people.css
@@ -0,0 +1,32 @@
+/* People: one card per person, led by the avatar, name, status and role tag; the invite form
+   shows the expiry fields of the role picked (CSS :has, no script). */
+@layer components {
+  .person {
+    display: flex;
+    align-items: center;
+    gap: var(--space-3);
+  }
+
+  .invite:has(input[name="role"]:checked) .expiry {
+    display: none;
+  }
+
+  .invite:has(input[name="role"][value="guest"]:checked) .expiry[data-role="guest"],
+  .invite:has(input[name="role"][value="viewer"]:checked) .expiry[data-role="viewer"] {
+    display: flex;
+  }
+
+  .qr-code {
+    inline-size: 100%;
+    max-inline-size: 18rem;
+    margin-inline: auto;
+    padding: var(--space-3);
+    border-radius: var(--radius-field);
+    background: var(--field);
+  }
+
+  .qr {
+    inline-size: 100%;
+    block-size: auto;
+  }
+}
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index 8ec596b..e2fe5c1 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -63,6 +63,7 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "68-timeline.css",
     "69-facts.css",
     "70-items.css",
+    "71-people.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/src/header.rs b/crates/web/src/header.rs
index 277b8c6..f50f6fe 100644
--- a/crates/web/src/header.rs
+++ b/crates/web/src/header.rs
@@ -76,16 +76,20 @@ pub(crate) struct SignedIn {
     tab: Tab,
 }
 
+/// A person's avatar: the first letter of the username, upper case.
+pub(crate) fn initial(username: &str) -> String {
+    username
+        .chars()
+        .take(1)
+        .flat_map(char::to_uppercase)
+        .collect()
+}
+
 impl SignedIn {
     pub(crate) fn of(user: &User, now: DateTime<Utc>, tab: Tab) -> Option<Self> {
         let username = user.username.to_string();
         Some(Self {
-            initial: username
-                .chars()
-                .next()
-                .into_iter()
-                .flat_map(char::to_uppercase)
-                .collect(),
+            initial: initial(&username),
             username,
             people: can(&user.actor(), Action::ManageUsers, now),
             tab,
diff --git a/crates/web/src/routes/people/expiry.rs b/crates/web/src/routes/people/expiry.rs
index d5df7f8..e00d127 100644
--- a/crates/web/src/routes/people/expiry.rs
+++ b/crates/web/src/routes/people/expiry.rs
@@ -6,6 +6,7 @@ use smoke_domain::people::ExpiryPick;
 use smoke_domain::types::Role;
 
 use crate::forms::Pairs;
+use crate::i18n::Tr;
 
 /// The guest quick picks: (form value, translation key).
 const GUEST_PICKS: [(&str, &str); 5] = [
@@ -22,12 +23,12 @@ const VIEWER_PICKS: [(&str, &str); 2] = [("never", "expiry-never"), ("date", "ex
 /// One radio button of the expiry fields.
 pub(crate) struct PickChoice {
     pub(crate) value: &'static str,
-    pub(crate) key: &'static str,
+    pub(crate) label: String,
     pub(crate) checked: bool,
 }
 
-/// The expiry fields for one role, named `<prefix>expiry` and `<prefix>expiry_date`
-/// (`templates/expiry_fields.html`).
+/// The expiry fields for one role, named `<prefix>expiry` and `<prefix>expiry_date` (the
+/// `expiry` macro of `templates/components.html`).
 pub(crate) struct ExpiryFields {
     pub(crate) role: &'static str,
     pub(crate) pick_name: String,
@@ -57,7 +58,12 @@ impl ExpiryDraft {
 
 /// The fields for `role`, with `draft` picked (the role's default when nothing is); `None` for
 /// residents, who never expire.
-pub(crate) fn fields(role: Role, prefix: &str, draft: &ExpiryDraft) -> Option<ExpiryFields> {
+pub(crate) fn fields(
+    tr: Tr,
+    role: Role,
+    prefix: &str,
+    draft: &ExpiryDraft,
+) -> Option<ExpiryFields> {
     let (picks, default): (&[(&'static str, &'static str)], &str) = match role {
         Role::Resident => return None,
         Role::Guest => (&GUEST_PICKS, "1w"),
@@ -72,7 +78,7 @@ pub(crate) fn fields(role: Role, prefix: &str, draft: &ExpiryDraft) -> Option<Ex
             .iter()
             .map(|&(value, key)| PickChoice {
                 value,
-                key,
+                label: tr.t(key),
                 checked: value == picked,
             })
             .collect(),
@@ -141,8 +147,12 @@ pub(crate) fn keys() -> Vec<&'static str> {
 
 #[cfg(test)]
 mod tests {
+    use smoke_domain::types::Locale;
+
     use super::*;
 
+    const EN: Tr = Tr(Locale::En);
+
     fn draft(pick: Option<&str>, date: &str) -> ExpiryDraft {
         ExpiryDraft {
             pick: pick.map(str::to_owned),
@@ -218,14 +228,15 @@ mod tests {
 
     #[test]
     fn the_fields_preselect_the_default_or_the_draft() {
-        let guest = fields(Role::Guest, "guest_", &ExpiryDraft::default()).unwrap();
+        let guest = fields(EN, Role::Guest, "guest_", &ExpiryDraft::default()).unwrap();
         assert_eq!(guest.pick_name, "guest_expiry");
         let checked: Vec<_> = guest.picks.iter().filter(|p| p.checked).collect();
         assert_eq!(checked.len(), 1);
         assert_eq!(checked[0].value, "1w");
-        let viewer = fields(Role::Viewer, "", &draft(Some("date"), "2026-02-01")).unwrap();
+        assert_eq!(checked[0].label, "in 1 week");
+        let viewer = fields(EN, Role::Viewer, "", &draft(Some("date"), "2026-02-01")).unwrap();
         assert_eq!(viewer.date, "2026-02-01");
         assert!(viewer.picks.iter().any(|p| p.value == "date" && p.checked));
-        assert!(fields(Role::Resident, "", &ExpiryDraft::default()).is_none());
+        assert!(fields(EN, Role::Resident, "", &ExpiryDraft::default()).is_none());
     }
 }
diff --git a/crates/web/src/routes/people/links.rs b/crates/web/src/routes/people/links.rs
index 91895f9..aab2cc0 100644
--- a/crates/web/src/routes/people/links.rs
+++ b/crates/web/src/routes/people/links.rs
@@ -96,8 +96,8 @@ fn invite_page(
                     checked: choice == role,
                 })
                 .collect(),
-            guest: expiry::fields(Role::Guest, "guest_", &draft.guest),
-            viewer: expiry::fields(Role::Viewer, "viewer_", &draft.viewer),
+            guest: expiry::fields(tr, Role::Guest, "guest_", &draft.guest),
+            viewer: expiry::fields(tr, Role::Viewer, "viewer_", &draft.viewer),
         },
     )
 }
diff --git a/crates/web/src/routes/people/mod.rs b/crates/web/src/routes/people/mod.rs
index 6569289..a85382f 100644
--- a/crates/web/src/routes/people/mod.rs
+++ b/crates/web/src/routes/people/mod.rs
@@ -20,7 +20,7 @@ use smoke_domain::types::{Role, UserId};
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
 use crate::forms::Pairs;
-use crate::header::{SignedIn, Tab};
+use crate::header::{SignedIn, Tab, initial};
 use crate::i18n::Tr;
 use crate::routes::people::expiry::{ExpiryDraft, ExpiryFields};
 use crate::routes::people_refusal;
@@ -31,7 +31,11 @@ use crate::time;
 pub(crate) struct PersonRow {
     pub(crate) id: i64,
     pub(crate) username: String,
+    /// The avatar's letter.
+    pub(crate) initial: String,
     pub(crate) role: &'static str,
+    /// The role tag's colour: residents green, guests amber, viewers grey.
+    pub(crate) tone: &'static str,
     /// "until 22/01/2026 19:05", "expired 15/01/2026 20:00" or "no end date".
     pub(crate) status: String,
     /// Invited but has not chosen a password yet.
@@ -51,6 +55,14 @@ struct PeoplePage {
     people: Vec<PersonRow>,
 }
 
+fn role_tone(role: Role) -> &'static str {
+    match role {
+        Role::Resident => "accent",
+        Role::Guest => "warn",
+        Role::Viewer => "muted",
+    }
+}
+
 pub(crate) fn role_key(role: Role) -> &'static str {
     match role {
         Role::Resident => "role-resident",
@@ -120,12 +132,14 @@ async fn people_page<S: Store>(
             PersonRow {
                 id: person.id.0,
                 username: person.username.to_string(),
+                initial: initial(person.username.as_str()),
                 role: role_key(person.role),
+                tone: role_tone(person.role),
                 status: status(tr, person, now, tz),
                 pending_setup: person.password_hash.is_none(),
                 can_expire: person.role != Role::Resident && !expired,
                 reactivate: expired
-                    .then(|| expiry::fields(person.role, "", &typed))
+                    .then(|| expiry::fields(tr, person.role, "", &typed))
                     .flatten(),
             }
         })
diff --git a/crates/web/templates/accept.html b/crates/web/templates/accept.html
index f1c53a6..fcef797 100644
--- a/crates/web/templates/accept.html
+++ b/crates/web/templates/accept.html
@@ -1,26 +1,21 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("accept-title") }}{% endblock %}
 {% block content %}
-<h1>{% if setup %}{{ tr.t1("accept-welcome", "username", username) }}{% else %}{{ tr.t("accept-title") }}{% endif %}</h1>
-<p>{% if setup %}{{ tr.t("accept-setup-intro") }}{% else %}{{ tr.t1("accept-reset-intro", "username", username) }}{% endif %}</p>
-{% if let Some(error) = error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
-<form method="post" class="stack">
-  <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
-  <label>{{ tr.t("settings-password-new") }}
-    <input type="password" name="password" autocomplete="new-password" minlength="10" maxlength="128" required>
-  </label>
-  <label>{{ tr.t("settings-password-repeat") }}
-    <input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>
-  </label>
-  <p class="hint">{{ tr.t("accept-password-hint") }}</p>
-  <fieldset class="choices">
-    <legend>{{ tr.t("language") }}</legend>
-    {% for language in tr.languages() %}
-    <label class="chip"><input type="radio" name="lang" value="{{ language.value }}"{% if language.current %} checked{% endif %}><span lang="{{ language.value }}">{{ language.name }}</span></label>
-    {% endfor %}
-  </fieldset>
-  <button type="submit">{{ tr.t("accept-submit") }}</button>
-</form>
+<section class="card">
+  <h1>{% if setup %}{{ tr.t1("accept-welcome", "username", username) }}{% else %}{{ tr.t("accept-title") }}{% endif %}</h1>
+  <p class="muted">{% if setup %}{{ tr.t("accept-setup-intro") }}{% else %}{{ tr.t1("accept-reset-intro", "username", username) }}{% endif %}</p>
+  {% if let Some(error) = error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+  <form method="post" class="stack">
+    <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
+    {% call c::field(tr.t("settings-password-new")) %}<input type="password" name="password" autocomplete="new-password" minlength="10" maxlength="128" required>{% endcall %}
+    {% call c::field(tr.t("settings-password-repeat")) %}<input type="password" name="repeat" autocomplete="new-password" minlength="10" maxlength="128" required>{% endcall %}
+    <p class="hint">{{ tr.t("accept-password-hint") }}</p>
+    <fieldset class="segmented">
+      <legend class="visually-hidden">{{ tr.t("language") }}</legend>
+      {% for language in tr.languages() %}{% call c::chip("radio", "lang", language.value, language.name, language.current) %}{% endcall %}{% endfor %}
+    </fieldset>
+    <button type="submit" class="button">{{ tr.t("accept-submit") }}</button>
+  </form>
+</section>
 {% endblock %}
diff --git a/crates/web/templates/components.html b/crates/web/templates/components.html
index 1d2fde6..81316bd 100644
--- a/crates/web/templates/components.html
+++ b/crates/web/templates/components.html
@@ -90,3 +90,15 @@
   {% endfor %}
 </nav>
 {% endmacro %}
+
+{# When an account stops working (spec §8.3): quick picks as chips, or a day. js/date-picks.js
+   keeps the two from being sent together. #}
+{% macro expiry(fields, legend, date_label) %}
+<fieldset class="expiry stack" data-date-picks data-role="{{ fields.role }}">
+  <legend>{{ legend }}</legend>
+  <div class="cluster">
+    {% for pick in fields.picks.iter() %}{% call chip("radio", fields.pick_name, pick.value, pick.label, pick.checked) %}{% endcall %}{% endfor %}
+  </div>
+  <input type="date" name="{{ fields.date_name }}" value="{{ fields.date }}" aria-label="{{ date_label }}">
+</fieldset>
+{% endmacro %}
diff --git a/crates/web/templates/expiry_fields.html b/crates/web/templates/expiry_fields.html
deleted file mode 100644
index a086b5e..0000000
--- a/crates/web/templates/expiry_fields.html
+++ /dev/null
@@ -1,8 +0,0 @@
-{# When an account stops working: quick picks or a day (spec §8.3). Expects `expiry`. #}
-<fieldset class="expiry choices" data-date-picks data-role="{{ expiry.role }}">
-  <legend>{{ tr.t("expiry-legend") }}</legend>
-  {% for pick in expiry.picks %}
-  <label class="chip"><input type="radio" name="{{ expiry.pick_name }}" value="{{ pick.value }}"{% if pick.checked %} checked{% endif %}><span>{{ tr.t(pick.key) }}</span></label>
-  {% endfor %}
-  <input type="date" name="{{ expiry.date_name }}" value="{{ expiry.date }}" aria-label="{{ tr.t("expiry-date") }}">
-</fieldset>
diff --git a/crates/web/templates/invite.html b/crates/web/templates/invite.html
index 5aa7179..c9e3269 100644
--- a/crates/web/templates/invite.html
+++ b/crates/web/templates/invite.html
@@ -1,24 +1,19 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("invite-title") }}{% endblock %}
 {% block content %}
+{% call c::back("/people", tr.t("people-title")) %}{% endcall %}
 <h1>{{ tr.t("invite-title") }}</h1>
-{% if let Some(error) = error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
-<form method="post" action="/people/invite" class="stack invite">
-  <label>{{ tr.t("login-username") }}
-    <input name="username" value="{{ username }}" autocomplete="off" autocapitalize="none"
-           autocorrect="off" spellcheck="false" maxlength="32" required>
-  </label>
+{% if let Some(error) = error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+<form method="post" action="/people/invite" class="card invite">
+  {% call c::field(tr.t("login-username")) %}<input name="username" value="{{ username }}" autocomplete="off" autocapitalize="none" autocorrect="off" spellcheck="false" maxlength="32" required>{% endcall %}
   <p class="hint">{{ tr.t("invite-username-hint") }}</p>
-  <fieldset class="choices">
-    <legend>{{ tr.t("invite-role") }}</legend>
-    {% for role in roles %}
-    <label class="chip"><input type="radio" name="role" value="{{ role.value }}"{% if role.checked %} checked{% endif %}><span>{{ tr.t(role.key) }}</span></label>
-    {% endfor %}
+  <fieldset class="segmented">
+    <legend class="visually-hidden">{{ tr.t("invite-role") }}</legend>
+    {% for role in roles %}{% call c::chip("radio", "role", role.value, tr.t(role.key), role.checked) %}{% endcall %}{% endfor %}
   </fieldset>
-  {% if let Some(expiry) = guest %}{% include "expiry_fields.html" %}{% endif %}
-  {% if let Some(expiry) = viewer %}{% include "expiry_fields.html" %}{% endif %}
-  <button type="submit">{{ tr.t("invite-submit") }}</button>
+  {% if let Some(expiry) = guest %}{% call c::expiry(expiry, tr.t("expiry-legend"), tr.t("expiry-date")) %}{% endcall %}{% endif %}
+  {% if let Some(expiry) = viewer %}{% call c::expiry(expiry, tr.t("expiry-legend"), tr.t("expiry-date")) %}{% endcall %}{% endif %}
+  <button type="submit" class="button">{{ tr.t("invite-submit") }}</button>
 </form>
 {% endblock %}
diff --git a/crates/web/templates/link.html b/crates/web/templates/link.html
index 4400d9f..b24da82 100644
--- a/crates/web/templates/link.html
+++ b/crates/web/templates/link.html
@@ -1,11 +1,14 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("link-title") }}{% endblock %}
 {% block content %}
-<h1>{% if setup %}{{ tr.t1("link-setup-title", "username", username) }}{% else %}{{ tr.t1("link-reset-title", "username", username) }}{% endif %}</h1>
-<p>{{ tr.t("link-scan") }}</p>
-<figure class="qr-code">{{ qr|safe }}</figure>
-<p class="notice">{{ tr.t1("link-valid-until", "time", until) }}</p>
-<p class="hint">{{ tr.t("link-text") }}</p>
-<p><code class="link-text">{{ url }}</code></p>
-<p><a href="/people">{{ tr.t("link-done") }}</a></p>
+<section class="card center">
+  <h1>{% if setup %}{{ tr.t1("link-setup-title", "username", username) }}{% else %}{{ tr.t1("link-reset-title", "username", username) }}{% endif %}</h1>
+  <p>{{ tr.t("link-scan") }}</p>
+  <figure class="qr-code">{{ qr|safe }}</figure>
+  {% call c::notice(tr.t1("link-valid-until", "time", until), false) %}{% endcall %}
+  <p class="hint">{{ tr.t("link-text") }}</p>
+  <code>{{ url }}</code>
+</section>
+<a href="/people" class="button">{{ tr.t("link-done") }}</a>
 {% endblock %}
diff --git a/crates/web/templates/people.html b/crates/web/templates/people.html
index a67e97a..b8cd695 100644
--- a/crates/web/templates/people.html
+++ b/crates/web/templates/people.html
@@ -1,33 +1,34 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("people-title") }}{% endblock %}
 {% block content %}
-<h1>{{ tr.t("people-title") }}</h1>
-{% if let Some(saved) = saved %}
-<p class="notice" role="status">{{ tr.t(saved) }}</p>
-{% endif %}
-{% if let Some(error) = error %}
-<p class="notice" role="alert">{{ tr.t(error) }}</p>
-{% endif %}
-<p><a href="/people/invite">{{ tr.t("people-invite") }}</a></p>
-<ul class="people">
-  {% for person in people %}
-  <li>
-    <p><strong>{{ person.username }}</strong> · {{ tr.t(person.role) }} · {{ person.status }}{% if person.pending_setup %} · {{ tr.t("people-status-setup") }}{% endif %}</p>
-    <form method="post" action="/people/{{ person.id }}/link">
-      <button type="submit" class="secondary">{% if person.pending_setup %}{{ tr.t("people-new-setup-link") }}{% else %}{{ tr.t("people-reset-password") }}{% endif %}</button>
-    </form>
-    {% if person.can_expire %}
-    <form method="post" action="/people/{{ person.id }}/expire">
-      <button type="submit" class="secondary">{{ tr.t("people-expire") }}</button>
-    </form>
+<div class="split">
+  <h1>{{ tr.t("people-title") }}</h1>
+  <a href="/people/invite" class="button">{{ tr.t("people-invite") }}</a>
+</div>
+{% if let Some(saved) = saved %}{% call c::notice(tr.t(saved), false) %}{% endcall %}{% endif %}
+{% if let Some(error) = error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
+{% for person in people %}
+<section class="card">
+  <div class="person">
+    <span class="avatar" aria-hidden="true">{{ person.initial }}</span>
+    <span class="row-text"><strong>{{ person.username }}</strong><span>{{ person.status }}{% if person.pending_setup %} · {{ tr.t("people-status-setup") }}{% endif %}</span></span>
+    <span class="tag" data-tone="{{ person.tone }}">{{ tr.t(person.role) }}</span>
+  </div>
+  <div class="cluster">
+    {% if person.pending_setup %}
+    {% call c::post_button("/people/{}/link"|format(person.id), tr.t("people-new-setup-link"), "secondary") %}{% endcall %}
+    {% else %}
+    {% call c::post_button("/people/{}/link"|format(person.id), tr.t("people-reset-password"), "secondary") %}{% endcall %}
     {% endif %}
-    {% if let Some(expiry) = person.reactivate %}
-    <form method="post" action="/people/{{ person.id }}/reactivate" class="stack">
-      {% include "expiry_fields.html" %}
-      <button type="submit" class="secondary">{{ tr.t("people-reactivate") }}</button>
-    </form>
-    {% endif %}
-  </li>
-  {% endfor %}
-</ul>
+    {% if person.can_expire %}{% call c::post_button("/people/{}/expire"|format(person.id), tr.t("people-expire"), "danger") %}{% endcall %}{% endif %}
+  </div>
+  {% if let Some(expiry) = person.reactivate %}
+  <form method="post" action="/people/{{ person.id }}/reactivate" class="stack">
+    {% call c::expiry(expiry, tr.t("expiry-legend"), tr.t("expiry-date")) %}{% endcall %}
+    <button type="submit" class="button" data-variant="secondary">{{ tr.t("people-reactivate") }}</button>
+  </form>
+  {% endif %}
+</section>
+{% endfor %}
 {% endblock %}
diff --git a/crates/web/tests/people.rs b/crates/web/tests/people.rs
index 3204d7e..e8bc330 100644
--- a/crates/web/tests/people.rs
+++ b/crates/web/tests/people.rs
@@ -42,13 +42,18 @@ async fn people_lists_everyone_with_their_role_and_expiry() {
     app.add_user("guest", Role::Guest, Locale::En).await;
     let cookie = app.sign_in("daniele").await;
     let html = text(app.send(get_with("/people", &cookie)).await).await;
-    assert!(
-        html.contains("<strong>daniele</strong> · resident · no end date"),
-        "{html}"
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.texts("section.card .person"),
+        [
+            "D daniele no end date resident",
+            "G guest until 22/01/2026 20:00 guest"
+        ]
     );
-    assert!(
-        html.contains("<strong>guest</strong> · guest · until 22/01/2026 20:00"),
-        "{html}"
+    assert_eq!(page.attrs(".person .tag", "data-tone"), ["accent", "warn"]);
+    assert_eq!(
+        page.text(r#"form[action="/people/2/expire"] button[data-variant=danger]"#),
+        "End access now"
     );
     assert!(html.contains(r#"action="/people/2/expire""#), "{html}");
     assert!(!html.contains(r#"action="/people/1/expire""#), "{html}");
````

### Task 13: Sign-in, guides, install, offline and error pages

**Proto commit:** `d8bbe00` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show d8bbe00`)

Spec §4. Sign in centred under the app icon (`{% block header %}` emptied); guides with back links and rows; install steps use the sprite (`share`, `add-square`), so no template draws its own SVG. CSS 72.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/72-guide.css
M	crates/web/css/40-layout.css
M	crates/web/src/assets.rs
M	crates/web/templates/error.html
M	crates/web/templates/guide.html
M	crates/web/templates/guide_logging.html
M	crates/web/templates/install.html
M	crates/web/templates/login.html
M	crates/web/templates/offline.html
M	crates/web/tests/guides.rs
M	crates/web/tests/pwa.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/guides.rs`, `crates/web/tests/pwa.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **617 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(web): sign-in, guides, install, offline and error pages in the new look

Sign in is centred under the app icon and name, with the languages at the bottom. The
guides are cards with a back link to Settings and a row to the other guide; How to rate shows
each level as a badge. The install steps take their Share and Add icons from the sprite, so
no template draws an SVG of its own. Offline and error pages are single cards.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `d8bbe00`):

````diff
diff --git a/crates/web/css/40-layout.css b/crates/web/css/40-layout.css
index 164897a..f255830 100644
--- a/crates/web/css/40-layout.css
+++ b/crates/web/css/40-layout.css
@@ -46,7 +46,7 @@
   }
 
   .center {
-    justify-content: center;
+    align-items: center;
     text-align: center;
   }
 }
diff --git a/crates/web/css/72-guide.css b/crates/web/css/72-guide.css
new file mode 100644
index 0000000..1831e38
--- /dev/null
+++ b/crates/web/css/72-guide.css
@@ -0,0 +1,44 @@
+/* The guide pages: tables with a line between rows, numbered rules, the install steps; and
+   the app icon above the sign-in card. */
+@layer components {
+  .guide th,
+  .guide td {
+    padding: var(--space-2) var(--space-1);
+  }
+
+  .guide tr + tr,
+  .scale tbody tr + tr {
+    border-block-start: 1px solid var(--line);
+  }
+
+  .guide thead th,
+  .scale thead th {
+    color: var(--muted);
+    font-size: var(--text-label);
+  }
+
+  .rules,
+  .steps {
+    display: flex;
+    flex-direction: column;
+    gap: var(--space-2);
+    padding-inline-start: var(--space-5);
+  }
+
+  .steps .icon {
+    display: inline-block;
+    vertical-align: text-bottom;
+  }
+
+  .welcome {
+    display: flex;
+    flex-direction: column;
+    align-items: center;
+    gap: var(--space-2);
+    padding-block: var(--space-5) var(--space-3);
+  }
+
+  .welcome > img {
+    border-radius: var(--radius-button);
+  }
+}
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index e2fe5c1..c76a24c 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -64,6 +64,7 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "69-facts.css",
     "70-items.css",
     "71-people.css",
+    "72-guide.css",
     "90-utilities.css",
 );
 
diff --git a/crates/web/templates/error.html b/crates/web/templates/error.html
index e49e02f..c1e1af3 100644
--- a/crates/web/templates/error.html
+++ b/crates/web/templates/error.html
@@ -1,11 +1,9 @@
 {% extends "base.html" %}
 {% block title %}{{ tr.t(title_key) }}{% endblock %}
 {% block content %}
-<h1>{{ tr.t(title_key) }}</h1>
-{% if internal %}
-<p>{{ tr.t1(message_key, "ref", request_id) }}</p>
-{% else %}
-<p>{{ tr.t(message_key) }}</p>
-{% endif %}
-<p><a href="/">{{ tr.t("error-back-home") }}</a></p>
+<section class="card">
+  <h1>{{ tr.t(title_key) }}</h1>
+  <p>{% if internal %}{{ tr.t1(message_key, "ref", request_id) }}{% else %}{{ tr.t(message_key) }}{% endif %}</p>
+  <a href="/" class="button">{{ tr.t("error-back-home") }}</a>
+</section>
 {% endblock %}
diff --git a/crates/web/templates/guide.html b/crates/web/templates/guide.html
index 9bc81a8..505ccc9 100644
--- a/crates/web/templates/guide.html
+++ b/crates/web/templates/guide.html
@@ -1,17 +1,23 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("rating-title") }}{% endblock %}
 {% block content %}
+{% call c::back("/settings", tr.t("settings-title")) %}{% endcall %}
 <h1>{{ tr.t("rating-title") }}</h1>
-<table class="guide">
-  <thead>
-    <tr><th scope="col">{{ tr.t("rating-level") }}</th><th scope="col">{{ tr.t("rating-description") }}</th></tr>
-  </thead>
-  <tbody>
-    {% for level in levels %}
-    <tr><th scope="row">{{ level.value }} · {{ level.name }}</th><td>{{ level.description }}</td></tr>
-    {% endfor %}
-  </tbody>
-</table>
-<p>{{ tr.t("rating-tip") }}</p>
-<p><a href="/guide/logging">{{ tr.t("guide-log-title") }}</a> · <a href="/">{{ tr.t("error-back-home") }}</a></p>
+<section class="card">
+  <table class="scale">
+    <thead>
+      <tr><th scope="col">{{ tr.t("rating-level") }}</th><th scope="col">{{ tr.t("rating-description") }}</th></tr>
+    </thead>
+    <tbody>
+      {% for level in levels %}
+      <tr><th scope="row"><span class="badge">{{ level.value }}</span></th><td><strong>{{ level.name }}</strong><br>{{ level.description }}</td></tr>
+      {% endfor %}
+    </tbody>
+  </table>
+  <p class="muted">{{ tr.t("rating-tip") }}</p>
+</section>
+<section class="card rows">
+  {% call c::row("/guide/logging", tr.t("guide-log-title"), "") %}{% endcall %}
+</section>
 {% endblock %}
diff --git a/crates/web/templates/guide_logging.html b/crates/web/templates/guide_logging.html
index 276c334..6073c36 100644
--- a/crates/web/templates/guide_logging.html
+++ b/crates/web/templates/guide_logging.html
@@ -1,24 +1,32 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("guide-log-title") }}{% endblock %}
 {% block content %}
+{% call c::back("/settings", tr.t("settings-title")) %}{% endcall %}
 <h1>{{ tr.t("guide-log-title") }}</h1>
-<table class="guide">
-  <thead>
-    <tr><th scope="col">{{ tr.t("guide-log-happened") }}</th><th scope="col">{{ tr.t("guide-log-action") }}</th></tr>
-  </thead>
-  <tbody>
-    {% for situation in situations %}
-    <tr><td>{{ situation.happened }}</td><td>{{ situation.action }}</td></tr>
-    {% endfor %}
-  </tbody>
-</table>
-<h2>{{ tr.t("guide-log-rules") }}</h2>
+<section class="card">
+  <table class="guide">
+    <thead>
+      <tr><th scope="col">{{ tr.t("guide-log-happened") }}</th><th scope="col">{{ tr.t("guide-log-action") }}</th></tr>
+    </thead>
+    <tbody>
+      {% for situation in situations %}
+      <tr><td>{{ situation.happened }}</td><td>{{ situation.action }}</td></tr>
+      {% endfor %}
+    </tbody>
+  </table>
+</section>
+{% call c::card(tr.t("guide-log-rules")) %}
 <ol class="rules">
   {% for rule in rules %}
   <li>{{ rule }}</li>
   {% endfor %}
 </ol>
-<h2>{{ tr.t("guide-log-sensitivity-title") }}</h2>
+{% endcall %}
+{% call c::card(tr.t("guide-log-sensitivity-title")) %}
 <p>{{ tr.t("guide-log-sensitivity") }}</p>
-<p><a href="/guide/rating">{{ tr.t("rating-title") }}</a> · <a href="/">{{ tr.t("error-back-home") }}</a></p>
+{% endcall %}
+<section class="card rows">
+  {% call c::row("/guide/rating", tr.t("rating-title"), "") %}{% endcall %}
+</section>
 {% endblock %}
diff --git a/crates/web/templates/install.html b/crates/web/templates/install.html
index 7330d00..267a44d 100644
--- a/crates/web/templates/install.html
+++ b/crates/web/templates/install.html
@@ -1,18 +1,20 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("install-title") }}{% endblock %}
 {% block content %}
-{% if welcome %}
-<p class="notice" role="status">{{ tr.t("install-welcome") }}</p>
-{% endif %}
+{% if welcome %}{% call c::notice(tr.t("install-welcome"), false) %}{% endcall %}{% endif %}
 <h1>{{ tr.t("install-title") }}</h1>
-<p>{{ tr.t("install-intro") }}</p>
-<p><button type="button" data-install hidden>{{ tr.t("install-button") }}</button></p>
-<h2>{{ tr.t("install-ios-title") }}</h2>
+<section class="card">
+  <p>{{ tr.t("install-intro") }}</p>
+  <button type="button" class="button" data-install hidden>{{ tr.t("install-button") }}</button>
+</section>
+{% call c::card(tr.t("install-ios-title")) %}
 <ol class="steps">
-  <li>{{ tr.t("install-ios-share") }} <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15V3m-4 4 4-4 4 4M6 10H5v11h14V10h-1" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/></svg></li>
-  <li>{{ tr.t("install-ios-add") }} <svg class="glyph" viewBox="0 0 24 24" aria-hidden="true"><rect x="4" y="4" width="16" height="16" rx="3" fill="none" stroke="currentColor" stroke-width="2"/><path d="M12 8v8m-4-4h8" stroke="currentColor" stroke-width="2" stroke-linecap="round"/></svg></li>
+  <li>{{ tr.t("install-ios-share") }} {% call c::icon("share") %}{% endcall %}</li>
+  <li>{{ tr.t("install-ios-add") }} {% call c::icon("add-square") %}{% endcall %}</li>
   <li>{{ tr.t("install-ios-confirm") }}</li>
 </ol>
 <p class="hint">{{ tr.t("install-other") }}</p>
-<p><a href="/">{{ tr.t("install-continue") }}</a></p>
+{% endcall %}
+<a href="/" class="button" data-variant="secondary">{{ tr.t("install-continue") }}</a>
 {% endblock %}
diff --git a/crates/web/templates/login.html b/crates/web/templates/login.html
index b6029ec..48d5b74 100644
--- a/crates/web/templates/login.html
+++ b/crates/web/templates/login.html
@@ -1,18 +1,19 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("login-title") }}{% endblock %}
+{% block header %}{% endblock %}
 {% block content %}
-<h1>{{ tr.t("login-title") }}</h1>
-{% if let Some(notice) = notice %}
-<p class="notice" role="alert">{{ tr.t(notice.key()) }}</p>
-{% endif %}
-<form method="post" action="/login" class="stack">
-  <label>{{ tr.t("login-username") }}
-    <input name="username" value="{{ username }}" autocomplete="username"
-           autocapitalize="none" autocorrect="off" spellcheck="false" maxlength="64" required>
-  </label>
-  <label>{{ tr.t("login-password") }}
-    <input type="password" name="password" autocomplete="current-password" maxlength="128" required>
-  </label>
-  <button type="submit">{{ tr.t("login-submit") }}</button>
-</form>
+<div class="welcome">
+  <img src="/static/icon.svg" alt="" width="64" height="64">
+  <span class="brand">{{ tr.t("app-name") }}</span>
+</div>
+<section class="card">
+  <h1>{{ tr.t("login-title") }}</h1>
+  {% if let Some(notice) = notice %}{% call c::notice(tr.t(notice.key()), true) %}{% endcall %}{% endif %}
+  <form method="post" action="/login" class="stack">
+    {% call c::field(tr.t("login-username")) %}<input name="username" value="{{ username }}" autocomplete="username" autocapitalize="none" autocorrect="off" spellcheck="false" maxlength="64" required>{% endcall %}
+    {% call c::field(tr.t("login-password")) %}<input type="password" name="password" autocomplete="current-password" maxlength="128" required>{% endcall %}
+    <button type="submit" class="button">{{ tr.t("login-submit") }}</button>
+  </form>
+</section>
 {% endblock %}
diff --git a/crates/web/templates/offline.html b/crates/web/templates/offline.html
index d56e430..55542de 100644
--- a/crates/web/templates/offline.html
+++ b/crates/web/templates/offline.html
@@ -1,7 +1,9 @@
 {% extends "base.html" %}
 {% block title %}{{ tr.t("offline-title") }}{% endblock %}
 {% block content %}
-<h1>{{ tr.t("offline-title") }}</h1>
-<p>{{ tr.t("offline-text") }}</p>
-<p><a href="/">{{ tr.t("offline-retry") }}</a></p>
+<section class="card">
+  <h1>{{ tr.t("offline-title") }}</h1>
+  <p>{{ tr.t("offline-text") }}</p>
+  <a href="/" class="button">{{ tr.t("offline-retry") }}</a>
+</section>
 {% endblock %}
diff --git a/crates/web/tests/guides.rs b/crates/web/tests/guides.rs
index 58a5a9a..1c8daf6 100644
--- a/crates/web/tests/guides.rs
+++ b/crates/web/tests/guides.rs
@@ -14,21 +14,19 @@ async fn the_logging_guide_has_the_table_and_rules_of_thumb() {
     let cookie = app.sign_in("guest").await;
     let response = app.send(get_with("/guide/logging", &cookie)).await;
     assert_eq!(response.status(), StatusCode::OK);
-    let html = text(response).await;
-    assert!(html.contains("<h1>How to log</h1>"), "{html}");
+    let page = Page::parse(&text(response).await);
+    assert_eq!(page.text("h1"), "How to log");
+    assert!(page.texts(".guide tbody tr").contains(
+        &"The smell went away completely and came back later Start a new event".to_owned()
+    ));
+    assert_eq!(page.count(".rules > li"), 5);
     assert!(
-        html.contains(
-            "<tr><td>The smell went away completely and came back later</td>\
-             <td>Start a new event</td></tr>"
-        ),
-        "{html}"
+        page.text(".rules")
+            .contains("Forgetting is fine (04:00 auto-close)"),
+        "the configured hour"
     );
-    assert_eq!(html.matches("<li>").count(), 5, "{html}");
-    assert!(
-        html.contains("Forgetting is fine (04:00 auto-close)"),
-        "the configured hour: {html}"
-    );
-    assert!(html.contains(r#"<a href="/guide/rating">How to rate</a>"#));
+    assert_eq!(page.text(r#"a.row[href="/guide/rating"]"#), "How to rate");
+    assert_eq!(page.attr("a.back", "href").as_deref(), Some("/settings"));
 }
 
 #[tokio::test]
@@ -45,11 +43,9 @@ async fn settings_and_the_rating_guide_lead_to_both_guides() {
         settings.texts("section.rows a.row"),
         ["How to rate", "How to log"]
     );
-    let rating = text(app.send(get_with("/guide/rating", &cookie)).await).await;
-    assert!(
-        rating.contains(r#"<a href="/guide/logging">How to log</a>"#),
-        "{rating}"
-    );
+    let rating = Page::parse(&text(app.send(get_with("/guide/rating", &cookie)).await).await);
+    assert_eq!(rating.text(r#"a.row[href="/guide/logging"]"#), "How to log");
+    assert_eq!(rating.count(".scale tbody tr"), 11);
 }
 
 #[tokio::test]
diff --git a/crates/web/tests/pwa.rs b/crates/web/tests/pwa.rs
index 6fbc7c7..f3681e4 100644
--- a/crates/web/tests/pwa.rs
+++ b/crates/web/tests/pwa.rs
@@ -91,12 +91,19 @@ async fn the_offline_page_needs_no_session() {
 async fn the_install_page_offers_the_button_and_the_iphone_steps() {
     let app = TestApp::new().await;
     let html = text(app.send(get("/install")).await).await;
+    let page = Page::parse(&html);
+    assert_eq!(
+        page.text("button[type=button][data-install][hidden]"),
+        "Install"
+    );
     assert!(
-        html.contains(r#"<button type="button" data-install hidden>Install</button>"#),
+        page.text(".steps").contains("Tap the Share button"),
         "{html}"
     );
-    assert!(html.contains("Tap the Share button"), "{html}");
-    assert_eq!(html.matches("<svg").count(), 2, "{html}");
+    assert_eq!(
+        page.attrs(".steps svg.icon use", "href"),
+        ["/static/icons.svg#share", "/static/icons.svg#add-square"]
+    );
     assert!(!html.contains("Your account is ready."), "{html}");
 
     app.add_user("guest", Role::Guest, Locale::Tr).await;
````

### Task 14: Graph filters and any period of days in `smoke_domain::graphs`

**Proto commit:** `6d4826e` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 6d4826e`)

Spec §7 (domain part). `Period` becomes `{ first, last }` with `month`, `year`, `last_days`, `new` (≤ 731 days), `granularity` (Day ≤ 62 days, else Week), `span` from local midnights; `DayPart`; `Filter::{reports, readings}`. `/graphs` keeps its month/year switch on the new API until Task 16 (`step`, `granularity()` matches).

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	crates/domain/src/graphs.rs
M	crates/web/src/routes/graphs.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/domain/src/graphs.rs (tests)`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **625 passed** (SQLite). Domain only plus the web adapter; run the Postgres suite too (the domain changed).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(domain): graph filters and any period of days in smoke_domain::graphs

Period becomes any run of local days (up to two years), counted per day up to about two
months and per week beyond; month, year and last-N-days constructors replace the enum.
Filter narrows reports by room or area (each report keeps only the chosen rooms), person,
part of the day (DayPart; the night crosses midnight), minimum level and hidden clean checks;
readings follow only the person and the part of the day. Pure and unit-tested. /graphs keeps
its month and year switch on the new Period until its filters land.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `6d4826e`):

````diff
diff --git a/crates/domain/src/graphs.rs b/crates/domain/src/graphs.rs
index ee956e1..46162c1 100644
--- a/crates/domain/src/graphs.rs
+++ b/crates/domain/src/graphs.rs
@@ -1,85 +1,251 @@
-//! The numbers behind the Graphs screen (spec §9 screen 5). Every function takes the reports the
-//! caller has already limited to one [`Period`] and to what the viewer may see, so guests get
-//! graphs of their own data only (spec §4).
+//! The numbers behind the Graphs screen (spec §9 screen 5). [`Filter`] narrows what the viewer
+//! may see to what the filters ask for; every other function takes the reports the caller has
+//! already limited to one [`Period`] and to what the viewer may see, so guests get graphs of
+//! their own data only (spec §4).
 
 use std::collections::BTreeMap;
 
 use chrono::{DateTime, Datelike, Days, Months, NaiveDate, Timelike, Utc};
 
-use crate::records::{Room, SmellReport};
+use crate::records::{Pm25Reading, ReportValues, Room, SmellReport};
 use crate::schedule::DailySchedule;
 use crate::types::{EventId, Level, RoomArea, RoomId, SymptomId, UserId};
 
-/// The stretch of local calendar time the graphs cover.
+/// The stretch of local calendar days the graphs cover, both ends included (look-and-feel spec
+/// §7). Up to about two months it is counted per day, beyond that per week (Monday to Sunday).
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
-pub enum Period {
-    /// The local calendar month that contains this date, counted per day.
-    Month(NaiveDate),
-    /// The local calendar year that contains this date, counted per week (Monday to Sunday).
-    Year(NaiveDate),
+pub struct Period {
+    first: NaiveDate,
+    last: NaiveDate,
+}
+
+/// How a [`Period`] is cut into buckets.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Granularity {
+    Day,
+    Week,
 }
 
 impl Period {
-    /// `[start, end)` in UTC.
-    pub fn span(&self, schedule: &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>) {
-        match *self {
-            Self::Month(date) => schedule.month_span(date),
-            Self::Year(date) => schedule.year_span(date),
+    /// The longest period counted per day: about two months.
+    pub const DAILY_UP_TO_DAYS: i64 = 62;
+    /// The longest period the graphs show, so that every query stays bounded.
+    pub const MAX_DAYS: i64 = 731;
+
+    /// From `first` to `last`, both included; `None` when `last` comes first or the period is
+    /// longer than [`Self::MAX_DAYS`].
+    pub fn new(first: NaiveDate, last: NaiveDate) -> Option<Self> {
+        let days = (last - first).num_days() + 1;
+        (1..=Self::MAX_DAYS)
+            .contains(&days)
+            .then_some(Self { first, last })
+    }
+
+    /// The local calendar month that contains `date`.
+    pub fn month(date: NaiveDate) -> Self {
+        let first = date.with_day(1).expect("every month has a first day");
+        let last = first + Months::new(1) - Days::new(1);
+        Self { first, last }
+    }
+
+    /// The local calendar year that contains `date`.
+    pub fn year(date: NaiveDate) -> Self {
+        let first = date.with_ordinal(1).expect("every year has a first day");
+        let last = first + Months::new(12) - Days::new(1);
+        Self { first, last }
+    }
+
+    /// The `days` days that end with `today`.
+    pub fn last_days(today: NaiveDate, days: u64) -> Self {
+        Self {
+            first: today - Days::new(days.saturating_sub(1)),
+            last: today,
         }
     }
 
-    /// The first local day of the period.
     pub fn first_day(&self) -> NaiveDate {
-        match *self {
-            Self::Month(date) => date.with_day(1).expect("every month has a first day"),
-            Self::Year(date) => date.with_ordinal(1).expect("every year has a first day"),
-        }
+        self.first
     }
 
-    /// The period before this one.
-    pub fn previous(&self) -> Self {
-        let first = self.first_day();
-        match self {
-            Self::Month(_) => Self::Month(first - Months::new(1)),
-            Self::Year(_) => Self::Year(first - Months::new(12)),
-        }
+    pub fn last_day(&self) -> NaiveDate {
+        self.last
     }
 
-    /// The period after this one.
-    pub fn next(&self) -> Self {
-        self.after(self.first_day())
+    /// The number of days, both ends included.
+    pub fn days(&self) -> i64 {
+        (self.last - self.first).num_days() + 1
     }
 
-    fn after(&self, first: NaiveDate) -> Self {
-        match self {
-            Self::Month(_) => Self::Month(first + Months::new(1)),
-            Self::Year(_) => Self::Year(first + Months::new(12)),
+    pub fn granularity(&self) -> Granularity {
+        if self.days() <= Self::DAILY_UP_TO_DAYS {
+            Granularity::Day
+        } else {
+            Granularity::Week
         }
     }
 
-    /// The first day of every bucket, in order: each day of a month; each week touching a year,
-    /// named by its Monday (the first may fall in December of the year before).
+    /// `[start, end)` in UTC: from the local midnight that starts the first day to the one that
+    /// ends the last.
+    pub fn span(&self, schedule: &DailySchedule) -> (DateTime<Utc>, DateTime<Utc>) {
+        (
+            schedule.day_span(self.first).0,
+            schedule.day_span(self.last).1,
+        )
+    }
+
+    /// The first day of every bucket, in order: each day, or each week touching the period,
+    /// named by its Monday (the first may fall before the period starts).
     pub fn buckets(&self) -> Vec<NaiveDate> {
-        let first = self.first_day();
-        let end = self.after(first).first_day();
-        let step = match self {
-            Self::Month(_) => Days::new(1),
-            Self::Year(_) => Days::new(7),
+        let step = match self.granularity() {
+            Granularity::Day => Days::new(1),
+            Granularity::Week => Days::new(7),
         };
-        std::iter::successors(Some(self.bucket_of(first)), |&day| Some(day + step))
-            .take_while(|&day| day < end)
+        std::iter::successors(Some(self.bucket_of(self.first)), |&day| Some(day + step))
+            .take_while(|&day| day <= self.last)
             .collect()
     }
 
     /// The first day of the bucket that holds `date`.
     pub fn bucket_of(&self, date: NaiveDate) -> NaiveDate {
+        match self.granularity() {
+            Granularity::Day => date,
+            Granularity::Week => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
+        }
+    }
+}
+
+/// A part of the day, by local observed time (look-and-feel spec §7).
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum DayPart {
+    /// 06:00–12:00.
+    Morning,
+    /// 12:00–18:00.
+    Afternoon,
+    /// 18:00–22:00.
+    Evening,
+    /// 22:00–06:00, across midnight.
+    Night,
+}
+
+impl DayPart {
+    pub const ALL: [DayPart; 4] = [
+        DayPart::Morning,
+        DayPart::Afternoon,
+        DayPart::Evening,
+        DayPart::Night,
+    ];
+
+    /// The local hours it starts and ends at; the end is excluded.
+    pub fn hours(self) -> (u32, u32) {
         match self {
-            Self::Month(_) => date,
-            Self::Year(_) => date - Days::new(u64::from(date.weekday().num_days_from_monday())),
+            Self::Morning => (6, 12),
+            Self::Afternoon => (12, 18),
+            Self::Evening => (18, 22),
+            Self::Night => (22, 6),
+        }
+    }
+
+    /// Whether the local `hour` (0–23) falls in this part.
+    pub fn contains(self, hour: u32) -> bool {
+        let (start, end) = self.hours();
+        if start < end {
+            (start..end).contains(&hour)
+        } else {
+            hour >= start || hour < end
         }
     }
 }
 
+/// What the graphs are narrowed to, besides the period (look-and-feel spec §7). The rooms, the
+/// area and the minimum level concern smell reports only; PM2.5 readings follow the person and
+/// the time of day. The default keeps everything.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Filter {
+    /// Only these rooms; when any is chosen, `area` is ignored.
+    pub rooms: Vec<RoomId>,
+    /// Only the rooms of this area.
+    pub area: Option<RoomArea>,
+    /// Only what this person logged.
+    pub person: Option<UserId>,
+    /// Only what was observed in this part of the day.
+    pub day_part: Option<DayPart>,
+    /// Leaves out reports below this level; 0 keeps them all.
+    pub min_level: u8,
+    /// Leaves out reports at 0.
+    pub hide_clean: bool,
+}
+
+impl Filter {
+    /// The reports the graphs count. Each report keeps only its rooms among those chosen (by
+    /// room, else by area) and is left out when none is left, a whole-home check included; its
+    /// intensity is then the highest level among the rooms kept, which the minimum level and
+    /// "hide clean checks" judge. `rooms` gives each room's area.
+    pub fn reports(
+        &self,
+        reports: Vec<SmellReport>,
+        rooms: &[Room],
+        schedule: &DailySchedule,
+    ) -> Vec<SmellReport> {
+        let wanted = |room: RoomId| {
+            if !self.rooms.is_empty() {
+                self.rooms.contains(&room)
+            } else {
+                self.area
+                    .is_none_or(|area| rooms.iter().any(|r| r.id == room && r.area == area))
+            }
+        };
+        let narrowing = !self.rooms.is_empty() || self.area.is_some();
+        reports
+            .into_iter()
+            .filter(|report| self.keeps(report.user_id, report.observed_at, schedule))
+            .filter_map(|mut report| {
+                if narrowing {
+                    let kept: Vec<_> = report
+                        .values
+                        .rooms()
+                        .iter()
+                        .copied()
+                        .filter(|room| wanted(room.room))
+                        .collect();
+                    if kept.is_empty() {
+                        return None;
+                    }
+                    report.values = ReportValues::new(
+                        kept,
+                        report.values.symptoms().to_vec(),
+                        report.values.note(),
+                    )
+                    .expect("some rooms of valid values are valid values");
+                }
+                Some(report)
+            })
+            .filter(|report| {
+                let intensity = report.values.intensity().get();
+                intensity >= self.min_level && !(self.hide_clean && intensity == 0)
+            })
+            .collect()
+    }
+
+    /// The PM2.5 readings the graphs show: those of the person, in the part of the day.
+    pub fn readings(
+        &self,
+        readings: Vec<Pm25Reading>,
+        schedule: &DailySchedule,
+    ) -> Vec<Pm25Reading> {
+        readings
+            .into_iter()
+            .filter(|reading| self.keeps(reading.user_id, reading.observed_at, schedule))
+            .collect()
+    }
+
+    /// Whether the person and the part of the day let a record through.
+    fn keeps(&self, user: UserId, observed_at: DateTime<Utc>, schedule: &DailySchedule) -> bool {
+        let hour = observed_at.with_timezone(&schedule.tz()).hour();
+        self.person.is_none_or(|person| person == user)
+            && self.day_part.is_none_or(|part| part.contains(hour))
+    }
+}
+
 /// One person's reports as (observed time, intensity), in time order.
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub struct PersonLine {
@@ -293,7 +459,8 @@ mod tests {
 
     #[test]
     fn a_month_is_counted_per_day() {
-        let february = Period::Month(date("2026-02-14"));
+        let february = Period::month(date("2026-02-14"));
+        assert_eq!(february.granularity(), Granularity::Day);
         let days = february.buckets();
         assert_eq!(days.len(), 28);
         assert_eq!(days[0], date("2026-02-01"));
@@ -308,7 +475,12 @@ mod tests {
     #[test]
     fn a_year_is_counted_per_week_from_monday() {
         // 1 January 2026 is a Thursday: the first week starts on Monday 29 December 2025.
-        let year = Period::Year(date("2026-06-01"));
+        let year = Period::year(date("2026-06-01"));
+        assert_eq!(
+            (year.first_day(), year.last_day()),
+            (date("2026-01-01"), date("2026-12-31"))
+        );
+        assert_eq!(year.granularity(), Granularity::Week);
         let weeks = year.buckets();
         assert_eq!(weeks[0], date("2025-12-29"));
         assert_eq!(weeks[1], date("2026-01-05"));
@@ -319,13 +491,221 @@ mod tests {
     }
 
     #[test]
-    fn periods_step_back_and_forward_from_their_first_day() {
-        let january = Period::Month(date("2026-01-31"));
-        assert_eq!(january.previous(), Period::Month(date("2025-12-01")));
-        assert_eq!(january.next(), Period::Month(date("2026-02-01")));
-        let year = Period::Year(date("2026-07-15"));
-        assert_eq!(year.previous(), Period::Year(date("2025-01-01")));
-        assert_eq!(year.next().first_day(), date("2027-01-01"));
+    fn up_to_about_two_months_is_counted_per_day() {
+        let today = date("2026-10-14");
+        let week = Period::last_days(today, 7);
+        assert_eq!((week.first_day(), week.days()), (date("2026-10-08"), 7));
+        assert_eq!(Period::last_days(today, 30).granularity(), Granularity::Day);
+        let two_months = Period::new(date("2026-01-01"), date("2026-03-03")).unwrap();
+        assert_eq!(
+            (two_months.days(), two_months.granularity()),
+            (62, Granularity::Day)
+        );
+        let longer = Period::new(date("2026-01-01"), date("2026-03-04")).unwrap();
+        assert_eq!(longer.granularity(), Granularity::Week);
+    }
+
+    #[test]
+    fn a_period_runs_forwards_and_at_most_two_years() {
+        let day = date("2026-10-14");
+        assert_eq!(Period::new(day, day).map(|p| p.days()), Some(1));
+        assert_eq!(Period::new(day, date("2026-10-13")), None, "backwards");
+        assert_eq!(
+            Period::new(date("2025-01-01"), date("2027-01-01")).map(|p| p.days()),
+            Some(731)
+        );
+        assert_eq!(
+            Period::new(date("2025-01-01"), date("2027-01-02")),
+            None,
+            "732 days"
+        );
+    }
+
+    #[test]
+    fn the_night_runs_across_midnight() {
+        let parts = |hour| {
+            DayPart::ALL
+                .into_iter()
+                .filter(|part| part.contains(hour))
+                .collect::<Vec<_>>()
+        };
+        assert_eq!(parts(5), [DayPart::Night]);
+        assert_eq!(parts(6), [DayPart::Morning]);
+        assert_eq!(parts(11), [DayPart::Morning]);
+        assert_eq!(parts(12), [DayPart::Afternoon]);
+        assert_eq!(parts(18), [DayPart::Evening]);
+        assert_eq!(parts(21), [DayPart::Evening]);
+        assert_eq!(parts(22), [DayPart::Night]);
+        assert_eq!(parts(0), [DayPart::Night]);
+        assert!(
+            (0..24).all(|hour| parts(hour).len() == 1),
+            "every hour in one part"
+        );
+    }
+
+    fn levels_of(reports: &[SmellReport]) -> Vec<Vec<(i64, u8)>> {
+        reports
+            .iter()
+            .map(|r| {
+                r.values
+                    .rooms()
+                    .iter()
+                    .map(|room| (room.room.0, room.level.get()))
+                    .collect()
+            })
+            .collect()
+    }
+
+    fn sample() -> Vec<SmellReport> {
+        vec![
+            // 21:00 in Rome, by user 1: bedroom (1) 6, stairwell (6) 2.
+            report(1, 1, "2026-01-10T20:00:00Z", &[(1, 6), (6, 2)], &[]),
+            // 08:00 in Rome, by user 2: kitchen (2) 3.
+            report(1, 2, "2026-01-11T07:00:00Z", &[(2, 3)], &[]),
+            // 23:30 in Rome, by user 1: a whole-home check.
+            report(0, 1, "2026-01-11T22:30:00Z", &[], &[]),
+            // 14:00 in Rome, by user 2: bedroom 0, stairwell 4.
+            report(2, 2, "2026-01-12T13:00:00Z", &[(1, 0), (6, 4)], &[]),
+        ]
+    }
+
+    fn rooms() -> Vec<Room> {
+        vec![
+            room(1, RoomArea::Inside),
+            room(2, RoomArea::Inside),
+            room(6, RoomArea::Common),
+        ]
+    }
+
+    #[test]
+    fn no_filter_keeps_everything() {
+        let kept = Filter::default().reports(sample(), &rooms(), &rome());
+        assert_eq!(kept, sample());
+    }
+
+    #[test]
+    fn chosen_rooms_narrow_each_report_and_win_over_the_area() {
+        let filter = Filter {
+            rooms: vec![RoomId(1)],
+            area: Some(RoomArea::Common),
+            ..Filter::default()
+        };
+        let kept = filter.reports(sample(), &rooms(), &rome());
+        assert_eq!(
+            levels_of(&kept),
+            [vec![(1, 6)], vec![(1, 0)]],
+            "the whole-home check names no room"
+        );
+        assert_eq!(
+            kept[1].values.intensity().get(),
+            0,
+            "the intensity of the rooms kept"
+        );
+    }
+
+    #[test]
+    fn an_area_keeps_its_rooms() {
+        let filter = Filter {
+            area: Some(RoomArea::Common),
+            ..Filter::default()
+        };
+        let kept = filter.reports(sample(), &rooms(), &rome());
+        assert_eq!(levels_of(&kept), [vec![(6, 2)], vec![(6, 4)]]);
+    }
+
+    #[test]
+    fn the_person_and_the_part_of_the_day_apply_to_reports_and_readings() {
+        let filter = Filter {
+            person: Some(UserId(1)),
+            day_part: Some(DayPart::Night),
+            ..Filter::default()
+        };
+        let kept = filter.reports(sample(), &rooms(), &rome());
+        assert_eq!(levels_of(&kept), [Vec::<(i64, u8)>::new()], "23:30, user 1");
+        let reading = |user, at: &str| Pm25Reading {
+            id: crate::types::ReadingId(1),
+            event_id: None,
+            user_id: UserId(user),
+            observed_at: t(at),
+            recorded_at: t(at),
+            value: crate::types::Pm25::new(10.0).unwrap(),
+            deleted_at: None,
+        };
+        let readings = vec![
+            reading(1, "2026-01-10T22:00:00Z"),
+            reading(2, "2026-01-10T22:00:00Z"),
+            reading(1, "2026-01-10T12:00:00Z"),
+        ];
+        let kept = filter.readings(readings, &rome());
+        assert_eq!(kept.len(), 1);
+        assert_eq!(
+            (kept[0].user_id, kept[0].observed_at),
+            (UserId(1), t("2026-01-10T22:00:00Z"))
+        );
+        let by_room = Filter {
+            rooms: vec![RoomId(1)],
+            min_level: 9,
+            ..Filter::default()
+        };
+        assert_eq!(
+            by_room
+                .readings(vec![reading(2, "2026-01-10T12:00:00Z")], &rome())
+                .len(),
+            1,
+            "rooms and level skip readings"
+        );
+    }
+
+    #[test]
+    fn the_minimum_level_and_hiding_clean_checks_judge_the_rooms_kept() {
+        let at_least_four = Filter {
+            min_level: 4,
+            ..Filter::default()
+        };
+        assert_eq!(
+            levels_of(&at_least_four.reports(sample(), &rooms(), &rome())),
+            [vec![(1, 6), (6, 2)], vec![(1, 0), (6, 4)]]
+        );
+        let bedroom_from_four = Filter {
+            rooms: vec![RoomId(1)],
+            min_level: 4,
+            ..Filter::default()
+        };
+        assert_eq!(
+            levels_of(&bedroom_from_four.reports(sample(), &rooms(), &rome())),
+            [vec![(1, 6)]]
+        );
+        let no_clean = Filter {
+            hide_clean: true,
+            ..Filter::default()
+        };
+        let kept = no_clean.reports(sample(), &rooms(), &rome());
+        assert_eq!(kept.len(), 3, "the whole-home check goes");
+        let bedroom_no_clean = Filter {
+            rooms: vec![RoomId(1)],
+            hide_clean: true,
+            ..Filter::default()
+        };
+        assert_eq!(
+            levels_of(&bedroom_no_clean.reports(sample(), &rooms(), &rome())),
+            [vec![(1, 6)]]
+        );
+    }
+
+    #[test]
+    fn an_event_counts_when_any_of_its_reports_is_kept() {
+        let filter = Filter {
+            area: Some(RoomArea::Common),
+            min_level: 3,
+            ..Filter::default()
+        };
+        let kept = filter.reports(sample(), &rooms(), &rome());
+        let counts = events_per_bucket(&kept, Period::month(date("2026-01-01")), &rome());
+        let events: usize = counts.iter().map(|(_, n)| n).sum();
+        assert_eq!(
+            events, 1,
+            "event 2 through its stairwell 4; event 1's stairwell 2 is too low"
+        );
     }
 
     #[test]
@@ -359,7 +739,7 @@ mod tests {
 
     #[test]
     fn an_event_counts_once_on_the_local_day_of_its_first_report() {
-        let january = Period::Month(date("2026-01-01"));
+        let january = Period::month(date("2026-01-01"));
         let reports = [
             // 00:30 on 11 January in Rome.
             report(1, 1, "2026-01-10T23:30:00Z", &[(1, 4)], &[]),
@@ -379,7 +759,7 @@ mod tests {
 
     #[test]
     fn a_year_counts_events_per_week() {
-        let year = Period::Year(date("2026-01-01"));
+        let year = Period::year(date("2026-01-01"));
         let reports = [
             report(1, 1, "2026-01-05T20:00:00Z", &[(1, 4)], &[]),
             report(2, 1, "2026-01-11T20:00:00Z", &[(1, 4)], &[]),
@@ -436,7 +816,7 @@ mod tests {
     #[test]
     fn each_area_keeps_its_highest_level_per_bucket() {
         let rooms = [room(1, RoomArea::Inside), room(6, RoomArea::Common)];
-        let january = Period::Month(date("2026-01-01"));
+        let january = Period::month(date("2026-01-01"));
         let reports = [
             report(1, 1, "2026-01-10T20:00:00Z", &[(1, 4), (6, 2)], &[]),
             report(1, 2, "2026-01-10T21:00:00Z", &[(1, 7)], &[]),
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
index dc4fbbd..8b8bad3 100644
--- a/crates/web/src/routes/graphs.rs
+++ b/crates/web/src/routes/graphs.rs
@@ -14,8 +14,8 @@ use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
 use serde::Deserialize;
 use serde_json::{Value, json};
 use smoke_domain::graphs::{
-    AreaLevels, Period, PersonLine, RoomHours, area_levels, events_per_bucket, intensity_lines,
-    room_hours, symptom_counts,
+    AreaLevels, Granularity, Period, PersonLine, RoomHours, area_levels, events_per_bucket,
+    intensity_lines, room_hours, symptom_counts,
 };
 use smoke_domain::permissions::{Action, can, can_see};
 use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
@@ -48,27 +48,46 @@ fn period(query: &GraphsQuery, today: NaiveDate) -> Period {
         .filter(|y| time::SUPPORTED_YEARS.contains(y))
         .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
     if let Some(first) = year {
-        return Period::Year(first);
+        return Period::year(first);
     }
     let month = query.month.as_deref().and_then(time::parse_month);
-    Period::Month(month.unwrap_or(today))
+    Period::month(month.unwrap_or(today))
 }
 
 /// The query string that asks for `period`.
 fn address(period: Period) -> String {
-    match period {
-        Period::Month(_) => format!("month={}", period.first_day().format("%Y-%m")),
-        Period::Year(_) => format!("year={}", period.first_day().year()),
+    match period.granularity() {
+        Granularity::Day => format!("month={}", period.first_day().format("%Y-%m")),
+        Granularity::Week => format!("year={}", period.first_day().year()),
+    }
+}
+
+/// The month or year before or after `period`.
+fn step(period: Period, forward: bool) -> Period {
+    let first = period.first_day();
+    let months = Months::new(match period.granularity() {
+        Granularity::Day => 1,
+        Granularity::Week => 12,
+    });
+    let first = if forward {
+        first + months
+    } else {
+        first - months
+    };
+    match period.granularity() {
+        Granularity::Day => Period::month(first),
+        Granularity::Week => Period::year(first),
     }
 }
 
 /// The other kind of period: a month's year, or a year's current month (January when the year
 /// is not this one).
 fn switched(period: Period, today: NaiveDate) -> Period {
-    match period {
-        Period::Month(date) => Period::Year(date),
-        Period::Year(date) if date.year() == today.year() => Period::Month(today),
-        Period::Year(date) => Period::Month(date),
+    let first = period.first_day();
+    match period.granularity() {
+        Granularity::Day => Period::year(first),
+        Granularity::Week if first.year() == today.year() => Period::month(today),
+        Granularity::Week => Period::month(first),
     }
 }
 
@@ -87,9 +106,9 @@ fn intl_locale(locale: Locale) -> &'static str {
 /// drift an hour off midnight after a clock change.
 fn tick_values(period: Period, schedule: &DailySchedule) -> Vec<i64> {
     let first = period.first_day();
-    let days: Vec<NaiveDate> = match period {
-        Period::Month(_) => period.buckets(),
-        Period::Year(_) => (0..12).map(|m| first + Months::new(m)).collect(),
+    let days: Vec<NaiveDate> = match period.granularity() {
+        Granularity::Day => period.buckets(),
+        Granularity::Week => (0..12).map(|m| first + Months::new(m)).collect(),
     };
     days.into_iter()
         .map(|day| schedule.day_span(day).0.timestamp_millis())
@@ -173,9 +192,9 @@ fn intensity_chart(
 fn bucket_labels(tr: Tr, period: Period, buckets: &[NaiveDate]) -> Vec<String> {
     buckets
         .iter()
-        .map(|&day| match period {
-            Period::Month(_) => day.day().to_string(),
-            Period::Year(_) => time::day_month(tr.0, day),
+        .map(|&day| match period.granularity() {
+            Granularity::Day => day.day().to_string(),
+            Granularity::Week => time::day_month(tr.0, day),
         })
         .collect()
 }
@@ -339,13 +358,13 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
     let lines = intensity_lines(&reports);
     let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule);
 
-    let (label, events_key, other_key) = match period {
-        Period::Month(_) => (
+    let (label, events_key, other_key) = match period.granularity() {
+        Granularity::Day => (
             period.first_day().format("%m/%Y").to_string(),
             "graphs-events-day",
             "graphs-year",
         ),
-        Period::Year(_) => (
+        Granularity::Week => (
             period.first_day().year().to_string(),
             "graphs-events-week",
             "graphs-month",
@@ -357,8 +376,8 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
             tr,
             signed_in: SignedIn::of(&user, now, Tab::Graphs),
             label,
-            previous: address(period.previous()),
-            next: address(period.next()),
+            previous: address(step(period, false)),
+            next: address(step(period, true)),
             other: address(switched(period, today)),
             other_key,
             own_only: !can(&actor, Action::ViewAll, now),
@@ -404,16 +423,16 @@ mod tests {
         let today = date("2026-10-14");
         assert_eq!(
             period(&query(Some("2026-01"), None), today),
-            Period::Month(date("2026-01-01"))
+            Period::month(date("2026-01-01"))
         );
         assert_eq!(
             period(&query(Some("2026-01"), Some("2025")), today),
-            Period::Year(date("2025-01-01"))
+            Period::year(date("2025-01-01"))
         );
         for (month, year) in [(Some("2026-13"), None), (None, Some("99999")), (None, None)] {
             assert_eq!(
                 period(&query(month, year), today),
-                Period::Month(today),
+                Period::month(today),
                 "{month:?} {year:?}"
             );
         }
@@ -422,7 +441,7 @@ mod tests {
     #[test]
     fn ticks_sit_on_local_midnights_across_a_clock_change() {
         let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
-        let ticks = tick_values(Period::Month(date("2026-10-01")), &schedule);
+        let ticks = tick_values(Period::month(date("2026-10-01")), &schedule);
         assert_eq!(ticks.len(), 31);
         let hour = 3_600_000;
         assert_eq!(ticks[24] - ticks[23], 24 * hour, "24 to 25 October");
@@ -432,7 +451,7 @@ mod tests {
             "the night of the 25th is an hour longer"
         );
         assert_eq!(ticks[26] - ticks[25], 24 * hour);
-        let year = tick_values(Period::Year(date("2026-01-01")), &schedule);
+        let year = tick_values(Period::year(date("2026-01-01")), &schedule);
         assert_eq!(year.len(), 12);
         assert_eq!(
             year[0],
@@ -444,15 +463,15 @@ mod tests {
     fn switching_keeps_the_year_and_prefers_this_month() {
         let today = date("2026-10-14");
         assert_eq!(
-            address(switched(Period::Month(date("2025-03-01")), today)),
+            address(switched(Period::month(date("2025-03-01")), today)),
             "year=2025"
         );
         assert_eq!(
-            address(switched(Period::Year(date("2026-01-01")), today)),
+            address(switched(Period::year(date("2026-01-01")), today)),
             "month=2026-10"
         );
         assert_eq!(
-            address(switched(Period::Year(date("2024-01-01")), today)),
+            address(switched(Period::year(date("2024-01-01")), today)),
             "month=2024-01"
         );
     }
````

### Task 15: Query strings as pairs: a repeated parameter never answers 400

**Proto commit:** `4cf7551` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 4cf7551`)

M6 carry-over (final review minor 1). `forms::QueryPairs` (FromRequestParts, `Infallible`) and `Pairs::contains`; Events, Log, Settings, People and Install drop their `Query` structs.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	crates/web/src/forms.rs
M	crates/web/src/routes/events.rs
M	crates/web/src/routes/install.rs
M	crates/web/src/routes/log.rs
M	crates/web/src/routes/people/mod.rs
M	crates/web/src/routes/settings.rs
M	crates/web/tests/events.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/events.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **626 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
fix(web): a repeated query parameter takes its first value instead of answering 400

QueryPairs reads the query string as pairs, like Pairs reads a form, and never rejects
the request. Events, Log, Settings, People and Install use it in place of typed Query
structs, so ?month=a&month=b or ?saved=x&saved=y fall back like any unknown value (M6 final
review, minor 1).

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `4cf7551`):

````diff
diff --git a/crates/web/src/forms.rs b/crates/web/src/forms.rs
index 9d41818..1d050ec 100644
--- a/crates/web/src/forms.rs
+++ b/crates/web/src/forms.rs
@@ -1,9 +1,12 @@
-//! Urlencoded forms whose field names repeat (`room=1&room=3`): `serde_urlencoded` cannot put
-//! those in a struct, so handlers read the pairs instead.
+//! Urlencoded forms and query strings whose names repeat (`room=1&room=3`): `serde_urlencoded`
+//! cannot put those in a struct, so handlers read the pairs instead.
+
+use std::convert::Infallible;
 
 use axum::Form;
 use axum::extract::rejection::FormRejection;
-use axum::extract::{FromRequest, Request};
+use axum::extract::{FromRequest, FromRequestParts, Query, Request};
+use axum::http::request::Parts;
 
 /// A posted form as its `(name, value)` pairs, in order.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
@@ -29,6 +32,11 @@ impl Pairs {
             .filter(|value| !value.is_empty())
     }
 
+    /// Whether `name` is there at all, even blank (`?welcome`).
+    pub(crate) fn contains(&self, name: &str) -> bool {
+        self.0.iter().any(|(key, _)| key == name)
+    }
+
     /// Every value of `name` parsed as an id; `None` if one is not a number.
     pub(crate) fn ids(&self, name: &str) -> Option<Vec<i64>> {
         self.0
@@ -68,6 +76,23 @@ impl<S: Send + Sync> FromRequest<S> for Pairs {
     }
 }
 
+/// The query string as its pairs. Unlike a typed `Query`, a repeated name (`?month=1&month=2`)
+/// or one that does not parse never answers 400: the page reads the first value, or none, and
+/// falls back to its default.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub(crate) struct QueryPairs(pub(crate) Pairs);
+
+impl<S: Send + Sync> FromRequestParts<S> for QueryPairs {
+    type Rejection = Infallible;
+
+    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
+        let pairs = Query::<Vec<(String, String)>>::try_from_uri(&parts.uri)
+            .map(|Query(pairs)| pairs)
+            .unwrap_or_default();
+        Ok(Self(Pairs(pairs)))
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/web/src/routes/events.rs b/crates/web/src/routes/events.rs
index 63b6fc6..6393a53 100644
--- a/crates/web/src/routes/events.rs
+++ b/crates/web/src/routes/events.rs
@@ -5,11 +5,10 @@
 use std::collections::{BTreeMap, HashMap};
 
 use askama::Template;
-use axum::extract::{Path, Query, State};
+use axum::extract::{Path, State};
 use axum::http::StatusCode;
 use axum::response::{IntoResponse, Redirect, Response};
 use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
-use serde::Deserialize;
 use smoke_domain::events::{FUTURE_TOLERANCE, large_gap_indices};
 use smoke_domain::logbook::{close_event, reopen_event};
 use smoke_domain::permissions::{Action, Actor, can, can_see};
@@ -20,7 +19,7 @@ use smoke_domain::types::{EventId, UserId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::forms::Pairs;
+use crate::forms::{Pairs, QueryPairs};
 use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::labels;
@@ -29,12 +28,6 @@ use crate::routes::{can_open, refusal};
 use crate::state::AppState;
 use crate::time;
 
-#[derive(Deserialize)]
-pub(crate) struct ListQuery {
-    /// `YYYY-MM`; the current month when absent or invalid.
-    month: Option<String>,
-}
-
 struct EventRow {
     id: i64,
     span: String,
@@ -96,7 +89,7 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
-    Query(query): Query<ListQuery>,
+    QueryPairs(query): QueryPairs,
 ) -> Result<Response, AppError> {
     let now = state.clock.now();
     auto_close(&state.store, &state.config.schedule, now).await?;
@@ -104,8 +97,7 @@ pub(crate) async fn list<S: Store + Clone + 'static>(
     let tz = schedule.tz();
     let actor = user.actor();
     let first = query
-        .month
-        .as_deref()
+        .get("month")
         .and_then(time::parse_month)
         .unwrap_or_else(|| schedule.local_date(now));
     let (from, to) = schedule.month_span(first);
diff --git a/crates/web/src/routes/install.rs b/crates/web/src/routes/install.rs
index 0fdcb74..ad9c9ad 100644
--- a/crates/web/src/routes/install.rs
+++ b/crates/web/src/routes/install.rs
@@ -2,14 +2,14 @@
 //! steps, and the page the service worker shows when there is no connection.
 
 use askama::Template;
-use axum::extract::{Query, State};
+use axum::extract::State;
 use axum::http::StatusCode;
 use axum::response::Response;
-use serde::Deserialize;
 use smoke_domain::repos::Store;
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
+use crate::forms::QueryPairs;
 use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::state::AppState;
@@ -23,17 +23,12 @@ struct InstallPage {
     welcome: bool,
 }
 
-#[derive(Deserialize)]
-pub(crate) struct InstallQuery {
-    welcome: Option<String>,
-}
-
 /// Open to everyone: someone may want to install before signing in.
 pub(crate) async fn show<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     user: Option<CurrentUser>,
     tr: Tr,
-    Query(query): Query<InstallQuery>,
+    QueryPairs(query): QueryPairs,
 ) -> Result<Response, AppError> {
     let now = state.clock.now();
     page(
@@ -41,7 +36,7 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
         &InstallPage {
             tr,
             signed_in: user.and_then(|CurrentUser(user)| SignedIn::of(&user, now, Tab::Settings)),
-            welcome: query.welcome.is_some(),
+            welcome: query.contains("welcome"),
         },
     )
 }
diff --git a/crates/web/src/routes/log.rs b/crates/web/src/routes/log.rs
index 768bcb2..a32f17e 100644
--- a/crates/web/src/routes/log.rs
+++ b/crates/web/src/routes/log.rs
@@ -1,12 +1,11 @@
 //! The Log screen (spec §9, screen 1): the event status, the month's summary card and the form.
 
 use askama::Template;
-use axum::extract::{Query, State};
+use axum::extract::State;
 use axum::http::StatusCode;
 use axum::response::Response;
 use chrono::{DateTime, Utc};
 use chrono_tz::Tz;
-use serde::Deserialize;
 use smoke_domain::permissions::{Action, can, can_see};
 use smoke_domain::records::{Event, Room, RoomLevel, SmellReport, Symptom, User};
 use smoke_domain::repos::Store;
@@ -17,7 +16,7 @@ use smoke_domain::types::{RoomArea, RoomId, SymptomId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::forms::Pairs;
+use crate::forms::{Pairs, QueryPairs};
 use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::entries::level_field;
@@ -249,20 +248,15 @@ pub(crate) struct LogPage {
     form: Option<FormView>,
 }
 
-#[derive(Deserialize)]
-pub(crate) struct ShowQuery {
-    saved: Option<String>,
-}
-
 pub(crate) async fn show<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
-    Query(query): Query<ShowQuery>,
+    QueryPairs(query): QueryPairs,
 ) -> Result<Response, AppError> {
     let now = state.clock.now();
     auto_close(&state.store, &state.config.schedule, now).await?;
-    let saved = match query.saved.as_deref() {
+    let saved = match query.get("saved") {
         Some("report") => Some("log-saved-report"),
         Some("pm25") => Some("log-saved-pm25"),
         Some("no-smell") => Some("log-saved-no-smell"),
diff --git a/crates/web/src/routes/people/mod.rs b/crates/web/src/routes/people/mod.rs
index a85382f..f643e6d 100644
--- a/crates/web/src/routes/people/mod.rs
+++ b/crates/web/src/routes/people/mod.rs
@@ -5,11 +5,10 @@ pub(crate) mod expiry;
 pub(crate) mod links;
 
 use askama::Template;
-use axum::extract::{Path, Query, State};
+use axum::extract::{Path, State};
 use axum::http::StatusCode;
 use axum::response::{IntoResponse, Redirect, Response};
 use chrono::{DateTime, Utc};
-use serde::Deserialize;
 use smoke_domain::DomainError;
 use smoke_domain::people::{PeopleError, expire, reactivate};
 use smoke_domain::permissions::{Action, can};
@@ -19,7 +18,7 @@ use smoke_domain::types::{Role, UserId};
 
 use crate::auth::CurrentUser;
 use crate::error::{AppError, page};
-use crate::forms::Pairs;
+use crate::forms::{Pairs, QueryPairs};
 use crate::header::{SignedIn, Tab, initial};
 use crate::i18n::Tr;
 use crate::routes::people::expiry::{ExpiryDraft, ExpiryFields};
@@ -157,19 +156,14 @@ async fn people_page<S: Store>(
     )
 }
 
-#[derive(Deserialize)]
-pub(crate) struct ListQuery {
-    saved: Option<String>,
-}
-
 pub(crate) async fn list<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
-    Query(query): Query<ListQuery>,
+    QueryPairs(query): QueryPairs,
 ) -> Result<Response, AppError> {
     require_manager(&user, state.clock.now())?;
-    let saved = match query.saved.as_deref() {
+    let saved = match query.get("saved") {
         Some("expired") => Some("people-saved-expired"),
         Some("reactivated") => Some("people-saved-reactivated"),
         _ => None,
diff --git a/crates/web/src/routes/settings.rs b/crates/web/src/routes/settings.rs
index c38f4d5..e54f392 100644
--- a/crates/web/src/routes/settings.rs
+++ b/crates/web/src/routes/settings.rs
@@ -3,7 +3,7 @@
 
 use askama::Template;
 use axum::Form;
-use axum::extract::{Query, State};
+use axum::extract::State;
 use axum::http::StatusCode;
 use axum::response::{IntoResponse, Redirect, Response};
 use serde::Deserialize;
@@ -18,6 +18,7 @@ use crate::auth::client_ip::ClientIp;
 use crate::auth::limiter::LoginLimiter;
 use crate::auth::{self, CurrentUser, password};
 use crate::error::{AppError, page};
+use crate::forms::QueryPairs;
 use crate::header::{SignedIn, Tab};
 use crate::i18n::Tr;
 use crate::routes::lists::{self, ListsView};
@@ -69,18 +70,13 @@ pub(crate) async fn settings_page<S: Store>(
     )
 }
 
-#[derive(Deserialize)]
-pub(crate) struct ShowQuery {
-    saved: Option<String>,
-}
-
 pub(crate) async fn show<S: Store + Clone + 'static>(
     State(state): State<AppState<S>>,
     CurrentUser(user): CurrentUser,
     tr: Tr,
-    Query(query): Query<ShowQuery>,
+    QueryPairs(query): QueryPairs,
 ) -> Result<Response, AppError> {
-    let saved = match query.saved.as_deref() {
+    let saved = match query.get("saved") {
         Some("language") => Some("settings-saved-language"),
         Some("password") => Some("settings-saved-password"),
         Some("lists") => Some("settings-saved-lists"),
diff --git a/crates/web/tests/events.rs b/crates/web/tests/events.rs
index 852fa19..74fefe0 100644
--- a/crates/web/tests/events.rs
+++ b/crates/web/tests/events.rs
@@ -655,3 +655,31 @@ async fn the_list_shows_each_days_checks_to_those_who_may_see_them() {
         "{html}"
     );
 }
+
+/// A repeated query parameter takes its first value instead of answering 400 (M6 carry-over).
+#[tokio::test]
+async fn a_repeated_month_takes_the_first_one() {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    app.add_user("daniele", Role::Resident, Locale::En).await;
+    let cookie = app.sign_in("daniele").await;
+    let response = app
+        .send(get_with("/events?month=2025-11&month=2025-12", &cookie))
+        .await;
+    assert_eq!(response.status(), StatusCode::OK);
+    let page = Page::parse(&text(response).await);
+    assert_eq!(page.text("nav.switcher strong"), "11/2025");
+    for path in [
+        "/?saved=report&saved=pm25",
+        "/settings?saved=password&saved=lists",
+        "/people?saved=expired&saved=x",
+    ] {
+        let response = app.send(get_with(path, &cookie)).await;
+        assert_eq!(response.status(), StatusCode::OK, "{path}");
+        assert_eq!(
+            Page::parse(&text(response).await).count(".notice[role=status]"),
+            1,
+            "{path}"
+        );
+    }
+}
````

### Task 16: Graphs: the Filters card and the filtered route

**Proto commit:** `a34605a` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show a34605a`)

Spec §7. `routes/graphs.rs` becomes `routes/graphs/{mod,charts,filters}.rs` (`git mv` then rewrite): `filters::read` (quick picks, typed dates win, area/rooms, person only from `people` and never for guests, part of day, min level, hide clean; unusable values fall back), `filters::view` (the card, the one-line summary), charts unchanged except `bucket_labels` (always day/month) and `tick_values` (granularity). The template replaces the month/year links with a collapsible GET form. `log-time-change` and `settings-change` merge into `action-change`; `graphs-month/-year` go; filter keys come. Tests: unit in `filters.rs`, route tests for each filter, the card, a viewer (M6 carry-over), a guest, a repeated parameter.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/src/routes/graphs/charts.rs
A	crates/web/src/routes/graphs/filters.rs
A	crates/web/src/routes/graphs/mod.rs
D	crates/web/src/routes/graphs.rs
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/i18n.rs
M	crates/web/templates/graphs.html
M	crates/web/templates/log.html
M	crates/web/templates/settings.html
M	crates/web/tests/graphs.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/src/routes/graphs/filters.rs (tests)`, `crates/web/tests/graphs.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **634 passed** (SQLite).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(graphs): a Filters card whose GET form puts every filter in the address

/graphs reads its filters with QueryPairs (routes/graphs/filters.rs): a quick pick (this
month, last 7 or 30 days, this year) or typed From and To; area or rooms; the person (never
for guests, and only someone the viewer may see); the part of the day; a minimum level;
hiding clean checks. Unusable values fall back to their default. The Filters card folds to a
one-line summary. Charts move to routes/graphs/charts.rs; the month/year switch is gone.
Route tests cover each filter, the card, a viewer, a guest and a repeated parameter.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `a34605a`):

````diff
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 03eebb7..7960dc1 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -26,7 +26,6 @@ log-common = Common areas
 log-symptoms = Symptoms
 log-time = Time
 log-time-now = now
-log-time-change = Change
 log-note = Note
 log-submit = Log
 log-update = Update: situation changed
@@ -76,6 +75,7 @@ summary-reports = { $count ->
 log-level-hint = Pick a level from 0 to 10.
 optional = optional
 action-add = Add
+action-change = Change
 summary-caption = This month
 tile-events = { $count ->
     [one] event
@@ -215,7 +215,6 @@ settings-password-wrong = The current password is not correct.
 settings-saved-language = Language saved.
 settings-saved-password = Password changed. Your other devices are signed out.
 settings-guides = Guides
-settings-change = Change
 settings-room-area = Area
 settings-lists-hint = Hidden items are no longer offered on the forms; reports that name them keep them.
 settings-list-hidden = hidden
@@ -292,8 +291,31 @@ offline-text = Smoke Tracker needs the internet to save what you log. Try again
 offline-retry = Try again
 
 graphs-title = Graphs
-graphs-month = Show one month
-graphs-year = Show the whole year
+graphs-filters = Filters
+graphs-dates = Dates
+graphs-range-month = This month
+graphs-range-7d = Last 7 days
+graphs-range-30d = Last 30 days
+graphs-range-year = This year
+graphs-from = From
+graphs-to = To
+graphs-where = Where
+graphs-area-all = All
+graphs-by = Reported by
+graphs-everyone = Everyone
+graphs-time = Time of day
+graphs-time-any = Any
+graphs-time-morning = Morning
+graphs-time-afternoon = Afternoon
+graphs-time-evening = Evening
+graphs-time-night = Night
+graphs-min-level = Minimum level
+graphs-level-any = any
+graphs-level-from = level { $level }+
+graphs-hide-clean = Hide clean checks
+graphs-no-clean = no clean checks
+graphs-reset = Reset
+graphs-show = Show graphs
 graphs-own-only = These graphs show only your own reports and readings.
 graphs-none = Nothing was logged in this period.
 graphs-needs-js = This graph needs JavaScript.
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index af069ac..505e8e9 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -26,7 +26,6 @@ log-common = Parti comuni
 log-symptoms = Sintomi
 log-time = Ora
 log-time-now = adesso
-log-time-change = Cambia
 log-note = Nota
 log-submit = Registra
 log-update = Aggiorna: la situazione è cambiata
@@ -76,6 +75,7 @@ summary-reports = { $count ->
 log-level-hint = Scegli un livello da 0 a 10.
 optional = facoltativo
 action-add = Aggiungi
+action-change = Cambia
 summary-caption = Questo mese
 tile-events = { $count ->
     [one] evento
@@ -215,7 +215,6 @@ settings-password-wrong = La password attuale non è corretta.
 settings-saved-language = Lingua salvata.
 settings-saved-password = Password cambiata. Gli altri dispositivi sono stati disconnessi.
 settings-guides = Guide
-settings-change = Cambia
 settings-room-area = Zona
 settings-lists-hint = Le voci nascoste non sono più proposte nei moduli; le segnalazioni che le citano le mantengono.
 settings-list-hidden = nascosto
@@ -292,8 +291,31 @@ offline-text = Smoke Tracker ha bisogno di internet per salvare ciò che registr
 offline-retry = Riprova
 
 graphs-title = Grafici
-graphs-month = Mostra un mese
-graphs-year = Mostra l'intero anno
+graphs-filters = Filtri
+graphs-dates = Date
+graphs-range-month = Questo mese
+graphs-range-7d = Ultimi 7 giorni
+graphs-range-30d = Ultimi 30 giorni
+graphs-range-year = Quest'anno
+graphs-from = Dal
+graphs-to = Al
+graphs-where = Dove
+graphs-area-all = Tutto
+graphs-by = Segnalato da
+graphs-everyone = Tutti
+graphs-time = Ora del giorno
+graphs-time-any = Qualsiasi
+graphs-time-morning = Mattina
+graphs-time-afternoon = Pomeriggio
+graphs-time-evening = Sera
+graphs-time-night = Notte
+graphs-min-level = Livello minimo
+graphs-level-any = qualsiasi
+graphs-level-from = livello { $level }+
+graphs-hide-clean = Nascondi i controlli puliti
+graphs-no-clean = senza controlli puliti
+graphs-reset = Azzera
+graphs-show = Mostra i grafici
 graphs-own-only = Questi grafici mostrano solo le tue segnalazioni e misurazioni.
 graphs-none = Nessuna registrazione in questo periodo.
 graphs-needs-js = Questo grafico richiede JavaScript.
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index dda048c..f2ef07b 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -26,7 +26,6 @@ log-common = Ortak alanlar
 log-symptoms = Belirtiler
 log-time = Saat
 log-time-now = şimdi
-log-time-change = Değiştir
 log-note = Not
 log-submit = Kaydet
 log-update = Güncelle: durum değişti
@@ -76,6 +75,7 @@ summary-reports = { $count ->
 log-level-hint = 0 ile 10 arasında bir seviye seç.
 optional = isteğe bağlı
 action-add = Ekle
+action-change = Değiştir
 summary-caption = Bu ay
 tile-events = { $count ->
     [one] olay
@@ -215,7 +215,6 @@ settings-password-wrong = Mevcut şifre doğru değil.
 settings-saved-language = Dil kaydedildi.
 settings-saved-password = Şifre değiştirildi. Diğer cihazlarınızda oturum kapatıldı.
 settings-guides = Kılavuzlar
-settings-change = Değiştir
 settings-room-area = Alan
 settings-lists-hint = Gizlenen öğeler artık formlarda sunulmaz; onları içeren kayıtlar korunur.
 settings-list-hidden = gizli
@@ -292,8 +291,31 @@ offline-text = Smoke Tracker'ın kaydettiklerinizi saklamak için internete ihti
 offline-retry = Tekrar dene
 
 graphs-title = Grafikler
-graphs-month = Tek bir ayı göster
-graphs-year = Tüm yılı göster
+graphs-filters = Filtreler
+graphs-dates = Tarihler
+graphs-range-month = Bu ay
+graphs-range-7d = Son 7 gün
+graphs-range-30d = Son 30 gün
+graphs-range-year = Bu yıl
+graphs-from = Başlangıç
+graphs-to = Bitiş
+graphs-where = Nerede
+graphs-area-all = Tümü
+graphs-by = Bildiren
+graphs-everyone = Herkes
+graphs-time = Günün saati
+graphs-time-any = Herhangi
+graphs-time-morning = Sabah
+graphs-time-afternoon = Öğleden sonra
+graphs-time-evening = Akşam
+graphs-time-night = Gece
+graphs-min-level = En düşük seviye
+graphs-level-any = herhangi
+graphs-level-from = seviye { $level }+
+graphs-hide-clean = Temiz kontrolleri gizle
+graphs-no-clean = temiz kontroller yok
+graphs-reset = Sıfırla
+graphs-show = Grafikleri göster
 graphs-own-only = Bu grafikler yalnızca kendi bildirimlerinizi ve ölçümlerinizi gösterir.
 graphs-none = Bu dönemde hiçbir kayıt yok.
 graphs-needs-js = Bu grafik için JavaScript gerekir.
diff --git a/crates/web/src/i18n.rs b/crates/web/src/i18n.rs
index e2eb4c7..6998831 100644
--- a/crates/web/src/i18n.rs
+++ b/crates/web/src/i18n.rs
@@ -263,7 +263,7 @@ mod tests {
         used.extend(settings::keys().into_iter().map(str::to_owned));
         used.extend(lists::KEYS.map(str::to_owned));
         used.extend(people::keys().into_iter().map(str::to_owned));
-        used.extend(graphs::KEYS.map(str::to_owned));
+        used.extend(graphs::keys().into_iter().map(str::to_owned));
         let english = message_ids(Locale::En);
         let missing: Vec<_> = used.difference(&english).collect();
         assert!(
diff --git a/crates/web/src/routes/graphs.rs b/crates/web/src/routes/graphs.rs
deleted file mode 100644
index 8b8bad3..0000000
--- a/crates/web/src/routes/graphs.rs
+++ /dev/null
@@ -1,478 +0,0 @@
-//! The Graphs screen (spec §9 screen 5). Each chart is a `<canvas>` whose Chart.js
-//! configuration travels as JSON in its `data-chart` attribute; `static/js/graphs.js` draws it, so
-//! no page carries an inline script. The room × hour heatmap is a plain table, coloured by CSS
-//! classes. The numbers come from `smoke_domain::graphs`, over what the viewer may see: guests
-//! get graphs of their own data only (spec §4).
-
-use std::collections::HashMap;
-
-use askama::Template;
-use axum::extract::{Query, State};
-use axum::http::StatusCode;
-use axum::response::Response;
-use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
-use serde::Deserialize;
-use serde_json::{Value, json};
-use smoke_domain::graphs::{
-    AreaLevels, Granularity, Period, PersonLine, RoomHours, area_levels, events_per_bucket,
-    intensity_lines, room_hours, symptom_counts,
-};
-use smoke_domain::permissions::{Action, can, can_see};
-use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
-use smoke_domain::repos::Store;
-use smoke_domain::schedule::DailySchedule;
-use smoke_domain::types::{Locale, UserId};
-
-use crate::auth::CurrentUser;
-use crate::error::{AppError, page};
-use crate::header::{SignedIn, Tab};
-use crate::i18n::Tr;
-use crate::labels;
-use crate::state::AppState;
-use crate::time;
-
-#[derive(Deserialize)]
-pub(crate) struct GraphsQuery {
-    /// `YYYY-MM`: that month, counted per day.
-    month: Option<String>,
-    /// `YYYY`: that year, counted per week. Wins over `month`.
-    year: Option<String>,
-}
-
-/// The period an address asks for; the current local month when it asks for none we accept.
-fn period(query: &GraphsQuery, today: NaiveDate) -> Period {
-    let year = query
-        .year
-        .as_deref()
-        .and_then(|y| y.parse::<i32>().ok())
-        .filter(|y| time::SUPPORTED_YEARS.contains(y))
-        .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
-    if let Some(first) = year {
-        return Period::year(first);
-    }
-    let month = query.month.as_deref().and_then(time::parse_month);
-    Period::month(month.unwrap_or(today))
-}
-
-/// The query string that asks for `period`.
-fn address(period: Period) -> String {
-    match period.granularity() {
-        Granularity::Day => format!("month={}", period.first_day().format("%Y-%m")),
-        Granularity::Week => format!("year={}", period.first_day().year()),
-    }
-}
-
-/// The month or year before or after `period`.
-fn step(period: Period, forward: bool) -> Period {
-    let first = period.first_day();
-    let months = Months::new(match period.granularity() {
-        Granularity::Day => 1,
-        Granularity::Week => 12,
-    });
-    let first = if forward {
-        first + months
-    } else {
-        first - months
-    };
-    match period.granularity() {
-        Granularity::Day => Period::month(first),
-        Granularity::Week => Period::year(first),
-    }
-}
-
-/// The other kind of period: a month's year, or a year's current month (January when the year
-/// is not this one).
-fn switched(period: Period, today: NaiveDate) -> Period {
-    let first = period.first_day();
-    match period.granularity() {
-        Granularity::Day => Period::year(first),
-        Granularity::Week if first.year() == today.year() => Period::month(today),
-        Granularity::Week => Period::month(first),
-    }
-}
-
-/// The language tag `Intl.DateTimeFormat` gets, chosen so that dates come out day first as on
-/// every other page (decision #95).
-fn intl_locale(locale: Locale) -> &'static str {
-    match locale {
-        Locale::En => "en-GB",
-        Locale::It => "it-IT",
-        Locale::Tr => "tr-TR",
-    }
-}
-
-/// Where the time axis puts its ticks, as milliseconds since the epoch: the local midnight of
-/// each day of a month, or of each month's first day in a year. Fixed steps of 24 hours would
-/// drift an hour off midnight after a clock change.
-fn tick_values(period: Period, schedule: &DailySchedule) -> Vec<i64> {
-    let first = period.first_day();
-    let days: Vec<NaiveDate> = match period.granularity() {
-        Granularity::Day => period.buckets(),
-        Granularity::Week => (0..12).map(|m| first + Months::new(m)).collect(),
-    };
-    days.into_iter()
-        .map(|day| schedule.day_span(day).0.timestamp_millis())
-        .collect()
-}
-
-/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
-/// on the right (spec §9). The x axis is milliseconds since the epoch.
-fn intensity_chart(
-    tr: Tr,
-    lines: &[PersonLine],
-    names: &HashMap<UserId, String>,
-    readings: &[Pm25Reading],
-    period: Period,
-    (from, to): (DateTime<Utc>, DateTime<Utc>),
-    schedule: &DailySchedule,
-) -> Value {
-    let mut datasets: Vec<Value> = lines
-        .iter()
-        .map(|line| {
-            let points: Vec<Value> = line
-                .points
-                .iter()
-                .map(|(at, level)| json!({ "x": at.timestamp_millis(), "y": level.get() }))
-                .collect();
-            json!({
-                "label": names.get(&line.user).map_or("", String::as_str),
-                "data": points,
-                "yAxisID": "level",
-            })
-        })
-        .collect();
-    if !readings.is_empty() {
-        let points: Vec<Value> = readings
-            .iter()
-            .map(|r| json!({ "x": r.observed_at.timestamp_millis(), "y": r.value.ugm3() }))
-            .collect();
-        datasets.push(json!({
-            "label": tr.t("graphs-pm25"),
-            "data": points,
-            "yAxisID": "pm25",
-            "borderDash": [6, 4],
-        }));
-    }
-    json!({
-        "type": "line",
-        "data": { "datasets": datasets },
-        "options": {
-            "scales": {
-                "x": {
-                    "type": "linear",
-                    "min": from.timestamp_millis(),
-                    "max": (to - TimeDelta::milliseconds(1)).timestamp_millis(),
-                },
-                "level": {
-                    "type": "linear",
-                    "position": "left",
-                    "min": 0,
-                    "max": 10,
-                    "title": { "display": true, "text": tr.t("rating-level") },
-                },
-                "pm25": {
-                    "type": "linear",
-                    "position": "right",
-                    "display": "auto",
-                    "beginAtZero": true,
-                    "grid": { "drawOnChartArea": false },
-                    "title": { "display": true, "text": tr.t("graphs-pm25") },
-                },
-            },
-        },
-        "time": {
-            "zone": schedule.tz().name(),
-            "locale": intl_locale(tr.0),
-            "ticks": tick_values(period, schedule),
-        },
-    })
-}
-
-/// The label under each bucket: the day of the month, or a week's Monday as day and month.
-fn bucket_labels(tr: Tr, period: Period, buckets: &[NaiveDate]) -> Vec<String> {
-    buckets
-        .iter()
-        .map(|&day| match period.granularity() {
-            Granularity::Day => day.day().to_string(),
-            Granularity::Week => time::day_month(tr.0, day),
-        })
-        .collect()
-}
-
-/// A bar chart of whole numbers with no legend; `across` lays the bars along the y axis, for
-/// long labels such as symptom names.
-fn count_chart(label: String, labels: Vec<String>, counts: Vec<usize>, across: bool) -> Value {
-    let (index, value) = if across { ("y", "x") } else { ("x", "y") };
-    json!({
-        "type": "bar",
-        "data": {
-            "labels": labels,
-            "datasets": [{ "label": label, "data": counts }],
-        },
-        "options": {
-            "indexAxis": index,
-            "scales": { value: { "beginAtZero": true, "ticks": { "precision": 0 } } },
-            "plugins": { "legend": { "display": false } },
-        },
-    })
-}
-
-/// How often each symptom was reported, most frequent first, by its shown name.
-fn symptom_chart(tr: Tr, reports: &[SmellReport], symptoms: &[Symptom]) -> Option<Value> {
-    let counts = symptom_counts(reports);
-    if counts.is_empty() {
-        return None;
-    }
-    let (names, counts) = counts
-        .into_iter()
-        .filter_map(|(id, count)| {
-            let symptom = symptoms.iter().find(|s| s.id == id)?;
-            Some((labels::symptom(tr, symptom), count))
-        })
-        .unzip();
-    Some(count_chart(
-        tr.t("graphs-symptoms-label"),
-        names,
-        counts,
-        true,
-    ))
-}
-
-/// Inside the home against common areas: each area's highest level per bucket, side by side.
-fn area_chart(tr: Tr, labels: Vec<String>, levels: &[(NaiveDate, AreaLevels)]) -> Value {
-    let series = |pick: fn(&AreaLevels) -> Option<u8>| -> Vec<Option<u8>> {
-        levels.iter().map(|(_, l)| pick(l)).collect()
-    };
-    json!({
-        "type": "bar",
-        "data": {
-            "labels": labels,
-            "datasets": [
-                { "label": tr.t("log-inside"), "data": series(|l| l.inside.map(|v| v.get())) },
-                { "label": tr.t("log-common"), "data": series(|l| l.common.map(|v| v.get())) },
-            ],
-        },
-        "options": {
-            "scales": {
-                "y": {
-                    "min": 0,
-                    "max": 10,
-                    "title": { "display": true, "text": tr.t("rating-level") },
-                },
-            },
-        },
-    })
-}
-
-/// One cell of the heatmap: the room's average level at that hour, rounded, which also picks
-/// the colour (`heat-0` to `heat-10`). `None` where nobody checked the room.
-type HeatCell = Option<u8>;
-
-struct HeatRow {
-    room: String,
-    cells: Vec<HeatCell>,
-}
-
-fn heat_rows(tr: Tr, rows: Vec<RoomHours>, rooms: &[Room]) -> Vec<HeatRow> {
-    rows.into_iter()
-        .filter_map(|row| {
-            let room = rooms.iter().find(|r| r.id == row.room)?;
-            Some(HeatRow {
-                room: labels::room(tr, room),
-                cells: row
-                    .hours
-                    .iter()
-                    .map(|tally| tally.mean().map(|mean| mean.round() as u8))
-                    .collect(),
-            })
-        })
-        .collect()
-}
-
-#[derive(Template)]
-#[template(path = "graphs.html")]
-struct GraphsPage {
-    tr: Tr,
-    signed_in: Option<SignedIn>,
-    label: String,
-    previous: String,
-    next: String,
-    other: String,
-    other_key: &'static str,
-    /// Guests see only their own data; the page says so.
-    own_only: bool,
-    empty: bool,
-    intensity: String,
-    events_key: &'static str,
-    events: String,
-    hours: std::ops::Range<u8>,
-    heatmap: Vec<HeatRow>,
-    symptoms: Option<String>,
-    areas: String,
-}
-
-pub(crate) async fn show<S: Store + Clone + 'static>(
-    State(state): State<AppState<S>>,
-    CurrentUser(user): CurrentUser,
-    tr: Tr,
-    Query(query): Query<GraphsQuery>,
-) -> Result<Response, AppError> {
-    let now = state.clock.now();
-    let schedule = &state.config.schedule;
-    let today = schedule.local_date(now);
-    let actor = user.actor();
-    let period = period(&query, today);
-    let span = period.span(schedule);
-
-    let reports: Vec<_> = state
-        .store
-        .reports_between(span.0, span.1)
-        .await?
-        .into_iter()
-        .filter(|r| can_see(&actor, r.user_id, now))
-        .collect();
-    let readings: Vec<_> = state
-        .store
-        .readings_between(span.0, span.1)
-        .await?
-        .into_iter()
-        .filter(|r| can_see(&actor, r.user_id, now))
-        .collect();
-    let names: HashMap<UserId, String> = state
-        .store
-        .users()
-        .await?
-        .into_iter()
-        .map(|u| (u.id, u.username.to_string()))
-        .collect();
-
-    let buckets = period.buckets();
-    let labels = bucket_labels(tr, period, &buckets);
-    let events = events_per_bucket(&reports, period, schedule)
-        .into_iter()
-        .map(|(_, count)| count)
-        .collect();
-    let rooms = state.store.rooms().await?;
-    let symptoms = state.store.symptoms().await?;
-    let areas = area_levels(&reports, &rooms, period, schedule);
-    let lines = intensity_lines(&reports);
-    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule);
-
-    let (label, events_key, other_key) = match period.granularity() {
-        Granularity::Day => (
-            period.first_day().format("%m/%Y").to_string(),
-            "graphs-events-day",
-            "graphs-year",
-        ),
-        Granularity::Week => (
-            period.first_day().year().to_string(),
-            "graphs-events-week",
-            "graphs-month",
-        ),
-    };
-    page(
-        StatusCode::OK,
-        &GraphsPage {
-            tr,
-            signed_in: SignedIn::of(&user, now, Tab::Graphs),
-            label,
-            previous: address(step(period, false)),
-            next: address(step(period, true)),
-            other: address(switched(period, today)),
-            other_key,
-            own_only: !can(&actor, Action::ViewAll, now),
-            empty: reports.is_empty() && readings.is_empty(),
-            intensity: intensity.to_string(),
-            events_key,
-            events: count_chart(tr.t("graphs-events-label"), labels.clone(), events, false)
-                .to_string(),
-            hours: 0..24,
-            heatmap: heat_rows(tr, room_hours(&reports, schedule), &rooms),
-            symptoms: symptom_chart(tr, &reports, &symptoms).map(|chart| chart.to_string()),
-            areas: area_chart(tr, labels, &areas).to_string(),
-        },
-    )
-}
-
-/// Every key passed to the template through a variable, for the translation tests.
-#[cfg(test)]
-pub(crate) const KEYS: [&str; 4] = [
-    "graphs-events-day",
-    "graphs-events-week",
-    "graphs-year",
-    "graphs-month",
-];
-
-#[cfg(test)]
-mod tests {
-    use super::*;
-
-    fn date(s: &str) -> NaiveDate {
-        s.parse().unwrap()
-    }
-
-    fn query(month: Option<&str>, year: Option<&str>) -> GraphsQuery {
-        GraphsQuery {
-            month: month.map(str::to_owned),
-            year: year.map(str::to_owned),
-        }
-    }
-
-    #[test]
-    fn the_address_picks_a_month_or_a_year_and_falls_back_to_this_month() {
-        let today = date("2026-10-14");
-        assert_eq!(
-            period(&query(Some("2026-01"), None), today),
-            Period::month(date("2026-01-01"))
-        );
-        assert_eq!(
-            period(&query(Some("2026-01"), Some("2025")), today),
-            Period::year(date("2025-01-01"))
-        );
-        for (month, year) in [(Some("2026-13"), None), (None, Some("99999")), (None, None)] {
-            assert_eq!(
-                period(&query(month, year), today),
-                Period::month(today),
-                "{month:?} {year:?}"
-            );
-        }
-    }
-
-    #[test]
-    fn ticks_sit_on_local_midnights_across_a_clock_change() {
-        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
-        let ticks = tick_values(Period::month(date("2026-10-01")), &schedule);
-        assert_eq!(ticks.len(), 31);
-        let hour = 3_600_000;
-        assert_eq!(ticks[24] - ticks[23], 24 * hour, "24 to 25 October");
-        assert_eq!(
-            ticks[25] - ticks[24],
-            25 * hour,
-            "the night of the 25th is an hour longer"
-        );
-        assert_eq!(ticks[26] - ticks[25], 24 * hour);
-        let year = tick_values(Period::year(date("2026-01-01")), &schedule);
-        assert_eq!(year.len(), 12);
-        assert_eq!(
-            year[0],
-            schedule.year_span(date("2026-01-01")).0.timestamp_millis()
-        );
-    }
-
-    #[test]
-    fn switching_keeps_the_year_and_prefers_this_month() {
-        let today = date("2026-10-14");
-        assert_eq!(
-            address(switched(Period::month(date("2025-03-01")), today)),
-            "year=2025"
-        );
-        assert_eq!(
-            address(switched(Period::year(date("2026-01-01")), today)),
-            "month=2026-10"
-        );
-        assert_eq!(
-            address(switched(Period::year(date("2024-01-01")), today)),
-            "month=2024-01"
-        );
-    }
-}
diff --git a/crates/web/src/routes/graphs/charts.rs b/crates/web/src/routes/graphs/charts.rs
new file mode 100644
index 0000000..6bab67c
--- /dev/null
+++ b/crates/web/src/routes/graphs/charts.rs
@@ -0,0 +1,270 @@
+//! The Chart.js configurations of the Graphs screen, as JSON, and the rows of the room × hour
+//! table. Each takes series from `smoke_domain::graphs` and adds only labels and axes.
+
+use std::collections::HashMap;
+
+use chrono::{DateTime, Datelike, Months, NaiveDate, TimeDelta, Utc};
+use serde_json::{Value, json};
+use smoke_domain::graphs::{
+    AreaLevels, Granularity, Period, PersonLine, RoomHours, symptom_counts,
+};
+use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
+use smoke_domain::schedule::DailySchedule;
+use smoke_domain::types::{Locale, UserId};
+
+use crate::i18n::Tr;
+use crate::{labels, time};
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
+/// Where the time axis puts its ticks, as milliseconds since the epoch: the local midnight of
+/// each day of a period counted per day, or of each month's first day in one counted per week. Fixed steps of 24 hours would
+/// drift an hour off midnight after a clock change.
+fn tick_values(period: Period, schedule: &DailySchedule) -> Vec<i64> {
+    let days: Vec<NaiveDate> = match period.granularity() {
+        Granularity::Day => period.buckets(),
+        Granularity::Week => {
+            let month = period
+                .first_day()
+                .with_day(1)
+                .expect("every month has a first day");
+            std::iter::successors(Some(month), |&first| Some(first + Months::new(1)))
+                .take_while(|&first| first <= period.last_day())
+                .filter(|&first| first >= period.first_day())
+                .collect()
+        }
+    };
+    days.into_iter()
+        .map(|day| schedule.day_span(day).0.timestamp_millis())
+        .collect()
+}
+
+/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
+/// on the right (spec §9). The x axis is milliseconds since the epoch.
+pub(crate) fn intensity_chart(
+    tr: Tr,
+    lines: &[PersonLine],
+    names: &HashMap<UserId, String>,
+    readings: &[Pm25Reading],
+    period: Period,
+    (from, to): (DateTime<Utc>, DateTime<Utc>),
+    schedule: &DailySchedule,
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
+    json!({
+        "type": "line",
+        "data": { "datasets": datasets },
+        "options": {
+            "scales": {
+                "x": {
+                    "type": "linear",
+                    "min": from.timestamp_millis(),
+                    "max": (to - TimeDelta::milliseconds(1)).timestamp_millis(),
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
+        "time": {
+            "zone": schedule.tz().name(),
+            "locale": intl_locale(tr.0),
+            "ticks": tick_values(period, schedule),
+        },
+    })
+}
+
+/// The label under each bucket: its day, or a week's Monday, as day and month.
+pub(crate) fn bucket_labels(tr: Tr, buckets: &[NaiveDate]) -> Vec<String> {
+    buckets
+        .iter()
+        .map(|&day| time::day_month(tr.0, day))
+        .collect()
+}
+
+/// A bar chart of whole numbers with no legend; `across` lays the bars along the y axis, for
+/// long labels such as symptom names.
+pub(crate) fn count_chart(
+    label: String,
+    labels: Vec<String>,
+    counts: Vec<usize>,
+    across: bool,
+) -> Value {
+    let (index, value) = if across { ("y", "x") } else { ("x", "y") };
+    json!({
+        "type": "bar",
+        "data": {
+            "labels": labels,
+            "datasets": [{ "label": label, "data": counts }],
+        },
+        "options": {
+            "indexAxis": index,
+            "scales": { value: { "beginAtZero": true, "ticks": { "precision": 0 } } },
+            "plugins": { "legend": { "display": false } },
+        },
+    })
+}
+
+/// How often each symptom was reported, most frequent first, by its shown name.
+pub(crate) fn symptom_chart(
+    tr: Tr,
+    reports: &[SmellReport],
+    symptoms: &[Symptom],
+) -> Option<Value> {
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
+pub(crate) fn area_chart(tr: Tr, labels: Vec<String>, levels: &[(NaiveDate, AreaLevels)]) -> Value {
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
+pub(crate) type HeatCell = Option<u8>;
+
+pub(crate) struct HeatRow {
+    pub(crate) room: String,
+    pub(crate) cells: Vec<HeatCell>,
+}
+
+pub(crate) fn heat_rows(tr: Tr, rows: Vec<RoomHours>, rooms: &[Room]) -> Vec<HeatRow> {
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
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    fn date(s: &str) -> NaiveDate {
+        s.parse().unwrap()
+    }
+
+    #[test]
+    fn ticks_sit_on_local_midnights_across_a_clock_change() {
+        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
+        let ticks = tick_values(Period::month(date("2026-10-01")), &schedule);
+        assert_eq!(ticks.len(), 31);
+        let hour = 3_600_000;
+        assert_eq!(ticks[24] - ticks[23], 24 * hour, "24 to 25 October");
+        assert_eq!(
+            ticks[25] - ticks[24],
+            25 * hour,
+            "the night of the 25th is an hour longer"
+        );
+        assert_eq!(ticks[26] - ticks[25], 24 * hour);
+        let year = tick_values(Period::year(date("2026-01-01")), &schedule);
+        assert_eq!(year.len(), 12);
+        assert_eq!(
+            year[0],
+            schedule.year_span(date("2026-01-01")).0.timestamp_millis()
+        );
+    }
+
+    #[test]
+    fn weekly_ticks_sit_on_the_first_of_each_month_inside_the_period() {
+        let schedule = DailySchedule::new(chrono_tz::Europe::Rome, 4).unwrap();
+        let period = Period::new(date("2026-01-15"), date("2026-06-30")).unwrap();
+        let ticks = tick_values(period, &schedule);
+        assert_eq!(ticks.len(), 5, "February to June");
+        assert_eq!(
+            ticks[0],
+            schedule.day_span(date("2026-02-01")).0.timestamp_millis()
+        );
+    }
+}
diff --git a/crates/web/src/routes/graphs/filters.rs b/crates/web/src/routes/graphs/filters.rs
new file mode 100644
index 0000000..55d49a4
--- /dev/null
+++ b/crates/web/src/routes/graphs/filters.rs
@@ -0,0 +1,454 @@
+//! The Graphs filters (look-and-feel spec §7): read from the address, so a view can be
+//! bookmarked or shared, and shown again as the Filters card with a one-line summary.
+
+use chrono::{Datelike, NaiveDate};
+use smoke_domain::graphs::{DayPart, Filter, Period};
+use smoke_domain::records::{Room, User};
+use smoke_domain::types::{Level, RoomArea, RoomId, UserId};
+
+use crate::forms::Pairs;
+use crate::i18n::Tr;
+use crate::routes::log::Choice;
+use crate::routes::people::role_key;
+use crate::{labels, time};
+
+/// The quick picks of the period: (address value, translation key).
+const RANGES: [(&str, &str); 4] = [
+    ("month", "graphs-range-month"),
+    ("7d", "graphs-range-7d"),
+    ("30d", "graphs-range-30d"),
+    ("year", "graphs-range-year"),
+];
+
+/// The areas: (address value, translation key); "all" is no area.
+const AREAS: [(&str, &str); 3] = [
+    ("all", "graphs-area-all"),
+    ("inside", "log-inside"),
+    ("common", "log-common"),
+];
+
+fn range_period(value: &str, today: NaiveDate) -> Option<Period> {
+    match value {
+        "month" => Some(Period::month(today)),
+        "7d" => Some(Period::last_days(today, 7)),
+        "30d" => Some(Period::last_days(today, 30)),
+        "year" => Some(Period::year(today)),
+        _ => None,
+    }
+}
+
+fn day_part_value(part: DayPart) -> &'static str {
+    match part {
+        DayPart::Morning => "morning",
+        DayPart::Afternoon => "afternoon",
+        DayPart::Evening => "evening",
+        DayPart::Night => "night",
+    }
+}
+
+fn day_part_key(part: DayPart) -> &'static str {
+    match part {
+        DayPart::Morning => "graphs-time-morning",
+        DayPart::Afternoon => "graphs-time-afternoon",
+        DayPart::Evening => "graphs-time-evening",
+        DayPart::Night => "graphs-time-night",
+    }
+}
+
+/// What the address asks for.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub(crate) struct Selection {
+    /// The quick pick, or `None` for dates typed in From and To.
+    pub(crate) range: Option<&'static str>,
+    pub(crate) period: Period,
+    pub(crate) filter: Filter,
+}
+
+/// Reads the filters from the query string. Anything missing or unusable falls back to its
+/// default: this month, all rooms, everyone, any time, any level. Typed dates win over a quick
+/// pick when both are valid and form a period. A person is kept only when `people` may be
+/// chosen from (never for guests) and is one of them.
+pub(crate) fn read(query: &Pairs, today: NaiveDate, people: &[User]) -> Selection {
+    let date = |name| {
+        query
+            .get(name)
+            .and_then(|value| value.parse::<NaiveDate>().ok())
+    };
+    let typed = date("from")
+        .zip(date("to"))
+        .and_then(|(first, last)| Period::new(first, last))
+        .filter(|period| {
+            [period.first_day(), period.last_day()]
+                .iter()
+                .all(|day| time::SUPPORTED_YEARS.contains(&day.year()))
+        });
+    let (range, period) = match typed {
+        Some(period) => (None, period),
+        None => RANGES
+            .iter()
+            .find(|(value, _)| query.get("range") == Some(*value))
+            .and_then(|&(value, _)| range_period(value, today).map(|p| (Some(value), p)))
+            .unwrap_or((Some("month"), Period::month(today))),
+    };
+    let area = match query.get("area") {
+        Some("inside") => Some(RoomArea::Inside),
+        Some("common") => Some(RoomArea::Common),
+        _ => None,
+    };
+    let person = query
+        .get("by")
+        .and_then(|value| value.parse().ok())
+        .map(UserId)
+        .filter(|id| people.iter().any(|user| user.id == *id));
+    Selection {
+        range,
+        period,
+        filter: Filter {
+            rooms: query
+                .ids("room")
+                .unwrap_or_default()
+                .into_iter()
+                .map(RoomId)
+                .collect(),
+            area,
+            person,
+            day_part: DayPart::ALL
+                .into_iter()
+                .find(|&part| query.get("time") == Some(day_part_value(part))),
+            min_level: query
+                .get("min")
+                .and_then(|value| value.parse().ok())
+                .filter(|level| *level <= Level::MAX)
+                .unwrap_or(0),
+            hide_clean: query.get("clean") == Some("hide"),
+        },
+    }
+}
+
+/// A radio button with a text value.
+pub(crate) struct Pick {
+    pub(crate) value: &'static str,
+    pub(crate) label: String,
+    pub(crate) checked: bool,
+}
+
+/// A person the graphs can be narrowed to.
+pub(crate) struct PersonOption {
+    pub(crate) value: i64,
+    pub(crate) name: String,
+    pub(crate) role: String,
+    pub(crate) selected: bool,
+}
+
+/// The Filters card.
+pub(crate) struct FiltersView {
+    /// "This month · Bedroom · daniele · 22–06 · level 5+".
+    pub(crate) summary: String,
+    pub(crate) ranges: Vec<Pick>,
+    /// The typed dates, empty while a quick pick is chosen.
+    pub(crate) from: String,
+    pub(crate) to: String,
+    pub(crate) areas: Vec<Pick>,
+    pub(crate) rooms: Vec<Choice>,
+    /// `None` for guests, who see only their own data.
+    pub(crate) people: Option<Vec<PersonOption>>,
+    pub(crate) times: Vec<Pick>,
+    pub(crate) min_level: u8,
+    /// What the minimum level reads as: "any" for 0, else "5+".
+    pub(crate) min_label: String,
+    pub(crate) hide_clean: bool,
+}
+
+/// "22–06": the hours of a part of the day.
+fn hours(part: DayPart) -> String {
+    let (start, end) = part.hours();
+    format!("{start:02}–{end:02}")
+}
+
+fn min_label(tr: Tr, level: u8) -> String {
+    if level == 0 {
+        tr.t("graphs-level-any")
+    } else {
+        format!("{level}+")
+    }
+}
+
+/// The Filters card for `selection`. `people` is `None` for guests.
+pub(crate) fn view(
+    tr: Tr,
+    selection: &Selection,
+    rooms: &[Room],
+    people: Option<&[User]>,
+) -> FiltersView {
+    let filter = &selection.filter;
+    let period = selection.period;
+    let typed = selection.range.is_none();
+    let date_value = |date: NaiveDate| {
+        if typed {
+            date.to_string()
+        } else {
+            String::new()
+        }
+    };
+    let mut summary = vec![match selection.range {
+        Some(range) => tr.t(RANGES
+            .iter()
+            .find(|(value, _)| *value == range)
+            .map_or("", |r| r.1)),
+        None => format!(
+            "{}–{}",
+            time::date(tr.0, period.first_day()),
+            time::date(tr.0, period.last_day())
+        ),
+    }];
+    let room_names: Vec<String> = filter
+        .rooms
+        .iter()
+        .map(|&id| labels::room_by_id(tr, rooms, id))
+        .filter(|name| !name.is_empty())
+        .collect();
+    let area = match filter.area {
+        Some(RoomArea::Inside) => "inside",
+        Some(RoomArea::Common) => "common",
+        None => "all",
+    };
+    if !room_names.is_empty() {
+        summary.push(room_names.join(", "));
+    } else if filter.area.is_some() {
+        summary.push(
+            tr.t(AREAS
+                .iter()
+                .find(|(value, _)| *value == area)
+                .map_or("", |a| a.1)),
+        );
+    }
+    if let Some(person) = filter
+        .person
+        .and_then(|id| people?.iter().find(|u| u.id == id))
+    {
+        summary.push(person.username.to_string());
+    }
+    if let Some(part) = filter.day_part {
+        summary.push(hours(part));
+    }
+    if filter.min_level > 0 {
+        summary.push(tr.t1("graphs-level-from", "level", &filter.min_level.to_string()));
+    }
+    if filter.hide_clean {
+        summary.push(tr.t("graphs-no-clean"));
+    }
+    FiltersView {
+        summary: summary.join(" · "),
+        ranges: RANGES
+            .iter()
+            .map(|&(value, key)| Pick {
+                value,
+                label: tr.t(key),
+                checked: selection.range == Some(value),
+            })
+            .collect(),
+        from: date_value(period.first_day()),
+        to: date_value(period.last_day()),
+        areas: AREAS
+            .iter()
+            .map(|&(value, key)| Pick {
+                value,
+                label: tr.t(key),
+                checked: value == area,
+            })
+            .collect(),
+        rooms: rooms
+            .iter()
+            .filter(|room| room.hidden_at.is_none() || filter.rooms.contains(&room.id))
+            .map(|room| Choice {
+                value: room.id.0,
+                label: labels::room(tr, room),
+                checked: filter.rooms.contains(&room.id),
+            })
+            .collect(),
+        people: people.map(|people| {
+            people
+                .iter()
+                .map(|user| PersonOption {
+                    value: user.id.0,
+                    name: user.username.to_string(),
+                    role: tr.t(role_key(user.role)),
+                    selected: filter.person == Some(user.id),
+                })
+                .collect()
+        }),
+        times: std::iter::once(Pick {
+            value: "",
+            label: tr.t("graphs-time-any"),
+            checked: filter.day_part.is_none(),
+        })
+        .chain(DayPart::ALL.into_iter().map(|part| Pick {
+            value: day_part_value(part),
+            label: format!("{} {}", tr.t(day_part_key(part)), hours(part)),
+            checked: filter.day_part == Some(part),
+        }))
+        .collect(),
+        min_level: filter.min_level,
+        min_label: min_label(tr, filter.min_level),
+        hide_clean: filter.hide_clean,
+    }
+}
+
+/// Every key this module looks up through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    RANGES
+        .iter()
+        .chain(&AREAS)
+        .map(|&(_, key)| key)
+        .chain(DayPart::ALL.into_iter().map(day_part_key))
+        .collect()
+}
+
+#[cfg(test)]
+mod tests {
+    use smoke_domain::records::User;
+    use smoke_domain::types::{Locale, Role, Username};
+
+    use super::*;
+
+    fn date(s: &str) -> NaiveDate {
+        s.parse().unwrap()
+    }
+
+    fn user(id: i64, name: &str) -> User {
+        User {
+            id: UserId(id),
+            username: Username::new(name).unwrap(),
+            password_hash: None,
+            role: Role::Resident,
+            locale: Locale::En,
+            expires_at: None,
+            created_at: "2026-01-01T00:00:00Z".parse().unwrap(),
+        }
+    }
+
+    fn read_query(pairs: &[(&str, &str)]) -> Selection {
+        read(
+            &Pairs::of(pairs),
+            date("2026-10-14"),
+            &[user(1, "daniele"), user(2, "partner")],
+        )
+    }
+
+    #[test]
+    fn without_filters_the_graphs_show_this_month_and_everything() {
+        let selection = read_query(&[]);
+        assert_eq!(selection.range, Some("month"));
+        assert_eq!(selection.period, Period::month(date("2026-10-01")));
+        assert_eq!(selection.filter, Filter::default());
+    }
+
+    #[test]
+    fn a_quick_pick_or_typed_dates_choose_the_period() {
+        assert_eq!(
+            read_query(&[("range", "7d")]).period,
+            Period::last_days(date("2026-10-14"), 7)
+        );
+        assert_eq!(
+            read_query(&[("range", "year")]).period,
+            Period::year(date("2026-10-14"))
+        );
+        let typed = read_query(&[
+            ("range", "7d"),
+            ("from", "2026-01-01"),
+            ("to", "2026-03-31"),
+        ]);
+        assert_eq!(typed.range, None, "typed dates win");
+        assert_eq!(
+            typed.period,
+            Period::new(date("2026-01-01"), date("2026-03-31")).unwrap()
+        );
+        for bad in [
+            &[("range", "decade")][..],
+            &[("from", "2026-03-31"), ("to", "2026-01-01")][..],
+            &[("from", "2020-01-01"), ("to", "2026-01-01")][..],
+            &[("from", "1999-01-01"), ("to", "1999-01-02")][..],
+            &[("from", "2026-01-01")][..],
+            &[("from", "junk"), ("to", "2026-01-01")][..],
+        ] {
+            assert_eq!(
+                read_query(bad).period,
+                Period::month(date("2026-10-14")),
+                "{bad:?}"
+            );
+        }
+    }
+
+    #[test]
+    fn each_filter_is_read_and_unusable_values_are_ignored() {
+        let selection = read_query(&[
+            ("area", "common"),
+            ("room", "3"),
+            ("room", "5"),
+            ("by", "2"),
+            ("time", "night"),
+            ("min", "5"),
+            ("clean", "hide"),
+        ]);
+        assert_eq!(
+            selection.filter,
+            Filter {
+                rooms: vec![RoomId(3), RoomId(5)],
+                area: Some(RoomArea::Common),
+                person: Some(UserId(2)),
+                day_part: Some(DayPart::Night),
+                min_level: 5,
+                hide_clean: true,
+            }
+        );
+        let ignored = read_query(&[
+            ("area", "garden"),
+            ("room", "x"),
+            ("by", "9"),
+            ("time", "noon"),
+            ("min", "11"),
+            ("clean", "yes"),
+        ]);
+        assert_eq!(ignored.filter, Filter::default());
+    }
+
+    #[test]
+    fn the_summary_names_every_filter_in_use() {
+        let tr = Tr(Locale::En);
+        let people = [user(1, "daniele")];
+        let selection = read(
+            &Pairs::of(&[
+                ("by", "1"),
+                ("time", "night"),
+                ("min", "5"),
+                ("area", "inside"),
+            ]),
+            date("2026-10-14"),
+            &people,
+        );
+        let view = view(tr, &selection, &[], Some(&people));
+        assert_eq!(
+            view.summary,
+            "This month · Inside · daniele · 22–06 · level 5+"
+        );
+        assert_eq!(view.from, "", "no typed dates while a quick pick is on");
+        let typed = read(
+            &Pairs::of(&[
+                ("from", "2026-01-01"),
+                ("to", "2026-01-31"),
+                ("clean", "hide"),
+            ]),
+            date("2026-10-14"),
+            &people,
+        );
+        let view = super::view(tr, &typed, &[], None);
+        assert_eq!(view.summary, "01/01/2026–31/01/2026 · no clean checks");
+        assert_eq!(
+            (view.from.as_str(), view.to.as_str()),
+            ("2026-01-01", "2026-01-31")
+        );
+        assert!(view.people.is_none());
+        assert_eq!(view.min_label, "any");
+    }
+}
diff --git a/crates/web/src/routes/graphs/mod.rs b/crates/web/src/routes/graphs/mod.rs
new file mode 100644
index 0000000..b1957b8
--- /dev/null
+++ b/crates/web/src/routes/graphs/mod.rs
@@ -0,0 +1,129 @@
+//! The Graphs screen (spec §9 screen 5, look-and-feel spec §7). The filters travel in the
+//! address ([`filters`]); each chart is a `<canvas>` whose Chart.js configuration travels as
+//! JSON in its `data-chart` attribute ([`charts`]), which `static/js/graphs.js` draws, so no
+//! page carries an inline script. The room × hour heatmap is a plain table, coloured by CSS
+//! classes. The numbers come from `smoke_domain::graphs`, over what the viewer may see: guests
+//! get graphs of their own data only (spec §4).
+
+mod charts;
+mod filters;
+
+use std::collections::HashMap;
+
+use askama::Template;
+use axum::extract::State;
+use axum::http::StatusCode;
+use axum::response::Response;
+use smoke_domain::graphs::{
+    Granularity, area_levels, events_per_bucket, intensity_lines, room_hours,
+};
+use smoke_domain::permissions::{Action, can, can_see};
+use smoke_domain::repos::Store;
+use smoke_domain::types::UserId;
+
+use self::charts::{
+    HeatRow, area_chart, bucket_labels, count_chart, heat_rows, intensity_chart, symptom_chart,
+};
+use self::filters::FiltersView;
+use crate::auth::CurrentUser;
+use crate::error::{AppError, page};
+use crate::forms::QueryPairs;
+use crate::header::{SignedIn, Tab};
+use crate::i18n::Tr;
+use crate::state::AppState;
+
+#[derive(Template)]
+#[template(path = "graphs.html")]
+struct GraphsPage {
+    tr: Tr,
+    signed_in: Option<SignedIn>,
+    filters: FiltersView,
+    /// Guests see only their own data; the page says so.
+    own_only: bool,
+    empty: bool,
+    intensity: String,
+    events_key: &'static str,
+    events: String,
+    hours: std::ops::Range<u8>,
+    heatmap: Vec<HeatRow>,
+    symptoms: Option<String>,
+    areas: String,
+}
+
+pub(crate) async fn show<S: Store + Clone + 'static>(
+    State(state): State<AppState<S>>,
+    CurrentUser(user): CurrentUser,
+    tr: Tr,
+    QueryPairs(query): QueryPairs,
+) -> Result<Response, AppError> {
+    let now = state.clock.now();
+    let schedule = &state.config.schedule;
+    let actor = user.actor();
+    let own_only = !can(&actor, Action::ViewAll, now);
+    let users = state.store.users().await?;
+    let people = (!own_only).then_some(users.as_slice());
+    let selection = filters::read(&query, schedule.local_date(now), people.unwrap_or_default());
+    let (period, filter) = (selection.period, &selection.filter);
+    let span = period.span(schedule);
+    let rooms = state.store.rooms().await?;
+    let symptoms = state.store.symptoms().await?;
+
+    let visible = state
+        .store
+        .reports_between(span.0, span.1)
+        .await?
+        .into_iter()
+        .filter(|r| can_see(&actor, r.user_id, now))
+        .collect();
+    let reports = filter.reports(visible, &rooms, schedule);
+    let visible = state
+        .store
+        .readings_between(span.0, span.1)
+        .await?
+        .into_iter()
+        .filter(|r| can_see(&actor, r.user_id, now))
+        .collect();
+    let readings = filter.readings(visible, schedule);
+    let names: HashMap<UserId, String> = users
+        .iter()
+        .map(|u| (u.id, u.username.to_string()))
+        .collect();
+
+    let labels = bucket_labels(tr, &period.buckets());
+    let events = events_per_bucket(&reports, period, schedule)
+        .into_iter()
+        .map(|(_, count)| count)
+        .collect();
+    let areas = area_levels(&reports, &rooms, period, schedule);
+    let lines = intensity_lines(&reports);
+    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule);
+    page(
+        StatusCode::OK,
+        &GraphsPage {
+            tr,
+            signed_in: SignedIn::of(&user, now, Tab::Graphs),
+            filters: filters::view(tr, &selection, &rooms, people),
+            own_only,
+            empty: reports.is_empty() && readings.is_empty(),
+            intensity: intensity.to_string(),
+            events_key: match period.granularity() {
+                Granularity::Day => "graphs-events-day",
+                Granularity::Week => "graphs-events-week",
+            },
+            events: count_chart(tr.t("graphs-events-label"), labels.clone(), events, false)
+                .to_string(),
+            hours: 0..24,
+            heatmap: heat_rows(tr, room_hours(&reports, schedule), &rooms),
+            symptoms: symptom_chart(tr, &reports, &symptoms).map(|chart| chart.to_string()),
+            areas: area_chart(tr, labels, &areas).to_string(),
+        },
+    )
+}
+
+/// Every key passed to the template through a variable, for the translation tests.
+#[cfg(test)]
+pub(crate) fn keys() -> Vec<&'static str> {
+    let mut keys = vec!["graphs-events-day", "graphs-events-week"];
+    keys.extend(filters::keys());
+    keys
+}
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
index 8dc6292..c347394 100644
--- a/crates/web/templates/graphs.html
+++ b/crates/web/templates/graphs.html
@@ -1,4 +1,5 @@
 {% extends "base.html" %}
+{% import "components.html" as c %}
 {% block title %}{{ tr.t("graphs-title") }}{% endblock %}
 {% block scripts %}
   <script src="/static/chart-4.5.1.umd.min.js" defer></script>
@@ -6,15 +7,55 @@
 {% endblock %}
 {% block content %}
 <h1>{{ tr.t("graphs-title") }}</h1>
-<nav class="months">
-  <a href="/graphs?{{ previous }}">{{ tr.t("events-previous") }}</a>
-  <strong>{{ label }}</strong>
-  <a href="/graphs?{{ next }}">{{ tr.t("events-next") }}</a>
-</nav>
-<p><a href="/graphs?{{ other }}">{{ tr.t(other_key) }}</a></p>
-{% if own_only %}
-<p class="hint">{{ tr.t("graphs-own-only") }}</p>
-{% endif %}
+{% call c::collapsible(tr.t("graphs-filters"), filters.summary, tr.t("action-change"), false) %}
+<form method="get" action="/graphs" class="stack filters">
+  <fieldset class="stack" data-date-picks>
+    <legend>{{ tr.t("graphs-dates") }}</legend>
+    <div class="cluster">
+      {% for pick in filters.ranges %}{% call c::chip("radio", "range", pick.value, pick.label, pick.checked) %}{% endcall %}{% endfor %}
+    </div>
+    <div class="split">
+      {% call c::field(tr.t("graphs-from")) %}<input type="date" name="from" value="{{ filters.from }}">{% endcall %}
+      {% call c::field(tr.t("graphs-to")) %}<input type="date" name="to" value="{{ filters.to }}">{% endcall %}
+    </div>
+  </fieldset>
+  <fieldset class="stack">
+    <legend>{{ tr.t("graphs-where") }}</legend>
+    <div class="segmented">
+      {% for pick in filters.areas %}{% call c::chip("radio", "area", pick.value, pick.label, pick.checked) %}{% endcall %}{% endfor %}
+    </div>
+    <div class="cluster">
+      {% for room in filters.rooms %}{% call c::chip("checkbox", "room", room.value, room.label, room.checked) %}{% endcall %}{% endfor %}
+    </div>
+  </fieldset>
+  {% if let Some(people) = filters.people %}
+  {% call c::field(tr.t("graphs-by")) %}
+  <select name="by">
+    <option value="">{{ tr.t("graphs-everyone") }}</option>
+    {% for person in people %}
+    <option value="{{ person.value }}" data-role="{{ person.role }}"{% if person.selected %} selected{% endif %}>{{ person.name }}</option>
+    {% endfor %}
+  </select>
+  {% endcall %}
+  {% endif %}
+  <fieldset class="stack">
+    <legend>{{ tr.t("graphs-time") }}</legend>
+    <div class="cluster">
+      {% for pick in filters.times %}{% call c::chip("radio", "time", pick.value, pick.label, pick.checked) %}{% endcall %}{% endfor %}
+    </div>
+  </fieldset>
+  <label class="field">
+    <span class="split">{{ tr.t("graphs-min-level") }} <output for="min-level">{{ filters.min_label }}</output></span>
+    <input type="range" id="min-level" name="min" min="0" max="10" step="1" value="{{ filters.min_level }}">
+  </label>
+  <label class="inline-field">{{ tr.t("graphs-hide-clean") }} <input type="checkbox" name="clean" value="hide"{% if filters.hide_clean %} checked{% endif %}></label>
+  <div class="actions">
+    <a href="/graphs" class="button" data-variant="secondary">{{ tr.t("graphs-reset") }}</a>
+    <button type="submit" class="button">{{ tr.t("graphs-show") }}</button>
+  </div>
+</form>
+{% endcall %}
+{% if own_only %}<p class="hint">{{ tr.t("graphs-own-only") }}</p>{% endif %}
 {% if empty %}
 <p>{{ tr.t("graphs-none") }}</p>
 {% else %}
diff --git a/crates/web/templates/log.html b/crates/web/templates/log.html
index fe69a5f..2628582 100644
--- a/crates/web/templates/log.html
+++ b/crates/web/templates/log.html
@@ -32,7 +32,7 @@
 {% if let Some(form) = form %}
 {% if let Some(error) = form.error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
 <form method="post" action="/log" class="stack">
-  {% call c::collapsible(tr.t("log-time"), form.time_hint(tr), tr.t("log-time-change"), !form.observed_at.is_empty()) %}
+  {% call c::collapsible(tr.t("log-time"), form.time_hint(tr), tr.t("action-change"), !form.observed_at.is_empty()) %}
   <input type="datetime-local" name="observed_at" value="{{ form.observed_at }}" aria-label="{{ tr.t("log-time") }}">
   {% endcall %}
   {% call f::rooms(form.fields, tr.t("log-rooms"), tr.t("log-how-to-rate"), tr.t("log-inside"), tr.t("log-common"), tr.t("log-level-hint")) %}{% endcall %}
diff --git a/crates/web/templates/settings.html b/crates/web/templates/settings.html
index 425c752..f7afcfd 100644
--- a/crates/web/templates/settings.html
+++ b/crates/web/templates/settings.html
@@ -25,7 +25,7 @@
 {% call c::card(tr.t("language")) %}
 {% call c::languages(tr.languages(), tr.t("language"), "settings") %}{% endcall %}
 {% endcall %}
-{% call c::collapsible(tr.t("settings-password"), "", tr.t("settings-change"), notices.password_error.is_some()) %}
+{% call c::collapsible(tr.t("settings-password"), "", tr.t("action-change"), notices.password_error.is_some()) %}
 {% if let Some(error) = notices.password_error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
 <form method="post" action="/settings/password" class="stack">
   <input type="text" name="username" value="{{ username }}" autocomplete="username" hidden>
@@ -37,7 +37,7 @@
 </form>
 {% endcall %}
 {% if let Some(lists) = lists %}
-{% call c::collapsible(tr.t("log-rooms"), "", tr.t("settings-change"), notices.list_error.is_some()) %}
+{% call c::collapsible(tr.t("log-rooms"), "", tr.t("action-change"), notices.list_error.is_some()) %}
 {% if let Some(error) = notices.list_error %}{% call c::notice(tr.t(error), true) %}{% endcall %}{% endif %}
 <p class="hint">{{ tr.t("settings-lists-hint") }}</p>
 <ul class="items">
@@ -55,7 +55,7 @@
   </fieldset>
 </form>
 {% endcall %}
-{% call c::collapsible(tr.t("log-symptoms"), "", tr.t("settings-change"), notices.list_error.is_some()) %}
+{% call c::collapsible(tr.t("log-symptoms"), "", tr.t("action-change"), notices.list_error.is_some()) %}
 <ul class="items">
   {% for item in lists.symptoms %}{% call list_item(item, tr.t("settings-list-hidden"), tr.t("settings-list-hide"), tr.t("settings-list-show")) %}{% endcall %}{% endfor %}
 </ul>
diff --git a/crates/web/tests/graphs.rs b/crates/web/tests/graphs.rs
index 59ffe90..39c56af 100644
--- a/crates/web/tests/graphs.rs
+++ b/crates/web/tests/graphs.rs
@@ -3,6 +3,7 @@
 mod common;
 
 use axum::http::StatusCode;
+use common::page::Page;
 use common::*;
 use serde_json::Value;
 use smoke_domain::logbook::EventChoice;
@@ -91,12 +92,12 @@ async fn household() -> (TestApp, String, String) {
 #[tokio::test]
 async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
     let (app, resident, _) = household().await;
-    let html = text(app.send(get_with("/graphs?month=2026-01", &resident)).await).await;
+    let html = text(app.send(get_with("/graphs", &resident)).await).await;
     assert!(
         html.contains(r#"<a href="/graphs" aria-current="page">"#),
         "{html}"
     );
-    assert!(html.contains("<strong>01/2026</strong>"), "{html}");
+    assert!(Page::parse(&html).exists("form.filters[method=get][action=\"/graphs\"]"));
     assert!(!html.contains("only your own"), "{html}");
     let intensity = &charts(&html)[0];
     assert_eq!(
@@ -127,7 +128,7 @@ async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
 #[tokio::test]
 async fn guests_see_graphs_of_their_own_reports_only() {
     let (app, _, guest) = household().await;
-    let html = text(app.send(get_with("/graphs?month=2026-01", &guest)).await).await;
+    let html = text(app.send(get_with("/graphs", &guest)).await).await;
     assert!(
         html.contains("These graphs show only your own reports and readings."),
         "{html}"
@@ -154,12 +155,12 @@ async fn guests_see_graphs_of_their_own_reports_only() {
 #[tokio::test]
 async fn a_month_counts_events_per_day() {
     let (app, resident, _) = household().await;
-    let html = text(app.send(get_with("/graphs?month=2026-01", &resident)).await).await;
+    let html = text(app.send(get_with("/graphs", &resident)).await).await;
     assert!(html.contains("<h2>Events per day</h2>"), "{html}");
     let events = &charts(&html)[1];
     let labels = events["data"]["labels"].as_array().unwrap();
     assert_eq!(labels.len(), 31);
-    assert_eq!(labels[0], "1");
+    assert_eq!(labels[0], "01/01");
     let counts = &events["data"]["datasets"][0]["data"];
     assert_eq!(counts[9], 1, "10 January: one event, two people");
     assert_eq!(counts[11], 1, "12 January");
@@ -175,16 +176,10 @@ async fn a_month_counts_events_per_day() {
 }
 
 #[tokio::test]
-async fn a_year_counts_events_per_week_and_links_back_to_a_month() {
+async fn this_year_counts_events_per_week() {
     let (app, resident, _) = household().await;
-    let html = text(app.send(get_with("/graphs?year=2026", &resident)).await).await;
-    assert!(html.contains("<strong>2026</strong>"), "{html}");
-    assert!(html.contains(r#"href="/graphs?year=2025""#), "{html}");
-    assert!(html.contains(r#"href="/graphs?year=2027""#), "{html}");
-    assert!(
-        html.contains(r#"<a href="/graphs?month=2026-01">Show one month</a>"#),
-        "{html}"
-    );
+    let html = text(app.send(get_with("/graphs?range=year", &resident)).await).await;
+    assert!(html.contains(r#"value="year" checked"#), "{html}");
     assert!(html.contains("<h2>Events per week</h2>"), "{html}");
     let events = &charts(&html)[1];
     let labels = events["data"]["labels"].as_array().unwrap();
@@ -204,7 +199,12 @@ async fn an_unusable_address_shows_this_month_and_an_empty_one_says_so() {
         let response = app.send(get_with(path, &resident)).await;
         assert_eq!(response.status(), StatusCode::OK, "{path}");
         let html = text(response).await;
-        assert!(html.contains("<strong>03/2026</strong>"), "{path}: {html}");
+        assert!(
+            Page::parse(&html)
+                .text("details.collapsible summary")
+                .contains("This month"),
+            "{path}: {html}"
+        );
         assert!(
             html.contains("Nothing was logged in this period."),
             "{path}: {html}"
@@ -253,7 +253,7 @@ async fn the_room_table_averages_each_rooms_level_by_local_hour() {
     app.no_smell(&me, "2026-01-11T09:00:00Z").await;
     let cookie = app.sign_in("daniele").await;
 
-    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
     assert!(html.contains("<h2>Rooms by hour of day</h2>"), "{html}");
     let living_row = heat_row("Living room", &[(19, 5)]);
     let bedroom_row = heat_row("Bedroom", &[(19, 0)]);
@@ -300,7 +300,7 @@ async fn symptoms_and_areas_are_compared() {
     .await;
     let cookie = app.sign_in("daniele").await;
 
-    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
     let charts = charts(&html);
     let symptoms = &charts[2];
     assert_eq!(symptoms["options"]["indexAxis"], "y");
@@ -327,7 +327,7 @@ async fn a_period_without_symptoms_or_rooms_says_so() {
     let me = app.add_user("daniele", Role::Resident, Locale::En).await;
     app.no_smell(&me, "2026-01-11T09:00:00Z").await;
     let cookie = app.sign_in("daniele").await;
-    let html = text(app.send(get_with("/graphs?month=2026-01", &cookie)).await).await;
+    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
     assert!(
         html.contains("No room was checked in this period."),
         "{html}"
@@ -338,3 +338,200 @@ async fn a_period_without_symptoms_or_rooms_says_so() {
     );
     assert_eq!(charts(&html).len(), 3, "intensity, events and areas");
 }
+
+/// What the filters leave: the intensity chart's line labels and the heatmap's rooms.
+async fn filtered(app: &TestApp, cookie: &str, query: &str) -> (Vec<String>, Vec<String>) {
+    let html = text(
+        app.send(get_with(&format!("/graphs?{query}"), cookie))
+            .await,
+    )
+    .await;
+    let lines = charts(&html)
+        .first()
+        .map(|chart| labels(chart).into_iter().map(str::to_owned).collect())
+        .unwrap_or_default();
+    (lines, Page::parse(&html).texts(".heatmap tbody th"))
+}
+
+/// A resident, a partner and a viewer; reports in three rooms at three times of day.
+async fn rooms_and_times() -> (TestApp, String, Vec<(&'static str, i64)>) {
+    let app = TestApp::new().await;
+    app.at("2026-01-20T12:00:00Z");
+    let me = app.add_user("daniele", Role::Resident, Locale::En).await;
+    let partner = app.add_user("partner", Role::Resident, Locale::En).await;
+    app.add_user("lawyer", Role::Viewer, Locale::En).await;
+    let bedroom = app.room("room-bedroom").await;
+    let stairwell = app.room("room-stairwell").await;
+    let kitchen = app.room("room-kitchen").await;
+    let at = |room, level| RoomLevel {
+        room,
+        level: Level::new(level).unwrap(),
+    };
+    // 23:00 in Rome: night.
+    let night = app
+        .log_values(
+            &me,
+            "2026-01-10T22:00:00Z",
+            vec![at(bedroom, 6)],
+            &[],
+            EventChoice::New { replacing: None },
+        )
+        .await;
+    let join = EventChoice::Join(night.report.event_id.unwrap());
+    // 23:30: the partner, on the stairwell.
+    app.log_values(
+        &partner,
+        "2026-01-10T22:30:00Z",
+        vec![at(stairwell, 3)],
+        &[],
+        join,
+    )
+    .await;
+    // 09:00 on the 12th: morning, the kitchen.
+    app.log_values(
+        &me,
+        "2026-01-12T08:00:00Z",
+        vec![at(kitchen, 4)],
+        &[],
+        EventChoice::New { replacing: None },
+    )
+    .await;
+    // 15:00 on the 13th: a clean check of the whole home.
+    app.no_smell(&me, "2026-01-13T14:00:00Z").await;
+    let cookie = app.sign_in("daniele").await;
+    let ids = vec![
+        ("bedroom", bedroom.0),
+        ("stairwell", stairwell.0),
+        ("kitchen", kitchen.0),
+        ("partner", partner.id.0),
+    ];
+    (app, cookie, ids)
+}
+
+#[tokio::test]
+async fn each_filter_narrows_the_graphs() {
+    let (app, cookie, ids) = rooms_and_times().await;
+    let id = |name| ids.iter().find(|(n, _)| *n == name).unwrap().1;
+    let strings = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
+
+    let (lines, rooms) = filtered(&app, &cookie, "").await;
+    assert_eq!(lines, strings(&["daniele", "partner"]));
+    assert_eq!(
+        rooms,
+        strings(&["Kitchen", "Bedroom", "Stairwell (shared)"])
+    );
+
+    let dates = "from=2026-01-12&to=2026-01-31";
+    assert_eq!(
+        filtered(&app, &cookie, dates).await,
+        (strings(&["daniele"]), strings(&["Kitchen"]))
+    );
+
+    let room = format!("room={}&area=common", id("bedroom"));
+    assert_eq!(
+        filtered(&app, &cookie, &room).await,
+        (strings(&["daniele"]), strings(&["Bedroom"])),
+        "rooms win over the area"
+    );
+    assert_eq!(
+        filtered(&app, &cookie, "area=common").await,
+        (strings(&["partner"]), strings(&["Stairwell (shared)"]))
+    );
+
+    let person = format!("by={}", id("partner"));
+    assert_eq!(
+        filtered(&app, &cookie, &person).await.0,
+        strings(&["partner"])
+    );
+
+    assert_eq!(
+        filtered(&app, &cookie, "time=morning").await.1,
+        strings(&["Kitchen"])
+    );
+    assert_eq!(
+        filtered(&app, &cookie, "time=night").await.1,
+        strings(&["Bedroom", "Stairwell (shared)"])
+    );
+
+    assert_eq!(
+        filtered(&app, &cookie, "min=5").await,
+        (strings(&["daniele"]), strings(&["Bedroom"]))
+    );
+
+    let html = text(app.send(get_with("/graphs?clean=hide", &cookie)).await).await;
+    let points = charts(&html)[0]["data"]["datasets"][0]["data"]
+        .as_array()
+        .unwrap()
+        .len();
+    assert_eq!(points, 2, "daniele's two reports without the clean check");
+    let html = text(app.send(get_with("/graphs", &cookie)).await).await;
+    let points = charts(&html)[0]["data"]["datasets"][0]["data"]
+        .as_array()
+        .unwrap()
+        .len();
+    assert_eq!(points, 3);
+}
+
+#[tokio::test]
+async fn the_filters_card_shows_what_is_chosen() {
+    let (app, cookie, ids) = rooms_and_times().await;
+    let bedroom = ids[0].1;
+    let path = format!(
+        "/graphs?room={bedroom}&by={}&time=night&min=5&clean=hide",
+        ids[3].1
+    );
+    let page = Page::parse(&text(app.send(get_with(&path, &cookie)).await).await);
+    assert_eq!(
+        page.text("details.collapsible summary"),
+        "Filters · This month · Bedroom · partner · 22–06 · level 5+ · no clean checks Change"
+    );
+    assert!(page.exists(&format!("input[name=room][value='{bedroom}'][checked]")));
+    assert_eq!(page.text("select[name=by] option[selected]"), "partner");
+    assert!(page.exists("input[name=time][value=night][checked]"));
+    assert_eq!(
+        page.attr("input[type=range][name=min]", "value").as_deref(),
+        Some("5")
+    );
+    assert_eq!(page.text("output[for=min-level]"), "5+");
+    assert!(page.exists("input[name=clean][value=hide][checked]"));
+    assert_eq!(
+        page.attr("a.button[data-variant=secondary]", "href")
+            .as_deref(),
+        Some("/graphs")
+    );
+}
+
+#[tokio::test]
+async fn a_viewer_sees_everyone_and_may_choose_a_person() {
+    let (app, _, _) = rooms_and_times().await;
+    let viewer = app.sign_in("lawyer").await;
+    let html = text(app.send(get_with("/graphs", &viewer)).await).await;
+    assert!(!html.contains("only your own"), "{html}");
+    let charts = charts(&html);
+    assert_eq!(labels(&charts[0]), ["daniele", "partner"]);
+    assert_eq!(
+        Page::parse(&html).texts("select[name=by] option"),
+        ["Everyone", "daniele", "lawyer", "partner"]
+    );
+}
+
+#[tokio::test]
+async fn guests_get_no_person_filter_and_cannot_pick_someone_else() {
+    let (app, _, guest) = household().await;
+    let html = text(app.send(get_with("/graphs?by=1", &guest)).await).await;
+    assert!(!Page::parse(&html).exists("select[name=by]"), "{html}");
+    assert_eq!(labels(&charts(&html)[0]), ["guest", "PM2.5 (µg/m³)"]);
+    assert!(!html.contains("daniele"), "{html}");
+}
+
+#[tokio::test]
+async fn a_repeated_filter_takes_the_first_value() {
+    let (app, cookie, _) = rooms_and_times().await;
+    let response = app
+        .send(get_with("/graphs?range=7d&range=year&min=4&min=9", &cookie))
+        .await;
+    assert_eq!(response.status(), StatusCode::OK);
+    let page = Page::parse(&text(response).await);
+    assert!(page.exists("input[name=range][value='7d'][checked]"));
+    assert_eq!(page.attr("input[name=min]", "value").as_deref(), Some("4"));
+}
````

### Task 17: Graphs look: token colours, series switch, Everyone line, person search

**Proto commit:** `b3c97d0` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show b3c97d0`)

Spec §7, §9 (charts, custom element). Datasets carry `token` and `group`; an Everyone line (hidden) for those who see everyone while no person is chosen; `intensity_chart` derives the span from the period (seven arguments). `js/graphs.js` resolves tokens and re-reads them on a theme change, hides legend items of hidden datasets, and isolates each chart in `try/catch` (M6 carry-over); `js/series-switch.js`, `js/person-filter.js` (`<person-filter>`), `js/range-output.js`; the `series` macro; cards and heatmap CSS (73). Keys `graphs-series-*`, `graphs-lines-person`, `graphs-search-people`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	crates/web/css/73-chart.css
A	crates/web/static/js/person-filter.js
A	crates/web/static/js/range-output.js
A	crates/web/static/js/series-switch.js
M	crates/web/locales/en/main.ftl
M	crates/web/locales/it/main.ftl
M	crates/web/locales/tr/main.ftl
M	crates/web/src/assets.rs
M	crates/web/src/routes/graphs/charts.rs
M	crates/web/src/routes/graphs/mod.rs
M	crates/web/static/js/app.js
M	crates/web/static/js/graphs.js
M	crates/web/static/sw.js
M	crates/web/templates/components.html
M	crates/web/templates/graphs.html
M	crates/web/tests/graphs.rs
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Write the tests first.** Apply the test hunks of the diff below (`crates/web/tests/graphs.rs`). Run `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target cargo test -p smoke-web` (or `-p smoke-domain`): the new tests fail or do not compile.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **634 passed** (SQLite). Look at `/graphs` in a browser in both themes (the charts need JavaScript).
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
feat(graphs): token colours, a series switch, an Everyone line and a searchable person filter

Chart datasets name a colour token (accent, series-2..4, amber) and a group; js/graphs.js
reads the tokens from the stylesheet and again when the system theme changes, and skips a
chart whose config fails (M6 carry-over). js/series-switch.js shows Both, Intensity or PM2.5
and By person or Everyone, a line the server adds for those who see everyone while no person
is chosen. <person-filter> turns the select into a sheet with a search box; js/range-output.js
labels the minimum level. The charts and the heatmap sit in cards; heat cells use the tokens'
ramp.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `b3c97d0`):

````diff
diff --git a/crates/web/css/73-chart.css b/crates/web/css/73-chart.css
new file mode 100644
index 0000000..b1521db
--- /dev/null
+++ b/crates/web/css/73-chart.css
@@ -0,0 +1,74 @@
+/* The Graphs screen: the filter form, the charts' frames and the room × hour table, whose
+   cells take the heatmap ramp of the tokens (dark text on 0–5, light on 6–10). */
+@layer components {
+  .filters legend {
+    margin-block-end: var(--space-2);
+  }
+
+  .filters .split > * {
+    flex: 1 1 0;
+    min-inline-size: 0;
+  }
+
+  .filters input[type="range"] {
+    inline-size: 100%;
+    min-block-size: var(--target);
+  }
+
+  person-filter {
+    display: block;
+  }
+
+  person-filter > .button {
+    inline-size: 100%;
+    justify-content: space-between;
+  }
+
+  .chart {
+    position: relative;
+    block-size: 16rem;
+  }
+
+  .scroll {
+    overflow-x: auto;
+  }
+
+  .heatmap {
+    inline-size: auto;
+    font-size: var(--text-tiny);
+    font-variant-numeric: tabular-nums;
+  }
+
+  .heatmap th,
+  .heatmap td {
+    min-inline-size: 1.5rem;
+    padding: var(--space-1) 0.125rem;
+    text-align: center;
+  }
+
+  .heatmap tbody th {
+    padding-inline-end: var(--space-2);
+    text-align: start;
+    white-space: nowrap;
+  }
+
+  .heat-0 { background: var(--heat-0); }
+  .heat-1 { background: var(--heat-1); }
+  .heat-2 { background: var(--heat-2); }
+  .heat-3 { background: var(--heat-3); }
+  .heat-4 { background: var(--heat-4); }
+  .heat-5 { background: var(--heat-5); }
+  .heat-6 { background: var(--heat-6); }
+  .heat-7 { background: var(--heat-7); }
+  .heat-8 { background: var(--heat-8); }
+  .heat-9 { background: var(--heat-9); }
+  .heat-10 { background: var(--heat-10); }
+
+  .heat-6,
+  .heat-7,
+  .heat-8,
+  .heat-9,
+  .heat-10 {
+    color: var(--on-accent);
+  }
+}
diff --git a/crates/web/locales/en/main.ftl b/crates/web/locales/en/main.ftl
index 7960dc1..e88c323 100644
--- a/crates/web/locales/en/main.ftl
+++ b/crates/web/locales/en/main.ftl
@@ -321,6 +321,13 @@ graphs-none = Nothing was logged in this period.
 graphs-needs-js = This graph needs JavaScript.
 graphs-intensity = Smell intensity over time
 graphs-pm25 = PM2.5 (µg/m³)
+graphs-series-both = Both
+graphs-series-intensity = Intensity
+graphs-series-pm25 = PM2.5
+graphs-series-both-title = Smell intensity and PM2.5
+graphs-series-pm25-title = PM2.5 over time
+graphs-lines-person = By person
+graphs-search-people = Search people
 graphs-events-day = Events per day
 graphs-events-week = Events per week
 graphs-events-label = Events
diff --git a/crates/web/locales/it/main.ftl b/crates/web/locales/it/main.ftl
index 505e8e9..c8bed50 100644
--- a/crates/web/locales/it/main.ftl
+++ b/crates/web/locales/it/main.ftl
@@ -321,6 +321,13 @@ graphs-none = Nessuna registrazione in questo periodo.
 graphs-needs-js = Questo grafico richiede JavaScript.
 graphs-intensity = Intensità dell'odore nel tempo
 graphs-pm25 = PM2.5 (µg/m³)
+graphs-series-both = Entrambi
+graphs-series-intensity = Intensità
+graphs-series-pm25 = PM2.5
+graphs-series-both-title = Intensità dell'odore e PM2.5
+graphs-series-pm25-title = PM2.5 nel tempo
+graphs-lines-person = Per persona
+graphs-search-people = Cerca persone
 graphs-events-day = Eventi al giorno
 graphs-events-week = Eventi alla settimana
 graphs-events-label = Eventi
diff --git a/crates/web/locales/tr/main.ftl b/crates/web/locales/tr/main.ftl
index f2ef07b..9b9fbf4 100644
--- a/crates/web/locales/tr/main.ftl
+++ b/crates/web/locales/tr/main.ftl
@@ -321,6 +321,13 @@ graphs-none = Bu dönemde hiçbir kayıt yok.
 graphs-needs-js = Bu grafik için JavaScript gerekir.
 graphs-intensity = Zaman içinde koku yoğunluğu
 graphs-pm25 = PM2.5 (µg/m³)
+graphs-series-both = İkisi
+graphs-series-intensity = Yoğunluk
+graphs-series-pm25 = PM2.5
+graphs-series-both-title = Koku yoğunluğu ve PM2.5
+graphs-series-pm25-title = Zaman içinde PM2.5
+graphs-lines-person = Kişiye göre
+graphs-search-people = Kişi ara
 graphs-events-day = Günlük olay sayısı
 graphs-events-week = Haftalık olay sayısı
 graphs-events-label = Olaylar
diff --git a/crates/web/src/assets.rs b/crates/web/src/assets.rs
index c76a24c..2199207 100644
--- a/crates/web/src/assets.rs
+++ b/crates/web/src/assets.rs
@@ -65,6 +65,7 @@ const STYLESHEET: (&str, &[&str]) = stylesheet!(
     "70-items.css",
     "71-people.css",
     "72-guide.css",
+    "73-chart.css",
     "90-utilities.css",
 );
 
@@ -88,6 +89,9 @@ const ASSETS: &[Asset] = &[
     asset!("js/graphs.js", "text/javascript; charset=utf-8"),
     asset!("js/install.js", "text/javascript; charset=utf-8"),
     asset!("js/menu.js", "text/javascript; charset=utf-8"),
+    asset!("js/person-filter.js", "text/javascript; charset=utf-8"),
+    asset!("js/range-output.js", "text/javascript; charset=utf-8"),
+    asset!("js/series-switch.js", "text/javascript; charset=utf-8"),
     asset!("js/service-worker.js", "text/javascript; charset=utf-8"),
     asset!("manifest.webmanifest", "application/manifest+json"),
     asset!("sw.js", "text/javascript; charset=utf-8"),
diff --git a/crates/web/src/routes/graphs/charts.rs b/crates/web/src/routes/graphs/charts.rs
index 6bab67c..1e23779 100644
--- a/crates/web/src/routes/graphs/charts.rs
+++ b/crates/web/src/routes/graphs/charts.rs
@@ -10,7 +10,7 @@ use smoke_domain::graphs::{
 };
 use smoke_domain::records::{Pm25Reading, Room, SmellReport, Symptom};
 use smoke_domain::schedule::DailySchedule;
-use smoke_domain::types::{Locale, UserId};
+use smoke_domain::types::{Level, Locale, UserId};
 
 use crate::i18n::Tr;
 use crate::{labels, time};
@@ -47,32 +47,60 @@ fn tick_values(period: Period, schedule: &DailySchedule) -> Vec<i64> {
         .collect()
 }
 
-/// Intensity over time: one line per person on the 0–10 axis, PM2.5 readings on their own axis
-/// on the right (spec §9). The x axis is milliseconds since the epoch.
+/// The colour tokens of the people's lines, in turn (`static/js/graphs.js` reads each token's
+/// value from the stylesheet, so dark mode and the palette live in one place).
+const PERSON_TOKENS: [&str; 4] = ["accent", "series-2", "series-3", "series-4"];
+
+fn points<'a>(levels: impl Iterator<Item = &'a (DateTime<Utc>, Level)>) -> Vec<Value> {
+    levels
+        .map(|(at, level)| json!({ "x": at.timestamp_millis(), "y": level.get() }))
+        .collect()
+}
+
+/// Intensity over time on the 0–10 axis and PM2.5 readings on their own axis on the right
+/// (spec §9). Each dataset names its `group`: one line per person ("person"), the same reports
+/// as one line ("everyone", hidden until the switch shows it, and left out when `everyone` is
+/// false because a person is chosen) and the readings ("pm25"). The x axis is milliseconds since
+/// the epoch.
 pub(crate) fn intensity_chart(
     tr: Tr,
     lines: &[PersonLine],
     names: &HashMap<UserId, String>,
     readings: &[Pm25Reading],
+    everyone: bool,
     period: Period,
-    (from, to): (DateTime<Utc>, DateTime<Utc>),
     schedule: &DailySchedule,
 ) -> Value {
+    let (from, to) = period.span(schedule);
     let mut datasets: Vec<Value> = lines
         .iter()
-        .map(|line| {
-            let points: Vec<Value> = line
-                .points
-                .iter()
-                .map(|(at, level)| json!({ "x": at.timestamp_millis(), "y": level.get() }))
-                .collect();
+        .zip(PERSON_TOKENS.iter().cycle())
+        .map(|(line, token)| {
             json!({
                 "label": names.get(&line.user).map_or("", String::as_str),
-                "data": points,
+                "data": points(line.points.iter()),
                 "yAxisID": "level",
+                "group": "person",
+                "token": token,
             })
         })
         .collect();
+    if everyone && !lines.is_empty() {
+        let mut all: Vec<_> = lines
+            .iter()
+            .flat_map(|line| &line.points)
+            .copied()
+            .collect();
+        all.sort_by_key(|&(at, _)| at);
+        datasets.push(json!({
+            "label": tr.t("graphs-everyone"),
+            "data": points(all.iter()),
+            "yAxisID": "level",
+            "group": "everyone",
+            "token": "accent",
+            "hidden": true,
+        }));
+    }
     if !readings.is_empty() {
         let points: Vec<Value> = readings
             .iter()
@@ -83,6 +111,8 @@ pub(crate) fn intensity_chart(
             "data": points,
             "yAxisID": "pm25",
             "borderDash": [6, 4],
+            "group": "pm25",
+            "token": "amber",
         }));
     }
     json!({
@@ -141,7 +171,7 @@ pub(crate) fn count_chart(
         "type": "bar",
         "data": {
             "labels": labels,
-            "datasets": [{ "label": label, "data": counts }],
+            "datasets": [{ "label": label, "data": counts, "token": "accent" }],
         },
         "options": {
             "indexAxis": index,
@@ -186,8 +216,16 @@ pub(crate) fn area_chart(tr: Tr, labels: Vec<String>, levels: &[(NaiveDate, Area
         "data": {
             "labels": labels,
             "datasets": [
-                { "label": tr.t("log-inside"), "data": series(|l| l.inside.map(|v| v.get())) },
-                { "label": tr.t("log-common"), "data": series(|l| l.common.map(|v| v.get())) },
+                {
+                    "label": tr.t("log-inside"),
+                    "data": series(|l| l.inside.map(|v| v.get())),
+                    "token": "accent",
+                },
+                {
+                    "label": tr.t("log-common"),
+                    "data": series(|l| l.common.map(|v| v.get())),
+                    "token": "amber",
+                },
             ],
         },
         "options": {
diff --git a/crates/web/src/routes/graphs/mod.rs b/crates/web/src/routes/graphs/mod.rs
index b1957b8..25fd37f 100644
--- a/crates/web/src/routes/graphs/mod.rs
+++ b/crates/web/src/routes/graphs/mod.rs
@@ -42,6 +42,8 @@ struct GraphsPage {
     own_only: bool,
     empty: bool,
     intensity: String,
+    /// The By person | Everyone switch: for those who see everyone, while no person is chosen.
+    everyone: bool,
     events_key: &'static str,
     events: String,
     hours: std::ops::Range<u8>,
@@ -96,7 +98,8 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
         .collect();
     let areas = area_levels(&reports, &rooms, period, schedule);
     let lines = intensity_lines(&reports);
-    let intensity = intensity_chart(tr, &lines, &names, &readings, period, span, schedule);
+    let everyone = !own_only && filter.person.is_none();
+    let intensity = intensity_chart(tr, &lines, &names, &readings, everyone, period, schedule);
     page(
         StatusCode::OK,
         &GraphsPage {
@@ -106,6 +109,7 @@ pub(crate) async fn show<S: Store + Clone + 'static>(
             own_only,
             empty: reports.is_empty() && readings.is_empty(),
             intensity: intensity.to_string(),
+            everyone,
             events_key: match period.granularity() {
                 Granularity::Day => "graphs-events-day",
                 Granularity::Week => "graphs-events-week",
diff --git a/crates/web/static/js/app.js b/crates/web/static/js/app.js
index d448e66..3ec8aff 100644
--- a/crates/web/static/js/app.js
+++ b/crates/web/static/js/app.js
@@ -4,4 +4,5 @@ import "./dialog.js";
 import "./date-picks.js";
 import "./install.js";
 import "./menu.js";
+import "./range-output.js";
 import "./service-worker.js";
diff --git a/crates/web/static/js/graphs.js b/crates/web/static/js/graphs.js
index efebcab..a1e199f 100644
--- a/crates/web/static/js/graphs.js
+++ b/crates/web/static/js/graphs.js
@@ -1,9 +1,31 @@
 // Draws the charts of the Graphs screen (spec §9 screen 5) with the vendored Chart.js. The
-// server writes each chart's configuration as JSON into its canvas's data-chart attribute; this
-// file adds only what JSON cannot carry: labels for the time axis, written in the app's time
-// zone (decision #95) rather than the browser's.
+// server writes each chart's configuration as JSON into its canvas's data-chart attribute;
+// this file adds only what JSON cannot carry: the colours, read from the stylesheet's tokens
+// (a dataset names its token) and read again when the system switches between light and dark,
+// and labels for the time axis, written in the app's time zone (decision #95).
+import "./series-switch.js";
+import "./person-filter.js";
+
 Chart.defaults.animation = false;
 Chart.defaults.maintainAspectRatio = false;
+Chart.defaults.plugins.legend.labels.filter = (item) => !item.hidden;
+
+const dark = matchMedia("(prefers-color-scheme: dark)");
+
+function token(name) {
+  return getComputedStyle(document.documentElement).getPropertyValue(`--${name}`).trim();
+}
+
+function paint(config) {
+  Chart.defaults.color = token("muted");
+  Chart.defaults.borderColor = token("line");
+  for (const dataset of config.data.datasets) {
+    if (dataset.token) {
+      dataset.borderColor = token(dataset.token);
+      dataset.backgroundColor = token(dataset.token);
+    }
+  }
+}
 
 function formatter(time, withClock) {
   const clock = withClock ? { hour: "2-digit", minute: "2-digit", hourCycle: "h23" } : {};
@@ -15,22 +37,39 @@ function formatter(time, withClock) {
   });
 }
 
+function withTimeAxis(config, time) {
+  const day = formatter(time, false);
+  const moment = formatter(time, true);
+  const x = config.options.scales.x;
+  x.ticks = { ...x.ticks, callback: (value) => day.format(value) };
+  x.afterBuildTicks = (scale) => {
+    scale.ticks = time.ticks.map((value) => ({ value }));
+  };
+  config.options.plugins = {
+    ...config.options.plugins,
+    tooltip: { callbacks: { title: (items) => moment.format(items[0].parsed.x) } },
+  };
+}
+
+const charts = [];
 for (const canvas of document.querySelectorAll("canvas[data-chart]")) {
-  const config = JSON.parse(canvas.dataset.chart);
-  const time = config.time;
-  delete config.time;
-  if (time) {
-    const day = formatter(time, false);
-    const moment = formatter(time, true);
-    const x = config.options.scales.x;
-    x.ticks = { ...x.ticks, callback: (value) => day.format(value) };
-    x.afterBuildTicks = (scale) => {
-      scale.ticks = time.ticks.map((value) => ({ value }));
-    };
-    config.options.plugins = {
-      ...config.options.plugins,
-      tooltip: { callbacks: { title: (items) => moment.format(items[0].parsed.x) } },
-    };
+  try {
+    const config = JSON.parse(canvas.dataset.chart);
+    const time = config.time;
+    delete config.time;
+    if (time) {
+      withTimeAxis(config, time);
+    }
+    paint(config);
+    charts.push(new Chart(canvas, config));
+  } catch (error) {
+    console.error(error);
   }
-  new Chart(canvas, config);
 }
+
+dark.addEventListener("change", () => {
+  for (const chart of charts) {
+    paint(chart.config);
+    chart.update("none");
+  }
+});
diff --git a/crates/web/static/js/person-filter.js b/crates/web/static/js/person-filter.js
new file mode 100644
index 0000000..aa7075f
--- /dev/null
+++ b/crates/web/static/js/person-filter.js
@@ -0,0 +1,73 @@
+// <person-filter> wraps the "Reported by" <select>. With JavaScript the select gives way to a
+// button that opens a sheet with a search box and everyone as radio buttons (name and role);
+// picking someone sets the select. Without JavaScript the select works on its own.
+class PersonFilter extends HTMLElement {
+  connectedCallback() {
+    const select = this.querySelector("select");
+    if (!select || this.dataset.ready) {
+      return;
+    }
+    this.dataset.ready = "";
+    const button = document.createElement("button");
+    button.type = "button";
+    button.className = "button";
+    button.dataset.variant = "secondary";
+    button.textContent = select.selectedOptions[0]?.text ?? "";
+    const dialog = this.sheet(select, button);
+    button.addEventListener("click", () => dialog.showModal());
+    select.hidden = true;
+    this.append(button);
+    document.body.append(dialog);
+  }
+
+  sheet(select, button) {
+    const dialog = document.createElement("dialog");
+    dialog.className = "sheet";
+    dialog.setAttribute("aria-label", this.closest("label")?.firstElementChild?.textContent ?? "");
+    const close = document.createElement("form");
+    close.method = "dialog";
+    const closer = document.createElement("button");
+    closer.className = "button";
+    closer.dataset.variant = "quiet";
+    closer.textContent = this.dataset.close ?? "";
+    close.append(closer);
+    const search = document.createElement("input");
+    search.type = "search";
+    search.placeholder = this.dataset.search ?? "";
+    search.setAttribute("aria-label", search.placeholder);
+    const list = document.createElement("div");
+    list.className = "rows";
+    for (const option of select.options) {
+      const label = document.createElement("label");
+      label.className = "row";
+      const radio = document.createElement("input");
+      radio.type = "radio";
+      radio.name = "person-filter";
+      radio.value = option.value;
+      radio.checked = option.selected;
+      const name = document.createElement("span");
+      name.className = "row-text";
+      name.textContent = option.text;
+      const role = document.createElement("span");
+      role.className = "muted";
+      role.textContent = option.dataset.role ?? "";
+      label.append(radio, name, role);
+      list.append(label);
+    }
+    search.addEventListener("input", () => {
+      const query = search.value.trim().toLowerCase();
+      for (const row of list.children) {
+        row.hidden = !row.textContent.toLowerCase().includes(query);
+      }
+    });
+    list.addEventListener("change", (event) => {
+      select.value = event.target.value;
+      button.textContent = select.selectedOptions[0]?.text ?? "";
+      dialog.close();
+    });
+    dialog.append(close, search, list);
+    return dialog;
+  }
+}
+
+customElements.define("person-filter", PersonFilter);
diff --git a/crates/web/static/js/range-output.js b/crates/web/static/js/range-output.js
new file mode 100644
index 0000000..a2edea7
--- /dev/null
+++ b/crates/web/static/js/range-output.js
@@ -0,0 +1,14 @@
+// A range input with data-output="<id>" writes its value into that <output> as it moves: its
+// data-zero text for 0 ("any"), else "5+". Without JavaScript the output shows the value the
+// page was loaded with.
+document.addEventListener("input", (event) => {
+  const range = event.target;
+  if (!(range instanceof HTMLInputElement) || !range.dataset.output) {
+    return;
+  }
+  const output = document.getElementById(range.dataset.output);
+  if (output) {
+    output.value = range.value === "0" ? (range.dataset.zero ?? "0") : `${range.value}+`;
+    range.setAttribute("aria-valuetext", output.value);
+  }
+});
diff --git a/crates/web/static/js/series-switch.js b/crates/web/static/js/series-switch.js
new file mode 100644
index 0000000..3afe0c9
--- /dev/null
+++ b/crates/web/static/js/series-switch.js
@@ -0,0 +1,41 @@
+// The switches above the intensity chart: Both, Intensity or PM2.5, and By person or Everyone.
+// Each dataset names its group ("person", "everyone", "pm25"); a switch shows the groups it
+// asks for and sets the chart's heading. Without JavaScript the switches stay hidden.
+const visible = {
+  both: ["lines", "pm25"],
+  intensity: ["lines"],
+  pm25: ["pm25"],
+};
+
+function apply(card) {
+  const chart = Chart.getChart(card.querySelector("canvas"));
+  if (!chart) {
+    return;
+  }
+  const series = card.querySelector('input[name="series"]:checked');
+  const lines = card.querySelector('input[name="lines"]:checked')?.value ?? "person";
+  const shown = visible[series?.value ?? "both"];
+  for (const dataset of chart.data.datasets) {
+    const group = dataset.group === "pm25" ? "pm25" : "lines";
+    const line = dataset.group === "pm25" || dataset.group === lines;
+    dataset.hidden = !(shown.includes(group) && line);
+  }
+  if (series?.dataset.title) {
+    card.querySelector("[data-series-title]").textContent = series.dataset.title;
+    chart.canvas.setAttribute("aria-label", series.dataset.title);
+  }
+  chart.update("none");
+}
+
+for (const card of document.querySelectorAll("[data-series-chart]")) {
+  for (const control of card.querySelectorAll("[data-series], [data-lines]")) {
+    control.hidden = false;
+  }
+}
+
+document.addEventListener("change", (event) => {
+  const card = event.target.closest?.("[data-series-chart]");
+  if (card) {
+    apply(card);
+  }
+});
diff --git a/crates/web/static/sw.js b/crates/web/static/sw.js
index c33902e..e38fb2e 100644
--- a/crates/web/static/sw.js
+++ b/crates/web/static/sw.js
@@ -12,6 +12,7 @@ const PRECACHE = [
   "/static/js/dialog.js",
   "/static/js/install.js",
   "/static/js/menu.js",
+  "/static/js/range-output.js",
   "/static/js/service-worker.js",
   "/static/htmx-2.0.11.min.js",
   "/static/favicon.svg",
diff --git a/crates/web/templates/components.html b/crates/web/templates/components.html
index 81316bd..b865d19 100644
--- a/crates/web/templates/components.html
+++ b/crates/web/templates/components.html
@@ -102,3 +102,9 @@
   <input type="date" name="{{ fields.date_name }}" value="{{ fields.date }}" aria-label="{{ date_label }}">
 </fieldset>
 {% endmacro %}
+
+{# One choice of a client-side switch: a radio that needs no form (js/series-switch.js reads it).
+   `title` is the chart heading it sets, if any. #}
+{% macro series(name, value, label, title, checked) %}
+<label class="chip"><input type="radio" name="{{ name }}" value="{{ value }}"{% if !title.is_empty() %} data-title="{{ title }}"{% endif %}{% if checked %} checked{% endif %}><span>{{ label }}</span></label>
+{% endmacro %}
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
index c347394..1aa5bda 100644
--- a/crates/web/templates/graphs.html
+++ b/crates/web/templates/graphs.html
@@ -30,12 +30,14 @@
   </fieldset>
   {% if let Some(people) = filters.people %}
   {% call c::field(tr.t("graphs-by")) %}
+  <person-filter data-search="{{ tr.t("graphs-search-people") }}" data-close="{{ tr.t("close") }}">
   <select name="by">
     <option value="">{{ tr.t("graphs-everyone") }}</option>
     {% for person in people %}
     <option value="{{ person.value }}" data-role="{{ person.role }}"{% if person.selected %} selected{% endif %}>{{ person.name }}</option>
     {% endfor %}
   </select>
+  </person-filter>
   {% endcall %}
   {% endif %}
   <fieldset class="stack">
@@ -45,8 +47,8 @@
     </div>
   </fieldset>
   <label class="field">
-    <span class="split">{{ tr.t("graphs-min-level") }} <output for="min-level">{{ filters.min_label }}</output></span>
-    <input type="range" id="min-level" name="min" min="0" max="10" step="1" value="{{ filters.min_level }}">
+    <span class="split">{{ tr.t("graphs-min-level") }} <output id="min-level-value" for="min-level">{{ filters.min_label }}</output></span>
+    <input type="range" id="min-level" name="min" min="0" max="10" step="1" value="{{ filters.min_level }}" data-output="min-level-value" data-zero="{{ tr.t("graphs-level-any") }}">
   </label>
   <label class="inline-field">{{ tr.t("graphs-hide-clean") }} <input type="checkbox" name="clean" value="hide"{% if filters.hide_clean %} checked{% endif %}></label>
   <div class="actions">
@@ -57,21 +59,32 @@
 {% endcall %}
 {% if own_only %}<p class="hint">{{ tr.t("graphs-own-only") }}</p>{% endif %}
 {% if empty %}
-<p>{{ tr.t("graphs-none") }}</p>
+<p class="card muted">{{ tr.t("graphs-none") }}</p>
 {% else %}
-<section>
-  <h2>{{ tr.t("graphs-intensity") }}</h2>
+<section class="card" data-series-chart>
+  <h2 data-series-title>{{ tr.t("graphs-series-both-title") }}</h2>
+  <div class="segmented" data-series hidden>
+    {% call c::series("series", "both", tr.t("graphs-series-both"), tr.t("graphs-series-both-title"), true) %}{% endcall %}
+    {% call c::series("series", "intensity", tr.t("graphs-series-intensity"), tr.t("graphs-intensity"), false) %}{% endcall %}
+    {% call c::series("series", "pm25", tr.t("graphs-series-pm25"), tr.t("graphs-series-pm25-title"), false) %}{% endcall %}
+  </div>
+  {% if everyone %}
+  <div class="segmented" data-lines hidden>
+    {% call c::series("lines", "person", tr.t("graphs-lines-person"), "", true) %}{% endcall %}
+    {% call c::series("lines", "everyone", tr.t("graphs-everyone"), "", false) %}{% endcall %}
+  </div>
+  {% endif %}
   <div class="chart">
-    <canvas data-chart="{{ intensity }}" role="img" aria-label="{{ tr.t("graphs-intensity") }}">{{ tr.t("graphs-needs-js") }}</canvas>
+    <canvas data-chart="{{ intensity }}" role="img" aria-label="{{ tr.t("graphs-series-both-title") }}">{{ tr.t("graphs-needs-js") }}</canvas>
   </div>
 </section>
-<section>
+<section class="card">
   <h2>{{ tr.t(events_key) }}</h2>
   <div class="chart">
     <canvas data-chart="{{ events }}" role="img" aria-label="{{ tr.t(events_key) }}">{{ tr.t("graphs-needs-js") }}</canvas>
   </div>
 </section>
-<section>
+<section class="card">
   <h2>{{ tr.t("graphs-rooms") }}</h2>
   {% if heatmap.is_empty() %}
   <p>{{ tr.t("graphs-rooms-none") }}</p>
@@ -91,7 +104,7 @@
   </div>
   {% endif %}
 </section>
-<section>
+<section class="card">
   <h2>{{ tr.t("log-symptoms") }}</h2>
   {% if let Some(symptoms) = symptoms %}
   <div class="chart">
@@ -101,7 +114,7 @@
   <p>{{ tr.t("graphs-symptoms-none") }}</p>
   {% endif %}
 </section>
-<section>
+<section class="card">
   <h2>{{ tr.t("graphs-areas") }}</h2>
   <p class="hint">{{ tr.t("graphs-areas-hint") }}</p>
   <div class="chart">
diff --git a/crates/web/tests/graphs.rs b/crates/web/tests/graphs.rs
index 39c56af..ccf3e1a 100644
--- a/crates/web/tests/graphs.rs
+++ b/crates/web/tests/graphs.rs
@@ -102,7 +102,38 @@ async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
     let intensity = &charts(&html)[0];
     assert_eq!(
         labels(intensity),
-        ["daniele", "partner", "guest", "PM2.5 (µg/m³)"]
+        ["daniele", "partner", "guest", "Everyone", "PM2.5 (µg/m³)"]
+    );
+    let groups: Vec<_> = intensity["data"]["datasets"]
+        .as_array()
+        .unwrap()
+        .iter()
+        .map(|d| (d["group"].as_str().unwrap(), d["token"].as_str().unwrap()))
+        .collect();
+    assert_eq!(
+        groups,
+        [
+            ("person", "accent"),
+            ("person", "series-2"),
+            ("person", "series-3"),
+            ("everyone", "accent"),
+            ("pm25", "amber")
+        ],
+        "colours are token names, resolved by graphs.js"
+    );
+    let everyone = &intensity["data"]["datasets"][3];
+    assert_eq!(everyone["hidden"], true, "the switch shows it");
+    assert_eq!(
+        everyone["data"].as_array().unwrap().len(),
+        3,
+        "every report"
+    );
+    let page = Page::parse(&html);
+    assert!(
+        page.exists("[data-series-chart] [data-series][hidden] input[name=series][value=pm25]")
+    );
+    assert!(
+        page.exists("[data-series-chart] [data-lines][hidden] input[name=lines][value=everyone]")
     );
     let datasets = &intensity["data"]["datasets"];
     assert_eq!(datasets[0]["yAxisID"], "level");
@@ -111,9 +142,9 @@ async fn residents_see_one_line_per_person_and_pm25_on_its_own_axis() {
         datasets[1]["data"][0]["x"],
         t("2026-01-10T19:00:00Z").timestamp_millis()
     );
-    assert_eq!(datasets[3]["yAxisID"], "pm25");
-    assert_eq!(datasets[3]["data"][0]["y"], 35.5);
-    assert_eq!(datasets[3]["data"].as_array().unwrap().len(), 2);
+    assert_eq!(datasets[4]["yAxisID"], "pm25");
+    assert_eq!(datasets[4]["data"][0]["y"], 35.5);
+    assert_eq!(datasets[4]["data"].as_array().unwrap().len(), 2);
     let scales = &intensity["options"]["scales"];
     assert_eq!(scales["level"]["max"], 10);
     assert_eq!(scales["pm25"]["position"], "right");
@@ -348,7 +379,15 @@ async fn filtered(app: &TestApp, cookie: &str, query: &str) -> (Vec<String>, Vec
     .await;
     let lines = charts(&html)
         .first()
-        .map(|chart| labels(chart).into_iter().map(str::to_owned).collect())
+        .map(|chart| {
+            chart["data"]["datasets"]
+                .as_array()
+                .unwrap()
+                .iter()
+                .filter(|dataset| dataset["group"] == "person")
+                .map(|dataset| dataset["label"].as_str().unwrap().to_owned())
+                .collect()
+        })
         .unwrap_or_default();
     (lines, Page::parse(&html).texts(".heatmap tbody th"))
 }
@@ -508,7 +547,7 @@ async fn a_viewer_sees_everyone_and_may_choose_a_person() {
     let html = text(app.send(get_with("/graphs", &viewer)).await).await;
     assert!(!html.contains("only your own"), "{html}");
     let charts = charts(&html);
-    assert_eq!(labels(&charts[0]), ["daniele", "partner"]);
+    assert_eq!(labels(&charts[0]), ["daniele", "partner", "Everyone"]);
     assert_eq!(
         Page::parse(&html).texts("select[name=by] option"),
         ["Everyone", "daniele", "lawyer", "partner"]
````

### Task 18: Biome for JavaScript and CSS, in CI

**Proto commit:** `6e179b2` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 6e179b2`)

Spec §9 (lint and format). `biome.json`; `biome ci .` clean (format applied; the reset's three `!important`s carry suppressions with reasons; one chip selector reordered for `noDescendingSpecificity`); a `biome` job in `ci.yml` with `biomejs/setup-biome@4c91541eaada48f67d7dbd7833600ce162b68f51 # v2.7.1`, `version: 2.5.15`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	biome.json
M	.github/workflows/ci.yml
M	crates/web/css/10-reset.css
M	crates/web/css/53-chip.css
M	crates/web/css/73-chart.css
M	crates/web/static/js/date-picks.js
M	crates/web/static/sw.js
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Tests.** This task adds no Rust test; its check is the tool run in Step 4.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **634 passed** (SQLite). Run Biome (see Global Constraints) and `actionlint .github/workflows/ci.yml`.
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
ci: Biome formats and lints the JavaScript and CSS

biome.json covers crates/web/static (minus the vendored *.min.js), crates/web/css and the
e2e folder; a new ci.yml job runs the standalone binary (biome ci, version 2.5.15, action
pinned to a commit), with no Node. The files are formatted, and the three !important rules of
the reset carry a suppression with their reason.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `6e179b2`):

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 48b57e1..2158b97 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -40,6 +40,19 @@ jobs:
       - name: audit hash ignores serde_json map ordering
         run: cargo test -p smoke-domain --lib --features serde_json/preserve_order audit
 
+  biome:
+    name: biome (JavaScript and CSS)
+    runs-on: ubuntu-latest
+    steps:
+      - uses: actions/checkout@v7
+      - name: install Biome, the standalone binary
+        # Pinned to a commit, not a tag: a third-party action's tags can be moved.
+        uses: biomejs/setup-biome@4c91541eaada48f67d7dbd7833600ce162b68f51 # v2.7.1
+        with:
+          version: 2.5.15
+      - name: format and lint (biome.json)
+        run: biome ci .
+
   coverage:
     name: coverage
     runs-on: ubuntu-latest
diff --git a/biome.json b/biome.json
new file mode 100644
index 0000000..d539bda
--- /dev/null
+++ b/biome.json
@@ -0,0 +1,18 @@
+{
+  "$schema": "https://biomejs.dev/schemas/2.5.15/schema.json",
+  "vcs": { "enabled": true, "clientKind": "git", "useIgnoreFile": true },
+  "files": {
+    "includes": [
+      "crates/web/static/**/*.js",
+      "crates/web/css/**/*.css",
+      "tests/e2e/**/*.js",
+      "!**/*.min.js",
+      "!**/node_modules"
+    ]
+  },
+  "formatter": { "indentStyle": "space", "indentWidth": 2, "lineWidth": 100 },
+  "javascript": { "formatter": { "quoteStyle": "double" }, "globals": ["Chart"] },
+  "css": { "formatter": { "enabled": true }, "linter": { "enabled": true } },
+  "linter": { "enabled": true },
+  "assist": { "enabled": false }
+}
diff --git a/crates/web/css/10-reset.css b/crates/web/css/10-reset.css
index 7d52c97..1fd15fd 100644
--- a/crates/web/css/10-reset.css
+++ b/crates/web/css/10-reset.css
@@ -56,7 +56,9 @@
     border-collapse: collapse;
   }
 
+  /* An !important in the first layer wins over every later layer: hidden stays hidden. */
   [hidden] {
+    /* biome-ignore lint/complexity/noImportantStyles: see above */
     display: none !important;
   }
 
@@ -64,7 +66,9 @@
     *,
     *::before,
     *::after {
+      /* biome-ignore lint/complexity/noImportantStyles: reduced motion beats every layer */
       animation-duration: 0.01ms !important;
+      /* biome-ignore lint/complexity/noImportantStyles: reduced motion beats every layer */
       transition-duration: 0.01ms !important;
     }
   }
diff --git a/crates/web/css/53-chip.css b/crates/web/css/53-chip.css
index 4b3a60a..ff8fb5a 100644
--- a/crates/web/css/53-chip.css
+++ b/crates/web/css/53-chip.css
@@ -51,7 +51,7 @@
   }
 
   .segmented > button,
-  .segmented > .chip > span {
+  .segmented > .chip > input + span {
     min-block-size: var(--target);
     border: 0;
     border-radius: 10px;
diff --git a/crates/web/css/73-chart.css b/crates/web/css/73-chart.css
index b1521db..f90ac7e 100644
--- a/crates/web/css/73-chart.css
+++ b/crates/web/css/73-chart.css
@@ -52,17 +52,39 @@
     white-space: nowrap;
   }
 
-  .heat-0 { background: var(--heat-0); }
-  .heat-1 { background: var(--heat-1); }
-  .heat-2 { background: var(--heat-2); }
-  .heat-3 { background: var(--heat-3); }
-  .heat-4 { background: var(--heat-4); }
-  .heat-5 { background: var(--heat-5); }
-  .heat-6 { background: var(--heat-6); }
-  .heat-7 { background: var(--heat-7); }
-  .heat-8 { background: var(--heat-8); }
-  .heat-9 { background: var(--heat-9); }
-  .heat-10 { background: var(--heat-10); }
+  .heat-0 {
+    background: var(--heat-0);
+  }
+  .heat-1 {
+    background: var(--heat-1);
+  }
+  .heat-2 {
+    background: var(--heat-2);
+  }
+  .heat-3 {
+    background: var(--heat-3);
+  }
+  .heat-4 {
+    background: var(--heat-4);
+  }
+  .heat-5 {
+    background: var(--heat-5);
+  }
+  .heat-6 {
+    background: var(--heat-6);
+  }
+  .heat-7 {
+    background: var(--heat-7);
+  }
+  .heat-8 {
+    background: var(--heat-8);
+  }
+  .heat-9 {
+    background: var(--heat-9);
+  }
+  .heat-10 {
+    background: var(--heat-10);
+  }
 
   .heat-6,
   .heat-7,
diff --git a/crates/web/static/js/date-picks.js b/crates/web/static/js/date-picks.js
index a2dfb6b..4269591 100644
--- a/crates/web/static/js/date-picks.js
+++ b/crates/web/static/js/date-picks.js
@@ -16,7 +16,12 @@ document.addEventListener("input", (event) => {
 document.addEventListener("change", (event) => {
   const radio = event.target;
   const picks = radio.closest?.("[data-date-picks]");
-  if (picks && radio instanceof HTMLInputElement && radio.type === "radio" && radio.value !== "date") {
+  if (
+    picks &&
+    radio instanceof HTMLInputElement &&
+    radio.type === "radio" &&
+    radio.value !== "date"
+  ) {
     for (const date of picks.querySelectorAll('input[type="date"]')) {
       date.value = "";
     }
diff --git a/crates/web/static/sw.js b/crates/web/static/sw.js
index e38fb2e..301f2e9 100644
--- a/crates/web/static/sw.js
+++ b/crates/web/static/sw.js
@@ -32,7 +32,9 @@ self.addEventListener("activate", (event) => {
   event.waitUntil(
     caches
       .keys()
-      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
+      .then((keys) =>
+        Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))),
+      )
       .then(() => self.clients.claim()),
   );
 });
````

### Task 19: Accessibility job: Playwright and axe-core, light and dark

**Proto commit:** `42b78eb` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 42b78eb`)

Spec §10. `tests/e2e/` (package.json, generated package-lock.json, config, seed, spec, .gitignore); the seed creates the resident with `create-resident` under `python3 -c "import pty, sys; pty.spawn(sys.argv[1:])"`, then uses the pages. The heatmap's scroll region gets `role="region" tabindex="0" aria-label` (axe `scrollable-region-focusable`). `reducedMotion: "reduce"` keeps the sheet animation from being caught half-faded. CI job `accessibility` (setup-node v7, Node 24, npm cache, `npx playwright install --with-deps chromium`, artifact 14 days); `.dockerignore` gains `tests`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
A	tests/e2e/.gitignore
A	tests/e2e/package-lock.json
A	tests/e2e/package.json
A	tests/e2e/playwright.config.js
A	tests/e2e/setup/app.js
A	tests/e2e/setup/seed.js
A	tests/e2e/specs/accessibility.spec.js
M	.dockerignore
M	.github/workflows/ci.yml
M	crates/web/templates/graphs.html
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Tests.** This task adds no Rust test; its check is the tool run in Step 4.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **634 passed** (SQLite). Locally: `cargo build -p smoke-tracker`, then in `tests/e2e`: `npm install` (writes the lock file; commit it), `npx playwright install chromium`, `SMOKE_TRACKER_BIN=/Users/daniele/Documents/Personal/smoke-tracker/target/debug/smoke-tracker npx playwright test` → 20 passed. Never commit `node_modules`, `results`, `report`, `.auth`.
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
ci: an accessibility job: Playwright and axe-core on the running app, light and dark

tests/e2e (package.json and lock file; node_modules never committed) starts the built app
on a fresh SQLite file, creates the resident with create-resident under a pseudo-terminal,
seeds through the pages (sign in, a report, a PM2.5 reading, an invite) and runs axe-core
(WCAG 2.0-2.2 A and AA) on nine pages, the rating sheet and the account menu at 390 px, in
both themes, failing on any violation or page error. A parallel ci.yml job runs it and keeps
the screenshots for 14 days. The heatmap's scrolling region becomes focusable.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `42b78eb`):

````diff
diff --git a/.dockerignore b/.dockerignore
index 23e8958..2fbe48a 100644
--- a/.dockerignore
+++ b/.dockerignore
@@ -5,6 +5,7 @@ target
 .githooks
 .superpowers
 docs
+tests
 data
 *.db
 *.db-*
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 2158b97..8dff987 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -53,6 +53,40 @@ jobs:
       - name: format and lint (biome.json)
         run: biome ci .
 
+  accessibility:
+    name: accessibility (axe, light and dark)
+    runs-on: ubuntu-latest
+    defaults:
+      run:
+        working-directory: tests/e2e
+    steps:
+      - uses: actions/checkout@v7
+      - uses: dtolnay/rust-toolchain@stable
+      - uses: Swatinem/rust-cache@v2
+      - name: build the app
+        working-directory: .
+        run: cargo build --locked -p smoke-tracker
+      - uses: actions/setup-node@v7
+        with:
+          node-version: 24
+          cache: npm
+          cache-dependency-path: tests/e2e/package-lock.json
+      - run: npm ci
+      - run: npx playwright install --with-deps chromium
+      - name: seed through the app, then axe every page at phone size
+        run: npx playwright test
+        env:
+          SMOKE_TRACKER_BIN: ${{ github.workspace }}/target/debug/smoke-tracker
+      - name: screenshots and report
+        if: always()
+        uses: actions/upload-artifact@v7
+        with:
+          name: accessibility-report
+          path: |
+            tests/e2e/report
+            tests/e2e/results
+          retention-days: 14
+
   coverage:
     name: coverage
     runs-on: ubuntu-latest
diff --git a/crates/web/templates/graphs.html b/crates/web/templates/graphs.html
index 1aa5bda..b16462f 100644
--- a/crates/web/templates/graphs.html
+++ b/crates/web/templates/graphs.html
@@ -90,7 +90,7 @@
   <p>{{ tr.t("graphs-rooms-none") }}</p>
   {% else %}
   <p class="hint">{{ tr.t("graphs-rooms-hint") }}</p>
-  <div class="scroll">
+  <div class="scroll" role="region" tabindex="0" aria-label="{{ tr.t("graphs-rooms") }}">
     <table class="heatmap">
       <thead>
         <tr><th scope="col">{{ tr.t("graphs-room") }}</th>{% for hour in hours.clone() %}<th scope="col">{{ hour }}</th>{% endfor %}</tr>
diff --git a/tests/e2e/.gitignore b/tests/e2e/.gitignore
new file mode 100644
index 0000000..e53b45e
--- /dev/null
+++ b/tests/e2e/.gitignore
@@ -0,0 +1,4 @@
+node_modules/
+results/
+report/
+.auth/
diff --git a/tests/e2e/package.json b/tests/e2e/package.json
new file mode 100644
index 0000000..1e683f8
--- /dev/null
+++ b/tests/e2e/package.json
@@ -0,0 +1,13 @@
+{
+  "name": "smoke-tracker-e2e",
+  "private": true,
+  "description": "Accessibility checks of the running app in a browser (look-and-feel spec §10)",
+  "type": "module",
+  "scripts": {
+    "test": "playwright test"
+  },
+  "devDependencies": {
+    "@axe-core/playwright": "4.13.0",
+    "@playwright/test": "1.63.0"
+  }
+}
diff --git a/tests/e2e/playwright.config.js b/tests/e2e/playwright.config.js
new file mode 100644
index 0000000..373d682
--- /dev/null
+++ b/tests/e2e/playwright.config.js
@@ -0,0 +1,32 @@
+// Starts the app on a fresh SQLite file and checks its pages at phone size, in the light and
+// the dark theme. SMOKE_TRACKER_BIN names the built binary (cargo build -p smoke-tracker).
+import { defineConfig } from "@playwright/test";
+import { BASE_URL, serverEnv, binary } from "./setup/app.js";
+
+export default defineConfig({
+  testDir: "./specs",
+  globalSetup: "./setup/seed.js",
+  fullyParallel: false,
+  workers: 1,
+  forbidOnly: Boolean(process.env.CI),
+  reporter: [["list"], ["html", { open: "never", outputFolder: "report" }]],
+  outputDir: "results",
+  use: {
+    baseURL: BASE_URL,
+    viewport: { width: 390, height: 844 },
+    // The bottom sheet's rise animation would be caught half-faded.
+    reducedMotion: "reduce",
+    storageState: ".auth/resident.json",
+  },
+  projects: [
+    { name: "light", use: { colorScheme: "light" } },
+    { name: "dark", use: { colorScheme: "dark" } },
+  ],
+  webServer: {
+    command: `"${binary}" serve`,
+    url: `${BASE_URL}/healthz`,
+    env: serverEnv,
+    reuseExistingServer: false,
+    timeout: 60_000,
+  },
+});
diff --git a/tests/e2e/setup/app.js b/tests/e2e/setup/app.js
new file mode 100644
index 0000000..7da35c4
--- /dev/null
+++ b/tests/e2e/setup/app.js
@@ -0,0 +1,23 @@
+// Where the app under test lives and how it is configured; shared by the config and the seed.
+import { mkdtempSync } from "node:fs";
+import { tmpdir } from "node:os";
+import { join, resolve } from "node:path";
+
+export const BASE_URL = "http://localhost:8181";
+export const RESIDENT = { username: "e2e", password: "a long enough passphrase" };
+
+export const binary = resolve(
+  process.env.SMOKE_TRACKER_BIN ?? join(import.meta.dirname, "../../../target/debug/smoke-tracker"),
+);
+
+const dataDir = process.env.E2E_DATA_DIR ?? mkdtempSync(join(tmpdir(), "smoke-e2e-"));
+process.env.E2E_DATA_DIR = dataDir;
+
+export const serverEnv = {
+  DATABASE_URL: `sqlite://${join(dataDir, "e2e.db")}`,
+  APP_BASE_URL: BASE_URL,
+  APP_TIMEZONE: "Europe/Rome",
+  AUTO_CLOSE_HOUR: "4",
+  BIND_ADDR: "127.0.0.1:8181",
+  RUST_LOG: "warn",
+};
diff --git a/tests/e2e/setup/seed.js b/tests/e2e/setup/seed.js
new file mode 100644
index 0000000..941697b
--- /dev/null
+++ b/tests/e2e/setup/seed.js
@@ -0,0 +1,52 @@
+// Seeds the app through the app itself (look-and-feel spec §10): the resident comes from the
+// create-resident command, which reads the password from a terminal, so it runs under a
+// pseudo-terminal; everything else is typed into the pages like a person would.
+import { execFileSync } from "node:child_process";
+import { chromium } from "@playwright/test";
+import { BASE_URL, RESIDENT, binary, serverEnv } from "./app.js";
+
+const PTY = "import pty, sys; pty.spawn(sys.argv[1:])";
+
+function createResident() {
+  execFileSync("python3", ["-c", PTY, binary, "create-resident", RESIDENT.username], {
+    input: `${RESIDENT.password}\n${RESIDENT.password}\n`,
+    env: { ...process.env, ...serverEnv },
+    stdio: ["pipe", "ignore", "inherit"],
+  });
+}
+
+/** Ticks a room and picks its level, as on the Log screen. */
+async function rate(page, room, level) {
+  const line = page.locator(".room", { hasText: room });
+  await line.locator(".room-check").click();
+  await line.locator(`.level:has(input[value="${level}"]) > span`).first().click();
+}
+
+export default async function seed() {
+  createResident();
+  const browser = await chromium.launch();
+  const page = await browser.newPage({ baseURL: BASE_URL });
+  await page.goto("/login");
+  await page.fill("input[name=username]", RESIDENT.username);
+  await page.fill("input[name=password]", RESIDENT.password);
+  await page.click("form[action='/login'] button[type=submit]");
+
+  await page.goto("/");
+  await rate(page, "Bedroom", 6);
+  await rate(page, "Living room", 3);
+  await page.locator("label.chip", { hasText: "Cough" }).click();
+  await page.click("button[value=log]");
+  await page.waitForURL("**/?saved=report");
+
+  await page.fill("input[name=pm25]", "18");
+  await page.click("button[value=log]");
+  await page.waitForURL("**/?saved=pm25");
+
+  await page.goto("/people/invite");
+  await page.fill("input[name=username]", "visitor");
+  await page.click("form.invite button[type=submit]");
+  await page.waitForSelector(".qr-code");
+
+  await page.context().storageState({ path: ".auth/resident.json" });
+  await browser.close();
+}
diff --git a/tests/e2e/specs/accessibility.spec.js b/tests/e2e/specs/accessibility.spec.js
new file mode 100644
index 0000000..158d5cd
--- /dev/null
+++ b/tests/e2e/specs/accessibility.spec.js
@@ -0,0 +1,56 @@
+// axe-core on the main pages, at phone size, in the light and the dark theme (one project
+// each). Any violation fails; a screenshot of each page is kept with the report.
+import AxeBuilder from "@axe-core/playwright";
+import { expect, test } from "@playwright/test";
+
+const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];
+
+const PAGES = [
+  ["log", "/"],
+  ["events", "/events"],
+  ["event", "/events/1"],
+  ["report", "/reports/1"],
+  ["graphs", "/graphs"],
+  ["settings", "/settings"],
+  ["people", "/people"],
+  ["invite", "/people/invite"],
+];
+
+async function check(page, testInfo, name) {
+  await testInfo.attach(name, {
+    body: await page.screenshot({ fullPage: true }),
+    contentType: "image/png",
+  });
+  const results = await new AxeBuilder({ page }).withTags(TAGS).analyze();
+  expect(results.violations, JSON.stringify(results.violations, null, 2)).toEqual([]);
+}
+
+for (const [name, path] of PAGES) {
+  test(`${name} has no accessibility violations`, async ({ page }, testInfo) => {
+    const errors = [];
+    page.on("pageerror", (error) => errors.push(String(error)));
+    page.on("console", (message) => message.type() === "error" && errors.push(message.text()));
+    await page.goto(path);
+    await check(page, testInfo, name);
+    expect(errors).toEqual([]);
+  });
+}
+
+test.describe("signed out", () => {
+  test.use({ storageState: { cookies: [], origins: [] } });
+
+  test("sign in has no accessibility violations", async ({ page }, testInfo) => {
+    await page.goto("/login");
+    await check(page, testInfo, "sign-in");
+  });
+});
+
+test("the open sheets and menus have no accessibility violations", async ({ page }, testInfo) => {
+  await page.goto("/");
+  await page.click("a[data-dialog=rating-guide]");
+  await expect(page.locator("dialog#rating-guide")).toBeVisible();
+  await check(page, testInfo, "rating-sheet");
+  await page.keyboard.press("Escape");
+  await page.click("summary.avatar");
+  await check(page, testInfo, "account-menu");
+});
````

`tests/e2e/package-lock.json` is generated by `npm install` in `tests/e2e`; commit the generated file.

### Task 20: Decisions, roadmap, README, Dependabot

**Proto commit:** `6ef96dc` (`git -C /Users/daniele/Documents/Personal/smoke-tracker-m7bproto show 6ef96dc`)

Decision rows #131–#140 (texts below), the ROADMAP row "7b Look & feel" before row 8, README (design system bullet, Biome and e2e commands, CI table row), Dependabot `npm` for `/tests/e2e`.

**Files** (status from the proto commit: A added, M modified, D deleted, R renamed):

```
M	.github/dependabot.yml
M	README.md
M	docs/DECISIONS.md
M	docs/plans/ROADMAP.md
```
- [ ] **Step 1: Start clean.** `df -h /System/Volumes/Data` (stop under 12 GB), `git status` clean on `milestone-7b/look-and-feel`, `git config user.email` is the noreply address.
- [ ] **Step 2: Tests.** This task adds no Rust test; its check is the tool run in Step 4.
- [ ] **Step 3: Implement.** Apply the rest of the diff. New files are given whole in the diff (`--- /dev/null`). Keep the code as given; any deviation is reported in the task summary.
- [ ] **Step 4: Verify.** `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` → **634 passed** (SQLite). `the_readme_tables_match_the_english_guides` must still pass.
- [ ] **Step 5: Commit** (`git add` the files listed above, nothing else) with this message, then check `git log -1 --format='%an%n%B'`:

```
docs: decisions #131-#140, the 7b roadmap row, README and Dependabot for the e2e folder

Records the look-and-feel changes (Log button, banner counts, graph filters, per-person and
Everyone lines with token colours, Install in Settings amending #121, move from the gap
warning, --control-border, code standards, structural tests and the accessibility job,
query pairs). The README gains the design system, Biome and the e2e run; Dependabot watches
tests/e2e.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

**Code** (proto `6ef96dc`):

````diff
diff --git a/.github/dependabot.yml b/.github/dependabot.yml
index 22d6d7b..7478e39 100644
--- a/.github/dependabot.yml
+++ b/.github/dependabot.yml
@@ -11,6 +11,10 @@ updates:
     directory: /
     schedule:
       interval: weekly
+  - package-ecosystem: npm
+    directory: /tests/e2e
+    schedule:
+      interval: weekly
   - package-ecosystem: docker
     directory: /
     schedule:
diff --git a/README.md b/README.md
index 5aa48a4..8de7477 100644
--- a/README.md
+++ b/README.md
@@ -121,8 +121,12 @@ flowchart LR
 
 Some choices, each recorded with its alternatives in the [decision log](docs/DECISIONS.md):
 
-- **Server-rendered pages** with plain forms; the little JavaScript there is (a dialog, the
-  install prompt, the charts) is progressive enhancement.
+- **Server-rendered pages** with plain forms; the little JavaScript there is (one ES module
+  per behaviour: dialogs, menus, the install prompt, the charts and their filters) is
+  progressive enhancement.
+- **A small design system without a build step:** design tokens and cascade layers in a few
+  CSS files joined at compile time, Askama macros for every repeated piece of markup, one SVG
+  icon sprite; light and dark follow the phone.
 - A **strict Content-Security-Policy** (`'self'` only, no inline scripts or styles): the charts
   read their data from `data-` attributes, the heatmap is a table coloured by CSS classes, and
   QR codes are inline SVG. A test scans the app's pages for anything the policy would block.
@@ -147,6 +151,11 @@ export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres
 cargo test --workspace         # Postgres tests are skipped when TEST_POSTGRES_URL is unset
 scripts/sqlx-prepare.sh        # after changing a query or a migration; commit the .sqlx/ changes
 cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
+biome ci .                     # JavaScript and CSS (the standalone Biome binary, 2.5.15)
+
+# The accessibility check in a browser (Node 24; installs Playwright's Chromium once)
+cargo build -p smoke-tracker
+cd tests/e2e && npm ci && npx playwright install chromium && npx playwright test
 ```
 
 Building needs no database: queries are checked against the committed `.sqlx/` metadata.
@@ -193,6 +202,7 @@ Pull requests into `main` need the checks marked *required* to pass.
 |---|---|---|
 | [`ci.yml`](.github/workflows/ci.yml) | push to `main`, pull requests | `fmt`, `clippy -D warnings`, the whole test suite on SQLite and a Postgres service (required); the committed `sqlx` query data (required); line coverage with `cargo-llvm-cov`, shown in the job summary and kept as an `lcov` artifact |
 | [`security.yml`](.github/workflows/security.yml) | push, pull requests, weekly | `cargo-deny` (RustSec advisories, licences, sources), `gitleaks`, a Trivy scan of the container image (all required) |
+| [`ci.yml`](.github/workflows/ci.yml) (more jobs) | push to `main`, pull requests | Biome on the JavaScript and CSS; the accessibility job: the app started on SQLite, seeded through its own pages, and axe-core on its pages at phone size in light and dark, with the screenshots kept for 14 days |
 | [`codeql.yml`](.github/workflows/codeql.yml) | push, pull requests, weekly | CodeQL for the Rust code, the workflows and the project's own JavaScript |
 | [`deploy.yml`](.github/workflows/deploy.yml) | push to `main` | Builds, scans and deploys the image to Cloud Run |
 | [`infra.yml`](.github/workflows/infra.yml) | changes under `infra/` | Terraform format, validation, tests and plan |
diff --git a/docs/DECISIONS.md b/docs/DECISIONS.md
index 486e7fa..2baba01 100644
--- a/docs/DECISIONS.md
+++ b/docs/DECISIONS.md
@@ -137,3 +137,13 @@ the old one; rows are never rewritten. Larger technical decisions also get an AD
 | 128 | 2026-10-14 | CI | **`codeql.yml`** runs CodeQL on push to `main`, pull requests and weekly for **Rust, the workflows (`actions`) and our own JavaScript**, all with `build-mode: none`; the vendored `*.min.js` files are ignored; results go to the repository's code-scanning alerts. It is **not a required check** yet | GitHub's "default setup" (no file in the repository); Rust only; Semgrep | Spec §13.1 keeps CodeQL "only if Rust analysis is stable": CodeQL lists Rust editions 2021 and 2024 as supported, no longer as a preview; free for public repositories; a file is versioned and reviewed like the other workflows. Default setup must stay off, or GitHub refuses the uploads |
 | 129 | 2026-10-14 | CI | **Coverage** is a separate `ci.yml` job: **`cargo llvm-cov`** runs the whole workspace on SQLite and a Postgres service, writes the line-coverage summary into the job summary and keeps `lcov.info` as a **14-day artifact**; no threshold, no badge, no upload to a coverage service | Codecov or Coveralls; `cargo-tarpaulin`; coverage inside the test job | Free and tokenless (no secret to protect); llvm-cov is exact on stable Rust; a separate job keeps the required `fmt · clippy · test` check unchanged, and a threshold would only invite tests written for the number |
 | 130 | 2026-10-14 | Docs | The **README** is the project's front page: what the app does, the spec §5 *How to log* rules and intensity guide, the §4 permissions table, the crate architecture, how to build, test and run it, the CI and security workflows, hosting, and the vendored libraries with their licences; a unit test checks that its intensity and *How to log* tables **match the English guide texts** | Link to the spec instead of copying the tables; generate the README from the translations | Spec §4 and §5 ask for the tables in the README; the test catches an edit to one side only, without a generator to maintain |
+| 131 | 2026-10-14 | UI | **One Log button** replaces *Log smell* and *Log PM2.5 only* and decides from the form (look-and-feel spec §6.1): rooms with a level above 0 make a smell report, all at 0 a clean check (PM2.5 saved alongside); no rooms and only PM2.5 make a reading; symptoms or a note without a room ask for a room; nothing asks for a room or a PM2.5 value. **Rooms ticked without any level count as no rooms beside a PM2.5 value**, because the Log screen ticks the rooms of the person's last report in the open event | Keep three buttons; refuse PM2.5 while a room is ticked | Fewer buttons on a screen used many times a day; without the rule a reading typed during an event would come back as "pick a level" |
+| 132 | 2026-10-14 | UI | The Log screen's **event banner** shows the open event's start, **how many of its reports the viewer may see** and the **highest room level among them with that room**; on a tie the room of the most recent report wins (observed, then recorded time, then the later room); no peak while every level is 0. Only those who see the banner today see it (look-and-feel spec §6.2) | Count every report of the event; show the event's intensity without a room | Guests learn nothing about other people's reports (#11); "6 (Bedroom)" says where to look |
+| 133 | 2026-10-14 | Graphs | **Graph filters in the address** (look-and-feel spec §7): a quick pick (this month, last 7 or 30 days, this year) or From/To dates (**at most two years**; typed dates win), **counted per day up to 62 days, per week beyond**; area or rooms (rooms win); the person (never offered to guests, and only someone the viewer may see); the part of the day (morning 06–12, afternoon 12–18, evening 18–22, night 22–06, by observed local time); a minimum level; hiding clean checks. A room or area filter **keeps only the chosen rooms of each report** and drops a report left with none (a whole-home check included); the minimum level and "hide clean checks" judge the rooms kept; PM2.5 readings follow only the dates, the person and the time of day; unusable values fall back to their default. Supersedes the month/year switch of #125 | Keep whole reports that touch a chosen room; a separate filter page; cookies | A bookmarkable GET form with no new storage query (`smoke_domain::graphs::Filter`, pure and unit-tested); narrowing makes "Bedroom" mean the bedroom on every chart |
+| 134 | 2026-10-14 | Graphs | The intensity chart keeps **one line per person** (#125) and adds an **Everyone line** for those who see everyone while no person is chosen, behind a **By person / Everyone** switch, and a **Both / Intensity / PM2.5** switch; both switches are client side and hidden without JavaScript. **Chart colours are token names** in the JSON (`accent`, `series-2…4`, `amber`), resolved by `graphs.js` from the stylesheet and again when the theme changes | Hex colours in the JSON; two charts | One palette in `tokens.css` for light and dark; spec §9 screen 5 and look-and-feel §7 |
+| 135 | 2026-10-14 | UI | **Install moves to an Install card at the top of Settings** (same behaviour: the browser's prompt, else `/install`; hidden when installed), and **who is signed in and Sign out move into the avatar menu** (a `<details>`, closed by an outside tap or Escape). Amends #121 | Keep the header link | Look-and-feel spec §6.5; the header holds only the app name and the avatar |
+| 136 | 2026-10-14 | Events | The **large-gap warning offers "Move 01:45 to a new event"** for the report after the gap, posting `target=new` to the existing move route; shown only to those allowed to move that report and **never for a no-smell report**, which cannot start an event (#105); only that report moves | Move the report and everything after it; a new route | Look-and-feel spec §6.4 with no new route or query; moving several reports at once stays future work |
+| 137 | 2026-10-14 | UI | A **`--control-border` token** (light `#7F8984`, dark `#6F7A74`, at least 3:1) outlines text inputs and check squares; rooms, level buttons and chips keep `--line` because their text identifies them | `--line` everywhere (1.3:1) | WCAG 1.4.11; research 2026-10-14 finding 4.7 |
+| 138 | 2026-10-14 | Code | **Front-end code standards** (look-and-feel spec §9): the stylesheet is small files under `crates/web/css/` joined at compile time (`concat!`) into one `/static/app.css`, every rule in a declared cascade layer, colours only as tokens in `20-tokens.css` (tests check all three); repeated markup is Askama macros (`components.html`, `report_fields.html`) taking translated text; one ES module per behaviour under `static/js/`, a custom element only for the person filter; one SVG sprite; no CSS nesting or `light-dark()` (iOS 16 floor); **Biome 2.5.15** (standalone binary) formats and lints the JS and CSS in CI | One stylesheet; `@import`; a Node build; stylelint | No duplication and no build step; the browser floor of the two phones; Biome needs no Node |
+| 139 | 2026-10-14 | Testing | Web tests assert on **structure with `scraper`** (dev-dependency; its Servo crates are MPL-2.0 and never reach the binary) through `tests/common/page.rs`; a scan refuses inline styles, scripts and handlers in every template; an **accessibility job** (Playwright 1.63 and `@axe-core/playwright` 4.13 in `tests/e2e/`, lock file committed, no `node_modules`) seeds the running app through its own pages and fails on any WCAG 2.x A/AA violation on nine pages, the rating sheet and the account menu, light and dark, at 390 px; screenshots are artifacts, no visual baselines | String matching; insta snapshots; Lighthouse CI; visual regression | Markup can change without breaking tests that check meaning; free on a public repository |
+| 140 | 2026-10-14 | Web | Query strings are read as **pairs that never reject the request**: a repeated parameter takes its first value and anything unusable falls back to the default, on every page with a query | Typed `Query` structs (400 on `?month=a&month=b`) | M6 final review, minor 1; the graph filters repeat `room` |
diff --git a/docs/plans/ROADMAP.md b/docs/plans/ROADMAP.md
index 06dd35b..69764b5 100644
--- a/docs/plans/ROADMAP.md
+++ b/docs/plans/ROADMAP.md
@@ -14,6 +14,7 @@ guesses.
 | 5 | **People, invites & PWA** | Invites with QR, account setup, password reset, expire/reactivate, settings (language, password, lists), manifest, service worker, install prompt, header Install button | §8.3, §9 (6–8), §9.1 | `05-people-and-pwa.md` |
 | 6 | **Graphs, guides & README** | Chart.js graphs, guide pages, README with permissions/rules/intensity tables, CodeQL, coverage | §5, §9 (5), §13.1 | `06-graphs-guides-readme.md` |
 | 7 | **Neon readiness** | ~~`migrate-to-postgres` command and its CI test~~ Not needed: production starts on Postgres (decision #77) | §10.2 | — |
+| 7b | **Look & feel** | Soft-card design in light and dark (tokens, CSS layers, Askama components, icon sprite, ES modules), avatar menu and tab bar, one Log button, event banner counts, graph filters with per-person lines, move-to-new-event from the gap warning, Biome and an accessibility CI job | Look-and-feel spec §1–§10 | `07b-look-and-feel.md` |
 | 8 | **Learning pack** | `docs/learning/` notes per milestone, optional exercises with tests, walkthroughs | Decision #43 | — |
 
 **After milestone 1 (done):** GitHub repo created, branch protection (ruleset) and push protection enabled, push-reminder issue created (#3).
````

## Development

Per task, from the worktree `/Users/daniele/Documents/Personal/smoke-tracker-m7b`:

```sh
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres cargo test --workspace   # Task 14 and the end
```

A quick look at a page: build `cargo build -p smoke-tracker`, create a resident under a
pseudo-terminal (`printf 'pw…\npw…\n' | python3 -c 'import pty,sys; pty.spawn(sys.argv[1:])' "$CARGO_TARGET_DIR/debug/smoke-tracker" create-resident me`
with `DATABASE_URL=sqlite://<scratch>/smoke.db`, `APP_BASE_URL=http://localhost:8080`,
`APP_TIMEZONE=Europe/Rome`, `AUTO_CLOSE_HOUR=4`, `BIND_ADDR=127.0.0.1:8080`), `serve`, and open
it in a browser at phone width in light and dark. Compare with `.superpowers/canvas/`.

## Spec coverage

| Look-and-feel spec | Tasks |
|---|---|
| §2 scope (no storage change, CSP, no-JS pages, 44 px, contrast, i18n) | all; tests in 1, 2, 4, 19 |
| §3.1 tokens, `--control-border`, heatmap ramp, theme-color | 1, 17 |
| §3.2 type, shape, spacing | 1, 2 |
| §3.3 header and avatar menu, tab bar, card, collapsible, chip, segmented, buttons, row, badge, banner, language switch | 2, 4, 7 |
| §4 screens | 7 (Log, Correct), 8 (Events, Event), 10 (Report, prompts), 11 (Settings), 12 (People, invite, link, setup), 13 (sign in, guides, install, offline, error), 16–17 (Graphs) |
| §5 Log screen | 6, 7 |
| §6.1 Log button | 5 |
| §6.2 banner counts | 6 |
| §6.3 / §7 graph filters, series switch, per-person and Everyone lines, token colours | 14, 16, 17 |
| §6.4 move to a new event | 9 |
| §6.5 Install in Settings, account in the avatar, footer languages | 4 |
| §9 code standards (CSS files and layers, macros, modules, custom element, sprite, Biome) | 1, 2, 3, 4, 17, 18 |
| §10 testing (scraper, template scan, decision table, banner, filters, guests, gap button, CSP, accessibility job) | 2, 5, 6, 9, 14–16, 19 |
| M6 carry-overs: duplicate query parameter, viewer graphs test, graphs.js try/catch | 15, 16, 17 |
| Decisions #131–#140, ROADMAP row 7b | 20 |
