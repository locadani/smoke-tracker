# Look & feel (milestone 7b) — design

**Status:** draft for review · 2026-10-14
**Design canvas:** "Smoke Tracker look & feel" (private claude.ai canvas, style **A · Soft cards**)
**Builds on:** the design spec `2026-10-13-smoke-tracker-design.md` (binding) and milestones 1–6.

## 1. Goal

Make the app look like a calm, clean, modern habit/health tracker that is pleasant to use many
times a day on a phone and good enough to show in a portfolio, **without changing how the app
works**. The screens, their contents, the flows and the three languages stay; the look and the
layout change.

**Success:** every screen matches the canvas in light and dark mode; nothing takes more taps than
today; all existing tests pass; the few behaviour changes listed in §6 are the only ones.

## 2. Scope and constraints

- **Front end:** Askama templates, `static/app.css`, `static/app.js`, `static/graphs.js`, new
  SVG icons. This is most of the work.
- **No database or storage change.** No migration, no new query in `storage-*`.
- **Small Rust changes are allowed only where §6 lists them** (web routes and, for graph filters,
  the pure `smoke_domain::graphs` code).
- **Unchanged rules:** CSP `'self'` only (no inline scripts or styles, no CDN, no web fonts);
  the system font stack; every page works without JavaScript except the charts (as today);
  touch targets at least 44 px; text contrast at least 4.5:1 in both themes.
- **i18n:** every new string gets en/it/tr keys; it/tr are drafts for native review.

## 3. Visual language

### 3.1 Tokens (CSS custom properties on `:root`, dark values under `prefers-color-scheme: dark`)

| Token | Light | Dark | Used for |
|---|---|---|---|
| `--bg` | `#F2F3F0` | `#111513` | page background |
| `--card` | `#FFFFFF` | `#1B201E` | cards, tab bar, menus |
| `--field` | `#FFFFFF` | `#151917` | inputs |
| `--text` | `#1B1F1D` | `#E7ECE9` | body text |
| `--muted` | `#5C6560` | `#A2ACA6` | labels, hints, secondary text |
| `--line` | `#DFE3DF` | `#2D3431` | borders, dividers |
| `--accent` | `#2F6B5E` | `#74C2AC` | primary buttons, links, selection |
| `--accent-soft` | `#E2EEEA` | `#21332E` | selected backgrounds, avatar |
| `--on-accent` | `#FFFFFF` | `#0C1311` | text on accent |
| `--warn-bg` / `--warn-text` | `#FBEFD9` / `#6A4710` | `#332812` / `#F2D9A2` | event-in-progress banner, large-gap warning, guest role |
| `--amber` | `#B7791F` | `#E0A84A` | PM2.5 series, common areas series |
| `--danger` | `#A33A2A` | `#F09A8A` | "End access now" |
| `--control-border` | `#7F8984` | `#6F7A74` | outline of **text inputs** (text, number, date-time, password, search, select, textarea) and of check squares; ≥ 3:1 (WCAG 1.4.11). Rooms, level buttons and chips keep `--line`, as they are identified by their text |

Heatmap: an 11-step sequential green ramp from `--accent-soft` (0) to a deep green (10), with
white text from level 6 (dark theme: reversed lightness). `theme-color` becomes `#2F6B5E`.

### 3.2 Type, shape, spacing

- System font stack. Sizes: page title 30/700, app name 22/750, card title 18/650, body 16,
  labels 14, small 12–13. Uppercase 13 px group labels (Inside, Common areas, day headers).
- Radii: cards 20, big buttons 16, fields and small buttons 12, chips fully round.
- Page gutter 16 px; 16 px between cards; cards padded 16–18 px.

### 3.3 Shared components

- **App header:** "Smoke Tracker" (22 px) on the left, **avatar** on the right (round initial).
  The avatar opens a small menu: "Signed in as *name*" and **Log out**. Built with `<details>`,
  so it works without JavaScript.
- **Bottom tab bar** (fixed at the bottom, safe-area aware): Log · Events · Graphs · People ·
  Settings, icon + label, current tab in accent. People appears only for those who see it today.
- **Card**, **collapsible card** (`<details>`: title + muted hint + accent action "Change"/"Add"
  with a chevron), **chip** (toggle), **segmented control**, **primary / secondary button**,
  **list row** (with chevron when it links), **badge**, **banner** (amber).
- **Language switch:** three-way segmented control in Settings; three text buttons at the bottom
  of the sign-in, invite and offline pages (no signed-in user there).

## 4. Screens

Screens on the canvas are drawn; the others reuse the same components with no new layout ideas.

| Screen | Canvas | Notes |
|---|---|---|
| **Log** | ✅ | §5 |
| **Events** list | ✅ | Month switcher card (‹ October 2026 ›); one card per day; rows with a square badge: amber + max level for an event, green "0" for a clean check, "PM" for a reading. Row text as today ("21:40 – in progress", "3 reports · max 6"). |
| **Event detail** | ✅ | Back link "‹ Events"; title + "span · Started by …"; timeline card (time column, dots, line; PM2.5 entries as hollow dots); large-gap warning inside the timeline with the move button (§6.4); "Close this event" card (end time + button); Reopen as a secondary button. |
| **Log detail** (`/entries/{id}`) | ✅ | Back link to its event; "edited" badge; facts as label/value rows in a card (time, author, PM2.5, rooms, symptoms, note); **Correct a mistake** as a full-width secondary button and the "Move to another event" card (select + Move) only when the log has a smell report; History card. A deleted log answers "Page not found" (Daniele, 2026-10-19: nothing can be deleted yet). |
| **Graphs** | ✅ | §7 |
| **Settings** | ✅ | Install card at the top (§6.5); Language; **Password**, **Rooms**, **Symptoms** as collapsible cards (lists: hidden items struck through with "Show again"; add field + Add; Inside/Common for rooms); Guides card with two links. |
| **People** | ✅ | "Invite someone" primary button beside the title; one card per person: avatar, name, status line, role tag (resident green, guest amber, viewer grey); actions as outlined buttons ("End access now" in red); expired accounts show the "Access ends" chips, date and Reactivate. |
| **Sign in** | ✅ | Centred: app icon (the existing `icon.svg`), app name, card with username, password, Sign in; languages at the bottom. |
| **How to rate** dialog | ✅ | Bottom sheet over a dimmed screen: handle, title, close button, levels 0–10 with name, tip, "Full guide →". Still a `<dialog>` with the full page as fallback. |
| Prompts (same smell / which event / auto-close notice) | — | Card with the question and the choices as full-width buttons. |
| Correct a report | — | Same cards as Log (Time, Rooms, Symptoms, PM2.5, Note), title "Correct a mistake". |
| Invite, QR link, account setup, install, offline, error, guide pages | — | Single centred card or plain cards; same components. |

## 5. Log screen

Top to bottom:

1. Header and title "Log a smell".
2. **Event banner** (amber, only when an event is open): "Smell event in progress", "since 21:40",
   "**3 reports · highest level 6 (Bedroom)**" (§6.2), and **Close event**. No icon.
3. **This month**: four small tiles (events, reports, bedroom days, clean checks) with a
   "This month" caption.
4. **Time** — collapsible card: "Time · now", "Change ⌄" opens the date-time field. Starts open
   when the form comes back with a time already set.
5. **Rooms** card: "How to rate" link; groups *Inside* and *Common areas*; each room a full-width
   row with a check box. A selected room is highlighted and opens its **level grid 0–10**
   (6 + 5 buttons, 44 px). The chosen number appears **only on its button**. Under the grid one
   line: before a choice "Pick a level from 0 to 10."; after it the level name in bold and its
   short description, e.g. "**Light.** Clearly smoke, but weak." (CSS only: each level carries
   its text, shown when checked.)
6. **Symptoms** card: chips.
7. **PM2.5** card, one line: "PM2.5 · optional" and a small number field with "µg/m³".
8. **Note** — collapsible card: "Note · optional", "Add ⌄". Starts open when it has text.
9. Buttons side by side: **No smell** (secondary, 2/5) and **Log** (primary, 3/5). Inside an
   open event the primary button keeps the label "Update: situation changed".

## 6. Behaviour changes (the only ones)

### 6.1 One Log button (replaces "Log smell" and "Log PM2.5 only")

The form decides from what is filled in (web route only):

| Filled in | Log does |
|---|---|
| Rooms with levels, any above 0 | Smell report as today ("Log smell"); PM2.5 saved alongside if given; the note goes on the log |
| Rooms, all at 0 | Clean check as today |
| No rooms, a PM2.5 value (± note) | PM2.5 reading alone, with the note |
| No rooms but symptoms | Error: "Pick at least one room." |
| No rooms, no PM2.5, only a note | Error: "Pick a room, or enter a PM2.5 value." |
| Nothing | Error: "Pick a room, or enter a PM2.5 value." |

*Updated by the log-entries spec (2026-10-19, decision #147).*

*No smell* is unchanged.

### 6.2 Event banner counts

The Log route also loads the open event's reports to show the report count and the highest
level reported in any room during the event, with that room (on a tie, the most recently
reported room). Shown only to those who see the banner today.

### 6.3 Graph filters (§7)

### 6.4 "Move to a new event" from the large-gap warning

The warning gets a button "Move *01:45* to a new event" (the time of the report after the gap) that posts to the **existing** move
route with `target=new` for the report after the gap. Shown only to those allowed to move that
report. Only that report moves (later reports stay; moving several at once is future work).

### 6.5 Install moves to Settings, account to the avatar

The header's *Install app* link moves to an Install card at the top of Settings (same
behaviour: browser prompt, else the `/install` page; hidden when running installed). The
"Signed in as" text and Log out move into the avatar menu. The footer language links move to
Settings (signed in) or stay at the page bottom (signed out).

## 7. Graphs

- **Filters card** at the top, `<details>` + GET form, so every filter lives in the URL and a
  view can be bookmarked or shared. Closed, it shows a one-line summary
  ("This month · Bedroom · daniele · 22–06 · level 5+").
- Filters:
  - **Dates:** quick picks *This month*, *Last 7 days*, *Last 30 days*, *This year*, plus From/To.
    Replaces today's month/year switch. Longer than about two months → events per week instead
    of per day.
  - **Where:** All / Inside / Common, plus room chips (any rooms chosen override the area).
  - **Reported by:** "Everyone ▾" field with a **search box** and a scrolling list (name + role).
    Without JavaScript it is a plain `<select>`. Not shown to guests, who keep seeing only their
    own data.
  - **Time of day:** Any, Morning 06–12, Afternoon 12–18, Evening 18–22, Night 22–06 (crosses
    midnight); by observed time.
  - **Minimum level:** slider 0–10 (0 = any).
  - **Hide clean checks:** switch.
  - **Reset** and **Show graphs**.
- **Which data each filter touches:** rooms, area and minimum level apply to smell reports; PM2.5
  readings follow only dates, person and time of day. Events per day counts an event when at
  least one of its reports matches.
- **First graph:** a **Both | Intensity | PM2.5** switch (client side) and a matching title.
  Intensity is drawn **one line per person** (main spec §9 screen 5); when no person filter is
  set, a **By person | Everyone** switch in the card header swaps to one line with every report.
  Tapping the chart keeps showing values (tooltips).
- Chart.js colours come from the tokens (accent for intensity/inside, amber for PM2.5/common).
- Implementation: the Graphs route parses the filters and narrows the reports and readings before
  the existing `smoke_domain::graphs` series code runs; filtering is pure and unit-tested in the
  domain crate. No storage change.

## 8. Changes to the main spec (approved by the user on 2026-10-14 and applied)

1. §9 screen 1: one **Log** button with the rules of §6.1; "Log PM2.5 only" removed (§7.6 then
   says a reading is logged alone by pressing Log with only PM2.5 filled in).
2. §9 screen 1: the event banner shows the report count and the highest level with its room.
3. §9 screen 5: graph filters and the series switch (§7).
4. §9.1 and decision #121: the Install button lives in Settings, not the header.
5. §7.7: the large-gap warning offers "Move to a new event" for the report after the gap.

## 9. Code standards

From the research in `docs/research/2026-10-14-frontend-standards.md` and the user's choices
(2026-10-14). Browser floor: **iOS 16** and current Chrome/Firefox.

- **No duplication:** every visual pattern exists once — one CSS component per pattern, one
  Askama macro per markup pattern, one JS module per behaviour, one icon sprite.
- **CSS:**
  - Source split into small files under `crates/web/css/` (reset, tokens, base, layout, one file
    per component), joined at compile time with `concat!(include_str!(…))` into the single
    served `/static/app.css`. No Node build step, still one request.
  - First line `@layer reset, tokens, base, layout, components, utilities;` and every rule inside
    a layer.
  - CUBE-style naming: flat component classes (`.card`, `.chip`, `.tabbar`), a few composition
    classes (`.stack`, `.cluster`), state through `data-*`/ARIA attributes and `:has()`; almost no
    utility classes.
  - Tokens as custom properties (§3.1); dark values in one `prefers-color-scheme: dark` block
    (no `light-dark()` and no native nesting: iOS 16). Logical properties throughout.
- **Templates:** `components.html` holds Askama macros (card, collapsible card, chip, button,
  list row, tab bar, header and avatar menu, icon, field), imported with `{% import %}`; content
  slots through `{% call %}`/`caller()`. Rust component structs only where there is logic.
  Templates receive translated strings, not the translator, where a macro needs text.
- **JavaScript:** `type="module"` entry plus one small module per behaviour; event delegation for
  stateless behaviour; an autonomous custom element only for per-instance widgets (the person
  search). Every page works without JavaScript first (progressive enhancement); htmx swaps keep
  working.
- **Accessibility, native HTML first:** `<details>` for collapsible cards and the avatar menu
  (plus small JS: closes on outside tap and Escape); `<dialog>` for the rating sheet and the person
  search; native radios for the 0–10 picker and segmented controls; native `<input type=range>`;
  native `<select>` as the person filter's no-JS baseline. Targets ≥ 44 px, contrast per §3.1.
- **Icons:** one sprite `static/icons.svg` (`<symbol>` + `currentColor`) used through the `icon`
  macro; decorative icons `aria-hidden`, icon-only buttons get an `aria-label`; a test checks
  every referenced id exists.
- **Charts:** `graphs.js` reads colours from the CSS tokens with `getComputedStyle` and re-reads
  them when the theme changes; animation off.
- **Lint and format:** Biome (standalone binary) for JS and CSS, in CI.

## 10. Testing

- Existing route and template tests updated for the new markup; no behaviour test removed.
- Rust HTML tests assert structure with the `scraper` crate (a shared helper in `tests/common`),
  not string matching; one global test fails if any template has an inline `style=` or
  `<script>` body.
- **Accessibility job (CI, parallel to the others):** Playwright + `@axe-core/playwright` in a
  small `tests/e2e/` folder (`package.json` + lock file, no `node_modules` committed). It starts
  the app on SQLite, creates a resident with the `create-resident` command, seeds data by using
  the app itself (sign in, log reports), and checks about 8 pages in light and dark at phone size.
  Screenshots are kept as job artifacts only (no visual-regression baselines; no Lighthouse CI).
- New tests: Log-button decision table (§6.1, every row); banner counts and tie rule; graph filter
  parsing and each filter's effect (domain unit tests + one route test per filter); guests never
  get the person filter; gap button posts to the move route and is hidden without permission;
  CSP still `'self'` only (no inline style/script in any template).
- Manual: every screen on a phone in light and dark, compared with the canvas.

## 11. Out of scope / future work

- Long lists: People grouped Active / Expired, "Show all (n)" for many symptoms and room chips,
  two-column room grid, hidden list items grouped at the bottom of Settings.
- An "Other" symptom with free text (needs a database change).
- Moving a report *and everything after it* to a new event in one step.
- A new app icon.
