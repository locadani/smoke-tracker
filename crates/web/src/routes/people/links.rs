//! Inviting someone and resetting a password (spec §8.3): the resident's side. The app shows a
//! QR code for `{APP_BASE_URL}/invite/{token}`; the token exists only in that page, the database
//! keeps its hash.

use askama::Template;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Response;
use chrono::{DateTime, Utc};
use smoke_domain::DomainError;
use smoke_domain::people::{NewPerson, PeopleError, invite, issue_link};
use smoke_domain::records::{Invite, User};
use smoke_domain::repos::{Commit, Store};
use smoke_domain::types::{InvitePurpose, Role, TokenHash, UserId, Username};

use crate::auth::CurrentUser;
use crate::error::{AppError, page};
use crate::forms::Pairs;
use crate::header::{SignedIn, Tab};
use crate::i18n::Tr;
use crate::routes::people::expiry::{self, ExpiryDraft, ExpiryFields};
use crate::routes::people::{require_manager, role_key};
use crate::routes::people_refusal;
use crate::state::AppState;
use crate::{qr, time};

/// The roles a resident can give, guest first: the usual case (spec §8.3).
const ROLES: [Role; 3] = [Role::Guest, Role::Viewer, Role::Resident];

/// A new link's token: 32 random bytes, as 64 hex characters.
fn new_token() -> Result<String, AppError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| AppError::internal(e.to_string()))?;
    Ok(hex::encode(bytes))
}

pub(crate) struct RoleChoice {
    pub(crate) value: &'static str,
    pub(crate) key: &'static str,
    pub(crate) checked: bool,
}

/// The invite form as typed, to show it again after a refusal.
#[derive(Debug, Clone, Default)]
struct InviteDraft {
    username: String,
    role: Option<Role>,
    guest: ExpiryDraft,
    viewer: ExpiryDraft,
}

impl InviteDraft {
    fn from_form(form: &Pairs) -> Self {
        Self {
            username: form.get("username").unwrap_or_default().to_owned(),
            role: form.get("role").and_then(|role| role.parse().ok()),
            guest: ExpiryDraft::from_form(form, "guest_"),
            viewer: ExpiryDraft::from_form(form, "viewer_"),
        }
    }
}

#[derive(Template)]
#[template(path = "invite.html")]
struct InvitePage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    error: Option<&'static str>,
    username: String,
    roles: Vec<RoleChoice>,
    guest: Option<ExpiryFields>,
    viewer: Option<ExpiryFields>,
}

fn invite_page(
    status: StatusCode,
    user: &User,
    tr: Tr,
    now: DateTime<Utc>,
    draft: InviteDraft,
    error: Option<&'static str>,
) -> Result<Response, AppError> {
    let role = draft.role.unwrap_or(Role::Guest);
    page(
        status,
        &InvitePage {
            tr,
            signed_in: SignedIn::of(user, now, Tab::People),
            error,
            username: draft.username,
            roles: ROLES
                .iter()
                .map(|&choice| RoleChoice {
                    value: choice.as_str(),
                    key: role_key(choice),
                    checked: choice == role,
                })
                .collect(),
            guest: expiry::fields(tr, Role::Guest, "guest_", &draft.guest),
            viewer: expiry::fields(tr, Role::Viewer, "viewer_", &draft.viewer),
        },
    )
}

pub(crate) async fn form<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    require_manager(&user, now)?;
    invite_page(StatusCode::OK, &user, tr, now, InviteDraft::default(), None)
}

#[derive(Template)]
#[template(path = "link.html")]
struct LinkPage {
    tr: Tr,
    signed_in: Option<SignedIn>,
    username: String,
    setup: bool,
    url: String,
    qr: String,
    until: String,
}

/// The QR code page. Rendered in the response to the post that made the link, never stored,
/// so reloading it cannot show the link again.
fn link_page<S>(
    state: &AppState<S>,
    user: &User,
    tr: Tr,
    person: &User,
    link: &Invite,
    token: &str,
) -> Result<Response, AppError> {
    let url = format!("{}/invite/{token}", state.config.base_url.origin());
    let now = state.clock.now();
    page(
        StatusCode::OK,
        &LinkPage {
            tr,
            signed_in: SignedIn::of(user, now, Tab::People),
            username: person.username.to_string(),
            setup: link.purpose == InvitePurpose::Setup,
            qr: qr::svg(&url),
            url,
            until: time::clock(link.expires_at, state.config.schedule.tz()),
        },
    )
}

/// Creates the account and shows its setup QR code (spec §8.3 steps 1–2).
pub(crate) async fn create<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    form: Pairs,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    require_manager(&user, now)?;
    let draft = InviteDraft::from_form(&form);
    let refuse = |draft: InviteDraft, key: &'static str| {
        invite_page(
            StatusCode::UNPROCESSABLE_ENTITY,
            &user,
            tr,
            now,
            draft,
            Some(key),
        )
    };
    let Ok(username) = Username::new(&draft.username) else {
        return refuse(draft, "people-error-username");
    };
    let Some(role) = draft.role else {
        return refuse(draft, "log-error-form");
    };
    let typed = match role {
        Role::Viewer => &draft.viewer,
        _ => &draft.guest,
    };
    let pick = match expiry::read(role, typed) {
        Ok(pick) => pick,
        Err(error) => return refuse(draft, error.key()),
    };
    let person = NewPerson {
        username,
        role,
        locale: user.locale,
        expires_at: pick.resolve(now, &state.config.schedule),
    };
    let token = new_token()?;
    let mut tx = state.store.begin(Some(user.id)).await?;
    let (invited, link) =
        match invite(&mut tx, &user.actor(), person, TokenHash::of(&token), now).await {
            Ok(created) => created,
            Err(error) => {
                drop(tx);
                let key = match error {
                    PeopleError::UsernameTaken => "people-error-username-taken",
                    PeopleError::Invalid(DomainError::ExpiryNotInFuture) => "people-error-past",
                    other => return Err(people_refusal(other)),
                };
                return refuse(draft, key);
            }
        };
    tx.commit().await?;
    tracing::info!(user = invited.id.0, role = %invited.role, "invited");
    link_page(&state, &user, tr, &invited, &link, &token)
}

/// A new setup or password-reset QR code for an existing account (spec §8.3).
pub(crate) async fn renew<S: Store + Clone + 'static>(
    State(state): State<AppState<S>>,
    CurrentUser(user): CurrentUser,
    tr: Tr,
    Path(id): Path<i64>,
) -> Result<Response, AppError> {
    let now = state.clock.now();
    let token = new_token()?;
    let mut tx = state.store.begin(Some(user.id)).await?;
    let (person, link) = issue_link(
        &mut tx,
        &user.actor(),
        UserId(id),
        TokenHash::of(&token),
        now,
    )
    .await
    .map_err(people_refusal)?;
    tx.commit().await?;
    link_page(&state, &user, tr, &person, &link, &token)
}

/// Every key this module passes through a variable, for the translation tests.
#[cfg(test)]
pub(crate) const KEYS: [&str; 3] = [
    "people-error-username",
    "people-error-username-taken",
    "log-error-form",
];
