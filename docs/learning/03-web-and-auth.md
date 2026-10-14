# 03 — Web and auth

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 8 (a login limiter), 9 (request rules), 10 (middleware and error chains). Reading time: about
> 60 minutes, plus the exercises.

## What you will learn

- How an async function becomes a web handler, and how its arguments say what it needs.
- How to write your own extractor, and what a rejection type decides.
- How a middleware is a closure that wraps the rest of the request, and in which order they run.
- How `Arc`, `Mutex` and the `HashMap` entry API share and change data across threads.
- How slow work leaves the async executor, and how errors and panics become pages.
- How templates and translations are checked when the app is built.
- How pure functions keep rules testable, and how tests drive the router.

## The map

This note covers the web layer: everything between an HTTP request and the domain functions of
note 01 and the storage traits of note 02. It also covers the binary that starts the server and
the image it ships in.

- `crates/web/src/lib.rs`: `router` builds the whole app, `serve` runs it.
- `crates/web/src/auth/`: sessions, the sign-in limiter, password hashing, the client address.
- `crates/web/src/error.rs` and `crates/web/src/security.rs`: error pages, headers, the CSRF check.
- `crates/web/src/i18n.rs` and `crates/web/templates/`: translations and templates.
- `crates/app/src/`: the command line, the configuration and the background tasks.
- `crates/web/tests/`: integration tests that call the router directly.
- `Dockerfile`: the small container image.

## 1. Axum: routes, handlers and extractors

**The idea.** A web server must send each request to the code that knows how to answer it. A
router maps a path and a method to a handler. In axum a handler is an `async fn` whose
arguments are extractors: types that pull one piece (the state, a cookie, the query string)
out of the request. If an extractor cannot do its job, axum answers the request itself and the
function body never runs, so a handler only sees values that already passed their checks.

**In our code.**

- `crates/web/src/lib.rs::router` builds a `Router` from `.route(path, get(handler))` calls,
  groups them with `.merge`, adds middleware with `.route_layer` and `.layer`, sets a
  `.fallback` and finishes with `.with_state`.
- `crates/web/src/routes/log.rs::show` takes `State<AppState<S>>`, `CurrentUser`, `Tr` and
  `Query<ShowQuery>`, and returns `Result<Response, AppError>`.
- `crates/web/src/routes/entries.rs::submit` takes the body-reading extractor `Pairs` as its
  last argument: a request body can be read once, so only the last extractor may consume it.
- Every handler is generic: `S: Store + Clone + 'static`. `Clone` because axum clones the
  state for each request; `'static` because tokio may move the handler's future to another
  thread and run it later, so it may not borrow local data.
- `crates/web/src/lib.rs::serve` runs `axum::serve` and finishes requests in flight on shutdown.

**Why this way.** A handler's signature shows what it needs without reading the body.
Alternatives: one large `match` on the path (what a router replaces); macros on handlers
(Rocket) or an actor model (Actix). Axum won (decision #2) because it is built on tokio, tower
and hyper, which the rest of the Rust web ecosystem shares.

**Read more.**

- [axum documentation](https://docs.rs/axum/latest/axum/) and
  [extractors](https://docs.rs/axum/latest/axum/extract/index.html)
- [axum: the `Router` type](https://docs.rs/axum/latest/axum/struct.Router.html)
- [Tokio tutorial](https://tokio.rs/tokio/tutorial)

## 2. Writing your own extractor

**The idea.** An extractor is any type that implements `FromRequestParts` (reads the headers
and other metadata) or `FromRequest` (may also consume the body). The impl has an associated
type, `Rejection`, that says what the failure looks like: a status, a redirect, or
`Infallible` (a type with no values) when it cannot fail.

**In our code.**

- `crates/web/src/auth/mod.rs::CurrentUser` has `type Rejection = Redirect`, so a signed-out
  visitor is sent to `/login`; `OptionalFromRequestParts` lets a handler ask for an `Option`.
- `crates/web/src/i18n.rs::Tr` has `type Rejection = Infallible`: it never fails, English is
  the default.
- `crates/web/src/auth/client_ip.rs::ClientIp` is implemented for the concrete
  `AppState<S>`, not for any state, because it reads the configuration from it.
- `crates/web/src/forms.rs::Pairs` implements `FromRequest`, the variant that may consume the
  body, by wrapping `Form<Vec<(String, String)>>` (decision #98).

**Why this way.** A small impl makes an argument list read like a sentence, and the rejection
type decides in one place what a failure looks like. Alternatives: read `request.extensions()`
by hand in every handler (repetition); the built-in `Extension<User>` (a missing user becomes
a plain 500 error, not a redirect to the login page).

**Read more.**

- [axum: `FromRequestParts`](https://docs.rs/axum/latest/axum/extract/trait.FromRequestParts.html)
- [std: `Infallible`](https://doc.rust-lang.org/std/convert/enum.Infallible.html)

## 3. Middleware: closures that wrap a handler

**The idea.** Some rules apply to many routes: sign-in, security headers, the cross-site check.
Middleware is a function that receives the request and a handle to "the rest of the chain",
may act before it, calls it, and may change the response on the way back. In axum,
`middleware::from_fn` turns an `async fn(Request, Next) -> Response` into such a layer.

**In our code.**

- `crates/web/src/auth/mod.rs::load_user` loads the session's user, signs out an expired
  viewer, renews the session at most daily and picks the language.
- `crates/web/src/auth/mod.rs::require_user` redirects to the login page;
  `crates/web/src/security.rs::headers` adds the security headers to every response;
  `crates/web/src/security.rs::check_origin` is the CSRF check.
- `crates/web/src/error.rs::render_errors`, the `CatchPanicLayer`, `TraceLayer` and request-id
  layers from `tower-http` are wired up in `crates/web/src/lib.rs::router`;
  `crates/web/src/request_id.rs::RandomRequestId` implements tower-http's `MakeRequestId`.
- The layer added last is the outermost. So a page request meets `render_errors`, then
  `check_origin`, then the session layer, then `load_user`, then `CatchPanicLayer`, then the
  handler; the response travels back out in reverse.

**Why this way.** A rule in one place cannot be forgotten in one handler. The order is
deliberate: `CatchPanicLayer` sits inside, so a panic becomes a response that `render_errors`
can still dress as a page. Alternatives: check inside each handler (one missing check is a
hole); implement `tower::Service` by hand (more code for the same effect; it is what to learn
next for reusable middleware).

**Read more.**

- [axum: middleware](https://docs.rs/axum/latest/axum/middleware/index.html)
- [tower](https://docs.rs/tower), [tower-http](https://docs.rs/tower-http) and
  [The Rust Book: closures](https://doc.rust-lang.org/book/ch13-01-closures.html)

**Try it.** [Exercise 10](../../exercises/src/ex10_middleware.rs).

## 4. Shared state: `Arc`, `Mutex` and the `HashMap` entry API

**The idea.** Requests run at the same time on several threads, so data they share and change
needs protection. `Arc` (atomic reference count) gives several owners one value across
threads. `Mutex` lets one thread at a time change it: `lock` returns a guard, and dropping the
guard releases the lock. `HashMap::entry` looks a key up once and lets you insert or change it.

**In our code.**

- `crates/web/src/state.rs::AppState` is a struct of `Arc`s (the clock, the configuration, the
  limiter, a semaphore) around the store. Its `Clone` is written by hand and only clones the
  `Arc`s, which is cheap.
- `crates/web/src/auth/limiter.rs::LoginLimiter` keeps a `Mutex<Tables>`: both counters sit
  behind one lock, so "check, then count" is one step (decision #60).
- `crates/web/src/auth/limiter.rs::strike` uses `map.entry(key).or_insert_with(...)` and then
  changes the value through the `&mut` it returns.
- `crates/web/src/auth/limiter.rs::is_locked` is generic over `K: Borrow<Q>`, so a
  `HashMap<String, _>` can be searched with a `&str`; `crates/web/src/auth/limiter.rs::address_key`
  counts IPv6 clients per /64 (decision #75).
- `crates/web/src/auth/limiter.rs::LoginLimiter::lock` recovers from a poisoned mutex with
  `PoisonError::into_inner`. A mutex is poisoned when a thread panicked while holding it.
  Recovering is right here: the counters stay valid numbers whatever happened.

**Why this way.** The standard `Mutex` (not tokio's) is right because the lock is never held
across an `.await`. Alternatives: a database table (survives restarts, but one more write per
sign-in; production runs one instance, decision #74); an `RwLock` (reads are not the common
case here); a channel to an owner task (more code for a few counters).

**Read more.**

- [The Rust Book: shared-state concurrency](https://doc.rust-lang.org/book/ch16-03-shared-state.html)
- [std: `Mutex`](https://doc.rust-lang.org/std/sync/struct.Mutex.html),
  [`Arc`](https://doc.rust-lang.org/std/sync/struct.Arc.html) and
  [`Entry`](https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html)
- [The Rust Book: hash maps](https://doc.rust-lang.org/book/ch08-03-hash-maps.html)

**Try it.** [Exercise 8](../../exercises/src/ex08_login_limiter.rs).

## 5. CPU-heavy work off the async executor: Argon2

**The idea.** Async tasks share a few worker threads and must give them back quickly, at each
`.await`. A long computation never awaits, so it would stop that thread from serving anyone
else. Tokio offers `spawn_blocking`, which runs a closure on a separate pool of threads.
Password hashing is the example: it is slow on purpose, so that guessing is expensive.

**In our code.**

- `crates/web/src/auth/password.rs::hash` and `crates/web/src/auth/password.rs::verify` use
  Argon2id with its default cost (decision #69). `verify` always does one run, with a dummy
  hash when the user does not exist, so timing does not reveal which accounts exist.
- `crates/web/src/auth/password.rs::run_limited` waits for a semaphore permit and then calls
  `spawn_blocking`; the permit moves into the closure, so it is held until the work ends.
- `crates/web/src/auth/password.rs::CONCURRENT_HASHES` is 2: each run needs about 19 MiB and
  the instance has 512 MiB.
- `crates/web/src/auth/password.rs::DUMMY_HASH` is a `LazyLock` that `prepare_dummy_hash`
  forces at startup; `crates/web/src/routes/login.rs::submit` is the whole flow.

**Why this way.** The semaphore stops a burst of sign-ins from using all the memory; the rest
wait. Alternatives: a plain `tokio::spawn` (still blocks a worker thread); a hashing thread
with a channel (more code, same effect); a cheaper algorithm (slowness is the point).

**Read more.**

- [argon2](https://docs.rs/argon2) and
  [std: `LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html)
- [tokio: `spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) and
  [`Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)
- [OWASP: password storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)

## 6. Sessions stored in the database

**The idea.** HTTP forgets you between requests. A session is a random id kept in a cookie that
the server maps to "this is user 3". The data stays on the server, which can end it any time.

**In our code.**

- `crates/web/src/sessions.rs::layer` sets the cookie: `HttpOnly`, `SameSite::Lax`, `Secure`
  over https, 30 days of inactivity, and a `__Host-` name prefix over https (decisions #24, #62).
- `crates/web/src/sessions.rs::DbSessions` implements tower-sessions' `SessionStore` over the
  domain's `SessionRepo`. It uses `#[async_trait]` because that trait comes from another
  crate that declares it that way (decision #56). Its `save` only updates a stored session and
  never recreates one (decision #111).
- `crates/web/src/sessions.rs::id_hash` stores only the SHA-256 of the session id (#57).
- `crates/web/src/auth/mod.rs::start_session` calls `session.cycle_id()` at sign-in, so an id
  planted before sign-in is worthless (session fixation); `crates/web/src/auth/mod.rs::RENEW_AFTER`
  renews the lifetime at most daily (#59).

**Why this way.** A server-side session ends instantly on a password change; a signed cookie
cannot be called back. Storing the hash means a leaked database holds no live sessions.
Alternatives: JWT cookies (no revocation without a server-side list); an in-memory store
(lost on every restart, not shared by two instances).

**Read more.**

- [tower-sessions](https://docs.rs/tower-sessions)
- [OWASP: session management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)

## 7. Errors and panics become pages

**The idea.** A handler that fails should return a value, and one place should decide how every
failure looks. A panic unwinds the stack, but a layer can catch it and turn it into a response.

**In our code.**

- `crates/web/src/error.rs::AppError` is a hand-written enum with `From` impls for
  `StoreError`, `askama::Error`, `tower_sessions::session::Error` and `JoinError`, so `?`
  works in handlers (note 02, section 5).
- Its `IntoResponse` impl logs the cause chain and attaches a small marker,
  `crates/web/src/error.rs::ErrorPage`, to a bare status response.
- `crates/web/src/error.rs::render_errors` is the outermost middleware: if the response carries
  the marker, it replaces the body with a translated page. The language comes from the cookie
  or the browser, because no session may exist.
- `crates/web/src/error.rs::chain` walks `Error::source` to print every cause;
  `crates/web/src/error.rs::panic_response` uses `downcast_ref` on the `Box<dyn Any + Send>`
  of a caught panic to read its message as a `&str` or a `String`.
- `crates/web/src/routes/mod.rs::refusal` maps a domain refusal to a message or an error.

**Why this way.** The user sees a request id; the log has the whole chain (decision #68). The
marker exists because `into_response` cannot know the visitor's language, but the middleware
can. Alternatives: render the page in every handler (repetition); `thiserror` for `AppError`
(it would work; the hand-written version is short and needs a custom `IntoResponse` anyway).

**Read more.**

- [axum: `IntoResponse`](https://docs.rs/axum/latest/axum/response/trait.IntoResponse.html)
- [std: `Any`](https://doc.rust-lang.org/std/any/trait.Any.html) and
  [`catch_unwind`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html)
- [The Rust Book: to panic or not to panic](https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html)

**Try it.** [Exercise 10](../../exercises/src/ex10_middleware.rs).

## 8. Templates: Askama

**The idea.** A template is an HTML file with holes. Askama turns it into Rust code at build
time, so a misspelled field is a build error, and output is HTML-escaped by default.

**In our code.**

- `crates/web/src/routes/log.rs::LogPage` has `#[derive(Template)]` and
  `#[template(path = "log.html")]`; its fields are what the template can read.
- `crates/web/templates/` holds the HTML: pages extend `base.html` and include fragments such
  as `report_fields.html`; `crates/web/src/error.rs::page` renders one with a status.
- `crates/web/src/labels.rs::room` keeps view logic in Rust, out of the template.

**Why this way.** The pages are plain HTML forms, so no script is needed (decision #96).
Alternatives: Tera or Handlebars (templates checked only at run time); Maud (HTML inside Rust
macros); a single-page app with a JSON API (much more JavaScript to secure).

**Read more.**

- [askama](https://docs.rs/askama)
- [htmx documentation](https://htmx.org/docs/)

## 9. Translations: Fluent and choosing the language

**The idea.** Plural rules differ by language, so one string per language is not enough. Fluent
is a translation format where each language file says how it forms plurals. The app must also
decide which language a request wants.

**In our code.**

- `crates/web/src/i18n.rs::LOCALES` is built by the `static_loader!` macro, which embeds the
  `.ftl` files from `crates/web/locales/` at compile time, with an English fallback (#66).
- `crates/web/src/i18n.rs::Tr` is a `Copy` newtype around `Locale` with `t`, `t1` and `n`
  for plain, one-argument and plural messages; `crates/web/src/i18n.rs::negotiate` tries the
  account, the cookie, `Accept-Language`, then English with `Option::or_else`.
- `crates/web/src/i18n.rs::from_accept_language` reads quality values and uses `is_none_or`
  and a let chain, stable in edition 2024:

```rust
if let Ok(locale) = primary.parse::<Locale>()
    && quality > 0.0
    && best.is_none_or(|(q, _)| quality > q)
{
```

- The test `crates/web/src/i18n.rs::every_key_the_pages_use_is_translated` scans templates and
  sources for keys and compares the three languages.

**Why this way.** Fluent handles plurals and gender per language, which `printf`-style strings
cannot. Alternatives: `gettext` (older tooling); a plain Rust map of strings (no plural
rules); the `rust-i18n` crate (a different file format, fewer tools around it).

**Read more.**

- [Project Fluent](https://projectfluent.org/), [fluent-templates](https://docs.rs/fluent-templates)
  and [std: `Option`](https://doc.rust-lang.org/std/option/enum.Option.html)
- [The Rust Reference: `if` and `if let`](https://doc.rust-lang.org/reference/expressions/if-expr.html)

**Try it.** [Exercise 9](../../exercises/src/ex09_request_rules.rs).

## 10. Small pure functions around the handlers

**The idea.** A pure function only turns its arguments into a result: no network, no clock, no
global. Security rules have many edge cases, so they are easiest to trust as pure functions.

**In our code.**

- `crates/web/src/security.rs::is_same_origin` decides from `Sec-Fetch-Site`, else `Origin`; a
  request with neither is refused (decision #63). Its unit tests sit beside it.
- `crates/web/src/security.rs::CSP` is a `const &str` with `\` line continuations.
- `crates/web/src/auth/client_ip.rs::resolve` takes the last entry of `X-Forwarded-For`,
  because that is the one the trusted proxy added (#61, #74).
- `crates/web/src/lib.rs::logged_path` returns either a `&'static str` literal or its input.
  The signature `fn(&str) -> &str` allows both, because the rules of lifetime elision tie the
  result to the argument, and a `'static` literal lives longer than any argument (#119).
- `crates/web/src/config.rs::BaseUrl::parse` parses an `http::Uri` and normalises it into a
  newtype (#62).

**Why this way.** The handler stays thin and the rule is tested without a server.
Alternative: logic in the middleware body, testable only through a whole request.

**Read more.**

- [MDN: Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP) and
  [`Sec-Fetch-Site`](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Sec-Fetch-Site)
- [OWASP: CSRF prevention](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- [The Rust Book: lifetime syntax](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)

**Try it.** [Exercise 9](../../exercises/src/ex09_request_rules.rs).

## 11. Configuration, the command line and background tasks

**The idea.** A program needs settings and commands. Settings come from environment variables;
commands come from the command line, which the `clap` crate reads into a struct. A server also
does work with no request behind it, such as deleting old sessions: tasks next to the server.

**In our code.**

- `crates/app/src/cli.rs::Cli` and `crates/app/src/cli.rs::Command` use clap's derive API: doc
  comments become `--help`, and `value_parser = parse_locale` checks a language. A test checks
  that a password cannot be passed as an argument (decision #73).
- `crates/app/src/config.rs::ServeConfig` has `from_lookup(get: impl Fn(&str) ->
  Option<String>)`: the environment is passed in as a closure, so tests need no real
  environment variables.
- `crates/app/src/lib.rs::run` matches on the command (exit code 1 when `verify-audit` finds
  problems, 2 for errors); `crates/app/src/lib.rs::init_logging` sets up `tracing_subscriber`
  with an `EnvFilter`, logging to stderr.
- `crates/app/src/commands/serve.rs::run` starts three sweeps with `tokio::spawn`, each a loop
  over `tokio::time::interval`, and calls `JoinHandle::abort` at shutdown;
  `crates/app/src/commands/serve.rs::shutdown_signal` waits for Ctrl-C or SIGTERM with
  `tokio::select!`.
- `crates/app/src/backend.rs::with_store` is a macro that runs one expression with whichever
  store was chosen, so each command is compiled twice and no `dyn` is involved (#67).

**Why this way.** A thin `main.rs` plus a library, `smoke_app` (#72), lets
`crates/app/tests/commands.rs` test the commands without a process. Alternatives:
`std::env::var` calls scattered in the code (nothing lists them); a config file (a second
place for secrets).

**Read more.**

- [clap: the derive tutorial](https://docs.rs/clap/latest/clap/_derive/index.html)
- [tracing](https://docs.rs/tracing), [tracing-subscriber](https://docs.rs/tracing-subscriber)
  and [tokio](https://docs.rs/tokio)
- [std: `ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html)

## 12. Integration tests that drive the router

**The idea.** A router is a value that answers requests, so a test can build one, hand it over
and read the response, with no network port. It tests the whole stack together.

**In our code.**

- `crates/web/tests/common/mod.rs::TestApp` builds a real router over a temporary SQLite file
  and a `FakeClock`, and calls it with `tower::ServiceExt::oneshot`.
- `crates/web/tests/security.rs::no_page_needs_inline_scripts_styles_or_eval` scans every
  page's HTML for inline scripts or styles, which the CSP would block.

**Why this way.** A request-level test runs in milliseconds and needs no browser. Alternatives:
only unit tests (they miss wiring mistakes); browser tests (slow, flaky, a toolchain to keep).

**Read more.**

- [The Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [tower](https://docs.rs/tower) and [http-body-util](https://docs.rs/http-body-util)

## 13. A small image: Docker and the release build

**The idea.** A free tier means a small image. A static binary (linked against musl, with no
system library to load) can run in an image with almost nothing else.

**In our code.**

- `Dockerfile` has two stages: it builds on `rust:1-alpine` with `cargo auditable build
  --release --locked`, then copies the binary to a distroless image and runs as user 65532
  with `ENTRYPOINT` and `CMD ["serve"]`.
- `.dockerignore` keeps `target`, `.git` and `docs` out of the build context, so it stays small.
- `.github/workflows/deploy.yml` and `.github/workflows/security.yml` scan the image with
  Trivy (decision #70).
- `Cargo.toml` sets `lto = "thin"`, `strip = true` and `codegen-units = 1` in
  `[profile.release]`: smaller and faster, slower to build. Where the image runs, and why, is
  in [the hosting note](hosting.md).

**Why this way.** A static binary needs no libc in the image, and distroless has no shell.
Alternatives: a Debian base (hundreds of megabytes); building on the host (depends on one
machine).

**Read more.**

- [The Cargo Book: workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
  (profiles are set once for the whole workspace)

## Check yourself

1. Why must `Pairs` be the last argument of `submit`?
2. In what order do the layers see a request, and where would a new "log every request" layer
   go if it had to see the error pages too?
3. Why does `LoginLimiter::begin` count an attempt before the password is checked?
4. What breaks if the password work is done directly in the handler, and what if the
   semaphore is removed?
5. Why does the app store the hash of the session id instead of the id?
6. What does `ErrorPage` do that returning an HTML page directly from
   `AppError::into_response` could not?

## Where next

Next: [04 — The logging flow](04-logging-flow.md).
Exercises for this note: [8, a login limiter](../../exercises/src/ex08_login_limiter.rs),
[9, request rules](../../exercises/src/ex09_request_rules.rs) and
[10, middleware and error chains](../../exercises/src/ex10_middleware.rs).
