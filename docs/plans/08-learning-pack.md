# Milestone 8: Learning pack — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Someone who is learning Rust can learn it from this project: six notes in
`docs/learning/` (one per area of the app, each pointing at the exact code, explaining why it was
written that way and what else was possible), an index with a reading order, and 17 optional
exercises (`todo!()` stubs with tests, kept outside the app) that practise the same concepts.

**Architecture:** Documentation and a standalone practice package only. **Zero application code
changes**: nothing under `crates/`, no `Cargo.toml` or `Cargo.lock` of the workspace, no
migration, no `.sqlx` file. The notes live in `docs/learning/NN-topic.md` (next to the existing
`hosting.md`). The exercises live in **`exercises/`**: a package with its own empty `[workspace]`
table and no dependencies, so the app's workspace (`members = ["crates/*"]`) never contains it
and the app's build, tests, clippy, coverage, Docker image and deploy are untouched.

**Tech Stack:** Markdown; Rust (edition 2024, standard library only) for the exercises. No new
crate, no new tool, no new workflow.

**Spec:** Decision #43 in `docs/DECISIONS.md` and the roadmap row 8
(`docs/plans/ROADMAP.md`). The design spec
`docs/specs/2026-10-13-smoke-tracker-design.md` is background only and is never edited; the
notes quote its section numbers where they explain a rule (§4 permissions, §5 intensity guide,
§6 audit, §7 events, §8 sign-in, §9 screens, §12 security headers, §13 CI).

**Provenance:** the 17 exercise stubs and 17 test files in this plan were written and compiled
before the plan was written, in a scratch package outside git, built with the shared
`CARGO_TARGET_DIR`. Results: with the stubs, `cargo test --no-fail-fast` compiles with no
warning and **0 of 166 tests pass** (165 fail with "not yet implemented", one, the `ALL` check of
Exercise 3, fails an assertion because its stub returns an empty list; all 17 test files fail);
with a reference solution kept in the planner's scratchpad (never in git), **166 of 166 pass**.
`cargo fmt --check` passes for both. With `exercises/` present in the worktree, `cargo metadata
--no-deps` at the repository root still lists exactly the six app packages. The code references
in this plan (317 of them) were checked with the reference checker under "Verification tools",
run over this file after it was written (0 failed), and the 116 documentation links in the
concept lists were fetched and all answered 200 (book pages were also checked for their page
titles). **Not verified locally:** the `deploy.yml` and `codeql.yml` edits (no `actionlint`, no
GitHub); the first pull-request run verifies them. **Found while planning:** a build-cache
trap, recorded in Global Constraints (two same-named packages in one shared target directory
share a fingerprint, so the reference run once executed the stubs).

## Decisions for the user to confirm

Defaults were chosen so nothing blocks; each is a decision row (#131–#134, appended by Task 1).

1. **Notes point at code by path and symbol, never by line number** (decision #131). Line
   numbers rot with every edit; `crates/domain/src/types.rs::Level` survives. The notes say
   which commit they were written against (`5b52604`).
2. **The exercises are a standalone package `exercises/`** with its own `[workspace]` table,
   standard library only (decision #132). Why not an excluded workspace member: every `--workspace`
   command CI runs (`fmt`, `clippy`, `test`, `llvm-cov`) would still include it, and its
   failing stubs would turn CI red.
3. **No CI job builds the exercises** (decision #132). They are compiled by whoever runs them.
   `deploy.yml` and `codeql.yml` ignore `exercises/**`, and `.dockerignore` lists `exercises`.
4. **Tests are separate files the learner does not edit** (decision #133), and no solution is
   ever committed.
5. **Walkthroughs and quizzes are chat requests**, not tooling (decision #134).
6. **Difficulty:** 17 exercises, about 20 to 60 minutes each; the index marks each one (one to
   three stars) so the learner can skip the hard ones.

## Questions for the user

Sources (spec, decisions, code, the agreed learning format) do not settle these; the plan takes
the smallest option and keeps the question open.

1. **Should CI compile the exercises?** A one-line job `cargo test --no-run` in `exercises/`
   would catch the stubs rotting when the Rust edition or toolchain changes, at the price of
   about a minute of CI per run and a new job. Plan: **no CI job**; the exercises are checked
   when someone runs them (by the milestone reviewers now, by you later). Adding it later is
   one job and does not change anything else.
2. **Should the root `README.md` link to the learning pack?** The README is the portfolio front
   page and has a test that pins its tables. Plan: **leave it unchanged**; the pack is
   reachable from `docs/learning/README.md` and from the roadmap. A link would be one line in
   its "Development" or "Hosting" neighbourhood.
3. **Italian and Turkish versions of the notes?** The app is trilingual; the notes are
   English only (you are learning Rust in English-language docs). Plan: English only.
4. **Should the notes be reviewed by you before they count as done?** Plan: reviewed by
   subagents for accuracy of every reference and link; you are the judge of whether the
   explanations teach well. The index tells you how to ask Claude to rewrite a section.

## Global Constraints

- Personal project. Before every commit, `git config user.name` must print `locadani` and
  `git config user.email` must print locadani's GitHub `…@users.noreply.github.com` address (the
  repository's local config already holds it; copy it from there, never retype it into a file). **Never write any email address in a tracked
  file** (notes, exercises and plan included): the pre-commit gitleaks hook (`git config
  core.hooksPath .githooks`) must be active; if it is missing, activate it before committing.
- Never commit secrets or tokens. The exercises' test data are made-up words and numbers only.
- Never modify `description.md`, `CLAUDE.md` or `claude.md`. **Never edit
  `docs/specs/2026-10-13-smoke-tracker-design.md`.** No task touches either.
- Every commit message ends with a blank line followed by exactly
  `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Verify each commit with
  `git log -1 --format='%an%n%B'` (the author line must read `locadani`; subagents tend to copy
  their own model name into the trailer).
- Commit subjects use Conventional Commits and name the Rust concept where there is one, for
  example `docs(learning): note 02, traits as the storage boundary and ownership in transactions`.
- **No application code change.** `git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock
  Dockerfile rust-toolchain.toml rustfmt.toml deny.toml scripts infra` must print nothing at the
  end of every task. **No tutorial comments in the application code**; all teaching lives in
  `docs/learning/` and in `exercises/`. If writing a note reveals a real bug in the app, do
  **not** fix it and do not write it into a note: describe it in the task report (file, symbol,
  what goes wrong) and the orchestrator reports it to the user.
- The only files outside `docs/learning/`, `exercises/` and this plan that this milestone
  changes are `docs/DECISIONS.md` (append only), `docs/plans/ROADMAP.md` (row 8),
  `.dockerignore`, `.github/workflows/deploy.yml` and `.github/workflows/codeql.yml` (each one
  `exercises` ignore line, Task 2).
- `docs/DECISIONS.md` is append-only: this plan adds rows **#131–#134** (the last row on the
  branch base is #130). If the `milestone-7b` branch merges first and also uses these numbers,
  whichever merges second renumbers (note it in the summary).
- **Notes are written for someone learning Rust**, not an expert: plain English, short
  sentences, every term explained the first time it appears, no emojis. Every Rust statement
  must be true for Rust 1.94+ edition 2024 and checkable against the linked documentation.
  Quote no more than a few lines of our code; point at it instead.
- **Code references use one exact format** so they can be checked: a backticked token
  `` `crates/domain/src/types.rs::Level` `` (path, `::`, symbol), or for a member
  `` `crates/domain/src/records.rs::ReportValues::new` ``; a bare path
  `` `crates/web/src/lib.rs` `` is allowed for a whole file; a folder ends with `/`. Never a line
  number, never a commit permalink. Everything else in backticks (types from the std library,
  crate names, shell commands) must not start with `crates/`, `docs/`, `.github/`, `scripts/`,
  `infra/` or `exercises/`.
- **External links** only to: `doc.rust-lang.org` (Book, std, Reference, Cargo, Rust by
  Example), `rust-lang.github.io`, `blog.rust-lang.org`, `docs.rs`, `serde.rs`, `tokio.rs`,
  `htmx.org`, `projectfluent.org`, `www.chartjs.org`, `developer.mozilla.org`,
  `cheatsheetseries.owasp.org`, `www.sqlite.org`, `www.postgresql.org`,
  `proptest-rs.github.io`, `github.com/taiki-e/cargo-llvm-cov`, and the three articles named
  in the concept lists. Every address used must have answered 200 when checked with the URL
  checker below. Never invent an address; if unsure, link the crate's `docs.rs/<crate>` page.
- Exercises: standard library only (no `[dependencies]`), `unsafe_code` is irrelevant (there is
  none), `cargo fmt --check` clean, **no warning** when compiled (the crate-level
  `#![allow(unused_variables, unused_imports, unused_mut, dead_code)]` in `lib.rs` is what keeps
  the stubs quiet). Every test fails on the stub. Stubs contain `todo!()` bodies and signatures
  and doc comments only; **no solution, no partial solution, no complete algorithm in a hint**.
- Builds: ONE shared target dir for every agent: `CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target`.
  Never create another target folder, never run `cargo clean` or `rm -rf` on a build folder
  (the user does that). Check `df -h /System/Volumes/Data` before the first build of a session;
  below 20 GB free, stop at a task boundary and report. The exercises compile in seconds.
- **The reference crate must not be named like the learner's crate.** The reference solution
  crate in the scratchpad is the package `smoke-exercises-reference` with `[lib] name =
  "smoke_exercises"`. Two packages with the same package name in one shared target directory
  share a build fingerprint, and Cargo then silently links the wrong one (this happened while
  planning: the reference run executed the stubs).
- The Postgres container is not needed (no task runs the app's test suite until the final
  verification; if it is run, `TEST_POSTGRES_URL=postgres://postgres@localhost:55432/postgres`).

## Review Focus

1. **A note that points at code that is not there**: a renamed or moved symbol, a path typo, a
   claim ("it uses `Mutex`") that the code does not bear out. Expected: every reference passes
   the reference checker and the reviewer opens each cited symbol and confirms the sentence
   about it. Check: `checkrefs.py` over the note (Tasks 3, 8, 12, 16, 20, 23, 26).
2. **A link that does not exist** or points to a different page than the sentence claims.
   Expected: every address answers 200 and the reviewer reads the link text against the page
   title. Check: `checkurls.sh` over the note's links (same tasks).
3. **The exercises reaching the app**: `exercises/` becoming a workspace member, a `--workspace`
   command compiling it, a Docker context or deploy trigger including it, coverage or CodeQL
   scanning stubs. Expected: root `cargo metadata` lists the six app packages only; the three
   ignore lines exist. Check: Task 2 and Task 27 commands.
4. **A test that does not test the learner**: a test passing on the stub, a test that cannot
   fail, a test that does not compile, a stub that warns, or a hint that gives the answer away.
   Expected: 0 passing tests on the stubs, all passing on the reference solution. Check: the
   run in every exercise task, and Task 27.
5. **A solution, a secret or an email in git**: a reference solution file committed, a
   personal address in a note, a token-looking string. Expected: none. Check: Task 27 greps and
   the gitleaks hook.
6. **A wrong or misleading Rust explanation** (for example saying `Mutex::lock` fails when
   another thread *holds* the lock, or that `Box<dyn Trait>` is always slower, or that `async
   fn` in traits needs the `async-trait` crate). Expected: each explanation is true and says
   what it is true for. Check: the reviewer compares every Rust claim against the linked
   documentation.
7. **A note that was true for 5b52604 and is false at merge time** (another branch changed the
   code). Expected: the "written against" line names the commit; stale symbols are found by
   re-running the reference checker before the branch is finished (Task 27).

## Out of scope (recorded so nobody hunts for it)

- **Any change to the app**, including bugs the notes might make visible (reported, not fixed),
  deferred M5/M6 minors, and the milestone 7b work.
- A rendered site (mdBook, GitHub Pages), generated API docs, a CI link or reference checker
  (decision #134), Italian or Turkish notes, video or slides.
- Solutions to the exercises. The learner writes them; Claude reviews them in chat on request.
- Exercises that need a database, a network, an async runtime or a third-party crate.

## Verification tools

Two small tools check the notes. They are **not** committed to the repository (decision
#134); the orchestrator saves them once in the scratchpad (for example
`<scratchpad>/tools/checkrefs.py` and `checkurls.sh`) and every task's reviewer runs them.

`checkrefs.py` (needs only the Python standard library; run with `python3 -I`):

```python
"""Checks the code references in a Markdown file against the repository.

Usage: python3 -I checkrefs.py REPO_ROOT MARKDOWN_FILE

A reference is a backticked token that starts with a repository path:
  `crates/domain/src/types.rs`                    the file must exist
  `crates/domain/src/types.rs::Level`             ...and define (fn/struct/enum/trait/const/static/
                                                   type/mod/macro_rules!) the item `Level`
  `crates/domain/src/records.rs::ReportValues::new`  ...the type must be defined and the member
                                                   name must appear in the file
(An item defined inside a macro call, such as `str_enum!( ... AuditAction { ... } )`, counts as
defined when its name starts an indented line followed by `{`.) `--allow-new` accepts missing
files and folders under `exercises/` and `docs/learning/` (used only when checking the plan).
For files that are not Rust (html, js, ftl, sql, toml, yml, css), the text after `::` must appear
in the file literally. A directory path (`crates/web/templates/`) must exist as a directory.
Exit status 1 when any reference fails.
"""

import re
import sys
from pathlib import Path

root = Path(sys.argv[1])
text = Path(sys.argv[2]).read_text()
roots = ("crates/", ".github/", ".githooks/", ".cargo/", "docs/", "scripts/", "infra/", "exercises/")
extra_files = ("Dockerfile", "Cargo.toml", ".dockerignore", ".gitignore", ".gitleaks.toml", "README.md", "deny.toml", "rustfmt.toml", "rust-toolchain.toml")
tokens = sorted(set(re.findall(r"`([^`\n]+)`", text)))
failures = []
checked = 0
for token in tokens:
    path_part, _, symbol = token.partition("::")
    if not (path_part.startswith(roots) or path_part in extra_files):
        continue
    if " " in path_part or "*" in path_part or "<" in path_part or "{" in path_part:
        continue
    checked += 1
    path = root / path_part
    if path_part.endswith("/"):
        if not path.is_dir() and not ("--allow-new" in sys.argv and path_part.startswith(("exercises/", "docs/learning/"))):
            failures.append(f"{token}: no such directory")
        continue
    if not path.is_file():
        # Files the plan itself creates are listed as `NEW path` in tasks; accept them when
        # the caller passes --allow-new.
        if "--allow-new" in sys.argv and (path_part.startswith("exercises/") or path_part.startswith("docs/learning/") or path_part.startswith("docs/plans/08")):
            continue
        failures.append(f"{token}: no such file")
        continue
    if not symbol:
        continue
    source = path.read_text()
    parts = symbol.split("::")
    if path.suffix == ".rs":
        head = parts[0]
        defined = re.search(
            rf"\b(?:fn|struct|enum|trait|const|static|type|mod|union)\s+{re.escape(head)}\b"
            rf"|macro_rules!\s+{re.escape(head)}\b"
            rf"|^\s+{re.escape(head)}\s*\{{",
            source,
            re.MULTILINE,
        )
        if not defined:
            failures.append(f"{token}: `{head}` is not defined in {path_part}")
        elif len(parts) > 1 and not re.search(rf"\b{re.escape(parts[-1])}\b", source):
            failures.append(f"{token}: `{parts[-1]}` does not appear in {path_part}")
    elif symbol not in source:
        failures.append(f"{token}: `{symbol}` does not appear in {path_part}")
print(f"{checked} references checked, {len(failures)} failed")
for failure in failures:
    print("  FAIL", failure)
sys.exit(1 if failures else 0)
```

`checkurls.sh` (needs `curl`; prints every address that does not answer 200):

```bash
#!/bin/sh
# Usage: checkurls.sh FILE  (one URL per line); prints every URL that does not answer 200
# (after one retry; the servers occasionally drop a request under parallel load).
grep -E '^https?://' "$1" | sort -u | xargs -n1 -P4 sh -c '
for try in 1 2; do
  c=$(curl -sL -m 25 -o /dev/null -w "%{http_code}" "$0")
  [ "$c" = 200 ] && exit 0
  sleep 1
done
echo "${c:-ERR} $0"'
echo "checked $(grep -cE '^https?://' "$1") urls"
```

Usage (run from the repository root of the worktree):

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/01-foundation-and-domain.md
grep -oE 'https?://[^ )>`"*]+' docs/learning/01-foundation-and-domain.md | sed 's/[.,;]$//' \
  | sort -u > <scratchpad>/urls-01.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-01.txt
```

The URL list goes into the scratchpad, never into the repository. Both commands must end with
`0 failed` and `checked N urls` with nothing listed above it.

The reference solution lives in the scratchpad as the package
`smoke-exercises-reference` (`<scratchpad>/exercises-reference/`: `Cargo.toml`, `src/`,
`tests/`). It is kept **out of git**. To check an exercise task against it:

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
REF=<scratchpad>/exercises-reference
cp exercises/tests/ex01_newtypes.rs "$REF/tests/"     # the test file of the task
(cd "$REF" && cargo test --test ex01_newtypes)        # expected: all pass
```

If `$REF` is missing (the scratchpad is gone), the implementer writes their own solution in a
new scratch folder (package name `smoke-exercises-reference`, `[lib] name = "smoke_exercises"`,
empty `[workspace]`), proves the tests pass against it, and keeps it out of git.

## File Structure

```
smoke-tracker/
├─ .dockerignore                           + `exercises`                       (Task 2)
├─ .github/workflows/deploy.yml            + `exercises/**` in paths-ignore    (Task 2)
├─ .github/workflows/codeql.yml            + `exercises/**` in paths-ignore    (Task 2)
├─ docs/
│  ├─ DECISIONS.md                         + rows #131–#134                    (Task 1)
│  ├─ plans/ROADMAP.md                     row 8 links this plan               (Task 1)
│  ├─ plans/08-learning-pack.md            this plan
│  └─ learning/
│     ├─ README.md                         NEW  index, reading order, how to run exercises
│     ├─ 01-foundation-and-domain.md       NEW  workspace, newtypes, enums, macros, audit chain
│     ├─ 02-storage.md                     NEW  traits, async in traits, errors, sqlx, contract tests
│     ├─ 03-web-and-auth.md                NEW  axum, extractors, middleware, state, sessions, i18n
│     ├─ 04-logging-flow.md                NEW  use cases as generics, pure rules, forms, lifetimes
│     ├─ 05-people-and-pwa.md              NEW  invites, secrets in types, QR, service worker, ETags
│     ├─ 06-graphs.md                      NEW  pure series, BTreeMap, iterators, Chart.js, CSP
│     └─ hosting.md                        existing, linked from the index, unchanged
└─ exercises/                              NEW  standalone package, not a workspace member
   ├─ Cargo.toml                           package smoke-exercises, empty [workspace], no deps
   ├─ .gitignore                           /target and /Cargo.lock
   ├─ src/lib.rs                           one `pub mod` per exercise
   ├─ src/ex01_newtypes.rs                 Level, Username, Password (stubs)         note 01
   ├─ src/ex02_permissions.rs              Role, Actor, Action, can, can_see         note 01
   ├─ src/ex03_text_enum.rs                the `text_enum!` macro                    note 01
   ├─ src/ex04_audit_chain.rs              hash chain: append, build, verify         note 01
   ├─ src/ex05_event_repo.rs               EventRepo trait, MemoryEvents, generics   note 02
   ├─ src/ex06_transaction.rs              Ledger and Tx: commit, drop = rollback    note 02
   ├─ src/ex07_store_errors.rs             StoreError, From, `?`, collect to Result  note 02
   ├─ src/ex08_login_limiter.rs            Limiter, SharedLimiter (HashMap, Mutex)   note 03
   ├─ src/ex09_request_rules.rs            negotiate, same-origin, client_ip, paths  note 03
   ├─ src/ex10_middleware.rs               closures as middleware, error chain       note 03
   ├─ src/ex11_log_prompt.rs               classify_observed, log_prompt, boundaries note 04
   ├─ src/ex12_report_values.rs            ReportValues::new, intensity              note 04
   ├─ src/ex13_form_pairs.rs               Pairs: get, ids, impl Iterator + 'a       note 04
   ├─ src/ex14_expiry_pick.rs              ExpiryPick::resolve, check_expiry         note 05
   ├─ src/ex15_pwa_assets.rs               etag_of, respond, network_first, svg      note 05
   ├─ src/ex16_buckets.rs                  Period, buckets, events_per_bucket        note 06
   ├─ src/ex17_series.rs                   intensity_lines, room_hours, tallies      note 06
   └─ tests/ex01_newtypes.rs … ex17_series.rs   one test file per exercise (166 tests)
```

**Why this order:** the decisions come first so their numbers are fixed (Task 1); the exercise
package and the three isolation edits next, with checks that the app is untouched (Task 2);
then, per area, the note first and its exercises after it (the exercises link back to the
note, so the note must exist), strictly in the order of the app's milestones; the index last,
because it lists everything (Task 26); a whole-pack verification closes the milestone
(Task 27).

**Task sizing:** one note, or one exercise, per task. A reviewer can reject an exercise whose
tests pass on the stub, or a note with a dead link, without touching its neighbours.

---


### Task 1: Decisions #131–#134 and the roadmap row

**Files:**
- Modify: `docs/DECISIONS.md` (append four rows after #130)
- Modify: `docs/plans/ROADMAP.md` (row 8)

**Interfaces:**
- Consumes: nothing.
- Produces: the decision numbers **#131 (notes format), #132 (exercises location and isolation),
  #133 (exercise format, tests, no solutions), #134 (walkthroughs in chat, no checker)** that the
  notes, the index and the isolation edits refer to.

- [ ] **Step 1: Verify the last decision is #130**

```bash
tail -n 1 docs/DECISIONS.md | cut -c1-40     # starts with: | 130 | 2026-10-14 | Docs |
grep -c '^| 13[1-9] ' docs/DECISIONS.md      # 0
```

If a row #131 or higher already exists, stop and report: another branch has used the numbers.

- [ ] **Step 2: Append the four rows**

Use the Edit tool on `docs/DECISIONS.md`: `old_string` is the end of row 130,
`without a generator to maintain |`, and `new_string` is that same text followed by a newline and
these four lines (no blank line between rows; the file must still end with one newline):

```markdown
| 131 | 2026-10-14 | Learning | The learning pack is **`docs/learning/NN-topic.md`**, one note per area (01 foundation and domain, 02 storage, 03 web and auth, 04 logging flow, 05 people and PWA, 06 graphs) plus an index `README.md`; `hosting.md` stays. A note points at our code by **file path and symbol name** (`crates/domain/src/types.rs::Level`), never by line number, says which commit it was written against, and links only to the Rust Book, the standard library, crate documentation and web-standards pages | Line numbers or permalinks into one commit; teaching comments inside the code; a generated book (mdBook) | Symbols survive edits and line numbers do not; teaching stays out of the portfolio code (#43); plain Markdown reads on GitHub with no tooling and no cost |
| 132 | 2026-10-14 | Learning | The **exercises** are a separate package in **`exercises/`**: its own `Cargo.toml` with an empty `[workspace]` table, so the app's `members = ["crates/*"]` workspace never contains it; **no dependencies** (standard library only), a git-ignored `target/` and `Cargo.lock`. `.dockerignore` lists `exercises`, and `deploy.yml` and `codeql.yml` ignore `exercises/**`. **No CI job builds or tests it** | A workspace member left out with `default-members` or `exclude`; a feature-gated module inside an app crate; a second repository; a CI job that only compiles it | A member still counts for every `--workspace` command CI runs (fmt, clippy, test, llvm-cov), and its failing stubs would turn them red; a separate package changes nothing about the app's build, image, coverage or lints; no dependencies means nothing for Dependabot, cargo-deny or Trivy; a compile-only CI job is one line to add later if the stubs rot |
| 133 | 2026-10-14 | Learning | Each exercise is `src/exNN_name.rs` (signatures and `todo!()` bodies, with a doc comment naming the note it practises, the app code it mirrors, the command to run it and a hint) plus `tests/exNN_name.rs`, which the learner does not edit; `cargo test --test exNN_name` checks a solution. Claude writes the stubs and tests; **the learner writes the code**. Every test must fail on the stub and pass on a reference solution, which is **never committed** | Tests inside the stub files; doc tests; solutions on another branch or in the repository | Integration tests see only the public signatures, so the stub is the whole contract and a test cannot be weakened by accident; a test that fails on the stub proves it checks something; a committed solution would end the exercise |
| 134 | 2026-10-14 | Learning | Walkthroughs, quizzes and reviews of the learner's solutions are **requests in chat** (the index shows how to phrase them); nothing is built for them. The notes' code references and links are checked by the reviewers of this milestone, **not by CI** | A CI reference and link checker; a generated study plan | One learner, one repository: a checker costs CI minutes and maintenance for a pack that is read occasionally; chat works against the current code |
```

- [ ] **Step 3: Update the roadmap row**

Use the Edit tool on `docs/plans/ROADMAP.md`. Replace the whole row 8 line

`| 8 | **Learning pack** | `docs/learning/` notes per milestone, optional exercises with tests, walkthroughs | Decision #43 | — |`

with

`| 8 | **Learning pack** | `docs/learning/` notes per area with an index, 17 optional exercises with tests in `exercises/` (outside the app), walkthroughs and quizzes on request in chat | Decision #43 | `08-learning-pack.md` |`

- [ ] **Step 4: Check**

```bash
grep -c '^| 13[1-4] ' docs/DECISIONS.md                                   # 4
tail -c 1 docs/DECISIONS.md | xxd -p                                      # 0a (ends with a newline)
grep -n '^| 8 |' docs/plans/ROADMAP.md                                    # shows 08-learning-pack.md
grep -c '@' docs/DECISIONS.md docs/plans/ROADMAP.md                       # same counts as before (no new address)
git diff --stat                                                           # only the two files
```

- [ ] **Step 5: Commit**

```bash
git config user.email                       # locadani's GitHub noreply address
git add docs/DECISIONS.md docs/plans/ROADMAP.md
git commit -m "docs: decisions #131-#134 for the learning pack, roadmap row 8 points at its plan

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 2: The exercises package, kept out of the app

**Files:**
- Create: `exercises/Cargo.toml`, `exercises/.gitignore`, `exercises/src/lib.rs`
- Modify: `.dockerignore` (one line), `.github/workflows/deploy.yml` (one line),
  `.github/workflows/codeql.yml` (one line)

**Interfaces:**
- Consumes: decision #132 (Task 1).
- Produces: an empty, compiling package `smoke-exercises` whose `src/lib.rs` the exercise
  tasks extend with one `pub mod exNN_name;` line each, in numeric order, and the isolation
  guarantees that Task 27 re-checks.

- [ ] **Step 1: Check the base state**

```bash
git status --short                                   # clean
test ! -e exercises && echo absent
df -h /System/Volumes/Data | tail -1                 # at least 20 GB free
cat .dockerignore | head -20
sed -n 1,14p .github/workflows/deploy.yml
sed -n 28,40p .github/workflows/codeql.yml
```

- [ ] **Step 2: Create `exercises/Cargo.toml`**

```toml
[package]
name = "smoke-exercises"
version = "0.1.0"
description = "Rust exercises that go with docs/learning. Not part of the app."
edition = "2024"
rust-version = "1.94"
license = "MIT"
publish = false

# An empty table makes this folder its own workspace root, so Cargo never treats it as a member
# of the app's workspace one level up, and the app's builds, tests and lints never see it.
[workspace]
```

No `[dependencies]` section, ever (decision #132).

- [ ] **Step 3: Create `exercises/.gitignore`**

```
/target
/Cargo.lock
```

- [ ] **Step 4: Create `exercises/src/lib.rs`**

```rust
//! Exercises that go with the notes in `docs/learning/`. Every function body is `todo!()`: write
//! it, then run the matching test file with `cargo test --test <name>`. Remove the `allow` below
//! once everything is solved.
#![allow(unused_variables, unused_imports, unused_mut, dead_code)]
```

(The exercise tasks append the `pub mod` lines below the `allow` line, with one blank line
between the `allow` and the first module.)

- [ ] **Step 5: Add the three ignore lines**

`.dockerignore`: add the line `exercises` after the line `docs` (keep the file's comment
style; a short comment line `# Practice package, not part of the app.` above it).

`.github/workflows/deploy.yml`: in `on.push.paths-ignore` add `- "exercises/**"` after
`- "docs/**"`. Do **not** touch anything else in the file: it must still run only on `push:
branches: [main]` (`infra/README.md`, "Rules for the deploy workflow").

`.github/workflows/codeql.yml`: in the `config: |` block, under `paths-ignore:` add
`- exercises` after `- crates/web/static/*.min.js` (same indentation as that line).
If `actionlint` is installed run `actionlint .github/workflows/deploy.yml
.github/workflows/codeql.yml`; if not, say so in the task report (the first pull-request run is
the check).

- [ ] **Step 6: Prove the package builds on its own and the app does not see it**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
(cd exercises && cargo test 2>&1 | tail -5)          # compiles; "running 0 tests"
(cd exercises && cargo fmt --check) && echo fmt-ok
cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"
# expected exactly: ['smoke-domain', 'smoke-storage-contract', 'smoke-storage-postgres', 'smoke-storage-sqlite', 'smoke-tracker', 'smoke-web']
cargo fmt --all --check && echo root-fmt-ok          # the app's own formatting check is unaffected
git status --short                                   # lists the three edits and exercises/ files only; no Cargo.lock, no target
git check-ignore -v exercises/Cargo.lock exercises/target # both ignored by exercises/.gitignore
```

- [ ] **Step 7: Commit**

```bash
git config user.email                                # locadani's GitHub noreply address
git add exercises/Cargo.toml exercises/.gitignore exercises/src/lib.rs .dockerignore .github/workflows/deploy.yml .github/workflows/codeql.yml
git commit -m "chore(exercises): standalone practice package outside the app workspace

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
```

---

### Task 3: Note 01 — foundation and domain

**Files:**
- Create: `docs/learning/01-foundation-and-domain.md`

**Interfaces:**
- Consumes: decisions #22, #25, #36, #37, #41, #45, #46, #51, #55, #104 in `docs/DECISIONS.md`;
  the code listed below.
- Produces: the note that Exercises 1 to 4 link back to
  (`docs/learning/01-foundation-and-domain.md`, with the section numbers below).

**Covers:** the Cargo workspace and layering; modules and privacy; newtypes with validating
constructors; derives and hand-written `Debug`; enums and `match`; `macro_rules!`; error types
with `thiserror`; time as a dependency (`Clock`); time zones; the hash-chained audit log; unit
and property tests; formatting, lints and CI.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 01 — Foundation and domain

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 1 (newtypes), 2 (permissions), 3 (macro), 4 (audit chain). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **A Cargo workspace splits the app into crates.**
   - In our code: `Cargo.toml` (root: `[workspace]`, `members = ["crates/*"]`,
     `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints.rust]`
     `unsafe_code = "forbid"`, `[workspace.lints.clippy]`); `crates/domain/Cargo.toml`
     (`[lints] workspace = true`, `chrono.workspace = true`); `crates/web/Cargo.toml`
     (`smoke-domain.workspace = true`, and the storage crate only under `[dev-dependencies]`);
     the dependency direction `smoke-tracker → smoke-web → smoke-domain ← storage-*`.
   - Why: one lockfile and one version per dependency; the pure `domain` crate depends on no
     database or web crate, so its rules run in milliseconds without I/O; Cargo refuses cyclic
     crate dependencies, so the layering is enforced by the compiler, not by habit (#22, #45).
     Alternatives: one crate with modules (faster to start, the layers are only a convention);
     separate repositories (versioning cost for one developer).
   - Read more: Rust Book ch. 14.3 `https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html`,
     Cargo reference `https://doc.rust-lang.org/cargo/reference/workspaces.html`.
2. **Modules and privacy.**
   - In our code: `crates/domain/src/lib.rs` (`pub mod` list and `pub use error::DomainError`);
     `crates/domain/src/types.rs::str_enum` re-exported with `pub(crate) use str_enum;` so other
     files of the crate can use the macro but the outside world cannot;
     `crates/storage-sqlite/src/lib.rs` (private `mod` lines, `pub use crate::tx::SqliteTx`);
     `crates/web/src/lib.rs` (almost everything private, `pub fn router`, `pub fn serve`).
   - Why: privacy is the tool that makes "only this crate can build this value" true (next
     concept). Alternatives: make everything `pub` (anyone can break an invariant); one big file.
   - Read more: Book ch. 7.2
     `https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html`.
3. **Newtypes that cannot be invalid.** (Exercise 1)
   - In our code: `crates/domain/src/types.rs::Level` (private field, `Level::new`,
     `Level::CLEAN`, `TryFrom<u8>`, `From<Level> for u8`, `#[serde(try_from = "u8", into =
     "u8")]` so JSON input is validated too); `crates/domain/src/types.rs::Username` (trim,
     lowercase, 3 to 32 characters, #51); `crates/domain/src/types.rs::ListName`;
     `crates/domain/src/types.rs::NewPassword` (counts `chars()`, not bytes);
     `crates/domain/src/types.rs::Pm25` (`is_finite` so NaN is refused); the id types
     (`UserId(pub i64)` and friends) as plain wrappers with no rule.
   - Why: the check happens once, at the door; everything inside takes a `Level` and never
     re-checks. Alternatives: plain `u8` with checks at each use (one forgotten check is a bug);
     a validation crate (a dependency for a four-line `if`); `assert!` in the constructor (a
     panic is the wrong answer to bad user input, a `Result` lets the page show a message).
   - Read more: Book ch. 5.1 `https://doc.rust-lang.org/book/ch05-01-defining-structs.html`;
     std `TryFrom` `https://doc.rust-lang.org/std/convert/trait.TryFrom.html`; serde container
     attributes `https://serde.rs/container-attrs.html`; the article "Parse, don't validate"
     `https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/`.
   - Try it: Exercise 1 `exercises/src/ex01_newtypes.rs`.
4. **Derives, and a Debug that does not leak.**
   - In our code: the long derive list on `crates/domain/src/types.rs::Level` (`Copy` because it
     is one byte, `Ord` so `Iterator::max` works in
     `crates/domain/src/records.rs::ReportValues::intensity`, `Hash` for map keys);
     `crates/domain/src/types.rs::NewPassword` and `crates/domain/src/types.rs::TokenHash`
     write their own `Debug` (`NewPassword(<redacted>)`, the first 8 characters of a hash) and
     `crates/domain/src/records.rs::PasswordHash` the same, so a stray `{:?}` in a log line
     cannot print a secret.
   - Why: `#[derive(Debug)]` prints every field; for secrets the derive is the wrong tool.
     Alternative: a `Secret<T>` wrapper crate (`secrecy`), worth it when there are many secrets.
   - Read more: std `Debug` `https://doc.rust-lang.org/std/fmt/trait.Debug.html`, std
     `Display` `https://doc.rust-lang.org/std/fmt/trait.Display.html`.
5. **Enums and match: permissions.** (Exercise 2)
   - In our code: `crates/domain/src/permissions.rs::Action` (variants that carry data:
     `EditEntry { owner: UserId }`), `crates/domain/src/permissions.rs::can` (an outer `match` on
     the role, an inner one on the action, no catch-all `_` on the role),
     `crates/domain/src/permissions.rs::can_see`, `crates/domain/src/permissions.rs::check_expiry`
     (matching a tuple with a guard), `crates/domain/src/permissions.rs::Actor`
     (`Option::is_some_and`).
   - Why: the compiler checks that every role and action combination is handled; adding a
     role later is a compile error in the right place. Alternatives: a permissions table in the
     database (more flexible, no compiler help, and spec §4 is fixed); a bag of boolean flags.
   - Read more: Book ch. 6 `https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html` and
     `https://doc.rust-lang.org/book/ch06-02-match.html`; std `Option`
     `https://doc.rust-lang.org/std/option/enum.Option.html`.
   - Try it: Exercise 2 `exercises/src/ex02_permissions.rs`.
6. **Macros: str_enum! and id_type!.** (Exercise 3)
   - In our code: `crates/domain/src/types.rs::id_type` (one macro, seven id types, doc
     comments passed through with `$(#[$meta:meta])*`); `crates/domain/src/types.rs::str_enum`
     (an enum stored as text: `as_str`, `FromStr`, `Display`, serde, `ALL`; `$crate::` paths,
     `stringify!`, the `$(...),+` repetition); used for `Role`, `RoomArea`, `CloseReason`,
     `InvitePurpose`, `Locale` (all in `crates/domain/src/types.rs`) and in
     `crates/domain/src/audit.rs::AuditAction`.
   - Why: the same 40 lines for ten enums would drift; a macro keeps the text form in one place.
     Alternatives: a derive such as `strum` (a dependency, and the stored text must match the
     database `CHECK` lists exactly, which an explicit literal makes visible); a hand-written
     impl per enum. Say honestly that macros are harder to read and to debug, and that a plain
     function or generic is preferred whenever it can do the job.
   - Read more: Reference `https://doc.rust-lang.org/reference/macros-by-example.html`, Rust by
     Example `https://doc.rust-lang.org/rust-by-example/macros.html`, Book ch. 20.5
     `https://doc.rust-lang.org/book/ch20-05-macros.html`.
   - Try it: Exercise 3 `exercises/src/ex03_text_enum.rs`.
7. **Errors: thiserror for libraries, anyhow for the app.**
   - In our code: `crates/domain/src/error.rs::DomainError` (`#[derive(Error)]`, `#[error("...
     {0}")]` messages, a variant with named fields `UnknownValue { kind, value }`);
     `crates/domain/src/logbook.rs::LogError` (`#[error(transparent)]` with `#[from]` so `?`
     converts a `DomainError` or `StoreError` into it); `crates/app/src/lib.rs::run` and
     `crates/app/src/main.rs` (`anyhow::Result`, `{error:#}` prints the cause chain, exit code 2).
   - Why: a library should return a precise enum the caller can `match` on; an application's
     `main` only needs to print and exit. Alternatives: `Box<dyn Error>` everywhere (callers
     cannot match); hand-written `Display` and `From` impls (what `thiserror` generates; the web
     crate's `AppError` does it by hand and Note 03 explains why).
   - Read more: Book ch. 9.2 `https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html`,
     `https://docs.rs/thiserror`, `https://docs.rs/anyhow`, std `Error`
     `https://doc.rust-lang.org/std/error/trait.Error.html`.
8. **Time is a dependency: Clock.**
   - In our code: `crates/domain/src/clock.rs::Clock` (`Send + Sync` supertraits),
     `crates/domain/src/clock.rs::SystemClock`, `crates/domain/src/clock.rs::FakeClock` (a
     `Mutex` inside so `set` and `advance` work through `&self`),
     `crates/domain/src/clock.rs::truncate_to_micros` (Postgres keeps microseconds, #46); a
     use: `crates/web/src/state.rs::AppState` holds `Arc<dyn Clock>`.
   - Why: a function that calls `Utc::now()` cannot be tested at 03:59 or across a daylight
     saving change; one that receives the time can. Alternatives: pass `now` as a plain
     argument everywhere (the domain functions do exactly that: `can(actor, action, now)`;
     the `Clock` is for the places that must fetch it themselves); a mocking crate.
   - Read more: Book ch. 10.2 `https://doc.rust-lang.org/book/ch10-02-traits.html`, std `Mutex`
     `https://doc.rust-lang.org/std/sync/struct.Mutex.html`, `https://docs.rs/chrono`.
9. **Time zones and daylight saving.**
   - In our code: `crates/domain/src/schedule.rs::DailySchedule` (`last_boundary`,
     `boundary_passed_since`, `day_span`, `month_span`, `year_span`; a day can be 23 or 25
     hours); the private `at_local` and its `match` on `chrono::LocalResult` (a skipped local
     time resolves to the end of the gap, a repeated one to its first occurrence); the instant
     is always `DateTime<Utc>` and the zone only converts at the edges (#46).
   - Why: store UTC, convert for display and for "which local day"; the 04:00 auto-close is a
     local-time rule. Alternatives: store local times (ambiguous twice a year); use fixed
     24-hour days (wrong twice a year).
   - Read more: `https://docs.rs/chrono`, `https://docs.rs/chrono-tz`.
10. **The audit chain: slices, iterators and Result.** (Exercise 4)
    - In our code: `crates/domain/src/audit.rs::AuditRecord`,
      `crates/domain/src/audit.rs::compute_hash` (SHA-256 over the previous hash, a newline and
      the record as canonical JSON), `crates/domain/src/audit.rs::write_canonical` (sorted keys
      written by hand, so the bytes never depend on how `serde_json` was configured by another
      crate: the CI step "audit hash ignores serde_json map ordering" in
      `.github/workflows/ci.yml` proves it), `crates/domain/src/audit.rs::ChainedEntry::seal`,
      `crates/domain/src/audit.rs::verify_chain` (a `for` loop over `&[ChainedEntry]` carrying
      `expected_prev`), `crates/domain/src/audit.rs::GENESIS_HASH`;
      `crates/domain/src/records.rs::SmellReport::upgrade_snapshot` returns a `Cow` so an
      unchanged snapshot is borrowed, not copied (#104).
    - Why: a hash chain makes an edited or deleted row detectable by anyone with the data and no
      trusted server (#25). Alternatives: an `updated_at` column (can be forged); a database
      trigger only (invisible to the application, and SQLite and Postgres differ); a signed log
      (needs key storage).
    - Read more: `https://docs.rs/sha2`, `https://docs.rs/serde_json`, std `Cow`
      `https://doc.rust-lang.org/std/borrow/enum.Cow.html`.
    - Try it: Exercise 4 `exercises/src/ex04_audit_chain.rs`.
11. **Tests next to the code, and property tests.**
    - In our code: `crates/domain/src/permissions.rs` (`#[cfg(test)] mod tests`, a `const
      ACTIONS` array and loops over every role and action);
      `crates/domain/src/audit.rs` (a `proptest!` block: arbitrary chains, one edit anywhere is
      caught; `use proptest::prelude::*`); `crates/domain/src/clock.rs` (`FakeClock` in tests);
      the integration tests in `crates/web/tests/common/mod.rs::TestApp` (Note 03).
    - Why: unit tests live in the file they test so a change and its test sit in one diff;
      property tests find the cases nobody thought of. Alternatives: only example tests;
      a separate `tests/` folder for everything (cannot reach private functions).
    - Read more: Book ch. 11.1 `https://doc.rust-lang.org/book/ch11-01-writing-tests.html`,
      Book ch. 11.3 `https://doc.rust-lang.org/book/ch11-03-test-organization.html`,
      `https://proptest-rs.github.io/proptest/intro.html`.
12. **Formatting, lints and the safety net.**
    - In our code: `rustfmt.toml`; `Cargo.toml` (`[workspace.lints]`);
      `.github/workflows/ci.yml` (job `check`: `cargo fmt --all --check`, clippy with
      `RUSTFLAGS: -D warnings`, `cargo test --workspace --locked`); `deny.toml` (licences and
      advisories, run by `.github/workflows/security.yml`); `.githooks/pre-commit` with
      `.gitleaks.toml` (a commit containing a secret is refused, #37); `rust-toolchain.toml`.
    - Why: the tools catch what review misses and cost nothing to run (#36). Alternatives:
      rely on review only; a paid scanner.
    - Read more: `https://doc.rust-lang.org/cargo/reference/workspaces.html` (the `[lints]`
      table), the Rust clippy lint list `https://rust-lang.github.io/rust-clippy/master/index.html`.

**Check yourself** (use or adapt; the note needs five or six): Why does `Level::new` return a
`Result` instead of panicking? What would break if `Level`'s field were `pub`? Why does `can`
have no `_ =>` arm on the role? What does `$crate` do inside `str_enum!`? What would happen
to old audit entries if `write_canonical` sorted keys differently? Why does `FakeClock` hold a
`Mutex`?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/01-foundation-and-domain.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/01-foundation-and-domain.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/01-foundation-and-domain.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-01.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-01.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/01-foundation-and-domain.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/01-foundation-and-domain.md
git commit -m "docs(learning): note 01, newtypes, enums, macros and the audit chain

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---


### Task 4: Exercise 1 — Validated newtypes (Level, Username, Password)

**Files:**
- Create: `exercises/src/ex01_newtypes.rs`, `exercises/tests/ex01_newtypes.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex01_newtypes;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/01-foundation-and-domain.md` (Task 3).
- Produces: the public items `RuleError`, `Level`, `Username`, `Password`, used only by this exercise's own test file.

**What the learner practises:** private fields, validating `new` returning `Result`, `TryFrom` and `From`, `Display`, a hand-written `Debug` that hides a secret, counting characters instead of bytes
**Difficulty:** one star (about 20 minutes). **Tests:** 12. **`todo!()` bodies in the stub:** 11.

- [ ] **Step 1: Create the stub `exercises/src/ex01_newtypes.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 1: validated newtypes.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 3.
//! Mirrors: `smoke_domain::types::{Level, Username, NewPassword}`.
//! Run: `cargo test --test ex01_newtypes`
//!
//! Hint: the only way to build each type is its `new`, which checks the rule, so a value that
//! exists is a valid value. Keep the field private. For `Password`, count characters with
//! `chars().count()`, not bytes with `len()`, and never print the text in `Debug`.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleError {
    InvalidLevel(u8),
    InvalidUsername(String),
    InvalidPasswordLength(usize),
}

/// A smell level from 0 (checked and clean) to 10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level(u8);

impl Level {
    pub const MAX: u8 = 10;
    pub const CLEAN: Level = Level(0);

    pub fn new(value: u8) -> Result<Self, RuleError> {
        todo!("accept 0..=MAX, otherwise RuleError::InvalidLevel(value)")
    }

    pub fn get(self) -> u8 {
        todo!()
    }

    pub fn is_clean(self) -> bool {
        todo!()
    }
}

impl TryFrom<u8> for Level {
    type Error = RuleError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        todo!("reuse Level::new")
    }
}

impl From<Level> for u8 {
    fn from(level: Level) -> Self {
        todo!()
    }
}

/// A login name: trimmed and lowercased, 3 to 32 characters, only a-z, 0-9, '.', '_' and '-'.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub const MIN_LEN: usize = 3;
    pub const MAX_LEN: usize = 32;

    /// The error carries the text as it was typed, not the cleaned-up version.
    pub fn new(raw: &str) -> Result<Self, RuleError> {
        todo!()
    }

    pub fn as_str(&self) -> &str {
        todo!()
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

/// A password being chosen: 10 to 128 characters, kept exactly as typed.
#[derive(Clone, PartialEq, Eq)]
pub struct Password(String);

impl Password {
    pub const MIN_CHARS: usize = 10;
    pub const MAX_CHARS: usize = 128;

    pub fn new(raw: String) -> Result<Self, RuleError> {
        todo!("the error carries the number of characters")
    }

    pub fn expose(&self) -> &str {
        todo!()
    }
}

impl fmt::Debug for Password {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!("print exactly: Password(<redacted>)")
    }
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex01_newtypes.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex01_newtypes::{Level, Password, RuleError, Username};

#[test]
fn levels_from_0_to_10_are_valid() {
    for value in 0..=10 {
        assert_eq!(Level::new(value).unwrap().get(), value);
    }
}

#[test]
fn a_level_above_10_is_refused_with_the_value() {
    assert_eq!(Level::new(11), Err(RuleError::InvalidLevel(11)));
    assert_eq!(Level::new(255), Err(RuleError::InvalidLevel(255)));
}

#[test]
fn only_zero_is_clean() {
    assert!(Level::CLEAN.is_clean());
    assert!(Level::new(0).unwrap().is_clean());
    assert!(!Level::new(1).unwrap().is_clean());
}

#[test]
fn levels_compare_by_value() {
    assert!(Level::new(3).unwrap() < Level::new(7).unwrap());
    let highest = [2, 9, 4]
        .into_iter()
        .map(|v| Level::new(v).unwrap())
        .max()
        .unwrap();
    assert_eq!(highest.get(), 9);
}

#[test]
fn levels_convert_both_ways() {
    assert_eq!(Level::try_from(5).unwrap().get(), 5);
    assert_eq!(Level::try_from(12), Err(RuleError::InvalidLevel(12)));
    assert_eq!(u8::from(Level::new(8).unwrap()), 8);
}

#[test]
fn a_username_is_trimmed_and_lowercased() {
    let name = Username::new("  Daniele ").unwrap();
    assert_eq!(name.as_str(), "daniele");
    assert_eq!(name.to_string(), "daniele");
}

#[test]
fn usernames_allow_letters_digits_dot_underscore_and_dash() {
    assert!(Username::new("a.b_c-d9").is_ok());
}

#[test]
fn usernames_have_3_to_32_characters() {
    assert!(Username::new("abc").is_ok());
    assert!(Username::new(&"a".repeat(32)).is_ok());
    assert_eq!(
        Username::new("ab"),
        Err(RuleError::InvalidUsername("ab".to_owned()))
    );
    assert!(Username::new(&"a".repeat(33)).is_err());
}

#[test]
fn other_characters_are_refused_and_the_error_keeps_what_was_typed() {
    assert_eq!(
        Username::new(" A B "),
        Err(RuleError::InvalidUsername(" A B ".to_owned()))
    );
    assert!(Username::new("café").is_err());
    assert!(Username::new("a/b").is_err());
}

#[test]
fn a_password_needs_10_to_128_characters() {
    assert_eq!(
        Password::new("123456789".to_owned()),
        Err(RuleError::InvalidPasswordLength(9))
    );
    assert!(Password::new("1234567890".to_owned()).is_ok());
    assert!(Password::new("x".repeat(128)).is_ok());
    assert_eq!(
        Password::new("x".repeat(129)),
        Err(RuleError::InvalidPasswordLength(129))
    );
}

#[test]
fn a_password_counts_characters_not_bytes() {
    assert!(Password::new("é".repeat(10)).is_ok());
    assert_eq!(
        Password::new("é".repeat(5)),
        Err(RuleError::InvalidPasswordLength(5))
    );
}

#[test]
fn a_password_is_kept_exactly_as_typed_and_never_printed() {
    let password = Password::new("  spaces and Capitals  ".to_owned()).unwrap();
    assert_eq!(password.expose(), "  spaces and Capitals  ");
    assert_eq!(format!("{password:?}"), "Password(<redacted>)");
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex01_newtypes;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex01_newtypes --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex01_newtypes 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 12 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex01_newtypes.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex01_newtypes 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 12 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex01_newtypes` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/01-foundation-and-domain.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex01_newtypes.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex01_newtypes.rs        # 11
grep -n '@' src/ex01_newtypes.rs tests/ex01_newtypes.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex01_newtypes.rs exercises/tests/ex01_newtypes.rs
git commit -m "docs(exercises): exercise 1, validated newtypes (Rust: newtype pattern, TryFrom, Debug)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 5: Exercise 2 — Who may do what, as one function

**Files:**
- Create: `exercises/src/ex02_permissions.rs`, `exercises/tests/ex02_permissions.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex02_permissions;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/01-foundation-and-domain.md` (Task 3).
- Produces: the public items `Role`, `Actor`, `Action`, `can`, `can_see` (`Role` is given, not a stub; Exercise 14 reuses it), used only by this exercise's own test file.

**What the learner practises:** enums that carry data, nested `match` without a catch-all, `Option::is_some_and`, tables turned into code
**Difficulty:** one star (about 25 minutes). **Tests:** 8. **`todo!()` bodies in the stub:** 4.

- [ ] **Step 1: Create the stub `exercises/src/ex02_permissions.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 2: who may do what, as one function.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 5.
//! Mirrors: `smoke_domain::permissions::{Actor, Action, can, can_see}`.
//! Run: `cargo test --test ex02_permissions`
//!
//! Hint: `match actor.role { ... }` with an inner `match action { ... }` per role, no `_`
//! catch-all on the roles so adding a role later is a compile error. `Option::is_some_and`
//! turns "has an expiry and it has passed" into one expression. Times are plain seconds.
//!
//! The rules (spec section 4):
//! - A resident may do everything and never expires.
//! - A guest may always view their own data; log entries, and edit entries or manage events
//!   they own, only while not expired. A guest may never view all, manage users or edit lists.
//! - A viewer may view everything and their own data while not expired, and nothing else.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Resident,
    Guest,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub role: Role,
    /// The second at which the account stops working; `None` means never.
    pub expires_at: Option<u64>,
}

impl Actor {
    /// Expired once `now` reaches `expires_at`.
    pub fn is_expired(&self, now: u64) -> bool {
        todo!()
    }

    /// Everyone may sign in except an expired viewer (an expired guest keeps read access to
    /// their own data).
    pub fn can_sign_in(&self, now: u64) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LogEntry,
    ViewAll,
    ViewOwn,
    EditEntry { owner: u32 },
    ManageEvent { owner: u32 },
    ManageUsers,
    EditLists,
}

pub fn can(actor: &Actor, action: Action, now: u64) -> bool {
    todo!()
}

/// Whether `actor` may see something logged by `owner`: everything with `ViewAll`, otherwise
/// only their own.
pub fn can_see(actor: &Actor, owner: u32, now: u64) -> bool {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex02_permissions.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex02_permissions::{Action, Actor, Role, can, can_see};

const NOW: u64 = 1_000;
const ME: u32 = 1;
const OTHER: u32 = 2;

fn actor(role: Role, expires_at: Option<u64>) -> Actor {
    Actor {
        id: ME,
        role,
        expires_at,
    }
}

fn every_action() -> [Action; 9] {
    [
        Action::LogEntry,
        Action::ViewAll,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::EditEntry { owner: OTHER },
        Action::ManageEvent { owner: ME },
        Action::ManageEvent { owner: OTHER },
        Action::ManageUsers,
        Action::EditLists,
    ]
}

#[test]
fn a_resident_may_do_everything() {
    let resident = actor(Role::Resident, None);
    for action in every_action() {
        assert!(can(&resident, action, NOW), "{action:?}");
    }
}

#[test]
fn an_active_guest_logs_and_manages_only_what_is_theirs() {
    let guest = actor(Role::Guest, Some(NOW + 1));
    let allowed = [
        Action::LogEntry,
        Action::ViewOwn,
        Action::EditEntry { owner: ME },
        Action::ManageEvent { owner: ME },
    ];
    for action in every_action() {
        assert_eq!(
            can(&guest, action, NOW),
            allowed.contains(&action),
            "{action:?}"
        );
    }
}

#[test]
fn an_expired_guest_can_only_view_their_own_data() {
    let guest = actor(Role::Guest, Some(NOW));
    for action in every_action() {
        assert_eq!(
            can(&guest, action, NOW),
            action == Action::ViewOwn,
            "{action:?}"
        );
    }
}

#[test]
fn an_active_viewer_views_everything_and_changes_nothing() {
    let viewer = actor(Role::Viewer, None);
    for action in every_action() {
        let expected = matches!(action, Action::ViewAll | Action::ViewOwn);
        assert_eq!(can(&viewer, action, NOW), expected, "{action:?}");
    }
}

#[test]
fn an_expired_viewer_can_do_nothing() {
    let viewer = actor(Role::Viewer, Some(NOW - 1));
    for action in every_action() {
        assert!(!can(&viewer, action, NOW), "{action:?}");
    }
}

#[test]
fn an_account_expires_exactly_at_its_expiry_second() {
    let guest = actor(Role::Guest, Some(NOW));
    assert!(!guest.is_expired(NOW - 1));
    assert!(guest.is_expired(NOW));
    assert!(!actor(Role::Resident, None).is_expired(u64::MAX));
}

#[test]
fn only_an_expired_viewer_cannot_sign_in() {
    assert!(actor(Role::Viewer, Some(NOW + 1)).can_sign_in(NOW));
    assert!(!actor(Role::Viewer, Some(NOW)).can_sign_in(NOW));
    assert!(actor(Role::Guest, Some(NOW)).can_sign_in(NOW));
    assert!(actor(Role::Resident, None).can_sign_in(NOW));
}

#[test]
fn people_see_what_their_role_allows() {
    let resident = actor(Role::Resident, None);
    let guest = actor(Role::Guest, Some(NOW + 1));
    let expired_guest = actor(Role::Guest, Some(NOW));
    let viewer = actor(Role::Viewer, None);
    let expired_viewer = actor(Role::Viewer, Some(NOW));

    assert!(can_see(&resident, OTHER, NOW));
    assert!(can_see(&viewer, OTHER, NOW));
    assert!(!can_see(&expired_viewer, OTHER, NOW));
    assert!(!can_see(&expired_viewer, ME, NOW));
    assert!(can_see(&guest, ME, NOW));
    assert!(!can_see(&guest, OTHER, NOW));
    assert!(can_see(&expired_guest, ME, NOW));
    assert!(!can_see(&expired_guest, OTHER, NOW));
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex02_permissions;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex02_permissions --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex02_permissions 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 8 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex02_permissions.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex02_permissions 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 8 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex02_permissions` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/01-foundation-and-domain.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex02_permissions.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex02_permissions.rs        # 4
grep -n '@' src/ex02_permissions.rs tests/ex02_permissions.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex02_permissions.rs exercises/tests/ex02_permissions.rs
git commit -m "docs(exercises): exercise 2, permissions as match (Rust: enums with data, exhaustive match)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 6: Exercise 3 — A macro that writes the same enum code every time

**Files:**
- Create: `exercises/src/ex03_text_enum.rs`, `exercises/tests/ex03_text_enum.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex03_text_enum;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/01-foundation-and-domain.md` (Task 3).
- Produces: the public items `UnknownValue` and the exported macro `text_enum!`, used only by this exercise's own test file.

**What the learner practises:** `macro_rules!` repetition (`$( ... ),+`), `$crate::` paths, `stringify!`, implementing `FromStr` and `Display` inside a macro. This is the hardest of the first four exercises: the stub already has the macro's pattern, only the expansion needs writing
**Difficulty:** three stars (about 45 minutes). **Tests:** 5. **`todo!()` bodies in the stub:** 4.

- [ ] **Step 1: Create the stub `exercises/src/ex03_text_enum.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 3: a declarative macro that writes the same code for every enum.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 6.
//! Mirrors: the `str_enum!` macro in `smoke_domain::types`.
//! Run: `cargo test --test ex03_text_enum`
//!
//! Hint: the pattern already captures the enum name and the `Variant => "text"` pairs. Inside
//! the expansion, repeat a piece of code once per pair with `$( ... )+` (or `,` as separator):
//! `match self { $( $name::$variant => $text ),+ }` writes one arm per variant. Replace every
//! `todo!()` in the expansion; `ALL` lists the variants in declaration order. `stringify!($name)`
//! turns the enum's name into text.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownValue {
    pub kind: &'static str,
    pub value: String,
}

#[macro_export]
macro_rules! text_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [$name] = &[];

            /// The stable text of this variant.
            pub fn as_str(self) -> &'static str {
                todo!("one match arm per variant")
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = $crate::ex03_text_enum::UnknownValue;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                todo!("the variant whose text is `s`, else UnknownValue {{ kind, value }}")
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                todo!("write as_str()")
            }
        }
    };
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex03_text_enum.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex03_text_enum::UnknownValue;
use smoke_exercises::text_enum;

text_enum!(
    /// A made-up enum for the test.
    Colour {
        Red => "red",
        DarkGreen => "dark_green",
        Blue => "blue",
    }
);

text_enum!(Answer { Yes => "yes", No => "no" });

#[test]
fn each_variant_has_its_text() {
    assert_eq!(Colour::Red.as_str(), "red");
    assert_eq!(Colour::DarkGreen.as_str(), "dark_green");
    assert_eq!(Colour::Blue.as_str(), "blue");
}

#[test]
fn all_lists_the_variants_in_declaration_order() {
    assert_eq!(Colour::ALL, &[Colour::Red, Colour::DarkGreen, Colour::Blue]);
    assert_eq!(Answer::ALL, &[Answer::Yes, Answer::No]);
}

#[test]
fn text_parses_back_to_the_variant() {
    for colour in [Colour::Red, Colour::DarkGreen, Colour::Blue] {
        assert_eq!(colour.as_str().parse::<Colour>(), Ok(colour));
    }
}

#[test]
fn unknown_text_names_the_enum_and_the_value() {
    assert_eq!(
        "purple".parse::<Colour>(),
        Err(UnknownValue {
            kind: "Colour",
            value: "purple".to_owned()
        })
    );
    assert_eq!(
        "Yes".parse::<Answer>(),
        Err(UnknownValue {
            kind: "Answer",
            value: "Yes".to_owned()
        })
    );
}

#[test]
fn display_shows_the_text() {
    assert_eq!(Colour::DarkGreen.to_string(), "dark_green");
    assert_eq!(format!("{}", Answer::No), "no");
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex03_text_enum;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex03_text_enum --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex03_text_enum 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 5 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex03_text_enum.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex03_text_enum 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 5 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex03_text_enum` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/01-foundation-and-domain.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex03_text_enum.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex03_text_enum.rs        # 4
grep -n '@' src/ex03_text_enum.rs tests/ex03_text_enum.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex03_text_enum.rs exercises/tests/ex03_text_enum.rs
git commit -m "docs(exercises): exercise 3, text_enum! macro (Rust: macro_rules!, FromStr, Display)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 7: Exercise 4 — A tamper-evident chain of entries

**Files:**
- Create: `exercises/src/ex04_audit_chain.rs`, `exercises/tests/ex04_audit_chain.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex04_audit_chain;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/01-foundation-and-domain.md` (Task 3).
- Produces: the public items `GENESIS`, `Entry`, `ChainError`, `link_hash` (given), `append`, `build_chain`, `verify_chain`, used only by this exercise's own test file.

**What the learner practises:** slices and `Vec`, `Option::map_or`, a loop that carries state, early `return Err(...)`, why checking the link before the content matters
**Difficulty:** two stars (about 30 minutes). **Tests:** 8. **`todo!()` bodies in the stub:** 3.

- [ ] **Step 1: Create the stub `exercises/src/ex04_audit_chain.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 4: a tamper-evident chain of entries.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 10.
//! Mirrors: `smoke_domain::audit::{ChainedEntry, compute_hash, verify_chain}`.
//! Run: `cargo test --test ex04_audit_chain`
//!
//! Hint: every entry stores the hash of the entry before it (`prev_hash`) and its own `hash`,
//! which covers `prev_hash`, its id and its payload. Change or remove any entry and every
//! link after it stops matching. `link_hash` is given; use it for both building and checking.
//! Walk the slice once, remembering the hash the next entry must point back to (a `for` loop
//! with a variable, or `windows(2)` for the pairs). Check the link before the content, so an
//! attacker who recomputes one entry's hash is still caught at the next entry.
//!
//! The app hashes with SHA-256 (the `sha2` crate). The standard library has no SHA-256, so
//! `link_hash` uses `DefaultHasher`: fine for learning, but NOT tamper-proof.

use std::hash::{DefaultHasher, Hash, Hasher};

/// The `prev_hash` of the first entry.
pub const GENESIS: u64 = 0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: u32,
    pub payload: String,
    pub prev_hash: u64,
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// `prev_hash` is not the hash of the entry before.
    BrokenLink { id: u32 },
    /// The content does not match the entry's own `hash`.
    HashMismatch { id: u32 },
}

/// Given: the hash of an entry's content after `prev_hash`.
pub fn link_hash(prev_hash: u64, id: u32, payload: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    prev_hash.hash(&mut hasher);
    id.hash(&mut hasher);
    payload.hash(&mut hasher);
    hasher.finish()
}

/// Adds an entry after the last one (ids count up from 1; the first entry follows `GENESIS`).
pub fn append(chain: &mut Vec<Entry>, payload: &str) {
    todo!()
}

/// A chain with one entry per payload, in order.
pub fn build_chain(payloads: &[&str]) -> Vec<Entry> {
    todo!("start empty and `append` each payload")
}

/// Checks the whole chain from `GENESIS` and reports the first problem found.
pub fn verify_chain(entries: &[Entry]) -> Result<(), ChainError> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex04_audit_chain.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex04_audit_chain::{
    ChainError, Entry, GENESIS, append, build_chain, link_hash, verify_chain,
};

fn chain() -> Vec<Entry> {
    build_chain(&["created report", "corrected report", "closed event"])
}

#[test]
fn entries_are_numbered_from_one_and_linked_in_order() {
    let entries = chain();
    let ids: Vec<u32> = entries.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3]);
    assert_eq!(entries[0].prev_hash, GENESIS);
    assert_eq!(entries[1].prev_hash, entries[0].hash);
    assert_eq!(entries[2].prev_hash, entries[1].hash);
    assert_eq!(
        entries[1].hash,
        link_hash(entries[0].hash, 2, "corrected report")
    );
}

#[test]
fn appending_extends_the_chain() {
    let mut entries = Vec::new();
    append(&mut entries, "first");
    append(&mut entries, "second");
    assert_eq!(entries, build_chain(&["first", "second"]));
}

#[test]
fn an_untouched_chain_verifies_and_so_does_an_empty_one() {
    assert_eq!(verify_chain(&chain()), Ok(()));
    assert_eq!(verify_chain(&[]), Ok(()));
}

#[test]
fn editing_a_payload_is_a_hash_mismatch_at_that_entry() {
    let mut entries = chain();
    entries[1].payload = "corrected report (edited)".to_owned();
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::HashMismatch { id: 2 })
    );
}

#[test]
fn removing_an_entry_breaks_the_link_after_it() {
    let mut entries = chain();
    entries.remove(1);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 3 })
    );
}

#[test]
fn removing_the_first_entry_breaks_the_link_of_the_new_first() {
    let mut entries = chain();
    entries.remove(0);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 2 })
    );
}

#[test]
fn recomputing_one_hash_hides_nothing_because_the_next_entry_still_points_at_the_old_one() {
    let mut entries = chain();
    entries[1].payload = "corrected report (edited)".to_owned();
    entries[1].hash = link_hash(entries[1].prev_hash, 2, &entries[1].payload);
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::BrokenLink { id: 3 })
    );
}

#[test]
fn a_tampered_last_entry_is_still_a_hash_mismatch() {
    let mut entries = chain();
    entries[2].payload = "closed event (edited)".to_owned();
    assert_eq!(
        verify_chain(&entries),
        Err(ChainError::HashMismatch { id: 3 })
    );
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex04_audit_chain;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex04_audit_chain --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex04_audit_chain 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 8 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex04_audit_chain.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex04_audit_chain 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 8 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex04_audit_chain` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/01-foundation-and-domain.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex04_audit_chain.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex04_audit_chain.rs        # 3
grep -n '@' src/ex04_audit_chain.rs tests/ex04_audit_chain.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex04_audit_chain.rs exercises/tests/ex04_audit_chain.rs
git commit -m "docs(exercises): exercise 4, hash-chained entries (Rust: slices, loops, Result)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 8: Note 02 — storage

**Files:**
- Create: `docs/learning/02-storage.md`

**Interfaces:**
- Consumes: decisions #38, #40, #45, #46, #47, #48, #49, #50, #52, #53, #55, #67, #77, #99, #109
  in `docs/DECISIONS.md`; the code listed below.
- Produces: the note that Exercises 5 to 7 link back to (section 1 is Exercise 5, section 4 is
  Exercise 6, section 5 is Exercise 7: keep this numbering).

**Covers:** traits as the storage boundary; native `async fn` in traits; associated types and
the transaction object; ownership and `Drop` in transactions; error layering with `From` and
`?`; compile-time checked SQL (sqlx); migrations and schema parity; one contract test suite for
two backends; SQLite and Postgres write serialisation; a generic `record<T: Audited>`;
conversions that keep the database honest.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 02 — Storage

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 5 (a repository trait), 6 (transactions and Drop), 7 (error types and the ? operator). Reading time: about
> 50 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **Traits as the storage boundary.** (Exercise 5)
   - In our code: `crates/domain/src/repos.rs::UserRepo`, `crates/domain/src/repos.rs::EventRepo`
     (reads), `crates/domain/src/repos.rs::EventWrites` (writes inside a transaction),
     `crates/domain/src/repos.rs::Store` (a trait with a **blanket impl**: `impl<T> Store for T
     where T: ...`, so any type that implements all the small traits is a `Store`
     automatically); `crates/domain/src/logbook.rs::log_smell` (generic: `T: EventWrites +
     ReportWrites + ReadingWrites`, it never names SQLite or Postgres);
     `crates/domain/src/sweep.rs::auto_close` (bounds such as `Transactional<Tx: EventWrites +
     JobWrites> + JobRepo`); the two implementations `crates/storage-sqlite/src/lib.rs::SqliteStore`
     and `crates/storage-postgres/src/lib.rs::PostgresStore`.
   - Why: the domain states what it needs, the database crates provide it; many small traits
     mean a function asks only for what it uses (interface segregation) and a test can
     implement one small trait. Static dispatch (generics) over `dyn` (#53, #67): the compiler
     makes one copy per backend, there is no vtable call and the async methods need no boxing;
     the price is more compiled code and generic parameters that spread (`AppState<S>`,
     `router<S>`). Alternatives: `Box<dyn Store>` (simple to pass around, but an object-safe
     trait with async methods needs boxing, and every call is dynamic); a single big trait (every
     fake must implement everything); an ORM (hides the SQL the audit chain must control).
   - Read more: Book ch. 10.1 `https://doc.rust-lang.org/book/ch10-01-syntax.html`, ch. 10.2
     `https://doc.rust-lang.org/book/ch10-02-traits.html`, ch. 18.2
     `https://doc.rust-lang.org/book/ch18-02-trait-objects.html`.
   - Try it: Exercise 5 `exercises/src/ex05_event_repo.rs`.
2. **Native `async fn` in traits (and when `async-trait` is still needed).**
   - In our code: `crates/domain/src/repos.rs::Transactional` (methods return `impl Future<Output
     = StoreResult<...>> + Send`, spelled out so callers may spawn the future on another
     thread); `crates/storage-sqlite/src/lib.rs::SqliteStore` (the impl simply writes `async fn
     begin`); the one place `#[async_trait]` remains: `crates/web/src/sessions.rs::DbSessions`
     implements tower-sessions' `SessionStore`, a trait defined in another crate that still
     uses it (#53, #56).
   - Why: since Rust 1.75 a trait can declare `async fn`; writing `impl Future + Send` in the
     trait promises the future is `Send`, which a plain `async fn` in a trait does not. The
     `async-trait` crate boxes every future (a heap allocation per call) and is only kept where a
     foreign trait forces it. Alternative: `async-trait` everywhere (works on old compilers,
     costs an allocation per call).
   - Read more: `https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html`, the async
     book `https://rust-lang.github.io/async-book/`, std `Future`
     `https://doc.rust-lang.org/std/future/trait.Future.html`, Book ch. 17
     `https://doc.rust-lang.org/book/ch17-00-async-await.html`.
3. **Associated types and the transaction object.**
   - In our code: `crates/domain/src/repos.rs::Transactional` (`type Tx: Commit`, and
     `begin(&self, actor: Option<UserId>)`); `crates/domain/src/repos.rs::Commit` (`fn
     commit(self)` takes `self`: a committed transaction cannot be used again);
     `crates/domain/src/repos.rs::WriteTx` and `crates/domain/src/repos.rs::Store` (supertraits
     combine bounds; `Transactional<Tx: WriteTx>` is an associated type bound); the concrete
     `crates/storage-sqlite/src/tx.rs::SqliteTx` and
     `crates/storage-postgres/src/tx.rs::PostgresTx`; why every write method lives on the
     transaction, not on the store: every write appends its audit entry in the same transaction
     (#47).
   - Why: an associated type lets each backend name its own transaction type while generic
     code stays backend-free. Alternative: a generic parameter on the trait
     (`Transactional<Tx>`), which would let one store have several transaction types, which we
     never want.
   - Read more: Book ch. 20.2 `https://doc.rust-lang.org/book/ch20-02-advanced-traits.html`
     (associated types), std `Send` `https://doc.rust-lang.org/std/marker/trait.Send.html`.
4. **Ownership at work: a transaction that rolls back when dropped.** (Exercise 6)
   - In our code: `crates/domain/src/repos.rs::Transactional` (the doc comment says dropping a
     transaction without `commit` rolls it back); `crates/web/src/routes/entries.rs::submit`
     (`drop(tx)` before returning a form again, and `tx.commit().await?` only on success);
     `crates/storage-sqlite/src/tx.rs::SqliteTx` holds a `sqlx::Transaction<'static, Sqlite>`,
     whose own `Drop` does the rollback; `&mut self` on every write method so two writes cannot
     overlap on one transaction.
   - Why: tying "undo" to scope exit means no code path can forget to roll back, including
     `?` early returns and panics. This is the RAII idea (Resource Acquisition Is Initialization):
     a value owns a resource and releases it in `Drop`. Alternatives: an explicit
     `rollback()` call (easy to forget on an error path); a closure-based
     `store.transaction(|tx| ...)` API (also safe, but nests awkwardly with `async`).
   - Read more: Book ch. 15.3 `https://doc.rust-lang.org/book/ch15-03-drop.html`, ch. 4.2
     `https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html`, std `Drop`
     `https://doc.rust-lang.org/std/ops/trait.Drop.html`.
   - Try it: Exercise 6 `exercises/src/ex06_transaction.rs`.
5. **Errors: `Result`, `?` and `From`.** (Exercise 7)
   - In our code: `crates/domain/src/repos.rs::StoreError` (variants `NotFound`, `Conflict`,
     `Invalid`, `Corrupt`, `Backend(Box<dyn Error + Send + Sync>)` with `#[source]`);
     `crates/domain/src/repos.rs::Conflict` (the data-rule failures a caller can act on);
     `crates/storage-sqlite/src/error.rs::IntoStore` (an *extension trait*: `.db()` on
     `Result<T, sqlx::Error>`), `crates/storage-sqlite/src/error.rs::into_store_error` (maps
     `ErrorKind::UniqueViolation` and friends to `Conflict`),
     `crates/storage-sqlite/src/error.rs::unique_conflict` (decides by the message text);
     `crates/storage-sqlite/src/convert.rs::corrupt`.
   - Why: callers must not depend on `sqlx::Error`, which differs between backends and would
     leak the database into the domain; a conflict the user can fix ("that username is taken")
     is a different thing from a broken disk. Alternatives: return `sqlx::Error` (leaks the
     backend); `anyhow::Error` in a library (callers cannot match); matching on the message text
     everywhere (we do it once, in `unique_conflict`, and name the weakness: the text is SQLite's).
   - Read more: Book ch. 9.2 `https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html`,
     Book ch. 9.3 `https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html`, std `From`
     `https://doc.rust-lang.org/std/convert/trait.From.html`, std `Error`
     `https://doc.rust-lang.org/std/error/trait.Error.html`.
   - Try it: Exercise 7 `exercises/src/ex07_store_errors.rs`.
6. **SQL checked at compile time with sqlx.**
   - In our code: `crates/storage-sqlite/src/audit.rs::fetch_entries` (`sqlx::query_as!`
     with a row struct, `$1` parameters), `crates/storage-sqlite/src/tx.rs::SqliteTx` (`sqlx::query!`
     in `record`); the offline data in `crates/storage-sqlite/.sqlx/` and
     `crates/storage-postgres/.sqlx/`, regenerated by `scripts/sqlx-prepare.sh` and checked by
     the CI job `sqlx` in `.github/workflows/ci.yml`; `.cargo/config.toml` (`SQLX_OFFLINE =
     "true"`, so building needs no database); `crates/storage-sqlite/sqlx.toml` (#52).
   - Why: the macro asks the database at build time whether the SQL is valid and what types
     the columns have, so a typo or a wrong column type is a compile error. The price is the
     offline data to commit. Alternatives: `sqlx::query` with runtime strings (no build-time check);
     an ORM such as Diesel or SeaORM (a larger API to learn and a model layer the audit chain
     would have to bypass); `rusqlite` plus `tokio-postgres` by hand (two APIs).
   - Read more: `https://docs.rs/sqlx`, `https://docs.rs/sqlx/latest/sqlx/macro.query.html`.
7. **Migrations, and two databases that must agree.**
   - In our code: `crates/storage-sqlite/migrations/0001_initial.sql` (`CHECK` constraints,
     the partial unique index `events_one_open`, #49), `crates/storage-sqlite/migrations/0004_checks.sql`
     (SQLite cannot drop `NOT NULL`, so it rebuilds three tables inside the migration, #109), the
     Postgres twins in `crates/storage-postgres/migrations/`;
     `crates/storage-sqlite/src/lib.rs::SqliteStore::migrate` (`sqlx::migrate!("./migrations")`
     embeds the files in the binary);
     `crates/storage-postgres/tests/schema_parity.rs::family` and the test around it, which
     compares column names, type families and nullability across the two migration sets.
   - Why: the schema is code that must be versioned, applied in order and checked; a parity test
     is the cheapest way to catch "I changed one backend". Alternatives: SQL by hand in a README;
     one migration set with conditional SQL (the dialects differ too much).
   - Read more: `https://docs.rs/sqlx` (the `migrate!` macro), SQLite and Postgres facts in
     `https://www.sqlite.org/lang_transaction.html`.
8. **One contract, two backends: a test macro.**
   - In our code: `crates/storage-contract/src/lib.rs::contract_tests` (a `#[macro_export]
     macro_rules!` with an internal `@each` rule that expands one `#[tokio::test]` per listed
     function, `$crate::$module::$test`); `crates/storage-contract/src/fixture.rs::Fixture`
     (a fresh migrated database, a `FakeClock`, a raw-SQL runner so tests can tamper with the
     data), `crates/storage-contract/src/fixture.rs::raw_runner`;
     `crates/storage-sqlite/tests/contract.rs::fixture` and
     `crates/storage-postgres/tests/contract.rs` (each backend calls
     `smoke_storage_contract::contract_tests!(fixture)`; the Postgres fixture
     `crates/storage-postgres/tests/common/mod.rs::fresh_database` returns `None`, skipping the
     tests, when `TEST_POSTGRES_URL` is unset on a developer machine, and panics when it is
     unset in CI, so CI can never skip them silently); #40, #45.
   - Why: the same behaviour tests run against both stores, so "SQLite and Postgres agree" is
     checked, not hoped for. Alternatives: copy the tests per backend (they drift); a trait-object
     test harness (async methods make `dyn` awkward); test only SQLite and trust Postgres.
   - Read more: Book ch. 11.3 `https://doc.rust-lang.org/book/ch11-03-test-organization.html`,
     Reference `https://doc.rust-lang.org/reference/macros-by-example.html`.
9. **Writers and readers: SQLite pools and Postgres locks.**
   - In our code: `crates/storage-sqlite/src/lib.rs::SqliteStore::connect` (one writer
     connection, `max_connections(1)`, WAL mode, and a separate read-only pool of four;
     in-memory databases are refused because the writer and readers must share a file),
     `crates/storage-sqlite/src/lib.rs::SqliteStore` `begin` (`BEGIN IMMEDIATE`);
     `crates/storage-postgres/src/lib.rs::PostgresStore` `begin` (`SET LOCAL lock_timeout`, `LOCK
     TABLE audit_log IN EXCLUSIVE MODE`); the contract test
     `crates/storage-contract/src/audit.rs::concurrent_writers_never_fork_the_chain` (#48).
   - Why: every audit entry links to the one before it, so writes must be serialised; each
     database does it its own way. Alternatives: one global `Mutex` in Rust (does not protect
     against the CLI process writing at the same time); optimistic retry on a unique-index
     failure (the `ChainForked` conflict exists as the backstop).
   - Read more: `https://www.sqlite.org/wal.html`, `https://www.sqlite.org/lang_transaction.html`,
     `https://www.postgresql.org/docs/current/sql-lock.html`.
10. **A generic helper: `record<T: Audited>`.**
    - In our code: `crates/domain/src/records.rs::Audited` (a trait with an associated
      `const ENTITY`, a required `audit_id`, and a provided `snapshot`);
      `crates/storage-sqlite/src/tx.rs::SqliteTx` `record` (generic over `T: Audited`, uses
      `T::ENTITY`; an `update` that changed nothing writes no entry); the verification side
      `crates/domain/src/verify.rs::Snapshot` (`Snapshot::of::<T>`) and
      `crates/domain/src/verify.rs::verify_store`.
    - Why: one function audits every kind of record; adding a record type means implementing
      three small items. Alternative: an enum of all records and a `match` (every new record
      edits one central function).
    - Read more: Book ch. 10.1 `https://doc.rust-lang.org/book/ch10-01-syntax.html`.
11. **Small conversions that keep the database honest.**
    - In our code: `crates/storage-sqlite/src/convert.rs::micros`,
      `crates/storage-sqlite/src/convert.rs::instant`,
      `crates/storage-sqlite/src/convert.rs::opt_instant` (instants as integer microseconds, #46;
      `Option::map` plus `transpose`), `crates/storage-sqlite/src/convert.rs::parse` (generic over
      `FromStr<Err = DomainError>`), `crates/storage-sqlite/src/convert.rs::canonical` (a value read
      back must equal its normalised form, or the data was edited by hand);
      `crates/storage-sqlite/src/reports.rs::assemble` (`u8::try_from(level).map_err(corrupt)?`,
      collecting an iterator of `Result`s into one `Result`);
      `crates/storage-sqlite/src/audit.rs::json` (`Option<Result>` turned into `Result<Option>`
      with `transpose`).
    - Why: the boundary between the database's loose types and the domain's strict ones is
      where bad data is caught; refusing to read corrupt data is better than showing it.
      Alternative: `as` casts (silently wrong when out of range).
    - Read more: std `TryFrom` `https://doc.rust-lang.org/std/convert/trait.TryFrom.html`, std
      `Iterator` `https://doc.rust-lang.org/std/iter/trait.Iterator.html`.

**Check yourself** (use or adapt; the note needs five or six): Why does `Commit::commit` take
`self` and not `&mut self`? What does `drop(tx)` do in `submit`, and what happens if that line
is removed and the function returns an error? Why do the storage traits return `impl Future +
Send` instead of using `async fn` directly? Where would a third backend plug in, and what would
the contract tests ask of it? Why does `unique_conflict` match on text, and what would break if
SQLite changed its message? What does the blanket `impl<T> Store for T` buy?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/02-storage.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/02-storage.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/02-storage.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-02.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-02.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/02-storage.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/02-storage.md
git commit -m "docs(learning): note 02, traits as the storage boundary, ownership in transactions and errors with From

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---

### Task 9: Exercise 5 — A repository trait, an in-memory store and a generic function

**Files:**
- Create: `exercises/src/ex05_event_repo.rs`, `exercises/tests/ex05_event_repo.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex05_event_repo;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/02-storage.md` (Task 8).
- Produces: the public items `Event`, `Conflict`, `EventRepo`, `MemoryEvents`, `close_open_event`, used only by this exercise's own test file.

**What the learner practises:** defining and implementing a trait, `Option` and `Result` returns, enforcing a rule the database would enforce with a unique index, a function generic over any implementation (`R: EventRepo`), testing with a second implementation
**Difficulty:** two stars (about 35 minutes). **Tests:** 7. **`todo!()` bodies in the stub:** 5.

- [ ] **Step 1: Create the stub `exercises/src/ex05_event_repo.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 5: a repository trait, an in-memory implementation and a generic function.
//!
//! Practises: `docs/learning/02-storage.md`, section 1.
//! Mirrors: `smoke_domain::repos::{EventRepo, EventWrites}` and the `events_one_open` index.
//! Run: `cargo test --test ex05_event_repo`
//!
//! Hint: `MemoryEvents` keeps a `Vec<Event>`; ids are 1, 2, 3... in creation order. The database
//! enforces "at most one open event" with a unique index; here `create_event` must enforce it
//! by returning `Conflict::AnotherEventOpen`. `close_open_event` is generic: it works with ANY
//! `EventRepo`, so write it using only the trait's methods, never `MemoryEvents`' fields.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Event {
    pub id: u32,
    pub owner: u32,
    pub opened_at: u64,
    pub closed_at: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    /// Another event is already open.
    AnotherEventOpen,
    /// Closing an event that is already closed.
    EventNotOpen,
    /// No event has this id.
    UnknownEvent,
    /// Closing before the event opened (the table's `closed_at >= opened_at` check).
    RuleViolated,
}

pub trait EventRepo {
    /// The event that is open, if any.
    fn open_event(&self) -> Option<Event>;
    fn event(&self, id: u32) -> Option<Event>;
    /// Opens a new event, or `Conflict::AnotherEventOpen`.
    fn create_event(&mut self, owner: u32, opened_at: u64) -> Result<Event, Conflict>;
    /// Closes event `id` at `at`. Unknown id: `UnknownEvent`; already closed: `EventNotOpen`;
    /// `at` before it opened: `RuleViolated`. Checked in that order.
    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict>;
}

#[derive(Debug, Default)]
pub struct MemoryEvents {
    events: Vec<Event>,
}

impl EventRepo for MemoryEvents {
    fn open_event(&self) -> Option<Event> {
        todo!()
    }

    fn event(&self, id: u32) -> Option<Event> {
        todo!()
    }

    fn create_event(&mut self, owner: u32, opened_at: u64) -> Result<Event, Conflict> {
        todo!()
    }

    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict> {
        todo!()
    }
}

/// Closes the open event at `at`, if there is one, and returns it. Nothing open is not an
/// error: `Ok(None)`.
pub fn close_open_event<R: EventRepo>(repo: &mut R, at: u64) -> Result<Option<Event>, Conflict> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex05_event_repo.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex05_event_repo::{
    Conflict, Event, EventRepo, MemoryEvents, close_open_event,
};

#[test]
fn a_new_event_is_open_and_numbered_from_one() {
    let mut repo = MemoryEvents::default();
    let event = repo.create_event(7, 100).unwrap();
    assert_eq!(
        event,
        Event {
            id: 1,
            owner: 7,
            opened_at: 100,
            closed_at: None
        }
    );
    assert_eq!(repo.open_event(), Some(event));
    assert_eq!(repo.event(1), Some(event));
    assert_eq!(repo.event(2), None);
}

#[test]
fn only_one_event_can_be_open() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    assert_eq!(repo.create_event(2, 200), Err(Conflict::AnotherEventOpen));
}

#[test]
fn closing_frees_the_slot_for_the_next_event() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    let closed = repo.close_event(1, 150).unwrap();
    assert_eq!(closed.closed_at, Some(150));
    assert_eq!(repo.open_event(), None);
    assert_eq!(repo.event(1), Some(closed));
    assert_eq!(repo.create_event(1, 200).unwrap().id, 2);
}

#[test]
fn closing_checks_the_id_then_the_state_then_the_time() {
    let mut repo = MemoryEvents::default();
    assert_eq!(repo.close_event(1, 10), Err(Conflict::UnknownEvent));
    repo.create_event(1, 100).unwrap();
    assert_eq!(repo.close_event(1, 99), Err(Conflict::RuleViolated));
    assert_eq!(repo.open_event().map(|e| e.id), Some(1));
    repo.close_event(1, 100).unwrap();
    assert_eq!(repo.close_event(1, 200), Err(Conflict::EventNotOpen));
}

#[test]
fn close_open_event_closes_what_is_open() {
    let mut repo = MemoryEvents::default();
    repo.create_event(1, 100).unwrap();
    let closed = close_open_event(&mut repo, 180).unwrap().unwrap();
    assert_eq!(closed.closed_at, Some(180));
    assert_eq!(repo.open_event(), None);
}

#[test]
fn close_open_event_with_nothing_open_is_not_an_error() {
    let mut repo = MemoryEvents::default();
    assert_eq!(close_open_event(&mut repo, 180), Ok(None));
}

/// A repository that is not `MemoryEvents` at all: `close_open_event` must work with it too.
struct Recording {
    open: Option<Event>,
    closed_with: Vec<(u32, u64)>,
}

impl EventRepo for Recording {
    fn open_event(&self) -> Option<Event> {
        self.open
    }

    fn event(&self, _id: u32) -> Option<Event> {
        self.open
    }

    fn create_event(&mut self, _owner: u32, _opened_at: u64) -> Result<Event, Conflict> {
        Err(Conflict::AnotherEventOpen)
    }

    fn close_event(&mut self, id: u32, at: u64) -> Result<Event, Conflict> {
        self.closed_with.push((id, at));
        let mut event = self.open.take().ok_or(Conflict::UnknownEvent)?;
        event.closed_at = Some(at);
        Ok(event)
    }
}

#[test]
fn close_open_event_only_uses_the_trait() {
    let mut repo = Recording {
        open: Some(Event {
            id: 9,
            owner: 1,
            opened_at: 5,
            closed_at: None,
        }),
        closed_with: Vec::new(),
    };
    close_open_event(&mut repo, 50).unwrap();
    assert_eq!(repo.closed_with, [(9, 50)]);
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex05_event_repo;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex05_event_repo --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex05_event_repo 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 7 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex05_event_repo.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex05_event_repo 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 7 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex05_event_repo` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/02-storage.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex05_event_repo.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex05_event_repo.rs        # 5
grep -n '@' src/ex05_event_repo.rs tests/ex05_event_repo.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex05_event_repo.rs exercises/tests/ex05_event_repo.rs
git commit -m "docs(exercises): exercise 5, repository trait with an in-memory store (Rust: traits, generics)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 10: Exercise 6 — A transaction that rolls back when dropped

**Files:**
- Create: `exercises/src/ex06_transaction.rs`, `exercises/tests/ex06_transaction.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex06_transaction;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/02-storage.md` (Task 8).
- Produces: the public items `Ledger`, `Tx`, `write_all`, used only by this exercise's own test file.

**What the learner practises:** a struct holding `&'a mut` (an exclusive borrow, so nothing else can touch the ledger meanwhile), a method taking `self` by value, implementing `Drop`, early return with `Err` running destructors. The stub's `Drop` impl is empty so that the file compiles quietly: the learner writes its body
**Difficulty:** two stars (about 35 minutes). **Tests:** 7. **`todo!()` bodies in the stub:** 7.

- [ ] **Step 1: Create the stub `exercises/src/ex06_transaction.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 6: a transaction that rolls back when it is dropped.
//!
//! Practises: `docs/learning/02-storage.md`, section 4.
//! Mirrors: `smoke_domain::repos::{Transactional, Commit}` and `smoke_storage_sqlite::SqliteTx`:
//! "dropping a transaction without `commit` rolls it back".
//! Run: `cargo test --test ex06_transaction`
//!
//! Hint: `Tx` holds an exclusive borrow of the `Ledger` (`&'a mut Ledger`), so nothing else can
//! touch the ledger while a transaction is alive, and the compiler enforces it. `commit` takes
//! `self` by value, so a committed transaction cannot be used again. Rust runs `Drop::drop`
//! when a value goes out of scope, also after `commit` consumed it: remember in `committed`
//! whether the commit happened, and count a rollback only when it did not.

#[derive(Debug, Default)]
pub struct Ledger {
    entries: Vec<String>,
    rollbacks: u32,
}

impl Ledger {
    pub fn new() -> Self {
        Self::default()
    }

    /// The committed entries.
    pub fn entries(&self) -> &[String] {
        todo!()
    }

    /// How many transactions ended without a commit.
    pub fn rollbacks(&self) -> u32 {
        todo!()
    }

    /// Starts a transaction. The ledger is borrowed mutably until it ends.
    pub fn begin(&mut self) -> Tx<'_> {
        todo!()
    }
}

pub struct Tx<'a> {
    ledger: &'a mut Ledger,
    pending: Vec<String>,
    committed: bool,
}

impl Tx<'_> {
    /// Buffers an entry; the ledger does not see it before `commit`.
    pub fn write(&mut self, entry: &str) {
        todo!()
    }

    /// What this transaction would add.
    pub fn pending(&self) -> &[String] {
        todo!()
    }

    /// Makes every buffered entry part of the ledger, in order.
    pub fn commit(self) {
        todo!()
    }
}

impl Drop for Tx<'_> {
    /// Without a commit: count one rollback on the ledger and discard the buffer.
    fn drop(&mut self) {
        // Write this: the stub does nothing.
    }
}

/// Writes every item in one transaction and commits, returning how many were written. An empty
/// item is refused with `Err("empty entry at index N")` before anything is committed, so
/// nothing from this call reaches the ledger.
pub fn write_all(ledger: &mut Ledger, items: &[&str]) -> Result<usize, String> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex06_transaction.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex06_transaction::{Ledger, write_all};

#[test]
fn a_new_ledger_is_empty() {
    let ledger = Ledger::new();
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 0);
}

#[test]
fn commit_adds_the_entries_in_order() {
    let mut ledger = Ledger::new();
    let mut tx = ledger.begin();
    tx.write("a");
    tx.write("b");
    tx.commit();
    assert_eq!(ledger.entries(), ["a", "b"]);
    assert_eq!(ledger.rollbacks(), 0);
}

#[test]
fn entries_wait_in_the_transaction_until_commit() {
    let mut ledger = Ledger::new();
    let mut tx = ledger.begin();
    tx.write("a");
    assert_eq!(tx.pending(), ["a"]);
    tx.commit();
    assert_eq!(ledger.entries(), ["a"]);
}

#[test]
fn dropping_without_commit_discards_and_counts_a_rollback() {
    let mut ledger = Ledger::new();
    {
        let mut tx = ledger.begin();
        tx.write("lost");
    }
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn a_rolled_back_transaction_leaves_earlier_commits_alone() {
    let mut ledger = Ledger::new();
    let mut first = ledger.begin();
    first.write("kept");
    first.commit();
    drop({
        let mut second = ledger.begin();
        second.write("lost");
        second
    });
    let mut third = ledger.begin();
    third.write("also kept");
    third.commit();
    assert_eq!(ledger.entries(), ["kept", "also kept"]);
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn write_all_commits_everything_or_nothing() {
    let mut ledger = Ledger::new();
    assert_eq!(write_all(&mut ledger, &["a", "b", "c"]), Ok(3));
    assert_eq!(ledger.entries(), ["a", "b", "c"]);

    assert_eq!(
        write_all(&mut ledger, &["d", "e", "", "f"]),
        Err("empty entry at index 2".to_owned())
    );
    assert_eq!(ledger.entries(), ["a", "b", "c"]);
    assert_eq!(ledger.rollbacks(), 1);
}

#[test]
fn write_all_with_nothing_to_write_commits_nothing() {
    let mut ledger = Ledger::new();
    assert_eq!(write_all(&mut ledger, &[]), Ok(0));
    assert!(ledger.entries().is_empty());
    assert_eq!(ledger.rollbacks(), 0);
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex06_transaction;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex06_transaction --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex06_transaction 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 7 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex06_transaction.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex06_transaction 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 7 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex06_transaction` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/02-storage.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex06_transaction.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex06_transaction.rs        # 7
grep -n '@' src/ex06_transaction.rs tests/ex06_transaction.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex06_transaction.rs exercises/tests/ex06_transaction.rs
git commit -m "docs(exercises): exercise 6, transactions that roll back on drop (Rust: ownership, lifetimes, Drop)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 11: Exercise 7 — Store errors, From and the ? operator

**Files:**
- Create: `exercises/src/ex07_store_errors.rs`, `exercises/tests/ex07_store_errors.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex07_store_errors;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/02-storage.md` (Task 8).
- Produces: the public items `Conflict`, `unique_conflict`, `StoreError`, `corrupt`, `read_level`, `read_levels`, `read_level_text`, used only by this exercise's own test file.

**What the learner practises:** implementing `Display` and `Error` (with `source`), `From` for `?`, `impl Display` arguments, `u8::try_from`, `map_err`, collecting an iterator of `Result`s
**Difficulty:** two stars (about 30 minutes). **Tests:** 10. **`todo!()` bodies in the stub:** 8.

- [ ] **Step 1: Create the stub `exercises/src/ex07_store_errors.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 7: error types, `From` and the `?` operator.
//!
//! Practises: `docs/learning/02-storage.md`, section 5.
//! Mirrors: `smoke_domain::repos::{Conflict, StoreError}` and the helpers in
//! `smoke_storage_sqlite::{error, convert}` (`unique_conflict`, `corrupt`).
//! Run: `cargo test --test ex07_store_errors`
//!
//! Hint: `?` on a `Result<T, E>` returns early with `Err(From::from(e))`, so a `From` impl is
//! all it takes to convert one error type into another. `iter.map(f).collect::<Result<Vec<_>, _>>()`
//! stops at the first `Err`. For `Error::source`, return the boxed error as `&dyn Error`.

use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conflict {
    UsernameTaken,
    AnotherEventOpen,
    ChainForked,
    Duplicate,
}

/// SQLite words a unique-constraint failure like `UNIQUE constraint failed: users.username`.
/// Which column or index is named decides the conflict; anything unrecognised is `Duplicate`:
///
/// - mentions `users.username`: `UsernameTaken`
/// - mentions `events_one_open`: `AnotherEventOpen`
/// - mentions `audit_log.`: `ChainForked`
pub fn unique_conflict(message: &str) -> Conflict {
    todo!()
}

#[derive(Debug)]
pub enum StoreError {
    Conflict(Conflict),
    /// Stored data breaks a rule the app guarantees.
    Corrupt(String),
    /// The database itself failed; the original error is kept as the source.
    Backend(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for StoreError {
    /// `Conflict`: "that write conflicts with stored data"; `Corrupt`: "stored data is corrupt:
    /// <reason>"; `Backend`: "database error" (the details are in `source`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl Error for StoreError {
    /// Only `Backend` has a source.
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        todo!()
    }
}

impl From<Conflict> for StoreError {
    fn from(conflict: Conflict) -> Self {
        todo!()
    }
}

/// A `Corrupt` error carrying `reason`'s text.
pub fn corrupt(reason: impl fmt::Display) -> StoreError {
    todo!()
}

/// A level as stored: an `i64` from 0 to 10. Anything else is `Corrupt`, for example
/// "level 11 is out of range" or "level -1 is out of range". Use `u8::try_from` and `?`.
pub fn read_level(stored: i64) -> Result<u8, StoreError> {
    todo!()
}

/// All the levels, or the first corrupt one. Use `collect::<Result<_, _>>()`.
pub fn read_levels(stored: &[i64]) -> Result<Vec<u8>, StoreError> {
    todo!()
}

/// A level from text such as a form field or a column read as `TEXT`: parse it, then check it
/// like `read_level`. A parse failure is `Corrupt` too. Use `?` twice and `corrupt` for the
/// parse error.
pub fn read_level_text(stored: &str) -> Result<u8, StoreError> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex07_store_errors.rs`** with exactly this content (the
learner never edits it):

```rust
use std::error::Error;

use smoke_exercises::ex07_store_errors::{
    Conflict, StoreError, corrupt, read_level, read_level_text, read_levels, unique_conflict,
};

#[test]
fn the_named_column_decides_the_conflict() {
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: users.username"),
        Conflict::UsernameTaken
    );
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: index 'events_one_open'"),
        Conflict::AnotherEventOpen
    );
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: audit_log.prev_hash"),
        Conflict::ChainForked
    );
}

#[test]
fn an_unrecognised_unique_failure_is_a_duplicate() {
    assert_eq!(
        unique_conflict("UNIQUE constraint failed: invites.token_hash"),
        Conflict::Duplicate
    );
    assert_eq!(unique_conflict(""), Conflict::Duplicate);
}

#[test]
fn errors_print_a_short_message() {
    assert_eq!(
        StoreError::Conflict(Conflict::Duplicate).to_string(),
        "that write conflicts with stored data"
    );
    assert_eq!(
        corrupt("note is not trimmed").to_string(),
        "stored data is corrupt: note is not trimmed"
    );
    assert_eq!(
        StoreError::Backend("disk full".into()).to_string(),
        "database error"
    );
}

#[test]
fn only_a_backend_error_has_a_source() {
    let backend = StoreError::Backend("disk full".into());
    assert_eq!(backend.source().unwrap().to_string(), "disk full");
    assert!(StoreError::Conflict(Conflict::Duplicate).source().is_none());
    assert!(corrupt("x").source().is_none());
}

#[test]
fn a_conflict_converts_into_a_store_error() {
    let error: StoreError = Conflict::UsernameTaken.into();
    assert!(matches!(
        error,
        StoreError::Conflict(Conflict::UsernameTaken)
    ));
}

#[test]
fn corrupt_accepts_anything_that_displays() {
    assert!(matches!(corrupt(42), StoreError::Corrupt(reason) if reason == "42"));
    assert!(matches!(corrupt(String::from("a")), StoreError::Corrupt(reason) if reason == "a"));
}

#[test]
fn levels_from_0_to_10_are_read() {
    assert_eq!(read_level(0).unwrap(), 0);
    assert_eq!(read_level(10).unwrap(), 10);
}

#[test]
fn a_level_out_of_range_is_corrupt_data() {
    for stored in [11, -1, 300, i64::MAX] {
        let error = read_level(stored).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("stored data is corrupt: level {stored} is out of range")
        );
    }
}

#[test]
fn many_levels_stop_at_the_first_corrupt_one() {
    assert_eq!(read_levels(&[1, 2, 10]).unwrap(), [1, 2, 10]);
    assert_eq!(read_levels(&[]).unwrap(), Vec::<u8>::new());
    let error = read_levels(&[1, 99, -5]).unwrap_err();
    assert_eq!(
        error.to_string(),
        "stored data is corrupt: level 99 is out of range"
    );
}

#[test]
fn text_levels_parse_then_check() {
    assert_eq!(read_level_text("7").unwrap(), 7);
    assert_eq!(
        read_level_text("12").unwrap_err().to_string(),
        "stored data is corrupt: level 12 is out of range"
    );
    let error = read_level_text("seven").unwrap_err();
    assert!(error.to_string().starts_with("stored data is corrupt: "));
    assert!(matches!(error, StoreError::Corrupt(_)));
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex07_store_errors;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex07_store_errors --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex07_store_errors 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 10 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex07_store_errors.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex07_store_errors 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 10 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex07_store_errors` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/02-storage.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex07_store_errors.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex07_store_errors.rs        # 8
grep -n '@' src/ex07_store_errors.rs tests/ex07_store_errors.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex07_store_errors.rs exercises/tests/ex07_store_errors.rs
git commit -m "docs(exercises): exercise 7, store errors (Rust: Error, From, the ? operator)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---


### Task 12: Note 03 — web and auth

**Files:**
- Create: `docs/learning/03-web-and-auth.md`

**Interfaces:**
- Consumes: decisions #2, #24, #56 to #63, #64, #66, #67, #68, #69, #70, #72, #74, #75 in
  `docs/DECISIONS.md`; `docs/learning/hosting.md` (existing, to be linked); the code listed below.
- Produces: the note that Exercises 8 to 10 link back to (section 4 is Exercise 8, sections 9
  and 10 are Exercise 9, sections 3 and 7 are Exercise 10: keep this numbering).

**Covers:** axum routes, handlers and extractors; custom extractors; middleware; shared state
(`Arc`, `Mutex`, the `HashMap` entry API); CPU work off the async executor; database sessions;
errors and panics that become translated pages; Askama templates; Fluent translations; small
pure functions around the handlers; configuration, the command line and background tasks;
integration tests that drive the router; the small container image.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 03 — Web and auth

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 8 (a login limiter), 9 (request rules), 10 (middleware and error chains). Reading time: about
> 60 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **Axum: routes, handlers and extractors.**
   - In our code: `crates/web/src/lib.rs::router` (a `Router` built from `.route(path,
     get(handler))`, grouped with `.merge`, `.route_layer`, `.layer`, `.fallback`,
     `.with_state`); a handler such as `crates/web/src/routes/log.rs::show` (an `async fn` whose
     arguments are extractors: `State<AppState<S>>`, `CurrentUser`, `Tr`, `Query<ShowQuery>`; the
     return type `Result<Response, AppError>`); `crates/web/src/routes/entries.rs::submit` (the
     body-consuming extractor `Pairs` must be the last argument); the generic parameter
     `S: Store + Clone + 'static` on every handler (`Clone` because axum clones the state per
     request, `'static` because the futures run on tokio's thread pool and may not borrow local
     data); `crates/web/src/lib.rs::serve` (`axum::serve` with graceful shutdown).
   - Why: a handler's signature shows what it needs, and extractors reject bad requests before
     the body of the function runs. Alternatives: one big `match` on the path (what a router
     replaces); a web framework with macros on handlers (Rocket) or an actor model (Actix): the
     choice of axum is decision #2, mainly for being built on tokio, tower and hyper, which the
     rest of the ecosystem uses.
   - Read more: `https://docs.rs/axum/latest/axum/`,
     `https://docs.rs/axum/latest/axum/extract/index.html`,
     `https://docs.rs/axum/latest/axum/struct.Router.html`, the tokio tutorial
     `https://tokio.rs/tokio/tutorial`.
2. **Writing your own extractor.**
   - In our code: `crates/web/src/auth/mod.rs::CurrentUser` (implements `FromRequestParts` with
     `type Rejection = Redirect`, so a signed-out visitor is sent to `/login`, and
     `OptionalFromRequestParts` so a handler can ask for `Option<CurrentUser>`);
     `crates/web/src/i18n.rs::Tr` (`type Rejection = Infallible`: it never fails, English is the
     default); `crates/web/src/auth/client_ip.rs::ClientIp` (implemented for the concrete
     `AppState<S>` because it reads the configuration); `crates/web/src/forms.rs::Pairs`
     (implements `FromRequest`, the variant that may consume the body, by wrapping
     `Form<Vec<(String, String)>>`, #98).
   - Why: a small impl makes every handler's argument list read like a sentence, and the
     rejection type decides what a failure looks like. Alternatives: read
     `request.extensions()` by hand in every handler (repetition); the built-in
     `Extension<User>` (works, but the missing-user case is a plain 500).
   - Read more: `https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html`, std
     `Infallible` `https://doc.rust-lang.org/std/convert/enum.Infallible.html`.
3. **Middleware: closures that wrap a handler.** (Exercise 10)
   - In our code: `crates/web/src/auth/mod.rs::load_user` (loads the session's user, signs out an
     expired viewer, renews the session at most daily, picks the language),
     `crates/web/src/auth/mod.rs::require_user` (redirects, with `HX-Redirect` for htmx
     requests), `crates/web/src/security.rs::headers` (adds the security headers to every
     response), `crates/web/src/security.rs::check_origin` (the CSRF check),
     `crates/web/src/error.rs::render_errors`, `tower_http`'s `CatchPanicLayer`,
     `TraceLayer` and the request-id layers wired up in `crates/web/src/lib.rs::router`;
     `crates/web/src/request_id.rs::RandomRequestId` (implements tower-http's
     `MakeRequestId`). Layer order: in axum the layer added **last** is the **outermost**, so
     the request meets `render_errors`, then `check_origin`, then the session layer, then
     `load_user`, then `CatchPanicLayer`, then the handler.
   - Why: cross-cutting rules (sign-in, headers, CSRF) live in one place instead of in every
     handler. `middleware::from_fn` turns an `async fn(Request, Next) -> Response` into a layer;
     under it is the `tower::Service` / `Layer` pair, which is what to learn next if you want to
     write reusable middleware. Alternatives: check inside each handler (one forgotten check is
     a hole); implement `Service` by hand (more code for the same effect).
   - Read more: `https://docs.rs/axum/latest/axum/middleware/index.html`, `https://docs.rs/tower`,
     `https://docs.rs/tower-http`, Book ch. 13.1 `https://doc.rust-lang.org/book/ch13-01-closures.html`.
   - Try it: Exercise 10 `exercises/src/ex10_middleware.rs`.
4. **Shared state: `Arc`, `Mutex` and the `HashMap` entry API.** (Exercise 8)
   - In our code: `crates/web/src/state.rs::AppState` (a struct of `Arc`s: the store, `Arc<dyn
     Clock>`, `Arc<WebConfig>`, `Arc<LoginLimiter>`, `Arc<Semaphore>`; a hand-written `Clone`
     that clones the `Arc`s, which is cheap); `crates/web/src/auth/limiter.rs::LoginLimiter`
     (`Mutex<Tables>`: both counters sit behind one lock so "check, then count" is one step, #60);
     `crates/web/src/auth/limiter.rs::strike` (`map.entry(key).or_insert_with(...)`, then update
     through the `&mut`); `crates/web/src/auth/limiter.rs::is_locked` (generic over `K:
     Borrow<Q>` so a `HashMap<String, _>` can be searched with a `&str`);
     `crates/web/src/auth/limiter.rs::address_key` (IPv6 counted per /64, #75); the private
     `lock` method that recovers from a poisoned mutex with `PoisonError::into_inner`.
   - Why: handlers run concurrently on several threads, so shared mutable data needs a lock;
     `Arc` gives shared ownership across threads, `Mutex` gives exclusive access. A std `Mutex`
     (not tokio's) is right here because the lock is never held across an `.await`. Say what a
     *poisoned* mutex is (a thread panicked while holding the lock) and why recovering is
     correct for this data. Alternatives: a database table for the counters (survives restarts,
     but one more write per sign-in; fine only because production runs one instance, #74);
     `RwLock` (reads are not the common case here); channels or an actor task.
   - Read more: Book ch. 16.3 `https://doc.rust-lang.org/book/ch16-03-shared-state.html`, std
     `Mutex` `https://doc.rust-lang.org/std/sync/struct.Mutex.html`, std `Arc`
     `https://doc.rust-lang.org/std/sync/struct.Arc.html`, std `Entry`
     `https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html`, Book ch. 8.3
     `https://doc.rust-lang.org/book/ch08-03-hash-maps.html`.
   - Try it: Exercise 8 `exercises/src/ex08_login_limiter.rs`.
5. **CPU-heavy work off the async executor: Argon2.**
   - In our code: `crates/web/src/auth/password.rs::hash`, `crates/web/src/auth/password.rs::verify`
     (a deliberately slow function, #69; `verify` always does one Argon2 run, with a dummy hash
     when the user does not exist, so timing does not reveal accounts);
     `crates/web/src/auth/password.rs::run_limited` (waits for a semaphore permit, then
     `tokio::task::spawn_blocking`; the permit **moves into** the blocking thread);
     `crates/web/src/auth/password.rs::CONCURRENT_HASHES` (2: each run needs about 19 MiB and the
     instance has 512 MiB); `crates/web/src/auth/password.rs::DUMMY_HASH` (a `LazyLock`, forced at
     startup by `prepare_dummy_hash`); `crates/web/src/routes/login.rs::submit` (the whole flow).
   - Why: an async task must not run long computations: it would stop the thread from serving
     other requests. `spawn_blocking` uses a separate pool, and the semaphore stops a burst of
     sign-ins from exhausting memory. Alternatives: a plain `tokio::spawn` (blocks a worker
     thread); a dedicated hashing thread with a channel (more code); a cheaper algorithm
     (wrong tradeoff: slowness is the point).
   - Read more: `https://docs.rs/argon2`,
     `https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html`,
     `https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html`, std `LazyLock`
     `https://doc.rust-lang.org/std/sync/struct.LazyLock.html`, OWASP password storage
     `https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html`.
6. **Sessions stored in the database.**
   - In our code: `crates/web/src/sessions.rs::layer` (cookie name, `Secure`, `HttpOnly`,
     `SameSite::Lax`, 30 days on inactivity; `__Host-` prefix over https, #24, #62);
     `crates/web/src/sessions.rs::DbSessions` (implements tower-sessions' `SessionStore` over the
     domain's `SessionRepo`; `#[async_trait]` because that trait comes from another crate, #56);
     `crates/web/src/sessions.rs::id_hash` (only the SHA-256 of the session id is stored, #57);
     `crates/web/src/auth/mod.rs::start_session` (`session.cycle_id()` at sign-in so an id
     planted before sign-in is worthless: session fixation); `crates/web/src/auth/mod.rs::RENEW_AFTER`
     (renew at most daily, #59); the `save` method only updates, never recreates (#111).
   - Why: server-side sessions can be ended instantly (password change, expiry) while a signed
     cookie cannot; storing the hash means a database leak is not a set of live sessions.
     Alternatives: JWT cookies (cannot be revoked without a server-side list); an in-memory store
     (lost on every restart and on every Cloud Run instance).
   - Read more: `https://docs.rs/tower-sessions`, OWASP session management
     `https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html`.
7. **Errors and panics become pages.** (Exercise 10)
   - In our code: `crates/web/src/error.rs::AppError` (a hand-written enum; `From` impls for
     `StoreError`, `askama::Error`, `tower_sessions::session::Error`, `JoinError` so `?` works in
     handlers); the `IntoResponse` impl, which logs the cause chain and attaches a small marker
     (`ErrorPage`) to a bare status response;
     `crates/web/src/error.rs::render_errors` (the outermost middleware: if the response carries
     the marker, it replaces the body with a translated page; the language comes from the
     cookie or the browser, because no session may exist);
     `crates/web/src/error.rs::chain` (walks `Error::source`);
     `crates/web/src/error.rs::panic_response` (`Box<dyn Any + Send>` and `downcast_ref` to
     recover the message of a panic caught by `CatchPanicLayer`); `crates/web/src/routes/mod.rs::refusal`.
   - Why: a handler that fails returns a value, and one place decides how every failure looks
     (#68); the user sees only a request id, the log has the full chain. Alternatives: render the
     error page inside every handler; `thiserror` for `AppError` (it would work; the hand-written
     version is short, and the type needs a custom `IntoResponse` anyway).
   - Read more: `https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html`, std `Any`
     `https://doc.rust-lang.org/std/any/trait.Any.html`, std `catch_unwind`
     `https://doc.rust-lang.org/std/panic/fn.catch_unwind.html`, Book ch. 9.3
     `https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html`.
   - Try it: Exercise 10 `exercises/src/ex10_middleware.rs`.
8. **Templates: Askama.**
   - In our code: `crates/web/src/routes/log.rs::LogPage` (`#[derive(Template)]`,
     `#[template(path = "log.html")]`, fields the template reads);
     `crates/web/src/error.rs::page` (renders with a status);
     `crates/web/templates/` (HTML files; `base.html` is extended, `report_fields.html` is
     included); `crates/web/src/labels.rs::room` (view logic stays in Rust).
   - Why: Askama compiles the templates into Rust code, so a misspelled field or a type error is a
     build error, and output is HTML-escaped by default (pages are plain HTML forms, so no
     script is needed, #96). Alternatives: Tera or Handlebars (runtime-checked templates); Maud (HTML
     inside Rust macros); a single-page app with a JSON API (much more JavaScript to secure).
   - Read more: `https://docs.rs/askama`, htmx `https://htmx.org/docs/`.
9. **Translations: Fluent and choosing the language.** (Exercise 9)
   - In our code: `crates/web/src/i18n.rs::LOCALES` (the `static_loader!` macro embeds the
     `.ftl` files from `crates/web/locales/` at compile time, English fallback, #66);
     `crates/web/src/i18n.rs::Tr` (a `Copy` newtype around `Locale` with `t`, `t1` and `n` for
     plain, one-argument and plural messages); `crates/web/src/i18n.rs::negotiate` (account, then
     cookie, then `Accept-Language`, then English: a chain of `Option::or_else`);
     `crates/web/src/i18n.rs::from_accept_language` (quality values, `is_none_or`, and a **let
     chain**: `if let Ok(locale) = ... && quality > 0.0 && ...`, stable in edition 2024);
     the test `crates/web/src/i18n.rs::every_key_the_pages_use_is_translated` (scans templates and
     sources for keys and compares the three languages).
   - Why: Fluent handles plurals and gender per language, which `printf`-style strings cannot.
     Alternatives: `gettext` (older tooling); a Rust map of strings (no plural rules); the
     `rust-i18n` crate (a different file format).
   - Read more: `https://projectfluent.org/`, `https://docs.rs/fluent-templates`, std
     `Option` `https://doc.rust-lang.org/std/option/enum.Option.html`, Reference (let chains)
     `https://doc.rust-lang.org/reference/expressions/if-expr.html`.
   - Try it: Exercise 9 `exercises/src/ex09_request_rules.rs`.
10. **Small pure functions around the handlers.** (Exercise 9)
    - In our code: `crates/web/src/security.rs::is_same_origin` (decides from `Sec-Fetch-Site`,
      else `Origin`; a request with neither is refused, #63) and its unit tests;
      `crates/web/src/security.rs::CSP` (a `const &str`, written as a literal with `\` line
      continuations); `crates/web/src/auth/client_ip.rs::resolve` (the **last** entry of
      `X-Forwarded-For`, because that is the one the trusted proxy added, #61, #74);
      `crates/web/src/lib.rs::logged_path` (returns either a `&'static str` literal or the input:
      the signature `fn(&str) -> &str` lets both through, #119); `crates/web/src/config.rs::BaseUrl::parse`
      (an `http::Uri` parsed and normalised into a newtype, #62).
    - Why: each of these decisions is a few lines with many edge cases; as plain functions
      from values to a value they are tested without a server, and the handler stays thin.
      Alternative: put the logic in the middleware body (testable only through a whole request).
    - Read more: MDN CSP `https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP`, MDN
      `Sec-Fetch-Site` `https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Sec-Fetch-Site`,
      OWASP CSRF `https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html`,
      Book ch. 10.3 `https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html` (why `logged_path`
      needs no lifetime annotation).
    - Try it: Exercise 9 `exercises/src/ex09_request_rules.rs`.
11. **Configuration, the command line and background tasks.**
    - In our code: `crates/app/src/cli.rs::Cli` and `crates/app/src/cli.rs::Command` (clap's
      derive API: doc comments become `--help`; `value_parser = parse_locale`; a test that a
      password cannot be passed as an argument, #73); `crates/app/src/config.rs::ServeConfig`
      with `from_lookup(get: impl Fn(&str) -> Option<String>)` (the environment is passed in as a
      closure, so tests need no real environment variables); `crates/app/src/lib.rs::run` (`match`
      on the command; exit code 1 for `verify-audit` problems, 2 for errors);
      `crates/app/src/lib.rs::init_logging` (`tracing_subscriber` with `EnvFilter`, logs on
      stderr, colours only on a terminal); `crates/app/src/commands/serve.rs::run`
      (`tokio::spawn` for three sweeps, each a loop over `tokio::time::interval`;
      `JoinHandle::abort` at shutdown) and `crates/app/src/commands/serve.rs::shutdown_signal`
      (`tokio::select!`-style waiting on Ctrl-C and SIGTERM); `crates/app/src/backend.rs::with_store`
      (a macro that runs one expression with whichever store was chosen, so every command is
      compiled twice and no `dyn` is involved, #67).
    - Why: a thin `main.rs` plus a library (`smoke_app`, #72) makes the command code testable
      (`crates/app/tests/commands.rs`). Alternatives: `std::env::var` calls scattered in the
      code; a config file (a second place for secrets).
    - Read more: `https://docs.rs/clap/latest/clap/_derive/index.html`, `https://docs.rs/tracing`,
      `https://docs.rs/tracing-subscriber`, `https://docs.rs/tokio`, std `ExitCode`
      `https://doc.rust-lang.org/std/process/struct.ExitCode.html`.
12. **Integration tests that drive the router.**
    - In our code: `crates/web/tests/common/mod.rs::TestApp` (a real router over a temporary
      SQLite file and a `FakeClock`, driven with in-memory requests via `tower::ServiceExt::oneshot`,
      no network port); the test files in `crates/web/tests/` (for example
      `crates/web/tests/security.rs::no_page_needs_inline_scripts_styles_or_eval`, which scans
      every page's HTML for inline scripts or styles).
    - Why: a request-level test covers routing, extractors, middleware, templates and the
      database in one go, in milliseconds, without a browser. Alternatives: only unit tests
      (miss wiring bugs); end-to-end browser tests (slow, flaky, a toolchain to maintain).
    - Read more: Book ch. 11.3 `https://doc.rust-lang.org/book/ch11-03-test-organization.html`,
      `https://docs.rs/tower`, `https://docs.rs/http-body-util`.
13. **A small image: Docker and the release build.**
    - In our code: `Dockerfile` (two stages: build on `rust:1-alpine`, run on distroless; a
      static musl binary; `cargo auditable build --release --locked`; user 65532; `ENTRYPOINT`
      and `CMD ["serve"]`); `.dockerignore` (why `target`, `.git` and `docs` stay out of the
      build context); `.github/workflows/deploy.yml` and `.github/workflows/security.yml` (Trivy
      scan of the image, #70); the profile `[profile.release]` in `Cargo.toml` (`lto = "thin"`,
      `strip = true`, `codegen-units = 1`: smaller and faster, slower to build). Link to
      [the hosting note](hosting.md) for where the image runs and why.
    - Why: a free tier means a small image and a small binary; a static binary needs no
      libc in the image. Alternatives: a Debian base (hundreds of MB); building on the host.
    - Read more: the existing note `docs/learning/hosting.md` as a Markdown link
      (`[Hosting](hosting.md)`), `https://doc.rust-lang.org/cargo/reference/workspaces.html`
      (profiles are workspace-wide).

**Check yourself** (use or adapt; the note needs five or six): Why must `Pairs` be the last
argument of `submit`? In what order do the layers see a request, and where would a new "log every
request" layer go if it had to see the error pages too? Why does `LoginLimiter::begin` count an
attempt before the password is checked? What breaks if the password work is done directly in
the handler, and what if the semaphore is removed? Why does the app store the hash of the
session id instead of the id? What does `ErrorPage` do that returning an HTML page directly from
`AppError::into_response` could not?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/03-web-and-auth.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/03-web-and-auth.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/03-web-and-auth.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-03.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-03.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/03-web-and-auth.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/03-web-and-auth.md
git commit -m "docs(learning): note 03, axum extractors and middleware, shared state and database sessions

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---

### Task 13: Exercise 8 — A sign-in limiter with a HashMap, shared between threads

**Files:**
- Create: `exercises/src/ex08_login_limiter.rs`, `exercises/tests/ex08_login_limiter.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex08_login_limiter;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/03-web-and-auth.md` (Task 12).
- Produces: the public items `Rule`, `PER_USERNAME`, `Limiter`, `SharedLimiter`, used only by this exercise's own test file.

**What the learner practises:** `HashMap::entry().or_insert_with`, `Copy` structs updated through `&mut`, `Option::is_some_and`, a `Mutex` wrapper whose methods take `&self`, `MutexGuard`, recovering from a poisoned lock, `Arc` and threads in the tests
**Difficulty:** three stars (about 50 minutes). **Tests:** 9. **`todo!()` bodies in the stub:** 7.

- [ ] **Step 1: Create the stub `exercises/src/ex08_login_limiter.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 8: a sign-in limiter with a `HashMap`, and the same limiter shared between threads.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, section 4.
//! Mirrors: `smoke_web::auth::limiter::{LoginLimiter, Rule, PER_USERNAME}`.
//! Run: `cargo test --test ex08_login_limiter`
//!
//! The rule: after `max_failures` attempts for one key within `window` seconds, the key is
//! locked for `lockout` seconds. Every admitted attempt is counted as a failure at once, before
//! the password is checked (so ten parallel guesses cannot all slip past the check);
//! `succeeded` takes the count back by forgetting the key.
//!
//! Hint: `map.entry(key.to_owned()).or_insert_with(|| Strikes::new(now))` finds or creates the
//! entry in one lookup and gives you `&mut Strikes`. An entry whose window has passed (and which
//! is not locked) starts over. The attempt that reaches `max_failures` is still admitted; the
//! lock applies from the next one. For `SharedLimiter`, `Mutex::lock` returns a `Result`
//! because a thread that panicked while holding the lock poisons it; this limiter's state is
//! always consistent, so use `unwrap_or_else(PoisonError::into_inner)`.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rule {
    pub max_failures: u32,
    /// Seconds.
    pub window: u64,
    /// Seconds.
    pub lockout: u64,
}

/// 5 attempts within 15 minutes lock a username for 15 minutes.
pub const PER_USERNAME: Rule = Rule {
    max_failures: 5,
    window: 15 * 60,
    lockout: 15 * 60,
};

#[derive(Debug, Clone, Copy)]
struct Strikes {
    failures: u32,
    since: u64,
    locked_until: Option<u64>,
}

impl Strikes {
    fn new(now: u64) -> Self {
        Self {
            failures: 0,
            since: now,
            locked_until: None,
        }
    }
}

#[derive(Debug)]
pub struct Limiter {
    rule: Rule,
    strikes: HashMap<String, Strikes>,
}

impl Limiter {
    pub fn new(rule: Rule) -> Self {
        todo!()
    }

    /// Admits an attempt for `key` at second `now`, or refuses it while the key is locked.
    pub fn begin(&mut self, key: &str, now: u64) -> bool {
        todo!()
    }

    /// A correct password: forget the key's failures.
    pub fn succeeded(&mut self, key: &str) {
        todo!()
    }
}

/// The same limiter behind `&self`, so several threads (or requests) can share one.
#[derive(Debug)]
pub struct SharedLimiter {
    inner: Mutex<Limiter>,
}

impl SharedLimiter {
    pub fn new(rule: Rule) -> Self {
        todo!()
    }

    pub fn begin(&self, key: &str, now: u64) -> bool {
        todo!()
    }

    pub fn succeeded(&self, key: &str) {
        todo!()
    }

    fn lock(&self) -> MutexGuard<'_, Limiter> {
        todo!("lock, ignoring poisoning")
    }
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex08_login_limiter.rs`** with exactly this content (the
learner never edits it):

```rust
use std::sync::Arc;
use std::thread;

use smoke_exercises::ex08_login_limiter::{Limiter, PER_USERNAME, SharedLimiter};

const T: u64 = 1_000;

fn attempts(limiter: &mut Limiter, key: &str, count: usize, now: u64) -> Vec<bool> {
    (0..count).map(|_| limiter.begin(key, now)).collect()
}

#[test]
fn the_fifth_attempt_is_admitted_and_the_sixth_refused() {
    let mut limiter = Limiter::new(PER_USERNAME);
    assert_eq!(
        attempts(&mut limiter, "alice", 7, T),
        [true, true, true, true, true, false, false]
    );
}

#[test]
fn the_lock_lasts_exactly_the_lockout() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 5, T);
    assert!(!limiter.begin("alice", T + 899));
    assert!(limiter.begin("alice", T + 900));
}

#[test]
fn failures_older_than_the_window_are_forgotten() {
    let mut limiter = Limiter::new(PER_USERNAME);
    assert_eq!(attempts(&mut limiter, "alice", 4, T), [true; 4]);
    assert_eq!(
        attempts(&mut limiter, "alice", 6, T + 901),
        [true, true, true, true, true, false]
    );
}

#[test]
fn failures_inside_the_window_add_up() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 3, T);
    assert_eq!(
        attempts(&mut limiter, "alice", 3, T + 600),
        [true, true, false]
    );
}

#[test]
fn keys_are_counted_separately() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 5, T);
    assert!(!limiter.begin("alice", T));
    assert!(limiter.begin("bob", T));
}

#[test]
fn a_correct_password_clears_the_count() {
    let mut limiter = Limiter::new(PER_USERNAME);
    attempts(&mut limiter, "alice", 4, T);
    limiter.succeeded("alice");
    assert_eq!(
        attempts(&mut limiter, "alice", 6, T),
        [true, true, true, true, true, false]
    );
}

#[test]
fn succeeding_for_an_unknown_key_changes_nothing() {
    let mut limiter = Limiter::new(PER_USERNAME);
    limiter.succeeded("nobody");
    assert!(limiter.begin("nobody", T));
}

#[test]
fn a_shared_limiter_admits_exactly_five_of_many_parallel_attempts() {
    let limiter = Arc::new(SharedLimiter::new(PER_USERNAME));
    let handles: Vec<_> = (0..16)
        .map(|_| {
            let limiter = Arc::clone(&limiter);
            thread::spawn(move || limiter.begin("alice", T))
        })
        .collect();
    let admitted = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|&admitted| admitted)
        .count();
    assert_eq!(admitted, 5);
}

#[test]
fn a_shared_limiter_forgets_a_key_after_a_success() {
    let limiter = SharedLimiter::new(PER_USERNAME);
    for _ in 0..4 {
        assert!(limiter.begin("alice", T));
    }
    limiter.succeeded("alice");
    for _ in 0..5 {
        assert!(limiter.begin("alice", T));
    }
    assert!(!limiter.begin("alice", T));
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex08_login_limiter;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex08_login_limiter --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex08_login_limiter 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 9 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex08_login_limiter.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex08_login_limiter 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 9 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex08_login_limiter` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/03-web-and-auth.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex08_login_limiter.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex08_login_limiter.rs        # 7
grep -n '@' src/ex08_login_limiter.rs tests/ex08_login_limiter.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex08_login_limiter.rs exercises/tests/ex08_login_limiter.rs
git commit -m "docs(exercises): exercise 8, sign-in limiter (Rust: HashMap entry API, Mutex, Arc)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 14: Exercise 9 — Small pure functions that decide things about a request

**Files:**
- Create: `exercises/src/ex09_request_rules.rs`, `exercises/tests/ex09_request_rules.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex09_request_rules;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/03-web-and-auth.md` (Task 12).
- Produces: the public items `Locale` (with a given `FromStr`), `negotiate`, `from_accept_language`, `is_same_origin`, `client_ip`, `logged_path`, used only by this exercise's own test file.

**What the learner practises:** `Option` combinators (`or_else`, `and_then`, `map_or`, `is_some_and`), string splitting and parsing, a function returning `&str` that is either its input or a literal, `std::net::IpAddr`
**Difficulty:** two stars (about 45 minutes). **Tests:** 13. **`todo!()` bodies in the stub:** 5.

- [ ] **Step 1: Create the stub `exercises/src/ex09_request_rules.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 9: small pure functions that decide things about a request.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, sections 9 and 10.
//! Mirrors: `smoke_web::i18n::{negotiate, from_accept_language}`,
//! `smoke_web::security::is_same_origin`, `smoke_web::auth::client_ip::resolve` and
//! `smoke_web::logged_path`.
//! Run: `cargo test --test ex09_request_rules`
//!
//! Hint: none of these need a loop over indices. `Option::or_else`, `and_then`, `map_or`,
//! `is_some_and`, `str::split`, `str::rsplit`, `strip_prefix` and `parse` do most of the work.

use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    It,
    Tr,
}

/// Given: "en", "it" or "tr" (nothing else, no case folding).
impl FromStr for Locale {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "en" => Ok(Self::En),
            "it" => Ok(Self::It),
            "tr" => Ok(Self::Tr),
            _ => Err(()),
        }
    }
}

/// The language of a request: the account's language when signed in, else the `lang` cookie,
/// else the browser's `Accept-Language`, else English. An unusable value is skipped, never an
/// error. The cookie is trimmed before parsing.
pub fn negotiate(
    account: Option<Locale>,
    cookie: Option<&str>,
    accept_language: Option<&str>,
) -> Locale {
    todo!()
}

/// The supported language with the highest quality in an `Accept-Language` value such as
/// `"fr;q=0.9, it-IT;q=0.8, tr;q=0.8"`; the first of equals wins. A missing `q` is 1.0, a
/// quality of 0 (or one that is not a number) excludes the language, the region part (`-IT`)
/// and the case of the tag are ignored.
pub fn from_accept_language(header: &str) -> Option<Locale> {
    todo!()
}

/// Whether a state-changing request comes from the app itself. `Sec-Fetch-Site` decides when
/// the browser sent it (only `same-origin` passes); otherwise `Origin` must equal `app_origin`,
/// ignoring case. A request with neither is refused.
pub fn is_same_origin(
    sec_fetch_site: Option<&str>,
    origin: Option<&str>,
    app_origin: &str,
) -> bool {
    todo!()
}

/// The client's address behind a trusted proxy: `X-Forwarded-For: client, proxy1` lists what
/// each hop saw, and only the LAST entry was added by the proxy we trust. A missing or
/// unparsable header falls back to the TCP `peer`, and with no peer either to `0.0.0.0`.
pub fn client_ip(forwarded_for: Option<&str>, peer: Option<IpAddr>) -> IpAddr {
    todo!()
}

/// The path as the logs may show it: an invite link's token works like a password until it is
/// used, so everything under `/invite/` is shown as `/invite/…` (with the real ellipsis
/// character). Other paths are returned as they are, without copying.
pub fn logged_path(path: &str) -> &str {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex09_request_rules.rs`** with exactly this content (the
learner never edits it):

```rust
use std::net::{IpAddr, Ipv4Addr};

use smoke_exercises::ex09_request_rules::{
    Locale, client_ip, from_accept_language, is_same_origin, logged_path, negotiate,
};

const APP: &str = "https://smoke.example.org";

#[test]
fn the_account_language_wins_over_everything() {
    assert_eq!(
        negotiate(Some(Locale::Tr), Some("it"), Some("en")),
        Locale::Tr
    );
}

#[test]
fn without_an_account_the_cookie_then_the_browser_decide() {
    assert_eq!(negotiate(None, Some(" it "), Some("tr")), Locale::It);
    assert_eq!(negotiate(None, None, Some("tr, en;q=0.5")), Locale::Tr);
    assert_eq!(negotiate(None, None, None), Locale::En);
}

#[test]
fn an_unusable_cookie_is_skipped_not_an_error() {
    assert_eq!(negotiate(None, Some("klingon"), Some("tr")), Locale::Tr);
    assert_eq!(negotiate(None, Some(""), None), Locale::En);
    assert_eq!(negotiate(None, Some("klingon"), Some("de")), Locale::En);
}

#[test]
fn the_highest_quality_supported_language_wins() {
    assert_eq!(
        from_accept_language("fr;q=0.9, it;q=0.8, tr;q=0.7"),
        Some(Locale::It)
    );
    assert_eq!(from_accept_language("it;q=0.5, tr;q=0.9"), Some(Locale::Tr));
}

#[test]
fn the_first_of_equals_wins_and_a_missing_quality_is_one() {
    assert_eq!(from_accept_language("tr, it"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=0.9, tr"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=0.8, tr;q=0.8"), Some(Locale::It));
}

#[test]
fn regions_and_case_are_ignored() {
    assert_eq!(from_accept_language("IT-ch"), Some(Locale::It));
    assert_eq!(from_accept_language("tr-TR,tr;q=0.9"), Some(Locale::Tr));
}

#[test]
fn a_zero_or_broken_quality_excludes_the_language() {
    assert_eq!(from_accept_language("it;q=0, tr;q=0.1"), Some(Locale::Tr));
    assert_eq!(from_accept_language("it;q=oops"), None);
    assert_eq!(from_accept_language("de, fr;q=0.8, *;q=0.1"), None);
    assert_eq!(from_accept_language(""), None);
}

#[test]
fn fetch_metadata_decides_when_the_browser_sent_it() {
    assert!(is_same_origin(Some("same-origin"), None, APP));
    for site in ["same-site", "cross-site", "none"] {
        assert!(!is_same_origin(Some(site), Some(APP), APP), "{site}");
    }
}

#[test]
fn without_fetch_metadata_the_origin_must_match_ignoring_case() {
    assert!(is_same_origin(None, Some(APP), APP));
    assert!(is_same_origin(None, Some("https://SMOKE.example.org"), APP));
    assert!(!is_same_origin(None, Some("https://evil.example"), APP));
    assert!(!is_same_origin(None, Some("null"), APP));
}

#[test]
fn a_request_with_neither_header_is_refused() {
    assert!(!is_same_origin(None, None, APP));
}

#[test]
fn the_proxys_entry_is_the_last_one() {
    let peer = Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)));
    assert_eq!(
        client_ip(Some("203.0.113.9"), peer),
        "203.0.113.9".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        client_ip(Some("6.6.6.6, 203.0.113.9"), peer),
        "203.0.113.9".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        client_ip(Some("1.1.1.1,  2001:db8::7 "), peer),
        "2001:db8::7".parse::<IpAddr>().unwrap()
    );
}

#[test]
fn a_missing_or_broken_header_falls_back_to_the_peer_then_to_nothing() {
    let peer = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    assert_eq!(client_ip(None, Some(peer)), peer);
    assert_eq!(client_ip(Some("not an address"), Some(peer)), peer);
    assert_eq!(client_ip(Some(""), Some(peer)), peer);
    assert_eq!(client_ip(None, None), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
}

#[test]
fn invite_tokens_are_cut_from_logged_paths() {
    assert_eq!(logged_path("/invite/0123abcd"), "/invite/…");
    assert_eq!(logged_path("/events/3"), "/events/3");
    assert_eq!(logged_path("/people/invite"), "/people/invite");
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex09_request_rules;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex09_request_rules --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex09_request_rules 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 13 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex09_request_rules.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex09_request_rules 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 13 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex09_request_rules` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/03-web-and-auth.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex09_request_rules.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex09_request_rules.rs        # 5
grep -n '@' src/ex09_request_rules.rs tests/ex09_request_rules.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex09_request_rules.rs exercises/tests/ex09_request_rules.rs
git commit -m "docs(exercises): exercise 9, request rules as pure functions (Rust: Option combinators, string parsing)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 15: Exercise 10 — Middleware as closures, and the chain of an error

**Files:**
- Create: `exercises/src/ex10_middleware.rs`, `exercises/tests/ex10_middleware.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex10_middleware;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/03-web-and-auth.md` (Task 12).
- Produces: the public items `Request`, `Response`, `Handler`, `require_user`, `security_headers`, `chain`, `panic_message`, used only by this exercise's own test file.

**What the learner practises:** `Box<dyn Fn>` handlers, closures that `move` a captured handler and flag, higher-order functions returning closures, walking `Error::source`, `Box<dyn Any>::downcast_ref`
**Difficulty:** three stars (about 50 minutes). **Tests:** 13. **`todo!()` bodies in the stub:** 4.

- [ ] **Step 1: Create the stub `exercises/src/ex10_middleware.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 10: middleware as closures, and errors on their way out.
//!
//! Practises: `docs/learning/03-web-and-auth.md`, sections 3 and 7.
//! Mirrors: `smoke_web::auth::require_user`, `smoke_web::security::headers`,
//! `smoke_web::error::{chain, panic_response}`.
//! Run: `cargo test --test ex10_middleware`
//!
//! Hint: a middleware takes the next handler and returns a new handler that may act before and
//! after calling it. Return `Box::new(move |request| { ... })`: `move` hands the closure its own
//! copy of `next` (and any flag), and `Box<dyn Fn>` lets every layer have the same type. `Fn`
//! (not `FnMut`) is enough because the layers keep no state of their own.

use std::any::Any;
use std::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub path: String,
    pub signed_in: bool,
}

impl Request {
    pub fn new(path: &str, signed_in: bool) -> Self {
        Self {
            path: path.to_owned(),
            signed_in,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    pub fn new(status: u16, body: &str) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: body.to_owned(),
        }
    }

    /// Given: a `303 See Other` to `location`.
    pub fn redirect(location: &str) -> Self {
        let mut response = Self::new(303, "");
        response.set_header("location", location);
        response
    }

    /// Given: the value of a header (names are lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Given: sets a header, replacing an earlier one of the same name.
    pub fn set_header(&mut self, name: &str, value: &str) {
        self.headers.retain(|(key, _)| key != name);
        self.headers.push((name.to_owned(), value.to_owned()));
    }
}

pub type Handler = Box<dyn Fn(&Request) -> Response>;

/// A signed-out request never reaches `next`: it is redirected to `/login`.
pub fn require_user(next: Handler) -> Handler {
    todo!()
}

/// Calls `next`, then adds to its response (replacing any header of the same name):
///
/// - `x-content-type-options: nosniff` and `x-frame-options: DENY`
/// - `referrer-policy: no-referrer` for paths under `/invite/`, `same-origin` for the rest
/// - `strict-transport-security: max-age=63072000`, only when `https` is true
/// - `cache-control: no-store`, only if the response did not set its own
pub fn security_headers(https: bool, next: Handler) -> Handler {
    todo!()
}

/// An error and all its sources, joined with ": ", for example
/// `database error: no such table: users`. Walk `Error::source` until it returns `None`.
pub fn chain(error: &(dyn Error + 'static)) -> String {
    todo!()
}

/// The message of a caught panic (what `std::panic::catch_unwind` returns in its `Err`). A
/// `panic!("literal")` carries a `&'static str`, a `panic!("{x}")` a `String`; anything else
/// becomes "a handler panicked". Use `Box<dyn Any>::downcast_ref`.
pub fn panic_message(panic: Box<dyn Any + Send + 'static>) -> String {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex10_middleware.rs`** with exactly this content (the
learner never edits it):

```rust
use std::cell::Cell;
use std::error::Error;
use std::fmt;
use std::panic::{AssertUnwindSafe, catch_unwind, panic_any};
use std::rc::Rc;

use smoke_exercises::ex10_middleware::{
    Handler, Request, Response, chain, panic_message, require_user, security_headers,
};

fn page(calls: Rc<Cell<u32>>) -> Handler {
    Box::new(move |_| {
        calls.set(calls.get() + 1);
        Response::new(200, "the page")
    })
}

#[test]
fn a_signed_in_request_reaches_the_page() {
    let calls = Rc::new(Cell::new(0));
    let app = require_user(page(calls.clone()));
    let response = app(&Request::new("/events", true));
    assert_eq!(response, Response::new(200, "the page"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_signed_out_request_is_redirected_and_never_reaches_the_page() {
    let calls = Rc::new(Cell::new(0));
    let app = require_user(page(calls.clone()));
    let response = app(&Request::new("/events", false));
    assert_eq!(response.status, 303);
    assert_eq!(response.header("location"), Some("/login"));
    assert_eq!(calls.get(), 0);
}

#[test]
fn security_headers_are_added_to_a_response() {
    let app = security_headers(false, page(Rc::default()));
    let response = app(&Request::new("/", true));
    assert_eq!(response.body, "the page");
    assert_eq!(response.header("x-content-type-options"), Some("nosniff"));
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    assert_eq!(response.header("referrer-policy"), Some("same-origin"));
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(response.header("strict-transport-security"), None);
}

#[test]
fn strict_transport_security_only_over_https() {
    let app = security_headers(true, page(Rc::default()));
    let response = app(&Request::new("/", true));
    assert_eq!(
        response.header("strict-transport-security"),
        Some("max-age=63072000")
    );
}

#[test]
fn invite_links_send_no_referrer() {
    let app = security_headers(true, page(Rc::default()));
    let response = app(&Request::new("/invite/0123abcd", false));
    assert_eq!(response.header("referrer-policy"), Some("no-referrer"));
    let other = app(&Request::new("/people/invite", false));
    assert_eq!(other.header("referrer-policy"), Some("same-origin"));
}

#[test]
fn the_pages_own_cache_control_is_kept_but_other_headers_are_replaced() {
    let app = security_headers(
        false,
        Box::new(|_| {
            let mut response = Response::new(200, "asset");
            response.set_header("cache-control", "no-cache");
            response.set_header("x-frame-options", "SAMEORIGIN");
            response
        }),
    );
    let response = app(&Request::new("/static/app.css", true));
    assert_eq!(response.header("cache-control"), Some("no-cache"));
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    let frames = response
        .headers
        .iter()
        .filter(|(name, _)| name == "x-frame-options")
        .count();
    assert_eq!(frames, 1);
}

#[test]
fn layers_nest_so_a_redirect_gets_the_security_headers_too() {
    let calls = Rc::new(Cell::new(0));
    let app = security_headers(true, require_user(page(calls.clone())));
    let response = app(&Request::new("/events", false));
    assert_eq!(response.status, 303);
    assert_eq!(response.header("x-frame-options"), Some("DENY"));
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(calls.get(), 0);
}

#[derive(Debug)]
struct Layer {
    text: &'static str,
    source: Option<Box<Layer>>,
}

impl fmt::Display for Layer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.text)
    }
}

impl Error for Layer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source
            .as_deref()
            .map(|layer| layer as &(dyn Error + 'static))
    }
}

#[test]
fn an_error_prints_with_all_its_causes() {
    let error = Layer {
        text: "database error",
        source: Some(Box::new(Layer {
            text: "no such table: users",
            source: None,
        })),
    };
    assert_eq!(chain(&error), "database error: no such table: users");
}

#[test]
fn an_error_without_a_cause_is_just_its_message() {
    let error = Layer {
        text: "not found",
        source: None,
    };
    assert_eq!(chain(&error), "not found");
}

#[test]
fn three_levels_are_all_listed() {
    let error = Layer {
        text: "request failed",
        source: Some(Box::new(Layer {
            text: "database error",
            source: Some(Box::new(Layer {
                text: "disk full",
                source: None,
            })),
        })),
    };
    assert_eq!(chain(&error), "request failed: database error: disk full");
}

fn caught(work: impl FnOnce()) -> Box<dyn std::any::Any + Send> {
    catch_unwind(AssertUnwindSafe(work)).unwrap_err()
}

#[test]
fn a_literal_panic_message_is_recovered() {
    assert_eq!(panic_message(caught(|| panic!("boom"))), "boom");
}

#[test]
fn a_formatted_panic_message_is_recovered() {
    let id = 7;
    assert_eq!(
        panic_message(caught(|| panic!("event {id} vanished"))),
        "event 7 vanished"
    );
}

#[test]
fn an_unknown_panic_payload_gets_a_generic_message() {
    assert_eq!(
        panic_message(caught(|| panic_any(42_u32))),
        "a handler panicked"
    );
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex10_middleware;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex10_middleware --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex10_middleware 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 13 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex10_middleware.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex10_middleware 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 13 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex10_middleware` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/03-web-and-auth.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex10_middleware.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex10_middleware.rs        # 4
grep -n '@' src/ex10_middleware.rs tests/ex10_middleware.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex10_middleware.rs exercises/tests/ex10_middleware.rs
git commit -m "docs(exercises): exercise 10, middleware as closures (Rust: Box<dyn Fn>, move closures, error sources)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---


### Task 16: Note 04 — the logging flow

**Files:**
- Create: `docs/learning/04-logging-flow.md`

**Interfaces:**
- Consumes: decisions #34, #86, #87, #88, #89, #96, #98, #105 to #108 in `docs/DECISIONS.md`; the
  code listed below.
- Produces: the note that Exercises 11 to 13 link back to (section 2 is Exercise 11, section 4 is
  Exercise 12, section 6 is Exercise 13: keep this numbering).

**Covers:** use cases as generic functions over a write transaction; business rules as pure
functions; a question that travels with its answer; a value that normalises its input once;
errors that become form messages; forms with repeated names and borrowed data (lifetimes);
view models and templates; small language features (`let … else`, let chains, `is_none_or`,
`matches!`); the catch-up sweep and one transaction for many writes.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 04 — The logging flow

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 11 (log prompts), 12 (report values), 13 (form pairs). Reading time: about
> 55 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **Use cases as generic functions over a write transaction.**
   - In our code: `crates/domain/src/logbook.rs::log_smell` (signature: `tx: &mut T` with `where
     T: EventWrites + ReportWrites + ReadingWrites`; permission check first, then the
     decision, then the writes), `crates/domain/src/logbook.rs::close_event`,
     `crates/domain/src/logbook.rs::reopen_event`, `crates/domain/src/logbook.rs::correct_report`,
     `crates/domain/src/logbook.rs::move_report`; the caller in
     `crates/web/src/routes/entries.rs::submit` (`begin`, call the use case, `commit`); the doc
     comment on `crates/domain/src/repos.rs::EventWrites` ("Reads inside the transaction, so a
     decision and the write that follows it see the same data").
   - Why: the rules (spec §7) live in the domain crate and run identically on SQLite and
     Postgres; because the use case receives the transaction, the caller decides when to commit
     and everything the use case wrote is applied together or not at all (#86). Alternatives:
     methods on the store (hides the transaction boundary); a service struct holding a
     connection pool (harder to test); the rules in SQL (two dialects, no unit tests).
   - Read more: Book ch. 10.1 `https://doc.rust-lang.org/book/ch10-01-syntax.html`, ch. 10.2
     `https://doc.rust-lang.org/book/ch10-02-traits.html`.
2. **Business rules as pure functions.** (Exercise 11)
   - In our code: `crates/domain/src/events.rs::classify_observed` (returns `Result<ObservedTime,
     DomainError>`), `crates/domain/src/events.rs::log_prompt` (one `match` on the tuple
     `(observed, open_event, unseen_auto_closed)`, one arm per case),
     `crates/domain/src/events.rs::no_smell_prompt`, `crates/domain/src/events.rs::new_event_state`,
     `crates/domain/src/events.rs::can_join_backdated`,
     `crates/domain/src/events.rs::large_gap_indices` (`windows(2)`, `enumerate`, `filter`,
     `map`, `collect`); the thresholds as `const` `TimeDelta` values
     (`crates/domain/src/events.rs::BACKDATE_THRESHOLD`, `FUTURE_TOLERANCE`, `LARGE_GAP`).
   - Why: a rule that takes values and returns a value needs no database, clock or mock to test,
     and a `match` on a tuple lets the compiler prove that every combination is decided. The spec
     (§7) is written as a table; the code is the same table. Alternatives: `if` chains (easy to
     leave a case out); a state-machine crate (too heavy for four states).
   - Read more: Book ch. 6.2 `https://doc.rust-lang.org/book/ch06-02-match.html`, std slice
     `https://doc.rust-lang.org/std/primitive.slice.html`, Book ch. 13.2
     `https://doc.rust-lang.org/book/ch13-02-iterators.html`.
   - Try it: Exercise 11 `exercises/src/ex11_log_prompt.rs`.
3. **A question that travels with its answer.**
   - In our code: `crates/domain/src/logbook.rs::EventChoice` (`New { replacing:
     Option<EventId> }`, `Join(EventId)`, `Continue(EventId)`: enum variants that carry data);
     `crates/domain/src/logbook.rs::LogError` (the `Stale` variant);
     `crates/web/src/routes/entries.rs::question_key` and
     `crates/web/src/routes/entries.rs::parse_choice` (the question and the answer are encoded in
     form fields, `asked=same-smell-<id>`); `crates/web/src/routes/entries.rs::submit` (compares what
     was asked with what is asked now, and asks again when they differ);
     `crates/web/src/routes/entries.rs::ask` (re-renders the form's follow-up page with the
     original fields re-posted as hidden inputs); #87.
   - Why: between "which event?" and the tap on "Yes", another phone may have closed or opened
     an event; applying the answer to the wrong event would corrupt the evidence. Carrying the
     question with the answer lets the server notice (a form of optimistic concurrency).
     Alternatives: remember the question in the session (lost with two tabs); lock the event
     while the user thinks (impossible over HTTP); ignore the mismatch (wrong data).
   - Read more: Book ch. 6.1 `https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html`.
4. **`ReportValues`: normalise once, trust everywhere.** (Exercise 12)
   - In our code: `crates/domain/src/records.rs::ReportValues::new` (takes `mut rooms: Vec<...>`
     and `mut symptoms`, sorts, rejects a room listed twice with `windows(2)`, de-duplicates,
     trims the note with `note.map(str::trim).filter(|n| !n.is_empty())`, limits it in
     characters); the private fields with getters `rooms`, `symptoms`, `note`;
     `crates/domain/src/records.rs::ReportValues::intensity` (`iter().map(..).max().unwrap_or(..)`,
     possible because `Level` is `Ord`), `crates/domain/src/records.rs::ReportValues::is_no_smell`,
     `crates/domain/src/records.rs::SmellReport` (`#[serde(flatten)]` puts the values' fields
     beside the report's own, #102, #105).
   - Why: sort and de-duplicate in one place, so every comparison, audit snapshot and query
     sees the same canonical shape (the audit hash depends on it: `canonical` in
     `crates/storage-sqlite/src/convert.rs::canonical` re-checks it when reading back).
     Alternatives: normalise in each caller (one will forget); store a `BTreeSet` (loses the
     per-room level pairing).
   - Read more: std `Option` `https://doc.rust-lang.org/std/option/enum.Option.html`, Book ch. 8.2
     `https://doc.rust-lang.org/book/ch08-02-strings.html` (why characters are counted, not
     bytes).
   - Try it: Exercise 12 `exercises/src/ex12_report_values.rs`.
5. **Errors that refuse: `LogError` and translation keys.**
   - In our code: `crates/domain/src/logbook.rs::LogError` (variants for what the user can fix,
     plus `Invalid(#[from] DomainError)` and `Store(#[from] StoreError)`);
     `crates/web/src/routes/mod.rs::refusal` (maps a `LogError` to `Result<&'static str,
     AppError>`: `Ok(key)` means "show the form again with this message", `Err` means "show an
     error page"); `crates/web/src/routes/entries.rs::read` (the form's own errors are
     translation keys, `Result<Entry, &'static str>`); `crates/web/src/routes/entries.rs::back_to_form`
     (status 422 and the typed values kept, #96).
   - Why: a `&'static str` key costs nothing, cannot be mistyped at runtime because tests list
     every key (`crates/web/src/routes/mod.rs::REFUSALS`, `FORM_ERRORS` in `entries.rs`) and the
     translation test checks them in all three languages. Alternatives: error enums with
     `Display` text (the domain would then own English text); `String` messages (nothing checks them).
   - Read more: Book ch. 9.2 `https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html`.
6. **Forms with repeated names, and borrowed data.** (Exercise 13)
   - In our code: `crates/web/src/forms.rs::Pairs` (`room=1&room=3` cannot be read into a struct
     with serde, so the form is kept as ordered `(String, String)` pairs, #98); its methods `get`
     (borrows from the pairs: returns `Option<&str>`), `ids` (`collect` into
     `Option<Vec<i64>>`), `names_with_prefix` (returns `impl Iterator<Item = &'a str> + 'a`,
     which borrows from both `self` and `prefix`), `without`;
     `crates/web/src/routes/log.rs::Draft` and `from_form` (what the form shows again after an
     error); `crates/web/src/routes/entries.rs::read_values` and
     `crates/web/src/routes/entries.rs::level_field` (the `level-<room id>` fields).
   - Why: the data stays where it is and callers get references into it, so nothing is
     copied; the compiler's lifetime checks prove the references cannot outlive the form.
     Say what the lifetime `'a` means in one sentence and use `names_with_prefix` as the example.
     Alternatives: return `String`s (an allocation per call); a `HashMap<String, Vec<String>>`
     (loses the order the form came in); axum's `Form<Struct>` (cannot express repeats).
   - Read more: Book ch. 10.3 `https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html`, Book
     ch. 4.2 `https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html`, std
     `Iterator` `https://doc.rust-lang.org/std/iter/trait.Iterator.html`.
   - Try it: Exercise 13 `exercises/src/ex13_form_pairs.rs`.
7. **View models, templates and a little CSS.**
   - In our code: `crates/web/src/routes/log.rs::fields` (builds `Fields`, `RoomChoice` and
     `Choice` values with `filter`/`map` chains and a closure `room_choices` that is reused for
     both areas); `crates/web/src/routes/log.rs::log_page` (reads the open event, the month's
     reports filtered with `can_see`, the summary, then builds `LogPage`);
     `crates/web/templates/log.html` and `crates/web/templates/report_fields.html` (the
     template only loops and prints; the decisions were made in Rust);
     `crates/web/static/app.css` (`.room:has(input[name="room"]:checked)`: the level row of a
     room appears only while the room is selected, with no JavaScript, #107).
   - Why: the page's structure and the page's logic stay apart: Rust decides what to show, the
     template shows it. Alternatives: logic in the template language (hard to test); a
     JavaScript component for the level rows (script and a CSP exception).
   - Read more: `https://docs.rs/askama`, MDN `:has()`
     `https://developer.mozilla.org/en-US/docs/Web/CSS/:has`.
8. **Small language features you will meet.**
   - In our code: `let … else` in `crates/web/src/assets.rs::respond` (`let Some(index) = ... else {
     return StatusCode::NOT_FOUND.into_response(); }`); let chains in
     `crates/domain/src/sweep.rs::auto_close` (`if let Some(last) = ... && !schedule.boundary_passed_since(...)`)
     and in `crates/web/src/routes/entries.rs::joinable_events`; `Option::is_none_or` in
     `crates/domain/src/events.rs::can_join_backdated`; `matches!` in
     `crates/domain/src/permissions.rs::can`; `Option::map_or` in
     `crates/domain/src/logbook.rs::close_at_last_report`.
   - Why: each removes a level of nesting or a temporary variable; say which Rust version
     made each stable (let chains need edition 2024 and Rust 1.88; `is_none_or` Rust 1.82;
     `let … else` Rust 1.65) and that the project's `rust-version` is 1.94 (root `Cargo.toml`).
   - Read more: Book ch. 6.3 `https://doc.rust-lang.org/book/ch06-03-if-let.html`, Reference
     `https://doc.rust-lang.org/reference/expressions/if-expr.html`, std `Option`
     `https://doc.rust-lang.org/std/option/enum.Option.html`.
9. **The catch-up sweep, and one transaction for many writes.**
   - In our code: `crates/domain/src/sweep.rs::auto_close` (runs only if a 04:00 boundary has
     passed since the stored last run, so it gives the same result at 04:00 or at 08:00 after the
     host slept; the close and the new `last_run_at` commit together, #34, #88);
     `crates/domain/src/logbook.rs::close_at_last_report` (`pub(crate)`: shared by the "new
     event" answer and the sweep); the callers `crates/web/src/routes/log.rs::show`,
     `crates/web/src/routes/entries.rs::submit` and
     `crates/app/src/commands/serve.rs::sweep_events` (startup and every five minutes); the
     contract tests `crates/storage-contract/src/logbook.rs::a_failed_report_rolls_back_the_new_event_close`
     and `crates/storage-contract/src/sweep.rs::on_dst_days_the_sweep_still_runs_at_four_local`.
   - Why: a scheduler that must fire at an exact moment fails when the host sleeps (free tiers
     stop idle containers); a "has a boundary passed since" check cannot miss. Alternatives: a cron
     job or `tokio-cron-scheduler` (needs the process awake at 04:00).
   - Read more: `https://docs.rs/tokio` (`tokio::time::interval`), Book ch. 16.1
     `https://doc.rust-lang.org/book/ch16-01-threads.html` (for `JoinHandle`, as background
     reading).

**Check yourself** (use or adapt; the note needs five or six): Why does `log_prompt` match on a
tuple, and what does the compiler do if a case is missing? What does `EventChoice::New {
replacing }` protect against? Why is `ReportValues`'s field list private? What does the `'a` in
`names_with_prefix<'a>` promise? Why does `refusal` return `Result<&'static str, AppError>` and
not just `&'static str`? What would break if the auto-close ran only at exactly 04:00?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/04-logging-flow.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/04-logging-flow.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/04-logging-flow.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-04.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-04.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/04-logging-flow.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/04-logging-flow.md
git commit -m "docs(learning): note 04, use cases as generics, pure rules and borrowed form data

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---

### Task 17: Exercise 11 — Which question does the Log screen ask?

**Files:**
- Create: `exercises/src/ex11_log_prompt.rs`, `exercises/tests/ex11_log_prompt.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex11_log_prompt;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/04-logging-flow.md` (Task 16).
- Produces: the public items `ObservedTime`, `TimeError`, `classify_observed`, `EventSummary`, `LogPrompt`, `log_prompt`, `no_smell_prompt`, `DailyBoundary`, `NewEventState`, `new_event_state`, `large_gap_indices`, used only by this exercise's own test file.

**What the learner practises:** enums that carry data, `match` on a tuple, `Result` and `saturating_sub`, simple arithmetic on `u64`, `windows(2)` on slices
**Difficulty:** two stars (about 40 minutes). **Tests:** 14. **`todo!()` bodies in the stub:** 7.

- [ ] **Step 1: Create the stub `exercises/src/ex11_log_prompt.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 11: which question does the Log screen ask? Enums, tuples and `match`.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 2.
//! Mirrors: `smoke_domain::events::{classify_observed, log_prompt, no_smell_prompt,
//! new_event_state, large_gap_indices}` and `smoke_domain::schedule::DailySchedule`.
//! Run: `cargo test --test ex11_log_prompt`
//!
//! Hint: match on a tuple of everything the decision depends on, one arm per case, and let the
//! compiler check that every combination is covered: `match (observed, open, unseen) { ... }`.
//! Times are seconds since the start of a day-zero UTC calendar; no time zones here.

pub const BACKDATE_THRESHOLD: u64 = 10 * 60;
pub const FUTURE_TOLERANCE: u64 = 60;
pub const LARGE_GAP: u64 = 3 * 60 * 60;
pub const DAY: u64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservedTime {
    Now,
    Backdated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeError {
    ObservedInFuture,
}

/// More than `FUTURE_TOLERANCE` ahead of `now` is an error (phone clocks drift a little);
/// more than `BACKDATE_THRESHOLD` behind is `Backdated`; anything between is `Now`. Be careful
/// not to subtract below zero from a `u64`: use `saturating_sub`.
pub fn classify_observed(observed_at: u64, now: u64) -> Result<ObservedTime, TimeError> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventSummary {
    pub id: u32,
    pub opened_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogPrompt {
    /// Nothing to ask.
    Direct,
    /// "Same smell still going?" about the open event.
    SameSmell(EventSummary),
    /// "Which event was this?" for a backdated report.
    WhichEvent,
    /// "This event was auto-closed: was it still going?"
    AutoClosed(EventSummary),
}

/// A backdated report always asks which event. Otherwise an open event is asked about first,
/// then an auto-closed one nobody has seen, else nothing.
pub fn log_prompt(
    observed: ObservedTime,
    open_event: Option<EventSummary>,
    unseen_auto_closed: Option<EventSummary>,
) -> LogPrompt {
    todo!()
}

/// A no-smell report logged now asks nothing (it joins the open event or stands alone); a
/// backdated one asks which event.
pub fn no_smell_prompt(observed: ObservedTime) -> LogPrompt {
    todo!()
}

/// A daily boundary such as 04:00, as seconds after midnight UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyBoundary {
    pub offset: u64,
}

impl DailyBoundary {
    /// The most recent boundary at or before `now`: today's if it has passed, else yesterday's.
    pub fn last_before(&self, now: u64) -> u64 {
        todo!()
    }

    /// Whether a boundary lies in `(since, now]`.
    pub fn passed_since(&self, since: u64, now: u64) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewEventState {
    Open,
    ClosedBackdated,
}

/// At most one event is open at a time and nothing stays open across a boundary: a new event
/// is born closed when another one is open or a boundary has passed since `observed_at`.
pub fn new_event_state(
    observed_at: u64,
    now: u64,
    boundary: &DailyBoundary,
    another_event_open: bool,
) -> NewEventState {
    todo!()
}

/// The indices of reports that follow a gap longer than `LARGE_GAP`, in a list sorted by time.
/// `[0, 100, 100 + LARGE_GAP + 1]` gives `[2]`. Use `windows(2)`.
pub fn large_gap_indices(observed_times: &[u64]) -> Vec<usize> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex11_log_prompt.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex11_log_prompt::{
    BACKDATE_THRESHOLD, DAY, DailyBoundary, EventSummary, FUTURE_TOLERANCE, LARGE_GAP, LogPrompt,
    NewEventState, ObservedTime, TimeError, classify_observed, large_gap_indices, log_prompt,
    new_event_state, no_smell_prompt,
};

const NOW: u64 = 100 * DAY + 20 * 3600;
const FOUR: DailyBoundary = DailyBoundary { offset: 4 * 3600 };

fn event(id: u32) -> EventSummary {
    EventSummary {
        id,
        opened_at: NOW - 3600,
    }
}

#[test]
fn a_time_close_to_now_is_now() {
    assert_eq!(classify_observed(NOW, NOW), Ok(ObservedTime::Now));
    assert_eq!(
        classify_observed(NOW - BACKDATE_THRESHOLD, NOW),
        Ok(ObservedTime::Now)
    );
    assert_eq!(
        classify_observed(NOW + FUTURE_TOLERANCE, NOW),
        Ok(ObservedTime::Now)
    );
}

#[test]
fn a_time_further_back_is_backdated() {
    assert_eq!(
        classify_observed(NOW - BACKDATE_THRESHOLD - 1, NOW),
        Ok(ObservedTime::Backdated)
    );
    assert_eq!(classify_observed(0, NOW), Ok(ObservedTime::Backdated));
}

#[test]
fn a_time_further_ahead_than_the_tolerance_is_refused() {
    assert_eq!(
        classify_observed(NOW + FUTURE_TOLERANCE + 1, NOW),
        Err(TimeError::ObservedInFuture)
    );
}

#[test]
fn near_the_start_of_time_nothing_overflows() {
    assert_eq!(classify_observed(0, 5), Ok(ObservedTime::Now));
}

#[test]
fn a_backdated_report_always_asks_which_event() {
    let open = Some(event(1));
    let closed = Some(event(2));
    assert_eq!(
        log_prompt(ObservedTime::Backdated, open, closed),
        LogPrompt::WhichEvent
    );
    assert_eq!(
        log_prompt(ObservedTime::Backdated, None, None),
        LogPrompt::WhichEvent
    );
}

#[test]
fn an_open_event_is_asked_about_before_an_auto_closed_one() {
    assert_eq!(
        log_prompt(ObservedTime::Now, Some(event(1)), Some(event(2))),
        LogPrompt::SameSmell(event(1))
    );
    assert_eq!(
        log_prompt(ObservedTime::Now, None, Some(event(2))),
        LogPrompt::AutoClosed(event(2))
    );
    assert_eq!(log_prompt(ObservedTime::Now, None, None), LogPrompt::Direct);
}

#[test]
fn a_no_smell_report_asks_only_when_backdated() {
    assert_eq!(no_smell_prompt(ObservedTime::Now), LogPrompt::Direct);
    assert_eq!(
        no_smell_prompt(ObservedTime::Backdated),
        LogPrompt::WhichEvent
    );
}

#[test]
fn the_last_boundary_is_todays_once_it_has_passed() {
    assert_eq!(
        FOUR.last_before(100 * DAY + 10 * 3600),
        100 * DAY + 4 * 3600
    );
    assert_eq!(FOUR.last_before(100 * DAY + 4 * 3600), 100 * DAY + 4 * 3600);
}

#[test]
fn before_todays_boundary_it_is_yesterdays() {
    assert_eq!(FOUR.last_before(100 * DAY + 3 * 3600), 99 * DAY + 4 * 3600);
    assert_eq!(FOUR.last_before(100 * DAY), 99 * DAY + 4 * 3600);
}

#[test]
fn a_boundary_passed_since_means_one_lies_after_that_instant_up_to_now() {
    let before = 100 * DAY + 3 * 3600;
    let after = 100 * DAY + 5 * 3600;
    assert!(FOUR.passed_since(before, after));
    assert!(!FOUR.passed_since(100 * DAY + 4 * 3600, after));
    assert!(!FOUR.passed_since(after, after + 3600));
}

#[test]
fn a_new_event_is_open_when_nothing_else_is() {
    assert_eq!(
        new_event_state(NOW - 60, NOW, &FOUR, false),
        NewEventState::Open
    );
}

#[test]
fn a_new_event_is_born_closed_when_another_is_open() {
    assert_eq!(
        new_event_state(NOW - 60, NOW, &FOUR, true),
        NewEventState::ClosedBackdated
    );
}

#[test]
fn a_new_event_is_born_closed_when_a_boundary_has_passed() {
    let yesterday_evening = 99 * DAY + 22 * 3600;
    assert_eq!(
        new_event_state(yesterday_evening, 100 * DAY + 8 * 3600, &FOUR, false),
        NewEventState::ClosedBackdated
    );
}

#[test]
fn only_gaps_longer_than_three_hours_are_flagged() {
    assert_eq!(large_gap_indices(&[0, 100, 100 + LARGE_GAP + 1]), [2]);
    assert!(large_gap_indices(&[0, LARGE_GAP]).is_empty());
    assert_eq!(
        large_gap_indices(&[0, LARGE_GAP + 1, LARGE_GAP + 2, 3 * LARGE_GAP]),
        [1, 3]
    );
    assert!(large_gap_indices(&[]).is_empty());
    assert!(large_gap_indices(&[5]).is_empty());
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex11_log_prompt;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex11_log_prompt --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex11_log_prompt 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 14 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex11_log_prompt.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex11_log_prompt 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 14 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex11_log_prompt` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/04-logging-flow.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex11_log_prompt.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex11_log_prompt.rs        # 7
grep -n '@' src/ex11_log_prompt.rs tests/ex11_log_prompt.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex11_log_prompt.rs exercises/tests/ex11_log_prompt.rs
git commit -m "docs(exercises): exercise 11, log prompt rules (Rust: match on tuples, saturating arithmetic, slices)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 18: Exercise 12 — A report that normalises its input

**Files:**
- Create: `exercises/src/ex12_report_values.rs`, `exercises/tests/ex12_report_values.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex12_report_values;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/04-logging-flow.md` (Task 16).
- Produces: the public items `RoomLevel`, `ReportError`, `ReportValues`, used only by this exercise's own test file.

**What the learner practises:** taking ownership of `Vec`s and sorting them, `dedup`, `windows(2)`, `Option::map`/`filter`, `str::trim`, `chars().count()`, getters that return borrowed slices
**Difficulty:** two stars (about 40 minutes). **Tests:** 10. **`todo!()` bodies in the stub:** 7.

- [ ] **Step 1: Create the stub `exercises/src/ex12_report_values.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 12: a report that normalises its input. Vectors, sorting and `Option`.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 4.
//! Mirrors: `smoke_domain::records::ReportValues`.
//! Run: `cargo test --test ex12_report_values`
//!
//! Hint: `new` takes ownership of the vectors (`mut rooms`), sorts and de-duplicates them, and
//! stores them in a struct whose fields are private, so every `ReportValues` that exists is
//! already in canonical form. `sort_unstable_by_key`, `dedup`, `windows(2)`, `Iterator::max`,
//! `Option::map`, `Option::filter` and `str::trim` are the tools. A `ReportValues` is the only
//! way to build the data, so the getters can simply return borrowed slices.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomLevel {
    pub room: u32,
    pub level: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportError {
    /// A room appears twice; the number is its id.
    RoomListedTwice(u32),
    /// A level above 10; the number is the level.
    InvalidLevel(u8),
    /// A note of more than `NOTE_MAX_CHARS` characters; the number is its length.
    NoteTooLong(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportValues {
    rooms: Vec<RoomLevel>,
    symptoms: Vec<u32>,
    note: Option<String>,
}

impl ReportValues {
    pub const NOTE_MAX_CHARS: usize = 2000;

    /// Rooms sorted by room id (and no room twice), symptoms sorted and unique, the note
    /// trimmed with a blank note meaning no note. Checks, in this order: levels, duplicate
    /// rooms, the note's length in characters (after trimming).
    pub fn new(
        mut rooms: Vec<RoomLevel>,
        mut symptoms: Vec<u32>,
        note: Option<&str>,
    ) -> Result<Self, ReportError> {
        todo!()
    }

    pub fn rooms(&self) -> &[RoomLevel] {
        todo!()
    }

    pub fn symptoms(&self) -> &[u32] {
        todo!()
    }

    pub fn note(&self) -> Option<&str> {
        todo!()
    }

    /// The level reported for `room`; `None` when the room was not checked.
    pub fn level_in(&self, room: u32) -> Option<u8> {
        todo!()
    }

    /// The report's intensity: its highest room level, 0 without rooms.
    pub fn intensity(&self) -> u8 {
        todo!()
    }

    /// A no-smell report: every room listed is at 0, or none is listed (the whole home).
    pub fn is_no_smell(&self) -> bool {
        todo!()
    }
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex12_report_values.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex12_report_values::{ReportError, ReportValues, RoomLevel};

fn room(room: u32, level: u8) -> RoomLevel {
    RoomLevel { room, level }
}

#[test]
fn rooms_and_symptoms_come_back_sorted_and_unique() {
    let values = ReportValues::new(
        vec![room(3, 2), room(1, 6), room(2, 0)],
        vec![9, 4, 9, 1, 4],
        None,
    )
    .unwrap();
    assert_eq!(values.rooms(), [room(1, 6), room(2, 0), room(3, 2)]);
    assert_eq!(values.symptoms(), [1, 4, 9]);
}

#[test]
fn a_room_listed_twice_is_refused_even_with_two_levels() {
    assert_eq!(
        ReportValues::new(vec![room(5, 1), room(2, 3), room(5, 7)], vec![], None),
        Err(ReportError::RoomListedTwice(5))
    );
}

#[test]
fn a_level_above_ten_is_refused() {
    assert_eq!(
        ReportValues::new(vec![room(1, 11)], vec![], None),
        Err(ReportError::InvalidLevel(11))
    );
    assert!(ReportValues::new(vec![room(1, 10)], vec![], None).is_ok());
}

#[test]
fn levels_are_checked_before_duplicates() {
    assert_eq!(
        ReportValues::new(vec![room(1, 3), room(1, 4), room(2, 99)], vec![], None),
        Err(ReportError::InvalidLevel(99))
    );
}

#[test]
fn a_note_is_trimmed_and_a_blank_note_is_no_note() {
    let noted = ReportValues::new(vec![], vec![], Some("  smelled in the hall \n")).unwrap();
    assert_eq!(noted.note(), Some("smelled in the hall"));
    for blank in ["", "   ", "\n\t"] {
        let values = ReportValues::new(vec![], vec![], Some(blank)).unwrap();
        assert_eq!(values.note(), None);
    }
    assert_eq!(
        ReportValues::new(vec![], vec![], None).unwrap().note(),
        None
    );
}

#[test]
fn a_note_is_limited_to_2000_characters_not_bytes() {
    let long_ok = "é".repeat(2000);
    assert_eq!(
        ReportValues::new(vec![], vec![], Some(&long_ok))
            .unwrap()
            .note(),
        Some(long_ok.as_str())
    );
    let too_long = "é".repeat(2001);
    assert_eq!(
        ReportValues::new(vec![], vec![], Some(&too_long)),
        Err(ReportError::NoteTooLong(2001))
    );
    let padded = format!("  {}  ", "x".repeat(2000));
    assert!(ReportValues::new(vec![], vec![], Some(&padded)).is_ok());
}

#[test]
fn the_level_of_a_room_is_none_when_it_was_not_checked() {
    let values = ReportValues::new(vec![room(1, 6), room(2, 0)], vec![], None).unwrap();
    assert_eq!(values.level_in(1), Some(6));
    assert_eq!(values.level_in(2), Some(0));
    assert_eq!(values.level_in(3), None);
}

#[test]
fn the_intensity_is_the_highest_room_level() {
    let values = ReportValues::new(vec![room(1, 2), room(2, 8), room(3, 5)], vec![], None).unwrap();
    assert_eq!(values.intensity(), 8);
}

#[test]
fn no_rooms_means_intensity_zero_and_no_smell() {
    let values = ReportValues::new(vec![], vec![], None).unwrap();
    assert_eq!(values.intensity(), 0);
    assert!(values.is_no_smell());
}

#[test]
fn rooms_all_at_zero_are_no_smell_but_one_smelly_room_is_not() {
    let clean = ReportValues::new(vec![room(1, 0), room(2, 0)], vec![], None).unwrap();
    assert!(clean.is_no_smell());
    let smelly = ReportValues::new(vec![room(1, 0), room(2, 1)], vec![], None).unwrap();
    assert!(!smelly.is_no_smell());
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex12_report_values;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex12_report_values --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex12_report_values 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 10 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex12_report_values.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex12_report_values 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 10 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex12_report_values` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/04-logging-flow.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex12_report_values.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex12_report_values.rs        # 7
grep -n '@' src/ex12_report_values.rs tests/ex12_report_values.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex12_report_values.rs exercises/tests/ex12_report_values.rs
git commit -m "docs(exercises): exercise 12, report values (Rust: Vec, sorting, Option, str)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 19: Exercise 13 — Reading a form whose field names repeat

**Files:**
- Create: `exercises/src/ex13_form_pairs.rs`, `exercises/tests/ex13_form_pairs.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex13_form_pairs;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/04-logging-flow.md` (Task 16).
- Produces: the public items `Pairs`, used only by this exercise's own test file.

**What the learner practises:** methods that return `&str` borrowed from `&self`, collecting into `Option<Vec<_>>`, an iterator that borrows two things (`impl Iterator<Item = &'a str> + 'a`). The stub's `names_with_prefix` returns an empty iterator so that the file compiles quietly: the learner replaces its body
**Difficulty:** three stars (about 45 minutes). **Tests:** 8. **`todo!()` bodies in the stub:** 4.

- [ ] **Step 1: Create the stub `exercises/src/ex13_form_pairs.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 13: reading a form whose field names repeat. Borrowing and lifetimes.
//!
//! Practises: `docs/learning/04-logging-flow.md`, section 6.
//! Mirrors: `smoke_web::forms::Pairs`.
//! Run: `cargo test --test ex13_form_pairs`
//!
//! Hint: `Pairs` owns its `String`s; every method that returns text only *borrows* it from the
//! `Pairs`, so no copying is needed. `names_with_prefix` returns an iterator that borrows both
//! `self` and `prefix`, which is why both carry the same lifetime `'a`. `Iterator::filter_map`,
//! `str::strip_prefix` and collecting into `Option<Vec<_>>` (stops at the first `None`) will
//! shorten the code.

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pairs(Vec<(String, String)>);

impl Pairs {
    /// Builds the pairs from `(name, value)` slices, keeping their order.
    pub fn of(pairs: &[(&str, &str)]) -> Self {
        todo!()
    }

    /// The first value of `name`, trimmed; `None` if the name is absent or its value blank.
    pub fn get(&self, name: &str) -> Option<&str> {
        todo!()
    }

    /// Every value of `name` parsed as an id, in order; `None` if one of them is not a number.
    /// A name that never appears gives `Some(vec![])`.
    pub fn ids(&self, name: &str) -> Option<Vec<i64>> {
        todo!()
    }

    /// What follows `prefix` in each field name that starts with it: the room ids of the
    /// `level-<id>` fields, for example.
    pub fn names_with_prefix<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        // The stub returns an empty iterator so that the test file compiles.
        std::iter::empty()
    }

    /// All pairs except those whose name is in `skip`: what a follow-up form re-posts as
    /// hidden fields.
    pub fn without(&self, skip: &[&str]) -> Vec<(String, String)> {
        todo!()
    }
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex13_form_pairs.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex13_form_pairs::Pairs;

fn form() -> Pairs {
    Pairs::of(&[
        ("note", " smelled in the hall "),
        ("room", "3"),
        ("room", " 1 "),
        ("level-3", "6"),
        ("level-1", "2"),
        ("pm25", "   "),
        ("symptom", "7"),
    ])
}

#[test]
fn get_returns_the_first_value_trimmed() {
    let doubled = Pairs::of(&[("note", " first "), ("note", "second")]);
    assert_eq!(doubled.get("note"), Some("first"));
    assert_eq!(form().get("note"), Some("smelled in the hall"));
}

#[test]
fn get_treats_a_blank_value_like_a_missing_one() {
    assert_eq!(form().get("pm25"), None);
    assert_eq!(form().get("missing"), None);
}

#[test]
fn ids_collects_every_value_in_order() {
    assert_eq!(form().ids("room"), Some(vec![3, 1]));
    assert_eq!(form().ids("symptom"), Some(vec![7]));
}

#[test]
fn ids_of_an_absent_name_is_an_empty_list() {
    assert_eq!(form().ids("missing"), Some(vec![]));
}

#[test]
fn ids_is_none_when_any_value_is_not_a_number() {
    let form = Pairs::of(&[("room", "3"), ("room", "three"), ("room", "5")]);
    assert_eq!(form.ids("room"), None);
    assert_eq!(Pairs::of(&[("room", "")]).ids("room"), None);
}

#[test]
fn names_with_prefix_lists_what_follows_the_prefix() {
    let form = form();
    let ids: Vec<&str> = form.names_with_prefix("level-").collect();
    assert_eq!(ids, ["3", "1"]);
    assert_eq!(form.names_with_prefix("nothing-").count(), 0);
}

#[test]
fn the_names_borrow_from_the_form_not_from_a_copy() {
    let form = form();
    let first = form.names_with_prefix("level-").next().unwrap();
    let again = form.names_with_prefix("level-").next().unwrap();
    assert_eq!(first.as_ptr(), again.as_ptr());
}

#[test]
fn without_drops_the_named_fields_and_keeps_order_and_values() {
    let rest = form().without(&["room", "level-3", "level-1"]);
    assert_eq!(
        rest,
        [
            ("note".to_owned(), " smelled in the hall ".to_owned()),
            ("pm25".to_owned(), "   ".to_owned()),
            ("symptom".to_owned(), "7".to_owned()),
        ]
    );
    assert_eq!(form().without(&[]).len(), 7);
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex13_form_pairs;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex13_form_pairs --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex13_form_pairs 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 8 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex13_form_pairs.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex13_form_pairs 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 8 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex13_form_pairs` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/04-logging-flow.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex13_form_pairs.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex13_form_pairs.rs        # 4
grep -n '@' src/ex13_form_pairs.rs tests/ex13_form_pairs.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex13_form_pairs.rs exercises/tests/ex13_form_pairs.rs
git commit -m "docs(exercises): exercise 13, form pairs (Rust: borrowing, lifetimes, impl Iterator)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---


### Task 20: Note 05 — people and the PWA

**Files:**
- Create: `docs/learning/05-people-and-pwa.md`

**Interfaces:**
- Consumes: decisions #15, #16, #32, #112 to #122 in `docs/DECISIONS.md`; the code listed
  below.
- Produces: the note that Exercises 14 and 15 link back to (section 2 is Exercise 14; sections 4,
  6 and 7 are Exercise 15: keep this numbering).

**Covers:** account use cases and their errors; enums that carry data (expiry picks); secrets
kept in types (token hashes, one-time links); a QR code drawn as inline SVG; moving values into
a blocking thread; HTTP caching with embedded files and ETags; the PWA (manifest, service
worker, install prompt); a link token that behaves like a password.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 05 — People, invites and the PWA

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 14 (expiry picks), 15 (assets, caching and an SVG drawing). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **Account use cases and their errors.**
   - In our code: `crates/domain/src/people.rs::invite`, `crates/domain/src/people.rs::issue_link`,
     `crates/domain/src/people.rs::usable_invite`, `crates/domain/src/people.rs::accept`,
     `crates/domain/src/people.rs::expire`, `crates/domain/src/people.rs::reactivate`,
     `crates/domain/src/people.rs::change_password` (generic over the write-transaction traits
     like the logbook, with permission checks against data read inside the transaction, #113);
     `crates/domain/src/people.rs::PeopleError` and its **hand-written** `From<StoreError>` (a
     unique-username conflict becomes `UsernameTaken`, a used invite becomes `LinkInvalid`, the
     rest passes through as `Store`); `crates/web/src/routes/mod.rs::people_refusal`;
     `crates/web/src/routes/people/mod.rs::require_manager` (residents only; others see the
     403 page and no menu link, #118).
   - Why: a `#[from]` would convert every `StoreError` the same way; here two of them have a
     meaning the user must see, so the conversion is written out. Alternatives: `#[from]` plus
     matching on `PeopleError::Store(StoreError::Conflict(..))` in the web layer (the web would then
     know storage details); checking permissions in the web layer only (a second front door, such
     as the CLI, would skip them).
   - Read more: Book ch. 9.2 `https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html`,
     std `From` `https://doc.rust-lang.org/std/convert/trait.From.html`.
2. **Expiry picks: enums with data.** (Exercise 14)
   - In our code: `crates/domain/src/people.rs::ExpiryPick` (unit variants `OneDay`, `Weekend`,
     `OneWeek`, `TwoWeeks`, `Never` and one that carries data, `Until(NaiveDate)`) and its
     `resolve(self, now, schedule) -> Option<DateTime<Utc>>` (a `match` with one arm per variant;
     `Weekend` uses `weekday().num_days_from_monday()` and the schedule's `day_span`, so "next
     Monday 00:00" is local time and a Monday means the next Monday, #115);
     `crates/domain/src/permissions.rs::check_expiry` (a `match` on `(role, expires_at)` with a
     guard arm); the form side in `crates/web/src/routes/people/expiry.rs`.
   - Why: "one of these six things" is exactly what an enum is; `Never` returning `None` makes
     "no expiry" part of the type instead of a magic date. Alternatives: a `u32` number of days
     (cannot say "weekend" or "a chosen date"); an `Option<DateTime>` picked in the web layer
     (the rule would sit in a handler and could not be tested without a request).
   - Read more: Book ch. 6.1 `https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html`, Book
     ch. 6.2 `https://doc.rust-lang.org/book/ch06-02-match.html`, `https://docs.rs/chrono`.
   - Try it: Exercise 14 `exercises/src/ex14_expiry_pick.rs`.
3. **Secrets in types: tokens, hashes and one-time links.**
   - In our code: `crates/web/src/routes/people/links.rs::new_token` (32 random bytes from
     `getrandom::fill` into a `[0u8; 32]` array, then `hex::encode`, #119);
     `crates/domain/src/types.rs::TokenHash` (`TokenHash::of` hashes with SHA-256; its `Debug`
     prints only a prefix); `crates/domain/src/records.rs::Invite` (the record has no token and no hash
     field at all; `NewInvite` hands the store a `TokenHash`; `LIFETIME` is five minutes,
     `is_usable` checks use and age, #16, #112);
     `crates/domain/src/repos.rs::InviteRepo` (`invite_by_token` takes a `&TokenHash`, so the
     raw token cannot even be passed to the store by mistake).
   - Why: whoever holds the link can set a password, so the link is a password: only its hash is
     kept, it lives five minutes, and it works once. The type system carries the rule (a
     `TokenHash` is not a `String`). Alternatives: storing the token (a database leak would hand
     out live links); a signed token with no storage (cannot be made single-use without storage).
   - Read more: `https://docs.rs/getrandom`, `https://docs.rs/sha2`, `https://docs.rs/hex`,
     Book ch. 5.1 `https://doc.rust-lang.org/book/ch05-01-defining-structs.html`.
4. **A QR code drawn as inline SVG.** (Exercise 15)
   - In our code: `crates/web/src/qr.rs::svg` (`QrCode::encode_text` from `qrcodegen`, a loop
     over every module, `write!` into a `String` with `use std::fmt::Write`, and an `expect` on
     a call that cannot fail for a `String`); `crates/web/src/qr.rs::QUIET_ZONE` (a named
     `const`); the page that shows it, `crates/web/src/routes/people/links.rs::create`;
     `crates/web/templates/link.html` (the SVG is inserted without escaping, which is safe
     only because the markup is generated by this function from numbers).
   - Why: the Content-Security-Policy forbids `data:` images (`crates/web/src/security.rs::CSP`),
     and inline SVG needs no file, no image library and no CSP exception; the token exists only in
     the page that shows it (#119). Alternatives: the `qrcode` crate (pulls in the `image` crate
     by default); a PNG served from a URL (the token would have to be stored to serve it).
   - Read more: `https://docs.rs/qrcodegen`, std `fmt::Write`
     `https://doc.rust-lang.org/std/fmt/trait.Write.html`, std `write!`
     `https://doc.rust-lang.org/std/macro.write.html`, MDN CSP
     `https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP`.
   - Try it: Exercise 15 `exercises/src/ex15_pwa_assets.rs` (`svg_path` and `svg`).
5. **Setting a password: blocking work and values moved into a thread.**
   - In our code: `crates/web/src/routes/accept.rs::submit` (the hash is computed with
     `password::run_limited(state.hash_permits.clone(), move || ...)`: a `move` closure takes
     ownership of what it uses, because it runs on another thread that may outlive the
     request); `crates/web/src/routes/accept.rs::usable` (the link is checked twice: before the
     slow work and again inside the transaction that uses it up);
     `crates/web/src/auth/mod.rs::start_session` (the device is signed in again with a new
     session id); `crates/web/src/routes/settings.rs::change` (the same pattern for a password
     change, which first checks the current password as a sign-in attempt, #116).
   - Why: Note 03 explained why Argon2 runs on a blocking thread; the new idea here is
     ownership: a closure sent to another thread must be `'static + Send`, so it takes its data by
     `move` (clone first what the handler still needs). Alternatives: borrowing data into the
     thread (the compiler refuses: the request may end first); scoped threads (not usable across
     an `.await`).
   - Read more: Book ch. 16.1 `https://doc.rust-lang.org/book/ch16-01-threads.html`, Book ch. 13.1
     `https://doc.rust-lang.org/book/ch13-01-closures.html`.
6. **HTTP caching: embedded files and ETags.** (Exercise 15)
   - In our code: `crates/web/src/assets.rs::asset` (a `macro_rules!` that builds an `Asset` with
     `include_bytes!(concat!("../static/", $path))`, so the file is compiled into the binary);
     `crates/web/src/assets.rs::ASSETS` (a `const` slice of `Asset`);
     `crates/web/src/assets.rs::ETAGS` (a `LazyLock<Vec<String>>`: the first 16 hex characters of each
     file's SHA-256, computed once on first use); `crates/web/src/assets.rs::respond` (`position`
     finds the asset, `If-None-Match` equal to the tag gives `304 Not Modified` with no body,
     `Cache-Control: no-cache` means "revalidate", #65); `crates/web/src/assets.rs::service_worker`
     (the same file served at `/sw.js` so its scope is the whole site); the tests
     `crates/web/src/assets.rs::every_file_in_the_static_folder_is_served` and
     `crates/web/src/assets.rs::the_vendored_libraries_are_the_published_releases`.
   - Why: one file to deploy and no path to get wrong at runtime; revalidation makes a deploy
     visible at once while a repeat visit costs only a `304`. Alternatives: `tower-http`'s
     `ServeDir` (reads files at runtime, so they must be in the image); the `rust-embed`
     crate (the same idea with less hand-written code, a dependency).
   - Read more: std `include_bytes!` `https://doc.rust-lang.org/std/macro.include_bytes.html`,
     std `LazyLock` `https://doc.rust-lang.org/std/sync/struct.LazyLock.html`, MDN ETag
     `https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag`.
   - Try it: Exercise 15 `exercises/src/ex15_pwa_assets.rs` (`etag_of` and `respond`).
7. **The PWA: manifest, service worker and the install prompt.** (Exercise 15)
   - In our code: `crates/web/static/manifest.webmanifest` (name, scope, icons, the "New smell
     report" shortcut); `crates/web/static/sw.js` (the `install`, `activate` and `fetch` handlers:
     precache a few files, navigations go to the network with `/offline` as the fallback, static
     files are fetched first and the cached copy is the fallback, form posts are never touched,
     #121); `crates/web/static/app.js` (`beforeinstallprompt` is captured so any element with
     `data-install` can open the browser's install dialog); `crates/web/src/routes/install.rs::show`
     and `crates/web/src/routes/install.rs::offline` (the pages); the test
     `crates/web/tests/pwa.rs::everything_the_service_worker_precaches_exists`. This section is
     about the web platform: the service worker must be JavaScript, and Rust's job is serving it
     correctly (section 6).
   - Why: network-first means a new deploy is never hidden behind an old cache; the price is that
     the app does not work offline beyond the offline page (offline logging is out of scope,
     spec §14). Alternatives: cache-first (fast, but stale after each deploy unless versioned);
     the Workbox library (more machinery than three event handlers).
   - Read more: MDN service workers
     `https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API`, MDN PWAs
     `https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps`, MDN manifest
     `https://developer.mozilla.org/en-US/docs/Web/Manifest`, MDN install prompt
     `https://developer.mozilla.org/en-US/docs/Web/API/BeforeInstallPromptEvent`.
   - Try it: Exercise 15 `exercises/src/ex15_pwa_assets.rs` (`network_first`).
8. **A link token that behaves like a password.**
   - In our code: `crates/web/src/security.rs::headers` (`Referrer-Policy: no-referrer` for paths
     under `/invite/`, `same-origin` elsewhere); `crates/web/src/lib.rs::logged_path` and
     `crates/web/src/lib.rs::request_span` (the token never reaches the app's own logs);
     decisions #119 and #122, which say honestly what remains: the platform's request log could
     still contain the token, accepted because a link works once and for five minutes.
   - Why: listing the leaks you cannot close is part of the design; mitigations are chosen by
     how long the secret lives. Alternatives: a URL fragment read by script (a script the CSP
     discourages, for little gain); a proxy that strips paths from the platform log.
   - Read more: MDN Referrer-Policy
     `https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy`, OWASP session
     management `https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html`.

**Check yourself** (use or adapt; the note needs five or six): Why is `Until` a variant with
data while `Weekend` has none? What does the compiler force you to do when a seventh pick is
added? Why does `invite_by_token` take a `TokenHash`? What does `move` do in the closure passed
to `run_limited`, and what would the compiler say without it? Why does the service worker not
cache pages? What does `LazyLock` buy in `ETAGS` that a plain `static` could not?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/05-people-and-pwa.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/05-people-and-pwa.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/05-people-and-pwa.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-05.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-05.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/05-people-and-pwa.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/05-people-and-pwa.md
git commit -m "docs(learning): note 05, enums with data, secrets in types and the service worker's cache strategy

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---

### Task 21: Exercise 14 — When does an account expire?

**Files:**
- Create: `exercises/src/ex14_expiry_pick.rs`, `exercises/tests/ex14_expiry_pick.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex14_expiry_pick;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/05-people-and-pwa.md` (Task 20); it imports the given `Role` enum from Exercise 2 (`exercises/src/ex02_permissions.rs`, Task 5), which is not a stub.
- Produces: the public items `DAY`, `weekday_from_monday`, `ExpiryPick`, `ExpiryError`, `check_expiry`, used only by this exercise's own test file.

**What the learner practises:** an enum with unit variants and one that carries a number, `match` with a guard on a tuple, integer arithmetic and `%`, reusing a type from another module
**Difficulty:** two stars (about 35 minutes). **Tests:** 11. **`todo!()` bodies in the stub:** 3.

- [ ] **Step 1: Create the stub `exercises/src/ex14_expiry_pick.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 14: when does an account expire? Enums that carry data, and `match` guards.
//!
//! Practises: `docs/learning/05-people-and-pwa.md`, section 2.
//! Mirrors: `smoke_domain::people::ExpiryPick::resolve` and
//! `smoke_domain::permissions::check_expiry`.
//! Run: `cargo test --test ex14_expiry_pick`
//!
//! Hint: time here is seconds since 1970-01-01 00:00 UTC, and a day is `DAY` seconds; no time
//! zones or daylight saving (the app uses `chrono` and `DailySchedule` for those). Day number
//! `d` is the `d`-th day since the epoch, and 1970-01-01 was a Thursday. For `check_expiry`,
//! `match (role, expires_at)` with a guard (`(_, Some(at)) if at <= now => ...`) reads like the
//! rules.

use crate::ex02_permissions::Role;

pub const DAY: u64 = 24 * 60 * 60;

/// 0 for Monday up to 6 for Sunday, for the day with this number (day 0 was a Thursday).
pub fn weekday_from_monday(day: u64) -> u64 {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryPick {
    OneDay,
    /// Until the start of the coming Monday; on a Monday, the next one.
    Weekend,
    OneWeek,
    TwoWeeks,
    /// Until the end of the day with this number (so that whole day is included).
    Until(u64),
    Never,
}

impl ExpiryPick {
    /// The second the account expires, `None` for never. `OneDay`, `OneWeek` and `TwoWeeks`
    /// count from `now`.
    pub fn resolve(self, now: u64) -> Option<u64> {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryError {
    ResidentsNeverExpire,
    GuestsMustExpire,
    ExpiryNotInFuture,
}

/// Residents never expire; guests always do; a viewer may or may not. Any expiry must lie in
/// the future (strictly after `now`). Checked in this order: resident, guest, future.
pub fn check_expiry(role: Role, expires_at: Option<u64>, now: u64) -> Result<(), ExpiryError> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex14_expiry_pick.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex02_permissions::Role;
use smoke_exercises::ex14_expiry_pick::{
    DAY, ExpiryError, ExpiryPick, check_expiry, weekday_from_monday,
};

// 2026-10-14, a Wednesday, is day number 20740.
const WEDNESDAY: u64 = 20_740;
const SUNDAY: u64 = 20_744;
const MONDAY: u64 = 20_745;
const NEXT_MONDAY: u64 = 20_752;

#[test]
fn days_know_their_weekday() {
    assert_eq!(weekday_from_monday(0), 3, "1970-01-01 was a Thursday");
    assert_eq!(weekday_from_monday(WEDNESDAY), 2);
    assert_eq!(weekday_from_monday(SUNDAY), 6);
    assert_eq!(weekday_from_monday(MONDAY), 0);
}

#[test]
fn quick_picks_count_from_now() {
    let now = WEDNESDAY * DAY + 18 * 3600;
    assert_eq!(ExpiryPick::OneDay.resolve(now), Some(now + DAY));
    assert_eq!(ExpiryPick::OneWeek.resolve(now), Some(now + 7 * DAY));
    assert_eq!(ExpiryPick::TwoWeeks.resolve(now), Some(now + 14 * DAY));
}

#[test]
fn never_has_no_end() {
    assert_eq!(ExpiryPick::Never.resolve(123), None);
}

#[test]
fn the_weekend_lasts_until_the_coming_monday_morning() {
    let wednesday_evening = WEDNESDAY * DAY + 20 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(wednesday_evening),
        Some(MONDAY * DAY)
    );
    let sunday_night = SUNDAY * DAY + 23 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(sunday_night),
        Some(MONDAY * DAY)
    );
}

#[test]
fn on_a_monday_the_weekend_means_the_next_one() {
    let monday_morning = MONDAY * DAY + 8 * 3600;
    assert_eq!(
        ExpiryPick::Weekend.resolve(monday_morning),
        Some(NEXT_MONDAY * DAY)
    );
    assert_eq!(
        ExpiryPick::Weekend.resolve(MONDAY * DAY),
        Some(NEXT_MONDAY * DAY)
    );
}

#[test]
fn a_chosen_day_lasts_until_the_end_of_that_day() {
    assert_eq!(
        ExpiryPick::Until(WEDNESDAY).resolve(WEDNESDAY * DAY + 5),
        Some((WEDNESDAY + 1) * DAY)
    );
    assert_eq!(
        ExpiryPick::Until(NEXT_MONDAY).resolve(0),
        Some((NEXT_MONDAY + 1) * DAY)
    );
}

const NOW: u64 = 1_000;

#[test]
fn residents_never_expire() {
    assert_eq!(
        check_expiry(Role::Resident, Some(NOW + 1), NOW),
        Err(ExpiryError::ResidentsNeverExpire)
    );
    assert_eq!(check_expiry(Role::Resident, None, NOW), Ok(()));
}

#[test]
fn guests_always_expire() {
    assert_eq!(
        check_expiry(Role::Guest, None, NOW),
        Err(ExpiryError::GuestsMustExpire)
    );
    assert_eq!(check_expiry(Role::Guest, Some(NOW + 1), NOW), Ok(()));
}

#[test]
fn viewers_may_or_may_not_expire() {
    assert_eq!(check_expiry(Role::Viewer, None, NOW), Ok(()));
    assert_eq!(check_expiry(Role::Viewer, Some(NOW + 1), NOW), Ok(()));
}

#[test]
fn an_expiry_must_lie_strictly_in_the_future() {
    assert_eq!(
        check_expiry(Role::Guest, Some(NOW), NOW),
        Err(ExpiryError::ExpiryNotInFuture)
    );
    assert_eq!(
        check_expiry(Role::Viewer, Some(NOW - 1), NOW),
        Err(ExpiryError::ExpiryNotInFuture)
    );
}

#[test]
fn a_resident_with_a_past_expiry_is_refused_as_a_resident_first() {
    assert_eq!(
        check_expiry(Role::Resident, Some(NOW - 1), NOW),
        Err(ExpiryError::ResidentsNeverExpire)
    );
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex14_expiry_pick;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex14_expiry_pick --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex14_expiry_pick 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 11 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex14_expiry_pick.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex14_expiry_pick 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 11 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex14_expiry_pick` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/05-people-and-pwa.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex14_expiry_pick.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex14_expiry_pick.rs        # 3
grep -n '@' src/ex14_expiry_pick.rs tests/ex14_expiry_pick.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex14_expiry_pick.rs exercises/tests/ex14_expiry_pick.rs
git commit -m "docs(exercises): exercise 14, expiry picks (Rust: enums with data, match guards)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 22: Exercise 15 — Embedded files, ETags, a network-first strategy and an SVG drawing

**Files:**
- Create: `exercises/src/ex15_pwa_assets.rs`, `exercises/tests/ex15_pwa_assets.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex15_pwa_assets;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/05-people-and-pwa.md` (Task 20).
- Produces: the public items `Asset`, `etag_of`, `Reply`, `respond`, `network_first`, `svg_path`, `svg`, used only by this exercise's own test file.

**What the learner practises:** `&'static` data and a function taking `&'static [Asset]`, `Iterator::find`, `Option<&str>` comparison, generic functions taking `FnOnce` closures (`impl FnOnce`), `Result::ok` and `Option::or_else`, building a `String` with `write!`
**Difficulty:** two stars (about 45 minutes). **Tests:** 12. **`todo!()` bodies in the stub:** 5.

- [ ] **Step 1: Create the stub `exercises/src/ex15_pwa_assets.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 15: static assets, caching and a QR-style drawing. `'static` data, closures and
//! `String` building.
//!
//! Practises: `docs/learning/05-people-and-pwa.md`, sections 4, 6 and 7.
//! Mirrors: `smoke_web::assets::{respond, ETAGS}`, the network-first `fetch` handler in
//! `static/sw.js`, and `smoke_web::qr::svg`.
//! Run: `cargo test --test ex15_pwa_assets`
//!
//! Hint: `respond` finds the asset with `iter().find(...)`; an `ETag` is a quoted validator, and
//! a browser that already has the file sends it back in `If-None-Match`, so equal means
//! "304, send no body". `write!(out, "...")` (from `std::fmt::Write`) appends formatted text to
//! a `String`; its `Result` cannot fail for a `String`, so `.expect(...)` is fine there.
//!
//! The app's tags are the first 16 hex characters of a SHA-256; the standard library has none,
//! so `etag_of` uses `DefaultHasher`.

use std::fmt::Write;
use std::hash::{DefaultHasher, Hash, Hasher};

pub struct Asset {
    pub path: &'static str,
    pub content_type: &'static str,
    pub body: &'static [u8],
}

/// A strong validator for `body`: a double quote, 16 lowercase hex digits of its hash, a double
/// quote. Equal bodies give equal tags and different bodies (almost always) different ones.
/// `format!("{:016x}", number)` writes a number as 16 hex digits.
pub fn etag_of(body: &[u8]) -> String {
    todo!()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    NotFound,
    /// `If-None-Match` equals the asset's tag: no body.
    NotModified {
        etag: String,
    },
    Found {
        etag: String,
        content_type: &'static str,
        body: &'static [u8],
    },
}

/// The reply for `path` among `assets`.
pub fn respond(assets: &'static [Asset], path: &str, if_none_match: Option<&str>) -> Reply {
    todo!()
}

/// The service worker's strategy for assets: ask the network first; if that fails, fall back
/// to the cached copy. `cached` must not run when the network answered (take it as `FnOnce` and
/// call it only on the error path). `None` when both fail.
pub fn network_first<T, E>(
    fetch: impl FnOnce() -> Result<T, E>,
    cached: impl FnOnce() -> Option<T>,
) -> Option<T> {
    todo!()
}

/// The `d` attribute of an SVG `<path>` that draws every dark module as a 1x1 square: for the
/// module in column `x` and row `y`, `M{x+quiet},{y+quiet}h1v1h-1z`, rows top to bottom and
/// each row left to right, all concatenated with no separator.
pub fn svg_path(modules: &[Vec<bool>], quiet_zone: usize) -> String {
    todo!()
}

/// The whole drawing: an `<svg>` element with `xmlns="http://www.w3.org/2000/svg"`,
/// `viewBox="0 0 S S"` where S is the number of rows plus twice the quiet zone, and one
/// `<path d="..." fill="#000"/>` inside.
pub fn svg(modules: &[Vec<bool>], quiet_zone: usize) -> String {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex15_pwa_assets.rs`** with exactly this content (the
learner never edits it):

```rust
use std::cell::Cell;

use smoke_exercises::ex15_pwa_assets::{
    Asset, Reply, etag_of, network_first, respond, svg, svg_path,
};

static ASSETS: &[Asset] = &[
    Asset {
        path: "app.css",
        content_type: "text/css; charset=utf-8",
        body: b"body { margin: 0 }",
    },
    Asset {
        path: "app.js",
        content_type: "text/javascript; charset=utf-8",
        body: b"console.log('hi')",
    },
];

#[test]
fn an_etag_is_a_quoted_hash_of_sixteen_hex_digits() {
    let tag = etag_of(b"hello");
    assert_eq!(tag.len(), 18);
    assert!(tag.starts_with('"') && tag.ends_with('"'));
    assert!(
        tag[1..17]
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}

#[test]
fn equal_bodies_share_a_tag_and_different_bodies_do_not() {
    assert_eq!(etag_of(b"same"), etag_of(b"same"));
    assert_ne!(etag_of(b"one"), etag_of(b"two"));
}

#[test]
fn an_unknown_path_is_not_found() {
    assert_eq!(respond(ASSETS, "missing.css", None), Reply::NotFound);
}

#[test]
fn a_known_path_is_served_with_its_tag() {
    let reply = respond(ASSETS, "app.css", None);
    assert_eq!(
        reply,
        Reply::Found {
            etag: etag_of(b"body { margin: 0 }"),
            content_type: "text/css; charset=utf-8",
            body: b"body { margin: 0 }",
        }
    );
}

#[test]
fn a_browser_with_the_current_tag_gets_not_modified() {
    let tag = etag_of(b"console.log('hi')");
    assert_eq!(
        respond(ASSETS, "app.js", Some(&tag)),
        Reply::NotModified { etag: tag.clone() }
    );
}

#[test]
fn a_browser_with_an_old_tag_gets_the_new_file() {
    let reply = respond(ASSETS, "app.js", Some("\"0000000000000000\""));
    assert!(matches!(reply, Reply::Found { .. }));
}

#[test]
fn the_network_answer_wins_and_the_cache_is_not_even_asked() {
    let asked = Cell::new(false);
    let got = network_first(
        || Ok::<_, ()>("fresh"),
        || {
            asked.set(true);
            Some("stale")
        },
    );
    assert_eq!(got, Some("fresh"));
    assert!(!asked.get());
}

#[test]
fn when_the_network_fails_the_cached_copy_is_used() {
    let got = network_first(|| Err::<&str, _>("offline"), || Some("stale"));
    assert_eq!(got, Some("stale"));
}

#[test]
fn when_both_fail_there_is_nothing() {
    let got = network_first(|| Err::<&str, _>("offline"), || None);
    assert_eq!(got, None);
}

fn grid() -> Vec<Vec<bool>> {
    vec![
        vec![true, false, true],
        vec![false, true, false],
        vec![true, false, false],
    ]
}

#[test]
fn every_dark_module_becomes_one_square_inside_the_quiet_zone() {
    assert_eq!(
        svg_path(&grid(), 4),
        "M4,4h1v1h-1zM6,4h1v1h-1zM5,5h1v1h-1zM4,6h1v1h-1z"
    );
}

#[test]
fn an_empty_grid_draws_no_path() {
    assert_eq!(svg_path(&[], 4), "");
    assert_eq!(svg_path(&[vec![false, false]], 0), "");
}

#[test]
fn the_drawing_is_sized_by_the_grid_and_the_margin() {
    let drawing = svg(&grid(), 4);
    assert!(drawing.starts_with("<svg "));
    assert!(drawing.ends_with("</svg>"));
    assert!(drawing.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(drawing.contains("viewBox=\"0 0 11 11\""));
    assert!(drawing.contains(&format!(
        "<path d=\"{}\" fill=\"#000\"/>",
        svg_path(&grid(), 4)
    )));
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex15_pwa_assets;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex15_pwa_assets --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex15_pwa_assets 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 12 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex15_pwa_assets.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex15_pwa_assets 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 12 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex15_pwa_assets` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/05-people-and-pwa.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex15_pwa_assets.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex15_pwa_assets.rs        # 5
grep -n '@' src/ex15_pwa_assets.rs tests/ex15_pwa_assets.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex15_pwa_assets.rs exercises/tests/ex15_pwa_assets.rs
git commit -m "docs(exercises): exercise 15, assets, ETags and network-first (Rust: 'static data, FnOnce closures, String building)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---


### Task 23: Note 06 — graphs

**Files:**
- Create: `docs/learning/06-graphs.md`

**Interfaces:**
- Consumes: decisions #11, #123 to #130 in `docs/DECISIONS.md`; the code listed below.
- Produces: the note that Exercises 16 and 17 link back to (section 2 is Exercise 16; section 3
  is Exercise 17: keep this numbering).

**Covers:** pure functions from reports to chart numbers; periods and buckets (`successors`);
grouping with `BTreeMap` and the entry API; `Option` ordering, `bool::then` and exact numeric
conversions; security by construction (filter first); JSON in a data attribute and a vendored
Chart.js under a strict CSP; an HTML table as a heatmap; documentation kept honest by a test;
CI additions (CodeQL, coverage); test databases that clean up with `Drop`.

**The note's structure (use exactly this skeleton, in this order):**

```markdown
# 06 — Graphs

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 16 (buckets), 17 (series). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn
(5 to 8 one-line bullets: the Rust ideas, in plain words.)

## The map
(One short paragraph: which part of the app this note covers, then a bullet list of the
crates and files involved, each as a checked `path` or `path::Symbol` reference with a
half-line description.)

## 1. <Concept title>
**The idea.** (2 to 5 sentences in plain English. Define every term the first time. Say what
problem the feature solves before saying what it is called.)

**In our code.** (Bullets. Each: a `path::Symbol` reference and one line on what to look at
there. At most two or three tiny quotes of real code, six lines each at most, copied exactly.)

**Why this way.** (What we chose and why, then one to three alternatives, each with the real
reason it lost here. Name the decision number from `docs/DECISIONS.md` when there is one.
Say when the alternative would have been the better choice, so the reader learns the trade.)

**Read more.** (Two to four links: the Rust Book chapter first, then std or crate docs.)

**Try it.** (Only where the concept list names an exercise: one sentence and the link
`../../exercises/src/exNN_name.rs`.)

## 2. <Concept title>
(…same four parts…)

## Check yourself
(Five or six questions, no answers. They are what "quiz me on this note" is built from.
Mix: "what happens if…", "why is X a `Y` and not a `Z`", "where would you change…".)

## Where next
(One line pointing to the next note, one line listing this note's exercises.)
```

**Rules for the prose.** Write for someone who has read about half of the Rust Book. Short
sentences; no jargon without a one-sentence explanation the first time; no "simply" or "just";
no emojis; no first-person plural about the author's feelings ("we love"). Say "the app" or
"our code". Every statement about Rust must be true for Rust 1.94 or later, edition 2024; when a
statement is true only in some situations, say which. Quote code only from the real file and
copy it exactly. Link an exercise as a normal Markdown link with a relative path, `[Exercise 3](../../exercises/src/ex03_text_enum.rs)`, never as a backticked reference (the exercises are created by later tasks). Do not put any email address in the note. A note is between 200 and 450 lines.

**The concepts to cover, in this order** (they become sections 1, 2, 3 … of the note, numbered exactly like this list, because the exercises refer to a section by its number; the heading text is yours. The references are real at `5b52604`: confirm each in the code before writing):

1. **Pure functions from reports to numbers.**
   - In our code: `crates/domain/src/graphs.rs::intensity_lines`,
     `crates/domain/src/graphs.rs::events_per_bucket`, `crates/domain/src/graphs.rs::room_hours`,
     `crates/domain/src/graphs.rs::symptom_counts`, `crates/domain/src/graphs.rs::area_levels`
     (each takes `&[SmellReport]` and returns plain data; the module comment says the caller has
     already limited the reports to one period and to what the viewer may see, #123);
     `crates/web/src/routes/graphs.rs::show` (fetches, filters with `can_see`, calls the domain,
     builds the page).
   - Why: the arithmetic (which day an event counts on, how a cell is averaged) is where the
     mistakes are, so it is the part with no I/O and many unit tests. Alternatives: compute
     in SQL with `GROUP BY` (two dialects, local days and Monday weeks are awkward in SQL, and a
     guest filter would have to be repeated); compute in the browser (the rules would be in
     JavaScript, untested by `cargo test`).
   - Read more: Book ch. 13.2 `https://doc.rust-lang.org/book/ch13-02-iterators.html`.
2. **Periods and buckets.** (Exercise 16)
   - In our code: `crates/domain/src/graphs.rs::Period` (`Month(NaiveDate)` or `Year(NaiveDate)`),
     with `span`, `first_day`, `previous`, `next`, `buckets` (`std::iter::successors` followed by
     `take_while`: "the first bucket, then one step further each time, until the end") and
     `bucket_of` (a week is named by its Monday: `num_days_from_monday`);
     `crates/domain/src/schedule.rs::DailySchedule::year_span` (chrono's `Months` and
     `with_ordinal`); `crates/domain/src/graphs.rs::zeroed` (a **generic** helper
     `fn zeroed<T: Copy>(...)` that fills every bucket with one starting value);
     `crates/domain/src/graphs.rs::events_per_bucket` (`BTreeMap::entry().and_modify(..).or_insert(..)`
     keeps the earliest report per event; an event counts once, as in the summary card, #123).
   - Why: a graph needs every bucket, also the empty ones; building the list of buckets first
     and then filling it avoids missing zeros. Weeks that start before the year do so because
     Monday-to-Sunday weeks do not align with years. Alternatives: group by `NaiveDate::iso_week`
     (ISO weeks put early January days in the previous year's week 52 or 53 and make the first
     bar confusing); a rolling 30 days (does not match the Events list).
   - Read more: std `successors` `https://doc.rust-lang.org/std/iter/fn.successors.html`, std
     `BTreeMap` `https://doc.rust-lang.org/std/collections/struct.BTreeMap.html`,
     `https://docs.rs/chrono`.
   - Try it: Exercise 16 `exercises/src/ex16_buckets.rs`.
3. **From reports to series: grouping with `BTreeMap`.** (Exercise 17)
   - In our code: `crates/domain/src/graphs.rs::intensity_lines` (`BTreeMap<UserId, Vec<_>>`,
     `entry(..).or_default().push(..)`, then `sort_by_key` and `map`/`collect`);
     `crates/domain/src/graphs.rs::room_hours` (a `BTreeMap<RoomId, [LevelTally; 24]>`: arrays up to
     32 elements implement `Default`, so `or_default()` makes a row of 24 empty tallies);
     `crates/domain/src/graphs.rs::symptom_counts` (`sort_by(|a, b|
     b.1.cmp(&a.1).then(a.0.cmp(&b.0)))`: most frequent first, then by id);
     `crates/domain/src/graphs.rs::LevelTally` (`Default` derived).
   - Why: a `BTreeMap` iterates in key order, so the output order is stable without a sort and
     tests can compare whole vectors; a `HashMap` would need an explicit sort for determinism.
     Alternatives: `HashMap` plus sort (more code); `Itertools::group_by` (a dependency).
   - Read more: std `BTreeMap` `https://doc.rust-lang.org/std/collections/struct.BTreeMap.html`,
     std `Default` `https://doc.rust-lang.org/std/default/trait.Default.html`, std `Ord`
     `https://doc.rust-lang.org/std/cmp/trait.Ord.html`.
   - Try it: Exercise 17 `exercises/src/ex17_series.rs`.
4. **`Option` ordering, `bool::then` and exact conversions.** (Exercise 17)
   - In our code: `crates/domain/src/graphs.rs::area_levels` (`*highest = (*highest).max(Some(room.level))`
     works because `None` is less than every `Some`, so "nothing yet" loses to any level);
     `crates/domain/src/graphs.rs::LevelTally::mean` (`(self.checks > 0).then(|| ...)` turns a
     condition into an `Option`; `f64::from(u32)` converts exactly, where `as f64` would also compile
     for lossy cases); `u32::from(room.level.get())` in `room_hours`.
   - Why: `From` conversions exist only where nothing can be lost, so using them makes the
     compiler check that; `as` always compiles and can truncate silently. Alternatives: `as`
     casts (fine for display, risky for data); `try_from` (for the cases that can fail).
   - Read more: std `Option` `https://doc.rust-lang.org/std/option/enum.Option.html`, std `From`
     `https://doc.rust-lang.org/std/convert/trait.From.html`.
5. **Security by construction: filter first, then count.**
   - In our code: `crates/web/src/routes/graphs.rs::show` (every report passes
     `smoke_domain::permissions::can_see` **before** any number is computed, so a guest's graphs
     are made from the guest's own reports only and no total can leak another person's data);
     `crates/domain/src/permissions.rs::can_see`; the test
     `crates/web/tests/graphs.rs::guests_see_graphs_of_their_own_reports_only`; decision #11
     (spec §4).
   - Why: filtering after counting would leak through totals and averages; filtering at the
     edge makes the pure functions safe by their input. Alternatives: pass the viewer into every
     function (each would have to remember to check); filter in SQL (a second copy of the rule).
   - Read more: Book ch. 13.2 `https://doc.rust-lang.org/book/ch13-02-iterators.html`.
6. **From numbers to a chart: JSON in a data attribute, a vendored library and the CSP.**
   - In our code: `crates/web/src/routes/graphs.rs::intensity_chart`,
     `crates/web/src/routes/graphs.rs::count_chart`, `crates/web/src/routes/graphs.rs::symptom_chart`,
     `crates/web/src/routes/graphs.rs::area_chart` (each builds a `serde_json::Value` with the
     `json!` macro: the Chart.js configuration); `crates/web/templates/graphs.html` (the JSON goes
     into a `data-chart` attribute, escaped by Askama); `crates/web/static/graphs.js` (reads each
     attribute and draws it, adding only the time-axis labels written with `Intl.DateTimeFormat`
     in the app's time zone, #95); the vendored `crates/web/static/chart-4.5.1.umd.min.js`, whose
     SHA-256 is pinned by `crates/web/src/assets.rs::the_vendored_libraries_are_the_published_releases`
     (#124); `crates/web/src/routes/graphs.rs::intl_locale` and `crates/web/src/routes/graphs.rs::period`
     (a hand-typed `?month=2026-13` falls back to the current month, status 200).
   - Why: the Content-Security-Policy allows only same-origin scripts, so the configuration cannot
     sit in an inline `<script>`; a data attribute is inert text the page's own script reads.
     Vendoring Chart.js (a copy committed to the repository, checked by hash) means no CDN, no
     third-party request and a build that never changes by itself. Alternatives: a JSON
     endpoint per chart (more requests); SVG drawn by the server (no interaction, a lot of
     drawing code); a Rust-to-WASM chart (a toolchain for little gain).
   - Read more: `https://docs.rs/serde_json`, `https://www.chartjs.org/docs/latest/`, MDN CSP
     `https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP`, MDN `Intl.DateTimeFormat`
     `https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat`.
7. **A heatmap as an HTML table.**
   - In our code: `crates/web/src/routes/graphs.rs::HeatRow` and
     `crates/web/src/routes/graphs.rs::heat_rows` (one row per room, 24 cells, each the rounded
     average with a CSS class `heat-0` to `heat-10`); `crates/web/static/app.css` (`.heat-0` …
     `.heat-10`, the accent colour at rising strength); decision #126.
   - Why: a table works without JavaScript, reads correctly with a screen reader, and classes (not
     `style=` attributes) keep the CSP strict. Alternatives: a Chart.js matrix plugin (a second
     library); a canvas (not accessible).
   - Read more: MDN CSP `https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP`.
8. **Documentation that a test keeps honest.**
   - In our code: `crates/web/src/routes/guide.rs::levels`, `crates/web/src/routes/guide.rs::situations`
     and `crates/web/src/routes/guide.rs::rules` (the guide texts come from the translation files);
     the test `crates/web/src/routes/guide.rs::the_readme_tables_match_the_english_guides`, which
     reads `README.md` **at compile time** with `include_str!` and checks that each guide row appears
     in it (#130); `crates/web/tests/guides.rs`.
   - Why: the spec asks for the same tables in the app and in the README; a test catches an edit to
     one side only without a generator to maintain. `include_str!` makes the file part of the
     build, so a missing README is a compile error. Alternatives: generate the README from the
     translations (a build step to maintain); link to the spec instead of copying (the README
     would not stand alone).
   - Read more: std `include_str!` `https://doc.rust-lang.org/std/macro.include_str.html`, Book
     ch. 11.1 `https://doc.rust-lang.org/book/ch11-01-writing-tests.html`.
9. **CI additions: CodeQL and coverage.**
   - In our code: `.github/workflows/codeql.yml` (a matrix over `rust`, `actions` and
     `javascript-typescript`, `build-mode: none`, vendored files ignored, #128);
     `.github/workflows/ci.yml` (the `coverage` job: `cargo llvm-cov --workspace --locked
     --no-report` on SQLite and a Postgres service, a summary in the job page, `lcov.info` kept
     14 days, a third-party action pinned to a **commit**, not a tag, #129).
   - Why: coverage shows which code the tests never run; it is a map, not a score (no
     threshold, so nobody writes a test just for the number). Pinning to a commit protects
     against a moved tag. Alternatives: Codecov (an account and a token); `cargo-tarpaulin`
     (less exact than llvm's instrumentation on stable).
   - Read more: `https://github.com/taiki-e/cargo-llvm-cov`.
10. **Test databases that clean up after themselves.**
    - In our code: `crates/storage-postgres/tests/common/mod.rs::TestDatabase` (implements `Drop`:
      it drops its database when the test ends; the work runs on a **new thread with its own
      runtime**, because the test's runtime may be the very one dropping the value and cannot wait on
      itself); `crates/storage-postgres/tests/common/mod.rs::fresh_database` (a unique name from the
      process id, a timestamp and an `AtomicU32` counter).
    - Why: a test that creates a database per run would otherwise leave thousands behind on the
      developer's server. This is RAII again (Note 02, section 4): cleanup tied to the value's
      lifetime. Alternatives: a cleanup script (forgotten); one shared database with transactions
      rolled back (the tests need to see commits).
    - Read more: Book ch. 15.3 `https://doc.rust-lang.org/book/ch15-03-drop.html`, std atomics
      `https://doc.rust-lang.org/std/sync/atomic/index.html`.

**Check yourself** (use or adapt; the note needs five or six): Why does `buckets` start from
`bucket_of(first_day)` and not from `first_day`? Why is the heatmap's cell an average and the
area chart's value a maximum? What does `Option<Level>`'s ordering do in `area_levels`? Where
exactly is a guest kept from seeing other people's data in the graphs, and what would break if
that moved after the counting? Why can the chart configuration not be an inline `<script>`? What
does `the_readme_tables_match_the_english_guides` fail on, and why can it read `README.md` with
`include_str!`?

- [ ] **Step 1: Read the code before writing a sentence about it**

Open every file named in the concept list (the whole file, not only the symbol). For each
concept confirm in the code that what the list says is true; the list was written from a
reading of the code at `5b52604` and may be imprecise. Where the code differs from the list,
the code wins: write what the code does. Check the base first:

```bash
git rev-parse --short HEAD~0 && git merge-base HEAD main
```

The note's "written against" line stays `5b52604` as long as `crates/` is identical to it:
`git diff 5b52604 --stat -- crates` must print nothing.

- [ ] **Step 2: Write the note**

Create `docs/learning/06-graphs.md` with the Write tool, following the skeleton and prose rules
above and covering every concept in the list, in that order. Add the "Check yourself" and
"Where next" sections. Do not invent concepts the list does not name, and do not drop any.

- [ ] **Step 3: Check every code reference**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/06-graphs.md
```

Expected: `N references checked, 0 failed`. Fix any failure by correcting the reference after
reading the code (never by deleting the sentence that needed it).

- [ ] **Step 4: Check every link**

```bash
grep -oE 'https?://[^ )>`"*]+' docs/learning/06-graphs.md | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-06.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-06.txt
```

Expected: only `checked N urls`. Then, for each link, check that its text matches what the page
is about (a link labelled "Rust Book: ownership" must open the ownership chapter).

- [ ] **Step 5: Self-review against the rules**

Re-read the note once as the learner would. Confirm: every concept in the list is there; every
"Why this way" names at least one real alternative and the reason it lost; every Rust claim is
true (when unsure, open the std or Book page and read it); no line numbers; no email address
(`grep -n '@' docs/learning/06-graphs.md`: a hit may only be code copied from the repository, never an
address); the exercise links point at files that
exist after the exercise tasks (`exercises/src/exNN_*.rs`; they are created later in this
milestone, so the links are checked in Task 27, not here); length 200 to 450 lines
(`wc -l`).

- [ ] **Step 6: Confirm the app is untouched, then commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/06-graphs.md
git commit -m "docs(learning): note 06, BTreeMap grouping, iterator buckets and charts under a strict CSP

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'                                          # author locadani, trailer exact
```

---

### Task 24: Exercise 16 — Counting per day or per week

**Files:**
- Create: `exercises/src/ex16_buckets.rs`, `exercises/tests/ex16_buckets.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex16_buckets;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/06-graphs.md` (Task 23).
- Produces: the public items `weekday_from_monday` (given), `Period`, `events_per_bucket`, used only by this exercise's own test file.

**What the learner practises:** an enum with struct-like variants, `std::iter::successors` with `take_while`, `BTreeMap` with `entry().and_modify().or_insert()`, `rem_euclid` for negative numbers, a `match` over `self` that shares an arm with `
**Difficulty:** three stars (about 50 minutes). **Tests:** 10. **`todo!()` bodies in the stub:** 4.

- [ ] **Step 1: Create the stub `exercises/src/ex16_buckets.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 16: counting per day or per week. `BTreeMap`, `std::iter::successors` and the
//! entry API.
//!
//! Practises: `docs/learning/06-graphs.md`, section 2.
//! Mirrors: `smoke_domain::graphs::{Period, events_per_bucket}`.
//! Run: `cargo test --test ex16_buckets`
//!
//! Hint: a *bucket* is the first day of the day or week a date falls in, so a graph can show
//! one bar per bucket. Days are numbers (day 0 = 1970-01-01, a Thursday); a week runs Monday to
//! Sunday and is named by its Monday. `std::iter::successors(Some(first), |&d| Some(d + step))`
//! produces first, first + step, first + 2 * step... and `take_while` stops it.
//! In `events_per_bucket`, `BTreeMap::entry(event).and_modify(|d| *d = (*d).min(day))
//! .or_insert(day)` keeps the earliest day per event.

use std::collections::BTreeMap;

/// Given: 0 for Monday up to 6 for Sunday.
pub fn weekday_from_monday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    /// `len` days from `first`, counted per day.
    Days { first: i64, len: i64 },
    /// `len` days from `first`, counted per week (Monday to Sunday).
    Weeks { first: i64, len: i64 },
}

impl Period {
    /// The first day after the period.
    pub fn end(&self) -> i64 {
        todo!()
    }

    /// The first day of the bucket that holds `day`: the day itself, or the Monday of its week.
    pub fn bucket_of(&self, day: i64) -> i64 {
        todo!()
    }

    /// The first day of every bucket, in order. For weeks the first bucket may start before
    /// the period does (the Monday before `first`); no bucket starts at or after `end`.
    pub fn buckets(&self) -> Vec<i64> {
        todo!()
    }
}

/// The number of events in each bucket of `period`, every bucket listed (also with 0), in
/// order. `reports` are `(event, day)` pairs; an event counts ONCE, in the bucket of its
/// earliest report; a report without an event (`None`, a clean check) counts for nothing; an
/// event whose earliest day is outside the period is left out.
pub fn events_per_bucket(period: Period, reports: &[(Option<u32>, i64)]) -> Vec<(i64, usize)> {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex16_buckets.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex16_buckets::{Period, events_per_bucket};

// 2026-10-19 is a Monday: day number 20745.
const MON: i64 = 20_745;
const TUE: i64 = MON + 1;
const SUN: i64 = MON + 6;
const NEXT_MON: i64 = MON + 7;

#[test]
fn a_day_period_has_one_bucket_per_day() {
    let period = Period::Days { first: MON, len: 7 };
    assert_eq!(period.end(), MON + 7);
    assert_eq!(period.bucket_of(TUE), TUE);
    assert_eq!(
        period.buckets(),
        [MON, MON + 1, MON + 2, MON + 3, MON + 4, MON + 5, SUN]
    );
}

#[test]
fn a_week_bucket_is_named_by_its_monday() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    assert_eq!(period.bucket_of(MON), MON);
    assert_eq!(period.bucket_of(SUN), MON);
    assert_eq!(period.bucket_of(NEXT_MON), NEXT_MON);
}

#[test]
fn weeks_start_on_monday_even_when_the_period_starts_midweek() {
    // Wednesday 2026-10-14 for 14 days: the weeks of Mon 12th, 19th and 26th touch it.
    let period = Period::Weeks {
        first: MON - 5,
        len: 14,
    };
    assert_eq!(period.buckets(), [MON - 7, MON, NEXT_MON]);
}

#[test]
fn no_bucket_starts_at_or_after_the_end() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    assert_eq!(period.buckets(), [MON, NEXT_MON]);
    let one_day = Period::Days { first: MON, len: 1 };
    assert_eq!(one_day.buckets(), [MON]);
}

#[test]
fn an_empty_period_has_no_buckets() {
    assert!(Period::Days { first: MON, len: 0 }.buckets().is_empty());
}

#[test]
fn every_bucket_is_listed_even_without_events() {
    let period = Period::Days { first: MON, len: 3 };
    assert_eq!(
        events_per_bucket(period, &[]),
        [(MON, 0), (MON + 1, 0), (MON + 2, 0)]
    );
}

#[test]
fn an_event_counts_once_on_the_day_of_its_earliest_report() {
    let period = Period::Days { first: MON, len: 4 };
    let reports = [
        (Some(1), MON + 2),
        (Some(1), MON),
        (Some(1), MON + 1),
        (Some(2), MON + 1),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 1), (MON + 1, 1), (MON + 2, 0), (MON + 3, 0)]
    );
}

#[test]
fn clean_checks_and_events_outside_the_period_count_for_nothing() {
    let period = Period::Days { first: MON, len: 2 };
    let reports = [
        (None, MON),
        (None, MON + 1),
        (Some(5), MON - 1),
        (Some(6), MON + 9),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 0), (MON + 1, 0)]
    );
}

#[test]
fn a_year_style_period_counts_events_per_week() {
    let period = Period::Weeks {
        first: MON,
        len: 14,
    };
    let reports = [
        (Some(1), TUE),
        (Some(2), SUN),
        (Some(3), NEXT_MON),
        (Some(3), NEXT_MON + 3),
    ];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 2), (NEXT_MON, 1)]
    );
}

#[test]
fn an_event_that_began_before_the_period_is_not_counted_in_it() {
    let period = Period::Days { first: MON, len: 3 };
    let reports = [(Some(1), MON - 1), (Some(1), MON + 1)];
    assert_eq!(
        events_per_bucket(period, &reports),
        [(MON, 0), (MON + 1, 0), (MON + 2, 0)]
    );
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex16_buckets;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex16_buckets --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex16_buckets 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 10 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex16_buckets.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex16_buckets 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 10 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex16_buckets` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/06-graphs.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex16_buckets.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex16_buckets.rs        # 4
grep -n '@' src/ex16_buckets.rs tests/ex16_buckets.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex16_buckets.rs exercises/tests/ex16_buckets.rs
git commit -m "docs(exercises): exercise 16, graph buckets (Rust: iter::successors, BTreeMap entry API)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 25: Exercise 17 — From reports to chart numbers

**Files:**
- Create: `exercises/src/ex17_series.rs`, `exercises/tests/ex17_series.rs`
- Modify: `exercises/src/lib.rs` (add the line `pub mod ex17_series;`)

**Interfaces:**
- Consumes: the package from Task 2 and the note `docs/learning/06-graphs.md` (Task 23).
- Produces: the public items `Area`, `Report`, `PersonLine`, `intensity_lines`, `LevelTally`, `RoomHours`, `room_hours`, `symptom_counts`, `AreaLevels`, `area_levels`, used only by this exercise's own test file.

**What the learner practises:** grouping with `BTreeMap` and `or_default`, a `[T; 24]` array as a map value, `sort_by` with `then`, `Option` ordering with `max(Some(..))`, `bool::then`, exact conversions with `f64::from`
**Difficulty:** two stars (about 45 minutes). **Tests:** 9. **`todo!()` bodies in the stub:** 5.

- [ ] **Step 1: Create the stub `exercises/src/ex17_series.rs`** with exactly this content (it is the
file the learner edits, so it must contain no solution):

```rust
//! Exercise 17: turning reports into chart numbers. `BTreeMap`, `or_default`, sorting and
//! `Option` ordering.
//!
//! Practises: `docs/learning/06-graphs.md`, section 3.
//! Mirrors: `smoke_domain::graphs::{intensity_lines, room_hours, symptom_counts, area_levels,
//! LevelTally}`.
//! Run: `cargo test --test ex17_series`
//!
//! Hint: group with a `BTreeMap` (its iteration is sorted by key, so the output is in a stable
//! order for free): `map.entry(key).or_default()` gives `&mut V` for a key, inserting
//! `V::default()` the first time. `sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)))` sorts
//! descending by one field and then ascending by another. `Option<T>` orders `None` below every
//! `Some`, so `highest.max(Some(level))` keeps the larger of "nothing yet" and a level.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Inside,
    Common,
}

/// One smell report, as far as the graphs care.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub user: u32,
    /// Seconds since the epoch.
    pub at: u64,
    /// The local hour of the day, 0 to 23.
    pub hour: usize,
    /// `(room, level)` pairs; empty means the whole home was fine.
    pub rooms: Vec<(u32, u8)>,
    pub symptoms: Vec<u32>,
}

/// One person's points as `(time, intensity)`, in time order. A report's intensity is its
/// highest room level, 0 without rooms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonLine {
    pub user: u32,
    pub points: Vec<(u64, u8)>,
}

/// One line per person, ordered by user id.
pub fn intensity_lines(reports: &[Report]) -> Vec<PersonLine> {
    todo!()
}

/// The levels one room was given at one hour: how many reports checked it (a clean check, level
/// 0, included) and the sum of their levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LevelTally {
    pub checks: u32,
    pub total: u32,
}

impl LevelTally {
    /// The average level, or `None` when nobody checked: `total / checks` as `f64`.
    pub fn mean(&self) -> Option<f64> {
        todo!()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoomHours {
    pub room: u32,
    pub hours: [LevelTally; 24],
}

/// A row per room that at least one report names, ordered by room id; each row has the tallies
/// of the 24 hours. A report naming no room adds nothing.
pub fn room_hours(reports: &[Report]) -> Vec<RoomHours> {
    todo!()
}

/// How many reports name each symptom, most frequent first, then by symptom id. Symptoms
/// nobody reported are left out.
pub fn symptom_counts(reports: &[Report]) -> Vec<(u32, usize)> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AreaLevels {
    pub inside: Option<u8>,
    pub common: Option<u8>,
}

/// The highest room level reported inside the home and in common areas. `areas` says which
/// area each room is in; a room missing from it is skipped. `None` where no room of that area
/// was checked.
pub fn area_levels(reports: &[Report], areas: &[(u32, Area)]) -> AreaLevels {
    todo!()
}
```

- [ ] **Step 2: Create the tests `exercises/tests/ex17_series.rs`** with exactly this content (the
learner never edits it):

```rust
use smoke_exercises::ex17_series::{
    Area, AreaLevels, LevelTally, PersonLine, Report, area_levels, intensity_lines, room_hours,
    symptom_counts,
};

fn report(user: u32, at: u64, hour: usize, rooms: &[(u32, u8)], symptoms: &[u32]) -> Report {
    Report {
        user,
        at,
        hour,
        rooms: rooms.to_vec(),
        symptoms: symptoms.to_vec(),
    }
}

#[test]
fn each_person_gets_a_line_in_time_order_with_the_highest_room_level() {
    let reports = [
        report(2, 300, 9, &[(1, 4)], &[]),
        report(1, 200, 8, &[(1, 2), (2, 7)], &[]),
        report(1, 100, 7, &[(3, 5)], &[]),
        report(2, 50, 6, &[], &[]),
    ];
    assert_eq!(
        intensity_lines(&reports),
        [
            PersonLine {
                user: 1,
                points: vec![(100, 5), (200, 7)]
            },
            PersonLine {
                user: 2,
                points: vec![(50, 0), (300, 4)]
            },
        ]
    );
}

#[test]
fn no_reports_no_lines() {
    assert!(intensity_lines(&[]).is_empty());
}

#[test]
fn a_tally_averages_and_has_no_mean_without_checks() {
    assert_eq!(LevelTally::default().mean(), None);
    assert_eq!(
        LevelTally {
            checks: 4,
            total: 6
        }
        .mean(),
        Some(1.5)
    );
    assert_eq!(
        LevelTally {
            checks: 1,
            total: 0
        }
        .mean(),
        Some(0.0)
    );
}

#[test]
fn the_heatmap_averages_levels_per_room_and_hour_counting_clean_checks() {
    let reports = [
        report(1, 1, 22, &[(1, 6), (2, 0)], &[]),
        report(1, 2, 22, &[(1, 2)], &[]),
        report(1, 3, 23, &[(2, 4)], &[]),
        report(1, 4, 5, &[], &[]),
    ];
    let rows = room_hours(&reports);
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].room, rows[1].room), (1, 2));
    assert_eq!(
        rows[0].hours[22],
        LevelTally {
            checks: 2,
            total: 8
        }
    );
    assert_eq!(rows[0].hours[22].mean(), Some(4.0));
    assert_eq!(rows[0].hours[23], LevelTally::default());
    assert_eq!(
        rows[1].hours[22],
        LevelTally {
            checks: 1,
            total: 0
        }
    );
    assert_eq!(rows[1].hours[23].mean(), Some(4.0));
    assert_eq!(rows[1].hours.len(), 24);
}

#[test]
fn rooms_nobody_named_are_left_out() {
    assert!(room_hours(&[report(1, 1, 3, &[], &[])]).is_empty());
    assert!(room_hours(&[]).is_empty());
}

#[test]
fn symptoms_are_counted_most_frequent_first_then_by_id() {
    let reports = [
        report(1, 1, 1, &[], &[3, 1]),
        report(1, 2, 1, &[], &[3]),
        report(1, 3, 1, &[], &[2, 3]),
        report(1, 4, 1, &[], &[2]),
        report(1, 5, 1, &[], &[]),
    ];
    assert_eq!(symptom_counts(&reports), [(3, 3), (2, 2), (1, 1)]);
}

#[test]
fn equal_counts_fall_back_to_the_lower_id() {
    let reports = [report(1, 1, 1, &[], &[9]), report(1, 2, 1, &[], &[4])];
    assert_eq!(symptom_counts(&reports), [(4, 1), (9, 1)]);
}

#[test]
fn each_area_keeps_its_highest_level() {
    let areas = [(1, Area::Inside), (2, Area::Inside), (3, Area::Common)];
    let reports = [
        report(1, 1, 1, &[(1, 3), (3, 8)], &[]),
        report(1, 2, 1, &[(2, 6)], &[]),
        report(1, 3, 1, &[(3, 2), (99, 10)], &[]),
    ];
    assert_eq!(
        area_levels(&reports, &areas),
        AreaLevels {
            inside: Some(6),
            common: Some(8)
        }
    );
}

#[test]
fn an_area_nobody_checked_is_none_and_a_clean_check_is_zero_not_none() {
    let areas = [(1, Area::Inside), (3, Area::Common)];
    let only_inside = [report(1, 1, 1, &[(1, 0)], &[])];
    assert_eq!(
        area_levels(&only_inside, &areas),
        AreaLevels {
            inside: Some(0),
            common: None
        }
    );
    assert_eq!(area_levels(&[], &areas), AreaLevels::default());
}
```

- [ ] **Step 3: Register the module**

Add `pub mod ex17_series;` to `exercises/src/lib.rs`, below the previous exercise's `pub mod` line
(exercises stay in numeric order; the first one goes below the `#![allow(...)]` line after one
blank line).

- [ ] **Step 4: Format and compile**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cd exercises
cargo fmt --check && echo fmt-ok
cargo test --test ex17_series --no-run 2>&1 | tail -5     # compiles, no warning
```

If `cargo fmt --check` complains, run `cargo fmt` and compare: the plan's code is already
formatted, so a difference means a copying slip.

- [ ] **Step 5: Prove every test fails on the stub**

```bash
cargo test --test ex17_series 2>&1 | grep -E "test result|panicked|^error"
```

Expected: `test result: FAILED. 0 passed; 9 failed`. Failures are `not yet implemented`
panics, or, for the few stubs that return a quiet default (named in their doc comment),
assertion failures. A test that **passes** on the stub, or a target that does not compile, is a
defect: stop and report it (do not "fix" the tests to make the stub pass).

- [ ] **Step 6: Prove every test passes against the reference solution**

```bash
REF=<scratchpad>/exercises-reference
cp tests/ex17_series.rs "$REF/tests/"
(cd "$REF" && cargo test --test ex17_series 2>&1 | grep -E "test result|^error")
```

Expected: `test result: ok. 9 passed; 0 failed`. The reference solution is kept **out of
git**; never copy anything from `$REF/src` into the repository. (If `$REF` is missing, see
"Verification tools": write one in a scratch folder first.)

- [ ] **Step 7: Check the stub's teaching text**

Read the stub's doc comment as the learner. It must have: `Practises:` (the note and section
number from this task), `Mirrors:` (real app symbols), `Run:` (the exact `cargo test --test
ex17_series` command) and a hint. The hint may name std functions, say how to think about the
problem and warn about a trap; it must not contain code that solves a whole function. Check
that the note `docs/learning/06-graphs.md` has a section numbered as `Practises:` says and that
it links back to this file (`exercises/src/ex17_series.rs`); if the note task has not produced that
section yet, report it. Then:

```bash
grep -c 'todo!' src/ex17_series.rs        # 5
grep -n '@' src/ex17_series.rs tests/ex17_series.rs   # no hit may be an email address
```

- [ ] **Step 8: Confirm the app is untouched, then commit**

```bash
cd ..
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git status --short                                                    # only the three exercise files
git config user.email                                                 # locadani's GitHub noreply address
git add exercises/src/lib.rs exercises/src/ex17_series.rs exercises/tests/ex17_series.rs
git commit -m "docs(exercises): exercise 17, report series (Rust: BTreeMap, sort_by, Option ordering)

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 26: The index — `docs/learning/README.md`

**Files:**
- Create: `docs/learning/README.md`

**Interfaces:**
- Consumes: the six notes (Tasks 3, 8, 12, 16, 20, 23), `docs/learning/hosting.md`, the 17
  exercises (Tasks 4 to 25), decisions #131 to #134.
- Produces: the pack's front page; nothing links to it from the app.

The index must be **one screen of reading** for the top part (the learner skims it), with the
detail below. Use exactly this structure:

````markdown
# Learning Rust with Smoke Tracker

(Two sentences: what this folder is, and that the app is real, so every note points at real
code.) Written against `main` at commit `5b52604`.

## Start here
(A numbered reading order, one line each, with the note's link and the reason for the order:
01 → 02 → 03 → 04 → 05 → 06; `hosting.md` is read any time after 03. Say that notes 01 to 03 are
the core and 04 to 06 can be read in any order.)

## The notes
| # | Note | You learn | Exercises |
|---|------|-----------|-----------|
(one row per note; the "You learn" column is five or six words; the Exercises column lists
the exercise numbers)

## The exercises
(Three sentences: they are optional, they live in `exercises/` and are **not part of the app**
(they are not built, tested or shipped with it), and the learner writes the code while the
tests are already there.)

### How to run one
```bash
cd exercises
cargo test --test ex01_newtypes     # one exercise
cargo test --no-fail-fast           # all of them, to see the whole picture
```
(Explain, in plain words: every function starts as `todo!()` so every test starts red; open
`exercises/src/ex01_newtypes.rs`, read the doc comment, write the bodies; never edit the files
in `exercises/tests/`; when the tests pass you are done. Say that `cargo` builds into
`exercises/target` unless `CARGO_TARGET_DIR` is set, that this folder is git-ignored, and that
removing it is `cargo clean` run inside `exercises/`.) Mention: solutions are not in the
repository on purpose; the doc comment's hint is the only help; ask Claude when stuck.

### The list
| # | Exercise | Note section | Difficulty | Practises |
(one row per exercise, 17 rows, from the table in this task; the exercise name is a link to its
`../../exercises/src/exNN_name.rs`; the note section is a link to the note)

## Ask Claude
(A short list of ready-to-paste requests, each in a code block or quote, grouped:
- Walkthroughs: "Walk me through `crates/domain/src/audit.rs`, top to bottom, as if I have read
  half the Rust Book." / "Explain section 4 of note 03 again with a different example."
- Quizzes: "Quiz me on note 02, one question at a time, and tell me what I got wrong." / "Give me
  three questions on lifetimes from note 04, harder each time."
- Exercises: "I am stuck on exercise 6, give me a nudge, not the answer." / "Review my solution to
  exercise 8: what would a Rust expert change, and why?" / "Explain why this test fails: <paste>."
- Beyond: "Show me how `<symbol>` would look without <feature>, and what we would lose."
Say plainly that Claude reads the current code, so answers match what is there today.)

## Conventions in the notes
(Four bullets: code is cited as `path::Symbol`, never by line number; "Decision #N" points into
`docs/DECISIONS.md`; each section ends with links to the Rust Book and the std or crate docs; a
"Check yourself" list at the end of every note is what "quiz me" is built from.)

## Other notes
(One line linking [hosting.md](hosting.md): why Cloud Run and Neon, secrets, backups.)
````

The exercise table content (exactly these rows; difficulty is the star count of each exercise
task; "Practises" is a few words):

| # | File | Note section | Stars | Practises |
|---|------|--------------|-------|-----------|
| 1 | `ex01_newtypes` | 01 §3 | 1 | validating constructors, `TryFrom`, a redacting `Debug` |
| 2 | `ex02_permissions` | 01 §5 | 1 | enums with data, nested `match` |
| 3 | `ex03_text_enum` | 01 §6 | 3 | `macro_rules!`, `FromStr`, `Display` |
| 4 | `ex04_audit_chain` | 01 §10 | 2 | slices, loops with state, `Result` |
| 5 | `ex05_event_repo` | 02 §1 | 2 | traits, generics over a trait |
| 6 | `ex06_transaction` | 02 §4 | 2 | ownership, `&mut`, `Drop` |
| 7 | `ex07_store_errors` | 02 §5 | 2 | `Error`, `From`, `?`, `collect` into `Result` |
| 8 | `ex08_login_limiter` | 03 §4 | 3 | `HashMap` entry API, `Mutex`, `Arc` |
| 9 | `ex09_request_rules` | 03 §9, §10 | 2 | `Option` combinators, parsing |
| 10 | `ex10_middleware` | 03 §3, §7 | 3 | `Box<dyn Fn>`, `move` closures, `Any` |
| 11 | `ex11_log_prompt` | 04 §2 | 2 | `match` on tuples, saturating arithmetic |
| 12 | `ex12_report_values` | 04 §4 | 2 | `Vec`, sorting, `Option`, `str` |
| 13 | `ex13_form_pairs` | 04 §6 | 3 | borrowing, lifetimes, `impl Iterator` |
| 14 | `ex14_expiry_pick` | 05 §2 | 2 | enums with data, match guards |
| 15 | `ex15_pwa_assets` | 05 §4, §6, §7 | 2 | `'static` data, `FnOnce`, `write!` |
| 16 | `ex16_buckets` | 06 §2 | 3 | `iter::successors`, `BTreeMap` entry |
| 17 | `ex17_series` | 06 §3, §4 | 2 | `BTreeMap`, `sort_by`, `Option` ordering |

- [ ] **Step 1: Check the inputs exist**

```bash
ls docs/learning/                      # 01-…06 notes, hosting.md
ls exercises/src exercises/tests       # 17 + lib.rs, and 17 test files
grep -c '^pub mod' exercises/src/lib.rs   # 17
```

- [ ] **Step 2: Write the index** with the Write tool, following the structure above. Section
numbers in the table and links must match the notes as written (open each note and confirm the
section numbering follows the concept lists); keep the part above "How to run one" under 40
lines.

- [ ] **Step 3: Check**

```bash
python3 -I <scratchpad>/tools/checkrefs.py "$PWD" docs/learning/README.md
grep -oE '\]\(([^)#]+)' docs/learning/README.md | sed 's/](//' | sort -u | while read -r p; do test -e "docs/learning/$p" && echo "ok $p" || echo "MISSING $p"; done
grep -n '@' docs/learning/README.md          # no email address
```

Expected: `0 failed`; every link target prints `ok`; no address. External links, if any, go
through `checkurls.sh` as in the note tasks.

- [ ] **Step 4: Commit**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile    # prints nothing
git config user.email                                                  # locadani's GitHub noreply address
git add docs/learning/README.md
git commit -m "docs(learning): index with reading order, exercise list and how to ask Claude

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
git log -1 --format='%an%n%B'
```

---

### Task 27: Whole-pack verification

**Files:**
- No file is created. A small fix to a note, an exercise or the index is allowed **only** if a
  check below fails, as a separate commit `docs(learning): fix <what>`.

**Interfaces:**
- Consumes: everything above.
- Produces: the evidence that the milestone is finished, for the reviewers and the summary.

- [ ] **Step 1: The app is untouched**

```bash
git diff 5b52604 --stat -- crates Cargo.toml Cargo.lock Dockerfile rust-toolchain.toml rustfmt.toml deny.toml scripts infra   # nothing
git diff 5b52604 --name-only | sort
```

Expected file list, and nothing else: `.dockerignore`, `.github/workflows/codeql.yml`,
`.github/workflows/deploy.yml`, `docs/DECISIONS.md`, `docs/learning/01-foundation-and-domain.md`,
`02-storage.md`, `03-web-and-auth.md`, `04-logging-flow.md`, `05-people-and-pwa.md`,
`06-graphs.md`, `README.md` (all under `docs/learning/`), `docs/plans/08-learning-pack.md`,
`docs/plans/ROADMAP.md`, and under `exercises/`: `.gitignore`, `Cargo.toml`, `src/lib.rs`, the
17 `src/exNN_*.rs` and the 17 `tests/exNN_*.rs`.

- [ ] **Step 2: The exercises are isolated**

```bash
export CARGO_TARGET_DIR=/Users/daniele/Documents/Personal/smoke-tracker/target
cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(sorted(p['name'] for p in json.load(sys.stdin)['packages']))"
# exactly the six app packages
grep -n 'exercises' .dockerignore .github/workflows/deploy.yml .github/workflows/codeql.yml   # one line each
git diff 5b52604 -- .github/workflows/deploy.yml | grep '^[+-]' | grep -v '^+++\|^---'       # only the one added line
grep -rn 'exercises' Cargo.toml .github/workflows/ci.yml .github/workflows/security.yml Dockerfile   # nothing
cargo fmt --all --check && echo root-fmt-ok
```

- [ ] **Step 3: Every test fails on the stubs and passes on the reference**

```bash
(cd exercises && cargo fmt --check && cargo test --no-fail-fast 2>&1 | grep -E "test result|^warning|^error\[" | awk '/test result/ {p+=$4; f+=$6} /^warning|^error\[/ {print} END {print "stubs: passed",p,"failed",f}')
# expected: no warning line, "stubs: passed 0 failed 166"
REF=<scratchpad>/exercises-reference
cp exercises/tests/*.rs "$REF/tests/"
(cd "$REF" && cargo test 2>&1 | grep -E "test result|^warning|^error\[" | awk '/test result/ {p+=$4; f+=$6} /^warning|^error\[/ {print} END {print "reference: passed",p,"failed",f}')
# expected: "reference: passed 166 failed 0"
```

- [ ] **Step 4: No solution, no secret, no address in git**

```bash
git ls-files exercises | sort                       # lib.rs, 17 stubs, 17 tests, Cargo.toml, .gitignore only
git ls-files | grep -i 'reference\|solution' ; echo "(nothing above means none)"
grep -c 'todo!' exercises/src/ex*.rs                # every stub has todo! (ex06 and ex13 have fewer: see their doc comments)
grep -rn '@' exercises docs/learning | grep -v 'Binary' | head   # no hit may be an email address
git grep -nE '[A-Za-z0-9._-]+@[A-Za-z0-9.-]+\.[a-z]{2,}' -- docs/learning exercises docs/plans/08-learning-pack.md | grep -v 'noreply@anthropic.com'   # nothing
gitleaks git --redact --no-banner --config .gitleaks.toml --log-opts '5b52604..HEAD' 2>&1 | tail -3   # if gitleaks is installed; every commit also passed the pre-commit hook
```

- [ ] **Step 5: Notes and index are accurate**

```bash
for f in docs/learning/0*.md docs/learning/README.md; do python3 -I <scratchpad>/tools/checkrefs.py "$PWD" "$f"; done   # each: 0 failed
cat docs/learning/0*.md docs/learning/README.md | grep -oE 'https?://[^ )>`"*]+' | sed 's/[.,;]$//' | sort -u > <scratchpad>/urls-all.txt
<scratchpad>/tools/checkurls.sh <scratchpad>/urls-all.txt        # only "checked N urls"
for f in docs/learning/0*.md; do grep -oE '\]\((\.\./\.\./exercises/[^)]+)' "$f" | sed 's/](//' | sort -u | while read -r p; do test -e "docs/learning/$p" && echo "ok $p" || echo "MISSING $p in $f"; done; done   # no MISSING
for f in docs/learning/0*.md; do echo "$f $(wc -l < "$f") lines"; done      # each 200 to 450
```

Also confirm that every exercise's doc comment "Practises:" section number exists in its note
and that the heading found there is on the topic the exercise practises (open the 17 stubs'
first lines and the matching headings).

- [ ] **Step 6: Report**

Write the task report: the commit list (`git log --oneline 5b52604..HEAD`), the numbers from
steps 3 and 5, any app issue noticed while reading the code (reported, not fixed; "none found"
if none), the disk state (`df -h /System/Volumes/Data`), and the cleanup commands for the
user, one per build folder:
`cargo clean --target-dir /Users/daniele/Documents/Personal/smoke-tracker/target` (the shared
folder; the exercises used it too). Never run them.

---

## Self-review (done by the planner after writing)

**Spec coverage.** Handoff requirements and where they are met: six notes with concepts,
exact files and symbols, alternatives and real links (Tasks 3, 8, 12, 16, 20, 23); index with
reading order, how to run exercises, how to ask Claude, linking `hosting.md` (Task 26);
decisions from #131 and the roadmap row (Task 1); exercises separate from the app with the
decision recorded and every leak path checked (Task 2, Task 27, decision #132); `todo!()`
stubs with tests, a hint and a link to the note each, solutions not committed (Tasks 4 to 25,
decision #133); zero app code changes (Global Constraints, every commit step, Task 27);
questions for the user (above); bugs listed, not fixed (Global Constraints, Task 27).

**Placeholder scan.** No "TBD", no "similar to Task N": every exercise task carries its full
code; every note task carries its concept list, template and steps. The deliberate openness is
the notes' prose, which the implementer writes from the concept lists.

**Type consistency.** The 17 exercise modules are named exactly as in `lib.rs`, the File
Structure, the index table and each task (`ex01_newtypes` … `ex17_series`); the test count per
file (12, 8, 5, 8, 7, 7, 10, 9, 13, 13, 14, 10, 8, 11, 12, 10, 9 = 166) is the one the planner
measured; section numbers named in the stubs' `Practises:` lines (01 §3, §5, §6, §10; 02 §1, §4,
§5; 03 §4, §9 and §10, §3 and §7; 04 §2, §4, §6; 05 §2, §4 §6 §7; 06 §2, §3) match the
numbering of the concept lists and the index table.

**Review Focus.** Each of the seven items has a task that checks it (see the "Check" lines);
the two that no task's tests exercise directly (a wrong Rust explanation, a note gone stale
by merge time) are covered by the reviewers' reading and by Task 27 step 5.
