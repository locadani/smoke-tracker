//! `create-resident <username>`: the bootstrap account (spec §8.2). No default credentials
//! exist anywhere; the password is typed at a prompt that does not echo.

use anyhow::bail;
use smoke_domain::records::{NewUser, User};
use smoke_domain::repos::{Commit, Transactional, UserWrites};
use smoke_domain::types::{Locale, NewPassword, Role, Username};
use smoke_web::password;

/// Asks twice, so a typo cannot lock the only resident out.
pub fn prompt_password() -> anyhow::Result<NewPassword> {
    let first = rpassword::prompt_password("Password (10–128 characters): ")?;
    let again = rpassword::prompt_password("Repeat the password: ")?;
    if first != again {
        bail!("the passwords do not match");
    }
    Ok(NewPassword::new(first)?)
}

/// Creates the resident in one audited transaction, as the system (no actor).
pub async fn create<S>(
    store: &S,
    username: Username,
    password: NewPassword,
    locale: Locale,
) -> anyhow::Result<User>
where
    S: Transactional<Tx: UserWrites>,
{
    let hash = tokio::task::spawn_blocking(move || password::hash(&password)).await??;
    let mut tx = store.begin(None).await?;
    let user = tx
        .create_user(NewUser {
            username,
            password_hash: Some(hash),
            role: Role::Resident,
            locale,
            expires_at: None,
        })
        .await?;
    tx.commit().await?;
    Ok(user)
}
