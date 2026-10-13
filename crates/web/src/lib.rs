//! The web layer of Smoke Tracker: Axum routes, Askama templates, Fluent translations and the
//! embedded static assets. It reaches the database only through the `smoke-domain` traits, so
//! every route works the same on SQLite and Postgres.

mod auth;
pub mod config;

pub use crate::auth::password;
