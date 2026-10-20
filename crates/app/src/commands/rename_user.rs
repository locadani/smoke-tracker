//! `rename-user <current> <new>`: fixes a username through an audited write. Editing the
//! `users` table by hand instead makes `verify-audit` flag the account (issue #35).

use anyhow::{anyhow, bail};
use smoke_domain::records::User;
use smoke_domain::repos::{Commit, Transactional, UserRepo, UserWrites};
use smoke_domain::types::Username;

/// Renames the user as the system (no actor). Sessions stay valid: they follow the user's id.
pub async fn rename<S>(store: &S, current: &Username, new: Username) -> anyhow::Result<User>
where
    S: UserRepo + Transactional<Tx: UserWrites>,
{
    if *current == new {
        bail!("{current} already has that username");
    }
    let user = store
        .user_by_username(current)
        .await?
        .ok_or_else(|| anyhow!("no user is called {current}"))?;
    let mut tx = store.begin(None).await?;
    let renamed = tx.set_username(user.id, new).await?;
    tx.commit().await?;
    Ok(renamed)
}
