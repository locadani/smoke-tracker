# 05 — People, invites and the PWA

> Written against `main` at commit `5b52604` (milestone 6 merged). Code is cited as
> `path::Symbol`, never by line number, so the note survives edits: if a symbol has been renamed,
> search the file for the old name. Exercises for this note: 14 (expiry picks), 15 (assets, caching and an SVG drawing). Reading time: about
> 45 minutes, plus the exercises.

## What you will learn

- How an error type can translate a storage error by hand, so the web layer never sees storage details.
- How an enum with one data-carrying variant models "one of these six things", and how `match` checks you cover them all.
- How a type such as `TokenHash` keeps a secret from leaking by being a different type from `String`.
- How a picture can be built as a `String` with `write!`, with no image library.
- What `move` does in a closure that runs on another thread, and why that thread needs `'static`.
- How a file is compiled into the program with `include_bytes!`, and how `LazyLock` computes a value once.
- How an ETag lets a browser ask "has this changed?" and get a `304` with no body.
- How a service worker (JavaScript) and the Rust server cooperate in a PWA.

## The map

This note covers the people side of the app: a resident invites someone, a QR code carries a
one-time link, the invitee sets a password, and later installs the app on a phone. It ends with
what the app does to stop that link leaking.

- `crates/domain/src/people.rs`: the account use cases and `PeopleError`.
- `crates/domain/src/permissions.rs::check_expiry`: the rule that decides who may expire.
- `crates/web/src/routes/people/expiry.rs`: reads the expiry fields of the forms.
- `crates/web/src/routes/people/links.rs`: creates a link and shows it (`new_token`, `create`).
- `crates/domain/src/types.rs::TokenHash` and `crates/domain/src/records.rs::Invite`: the secret and its record.
- `crates/web/src/qr.rs`: draws the QR code as SVG.
- `crates/web/src/routes/accept.rs`: the page behind the link, where the password is set.
- `crates/web/src/assets.rs`: the files compiled into the binary, with their ETags.
- `crates/web/static/sw.js`, `crates/web/static/manifest.webmanifest`, `crates/web/static/app.js`: the PWA.
- `crates/web/src/security.rs` and `crates/web/src/lib.rs`: headers and log rules around the link.

## 1. Account use cases and their errors

**The idea.** Inviting a person, expiring an account and changing a password are rules, not
screen details. A rule that lives in a web handler is skipped by anything that is not that
handler. So each one is a function in the domain crate: it checks that the actor may do it,
reads the data it needs inside a transaction (a group of reads and writes the database applies
all together or not at all), and writes. When something goes wrong the function returns a
`PeopleError`, an enum that names each way it can fail.

**In our code.**

- `crates/domain/src/people.rs::invite` and `crates/domain/src/people.rs::issue_link` create
  an account with a link, or a new link for an existing one. Both start with a permission check.
- `crates/domain/src/people.rs::usable_invite` and `crates/domain/src/people.rs::accept` look up
  a link and use it up.
- `crates/domain/src/people.rs::expire`, `crates/domain/src/people.rs::reactivate` and
  `crates/domain/src/people.rs::change_password` follow the same shape. All are generic over the
  write-transaction traits (`T: UserWrites + InviteWrites`), like the logbook in note 04, and the
  permission check uses data read inside the transaction (#113).
- `crates/domain/src/people.rs::PeopleError` has a hand-written `From<StoreError>`. Read the
  arms: two storage conflicts mean something to the user, everything else passes through.
- `crates/web/src/routes/mod.rs::people_refusal` turns what is left into a 403, a 404 or an
  internal error. `crates/web/src/routes/people/mod.rs::require_manager` answers 403 to anyone
  but a resident, and the menu hides the link (#118).

```rust
        match error {
            StoreError::Conflict(Conflict::UsernameTaken) => Self::UsernameTaken,
            StoreError::Conflict(Conflict::InviteUsed) => Self::LinkInvalid,
            StoreError::Invalid(invalid) => Self::Invalid(invalid),
            other => Self::Store(other),
        }
```

**Why this way.** A `#[from]` attribute (from the `thiserror` crate) writes one conversion that
treats every `StoreError` the same. Here two of them have a meaning the user must see, so the
conversion is written out, and the web layer only ever matches on `UsernameTaken` and
`LinkInvalid`. Alternative: keep `#[from]` and match on `PeopleError::Store(StoreError::Conflict(..))`
in the web layer. It lost because the web layer would then know storage details. Alternative:
check permissions in the web layer only. It lost because a second front door, such as the
command line, would skip them. The web-only check would be the better choice for a rule that is
purely about a screen, such as which button to show.

**Read more.**

- [The Rust Book, 9.2: recoverable errors with `Result`](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html)
- [std: the `From` trait](https://doc.rust-lang.org/std/convert/trait.From.html)

## 2. Expiry picks: enums with data

**The idea.** A guest account must end; a viewer's may or may not; a resident's never does. The
form offers a few choices: one day, the weekend, one week, two weeks, a chosen day, or never.
That is "one of these six things", and Rust has a type for it: an enum, where each variant is
one thing. A variant can also carry data. Five of the six picks need nothing more; "until this
day" needs the day.

**In our code.**

- `crates/domain/src/people.rs::ExpiryPick` has unit variants (`OneDay`, `Weekend`, `OneWeek`,
  `TwoWeeks`, `Never`) and one that carries data, `Until(NaiveDate)`.
- Its `resolve` method takes `self`, the current time and the schedule, and returns
  `Option<DateTime<Utc>>`. It is a `match` with one arm per variant. `Weekend` asks the weekday
  for `num_days_from_monday()` and uses the schedule's `day_span`, so "next Monday 00:00" is
  local time, and on a Monday it means the Monday after (#115).
- `crates/domain/src/permissions.rs::check_expiry` is a `match` on the pair `(role, expires_at)`
  with a guard arm (`if at <= now`).
- `crates/web/src/routes/people/expiry.rs::read` turns what the form posted into an `ExpiryPick`.

```rust
    match (role, expires_at) {
        (Role::Resident, Some(_)) => Err(DomainError::ResidentsNeverExpire),
        (Role::Guest, None) => Err(DomainError::GuestsMustExpire),
        (_, Some(at)) if at <= now => Err(DomainError::ExpiryNotInFuture),
        _ => Ok(()),
    }
```

The arms are tried in order, so the order is part of the rule: a resident with a date in the
past gets "residents never expire", not "not in the future".

**Why this way.** `Never` resolves to `None`, so "no expiry" is part of the type and not a magic
date far in the future. When a seventh pick is added, every `match` without a catch-all arm
stops compiling until it handles it; that is the compiler doing your checklist. Alternative: a
number of days as a `u32`. It lost because it cannot say "weekend" or "a chosen date".
Alternative: an `Option<DateTime>` picked in the web layer. It lost because the rule would sit
in a handler and could not be tested without a request. A plain number would have been better
if the only choices were "N days".

**Read more.**

- [The Rust Book, 6.1: defining an enum](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html)
- [The Rust Book, 6.2: the `match` control flow construct](https://doc.rust-lang.org/book/ch06-02-match.html)
- [chrono on docs.rs](https://docs.rs/chrono)

**Try it.** [Exercise 14](../../exercises/src/ex14_expiry_pick.rs).

## 3. Secrets in types: tokens, hashes and one-time links

**The idea.** Whoever holds the link can set a password for that account, so the link is a
password. A password is never stored as typed; the app stores a fingerprint of it, a hash, which
cannot be turned back into the original. The link works the same way: only its SHA-256 hash is
kept, and the real token exists in the page that shows the QR code.

**In our code.**

- `crates/web/src/routes/people/links.rs::new_token` fills a `[0u8; 32]` array with
  `getrandom::fill` and returns it as hex (#119).
- `crates/domain/src/types.rs::TokenHash` is a tuple struct around a `String`. `TokenHash::of`
  hashes; its `Debug` implementation prints only a prefix, so a stray `{:?}` in a log line
  cannot leak a full hash.
- `crates/domain/src/records.rs::Invite` has no token and no hash field at all, so neither can
  reach the audit snapshot (#112). `NewInvite` hands the store a `TokenHash`. `Invite::LIFETIME`
  is five minutes and `is_usable` checks that the link is unused and young (#16).
- `crates/domain/src/repos.rs::InviteRepo` has `invite_by_token(&self, token: &TokenHash)`. A
  raw token is a `&str`, so it cannot be passed to the store by mistake: the compiler refuses.

```rust
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(hex::encode(bytes))
```

**Why this way.** The rule is "a link is a password": keep only its hash, let it live five
minutes, let it work once. The type system carries the rule, because a `TokenHash` is not a
`String` and cannot be mixed up with one. Alternative: store the token. It lost because a
database leak would hand out live links. Alternative: a signed token with no storage. It lost
because it cannot be made single-use without storing something. A signed token is the better
choice when a link may be reused freely, such as a public "unsubscribe" link.

**Read more.**

- [The Rust Book, 5.1: defining and instantiating structs](https://doc.rust-lang.org/book/ch05-01-defining-structs.html)
- [getrandom on docs.rs](https://docs.rs/getrandom)
- [sha2 on docs.rs](https://docs.rs/sha2)
- [hex on docs.rs](https://docs.rs/hex)

## 4. A QR code drawn as inline SVG

**The idea.** A QR code is a square grid of dark and light cells (modules). The app must show one
for the link, but its Content-Security-Policy (a header that lists where the page may load things
from) does not allow `data:` images. SVG is text that describes a drawing, and a page can carry
it inline, so the grid is drawn as one `<path>` with a tiny square for each dark module.

**In our code.**

- `crates/web/src/qr.rs::svg` asks `QrCode::encode_text` (from the `qrcodegen` crate) for the
  grid, loops over every module, appends to a `String` with `write!` (needs
  `use std::fmt::Write`), and ends with `format!`. The `expect` calls are on results that cannot
  fail here: a link is far below the QR capacity, and writing to a `String` never errors.
- `crates/web/src/qr.rs::QUIET_ZONE` is a named `const`: the light margin the QR specification
  asks for, in modules.
- `crates/web/src/routes/people/links.rs::create` renders the page that shows the QR code.
- `crates/web/templates/link.html` inserts the SVG with `|safe`, which turns off escaping. That
  is acceptable only because the function builds the markup from numbers, never from user text.
- `crates/web/src/security.rs::CSP` sets `img-src 'self'`.

```rust
            if code.get_module(x, y) {
                let (left, top) = (x + QUIET_ZONE, y + QUIET_ZONE);
                write!(path, "M{left},{top}h1v1h-1z").expect("writing to a String cannot fail");
            }
```

**Why this way.** Inline SVG needs no file, no image library and no CSP exception, and the token
exists only in the response that shows it (#119). Alternative: the `qrcode` crate. It lost
because it pulls in the `image` crate by default. Alternative: a PNG served from a URL. It lost
because the token would have to be stored to serve it again. The `qrcode` crate would be the
better choice in an app that already needs to produce images.

**Read more.**

- [qrcodegen on docs.rs](https://docs.rs/qrcodegen)
- [std: the `fmt::Write` trait](https://doc.rust-lang.org/std/fmt/trait.Write.html)
- [std: the `write!` macro](https://doc.rust-lang.org/std/macro.write.html)
- [MDN: Content-Security-Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)

**Try it.** [Exercise 15](../../exercises/src/ex15_pwa_assets.rs) (`svg_path` and `svg`).

## 5. Setting a password: blocking work and values moved into a thread

**The idea.** Hashing a password with Argon2 is deliberately slow: it uses a lot of CPU and
memory so that guessing is expensive. Note 03 explained why that work runs on a blocking thread
and not on the threads that serve requests. The new idea here is ownership. The work runs on a
different thread that may still be running after the request has ended, so it cannot borrow
anything from the request. It must own what it uses.

**In our code.**

- `crates/web/src/routes/accept.rs::submit` computes the hash with
  `password::run_limited(state.hash_permits.clone(), move || ...)`. The `move` closure takes
  ownership of `new_password`; the `Arc` of permits is cloned first because the handler's state
  still needs it.
- `crates/web/src/auth/password.rs::run_limited` is where the bounds live: the closure is
  `impl FnOnce() -> T + Send + 'static`. `'static` here means "owns everything it needs", not
  "lives forever".
- `crates/web/src/routes/accept.rs::usable` checks the link before the slow work. The domain
  function `accept` checks it again with `usable_invite` inside the transaction that uses it up,
  because another request may have used it in between.
- `crates/web/src/auth/mod.rs::start_session` signs the device in again with a new session id.
- `crates/web/src/routes/settings.rs::change` does the same for a password change. It first
  checks the current password as a sign-in attempt, under the same limits (#116).

**Why this way.** A closure sent to another thread must be `'static + Send`, so it takes its
data by `move`. Without `move`, the compiler reports that the closure may outlive the function
because it borrows a local variable, and suggests `move`. Alternative: borrowing data into the
thread. The compiler refuses, because the request may end first. Alternative: scoped threads
(`std::thread::scope`) can borrow, but they make the caller wait for the thread, which would block
a server thread; an async handler cannot wait that way. Scoped threads are the better choice
in a plain program that wants to split a calculation across cores and then continue.

**Read more.**

- [The Rust Book, 16.1: using threads to run code simultaneously](https://doc.rust-lang.org/book/ch16-01-threads.html)
- [The Rust Book, 13.1: closures](https://doc.rust-lang.org/book/ch13-01-closures.html)

## 6. HTTP caching: embedded files and ETags

**The idea.** The browser downloads the CSS, the scripts and the icons. Fetching them again on
every visit wastes time, but caching them forever hides a new deploy. An ETag solves it: a short
label for one version of a file. The browser keeps the file and the label; next time it asks
"do you still have `"abc"`?" by sending it in `If-None-Match`. If the label still matches, the
server answers `304 Not Modified` with no body.

**In our code.**

- `crates/web/src/assets.rs::asset` is a `macro_rules!` that builds an `Asset` with
  `include_bytes!(concat!("../static/", $path))`: the file's bytes are compiled into the binary.
- `crates/web/src/assets.rs::ASSETS` is a `const` slice of them, listed explicitly.
- `crates/web/src/assets.rs::ETAGS` is a `LazyLock<Vec<String>>`: the first 16 hex characters of
  each file's SHA-256, computed once, on first use.
- `crates/web/src/assets.rs::respond` uses `position` to find the asset, compares
  `If-None-Match` with the tag, and returns `304` with no body on a match. `Cache-Control:
  no-cache` means "keep it, but revalidate" (#65).
- `crates/web/src/assets.rs::service_worker` serves the same `sw.js` at `/sw.js`, so its scope is
  the whole site and not only `/static/`.
- The tests `crates/web/src/assets.rs::every_file_in_the_static_folder_is_served` and
  `crates/web/src/assets.rs::the_vendored_libraries_are_the_published_releases` keep the list
  and the bundled libraries honest.

```rust
static ETAGS: LazyLock<Vec<String>> = LazyLock::new(|| {
    ASSETS
        .iter()
        .map(|asset| format!("\"{}\"", &hex::encode(Sha256::digest(asset.body))[..16]))
        .collect()
});
```

**Why this way.** One file to deploy and no path to get wrong at runtime; revalidation makes a
deploy visible at once, while a repeat visit costs a `304`. A plain `static` must be built from
a value known at compile time, and a SHA-256 call is not; `LazyLock` runs the closure on first
use and then hands out the same value. Alternative: `tower-http`'s `ServeDir`. It lost because it
reads files at runtime, so they would have to be in the Docker image. Alternative: the
`rust-embed` crate. It does the same with less hand-written code, but adds a dependency for
twelve files. `rust-embed` would be the better choice with hundreds of files.

**Read more.**

- [std: the `include_bytes!` macro](https://doc.rust-lang.org/std/macro.include_bytes.html)
- [std: `LazyLock`](https://doc.rust-lang.org/std/sync/struct.LazyLock.html)
- [MDN: the `ETag` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/ETag)

**Try it.** [Exercise 15](../../exercises/src/ex15_pwa_assets.rs) (`etag_of` and `respond`).

## 7. The PWA: manifest, service worker and the install prompt

**The idea.** A progressive web app (PWA) is a website a phone can install: it gets an icon and
opens without the browser's address bar. It needs a manifest (a JSON file with the name, icons
and start page) and usually a service worker (a script the browser runs in the background that
can answer requests itself). This section is about the web platform: the service worker must be
JavaScript. Rust's job is to serve these files correctly, which is section 6.

**In our code.**

- `crates/web/static/manifest.webmanifest` holds the name, the scope, the icons and a "New smell
  report" shortcut.
- `crates/web/static/sw.js` has three handlers. `install` precaches a few files, `activate`
  deletes old caches, and `fetch` sends navigations to the network with `/offline` as the
  fallback. Static files are fetched first and the cached copy is the fallback. Form posts are
  never touched (#121).
- `crates/web/static/app.js` captures `beforeinstallprompt` so any element with `data-install`
  can open the browser's install dialog.
- `crates/web/src/routes/install.rs::show` and `crates/web/src/routes/install.rs::offline` are
  the two pages.
- The test `crates/web/tests/pwa.rs::everything_the_service_worker_precaches_exists` reads the
  `PRECACHE` list out of `sw.js` and requests each path, because one missing file would make the
  whole install fail.

**Why this way.** Network-first means a new deploy is never hidden behind an old cache. The price
is that the app does not work offline beyond the offline page; offline logging is out of scope
(spec §14). Alternative: cache-first. It is fast but stale after each deploy unless the files
are versioned. Alternative: the Workbox library. It lost because it is more machinery than three
event handlers. Cache-first would be the better choice for an app that must work on a train.

**Read more.**

- [MDN: Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [MDN: Progressive web apps](https://developer.mozilla.org/en-US/docs/Web/Progressive_web_apps)
- [MDN: Web app manifest](https://developer.mozilla.org/en-US/docs/Web/Manifest)
- [MDN: `BeforeInstallPromptEvent`](https://developer.mozilla.org/en-US/docs/Web/API/BeforeInstallPromptEvent)

**Try it.** [Exercise 15](../../exercises/src/ex15_pwa_assets.rs) (`network_first`).

## 8. A link token that behaves like a password

**The idea.** A secret in a URL leaks in places a password does not: the browser's `Referer`
header (the address of the page you came from), server logs, a shared screenshot. You cannot
close every leak. The honest approach is to close the ones you can, list the ones you cannot,
and shorten the life of the secret so a leak matters less.

**In our code.**

- `crates/web/src/security.rs::headers` sends `Referrer-Policy: no-referrer` for paths under
  `/invite/` and `same-origin` elsewhere.
- `crates/web/src/lib.rs::logged_path` replaces the token with `/invite/…`, and
  `crates/web/src/lib.rs::request_span` uses it, so the token never reaches the app's own logs.
- Decisions #119 and #122 say what remains: the platform's request log could still contain the
  token. It is accepted because a link works once and for five minutes (#16).

**Why this way.** The mitigations are chosen by how long the secret lives, and the leaks that
stay are written down. Alternative: put the token in a URL fragment (the part after `#`, which the
browser never sends) and read it with script. It lost because the policy discourages extra
script, for little gain. Alternative: a proxy that strips paths from the platform log. It lost
because it is another moving part to run, for a link that is already short-lived. A fragment
would be the better choice if the log were readable by people other than the owner.

**Read more.**

- [MDN: the `Referrer-Policy` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Referrer-Policy)
- [OWASP: Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)

## Check yourself

1. Why is `Until` a variant with data while `Weekend` has none?
2. What does the compiler force you to do when a seventh pick is added to `ExpiryPick`?
3. Why does `invite_by_token` take a `&TokenHash` and not a `&str`?
4. What does `move` do in the closure passed to `run_limited`, and what would the compiler say without it?
5. Why does the service worker not cache pages, and where would you change that?
6. What does `LazyLock` buy in `ETAGS` that a plain `static` could not?

## Where next

Next: note 06, graphs (series, iterators and charts).

Exercises for this note: [Exercise 14](../../exercises/src/ex14_expiry_pick.rs) (expiry picks) and [Exercise 15](../../exercises/src/ex15_pwa_assets.rs) (assets, caching and an SVG drawing).
