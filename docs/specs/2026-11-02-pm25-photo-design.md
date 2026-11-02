# PM2.5 photo — design

**Status:** approved · 2026-11-02
**Builds on:** the log entries design (`2026-10-19-log-entries-design.md`), the hosting design
(`2026-10-13-hosting-infra-design.md`) and decisions #25, #47, #48, #76, #84.

## 1. Why

People in the household log PM2.5 several times a day (3–4) from an IKEA ALPSTUGA sensor, and
need to record bad episodes **now**. Typing the value works, but there is no proof of what the
sensor showed and when. A photo of the sensor, kept with the reading, is that proof; reading the
number off the photo saves typing it.

The photo is **evidence**, not a second source of truth: the confirmed `value_ugm3` stays the
only PM2.5 value in the app.

## 2. Goals, non-goals, anti-goals

- **Goals**
  - From the log form, take a photo of the sensor; the app reads the PM2.5 number and pre-fills
    the field; the person confirms or edits it, then presses Log as today.
  - The photo is saved with the reading and shown on the entry page.
  - Date and time are burned into the photo's top-left corner (watermark).
  - The photo is tamper-evident like the rest of the data (decision #25).
  - Works on Android and iPhone, upright and sideways.
- **Non-goals (this version)**
  - Our own Rust digit reader. It **will** come later and replace Google Cloud Vision, so the
    reader sits behind its own seam (§6).
  - Removing or replacing a photo on an already saved reading.
  - Photos for smell reports or entries without a PM2.5 value.
  - Reading anything other than PM2.5 (the sensor is always on its PM2.5 screen).
- **Anti-goals**
  - Storing what the reader detected: there is **one** value, the confirmed one.
  - Any API key, token or secret in the repo or on the work PC; any paid tier.
  - Photos inside Postgres (they would be copied into every nightly dump, ×30 kept).
  - Losing an entry because the reader or the bucket is down when it doesn't need to be (§7).

## 3. The sensor (what the reader sees)

ALPSTUGA: white dot-matrix LEDs behind a white front, label "PM2.5" on the left, the number on
the right, sometimes an orange air-quality dot. In dim mode the camera blurs the dots into solid
strokes. Photos are taken close-up with nothing else beside the sensor; the number is the one
next to "PM2.5", or the only number in the picture. Sample photos (personal, never
committed; readable at these paths): `/Users/daniele/Downloads/IMG_20261013_205916.jpg` ("14",
dim) and `/Users/daniele/Downloads/IMG_20261016_202545.jpg` ("19"). They include a phone
showing the clock; real use won't.

## 4. User flow (mockups: `.superpowers/brainstorm/…/pm-photo-flow.html`, `pm-photo-landscape.html`)

1. **Log form:** inside the existing PM2.5 card, a button "📷 Take photo of sensor". Typing the
   value by hand is unchanged.
2. **Choosing the camera:** on tap, the page checks synchronously whether the browser supports
   torch control (`navigator.mediaDevices.getSupportedConstraints().torch`).
   - **Supported (Android Chrome):** our own full-screen camera: live viewfinder, a guide box,
     the top-left corner shaded with the live date and time ("timestamp goes here"), shutter,
     close, and a ⚡ torch toggle. Rotates with the phone (controls on the side when sideways).
   - **Not supported (iPhone Safari):** the phone's normal camera app opens
     (`<input type="file" accept="image/*" capture="environment">`), which has its own flash.
   - **Our camera fails to start** (permission denied, no camera): it shows a "Use phone
     camera" button (a browser only opens the camera app from a tap).
   - To verify on both phones: that iPhone Safari does not report torch support.
3. **After the shot (both paths):** the phone burns the watermark into the top-left of the
   upright image, shrinks it to at most the size limit (§5), shows a preview with **Retake** and
   **Remove**, and sends it to the reader. The PM2.5 field is pre-filled and highlighted with
   "Read from photo, check it". If nothing could be read: "Couldn't read the number, please type
   it"; the photo stays attached.
4. **Log:** the form is sent as `multipart/form-data` with the photo (the page keeps the
   processed photo in memory and re-attaches it if the form comes back with an error). The Log
   button is disabled while sending, so a double tap can't create two entries; no server-side
   dedupe *(builder's call)*. A new photo without a PM2.5 value is refused with a clear message.
5. **Entry page:** a photo card (only when there is one): the photo (tap to enlarge),
   "Taken 14:35 · logged 14:36" (taken = phone clock, logged = the entry's server `recorded_at`). The PM2.5 line gets a 📷 chip.

All new text in English, Italian and Turkish (the three locales).

## 5. The photo

- **Watermark:** `dd.mm.yyyy HH:MM` in the person's time zone as the app already uses it, white on a dark box, top-left of
  the photo as it is viewed (EXIF orientation applied first). Time = when the photo was taken
  (our camera: the shutter moment; camera app: the file's `lastModified`, else now).
- **Size limit:** default **400 KB** (user, 2026-11-02: keeps JPEG quality 0.85 at 2400 px), set by one configuration value (env var, e.g.
  `PHOTO_MAX_BYTES`), changeable in the deploy config without a code change. The server injects the value into the
  log page (e.g. a data attribute), so phone and server always agree. The phone encodes
  JPEG, lowering quality and then resolution step by step until it fits. The server refuses
  anything above the limit. *Builder's job:* try the step ladder on the two sample photos and
  report which resolution/quality lands under 300 KB, with side-by-side crops of the digits, so
  the user can judge the quality.
- **Re-encoding drops EXIF** (no GPS or device data is stored), which is fine: the watermark and
  `taken_at` carry the time.

## 6. Reading the number (Google Cloud Vision now, Rust later)

- A **reader port** (a trait, e.g. `Pm25Reader`: image bytes → `Option<f64>`) with:
  - `CloudVisionReader`: calls Vision `TEXT_DETECTION` at the **EU endpoint**
    (`eu-vision.googleapis.com`) so images stay in Europe; authenticates as the Cloud Run
    service account via the metadata server (no key). Picks the number to the right of the
    "PM2.5" text, else the only number; anything else → `None`.
  - `NoReader` (local development and tests): always `None`, so local runs need no Google
    access.
  - Later: a pure-Rust dot-matrix reader replaces `CloudVisionReader` behind the same port.
- A small endpoint (e.g. `POST /log/pm25/read`) takes the photo and returns the number or
  nothing. It **stores nothing**; the photo is sent again with the form (up to 400 KB, negligible).
- Free tier: 1,000 Vision requests/month; expected ~120.

## 7. Storage

- **Bucket:** a new private bucket in **europe-west3** (the user chose Frankfurt over the free US
  tier: about a cent a year), uniform access, no public access, managed in Terraform next to the
  backups bucket. The Cloud Run service account gets create + read on it, **not** delete or
  overwrite. Object names are the photo's fingerprint (`pending/<sha256>.jpg`,
  `photos/YYYY/MM/<sha256>.jpg`): never guessable without the photo, and the same photo always
  gets the same name. A bucket **lifecycle rule deletes everything under `pending/` after
  1 day**.
- **Photo store port** (trait): `GcsPhotoStore` (JSON API, metadata-server auth) and
  `LocalPhotoStore` (a folder, for development and tests).
- **Serving:** the app streams the photo from the bucket at an authenticated route that
  follows exactly the entry page's access rules (whoever can open the entry sees its photo),
  with `Cache-Control: private, no-store`; the bucket is never exposed.
- **Outbound HTTP:** one small client (`reqwest` with rustls, minimal features). Google auth
  and the Vision/Storage calls are hand-written (metadata-server token + JSON), no Google SDK
  crates: smaller binary, fewer dependencies, more Rust learned *(builder's call)*.
- **Consistency (user's choice, 2026-11-02: option 1, database is the truth).** No transaction
  can span the bucket and the database, so the order guarantees that **an entry never points to
  a missing or different photo**, and anything left over cleans itself up:
  1. First press of Log: the photo is validated and stored at `pending/<sha256>.jpg`. The
     "which event?" question and error re-renders carry only a reference (hidden fields).
  2. Final save: the server copies it server-side to `photos/YYYY/MM/<sha256>.jpg`, re-reads
     it and checks length + fingerprint, and only then commits the reading + photo row.
  3. Walked away at step 1: the pending file is deleted by the lifecycle rule within a day.
  4. Database failure after step 2: pressing Log again reuses the same file (same name), so no
     duplicate. `verify-audit` reports any permanent photo that no entry points to.
  If storing fails, nothing is written and the form comes back with the value kept and an
  error.

## 8. Database (migration 0006, SQLite and Postgres, kept in step as today)

```
pm25_photos
  id               primary key (assigned like other evidence rows)
  pm25_reading_id  → pm25_readings.id, UNIQUE NOT NULL   (one photo per reading)
  object_key       TEXT NOT NULL   where the photo is in the bucket
  bytes            INTEGER NOT NULL
  sha256           TEXT NOT NULL   fingerprint of the exact stored bytes
  taken_at         BIGINT NOT NULL phone clock (same as the watermark), µs since epoch (#46)
```

- No upload time is stored (user, 2026-11-02): the entry's `recorded_at` is the server time.
- No PM2.5 number in this table: `pm25_readings.value_ugm3` stays the single source of truth.
- The photo is saved in the **same transaction** as its log entry, and the photo's details
  (`sha256`, `object_key`, `bytes`, `taken_at`) are part of the **log entry's
  audit snapshot** (#147: one audit row per entry, the whole log as snapshot). The entry's hash
  therefore covers the photo's fingerprint: the photo is provably linked to that entry, and
  changing the file, the fingerprint or the link breaks verification.
- For entries without a photo the field is **omitted** from the snapshot (not written as
  `null`), so existing snapshots and their hashes stay byte-identical (#104).
- `verify-audit` already recomputes each entry's snapshot from the tables; it now includes the
  photo row, so a changed, relinked or deleted `pm25_photos` row is caught by the existing path.
  Contract tests cover those three cases.
- `verify-audit` additionally re-fingerprints each photo file when it has read access to the
  photo store; without access it says "photo files not checked" instead of failing. Running it
  in the cloud (a Cloud Run job, like the backup) is a follow-up, not this version.
- Photos are never replaced; when an entry is deleted (soft delete), the photo row and file stay.

## 9. Security and limits

- `Permissions-Policy`: `camera=(self)` instead of `camera=()`.
- CSP: `img-src 'self' blob:` for the preview; nothing else changes.
- The global 16 KB body limit stays; only the read and log routes accept up to the photo limit
  plus form overhead (derived from the same setting). Multipart parsing needs axum's
  `multipart` feature; CSRF protection works as on the other forms.
- On both photo routes the sign-in check runs **before** the body is read, so anonymous
  requests are refused without accepting the larger upload.
- The server checks the upload is a JPEG (magic bytes) within the limit before reading or
  storing it.
- The service worker must not cache photo routes.

## 10. Testing

- Unit: reader number-picking (from mocked Vision responses: label + number, only number, time
  text, nothing), size-limit refusal, "photo without value" refusal.
- Storage contract tests run against both backends, as today; `LocalPhotoStore` in web tests.
- `verify-audit`: detects a changed and a missing photo.
- Manual on real phones: Android (our camera, torch, sideways) and iPhone (camera app path).
- The size-ladder report from §5, before merge.

## 11. Left to the builder

Exact route names, module layout, the JPEG step ladder, how the watermark is drawn (canvas),
Vision response parsing, and how `verify-audit` gets bucket access (read-only).
