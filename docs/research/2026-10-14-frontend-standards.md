# Front-end standards for a server-rendered Rust app: findings

**Checked:** 2026-10-14 · **For:** milestone 7b (look & feel), branch `milestone-7b/look-and-feel`
**Method:** primary sources only (MDN, W3C, caniuse, official docs/repos); bracketed numbers `[n]` point to **Sources** at the end. "Repo:" marks something read in this worktree. "Judgement" marks my own conclusion, which no source states; challenge those first. Nothing was built or run, except one `rustc` one-liner (item 1.7) and a contrast calculation (item 4.7).

## Summary of recommendations

| Topic | Recommendation | Alternative rejected |
|---|---|---|
| CSS tokens | Custom properties on `:root` (the spec's §3.1 table), one `@media (prefers-color-scheme: dark)` override block, `color-scheme: light dark` | `light-dark()` per token (needs iOS 17.5 [7]) |
| Cascade layers | Yes: `@layer reset, tokens, base, layout, components, utilities;` first line, everything inside a layer (iOS 15.4+ [3]) | Specificity discipline alone |
| Naming | CUBE-style: flat component classes (`.card`, `.chip`), a few composition classes (`.stack`), state via `data-`/ARIA attributes, almost no utilities | BEM (its `&__x` nesting is invalid in native CSS [11]); utility-first (needs a generator) |
| Nesting / `:has()` / container queries / logical props | `:has()` and logical properties yes; nesting only if iOS floor is 17.2+; container queries not needed now | Nesting on an iOS 16 floor (partial support [4]) |
| Theme switch | `prefers-color-scheme` only, no manual switch | Manual `data-theme` toggle (not in the spec; extra state) |
| CSS files | Several source files under `crates/web/css/`, concatenated at compile time with `concat!(include_str!(..))` into the one `/static/app.css` | Native `@import` (serial requests, must be first [10]); `build.rs`; one 800-line file |
| Askama reuse | `components.html` of macros (`{% import %}`), `{% call %}`/`caller()` for slot components; typed structs only for components with logic; pass translated strings, not `tr` | Include-per-component; deep `extends` chains |
| JS | `type="module"`, one entry file plus one module per behaviour; delegation on `document` for stateless behaviour; autonomous custom elements for per-instance widgets | Classic scripts; `htmx.onLoad` initialisers; customised built-ins (Safari refuses [21]) |
| Accessibility | Native first: `<details>` for disclosure and the avatar menu, `<dialog>` for sheets, native radios for segmented control and level picker, native `<input type=range>` | `role="menu"`, ARIA combobox, `aria-pressed` segmented buttons, custom slider |
| Person filter | Native `<select>` baseline, enhanced to a `<dialog>` with a search box and a radio list | APG combobox with `aria-activedescendant` |
| Contrast | Add a `--control-border` token at 3:1; the spec's `--line` is only 1.3:1 (finding 4.7) | Using `--line` for field borders |
| Icons | One external sprite `/static/icons.svg` + an `icon` macro; decorative `aria-hidden`; icon-only buttons get an `aria-label` | Inline SVG per template |
| Chart.js | Read tokens with `getComputedStyle` once at start; re-read and `update('none')` on `matchMedia` change; keep animation off | Hard-coded hex in server JSON |
| Lint/format | Biome (standalone binary, no Node) for JS + CSS format; stylelint-config-standard via `npx` in one CI job (needs your approval); djLint for templates, advisory | ESLint + Prettier (more packages, no gain) |
| CI checks | axe-core via Playwright on ~8 pages, light and dark, mobile viewport; no visual-regression gate; Lighthouse CI skipped | Visual baselines in repo; Lighthouse CI; pa11y-ci |
| HTML tests | `scraper` helper in `tests/common` asserting on structure; `insta` only for small component fragments | Whole-page `insta` snapshots; more `html.contains(..)` |

---

## 0. What the repo does today (read, not researched)

- Repo: `crates/web/static/app.css` is 449 lines, one file, with `--gap` and `--accent` as the only tokens, no dark values, a `<meta name="theme-color">` of `#b4532a` (`templates/base.html`).
- Repo: 24 Askama templates; `base.html` is the only `extends`; reuse is by `{% include %}` (`report_fields.html`, `room_level.html`, `list_item.html`, `expiry_fields.html`, `rating_dialog.html`). No macros. Include files read the caller's variables (`item`, `choice`).
- Repo: `app.js` (70 lines) already uses delegated listeners on `document` for `data-dialog` and `data-install`; `graphs.js` (36 lines) reads `canvas[data-chart]` JSON. Classic `defer` scripts. htmx 2.0.11 is loaded on every page, but **no template uses an `hx-` attribute** (grep), so htmx swaps are not a current concern.
- Repo: `assets.rs` lists every static file by hand and has a test that `static/` contents equal the list (non-recursive `read_dir`); `sw.js` has its own precache list that must grow with any new asset.
- Repo: `security.rs` CSP is `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; ...`. There are no inline scripts and no nonce, so nothing in this document needs one.
- Repo: tests in `crates/web/tests/*.rs` assert with `html.contains("<legend>Level in Bedroom</legend>")` and similar exact-markup strings. There is no `scraper` or `insta` dependency.

---

## 1. CSS architecture

### 1.1 Browser support (caniuse, checked 2026-10-14)

| Feature | iOS Safari | Chrome | Firefox | Global |
|---|---|---|---|---|
| Cascade layers [3] | 15.4 | 99 | 97 | 96.89% |
| Nesting [4] | **17.2** (partial 16.5-17.1) | 120 (partial 112-119) | 117 | 94.48% |
| `:has()` [5] | 15.4 | 105 | 121 | 95.91% |
| Size container queries [6] | 16.0 | 106 | 110 | 95.89% |
| `light-dark()` [7] | **17.5** | 123 | 120 | 91.28% |
| `<details name>` [37] | 17.2 | 120 | 130 | 92.49% |
| `<dialog>` [38] | 15.4 | 37 | 98 | 97.12% |
| `:focus-visible` [69] | 15.4 | 86 | 4 | 96.47% |
| JS modules [23] | 11 | 61 | 60 | 97.23% |

So the **iOS floor is the real decision** (see Open questions). On an iOS 16 floor, layers, `:has()`, container queries, `<dialog>`, modules are safe; nesting and `light-dark()` are not. MDN also marks `light-dark()` Baseline 2024 and nesting as needing `CSSNestedDeclarations` for correct ordering [8][11].

### 1.2 Design tokens as custom properties

**Recommendation for this repo:** implement the spec's table (§3.1) exactly as custom properties on `:root`, light values first, then a single dark override block. Add `color-scheme: light dark` so form controls, scrollbars and the canvas colour follow the theme [8][15]. Never use a raw hex outside `tokens.css`. Add the spacing/radius scale as tokens too (`--radius-card: 20px`, `--space-3: 1rem`), so component files contain no magic numbers.

```css
/* css/tokens.css */
@layer tokens {
  :root {
    color-scheme: light dark;
    --bg: #F2F3F0;   --card: #FFFFFF;  --text: #1B1F1D;  --muted: #5C6560;
    --accent: #2F6B5E; --accent-soft: #E2EEEA; --on-accent: #FFFFFF;
    --control-border: #7F8984;           /* new, see 4.7 */
    --radius-card: 20px; --radius-field: 12px; --gutter: 16px;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #111513;  --card: #1B201E;  --text: #E7ECE9;  --muted: #A2ACA6;
      --accent: #74C2AC; --accent-soft: #21332E; --on-accent: #0C1311;
      --control-border: #6F7A74;
    }
  }
}
```

**Alternative and why not:** `light-dark(#F2F3F0, #111513)` per token halves the lines and removes the override block, and MDN shows it works with `var()` [8]. But it needs iOS 17.5 [7], and in a browser without it every colour declaration becomes invalid (Judgement: the page would render with default colours). A 14-line override block costs little. Revisit when the iOS floor is 17.5.

### 1.3 Cascade layers

MDN: later-declared layers win, **unlayered styles beat layered ones**, and inside the layer order specificity is ignored; `!important` reverses the order [9]. A statement at the top fixes the order wherever the rules appear [9].

**Recommendation:** declare the order once, at the top of the concatenated file, and put *all* rules in a layer, so no unlayered rule ever silently wins:

```css
@layer reset, tokens, base, layout, components, utilities;
```

Use `layout` for `.stack`/`.cluster`/page shell, `components` for `.card`, `.chip`, `.btn`, `utilities` for the handful of `.visually-hidden`-style helpers. This removes the need for specificity tricks and `!important`.

**Alternative:** no layers, rely on source order. Works at 450 lines, becomes fragile at 1000+ with many states; layers cost one line. Not recommended against.

### 1.4 Naming methodology

- **BEM** names `block__element--modifier`. I could not fetch the official site (en.bem.info returned 404, getbem.com did not resolve), so BEM's own claims are not quoted here. One checkable fact: MDN says the Sass-style `&__child` concatenation is **not valid** in native nesting [11], so BEM's element naming cannot be written with native nesting, only repeated in full.
- **CUBE CSS** (Composition, Utility, Block, Exception; its site says it is "designed to work with the medium", the cascade and inheritance "are embraced", and that it has been used from large sites to "tiny blogs" [16]).
- **Utility-first** needs a generator to stay small (Judgement); a no-Node-build repo would hand-write utilities, which is CUBE's own "utility" tier anyway.

**Recommendation:** CUBE CSS, trimmed: write element-level `base` rules (`h1`, `button`, `label`), a few composition classes (`.stack` exists already), one flat class per component (`.card`, `.chip`, `.row`, `.tabbar`), state by attribute (`[aria-current="page"]`, `details[open]`, `input:checked`, `[data-tone="warn"]`), not extra classes. Keep the class vocabulary in a comment block at the top of `components.css` and nowhere else.

```css
@layer components {
  .chip > input { position: absolute; opacity: 0; }          /* native control kept, 4.8 */
  .chip > span  { display: inline-flex; min-block-size: 44px; padding-inline: 1rem;
                  border: 1px solid var(--control-border); border-radius: 999px; }
  .chip:has(> input:checked) > span { background: var(--accent-soft); border-color: var(--accent); }
  .chip:has(> input:focus-visible) > span { outline: 2px solid var(--accent); outline-offset: 2px; }
}
```

### 1.5 Native nesting, `:has()`, container queries, logical properties

- **`:has()`**: yes. It is how "a selected room is highlighted" (spec §5) and chips are styled with no JS (above). MDN performance advice: anchor to a specific container, use `>`, keep subtrees small, never `body:has(...)` [14]. Since iOS 15.4 [5].
- **Logical properties**: yes, always (`margin-inline`, `padding-block`, `inset-inline-end`, `inline-size`). MDN: they map to the writing direction, so content "will be rendered as intended" in other writing modes [12]. The app has no RTL language now (en/it/tr), so this is consistency and future-proofing at no cost (Judgement).
- **Nesting**: MDN documents relaxed syntax (nested type selectors without `&`) and the `:is()`-like specificity of `&` [11], but iOS Safari only fully supports it from 17.2 [4]. **Recommendation:** do not adopt until the iOS floor is confirmed 17.2+; flat selectors with `:has()` already cover the spec's needs. If adopted, nest at most one level.
- **Container queries**: supported from iOS 16.0 [6][13] but the app is a single column capped at 32rem; a component never lives in two different widths. Not needed (Judgement). Revisit if the two-column room grid (spec §10) lands.

### 1.6 `prefers-color-scheme` vs manual switch

MDN: the media query reflects the OS/browser preference; a manual override needs a root attribute plus `color-scheme`, JS and `localStorage` [15]. The spec specifies dark under `prefers-color-scheme: dark` and has no toggle (§3.1). **Recommendation:** media query only. A persisted toggle adds JS state, a flash-of-wrong-theme problem before the script runs, and a test matrix, for two users whose phones already switch at sunset (Judgement).

### 1.7 One file or several; serving without a build step

- **Native `@import`**: MDN: it must come before every other rule except `@charset`/`@layer`, supports `layer(name)`, and cannot be nested in `@media` [10]. Each import is a second request discovered after the first stylesheet is parsed, i.e. a chain; Lighthouse defines such dependent request chains as the thing that hurts load time [68]. (That an `@import` is discovered only after its parent is parsed is not stated on the MDN page I fetched; Judgement.)
- **`<link>` per file**: parallel, but `assets.rs` and `sw.js` need an entry per file, and unlayered load order must be managed in HTML.
- **`build.rs` / startup concatenation**: works, but adds a second place where the asset list lives.
- **Compile-time `concat!`**: I verified with `rustc` that `concat!(include_str!("a.css"), include_str!("b.css")).as_bytes()` compiles in a `const` of type `&[u8]` (output length matched). So `assets.rs` can keep serving **one URL with one ETag** and no build script:

**Recommendation for this repo:**

```
crates/web/css/            # NOT under static/, so the "every file in static/ is served" test is untouched
  00-layers.css  10-tokens.css  20-base.css  30-layout.css
  40-components.css (or components/*.css)  90-utilities.css
```
```rust
// assets.rs: a second macro beside asset!
macro_rules! asset_css {
    ($($file:literal),+ $(,)?) => { Asset {
        path: "app.css", content_type: "text/css; charset=utf-8",
        body: concat!($(include_str!(concat!("../css/", $file))),+).as_bytes(),
    } };
}
// asset_css!("00-layers.css", "10-tokens.css", /* ... */)
```
Order in the macro call is the cascade order; `00-layers.css` holds the `@layer` statement so it is first. Dev loop is unchanged (a rebuild, like `include_bytes!` today). `sw.js` and templates keep `/static/app.css`. Add a test that the `css/` folder and the macro list agree (same pattern as the existing static-folder test).

**Alternative:** keep a single file. Acceptable for M7b if it stays under ~800 lines, but several files is what a reviewer reading the repo as a portfolio expects ("tidy and logically split", CLAUDE.md).

---

## 2. Template reuse in Askama 0.16.1

Facts below are from the Askama book [1] and the 0.16.1 source in the local cargo registry (`askama_derive-0.16.1`, `askama_parser-0.16.1`) [2].

- **Macros**: `{% macro name(a, b=default) %}`, named or positional arguments (optional last), type annotations allowed, scope inherited from the call site [1].
- **Call blocks**: `{% call(x) name(args) %} body {% endcall %}`; inside the macro `{{ caller() }}` inserts the body, may take arguments and be called several times; `caller is defined` supports both forms; a nested macro call overwrites `caller` (alias it with `{% set outer = caller %}`) [1]. The parser has `call` and the generator handles `caller` as an expression [2].
- **Import**: `{% import "components.html" as c %}` then `{{ c::chip(...) }}`; `let`/`set` variables cannot be imported [1].
- **Include**: the included file sees the **whole context**, including loop variables; path must be a literal [1]. That is why `list_item.html` works today, and also why it is an implicit contract ("Expects `item`").
- **Inheritance**: `extends`, `block`, `super()`. Blocks may nest but not sit inside `if`/`for`; the book does not describe multi-level `extends` [1]. **Judgement:** keep depth at 1 (`base.html` only), plus at most a `{% block %}` for the page header.
- **Typed component structs**: a struct deriving `Template` can be rendered inside another with `{{ section }}`; output is **escaped** unless you add `|safe` or `impl askama::filters::HtmlSafe for Section {}` [1]. Block fragments (`block = "name"`) render one block alone [1]; the derive source also parses a `blocks` attribute that the book does not document [2], so do not rely on it.
- **i18n**: Repo: templates call `tr.t("key")` where `tr` is a `Tr` field on each page struct (`error.rs`). Since macros inherit the call-site scope [1], a macro *could* call `tr`, but the book does not say whether an **imported** macro sees the importer's fields; I did not verify it, so run a 10-line spike before depending on it.
- **CSP nonce**: none needed: CSP is `'self'` only and there are no inline scripts/styles [repo: `security.rs`]. Components must simply never emit `style=`, `on*=` or inline `<script>`/`<style>`.

### Recommendation: three tiers

1. **Presentational components = macros** in `templates/components.html` (or `components/*.html`, one concern per file). Take plain values, including **already translated strings**, so components never depend on `tr` (no spike risk, trivially testable):
2. **Slot components = call blocks** (`card`, `collapsible`, `list_row`).
3. **Typed struct only where there is logic** (computing "current tab", showing People only for some roles, avatar initial). Used once, so the struct lives next to the page code; the `base.html` header and tab bar are used on every page but defined once, so a macro or inline block in `base.html` is enough. Add a struct only if the logic gets a unit test of its own.

```html
{# templates/components.html #}
{% macro chip(kind, name, value, checked, label) %}
<label class="chip"><input type="{{ kind }}" name="{{ name }}" value="{{ value }}"{% if checked %} checked{% endif %}><span>{{ label }}</span></label>
{% endmacro %}

{% macro card(title) %}
<section class="card">{% if title != "" %}<h2 class="card-title">{{ title }}</h2>{% endif %}{{ caller() }}</section>
{% endmacro %}

{% macro collapsible(title, hint, action, open) %}
<details class="card collapsible"{% if open %} open{% endif %}>
  <summary><span class="card-title">{{ title }}</span><span class="muted">{{ hint }}</span><span class="action">{{ action }}{% call icon("chevron-down") %}</span></summary>
  <div class="card-body">{{ caller() }}</div>
</details>
{% endmacro %}

{% macro icon(name) %}<svg class="icon" aria-hidden="true" focusable="false"><use href="/static/icons.svg#{{ name }}"/></svg>{% endmacro %}
```
```html
{# use #}
{% import "components.html" as c %}
{% call c::collapsible(tr.t("log-time"), tr.t("log-time-now"), tr.t("change"), form.time_set) %}
  <input type="datetime-local" name="at" value="{{ form.at }}">
{% endcall %}
```
(Macro calls use the `{% call name(args) %}` statement [1]. I did not compile these snippets, so treat them as a sketch and spike them with the real compiler.)

| Component | Form | Notes |
|---|---|---|
| Card | macro + `caller()` | `title` optional |
| Collapsible card | macro + `caller()`, native `<details>` | `open` from server state (spec §5: starts open when the form returns with a value) |
| Chip | macro `chip(kind,...)` | replaces the chip markup in `room_level.html` and the symptom list; `kind` is `checkbox` or `radio` |
| Button | one `.btn` class + `data-variant` (`primary`, `secondary`, `danger`); a macro only if the markup carries logic | a macro for a bare `<button>` is noise |
| List row | macro + `caller()`; renders `<a>` when `href` is set, else `<div>` | chevron icon only when it links |
| Tab bar | inline in `base.html`, `{% if me.people %}` as today | single use; the current tab via `aria-current="page"` (APG disclosure-navigation shows nav as links, not tabs [25]) |
| Header + avatar menu | inline in `base.html` | `<details class="menu">` (4.2) |

**Alternatives rejected:** (a) `include` per component: implicit variable contracts, no named arguments [1]. (b) A struct per component: more Rust than HTML for something that has no logic, plus `HtmlSafe` boilerplate [1]. (c) Deep `extends` chains: the book does not cover them [1].

---

## 3. Vanilla JS progressive enhancement

### 3.1 Modules vs classic scripts

MDN: `<script type="module">` is deferred by default, strict, module-scoped, runs once even if referenced twice; import specifiers need full relative paths with the extension; native modules do not replace bundlers for chunking/minification [22]. Supported since iOS 11 [23].

**Recommendation:** `<script type="module" src="/static/app.js">`. Folder `crates/web/static/js/`: `app.js` (entry: imports the behaviours), `dialog.js`, `install.js`, `expiry.js`, `person-filter.js`, `service-worker.js`; `graphs.js` stays a separate page-specific module. Constraints this puts on the repo:
- Every file must be added to `assets.rs` and `sw.js` precache (and `static/` stays flat or the read_dir test becomes recursive).
- Keep the import graph one level deep (entry imports leaves) so there is no import waterfall; optionally `<link rel="modulepreload">` for the leaves [22].
- `graphs.js` needs the global `Chart` from the classic UMD file. Both are deferred, so document order should hold, but verify in a browser test; if in doubt keep `graphs.js` classic.

**Alternative:** keep classic scripts. With ~150 lines total they work, but one growing file with section comments is the pattern that creates duplication; modules give a file per behaviour with zero tooling. (Judgement.)

### 3.2 Custom elements vs data-attribute initialisers vs delegation

- MDN: autonomous custom elements work everywhere; customised built-ins (`<ul is=...>`) are not planned by Safari [21]. `connectedCallback` runs each time the element is added to the document; do work there, not in the constructor [21].
- htmx: `htmx:load` fires once at startup (target `body`) and for each newly loaded node (`detail.elt`) [20]; `htmx.onLoad(fn)` exists to initialise third-party code on swapped content [18][19].
- Repo: no `hx-` attributes today, so swaps are hypothetical.

**Recommendation (a three-way split):**
1. **Stateless, page-wide behaviour → event delegation on `document`** (what `app.js` does for `data-dialog`, `data-install`, expiry). Listeners on `document` survive any swap, so need no `htmx:load`. Keep this default.
2. **Per-instance widgets with their own state → autonomous custom element** that wraps the server-rendered, working HTML: `<person-filter>` around the `<select>`, `<series-switch>` around the three radios and the chart. Upgrading is automatic when the element appears, including inside swapped HTML, with no `htmx.onLoad` glue [21]; without JS the wrapped native control works.
3. **`htmx.onLoad`**: only if a third-party library needs per-node init [19]. Not needed now.

```js
// js/person-filter.js
class PersonFilter extends HTMLElement {
  connectedCallback() {
    const select = this.querySelector("select");
    if (!select || this.dataset.ready) return;
    this.dataset.ready = "";
    /* build the dialog + search + radio list from select.options; write back to select on pick */
  }
}
customElements.define("person-filter", PersonFilter);
```
Pick data-attribute initialisers (`[data-x]` + `querySelectorAll` at load) never: they need the `htmx:load` bookkeeping that delegation and custom elements avoid (Judgement).

### 3.3 Organising small behaviours

One file per behaviour exporting nothing (or a `register()`); `app.js` is only imports. Shared tiny helpers (`closest`, `qs`) in `dom.js`. File header comment states the no-JS fallback for each behaviour (the repo already does this in `app.js`). Test the fallback by disabling JS in the Playwright run (section 8).

---

## 4. Accessibility patterns

Principle: APG's own first rule is "No ARIA is better than Bad ARIA" and ARIA roles add no keyboard behaviour or styling [32]; so native elements first.

### 4.1 Disclosure / collapsible card

APG disclosure: a `button` with `aria-expanded`, optional `aria-controls`, Enter and Space toggle [26]. The APG page does not mention `<details>`. MDN: `<details>` toggles on click and on Space, is Widely available since January 2020, implicit role `group`, `open` set server-side [35].
**Recommendation:** `<details class="card collapsible"><summary>…</summary>…</details>` (macro in section 2). Keep the summary text-only (no links or buttons inside it), so "Change ⌄" is styled text; the whole summary is the 44 px target. `<details name>` (exclusive group) is not needed and is iOS 17.2+ [37].
**Alternative:** button + `aria-expanded` + JS: more code, fails without JS.

### 4.2 Avatar menu: disclosure, not `role="menu"`

APG's disclosure-navigation example says it deliberately does **not** use the `menu` role because it lacks "the complex functionality assistive technologies expect" of a menu widget, and typical site navigation does not need the menu/menubar keyboard model [25]. The menu-button pattern describes the full `role="menu"` model [24]. The avatar menu holds a name line and a Log out button: a disclosure.
**Recommendation:** `<details class="menu"><summary aria-label="{{ tr.t("account") }}">{{ initial }}</summary><div class="menu-panel"><p>Signed in as …</p><form method="post" action="/logout"><button>Log out</button></form></div></details>`. Add one small delegated enhancement: Escape and outside click close it, focus returns to the summary. Without JS it still opens and closes.
**Alternative:** `role="menu"`/`menuitem`: requires arrow-key roving focus, typeahead, Escape [24]; more JS and more ways to be wrong for two items.

### 4.3 Person filter (combobox/listbox with search)

APG combobox needs `role="combobox"` on the input, `aria-controls`, `aria-expanded`, `aria-autocomplete`, DOM focus staying on the input with `aria-activedescendant` pointing into the listbox, plus the arrow/Enter/Escape model [27]. APG also lists a dialog popup variant where focus moves into the dialog [27].
**Recommendation:** baseline is the native `<select name="by">` (works without JS, native iOS/Android picker). Enhance with the `<person-filter>` custom element: a button showing the current name opens a `<dialog>` containing a search `<input type="search">` and a `<fieldset>` of native radios (name + role); typing hides non-matching labels; choosing a radio writes the `<select>` value and closes. Everything announced is native radio-group semantics, no ARIA widget. Hidden `<select>` stays the form field, so the GET URL is unchanged.
**Alternative:** APG combobox: more ARIA surface, more assistive-technology variance, and it is built for long lists; with a handful of people the dialog is lighter (Judgement).

### 4.4 Dialog / bottom sheet

MDN: `showModal()` makes the rest inert, gives implicit `aria-modal="true"`, Esc closes, `::backdrop` styles the dim, `form method="dialog"` closes with no request, and an explicit close button is the most reliable way out; `autofocus` picks the initial focus; `closedby="any"` adds light dismiss [36]. APG modal dialog requires an accessible name (`aria-labelledby`), focus moves in on open and back to the opener on close, a visible close button [28]. Supported on iOS 15.4+ [38].
**Recommendation:** keep the repo's pattern (`<dialog aria-labelledby>`, `<form method="dialog">` close button, link fallback to the full page). Style as a sheet with `dialog { margin-block-end: 0; inline-size: 100%; max-inline-size: 32rem; border-radius: var(--radius-card) var(--radius-card) 0 0 }` and `::backdrop`. Add `autofocus` to the close button. Slide-in animation only inside `@media (prefers-reduced-motion: no-preference)` (MDN advises default = full experience and reduce overrides; either way, replace motion with fade [65]). Invoker `command="show-modal"` is in MDN's dialog page [36], but I did not check its support, so keep `data-dialog` JS.

### 4.5 Segmented control

Three different jobs in the spec:
- **Language switch (Settings)**: choose one value and submit: **native radios** in a `<fieldset><legend>` styled as a segmented control, or (as today) three submit buttons; keep today's `<button name="lang" value>` form because a switch applies immediately and the sign-in/invite/offline pages already use it. Mark the current one with `aria-current="true"` and `disabled`-looking style, not `aria-pressed`.
- **Series switch (Both / Intensity / PM2.5)**: client-side choose-one → **native radio group** (`<input type=radio>` visually hidden, `.chip`-like label). APG radio group: arrows move and check, Tab enters on the checked one [29], which native radios already do (the APG page itself does not mention native inputs).
- **Not tabs**: APG tabs are for sets of panels where one is shown at a time [31]; a series switch changes one chart, not panels. Not the bottom tab bar either: those are links (use `aria-current="page"` [25]).
- **`aria-pressed` buttons**: APG toggle buttons are for on/off state and the label must stay the same [70]. Use for the "Hide clean checks" switch only if it is a button; as a form field a native checkbox is better.

### 4.6 Level 0-10 picker and range slider

- **Level picker**: Repo: already `<fieldset class="levels"><legend>` of native radios (`room_level.html`). Keep; the visible chosen level and its description (spec §5) by `:has(:checked)`/sibling CSS. 11 radios as a 6+5 grid is fine; arrows work natively [29].
- **Slider (minimum level)**: native `<input type="range" min=0 max=10 step=1>` plus `<output>`; the 0 = "any" text is set in JS (`aria-valuetext`), without JS the number shows. APG's custom slider page warns that touch-based assistive technologies may not generate the needed input for custom sliders and to test on touch devices [30]; a native range avoids that. Thumb at least 24 px (WCAG 2.5.8, [33]), aim for 44 px like everything else.

### 4.7 Contrast and target size (WCAG 2.2)

- **2.5.8 Target Size (Minimum), AA**: 24x24 CSS px, with spacing, inline, equivalent, essential exceptions [33]. The spec's 44 px is stricter (44 px is the AAA 2.5.5 value; the text of 2.5.5 was not quotable from my fetch [34], so treat the number as from memory).
- **1.4.3**: text 4.5:1, large text 3:1. **1.4.11**: UI components and graphical objects 3:1 against adjacent colours [34]. **2.4.7** focus visible; **2.4.11** focus not obscured (important: the fixed bottom tab bar must not cover a focused field, use `scroll-padding-block-end`) [34].
- **I computed the contrast of the spec's tokens (§3.1)**, WCAG relative-luminance formula, light / dark:

| Pair | Light | Dark |
|---|---|---|
| text on bg / card | 14.96 / 16.66 | 15.41 / 13.82 |
| muted on bg / card / field | 5.41 / 6.03 / 6.03 | 7.88 / 7.07 / 7.60 |
| accent on card / bg / accent-soft | 6.20 / 5.57 / 5.21 | 7.91 / 8.82 / 6.37 |
| on-accent on accent | 6.20 | 9.00 |
| warn-text on warn-bg | 7.32 | 10.48 |
| danger on card | 6.57 | 7.62 |
| amber on card (chart series, needs 3:1) | 3.64 | 7.76 |
| **line on card / bg** | **1.30 / 1.16** | **1.29 / 1.44** |

All text pairs pass 4.5:1. **Finding:** `--line` (1.3:1 on the card) is fine for dividers but fails 1.4.11 if it is the only thing that shows an input, chip or checkbox boundary, and the spec's `--field` equals `--card` in light mode (both `#FFFFFF`). **Recommendation:** add `--control-border` (light `#7F8984` = 3.61:1 on white, 3.24:1 on bg; dark `#6F7A74` = 3.70:1 on card, 3.98:1 on field) for field/chip/checkbox borders; keep `--line` for dividers. Amber in light mode is 3.64:1, so it passes for chart lines but must not carry text.

---

## 5. SVG icons under CSP `'self'`

- MDN: `<use href="file.svg#id">` can reference an external file; it may be subject to same-origin policy, so serve the sprite from your own origin; `xlink:href` is deprecated in favour of `href`; CSS properties are not guaranteed to reach the clone unless inherited [39]. The page does not discuss `currentColor`; inherited properties such as `color` do pass into the shadow tree (Judgement, verify in the browser run).
- Which CSP directive governs `<use>` fetches is not stated on that page. `img-src 'self'` and `default-src 'self'` both allow same-origin; test once with the browser console open.

**Recommendation:** one sprite `crates/web/static/icons.svg` of `<symbol id="log" viewBox=…>` elements using `fill/stroke="currentColor"`, and the `icon` macro (section 2). Add the file to `assets.rs` (`image/svg+xml`) and to the `sw.js` precache so tab icons work offline. A Rust test should assert every `#id` referenced by a template exists in the sprite (scraper or plain string scan).
- **Decorative** (icon + visible label, like the tab bar): `aria-hidden="true" focusable="false"`; WCAG 1.1.1 allows decoration to be "implemented in a way that it can be ignored by assistive technology" [34].
- **Meaningful, icon-only** (e.g. a close button): the **button** carries `aria-label` (Repo: `rating_dialog.html` already does this for "✕"); the SVG stays `aria-hidden`. No `<title>` per icon.

**Alternative:** inline `<svg>` per template: no request, but path data repeated in every page that uses it and in every `macro` expansion; larger HTML for 5+ nav icons on every page (Judgement). Choose inline only if the sprite fails the CSP/`currentColor` test above.

---

## 6. Chart.js 4 theming from CSS tokens

Facts: `Chart.defaults.color`, `borderColor`, `backgroundColor` set global colours [40]; mutate `chart.options` in place and call `chart.update()`, `update('none')` skips animation [41]; canvas needs `role="img"` + `aria-label` or fallback content [42]. Repo: `graphs.js` already sets `Chart.defaults.animation = false`, and the server puts the chart config in `data-chart` JSON.

**Recommendation:**
1. The server emits **token names**, not colours (`"borderColor": "accent"`, `"amber"`), keeping the palette in one place (CSS).
2. `graphs.js` resolves them once:
```js
const css = getComputedStyle(document.documentElement);
const token = (name) => css.getPropertyValue(`--${name}`).trim();
Chart.defaults.color = token("muted");
Chart.defaults.borderColor = token("line");
const resolve = (value) => (typeof value === "string" && /^[a-z-]+$/.test(value) ? token(value) : value);
```
3. Dark-mode switching while the page is open: `matchMedia("(prefers-color-scheme: dark)").addEventListener("change", …)`, re-resolve (create a fresh `getComputedStyle` object; it is live in browsers but be explicit), assign the colours into `chart.options`/datasets and `chart.update("none")` [41][15].
4. **Reduced motion**: already satisfied by `animation = false`. If animation is ever enabled: `Chart.defaults.animation = matchMedia("(prefers-reduced-motion: reduce)").matches ? false : { duration: 200 }` [65].
5. Each `<canvas>`: `role="img"`, `aria-label` with the chart title, and a text fallback or the table already on the page [42]. The heatmap is a `<table>` in the spec, which is the accessible form.

**Alternative:** hex values in the server JSON: two palettes to maintain, dark mode needs server awareness.

---

## 7. Linting and formatting

| Tool | Versions checked 2026-10-14 | Needs Node? | Notes |
|---|---|---|---|
| stylelint + `stylelint-config-standard` | stylelint 17.16.0 [44] | Yes (`npx stylelint "**/*.css"`) [43] | The reference CSS linter. Catches duplicate properties, descending specificity, invalid values, custom-property naming. |
| Biome | 2.5.15 [47] | **No**: standalone binary per platform from GitHub releases [48]; `biomejs/setup-biome@v2` + `biome ci .` [49] | JS: parse/format/lint/plugins; CSS: parse/format/lint/plugins; HTML: formatter experimental, opt-in [46]. CSS rule depth vs stylelint not verified (rules page was a 404). |
| ESLint + Prettier | not researched | Yes, many packages | Rejected: more moving parts for the same job as Biome (Judgement). |
| djLint | 1.46.4 [51] | No (`pip install djlint`) [50] | Lists Askama among supported template languages [50]; has H-rules (HTML) and T-rules (template) and in-template `{# djlint:off H022 #}` switches [52]. Flags (`--check`, `--lint`, `--profile`) not documented on the pages I fetched: confirm with `djlint --help`. |

**Recommendation (no committed `node_modules` in any case):**
- **Biome standalone**, pinned to 2.5.15, for `static/js/**` and `css/**`: `biome ci crates/web`. Config `biome.json` at the repo root. No Node, no lockfile.
- **stylelint** is the one that needs Node. If you approve it: `tools/lint/package.json` + `package-lock.json` (committed, no `node_modules`), CI job `npm ci && npx stylelint "crates/web/css/**/*.css"` with `stylelint-config-standard` and a custom-property pattern. Biome would then only format CSS if their formatters agree; otherwise let stylelint own CSS and Biome own JS. See Open questions.
- **djLint**: run in `--lint` mode as an advisory (non-blocking) job first, because Askama syntax is not Jinja and an unreviewed formatter rewriting 24 templates is risky; never auto-format in CI.
- Template-level rules that matter more than HTML style are enforced by Rust tests (section 9): no `style=`, `on*=`, inline `<script>`.

---

## 8. Free automated checks in CI

Repo is **public**, so standard GitHub-hosted runner minutes are free [62]; the cost is therefore wall time and maintenance, not money. Existing jobs: fmt/clippy/test, coverage, sqlx offline data (`ci.yml`).

| Check | CI time (est.) | Flakiness | Setup | Worth it? |
|---|---|---|---|---|
| axe-core via Playwright (`@axe-core/playwright`) [53][58] | 2-4 min incl. browser install `npx playwright install --with-deps` [55] | Low (deterministic rules) | Medium-high: needs a running seeded server | **Yes** |
| pa11y-ci [59] | 1-3 min | Low | Medium: same server need, htmlcs default runner, axe optional | No: second tool for what Playwright+axe already does |
| Playwright visual regression [54] | 3-6 min | **High across machines**: baselines are per browser+platform and differ with fonts and rendering; run where baselines were made [54] | High, plus baseline churn | **No** for M7b |
| Lighthouse CI [60][61] | 3-5 min (default 3 runs per URL [61]) | Medium: performance varies; presets already warn instead of fail for flaky audits [61] | Medium | **No**: its accessibility score is axe-based and overlaps; performance of a 3-file app is not the risk (Judgement) |

(CI time figures are my estimates; the sources give no timings.)

**Recommended minimal set:**
1. **Rust structure tests** (section 9) in the existing job: zero new infra, catches markup contract regressions.
2. **One axe job**: `tests/e2e/` with `package.json` + lockfile, Playwright Chromium, `AxeBuilder.withTags(["wcag2a","wcag2aa","wcag21aa","wcag22aa"])` (the `wcag22aa` tag exists [58]; Playwright's page shows the 2.1 tags [53]). Visit about 8 pages (login, log, log with open event, events, event, graphs, settings, people) at a 390 px viewport, once per colour scheme (`page.emulateMedia({ colorScheme })`, Playwright API from memory, verify) so colour contrast is checked in both themes. The server: build the release binary, start it with a SQLite file seeded by a small SQL/`curl` script or a `--demo-data` dev flag (spec change, so a question for you). Assert zero violations; track unavoidable ones in an allow-list file with a reason. axe's README claims it finds on average 57% of WCAG issues automatically [57] and Playwright says automation cannot catch everything [53]: keep a manual checklist (VoiceOver on iOS, TalkBack on Android, keyboard-only) in the milestone's manual test list.
3. **Screenshots as artifacts, not gates**: the same job can upload mobile screenshots light/dark via `actions/upload-artifact` [55] so reviewers compare with the canvas; no baselines in git.

**If you later want visual regression:** pin one runner image and browser, store baselines only for `chromium-linux`, generate them from a `workflow_dispatch` job (never on a laptop, platform suffix differs [54]), set `maxDiffPixels` and hide volatile elements (time, dates) with `stylePath` [54], and keep baselines in the repo under `tests/e2e/__screenshots__/` (a few hundred KB of PNG; mind the "app must occupy little space" rule).

---

## 9. Testing server-rendered HTML in Rust

- **Today:** tests send requests through the in-process router (`TestApp`, `get`, `post_form`) and assert with `html.contains(r#"value="x" checked><span>Bedroom"#)` etc. These bind tests to exact attribute order and whitespace, so M7b's markup rewrite will break many for no behavioural reason.
- **`scraper` 0.27.0** (ISC): parses HTML with html5ever and queries with CSS selectors: `Html::parse_document(&html)`, `Selector::parse("h1.foo")` [63]. Dev-dependency only, so no effect on the binary.
- **`insta`**: `assert_snapshot!`, `cargo insta review`; new snapshots are not written in CI and `INSTA_UPDATE` resolves to `no` there [64].

**Recommendation:**
1. Add `crates/web/tests/common/page.rs`: `Page::parse(&str)` with `text(selector)`, `attr(selector, name)`, `exists`, `all(selector)`. Assert on **semantics**: `form[action="/log"] button[type=submit]` text, `fieldset.levels legend`, `input[name=room][checked]`, `[aria-current=page]` - not on class lists or whitespace.
2. One **global invariant test** over every page the router can render (loop through the existing fixtures): `[style]`, `[onclick]`/`[on*]`, `script:not([src])`, `style` elements, `link[href^=http]` must not exist (spec §9 "CSP still `'self'`"); every `<img>` has `alt`; every `input` has a label (`label[for]` or wrapping); every `use[href]` id exists in the sprite; every `<dialog>` has `aria-labelledby`.
3. `insta` only for **component fragments** (render one macro through a tiny `#[derive(Template)]` test struct) where the exact markup is the contract (chip, collapsible card), as inline snapshots.

```rust
let page = Page::parse(&body);
assert_eq!(page.text("legend"), "Level in Bedroom");
assert!(page.exists(r#"input[name=room][checked]"#));
assert!(!page.exists("[style], [onclick], script:not([src])"));
```

**Alternative:** whole-page `insta` snapshots: one reviewable diff per page, but a refresh milestone changes every snapshot at once, so reviewers accept blindly (Judgement). Stay with structural assertions.

---

## Open questions for the user

1. **iOS/Android floor.** Which iOS versions do the two phones run? If 17.5+: use `light-dark()` and nesting. If 17.2: nesting yes, `light-dark()` no. If lower: neither (this document's default).
2. **Add Node to CI?** Needed only for stylelint and for the Playwright+axe job (Biome and djLint do not). Is a small `tools/` and `tests/e2e/` folder with a committed `package-lock.json` (no `node_modules`) acceptable?
3. **Seed data for the browser job.** A dev-only `--demo-data` flag or script would be a code addition outside the spec; OK, or should the axe job seed through the HTTP API with `curl`?
4. **Avatar-menu behaviour**: is "closes on outside click and Escape" (small JS) wanted, or plain disclosure only?
5. **Person filter**: dialog + search (recommended) or keep a plain `<select>` and drop the search from the spec (§7)?
6. **Language switch in Settings**: submit-buttons (as today) or radios (needs an extra Save)?
7. **Add `--control-border`** to the spec's token table (the design spec needs your approval before I edit it; CLAUDE.md forbids changing your context files without approval).
8. **Several CSS files** with `concat!` in `assets.rs` (recommended) or one file?
9. **Visual regression**: skip (recommended) or add Chromium-linux baselines later?

---

## Sources (all checked 2026-10-14)

1. Askama book, template syntax: https://askama.rs/en/latest/template_syntax.html
2. Askama 0.16.1 crate source (`askama`, `askama_derive`, `askama_parser` 0.16.1), local cargo registry copy of crates.io
3. caniuse, CSS cascade layers: https://caniuse.com/css-cascade-layers
4. caniuse, CSS nesting: https://caniuse.com/css-nesting
5. caniuse, `:has()`: https://caniuse.com/css-has
6. caniuse, container queries: https://caniuse.com/css-container-queries
7. caniuse, `light-dark()`: https://caniuse.com/mdn-css_types_color_light-dark
8. MDN, `light-dark()`: https://developer.mozilla.org/en-US/docs/Web/CSS/color_value/light-dark
9. MDN, `@layer`: https://developer.mozilla.org/en-US/docs/Web/CSS/@layer
10. MDN, `@import`: https://developer.mozilla.org/en-US/docs/Web/CSS/@import
11. MDN, Using CSS nesting: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_nesting/Using_CSS_nesting
12. MDN, CSS logical properties and values: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_logical_properties_and_values
13. MDN, Container queries: https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_containment/Container_queries
14. MDN, `:has()`: https://developer.mozilla.org/en-US/docs/Web/CSS/:has
15. MDN, `prefers-color-scheme`: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-color-scheme
16. CUBE CSS: https://cube.fyi/
17. (BEM official site: en.bem.info naming-convention 404 and getbem.com unreachable on this date; not used for claims)
18. htmx API, `htmx.onLoad`: https://htmx.org/api/#onLoad
19. htmx docs, third-party initialisation and `hx-boost`: https://htmx.org/docs/#3rd-party
20. htmx events, `htmx:load`: https://htmx.org/events/#htmx:load
21. MDN, Using custom elements: https://developer.mozilla.org/en-US/docs/Web/API/Web_components/Using_custom_elements
22. MDN, JavaScript modules: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Modules
23. caniuse, JS modules: https://caniuse.com/es6-module
24. WAI-ARIA APG, menu button: https://www.w3.org/WAI/ARIA/apg/patterns/menu-button/
25. WAI-ARIA APG, disclosure navigation example: https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/examples/disclosure-navigation/
26. WAI-ARIA APG, disclosure: https://www.w3.org/WAI/ARIA/apg/patterns/disclosure/
27. WAI-ARIA APG, combobox: https://www.w3.org/WAI/ARIA/apg/patterns/combobox/
28. WAI-ARIA APG, modal dialog: https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/
29. WAI-ARIA APG, radio group: https://www.w3.org/WAI/ARIA/apg/patterns/radio/
30. WAI-ARIA APG, slider: https://www.w3.org/WAI/ARIA/apg/patterns/slider/
31. WAI-ARIA APG, tabs: https://www.w3.org/WAI/ARIA/apg/patterns/tabs/
32. WAI-ARIA APG, read me first: https://www.w3.org/WAI/ARIA/apg/practices/read-me-first/
33. W3C, Understanding SC 2.5.8 Target Size (Minimum): https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
34. W3C, WCAG 2.2: https://www.w3.org/TR/WCAG22/ (1.4.3, 1.4.11, 2.4.7, 2.4.11, 1.1.1, 2.5.5)
35. MDN, `<details>`: https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/details
36. MDN, `<dialog>`: https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/dialog
37. caniuse, `details` `name` attribute: https://caniuse.com/mdn-html_elements_details_name
38. caniuse, `<dialog>`: https://caniuse.com/dialog
39. MDN, SVG `<use>`: https://developer.mozilla.org/en-US/docs/Web/SVG/Reference/Element/use
40. Chart.js, colors: https://www.chartjs.org/docs/latest/general/colors.html
41. Chart.js, updating charts: https://www.chartjs.org/docs/latest/developers/updates.html
42. Chart.js, accessibility: https://www.chartjs.org/docs/latest/general/accessibility.html
43. stylelint, getting started: https://stylelint.io/user-guide/get-started/
44. stylelint releases (17.16.0): https://github.com/stylelint/stylelint/releases
45. Biome, getting started: https://biomejs.dev/guides/getting-started/
46. Biome, language support: https://biomejs.dev/internals/language-support/
47. Biome releases (2.5.15): https://github.com/biomejs/biome/releases/latest
48. Biome, manual installation (standalone binary): https://biomejs.dev/guides/manual-installation/
49. Biome, continuous integration recipe: https://biomejs.dev/recipes/continuous-integration/
50. djLint: https://www.djlint.com/ and https://djlint.com/docs/usage/
51. djLint releases (1.46.4): https://github.com/djlint/djLint/releases
52. djLint, linter rules: https://djlint.com/docs/linter/
53. Playwright, accessibility testing: https://playwright.dev/docs/accessibility-testing
54. Playwright, visual comparisons: https://playwright.dev/docs/test-snapshots
55. Playwright, CI intro (GitHub Actions): https://playwright.dev/docs/ci-intro
56. Playwright, browsers: https://playwright.dev/docs/browsers (WebKit differs from branded Safari; example sizes Chromium ~281M, WebKit ~180M, Firefox ~187M)
57. axe-core README: https://github.com/dequelabs/axe-core
58. axe-core API, tags: https://github.com/dequelabs/axe-core/blob/develop/doc/API.md
59. pa11y-ci: https://github.com/pa11y/pa11y-ci
60. Lighthouse CI, getting started: https://github.com/GoogleChrome/lighthouse-ci/blob/main/docs/getting-started.md
61. Lighthouse CI, configuration: https://github.com/GoogleChrome/lighthouse-ci/blob/main/docs/configuration.md
62. GitHub Docs, billing for Actions: https://docs.github.com/en/billing/managing-billing-for-your-products/managing-billing-for-github-actions/about-billing-for-github-actions
63. scraper 0.27.0: https://docs.rs/scraper/latest/scraper/
64. insta, quickstart: https://insta.rs/docs/quickstart/
65. MDN, `prefers-reduced-motion`: https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion
66. MDN, `env()` (safe-area insets; `viewport-fit=cover` is from memory): https://developer.mozilla.org/en-US/docs/Web/CSS/env
67. Chrome for Developers, Lighthouse overview (categories listed: Performance, Accessibility, Best Practices, SEO, Agentic browsing): https://developer.chrome.com/docs/lighthouse/overview
68. Chrome for Developers, critical request chains: https://developer.chrome.com/docs/lighthouse/performance/critical-request-chains
69. caniuse, `:focus-visible`: https://caniuse.com/css-focus-visible
70. WAI-ARIA APG, button (toggle button): https://www.w3.org/WAI/ARIA/apg/patterns/button/

*Notes on 4.5: the toggle-button citation is [70]; the in-text "[70]" means source 70.*
*Safe-area (spec §3.3 "safe-area aware"): `env(safe-area-inset-bottom)` for the fixed tab bar and `viewport-fit=cover` in the viewport meta [66]; add to base.html when the tab bar lands.*
