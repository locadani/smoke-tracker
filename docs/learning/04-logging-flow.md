# 04 — The logging flow

> Written against commit `eda7a09` (milestone 7b, look and feel). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 11 (log prompts), 12 (report values), 13 (form pairs). Reading time: about
> 55 minutes, plus the exercises.

## What you will learn

- How a function can work with any database by asking only for the traits it needs.
- How a `match` on a tuple turns a table of rules into code the compiler checks.
- How an enum variant can carry data, and why that makes a stale answer detectable.
- How a type with private fields and one constructor guarantees its own shape.
- How errors become translation keys instead of text.
- What a lifetime such as `'a` promises when a function hands out references.
- How a view model keeps decisions in Rust and only printing in the template.
- A few small language features (`let … else`, let chains, `is_none_or`, `matches!`, `map_or`) and the Rust version that added each.

## The map

This note follows one tap on the Log button: the form is read, the app may ask which
event the report belongs to, and then the report is written in one transaction. It also covers
the 04:00 sweep, which uses the same building blocks.

- `crates/domain/src/logbook.rs`: the use cases (`log_smell`, `close_event`, `reopen_event`, `correct_report`, `move_report`) and `LogError`.
- `crates/domain/src/events.rs`: the pure rules (which question to ask, what counts as backdated).
- `crates/domain/src/records.rs`: `ReportValues` and `SmellReport`.
- `crates/domain/src/sweep.rs`: the catch-up sweep that closes events at 04:00.
- `crates/domain/src/repos.rs`: the storage traits the use cases are generic over.
- `crates/web/src/routes/entries.rs`: reads the form, asks the question, runs the use case.
- `crates/web/src/routes/log.rs`: the Log screen and its view models.
- `crates/web/src/forms.rs`: `Pairs` and `QueryPairs`, the extractors for forms and query strings with repeated names.
- `crates/web/src/routes/mod.rs`: `refusal`, which turns a `LogError` into a message.

## 1. Use cases as generic functions over a write transaction

**The idea.** A use case is one thing a person can do: log a smell, close an event, move a
report. Each one checks a permission, looks at the current data, decides, and then writes. If
the decision looks at one version of the data and the write happens against another, the result
can be wrong. So the use case runs inside a transaction: a group of reads and writes that the
database applies all together or not at all. The use case does not open the transaction. The
caller hands it in, and the caller decides when to commit.

**In our code.**

- `crates/domain/src/logbook.rs::log_smell` takes `tx: &mut T` where `T: EventWrites +
  ReportWrites + ReadingWrites`. Read the order inside: the permission check, then
  `classify_observed` and `tx.open_event()`, then the decision, then the writes.
- `crates/domain/src/logbook.rs::close_event`, `crates/domain/src/logbook.rs::reopen_event`,
  `crates/domain/src/logbook.rs::correct_report` and `crates/domain/src/logbook.rs::move_report`
  have the same shape with smaller bounds: only the traits each one needs.
- `crates/domain/src/repos.rs::EventWrites::open_event` documents why the reads live on the transaction:
  "Reads inside the transaction, so a decision and the write that follows it see the same data".
- `crates/web/src/routes/entries.rs::submit` is the caller: `begin`, call the use case, then
  `commit`. On an error it calls `drop(tx)` and never commits.

```rust
    let mut tx = state.store.begin(Some(user.id)).await?;
    match log_smell(&mut tx, &actor, entry.clone(), choice, &schedule, now).await {
```

**Why this way.** The domain crate owns the rules and knows nothing about SQLite or Postgres.
`T` is a generic parameter: a placeholder for any type that implements the three traits. The
compiler writes one copy of `log_smell` for each concrete transaction type it is used with
(static dispatch), so there is no runtime lookup. Both backends pass the same contract tests, so
the rules run identically on both (#86). Because the caller commits, everything the use case
wrote is applied together; the contract test
`crates/storage-contract/src/logbook.rs::a_failed_report_rolls_back_the_new_event_close` shows a
failing report undoing the event close that came before it.

The alternatives each lost for a real reason. Methods on the store (`store.log_smell(...)`)
would hide the transaction boundary inside the store, so a caller could not group two use cases
in one transaction. A service struct holding a connection pool is harder to test, because every
test must build the pool. Rules written in SQL would need two dialects and could not be
unit-tested. A service struct would have been the better choice if the rules needed state that
lives across requests; these rules need none.

**Read more.** [Rust Book: generic data types](https://doc.rust-lang.org/book/ch10-01-syntax.html),
[Rust Book: traits and trait bounds](https://doc.rust-lang.org/book/ch10-02-traits.html).

## 2. Business rules as pure functions

**The idea.** A pure function takes values and returns a value. It reads no clock, opens no
connection and changes nothing else. That makes a rule easy to test: call it with some inputs
and compare the output. The question "what should the Log screen ask?" depends on three facts,
and the spec (§7) lists the answer for every combination. The code is that list.

**In our code.**

- `crates/domain/src/events.rs::classify_observed` decides whether a report time is now, in
  the past, or too far in the future. It returns `Result<ObservedTime, DomainError>`.
- `crates/domain/src/events.rs::log_prompt` matches on the tuple
  `(observed, open_event, unseen_auto_closed)`, one arm per case.
- `crates/domain/src/events.rs::no_smell_prompt`, `crates/domain/src/events.rs::new_event_state`
  and `crates/domain/src/events.rs::can_join_backdated` are the same kind of function.
- The thresholds are `const` values of the `chrono` type `TimeDelta`:
  `crates/domain/src/events.rs::BACKDATE_THRESHOLD`,
  `crates/domain/src/events.rs::FUTURE_TOLERANCE` and `crates/domain/src/events.rs::LARGE_GAP`.
- `crates/domain/src/events.rs::large_gap_indices` finds reports that follow a long gap. The
  event page uses it for the gap warning. `crates/web/src/routes/events.rs::Gap` carries
  `movable`, the report after the gap, which gets a "Move to a new event" button when the viewer
  may move it and it is not a no-smell report (#140).

```rust
    match (observed, open_event, unseen_auto_closed) {
        (ObservedTime::Backdated, _, _) => LogPrompt::WhichEvent,
        (ObservedTime::Now, Some(open), _) => LogPrompt::SameSmell(open),
        (ObservedTime::Now, None, Some(closed)) => LogPrompt::AutoClosed(closed),
        (ObservedTime::Now, None, None) => LogPrompt::Direct,
    }
```

```rust
    observed_times
        .windows(2)
        .enumerate()
        .filter(|(_, pair)| pair[1] - pair[0] > LARGE_GAP)
        .map(|(i, _)| i + 1)
        .collect()
```

**Why this way.** A `match` must be exhaustive: if you add a fourth value to `ObservedTime` or
leave one combination out, the code does not compile. Putting all three facts in a tuple means
the compiler checks the whole table, not one fact at a time. The `_` in the first arm says "the
other two facts do not matter when the report is backdated". In `large_gap_indices`,
`windows(2)` yields every pair of neighbours as a slice, `enumerate` numbers the pairs, and the
`+ 1` turns "pair number `i`" into "the report after the gap".

An `if` chain would also work, but it is easy to leave a case out and nothing warns you. A state
machine crate would be too heavy for four states. The `if` chain would be the better choice if
the decision were a sequence of unrelated early checks rather than a table.

**Read more.** [Rust Book: the match control flow construct](https://doc.rust-lang.org/book/ch06-02-match.html),
[std: slices](https://doc.rust-lang.org/std/primitive.slice.html),
[Rust Book: processing a series of items with iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html).

**Try it.** [Exercise 11](../../exercises/src/ex11_log_prompt.rs): write the prompt rules and the
gap finder yourself.

## 3. A question that travels with its answer

**The idea.** Two phones can use the same home log. You tap Log and the app asks "Same
smell still going?" about the open event. While you read the question, the other phone closes
that event. If your "Yes" were applied to the event as it is now, the report would land in an
event that has ended. The fix is to send the question back together with the answer, so the
server can compare "what you were asked" with "what I would ask now". This idea is called
optimistic concurrency: act on what you saw, and check at the end that it is still true.

**In our code.**

- `crates/domain/src/logbook.rs::EventChoice` is an enum whose variants carry data:
  `New { replacing: Option<EventId> }`, `Join(EventId)` and `Continue(EventId)`.
  `replacing` is the event that was open when the answer was given.
- `crates/domain/src/logbook.rs::LogError` has a `Stale` variant for "the answer no longer fits".
  Inside `log_smell`, the `New` arm compares the open event with `replacing` and returns
  `Stale` when they differ.
- `crates/web/src/routes/entries.rs::question_key` names a question
  (`same-smell-<id>`, `which-event`, `auto-closed-<id>`).
- `crates/web/src/routes/entries.rs::parse_choice` reads the answer the buttons post:
  `new`, `join-<id>` or `continue-<id>`.
- `crates/web/src/routes/entries.rs::submit` compares the posted `asked` with the key of the
  question that applies now. When they differ it asks again.
- `crates/web/src/routes/entries.rs::ask` renders the follow-up page. The first form's fields
  come back as hidden inputs through `Pairs::without`, and `crates/web/templates/prompt.html`
  prints them.

```rust
    match prompt {
        LogPrompt::Direct => None,
        LogPrompt::SameSmell(open) => Some(format!("same-smell-{}", open.id)),
        LogPrompt::WhichEvent => Some("which-event".to_owned()),
        LogPrompt::AutoClosed(closed) => Some(format!("auto-closed-{}", closed.id)),
    }
```

**Why this way.** There are two checks and each catches something the other cannot. The web
layer compares keys before it opens a transaction and shows the new question. The domain layer
checks again inside the transaction, because another request could change the data between the
web layer's read and the write. `Stale` is the second line of defence, and `submit` turns it into
either a fresh question or the message `refused-stale` (#87).

Variants that carry data keep the answer and its context in one value: `Join(EventId)` cannot
exist without an id. With a plain list of names plus a separate id you could forget one half.

The alternatives lose for plain reasons. Remembering the question in the session fails with two
tabs open, because they share one session. Locking the event while the person thinks is not
possible over HTTP: the server cannot know whether the person walked away. Ignoring the mismatch
saves code and corrupts the evidence the app exists to keep. A lock would be the right choice in
a program that holds a connection open for the whole interaction.

**Read more.** [Rust Book: defining an enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html),
[std: `Option`](https://doc.rust-lang.org/std/option/enum.Option.html).

## 4. `ReportValues`: normalise once, trust everywhere

**The idea.** A smell report says which rooms (each with a level), which symptoms, and an
optional note. The same report can arrive in many shapes: rooms in any order, a symptom twice,
a note with spaces around it. If every part of the app had to cope with every shape, one of them
would forget. The remedy is to clean the data once, in the only place that can create the value,
and to keep the fields private so nobody can break the cleaned shape afterwards.

**In our code.**

- `crates/domain/src/records.rs::ReportValues::new` takes `mut rooms: Vec<RoomLevel>` and
  `mut symptoms: Vec<SymptomId>`. It sorts the rooms, rejects a room listed twice (using
  `windows(2)`), sorts and de-duplicates the symptoms, trims the note and limits its length in
  characters.
- The fields are private. Callers read them through the getters `rooms`, `symptoms` and `note`,
  which return borrowed views (`&[RoomLevel]`, `&[SymptomId]`, `Option<&str>`).
- `crates/domain/src/records.rs::ReportValues::intensity` is the highest room level:
  `iter().map(..).max().unwrap_or(..)`. `max` needs the items to be `Ord`, and `Level` derives it.
- `crates/domain/src/records.rs::ReportValues::is_no_smell` says whether every level is 0.
- `crates/domain/src/records.rs::SmellReport` holds the values with `#[serde(flatten)]`, so the
  JSON snapshot shows the values' fields beside the report's own (#102, #105).
- `crates/storage-sqlite/src/reports.rs::assemble` builds a stored report by calling
  `ReportValues::new` again, so what is read back has the same shape as what was written.

```rust
        let note = note.map(str::trim).filter(|n| !n.is_empty());
```

**Why this way.** `new` takes the vectors by value. The caller gives them up, so `new` may sort
them in place without copying and without surprising the caller. A blank note becomes `None` in
one line: `map` trims the text if there is one, and `filter` drops it when nothing is left. The
length limit uses `chars().count()`, because a Rust `String` stores UTF-8 bytes and `len()` counts
bytes: a byte limit would count each accented letter as two. Counting characters is
close to what a person sees, though not exact for every script.

The audit log hashes each record serialised to JSON, so two equal reports must serialise the
same way. Sorted rooms and symptoms give that. The alternatives are weaker. Normalising in each
caller works until one caller forgets. A `BTreeSet` of rooms would sort for free but would lose
the pairing between a room and its level. A type with public fields would have been the better
choice if the struct only carried data with no rule to protect.

**Read more.** [std: `Option`](https://doc.rust-lang.org/std/option/enum.Option.html),
[Rust Book: storing UTF-8 encoded text with strings](https://doc.rust-lang.org/book/ch08-02-strings.html).

**Try it.** [Exercise 12](../../exercises/src/ex12_report_values.rs): build a `ReportValues`
that cannot exist in a non-canonical shape.

## 5. Errors that refuse: `LogError` and translation keys

**The idea.** When a use case refuses, two different readers need to know why. The program needs
a value it can `match` on. The person needs a sentence in their own language. If the domain
crate wrote English sentences, it would own the user interface text. So the domain returns an
enum, and the web layer maps each variant to a translation key: a short, fixed name such as
`refused-stale` that the translation files turn into text.

**In our code.**

- `crates/domain/src/logbook.rs::LogError` has variants for what the user can fix (`Stale`,
  `AnotherEventOpen`, `OutsideEvent`, …), plus `Invalid(#[from] DomainError)` and
  `Store(#[from] StoreError)`. The `#[from]` attribute (from the `thiserror` crate) writes the
  conversion that lets `?` turn those errors into a `LogError`.
- `crates/web/src/routes/mod.rs::refusal` returns `Result<&'static str, AppError>`. `Ok(key)`
  means "show the form again with this message"; `Err` means "show an error page".
- `crates/web/src/routes/entries.rs::read` returns `Result<Entry, &'static str>`: the form's own
  mistakes are keys too. It is also where the one Log button decides what the form is (below).
- `crates/web/src/routes/entries.rs::back_to_form` renders the Log screen again with status 422
  and what the person typed (#96).

The Log screen has one Log button, not one button per kind of entry (#135). `read` looks at what
is filled in and picks: rooms with a level above 0 make a smell report; rooms all at 0 make a
clean check, with any PM2.5 value saved beside it; no rooms and only a PM2.5 value make a
reading (`Entry::Pm25Only`); symptoms or a note without a room give `log-error-rooms`; an empty
form gives `log-error-empty`. The "No smell" button keeps its own `action` value. The decision
is a `match` with guards on the PM2.5 value and on what was picked:

```rust
        match pm25 {
            Some(value) if !described => return Ok(Entry::Pm25Only { observed_at, value }),
            _ if !picked.is_empty() => {}
            _ if described => return Err("log-error-rooms"),
            _ => return Err("log-error-empty"),
        }
```

Arms are tried in order, and a guard (`if ...`) must hold for its arm to count, so the order is
part of the rule. The test
`crates/web/src/routes/entries.rs::the_log_button_decides_from_what_is_filled_in` walks the
table row by row.

**Why this way.** A `&'static str` is a string that lives for the whole program, such as a
literal in the source. Passing one costs nothing. A key can still be wrong at runtime, so tests
list every key: `crates/web/src/routes/mod.rs::REFUSALS` and `FORM_ERRORS` in
`crates/web/src/routes/entries.rs`, and the translation tests check that each key exists in all
three languages. `refusal` returns a `Result` and not a bare key because the two outcomes differ
in kind: a form message is the user's to fix, while "not allowed" or a database failure is not,
and the type forces each caller to handle both.

An error enum with `Display` text would put English into the domain crate. Plain `String`
messages would compile whatever was written in them, so a typo would only show on a screen. The
`Display` text would be the better choice for a command-line tool with one language.

**Read more.** [Rust Book: recoverable errors with `Result`](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html).

## 6. Forms with repeated names, and borrowed data

**The idea.** A form with several rooms ticked posts `room=1&room=3`: the same name twice. The
usual way to read a form is into a struct, one field per name, and that cannot hold a repeat. So
the app keeps the form as it came: an ordered list of `(name, value)` pairs, and asks it
questions. Answering a question should not copy the text. A method can hand back a reference, a
borrowed view into the pairs. The compiler then has to be sure the reference never outlives the
pairs, and a lifetime is how a function tells it so.

**In our code.**

- `crates/web/src/forms.rs::Pairs` wraps `Vec<(String, String)>` (#98).
- `crates/web/src/forms.rs::Pairs::get` returns `Option<&str>`, borrowed from the pairs.
- `crates/web/src/forms.rs::Pairs::ids` collects into `Option<Vec<i64>>`: the whole result is
  `None` if one value is not a number.
- `crates/web/src/forms.rs::Pairs::names_with_prefix` returns
  `impl Iterator<Item = &'a str> + 'a`.
- `crates/web/src/forms.rs::Pairs::without` returns the pairs that are not named in a skip list.
- `crates/web/src/forms.rs::Pairs::contains` says whether a name is there at all, even blank.
- `crates/web/src/forms.rs::QueryPairs` reads a query string into the same `Pairs`. A repeated
  name takes its first value and a query that does not parse is an empty list; it never answers
  400 (#144).
- `crates/web/src/routes/log.rs::Draft` and `Draft::from_form` build what the form shows again
  after an error.
- `crates/web/src/routes/entries.rs::read_values` and
  `crates/web/src/routes/entries.rs::level_field` read the `level-<room id>` fields.

```rust
    pub(crate) fn names_with_prefix<'a>(
        &'a self,
        prefix: &'a str,
    ) -> impl Iterator<Item = &'a str> + 'a {
```

**Why this way.** A lifetime such as `'a` is a name for "how long some borrowed data stays
valid". In `names_with_prefix`, the signature says: the returned names borrow from `self`, and the
iterator itself also holds `prefix`, so the iterator must not be used after either is gone. The
compiler checks that at every call. Nothing is copied: callers get slices of the original
strings. `prefix` carries `'a` as well because the closure inside the iterator captures it, so
it must stay valid as long as the iterator does.

Returning `String`s would allocate once per call. A `HashMap<String, Vec<String>>` would lose the
order the form arrived in. axum's `Form<Struct>` cannot express repeats at all. The pairs wrapper
costs one small extractor, and its `FromRequest` implementation reuses axum's own `Form` for the
parsing. Owned `String`s would be the better choice if the data had to outlive the request.

`QueryPairs` is a second extractor in the same file. It implements `FromRequestParts`, not
`FromRequest`, because a query string lives in the request's address and needs no body. Its
`Rejection` type is `Infallible`, an error type with no values, which tells the compiler that
extraction cannot fail. A typed `Query<Struct>` would answer 400 for `?month=a&month=b`; the
pages prefer to show their default.

**Read more.** [Rust Book: validating references with lifetimes](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html),
[Rust Book: references and borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html),
[std: `Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html),
[std: `Infallible`](https://doc.rust-lang.org/std/convert/enum.Infallible.html).

**Try it.** [Exercise 13](../../exercises/src/ex13_form_pairs.rs): write a pairs type with
borrowing getters.

## 7. View models, templates and a little CSS

**The idea.** A page has two jobs: deciding what to show and drawing it. If the template language
decides, the decisions are hard to test. So Rust builds a view model, a plain struct holding
exactly what the page needs, and the template only loops over it and prints.

**In our code.**

- `crates/web/src/routes/log.rs::fields` builds `Fields`, `RoomChoice` and `Choice` values with
  `filter`/`map` chains. The closure `room_choices` is defined once and used for both areas
  (inside and common).
- `crates/web/src/routes/log.rs::log_page` reads the open event and the month's reports (keeping
  those `can_see` allows), makes the summary, and builds `LogPage`.
- `crates/web/src/routes/log.rs::banner` builds `Banner`, the open event's start, the number of
  reports the viewer may see and the highest level with its room (#136). The highest room comes
  from `crates/web/src/routes/log.rs::peak`.
- `crates/web/templates/log.html` and `crates/web/templates/report_fields.html` loop and print.
  The only conditions left are about display, such as `{% if form.update %}`.
  Repeated markup is an Askama macro in `crates/web/templates/components.html`, such as `chip`
  and `field`, called with `{% call %}`.
- `crates/web/css/66-room.css` hides a room's level row unless the room is ticked, using the
  selector `.room:has(> .room-check input:checked)` and its `:not(:has(...))` twin (#107).

`peak` shows a useful trick. `max_by_key` takes the largest key, and a tuple compares its
parts in order, so the key `(room.level, when)` means "the highest level, and on a tie the most
recent report". The tie rule is one tuple, not an `if`. The tests
`crates/web/src/routes/log.rs::the_peak_is_the_highest_room_level_of_the_event` and
`crates/web/src/routes/log.rs::on_a_tie_the_most_recently_reported_room_wins` pin it down.

**Why this way.** Rust decides and the template shows, so a unit test can check a decision
without rendering a page, and askama checks the template against the struct when the app is
built (note 03). The CSS `:has()` selector matches an element that contains a matching
descendant, so the browser shows the level row of a ticked room with no JavaScript. Where
`:has()` is unsupported the rows stay visible, which still works.

Logic inside the template language would be hard to test and easy to get wrong. A JavaScript
component for the level rows would need a script and its security-policy allowance. A script
would have been the better choice for behaviour CSS cannot express, such as reacting to data
fetched after the page loads.

**Read more.** [askama on docs.rs](https://docs.rs/askama),
[std: `Iterator::max_by_key`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.max_by_key),
[MDN: `:has()`](https://developer.mozilla.org/en-US/docs/Web/CSS/:has).

## 8. Small language features you will meet

**The idea.** Some Rust features exist only to remove one level of nesting or one temporary
variable. You will see these all over the app.

**In our code.**

- `let … else` in `crates/web/src/assets.rs::respond`: `let Some(index) = ... else { return
  StatusCode::NOT_FOUND.into_response(); };` binds on success and must leave the function on
  failure.
- A let chain joins several conditions, including `if let`, with `&&`. See
  `crates/domain/src/sweep.rs::auto_close` (`if let Some(last) = ... && !schedule...`) and
  `crates/web/src/routes/entries.rs::joinable_events`.
- `Option::is_none_or` in `crates/domain/src/events.rs::can_join_backdated`: true when the
  option is `None` (the event is still open) or the closure returns true.
- `matches!` in `crates/domain/src/permissions.rs::can`: a `match` that returns whether a pattern
  fits.
- `Option::map_or` in `crates/domain/src/logbook.rs::close_at_last_report`: a default for
  `None`, a function for `Some`.

**Why this way.** Each one is the shorter way to say something you could say with a `match`.
`let … else` keeps the happy path unindented. A let chain needs edition 2024 and Rust 1.88.
`is_none_or` arrived in Rust 1.82 and `let … else` in Rust 1.65. The root `Cargo.toml` sets
`rust-version = "1.94"`, so all four are safe here; on an older compiler the let chains would
not build. A full `match` is clearer when each branch does a lot, and these features lose once
the code in the branches grows past a line or two.

**Read more.** [Rust Book: concise control flow with `if let` and `let … else`](https://doc.rust-lang.org/book/ch06-03-if-let.html),
[Rust Reference: `if` expressions](https://doc.rust-lang.org/reference/expressions/if-expr.html),
[std: `Option`](https://doc.rust-lang.org/std/option/enum.Option.html).

## 9. The catch-up sweep, and one transaction for many writes

**The idea.** Events must not stay open across 04:00. The obvious plan is "wake at 04:00 and
close them". It breaks when the host is asleep at 04:00, and free hosting stops idle programs.
A catch-up job asks a different question: "has a 04:00 passed since the last time I ran?" If
yes, do the work now. The answer is the same at 04:00 sharp and at 08:00 after a long sleep.

**In our code.**

- `crates/domain/src/sweep.rs::auto_close` reads the stored last run. If no boundary has passed
  since, it returns `SweepOutcome::NotDue`. Otherwise it begins a transaction, closes the open
  event if it started before the boundary, records the new `last_run_at`, and commits (#34, #88).
  Its bound `Transactional<Tx: EventWrites + JobWrites>` names what the transaction must offer.
- `crates/domain/src/logbook.rs::close_at_last_report` is `pub(crate)`: visible inside the domain
  crate only. The "new event" answer and the sweep share it.
- Callers, for example: `crates/web/src/routes/log.rs::show`, `crates/web/src/routes/entries.rs::submit` and
  `crates/app/src/commands/serve.rs::sweep_events` (at startup and every five minutes, from a
  `tokio::time::interval`).
- Tests: `crates/storage-contract/src/sweep.rs::on_dst_days_the_sweep_still_runs_at_four_local`
  covers the days the clock changes, and the rollback test from section 1 covers "all or nothing".

**Why this way.** The check costs one read when nothing is due, so running it before every Log
screen is cheap. Closing the event and recording the run in one transaction means a crash cannot
leave the event closed with the run unrecorded, or the reverse. A cron job or a scheduler crate
such as `tokio-cron-scheduler` needs the process awake at 04:00 and would be the better choice
on a server that never sleeps. How the spawned timer loop is started is in
[note 03](03-web-and-auth.md), section 11.

**Read more.** [tokio::time::interval](https://docs.rs/tokio/latest/tokio/time/fn.interval.html).

## Check yourself

1. Why does `log_prompt` match on a tuple, and what does the compiler do if a case is missing?
2. What does `EventChoice::New { replacing }` protect against, and why is the same check made in
   the web layer and inside the transaction?
3. Why are the fields of `ReportValues` private, and what would break if `rooms` were `pub`?
4. What does the `'a` in `names_with_prefix<'a>` promise to the caller?
5. Why does `refusal` return `Result<&'static str, AppError>` and not just `&'static str`?
6. What would break if the auto-close ran only at exactly 04:00, and where would you change the
   code to run it at another hour?

## Where next

Next: [05 — People and the PWA](05-people-and-pwa.md) (accounts, invites and the installable app).

Exercises for this note: [Exercise 11](../../exercises/src/ex11_log_prompt.rs) (log prompts),
[Exercise 12](../../exercises/src/ex12_report_values.rs) (report values) and
[Exercise 13](../../exercises/src/ex13_form_pairs.rs) (form pairs).
