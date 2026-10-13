//! The auto-close notice (spec §7.4): "The event from [date time] was auto-closed."

use axum::extract::State;
use axum::response::Redirect;
use chrono::{DateTime, Utc};
use smoke_domain::events::auto_close_notice_is_for;
use smoke_domain::records::{Event, User};
use smoke_domain::repos::{Commit, EventWrites, Store};

use crate::auth::CurrentUser;
use crate::error::AppError;
use crate::state::AppState;

/// The unseen auto-close notices meant for `user`, oldest first: only the last one is offered,
/// and answering it marks them all seen.
pub(crate) async fn pending<S: Store>(
    state: &AppState<S>,
    user: &User,
    now: DateTime<Utc>,
) -> Result<Vec<Event>, AppError> {
    let viewer = user.actor();
    let mut pending = Vec::new();
    for event in state.store.unseen_auto_closed().await? {
        let owner = if event.owner_id == user.id {
            Some(user.clone())
        } else {
            state.store.user(event.owner_id).await?
        };
        if owner.is_some_and(|owner| auto_close_notice_is_for(&viewer, &owner.actor(), now)) {
            pending.push(event);
        }
    }
    Ok(pending)
}

/// Marks the notices seen, inside the caller's transaction.
pub(crate) async fn mark_seen<T: EventWrites>(
    tx: &mut T,
    notices: &[Event],
) -> Result<(), AppError> {
    for notice in notices {
        tx.mark_auto_close_notice_seen(notice.id).await?;
    }
    Ok(())
}

/// "Dismiss" on the banner shown while another event is open.
pub(crate) async fn dismiss<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
) -> Result<Redirect, AppError> {
    let notices = pending(&state, &user, state.clock.now()).await?;
    if !notices.is_empty() {
        let mut tx = state.store.begin(Some(user.id)).await?;
        mark_seen(&mut tx, &notices).await?;
        tx.commit().await?;
    }
    Ok(Redirect::to("/"))
}
