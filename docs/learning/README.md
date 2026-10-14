# Learning Rust with Smoke Tracker

This folder is a set of study notes and optional exercises for learning Rust. The app is real and
deployed, so every note points at real code. Written against `main` at commit `5b52604`.

## Start here

1. [01 Foundation and domain](01-foundation-and-domain.md): the types everything else stands on.
2. [02 Storage](02-storage.md): traits and transactions, built on the types of 01.
3. [03 Web and auth](03-web-and-auth.md): handlers and shared state, built on the storage of 02.
4. [04 Logging flow](04-logging-flow.md): the core use case, once 01 to 03 are familiar.
5. [05 People and PWA](05-people-and-pwa.md): accounts, secrets, caching and the install prompt.
6. [06 Graphs](06-graphs.md): iterators and maps turned into charts.

Notes 01 to 03 are the core and are best read in order; 04 to 06 can be read in any order.
[hosting.md](hosting.md) can be read any time after 03.

## The notes

| # | Note | You learn | Exercises |
|---|------|-----------|-----------|
| 01 | [Foundation and domain](01-foundation-and-domain.md) | workspaces, newtypes, enums, macros, errors | 1 to 4 |
| 02 | [Storage](02-storage.md) | traits, async, ownership, `Result` and `?` | 5 to 7 |
| 03 | [Web and auth](03-web-and-auth.md) | Axum, closures, `Arc`, `Mutex`, templates | 8 to 10 |
| 04 | [Logging flow](04-logging-flow.md) | generics, pure rules, lifetimes, view models | 11 to 13 |
| 05 | [People and PWA](05-people-and-pwa.md) | secret types, `move`, `'static`, ETags | 14, 15 |
| 06 | [Graphs](06-graphs.md) | iterators, `BTreeMap`, `Option` ordering, CSP | 16, 17 |

## The exercises

They are optional, and they live in `exercises/`, **not part of the app**: they are not built,
tested or shipped with it. You write the code; the tests are already there. Decisions #131 to #134
in [`docs/DECISIONS.md`](../DECISIONS.md) explain the choices.

### How to run one

```bash
cd exercises
cargo test --test ex01_newtypes     # one exercise
cargo test --no-fail-fast           # all of them, to see the whole picture
```

Every function starts as `todo!()`, so every test starts red. Open
`exercises/src/ex01_newtypes.rs`, read the doc comment, and write the bodies. Never edit the files
in `exercises/tests/`: when the tests pass, you are done. Solutions are not in the repository on
purpose; the doc comment's hint is the only help, and you can ask Claude when stuck.

`cargo` builds into a `target` folder inside `exercises/` unless `CARGO_TARGET_DIR` is set. That folder is
git-ignored, and removing it is `cargo clean` run inside `exercises/`.

### The list

| # | Exercise | Note section | Difficulty | Practises |
|---|----------|--------------|------------|-----------|
| 1 | [`ex01_newtypes`](../../exercises/src/ex01_newtypes.rs) | [01 §3](01-foundation-and-domain.md#3-newtypes-that-cannot-be-invalid) | ★ | validating constructors, `TryFrom`, a redacting `Debug` |
| 2 | [`ex02_permissions`](../../exercises/src/ex02_permissions.rs) | [01 §5](01-foundation-and-domain.md#5-enums-and-match-permissions) | ★ | enums with data, nested `match` |
| 3 | [`ex03_text_enum`](../../exercises/src/ex03_text_enum.rs) | [01 §6](01-foundation-and-domain.md#6-macros-str_enum-and-id_type) | ★★★ | `macro_rules!`, `FromStr`, `Display` |
| 4 | [`ex04_audit_chain`](../../exercises/src/ex04_audit_chain.rs) | [01 §10](01-foundation-and-domain.md#10-the-audit-chain-slices-iterators-and-result) | ★★ | slices, loops with state, `Result` |
| 5 | [`ex05_event_repo`](../../exercises/src/ex05_event_repo.rs) | [02 §1](02-storage.md#1-traits-as-the-storage-boundary) | ★★ | traits, generics over a trait |
| 6 | [`ex06_transaction`](../../exercises/src/ex06_transaction.rs) | [02 §4](02-storage.md#4-ownership-at-work-a-transaction-that-rolls-back-when-dropped) | ★★ | ownership, `&mut`, `Drop` |
| 7 | [`ex07_store_errors`](../../exercises/src/ex07_store_errors.rs) | [02 §5](02-storage.md#5-errors-result--and-from) | ★★ | `Error`, `From`, `?`, `collect` into `Result` |
| 8 | [`ex08_login_limiter`](../../exercises/src/ex08_login_limiter.rs) | [03 §4](03-web-and-auth.md#4-shared-state-arc-mutex-and-the-hashmap-entry-api) | ★★★ | `HashMap` entry API, `Mutex`, `Arc` |
| 9 | [`ex09_request_rules`](../../exercises/src/ex09_request_rules.rs) | [03 §9](03-web-and-auth.md#9-translations-fluent-and-choosing-the-language), [03 §10](03-web-and-auth.md#10-small-pure-functions-around-the-handlers) | ★★ | `Option` combinators, parsing |
| 10 | [`ex10_middleware`](../../exercises/src/ex10_middleware.rs) | [03 §3](03-web-and-auth.md#3-middleware-closures-that-wrap-a-handler), [03 §7](03-web-and-auth.md#7-errors-and-panics-become-pages) | ★★★ | `Box<dyn Fn>`, `move` closures, `Any` |
| 11 | [`ex11_log_prompt`](../../exercises/src/ex11_log_prompt.rs) | [04 §2](04-logging-flow.md#2-business-rules-as-pure-functions) | ★★ | `match` on tuples, saturating arithmetic |
| 12 | [`ex12_report_values`](../../exercises/src/ex12_report_values.rs) | [04 §4](04-logging-flow.md#4-reportvalues-normalise-once-trust-everywhere) | ★★ | `Vec`, sorting, `Option`, `str` |
| 13 | [`ex13_form_pairs`](../../exercises/src/ex13_form_pairs.rs) | [04 §6](04-logging-flow.md#6-forms-with-repeated-names-and-borrowed-data) | ★★★ | borrowing, lifetimes, `impl Iterator` |
| 14 | [`ex14_expiry_pick`](../../exercises/src/ex14_expiry_pick.rs) | [05 §2](05-people-and-pwa.md#2-expiry-picks-enums-with-data) | ★★ | enums with data, match guards |
| 15 | [`ex15_pwa_assets`](../../exercises/src/ex15_pwa_assets.rs) | [05 §4](05-people-and-pwa.md#4-a-qr-code-drawn-as-inline-svg), [05 §6](05-people-and-pwa.md#6-http-caching-embedded-files-and-etags), [05 §7](05-people-and-pwa.md#7-the-pwa-manifest-service-worker-and-the-install-prompt) | ★★ | `'static` data, `FnOnce`, `write!` |
| 16 | [`ex16_buckets`](../../exercises/src/ex16_buckets.rs) | [06 §2](06-graphs.md#2-periods-and-buckets) | ★★★ | `iter::successors`, `BTreeMap` entry |
| 17 | [`ex17_series`](../../exercises/src/ex17_series.rs) | [06 §3](06-graphs.md#3-from-reports-to-series-grouping-with-btreemap), [06 §4](06-graphs.md#4-option-ordering-boolthen-and-exact-conversions) | ★★ | `BTreeMap`, `sort_by`, `Option` ordering |

## Ask Claude

Claude reads the current code, so answers match what is there today. Paste any of these.

Walkthroughs:

> Walk me through `crates/domain/src/audit.rs`, top to bottom, as if I have read half the Rust Book.

> Explain section 4 of note 03 again with a different example.

Quizzes:

> Quiz me on note 02, one question at a time, and tell me what I got wrong.

> Give me three questions on lifetimes from note 04, harder each time.

Exercises:

> I am stuck on exercise 6, give me a nudge, not the answer.

> Review my solution to exercise 8: what would a Rust expert change, and why?

> Explain why this test fails: `<paste>`.

Beyond:

> Show me how `<symbol>` would look without `<feature>`, and what we would lose.

## Conventions in the notes

- Code is cited as `path::Symbol`, never by line number, so a note survives edits.
- "Decision #N" points into [`docs/DECISIONS.md`](../DECISIONS.md).
- Each section ends with links to the Rust Book and to the std or crate docs.
- A "Check yourself" list ends every note; "quiz me" is built from it.

## Other notes

- [hosting.md](hosting.md): why Cloud Run and Neon, secrets, backups.
