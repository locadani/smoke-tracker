//! Sensor photos: reading the number from one, carrying an uploaded photo through the Log form,
//! and serving it back to whoever may open its entry (photo spec §4, §7, §9).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE};
use axum::response::{IntoResponse, Response};
use chrono::{DateTime, Utc};
use smoke_domain::permissions::{Action, can, can_see};
use smoke_domain::records::{NewReading, Pm25Photo, Sha256Hex, User};
use smoke_domain::repos::Store;
use smoke_domain::types::{Pm25, ReadingId};
use smoke_photos::{PhotoError, PhotoStore, validate};

use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::forms::{Pairs, Upload};
use crate::i18n::Tr;
use crate::routes::entries::back_to_form;
use crate::state::AppState;

/// The hidden fields that carry an uploaded photo through the question page and error re-renders.
const FIELDS: [&str; 3] = ["photo_sha256", "photo_bytes", "photo_taken_at"];

/// An uploaded photo waiting in the store, named by the hidden fields.
#[derive(Debug, Clone)]
pub(crate) struct PhotoRef {
    sha256: Sha256Hex,
    bytes: u32,
    taken_at: DateTime<Utc>,
}

/// Why the photo of a post cannot be used.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Problem {
    /// Not a JPEG, not what the page uploaded, or gone from the store: the person must retake it.
    Invalid,
    /// Beyond the size limit.
    TooLarge,
    /// The photo store did not answer: nothing was written, trying again may work.
    Unavailable,
}

/// The photo fields of `form`, when all three are there, to be carried on as hidden inputs.
pub(crate) fn carried_fields(form: &Pairs) -> Vec<(&'static str, String)> {
    let values: Option<Vec<_>> = FIELDS.iter().map(|name| form.get(name)).collect();
    values
        .map(|values| {
            FIELDS
                .into_iter()
                .zip(values.into_iter().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// Reads the photo of a Log post. A new file is validated and stored as pending, and the form
/// then carries it by reference; a reference alone is parsed strictly. A new file wins over a
/// reference. `Ok(None)` means no photo at all.
pub(crate) async fn intake(
    store: &PhotoStore,
    max_bytes: usize,
    form: &mut Pairs,
    file: Option<Vec<u8>>,
) -> Result<Option<PhotoRef>, Problem> {
    let Some(file) = file else {
        return reference(form, max_bytes);
    };
    let taken_at = form
        .get("photo_taken_at")
        .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
        .map(|at| at.with_timezone(&Utc))
        .ok_or(Problem::Invalid)?;
    let photo = validate(file, max_bytes).map_err(|error| match error {
        PhotoError::TooLarge => Problem::TooLarge,
        _ => Problem::Invalid,
    })?;
    store.put_pending(&photo).await.map_err(unavailable)?;
    let bytes = u32::try_from(photo.bytes.len()).map_err(|_| Problem::TooLarge)?;
    form.set("photo_sha256", photo.sha256.as_str().to_owned());
    form.set("photo_bytes", bytes.to_string());
    form.set("photo_taken_at", taken_at.to_rfc3339());
    Ok(Some(PhotoRef {
        sha256: photo.sha256,
        bytes,
        taken_at,
    }))
}

fn reference(form: &Pairs, max_bytes: usize) -> Result<Option<PhotoRef>, Problem> {
    let [sha256, bytes, taken_at] = FIELDS.map(|name| form.get(name));
    if (sha256, bytes, taken_at) == (None, None, None) {
        return Ok(None);
    }
    let (Some(sha256), Some(bytes), Some(taken_at)) = (sha256, bytes, taken_at) else {
        return Err(Problem::Invalid);
    };
    let sha256 = Sha256Hex::parse(sha256).map_err(|_| Problem::Invalid)?;
    let bytes: u32 = bytes.parse().map_err(|_| Problem::Invalid)?;
    if bytes == 0 || usize::try_from(bytes).is_ok_and(|n| n > max_bytes) {
        return Err(Problem::Invalid);
    }
    let taken_at = DateTime::parse_from_rfc3339(taken_at)
        .map_err(|_| Problem::Invalid)?
        .with_timezone(&Utc);
    Ok(Some(PhotoRef {
        sha256,
        bytes,
        taken_at,
    }))
}

/// The reading to save: `value` alone, or with its photo once the store has copied it to its
/// permanent name and checked length and fingerprint. Only after this returns `Ok` may the
/// entry be written, so an entry never points at a missing or different photo.
pub(crate) async fn finalise<S>(
    state: &AppState<S>,
    photo: Option<&PhotoRef>,
    value: Pm25,
    now: DateTime<Utc>,
) -> Result<NewReading, Problem> {
    let Some(photo) = photo else {
        return Ok(NewReading::without_photo(value));
    };
    let key = state
        .photos
        .promote(&photo.sha256, photo.bytes, now)
        .await
        .map_err(|error| match error {
            PhotoError::Missing | PhotoError::Mismatch => Problem::Invalid,
            other => unavailable(other),
        })?;
    Ok(NewReading {
        value,
        photo: Some(Pm25Photo {
            key,
            bytes: photo.bytes,
            sha256: photo.sha256.clone(),
            taken_at: photo.taken_at,
        }),
    })
}

fn unavailable(error: PhotoError) -> Problem {
    tracing::warn!(%error, "photo store failed");
    Problem::Unavailable
}

/// What a photo problem becomes: a refusal with the form back, or 413 for an oversized file.
pub(crate) async fn refuse<S: Store>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    form: &Pairs,
    problem: Problem,
) -> Result<Response, AppError> {
    match problem {
        Problem::TooLarge => Ok(StatusCode::PAYLOAD_TOO_LARGE.into_response()),
        Problem::Invalid => {
            let form = form.clone().omitting(&FIELDS);
            back_to_form(state, user, tr, now, &form, "photo-invalid").await
        }
        Problem::Unavailable => {
            back_to_form(state, user, tr, now, form, "photo-store-unavailable").await
        }
    }
}

/// `POST /log/pm25/read`: the number the reader finds in an uploaded photo, as JSON. Nothing is
/// stored.
pub(crate) async fn read<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    upload: Upload,
) -> Result<Response, AppError> {
    if !can(&user.actor(), Action::LogEntry, state.clock.now()) {
        return Err(AppError::NotAllowed);
    }
    let Some(file) = upload.photo else {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    };
    let photo = match validate(file, state.config.photo_max_bytes) {
        Ok(photo) => photo,
        Err(PhotoError::TooLarge) => return Ok(StatusCode::PAYLOAD_TOO_LARGE.into_response()),
        Err(_) => return Ok(StatusCode::BAD_REQUEST.into_response()),
    };
    let value = state.reader.read(&photo.bytes).await;
    let body = serde_json::json!({ "value": value }).to_string();
    Ok(([(CONTENT_TYPE, "application/json")], body).into_response())
}

/// `GET /photos/{reading_id}`: the photo, for whoever may open the entry that owns it.
pub(crate) async fn show<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let key = state
        .store
        .entry_of_reading(ReadingId(id))
        .await?
        .filter(|entry| !entry.is_deleted() && can_see(&user.actor(), entry.user_id, now))
        .and_then(|entry| entry.reading?.photo)
        .map(|photo| photo.key)
        .ok_or(AppError::NotFound)?;
    match state.photos.get(&key).await {
        Ok(Some(bytes)) => Ok((
            [
                (CONTENT_TYPE, "image/jpeg"),
                (CACHE_CONTROL, "private, no-store"),
            ],
            bytes,
        )
            .into_response()),
        Ok(None) => Err(AppError::NotFound),
        Err(error) => {
            tracing::warn!(%error, key = key.as_str(), "could not read a stored photo");
            Err(AppError::NotFound)
        }
    }
}
