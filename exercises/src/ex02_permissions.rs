//! Exercise 2: who may do what, as one function.
//!
//! Practises: `docs/learning/01-foundation-and-domain.md`, section 5.
//! Mirrors: `smoke_domain::permissions::{Actor, Action, can, can_see}`.
//! Run: `cargo test --test ex02_permissions`
//!
//! Hint: `match actor.role { ... }` with an inner `match action { ... }` per role, no `_`
//! catch-all on the roles so adding a role later is a compile error. `Option::is_some_and`
//! turns "has an expiry and it has passed" into one expression. Times are plain seconds.
//!
//! The rules (spec section 4):
//! - A resident may do everything and never expires.
//! - A guest may always view their own data; log entries, and edit entries or manage events
//!   they own, only while not expired. A guest may never view all, manage users or edit lists.
//! - A viewer may view everything and their own data while not expired, and nothing else.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Resident,
    Guest,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Actor {
    pub id: u32,
    pub role: Role,
    /// The second at which the account stops working; `None` means never.
    pub expires_at: Option<u64>,
}

impl Actor {
    /// Expired once `now` reaches `expires_at`.
    pub fn is_expired(&self, now: u64) -> bool {
        todo!()
    }

    /// Everyone may sign in except an expired viewer (an expired guest keeps read access to
    /// their own data).
    pub fn can_sign_in(&self, now: u64) -> bool {
        todo!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    LogEntry,
    ViewAll,
    ViewOwn,
    EditEntry { owner: u32 },
    ManageEvent { owner: u32 },
    ManageUsers,
    EditLists,
}

pub fn can(actor: &Actor, action: Action, now: u64) -> bool {
    todo!()
}

/// Whether `actor` may see something logged by `owner`: everything with `ViewAll`, otherwise
/// only their own.
pub fn can_see(actor: &Actor, owner: u32, now: u64) -> bool {
    todo!()
}
