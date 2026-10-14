# 06 — Graphs

> Written against commit `eda7a09` (milestone 7b, look and feel). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 16 (buckets), 17 (series). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn

- How a function that takes plain data and returns plain data is easy to test, and why the arithmetic lives there.
- How `std::iter::successors` with `take_while` builds a list of days or weeks without a loop counter.
- How a `BTreeMap` groups items by key and keeps them in key order, and what `entry`, `or_default` and `and_modify` do.
- Why `None` is less than every `Some`, and how that makes "the highest level so far" a one-liner.
- Why `f64::from(x)` and `bool::then` say more than `as f64` and an `if`.
- Why filtering the input first is safer than filtering the output, and how a pure `Filter` in the domain crate does the narrowing.
- How JSON in an HTML attribute, a vendored script and a strict Content-Security-Policy fit together.
- How `include_str!` lets a test read a file at compile time, and how `Drop` cleans up a test database.
- How one module becomes a directory of files without changing what the rest of the program sees.

## The map

This note covers the Graphs screen: the filters in the address, the numbers behind each chart, how they reach the browser, and
three smaller things around it (the README test, CI, and the test databases).

- `crates/domain/src/graphs.rs`: the pure functions, the `Period` type and the `Filter` that narrows the reports.
- `crates/domain/src/schedule.rs::DailySchedule`: local days and the bounds of a month or a year.
- `crates/web/src/routes/graphs/mod.rs`: fetches the reports, filters them, builds the page.
- `crates/web/src/routes/graphs/filters.rs`: reads the filters from the address and builds the Filters card.
- `crates/web/src/routes/graphs/charts.rs`: the Chart.js configuration of each chart and the heatmap rows.
- `crates/web/templates/graphs.html`: the page; one `<canvas>` per chart and the heatmap table.
- `crates/web/static/js/graphs.js`: draws each chart from its attribute; `crates/web/static/js/series-switch.js` and `crates/web/static/js/person-filter.js` are the two small enhancements around it.
- `crates/web/src/assets.rs`: the files compiled into the binary, with the hash test for the libraries.
- `crates/web/src/routes/guide.rs`: the guide texts and the test that compares them with the README.
- `.github/workflows/codeql.yml` and `.github/workflows/ci.yml`: security scanning and coverage.
- `crates/storage-postgres/tests/common/mod.rs`: the test database that drops itself.

## 1. Pure functions from reports to numbers

**The idea.** A graph is a summary: how many events per day, which room is smelly at which hour.
The risky part is the arithmetic. Which day does a report at 00:30 count on? How is a cell
averaged? If that arithmetic is mixed with database calls and HTML, each test needs a database
and a page. A pure function is a function whose result depends only on its arguments and which
does nothing else (no reading, no writing). Pure functions need only values to test, so there
can be many tests, and they run in microseconds.

**In our code.**

- `crates/domain/src/graphs.rs::intensity_lines`, `crates/domain/src/graphs.rs::events_per_bucket`,
  `crates/domain/src/graphs.rs::room_hours`, `crates/domain/src/graphs.rs::symptom_counts` and
  `crates/domain/src/graphs.rs::area_levels` each take a slice of `SmellReport` and return plain
  data. Read the comment at the top of the file: the caller has already limited the reports to one
  period and to what the viewer may see (#123).
- `crates/web/src/routes/graphs/mod.rs::show` is the other half. It fetches the reports for the
  period, filters them, calls the domain functions and hands the results to the page. The rules about
  days, weeks and counting stay in the domain crate; the route keeps only presentation choices,
  such as rounding an average for a colour class and picking the axis ticks.

**Why this way.** The functions are pure so that the tests at the bottom of
`crates/domain/src/graphs.rs` can cover time zones, empty buckets and clean checks with a few
lines each. Alternatives: computing in SQL with `GROUP BY` lost because the app has two
database dialects, "local day" and "Monday week" are awkward in SQL, and a guest filter would
have to be repeated there. Computing in the browser lost because the rules would then be
JavaScript that `cargo test` never runs. SQL would be the better choice for a very large table
where moving every row into memory is too slow; here a month of reports for one household is small.

**Read more.** [The Rust Book: processing a series of items with iterators](https://doc.rust-lang.org/book/ch13-02-iterators.html),
[std: slices](https://doc.rust-lang.org/std/primitive.slice.html).

## 2. Periods and buckets

**The idea.** A bar chart needs one bar per day (or week), including the days with nothing in
them, or the chart would skip empty days and mislead. A *bucket* is one bar's slot, named by its first day: the day itself in a
daily chart, the Monday of the week in a weekly one; a *period* is the stretch the chart covers. The app builds the list of
buckets first and fills it afterwards, so a missing zero cannot happen.

**In our code.**

- `crates/domain/src/graphs.rs::Period` is a struct with a first and a last day, both private.
  `Period::new` returns `Option<Self>` and refuses a period that ends before it starts or is
  longer than two years (`Period::MAX_DAYS`). `Period::month`, `Period::year` and
  `Period::last_days` are the other ways to make one, so no caller can build a bad period
  (the same idea as `ReportValues` in note 04).
- `crates/domain/src/graphs.rs::Period::granularity` returns the enum
  `crates/domain/src/graphs.rs::Granularity`: `Day` for up to 62 days
  (`Period::DAILY_UP_TO_DAYS`), `Week` beyond. `Period::span` gives the UTC bounds, and
  `crates/domain/src/schedule.rs::DailySchedule::year_span` is still used for the year's ticks.
- `crates/domain/src/graphs.rs::Period::buckets` makes the list with `std::iter::successors`:
  "the first bucket, then one step further each time", stopped by `take_while` at the end.

  ```rust
  std::iter::successors(Some(self.bucket_of(self.first)), |&day| Some(day + step))
      .take_while(|&day| day <= self.last)
      .collect()
  ```

- `crates/domain/src/graphs.rs::Period::bucket_of` names a week by its Monday, using
  `num_days_from_monday`. That is why `buckets` starts from `bucket_of(first)` and not from
  `first`: the first week of 2026 starts on Monday 29 December 2025.
- `crates/domain/src/graphs.rs::zeroed` is a generic helper, `fn zeroed<T: Copy>(...)`. It fills
  every bucket with one starting value, a `0` for events or an empty `AreaLevels` for areas.
- `crates/domain/src/graphs.rs::events_per_bucket` keeps the earliest report of each event with
  `BTreeMap::entry(..).and_modify(..).or_insert(..)`: if the event is there already, keep the
  smaller time; if not, insert it. An event counts once, as in the summary card (#94, restated in #123).

**Why this way.** A graph that starts at the first bar of the year is wrong when the year
does not start on a Monday, so the first bucket may start in the year before. Alternatives
recorded in decision #123: a rolling 30/90/365 days, and months as a year's buckets. Milestone 7b
(#137) replaced the two fixed kinds of period with any stretch of days: the granularity follows
the length, so a long stretch needs coarser buckets, hence a count per week. Every period is
at most two years, which bounds every query.

**Read more.** [std: the `successors` function](https://doc.rust-lang.org/std/iter/fn.successors.html),
[std: `BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html),
[the `chrono` crate](https://docs.rs/chrono).

**Try it.** [Exercise 16](../../exercises/src/ex16_buckets.rs): build the buckets of a period and
count events per bucket.

## 3. From reports to series: grouping with `BTreeMap`

**The idea.** Grouping means turning a flat list ("report, report, report") into "for each key, its
items". In Rust the usual tool is a map. `BTreeMap<K, V>` keeps its keys sorted, so walking it
gives the same order every time. The *entry API* looks at a key once and lets you say what to do
whether it is there or not.

**In our code.**

- `crates/domain/src/graphs.rs::intensity_lines` builds a `BTreeMap<UserId, Vec<_>>`. For each
  report, `entry(..).or_default().push(..)` finds the person's list (creating an empty one the
  first time) and adds a point. Then `sort_by_key` puts each list in time order and `map` and
  `collect` turn the map into a `Vec<PersonLine>`.
- `crates/domain/src/graphs.rs::room_hours` has a `BTreeMap<RoomId, [LevelTally; 24]>`. Arrays up
  to 32 elements implement `Default` when the element type does, so `or_default()` creates a whole
  row of 24 empty tallies. `crates/domain/src/graphs.rs::LevelTally` derives `Default` for this.
- `crates/domain/src/graphs.rs::symptom_counts` sorts the counted symptoms with
  `sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)))`: most frequent first, ties broken by id.
  Swapping `a` and `b` in the first comparison is what reverses the order.

**Why this way.** A `BTreeMap` iterates in key order, so the output is stable without a sort and
the tests compare whole vectors. Alternatives: a `HashMap` lost because its order changes between
runs, so every function would need a sort to be deterministic. An external
grouping helper lost because it would be a new dependency for a loop of five lines. A `HashMap` would be the better choice when the
keys have no order or when there are very many of them and order does not matter.

**Read more.** [std: `BTreeMap`](https://doc.rust-lang.org/std/collections/struct.BTreeMap.html),
[std: the `Default` trait](https://doc.rust-lang.org/std/default/trait.Default.html),
[std: the `Ord` trait](https://doc.rust-lang.org/std/cmp/trait.Ord.html).

**Try it.** [Exercise 17](../../exercises/src/ex17_series.rs): group reports into per-person
series and rank symptoms.

## 4. `Option` ordering, `bool::then` and exact conversions

**The idea.** Three small tools that make code say what it means. `Option<T>` is ordered when `T`
is: `None` is less than every `Some`. A `bool` can become an `Option` with `then`: `Some(value)`
when true, `None` when false. And a number can be converted with `From` only where nothing can
be lost, while `as` always compiles and may cut digits silently.

**In our code.**

- `crates/domain/src/graphs.rs::area_levels` has `*highest = (*highest).max(Some(room.level))`.
  The slot starts as `None`, and `None` loses to any level, so "nothing yet" is replaced by the
  first level and after that by the larger one. `Level` derives `Ord`, which is what lets
  `Option<Level>` be compared.
- `crates/domain/src/graphs.rs::LevelTally::mean` returns `None` when nobody checked the room:
  `(self.checks > 0).then(|| f64::from(self.total) / f64::from(self.checks))`. The closure runs
  only when the condition is true, so with no checks the division is never evaluated. Dividing floats by zero would not panic
  anyway, it would give `NaN`; the point of `None` is that the page can show "no data" instead.
- `u32::from(room.level.get())` in `crates/domain/src/graphs.rs::room_hours` widens a `u8` to a
  `u32`. If the type of the total changed to something smaller, this line would stop compiling.
- `crates/web/src/routes/graphs/charts.rs::heat_rows` does use `as u8` on a rounded average. It is
  deliberate: the value is between 0 and 10 and is only for a colour class. Where a cast is safe
  by construction and for display, `as` is acceptable.

**Why this way.** `From` exists only for lossless conversions, so using it makes the compiler check
the claim "this cannot lose anything". Alternatives: `as` everywhere lost for data because a wrong
type would compile and produce a wrong number. `try_from` is the right tool where the conversion
can fail, for example `u8::try_from(x)` for an arbitrary integer.

**Read more.** [std: the `Option` enum](https://doc.rust-lang.org/std/option/enum.Option.html),
[std: the `From` trait](https://doc.rust-lang.org/std/convert/trait.From.html).

**Try it.** [Exercise 17](../../exercises/src/ex17_series.rs) also asks for a highest level that
starts as `None`.

## 5. Security by construction: filter first, then count

**The idea.** Guests may see only their own entries and graphs (#11). The easy mistake is to count
everything and hide the other people's data afterwards: a total or an average would still include
what the guest must not see. The safe design removes what the viewer may not see before any
number is made. Then the counting functions cannot leak, because they never receive the data.

**In our code.**

- `crates/web/src/routes/graphs/mod.rs::show` passes each report through
  `smoke_domain::permissions::can_see` right after fetching it, before `events_per_bucket` or any
  other function runs. The same filter is applied to the PM2.5 readings. Only then do the
  chosen filters narrow the result further (below).
- `crates/domain/src/permissions.rs::can_see` is the single rule that says who sees whose data.
- The test `crates/web/tests/graphs.rs::guests_see_graphs_of_their_own_reports_only` checks the
  page of a guest: only the guest's own line, event count and heatmap cell appear, and the
  names of the other people appear nowhere.
- The person filter follows the same rule. `crates/web/src/routes/graphs/mod.rs::show` gives
  `crates/web/src/routes/graphs/filters.rs::read` the list of people only to those who see
  everyone; for a guest the list is empty, so a typed `?by=` is ignored. The test
  `crates/web/tests/graphs.rs::guests_get_no_person_filter_and_cannot_pick_someone_else` checks it.

**The filters are pure too.** The chosen filters (rooms or area, person, part of the day, minimum
level, hide clean checks) live in `crates/domain/src/graphs.rs::Filter`, a plain struct whose
default keeps everything. `crates/domain/src/graphs.rs::Filter::reports` takes the reports by
value and returns the ones to count; `crates/domain/src/graphs.rs::Filter::readings` does the
same for PM2.5 readings. A room or area filter keeps only the chosen rooms of each report and
drops a report left with none (#137). Two idioms carry it:

```rust
        self.person.is_none_or(|person| person == user)
            && self.day_part.is_none_or(|part| part.contains(hour))
```

`Option::is_none_or` reads "no filter chosen, or it matches", so an unset filter lets
everything through. And `filter_map` filters and changes in one step: the closure returns
`Some(report)` to keep it (possibly narrowed) or `None` to drop it. The part of the day is an
enum, `crates/domain/src/graphs.rs::DayPart`, whose `contains` handles the night, which crosses
midnight. Because all this is in the domain crate, its tests need no page and no database, and
the web layer only turns the address into a `Filter`: `crates/web/src/routes/graphs/filters.rs::read`
reads each name from `Pairs` and falls back to the default when a value is missing or unusable.

**Why this way.** Filtering at the edge makes the pure functions safe by their input. Alternatives:
passing the viewer into every function lost because each of them would have to remember to check,
and one forgotten check is a leak. Filtering in SQL lost because it would be a second copy of the
rule, in a second dialect. Passing the viewer would be the better choice if different functions
needed different parts of what the viewer may see.

**Read more.** [std: `Iterator::filter`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter),
[std: `Iterator::filter_map`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#method.filter_map),
[std: `Option::is_none_or`](https://doc.rust-lang.org/std/option/enum.Option.html#method.is_none_or).

## 6. From numbers to a chart: JSON in a data attribute, a vendored library and the CSP

**The idea.** A browser draws the charts with a JavaScript library (Chart.js). The server must hand
it the numbers. The page also carries a *Content-Security-Policy* (CSP), a response header in which
the app tells the browser which scripts it may run: here only scripts from the app's own address.
That rules out an inline *executable* `<script>` with the numbers in it. The numbers travel as
JSON inside an attribute, which is inert text that the page's own script reads. (A
`<script type="application/json">` block is not executed, so it would also pass; see below.)

**In our code.**

- `crates/web/src/routes/graphs/charts.rs::intensity_chart`, `crates/web/src/routes/graphs/charts.rs::count_chart`,
  `crates/web/src/routes/graphs/charts.rs::symptom_chart` and `crates/web/src/routes/graphs/charts.rs::area_chart`
  build a `serde_json::Value` with the `json!` macro: the Chart.js configuration, written like
  the JSON it becomes.
- `crates/web/templates/graphs.html` writes each one into a `data-chart` attribute. Askama escapes
  the quotes, so the JSON cannot break out of the attribute.
- `crates/web/static/js/graphs.js` reads each attribute with `JSON.parse` and draws it. It adds only
  what JSON cannot carry: time-axis labels made with `Intl.DateTimeFormat` in the app's time
  zone, not the browser's (#95), and the colours.
- Colours travel as token names. A dataset in the JSON says `"token": "accent"` and the script
  reads that token's value from the stylesheet again when the system switches between light and
  dark (#138). See `crates/web/src/routes/graphs/charts.rs::PERSON_TOKENS`.
- The intensity chart has an Everyone line next to one line per person, and each dataset names
  its `group`. The switches above the chart (By person or Everyone; Both, Intensity or PM2.5)
  are in `crates/web/static/js/series-switch.js`, which shows or hides datasets. They are hidden
  without JavaScript, so the page still shows one line per person. The Everyone line is built
  in `crates/web/src/routes/graphs/charts.rs::intensity_chart` by merging every person's points
  and sorting them by time, and only when no person is chosen.
- `crates/web/static/chart-4.5.1.umd.min.js` is a vendored copy of Chart.js. The test
  `crates/web/src/assets.rs::the_vendored_libraries_are_the_published_releases` checks its SHA-256
  hash (#124).
- `crates/web/src/routes/graphs/filters.rs::read` reads the period and every filter from the
  address. The quick picks are `?range=`, typed dates are `?from=` and `?to=`, and typed dates
  win. A hand-typed bad value falls back to its default with status 200, and a repeated name
  takes its first value (#144).
  `crates/web/src/routes/graphs/charts.rs::intl_locale` picks the language tag for the date labels.

**Why this way.** The CSP allows only same-origin scripts, so inline executable code is blocked;
the numbers must be inert data. Two forms are: an attribute, or a non-executed
`<script type="application/json">` block. Decision #124 chose the attribute and rejected the
block; Askama's escaping already makes an attribute safe, and one `data-chart` per canvas keeps
each configuration next to the chart it belongs to. The block would be the better choice for
very large payloads, where an attribute becomes unwieldy. The CSP also stays `'self'` with no
exception because Chart.js 4 sets styles through the CSSOM, which the policy allows, so no
`style-src` relaxation is needed. Vendoring means a copy committed to the repository and checked
by hash: no CDN, no third-party request, and a build that never changes by itself. Alternatives: a
JSON endpoint per chart lost because it means more requests and more routes to protect. SVG
drawn by the server lost because it has no interaction and takes a lot of drawing code. A
Rust-to-WASM chart lost because it needs a whole extra toolchain for little gain. A CDN
would be the better choice for an app with many pages using the same library and no privacy goal.

**Read more.** [serde_json](https://docs.rs/serde_json), [Chart.js documentation](https://www.chartjs.org/docs/latest/),
[MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP),
[MDN: `Intl.DateTimeFormat`](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Intl/DateTimeFormat).

## 7. A heatmap as an HTML table

**The idea.** A heatmap is a grid where the colour of each cell shows a number. It does not need a
drawing library: an HTML table with one coloured cell per value is a heatmap.

**In our code.**

- `crates/web/src/routes/graphs/charts.rs::HeatRow` is one row: a room name and 24 cells.
  `crates/web/src/routes/graphs/charts.rs::heat_rows` fills it: one row per room a report named, each
  cell the rounded average level at that hour (`None` where nobody checked), and the template
  gives each cell a class from `heat-0` to `heat-10`.
- `crates/web/css/73-chart.css` defines `.heat-0` to `.heat-10`, each reading a `--heat-N` colour
  token from `crates/web/css/20-tokens.css`, so light and dark mode each have their own steps.
  See decision #126.

**Why this way.** A table works without JavaScript, reads correctly with a screen reader (the room
is a row header, the hour a column header), and a CSS class keeps the CSP strict because the
policy forbids `style=` attributes. Alternatives: a Chart.js matrix plugin lost because it is a
second library to vendor and check. A `<canvas>` lost because a screen reader cannot read it.
This is also why the heatmap cell is an average and the area chart's bar is a maximum: the
heatmap answers "how smelly is this room usually at this hour", the area chart follows the rule
of a report's intensity, its highest room level.

**Read more.** [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP).

## 8. Documentation that a test keeps honest

**The idea.** The spec asks for the same guide tables in the app and in the README. Two copies
drift apart unless something checks them. A test can do that checking: it reads one copy and
asserts that the other contains each row.

**In our code.**

- `crates/web/src/routes/guide.rs::levels`, `crates/web/src/routes/guide.rs::situations` and
  `crates/web/src/routes/guide.rs::rules` build the guide rows from the translation files.
- `crates/web/src/routes/guide.rs::the_readme_tables_match_the_english_guides` reads `README.md`
  with `include_str!`, builds each expected row such as `| 3 | name | description |` and fails with
  "README lacks ..." when a row is missing (#130).
- `crates/web/tests/guides.rs` checks the pages themselves.

**Why this way.** `include_str!` puts the file's text into the program while it is compiled, so a
missing README is a compile error, not a test that quietly passes. Alternatives: generating the
README from the translations lost because it is a build step to maintain. Linking to the spec
instead of copying lost because the README would not stand alone for a visitor. A generator would
be the better choice with many tables or many languages.

**Read more.** [std: the `include_str!` macro](https://doc.rust-lang.org/std/macro.include_str.html),
[The Rust Book: writing tests](https://doc.rust-lang.org/book/ch11-01-writing-tests.html).

## 9. CI additions: CodeQL and coverage

**The idea.** Two checks run on every change in GitHub. CodeQL reads the code for known kinds of
security mistakes. Coverage records which lines the tests ran, so you can see which code has no
test. Coverage is a map, not a score.

**In our code.**

- `.github/workflows/codeql.yml` runs a matrix over `rust`, `actions` and `javascript-typescript`
  with `build-mode: none`, so it reads the source without building. The vendored `*.min.js` files
  are ignored because they are not our code (#128).
- `.github/workflows/ci.yml` has a `coverage` job: `cargo llvm-cov --workspace --locked
  --no-report` runs the tests on SQLite and a Postgres service, a summary goes into the job
  page, and `lcov.info` is kept for 14 days. The `taiki-e/install-action` step is pinned to a
  commit, not a tag (#129).

**Why this way.** There is no threshold, so nobody writes a test only to move a number. A commit
hash cannot be moved, but a tag can, so pinning protects against a third-party action changing
under us. Alternatives: Codecov lost because it needs an account and a token. `cargo-tarpaulin`
lost because llvm's instrumentation is more exact on stable Rust. A threshold would be the better
choice in a team that wants a rule a reviewer can point to.

**Read more.** [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov).

## 10. Test databases that clean up after themselves

**The idea.** Each Postgres test gets a fresh database, so tests cannot disturb each other. Without
cleanup, every run leaves its databases behind. The tool is `Drop`: a method Rust calls when a value
goes out of scope. This is the same idea as the transaction that rolls back in note 02, section 4:
cleanup tied to the life of a value.

**In our code.**

- `crates/storage-postgres/tests/common/mod.rs::TestDatabase` implements `Drop`. When the test
  ends it runs `DROP DATABASE ... WITH (FORCE)`. The work runs on a new thread with its own runtime,
  because the test's runtime may be the one dropping the value and cannot wait on itself. A failure
  is printed, never a panic inside `drop`.
- `crates/storage-postgres/tests/common/mod.rs::fresh_database` makes the name unique from the
  process id, a timestamp and an `AtomicU32` counter, so tests running at the same time never
  pick the same name.

**Why this way.** Cleanup that depends on a person remembering fails; cleanup tied to a value does
not. Alternatives: a cleanup script lost because it gets forgotten. One shared database with
each test rolled back lost because these tests need to see committed data. A shared database
would be the better choice if setup were slow and the tests never committed.

**Read more.** [The Rust Book: the `Drop` trait](https://doc.rust-lang.org/book/ch15-03-drop.html),
[std: atomics](https://doc.rust-lang.org/std/sync/atomic/index.html).

## 11. One module, three files

**The idea.** In milestone 6 the whole Graphs route was one file. Milestone 7b added the filters,
so the file would have grown past what is pleasant to read. Rust lets a module turn into a
directory without changing its name for anyone outside: `routes/graphs.rs` became
`routes/graphs/mod.rs` plus two files, and `routes::graphs::show` is still the path the router uses.

**In our code.**

- `crates/web/src/routes/graphs/mod.rs` declares `mod charts;` and `mod filters;`. They have no
  `pub`, so they are private to the `graphs` module.
- `crates/web/src/routes/graphs/mod.rs` brings names in with `use self::charts::{...}` and
  `use self::filters::FiltersView`. `self::` means "this module's own child".
- Items that the parent needs are `pub(crate)`, for example `charts::heat_rows`. Items that only
  the child needs, such as `charts::tick_values` and `filters::range_period`, are private.
- `crates/web/src/routes/graphs/mod.rs::keys` collects the translation keys of the whole
  directory for the translation tests, by calling `filters::keys`.

**Why this way.** `filters.rs` knows about the address and the Filters card, `charts.rs` knows
Chart.js and the heatmap, and `mod.rs` only wires them to the store and the template. Each file
has one reason to change. A private module also stops the rest of the app from reaching into
the chart code by accident. Alternatives: keep one long file, which is easy at first and hard to
read once it keeps growing; or make three sibling modules under `routes`, which would make the filters
and charts visible to every route. One file would be the better choice for a module that
stays small.

**Read more.** [Rust Book: separating modules into different files](https://doc.rust-lang.org/book/ch07-05-separating-modules-into-different-files.html),
[Rust Book: paths for referring to an item in the module tree](https://doc.rust-lang.org/book/ch07-03-paths-for-referring-to-an-item-in-the-module-tree.html).

## Check yourself

1. Why does `buckets` start from `bucket_of(first_day)` and not from `first_day`, and what would the
   first bar of 2026 show if it did not?
2. Why is a heatmap cell an average, and the area chart's value a maximum?
3. What does the ordering of `Option<Level>` do in `area_levels`, and what would break if the slot
   started as `Some(Level)` of zero instead of `None`?
4. Where exactly is a guest kept from seeing other people's data in the graphs, and what would
   break if that moved after the counting?
5. Why can the chart configuration not be an inline executable `<script>`, which two inert forms
   would pass the CSP, and why did the app pick the attribute?
6. What does `the_readme_tables_match_the_english_guides` fail on, and why can it read `README.md`
   with `include_str!`?
7. Why does `Filter::reports` take the reports by value, and what does `is_none_or` do for a
   filter that is not set?
8. What does the `mod charts;` line in `graphs/mod.rs` make private, and how does `show` still
   use `heat_rows`?

## Where next

Next: there is no later note yet; to revisit the start, read [note 01](01-foundation-and-domain.md).
Exercises for this note: [Exercise 16](../../exercises/src/ex16_buckets.rs) (buckets) and
[Exercise 17](../../exercises/src/ex17_series.rs) (series).
