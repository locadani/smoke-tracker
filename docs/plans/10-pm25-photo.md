# PM2.5 photo Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** From the Log form, take a photo of the ALPSTUGA sensor; the app reads the PM2.5 value
(Google Cloud Vision, EU) and pre-fills the field; the confirmed value is saved with the
watermarked photo, whose fingerprint is part of the log entry's audited snapshot.

**Architecture:** The domain models a photo as part of a PM2.5 reading (`Pm25Reading.photo`), so
"no value, no photo" is a type rule, and the entry snapshot covers it. A new crate `smoke-photos`
holds the two outbound adapters behind enums (`PhotoStore`: local folder or GCS; `Pm25Reader`:
none or Cloud Vision; later a Rust reader variant) plus upload validation. `smoke-web` takes the
photo as `multipart/form-data`, stores it when Log is pressed, and carries a reference
(hidden fields) through the "which event?" question and error re-renders. Phone-side JS does
camera, watermark and shrinking. Terraform adds a private Frankfurt bucket.

**Tech Stack:** Rust (edition 2024, MSRV 1.94), axum 0.8 (+ `multipart`), askama 0.16,
sqlx 0.9 (SQLite + Postgres), tokio, `reqwest` (rustls, json), `sha2` (already used),
`base64`, plain browser JS, Terraform (google provider), Playwright e2e.

**Spec:** `docs/specs/2026-11-02-pm25-photo-design.md` (approved). Builds on
`docs/specs/2026-10-19-log-entries-design.md`; previous comparable plan: `docs/plans/09-log-entries.md`.

## Global Constraints

- Personal project. Before every commit `git config user.name` must print `locadani`. Never put an email address in a tracked file. No push, no merge, no PR by agents.
- Every commit message ends with a blank line and `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`. Conventional Commits; name the Rust concept where there is one.
- Never commit secrets, keys or tokens; the gitleaks hook (`git config core.hooksPath .githooks`) must be active. Google auth is only via the metadata server (no key files, no env tokens).
- Never modify `description.md`, `claude.md`, `CLAUDE.md`, `docs/specs/*` or `docs/DECISIONS.md` without the user's explicit approval. Never commit the sample photos or anything derived from them.
- At every task boundary: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (SQLite and Postgres), `cargo deny check` pass.
- Postgres tests need `TEST_POSTGRES_URL` (locally `postgres://postgres@localhost:55432/postgres`, container `smoke-pg`). After changing any `query!` run `scripts/sqlx-prepare.sh` and commit both `.sqlx/`; `scripts/sqlx-prepare.sh --check` passes.
- Migrations are append-only: new `0006_pm25_photos.sql` in both backends; `schema_parity.rs` passes with its column count updated.
- Audit entries are never rewritten (decision #104). Snapshots of entries **without** a photo must stay byte-identical to today's.
- `smoke-web` never sees SQL. CSP stays `'self'`-only except `img-src 'self' blob:`; no inline script/style; new pages join `no_page_needs_inline_scripts_styles_or_eval`.
- Guests see only their own entries (`permissions::can_see`); someone else's photo answers 404, exactly like `/entries/{id}`.
- Every new translation key in `en`, `it`, `tr`. No tutorial comments in code. Every "now" via `Clock`.
- New crates allowed: `reqwest` (`default-features = false`, `rustls-tls`, `json`), axum feature `multipart`, `base64`. Nothing else; no Google SDK crates.
- Size limit: default `PHOTO_MAX_BYTES = 307200` (300 KB). The global 16 KB body limit stays; only `POST /log` and `POST /log/pm25/read` get `PHOTO_MAX_BYTES + 16 KB`, and both check sign-in before reading the body.
- Vision: `https://eu-vision.googleapis.com/v1/images:annotate`, feature `TEXT_DETECTION`. Bucket: `europe-west3`, uniform access, public access prevention enforced, runtime SA has `roles/storage.objectCreator` + `roles/storage.objectViewer` on it only.
- Disk: one shared `CARGO_TARGET_DIR`; never `rm -rf`/`cargo clean`; check `df -h /System/Volumes/Data` (>= 20 GB) before big builds; give the user `cargo clean` commands at the end.

## Review Focus

0. **Bucket and database out of step.** Expected (spec §7, option 1): an entry never points to a missing or different photo; leftovers only under `pending/` (auto-deleted) or reported by `verify-audit`. Tests: `the_entry_is_written_only_after_the_photo_is_permanent`, `logging_again_after_a_failure_reuses_the_same_file` (Task 6), `promoting_a_tampered_pending_photo_fails` (Task 3), `verify_audit_reports_a_photo_without_entry` (Task 9).
1. **A photo that doesn't survive the "which event?" question or an error re-render.** Expected: the photo stored at the first Log press is attached to the entry saved after the question / after fixing the error, never lost, never stored twice. Tests: `a_photo_survives_the_event_question`, `a_photo_survives_a_refused_form` (Task 6).
2. **A forged photo reference** in the hidden fields (key of another file, wrong fingerprint, path tricks like `../`). Expected: refused (form back with `photo-invalid`), nothing saved. Tests: `a_forged_photo_reference_is_refused`, `the_entry_is_written_only_after_the_photo_is_permanent`, `logging_again_after_a_failure_reuses_the_same_file`, `photo_keys_reject_paths` (Tasks 3, 6).
3. **Old entries' hashes change** because the snapshot gained a field. Expected: `verify-audit` clean on a database migrated from 0005 with existing entries; snapshot JSON without photo equals the pre-feature JSON. Test: `a_snapshot_without_photo_is_unchanged` (Task 1), `verify_audit_is_clean_after_0006` (Task 2).
4. **Oversized or non-JPEG upload, or anonymous upload.** Expected: 413 / `photo-invalid` / redirect to login, without buffering beyond the limit. Tests: `an_oversized_photo_is_refused`, `a_png_is_refused`, `anonymous_photo_posts_are_refused_before_the_body` (Task 6).
5. **Vision down or slow.** Expected: `/log/pm25/read` answers "couldn't read" within 10 s; Log still works (photo store is independent). Test: `a_reader_failure_is_just_no_number` (Task 4).

---

## File Structure

```
crates/
├─ domain/src/records.rs        Pm25Photo, PhotoKey, Sha256Hex; Pm25Reading.photo; NewReading; NewEntry.reading
├─ domain/src/logbook.rs        SmellEntry.pm25 / ReadingEntry carry NewReading
├─ storage-{sqlite,postgres}/
│  ├─ migrations/0006_pm25_photos.sql
│  └─ src/entries.rs            insert + load the photo row
├─ storage-contract/src/entries.rs, audit.rs   photo round-trip + tamper tests
├─ photos/  (new crate smoke-photos)
│  ├─ src/lib.rs                re-exports
│  ├─ src/upload.rs             validate(bytes, max) -> ValidPhoto (JPEG, size, sha256, key)
│  ├─ src/store.rs              enum PhotoStore { Local, Gcs }
│  ├─ src/google.rs             metadata-server token cache, shared reqwest client
│  └─ src/reader.rs             enum Pm25Reader { None, CloudVision }; pick_pm25()
├─ web/src/routes/photos.rs     POST /log/pm25/read, GET /photos/{reading_id}
├─ web/src/routes/entries.rs    multipart Log, photo reference fields
├─ web/src/{security,lib,config,state,forms}.rs
├─ web/static/js/pm25-photo.js  button, camera choice, watermark, shrink, preview, read
├─ web/static/js/pm25-camera.js our camera screen (torch, overlay, rotation)
├─ web/css/74-photo.css, templates/{log,entry}.html, locales/{en,it,tr}/main.ftl
└─ app/src/{config,backend}.rs, commands/verify_audit.rs
infra/modules/cloud-run-app/photos.tf (+ variables, outputs, tests), infra/prod/main.tf
docs/runbooks/pm25-photo.md     console steps the user does (enable Vision API)
tests/e2e/specs/pm25-photo.spec.js
```

---

### Task 1: Domain — a photo belongs to a reading

**Files:** Modify `crates/domain/src/records.rs`, `crates/domain/src/logbook.rs`, `crates/domain/src/error.rs`, every caller of `NewEntry::new`, `SmellEntry`, `ReadingEntry` (compiler-guided). Test: in-module tests of `records.rs`, `logbook.rs`.

**Interfaces — Produces:**
- `pub struct PhotoKey(String)` — `PhotoKey::parse(&str) -> Result<Self, DomainError>` accepts only `photos/YYYY/MM/<64 lowercase hex>.jpg`; `PhotoKey::for_photo(sha: &Sha256Hex, at: DateTime<Utc>) -> Self` = `photos/{at:%Y}/{at:%m}/{sha}.jpg` (UTC); `as_str()`. Serialises as a string.
- `pub struct Sha256Hex(String)` — `parse` accepts exactly 64 lowercase hex; `as_str()`.
- `pub struct Pm25Photo { pub key: PhotoKey, pub bytes: u32, pub sha256: Sha256Hex, pub taken_at: DateTime<Utc> }` (Serialize, Clone, PartialEq, Debug).
- `Pm25Reading` gains `#[serde(skip_serializing_if = "Option::is_none")] pub photo: Option<Pm25Photo>`.
- `pub struct NewReading { pub value: Pm25, pub photo: Option<Pm25Photo> }`; `NewEntry.reading: Option<NewReading>` (the `NewEntry::new` parameter changes type accordingly).
- `SmellEntry.pm25: Option<NewReading>`; `ReadingEntry { observed_at, reading: NewReading, note }`.
- `DomainError::InvalidPhotoRef`.

- [ ] **Step 1: Failing tests** in `records.rs`:
  - `a_snapshot_without_photo_is_unchanged`: build a `LogEntry` with a reading and no photo; `assert_eq!(entry.snapshot(), json!({...today's exact shape, "reading": {"id": 7, "value": 12.5}...}))` — copy the shape from the existing snapshot test.
  - `a_snapshot_with_photo_includes_it`: same entry with a photo; `snapshot()["reading"]["photo"] == json!({"key": "photos/2026/11/<64 hex>.jpg", "bytes": 250000, "sha256": "<64 hex>", "taken_at": "2026-11-02T13:35:00Z"})` (use whatever instant format the existing snapshots use).
  - `photo_keys_reject_paths`: `PhotoKey::parse` refuses `"../x"`, `"photos/2026/11/abc.jpg"`, uppercase hex, `"photos/2026/13/<hex>.jpg"`, and accepts a valid key.
- [ ] **Step 2:** `cargo test -p smoke-domain` — FAIL (types missing).
- [ ] **Step 3:** Implement the types above; update `log_smell`/`log_reading` and all callers so they compile (readings without photo pass `photo: None`).
- [ ] **Step 4:** `cargo test --workspace` — PASS.
- [ ] **Step 5:** Commit `feat(domain): a PM2.5 reading can carry a photo (newtype validation)`.

### Task 2: Storage — migration 0006 and the photo row

**Files:** Create `crates/storage-{sqlite,postgres}/migrations/0006_pm25_photos.sql`. Modify `crates/storage-{sqlite,postgres}/src/entries.rs`, `crates/storage-postgres/tests/schema_parity.rs`, `crates/storage-contract/src/{entries,audit,migrations}.rs`, `.sqlx/`.

**Interfaces:** Consumes Task 1. Produces: `create_entry` persists `NewEntry.reading.photo`; every entry load fills `Pm25Reading.photo`.

Schema (both backends, Postgres types shown; SQLite `INTEGER`/`TEXT`):
```sql
CREATE TABLE pm25_photos (
    pm25_reading_id BIGINT NOT NULL PRIMARY KEY REFERENCES pm25_readings (id),
    object_key      TEXT   NOT NULL UNIQUE,
    bytes           BIGINT NOT NULL CHECK (bytes > 0),
    sha256          TEXT   NOT NULL CHECK (length(sha256) = 64),
    taken_at        BIGINT NOT NULL
);
```
(The reading id is the key: one photo per reading, no separate id. No upload time is stored (user's decision): the entry's `recorded_at` is the server time.)

- [ ] **Step 1: Failing contract tests** (run for both backends):
  - `an_entry_keeps_its_photo`: create with photo → `entry(id)` returns the same `Pm25Photo`.
  - `a_changed_photo_fingerprint_is_detected`, `a_relinked_photo_is_detected`, `a_deleted_photo_row_is_detected`: direct SQL edit after create → `verify` reports that entry (same style as `relinking_a_reading_to_another_entry_is_detected`).
  - `verify_audit_is_clean_after_0006`: fixture with entries created before 0006 (migrate to 0005, insert via store, migrate to 0006) → verify reports nothing.
- [ ] **Step 2:** run — FAIL.
- [ ] **Step 3:** Migration + insert (inside `create_entry`, after the reading insert) + load (join or second query, same as readings) in both backends; update `schema_parity` count (+5); `scripts/sqlx-prepare.sh`.
- [ ] **Step 4:** `cargo test --workspace` with `TEST_POSTGRES_URL` — PASS; `scripts/sqlx-prepare.sh --check` — PASS.
- [ ] **Step 5:** Commit `feat(storage): pm25_photos table, part of the audited log entry`.

### Task 3: `smoke-photos` crate — validation and storage

**Files:** Create `crates/photos/{Cargo.toml,src/lib.rs,src/upload.rs,src/store.rs,src/google.rs}`; add to workspace members and `[workspace.dependencies]`; `deny.toml` only if a new licence must be allowed (stop and ask if so).

**Interfaces — Produces:**
- `pub struct ValidPhoto { pub bytes: Vec<u8>, pub sha256: Sha256Hex }`
- `pub fn validate(bytes: Vec<u8>, max: usize) -> Result<ValidPhoto, PhotoError>` — refuses empty, `> max`, not starting with `FF D8 FF`.
- `pub fn pending_name(sha: &Sha256Hex) -> String` = `pending/{sha}.jpg`; `PhotoKey::for_photo(sha, at)` (Task 1) = `photos/{at:%Y}/{at:%m}/{sha}.jpg` (from Task 1).
- `pub enum PhotoStore { Local(LocalPhotoStore), Gcs(GcsPhotoStore) }` with
  `async fn put_pending(&self, photo: &ValidPhoto) -> Result<(), PhotoError>` (same name already there = Ok; GCS `ifGenerationMatch=0`, never overwrites);
  `async fn promote(&self, sha: &Sha256Hex, bytes: u32, at: DateTime<Utc>) -> Result<PhotoKey, PhotoError>` — server-side copy `pending/` → `photos/…` (destination already there = Ok), then re-reads the destination and checks length + sha256, else `PhotoError::Mismatch`; a missing pending file → `PhotoError::Missing`;
  `async fn get(&self, key: &PhotoKey) -> Result<Option<Vec<u8>>, PhotoError>`;
  `async fn list_photos(&self) -> Result<Vec<PhotoKey>, PhotoError>` (for `verify-audit`).
- `LocalPhotoStore::new(root: PathBuf)`; `GcsPhotoStore::new(bucket: String, auth: GoogleAuth)`.
- `google.rs`: `pub struct GoogleAuth` — `async fn token(&self) -> Result<String, PhotoError>` from `http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token` (`Metadata-Flavor: Google`), cached until 60 s before expiry; holds the one `reqwest::Client` (timeout 10 s). The token is never logged.
- `pub enum PhotoError { TooLarge, NotJpeg, Empty, Missing, Mismatch, Unavailable(String) }` (thiserror).

- [ ] **Step 1: Failing tests:** `validate` (`an_oversized_photo_is_refused` at `max + 1`, `a_png_is_refused`, `the_fingerprint_is_sha256_of_the_bytes` against a known vector); `the_key_is_the_month_and_fingerprint`; `LocalPhotoStore` in a temp dir (`pending_then_promote_then_get_returns_the_bytes`, `promoting_twice_is_fine`, `promoting_a_missing_pending_photo_fails`, `promoting_a_tampered_pending_photo_fails` (edit the file between put and promote → `Mismatch`), `a_missing_key_is_none`, `list_photos_lists_only_permanent_photos`). GCS: unit-test only request building (`gcs_upload_never_overwrites` asserts `ifGenerationMatch=0`; `gcs_promote_copies_server_side` asserts the `rewriteTo` URL with `ifGenerationMatch=0`) — no network in tests.
- [ ] **Step 2:** `cargo test -p smoke-photos` — FAIL.
- [ ] **Step 3:** Implement. GCS JSON API: upload `POST https://storage.googleapis.com/upload/storage/v1/b/{bucket}/o?uploadType=media&name={name}&ifGenerationMatch=0` (412 → Ok); copy `POST .../storage/v1/b/{bucket}/o/{src}/rewriteTo/b/{bucket}/o/{dst}?ifGenerationMatch=0` (412 → Ok; loop on `rewriteToken`); download `GET .../o/{urlencoded}?alt=media` (404 → None); list `GET .../o?prefix=photos/` (paged).
- [ ] **Step 4:** tests + `cargo deny check` — PASS.
- [ ] **Step 5:** Commit `feat(photos): validated photos in a local folder or a GCS bucket (enum dispatch)`.

### Task 4: `smoke-photos` — reading the number

**Files:** Create `crates/photos/src/reader.rs`, `crates/photos/tests/fixtures/vision-*.json` (hand-written, no real photos).

**Interfaces — Produces:**
- `pub enum Pm25Reader { None, CloudVision(GoogleAuth) }` with `async fn read(&self, jpeg: &[u8]) -> Option<f64>` — never errors: any failure or timeout (10 s) logs a warning and returns `None`.
- `pub fn pick_pm25(text_annotations: &[TextAnnotation]) -> Option<f64>` (pure) with `TextAnnotation { description: String, bounding_poly: Vec<(i32, i32)> }` deserialised from Vision's `textAnnotations[1..]` (word level).
  Rule: numbers are words matching `^\d{1,4}(\.\d)?$`, value 0–1000. If a word containing `PM2.5` (also `PM2,5`, `PM25`) exists, take the nearest number whose box lies to its right on an overlapping vertical band; else, if exactly one number exists, take it; else `None`.

- [ ] **Step 1: Failing tests** with fixtures: `label_and_number` (→ 19.0), `only_a_number` (→ 14.0), `two_numbers_without_label` (→ None), `nothing_readable` (→ None), `a_time_is_not_a_number` (`"20:59"` alone → None), `a_reader_failure_is_just_no_number` (`Pm25Reader::CloudVision` pointed at an unroutable endpoint via a test constructor → `None` in < 11 s).
- [ ] **Step 2:** run — FAIL.
- [ ] **Step 3:** Implement; request body `{"requests":[{"image":{"content": base64}, "features":[{"type":"TEXT_DETECTION"}]}]}` to the EU endpoint, bearer token from `GoogleAuth`.
- [ ] **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(photos): read PM2.5 from Cloud Vision text, behind a swappable reader`.

### Task 5: Configuration and wiring

**Files:** Modify `crates/app/src/config.rs`, `crates/app/src/backend.rs` (or where `AppState::new` is called), `crates/web/src/{config,state}.rs`, `.env.example`.

**Interfaces — Produces:** `WebConfig.photo_max_bytes: usize`; `AppState.photos: Arc<PhotoStore>`, `AppState.reader: Arc<Pm25Reader>`; `AppState::new(store, clock, config, photos, reader)`.
Env: `PHOTO_MAX_BYTES` (default 307200, must be 10 KB–5 MB), `PHOTO_BUCKET` (set → GCS, else `PHOTO_DIR`, default `data/photos`, local), `PM25_READER` (`vision` | `none`, default `none`).

- [ ] **Step 1: Failing tests** in `app/src/config.rs` (style of `every_variable_can_be_set`, `invalid_values_name_the_variable`): defaults, bucket selects GCS, bad `PHOTO_MAX_BYTES` names the variable.
- [ ] **Step 2–4:** implement, update web test helpers to pass `PhotoStore::Local(tempdir)` and `Pm25Reader::None`; `cargo test --workspace` PASS.
- [ ] **Step 5:** Commit `feat(app): photo store, reader and size limit from the environment`.

### Task 6: Web — routes, multipart Log, security headers

**Files:** Create `crates/web/src/routes/photos.rs`. Modify `crates/web/src/{lib,security,forms}.rs`, `routes/{mod,entries}.rs`, `Cargo.toml` (axum `multipart`), `static/js/service-worker.js`, locales. Tests: `crates/web/tests/photos.rs`, `tests/security.rs`.

**Interfaces:** Consumes Tasks 1, 3, 4, 5. Produces routes:
- `POST /log/pm25/read` (multipart field `photo`) → JSON `{"value": 19.0}` or `{"value": null}`; stores nothing.
- `POST /log` accepts `multipart/form-data` (text fields as today + optional file `photo` + optional `photo_taken_at` RFC 3339) **and** still urlencoded (existing tests). A file is validated and `put_pending` immediately; from then on the photo travels as hidden fields `photo_sha256`, `photo_bytes`, `photo_taken_at` through `ask` and `back_to_form`. Only right before the final `log_smell`/`log_reading` does the handler call `promote` (which re-checks length + fingerprint) and put the returned `PhotoKey` into `NewReading.photo`; `Missing`/`Mismatch` → `back_to_form(.., "photo-invalid")`, nothing written. The database transaction commits only after `promote` succeeded.
- A photo with no PM2.5 value → `back_to_form(.., "photo-needs-pm25")`.
- `GET /photos/{reading_id}` → JPEG, `Cache-Control: private, no-store`; 404 unless `can_see` on the entry's author (same as `/entries/{id}`).
- `Pairs::from_multipart` (or a `LogForm` extractor) returning pairs + optional photo bytes; reads the body only after `CurrentUser` succeeded; route layer `DefaultBodyLimit::max(photo_max_bytes + 16 * 1024)` on these two routes only.
- `security.rs`: `img-src 'self' blob:`; `camera=(self)`.

- [ ] **Step 1: Failing tests** (`crates/web/tests/photos.rs`, existing `app.at("…")` harness): `a_reading_with_a_photo_is_saved`, `a_photo_survives_the_event_question`, `a_photo_survives_a_refused_form`, `a_forged_photo_reference_is_refused`, `a_photo_without_pm25_is_refused`, `an_oversized_photo_is_refused` (413), `a_png_is_refused`, `anonymous_photo_posts_are_refused_before_the_body`, `the_read_route_returns_null_without_a_reader`, `guests_cannot_see_other_peoples_photos` (404), `photos_are_never_cached`, `other_routes_keep_the_16_kb_limit`; in `tests/security.rs` update the CSP/Permissions-Policy expectations.
- [ ] **Step 2:** run — FAIL.
- [ ] **Step 3:** implement; service worker: never cache `/photos/` or `/log/pm25/read`. New keys `photo-invalid`, `photo-needs-pm25` in en/it/tr.
- [ ] **Step 4:** `cargo test --workspace` PASS.
- [ ] **Step 5:** Commit `feat(web): photo upload on Log with per-route body limits`.

### Task 7: Web — log form and entry page markup

**Files:** Modify `templates/log.html`, `templates/entry.html`, `routes/{log,entry_page}.rs`, `css/74-photo.css` (+ stylesheet list), locales. Tests: `crates/web/tests/{log,entries}.rs`.

- Log form: `enctype="multipart/form-data"`; in the PM2.5 card a button `data-pm25-photo` with `data-max-bytes="{{ max }}"` and `data-tz="{{ tz }}"`, a hidden `<input type="file" accept="image/*" capture="environment" name="photo" hidden>`, a preview area, and the hidden reference fields when present (a re-render shows a "Photo attached ✓" line with Remove; no thumbnail before saving).
- Entry page: photo card when `reading.photo` exists — `<img src="/photos/{reading_id}">` in a link to itself (tap to enlarge = open full image), "Taken {taken_at} · logged {recorded_at}" in the user's zone; 📷 chip on the PM2.5 line.
- Keys: `photo-take`, `photo-read-ok`, `photo-read-none`, `photo-retake`, `photo-remove`, `photo-attached`, `photo-card-title`, `photo-taken-logged`, `photo-use-phone-camera`, `photo-camera-hint`.

- [ ] **Step 1: Failing tests:** `the_log_form_offers_a_photo_with_the_limit` (button + `data-max-bytes="307200"`), `the_entry_page_shows_its_photo`, `an_entry_without_photo_has_no_photo_card`; both pages stay in `no_page_needs_inline_scripts_styles_or_eval`.
- [ ] **Step 2–4:** implement; PASS.
- [ ] **Step 5:** Commit `feat(web): photo button on the log form, photo card on the entry`.

### Task 8: Phone-side JS — camera, watermark, shrink, read

**Files:** Create `static/js/pm25-photo.js`, `static/js/pm25-camera.js`; register both in `assets.rs`; script tags in `log.html`. Test: `tests/e2e/specs/pm25-photo.spec.js` (fixture: a small synthetic JPEG generated in the test, never the user's photos).

Behaviour (spec §4–§5):
- Tap → `navigator.mediaDevices?.getSupportedConstraints?.().torch` true → open `pm25-camera.js` overlay (`getUserMedia({video:{facingMode:"environment"}})`, guide box, shaded top-left with live `dd.mm.yyyy HH:MM` in `data-tz`, shutter, close, ⚡ via `track.applyConstraints({advanced:[{torch}]})`, layout follows `screen.orientation`); on any start error show a "Use phone camera" button that clicks the file input. Else click the file input directly.
- After capture/selection: draw upright onto a canvas (`createImageBitmap(file, {imageOrientation:"from-image"})`), burn the watermark top-left (white text on `rgb(0 0 0 / 70%)` box, font ≈ 3.2 % of the shorter side), encode JPEG with the ladder: quality 0.85, 0.75, 0.65, 0.55 at longest side 2400 px; then the same at 2000, 1600, 1280 px; first result ≤ `data-max-bytes` wins.
- Put the result into the file input via `DataTransfer`, set hidden `photo_taken_at` (shutter time or `file.lastModified`), show preview (`URL.createObjectURL`) with Retake/Remove, `POST /log/pm25/read`; fill the PM2.5 field only if it is empty or was filled by a previous read, and mark it (`data-from-photo`, chip `photo-read-ok`); else show `photo-read-none`.
- Log button disabled after submit (double tap).

- [ ] **Step 1: Failing e2e:** `choosing_a_photo_previews_it_and_fills_nothing_without_a_reader`, `the_uploaded_photo_is_at_most_the_limit` (intercept the request, assert size ≤ 307200 for a 4000×3000 synthetic image), `remove_clears_the_photo`, `the_log_button_disables_on_submit`.
- [ ] **Step 2–4:** implement; `npx playwright test pm25-photo` PASS; `npx biome check` PASS.
- [ ] **Step 5:** Commit `feat(web): camera, watermark and size ladder on the phone`.

### Task 9: `verify-audit` checks the photo files

**Files:** Modify `crates/app/src/commands/verify_audit.rs`, `crates/app/tests/commands.rs`.

- When a photo store is configured and reachable, for each entry with a photo: `get(key)` → missing or sha256/length mismatch is reported as `photo file changed or missing: entry {id}`; and every key from `list_photos()` that no entry references is reported as `photo file without entry: {key}`. Without `PHOTO_BUCKET` in a prod run (or store unavailable) print `photo files not checked` and keep the exit code of the database check.
- [ ] **Step 1: Failing tests:** `verify_audit_reports_a_changed_photo_file`, `verify_audit_reports_a_missing_photo_file`, `verify_audit_reports_a_photo_without_entry`, `verify_audit_without_a_store_says_not_checked` (local store in a temp dir).
- [ ] **Step 2–4:** implement; PASS.
- [ ] **Step 5:** Commit `feat(app): verify-audit re-fingerprints photo files`.

### Task 10: Infrastructure and runbook

**Files:** Create `infra/modules/cloud-run-app/photos.tf`; modify `variables.tf`, `outputs.tf`, `tests/cloud_run_app.tftest.hcl`, `infra/prod/main.tf` (env `PHOTO_BUCKET`, `PM25_READER = "vision"`). Create `docs/runbooks/pm25-photo.md`.

- Bucket `${app}-photos-${project}` in `var.region`, `uniform_bucket_level_access = true`, `public_access_prevention = "enforced"`, one lifecycle rule: `Delete` when `age = 1` and `matches_prefix = ["pending/"]` (nothing else is ever deleted), soft delete default. IAM members: runtime SA `roles/storage.objectCreator` and `roles/storage.objectViewer` on that bucket only.
- Runbook (user, web console, work-PC rule): enable **Cloud Vision API** in the project; confirm the budget alert exists; deploy; test on Android + iPhone (torch detection, sideways); run `verify-audit`.
- [ ] **Step 1: Failing tftest:** bucket location `EUROPE-WEST3`, the only lifecycle rule deletes `pending/` after 1 day, public access prevention enforced, exactly the two roles for the runtime SA, no `storage.objectAdmin`.
- [ ] **Step 2–4:** `terraform -chdir=infra/modules/cloud-run-app test` PASS; `terraform fmt -check -recursive infra` PASS.
- [ ] **Step 5:** Commit `feat(infra): private photo bucket in Frankfurt`.

### Task 11: Size report for the user (no commit)

- Run the Task 8 ladder (Playwright page script) on `/Users/daniele/Downloads/IMG_20261013_205916.jpg` and `/Users/daniele/Downloads/IMG_20261016_202545.jpg`; write outputs to the session scratchpad only: for each ladder step that fits ≤ 300 KB, the file size, resolution, and a 1:1 crop of the digits. Show the user the crops next to the originals' crops; they decide whether 300 KB is enough (change of default needs their OK). Nothing from this task is committed.
