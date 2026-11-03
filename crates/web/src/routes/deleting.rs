//! The routes that delete: an entry from its page. The confirmation step is the entry page
//! itself, opened with `?delete`.

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Redirect, Response};
use smoke_domain::deleting::delete_entry as delete_entry_use_case;
use smoke_domain::records::report_views;
use smoke_domain::repos::{Commit, Store};
use smoke_domain::sweep::auto_close;
use smoke_domain::types::LogEntryId;

use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::routes::{can_open, refusal};
use crate::state::AppState;

/// Deletes the entry, then lands on its event if it still has one the user may open, otherwise
/// on History.
pub(crate) async fn delete_entry<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    auto_close(&state.store, &state.config.schedule, now).await?;
    let actor = user.actor();
    let mut tx = state.store.begin(Some(user.id)).await?;
    let event = match delete_entry_use_case(&mut tx, &actor, LogEntryId(id), now).await {
        Ok(event) => event,
        Err(error) => {
            drop(tx);
            return Err(refusal(error).err().unwrap_or(AppError::NotAllowed));
        }
    };
    tx.commit().await?;
    if let Some(id) = event
        && let Some(event) = state.store.event(id).await?
    {
        let entries = state.store.entries_for_event(id).await?;
        if can_open(&actor, &event, &report_views(&entries), now) {
            return Ok(Redirect::to(&format!("/events/{id}")).into_response());
        }
    }
    Ok(Redirect::to("/events").into_response())
}
