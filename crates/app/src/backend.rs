//! The storage backend, picked once from `DATABASE_URL` (spec §10.1).
//!
//! Commands are generic over [`Store`](smoke_domain::repos::Store); [`with_store!`] runs one
//! with whichever backend is configured, so the compiler builds a copy of it per backend and no
//! call goes through a trait object.

use std::sync::Arc;

use anyhow::Context;
use smoke_domain::clock::Clock;
use smoke_domain::repos::StoreResult;
use smoke_domain::starter::seed_starter_lists;
use smoke_storage_postgres::PostgresStore;
use smoke_storage_sqlite::SqliteStore;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Sqlite,
    Postgres,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("DATABASE_URL must start with sqlite: or postgres://")]
pub struct UnknownBackend;

impl BackendKind {
    pub fn of(url: &str) -> Result<Self, UnknownBackend> {
        let url = url.trim();
        if url.starts_with("sqlite:") {
            Ok(Self::Sqlite)
        } else if url.starts_with("postgres://") || url.starts_with("postgresql://") {
            Ok(Self::Postgres)
        } else {
            Err(UnknownBackend)
        }
    }
}

pub enum Backend {
    Sqlite(SqliteStore),
    Postgres(PostgresStore),
}

impl Backend {
    pub async fn connect(url: &str, clock: Arc<dyn Clock>) -> anyhow::Result<Self> {
        let url = url.trim();
        Ok(match BackendKind::of(url)? {
            BackendKind::Sqlite => Self::Sqlite(
                SqliteStore::connect(url, clock)
                    .await
                    .context("could not open the SQLite database")?,
            ),
            BackendKind::Postgres => Self::Postgres(
                PostgresStore::connect(url, clock)
                    .await
                    .context("could not connect to Postgres")?,
            ),
        })
    }

    pub async fn migrate(&self) -> StoreResult<()> {
        match self {
            Self::Sqlite(store) => store.migrate().await,
            Self::Postgres(store) => store.migrate().await,
        }
    }

    /// Migrations, then the starter rooms and symptoms if the lists are empty. Safe to repeat.
    pub async fn prepare(&self) -> anyhow::Result<()> {
        self.migrate().await.context("migrations failed")?;
        with_store!(self, |store| seed_starter_lists(store).await)
            .context("seeding the starter lists failed")?;
        Ok(())
    }
}

/// `with_store!(backend, |store| expr)` evaluates `expr` with `store` bound to the concrete
/// store inside `backend` (by value, or by reference for `&backend`).
macro_rules! with_store {
    ($backend:expr, |$store:ident| $body:expr) => {
        match $backend {
            $crate::backend::Backend::Sqlite($store) => $body,
            $crate::backend::Backend::Postgres($store) => $body,
        }
    };
}
pub(crate) use with_store;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_picks_the_backend() {
        assert_eq!(
            BackendKind::of("sqlite://data/smoke.db"),
            Ok(BackendKind::Sqlite)
        );
        assert_eq!(BackendKind::of("sqlite:smoke.db"), Ok(BackendKind::Sqlite));
        assert_eq!(
            BackendKind::of("postgres://app@db.example/smoke?sslmode=require"),
            Ok(BackendKind::Postgres)
        );
        assert_eq!(
            BackendKind::of("postgresql://app@db/smoke"),
            Ok(BackendKind::Postgres)
        );
        for url in ["mysql://db/smoke", "data/smoke.db", ""] {
            assert_eq!(BackendKind::of(url), Err(UnknownBackend), "{url}");
        }
    }
}
