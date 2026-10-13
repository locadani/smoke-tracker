# Smoke Tracker

A small, installable web app (PWA) for logging cigarette smoke that drifts into an apartment
from neighbours. It records how strong the smell is, which rooms it reached, any symptoms, and
PM2.5 readings, and builds a detailed, tamper-evident record over time.

Written in Rust (Axum, Askama, htmx, SQLite).

> **Status:** early development. See the [roadmap](docs/plans/ROADMAP.md).

## Development

Requirements: [rustup](https://rustup.rs), [gitleaks](https://github.com/gitleaks/gitleaks)
(`brew install gitleaks`), Docker, and sqlx-cli:

```sh
git config core.hooksPath .githooks   # once per clone: blocks commits containing secrets
cargo install sqlx-cli --version 0.9.0 --no-default-features --features sqlite,postgres,rustls,sqlx-toml

# A throwaway local Postgres for the Postgres tests (no password, local only)
docker run -d --name smoke-pg -e POSTGRES_HOST_AUTH_METHOD=trust -p 5432:5432 postgres:17-alpine
export TEST_POSTGRES_URL=postgres://postgres@localhost:5432/postgres

cargo test --workspace         # Postgres tests are skipped when TEST_POSTGRES_URL is unset
scripts/sqlx-prepare.sh        # after changing a query or a migration; commit the .sqlx/ changes
```

Building needs no database: queries are checked against the committed `.sqlx/` metadata.

## Running it

Configuration comes from environment variables only; [`.env.example`](.env.example) lists
every one.

```sh
cp .env.example .env && mkdir -p data
set -a; . ./.env; set +a
cargo run -p smoke-tracker -- create-resident <username>   # prompts for the password
cargo run -p smoke-tracker -- serve                        # http://localhost:8080
cargo run -p smoke-tracker -- verify-audit                 # exits with 1 if the history was altered
```

### Container

Every push to `main` publishes `ghcr.io/locadani/smoke-tracker`: one static binary on a
distroless base, with the database in the `/data` volume.

```sh
docker build -t smoke-tracker .
docker volume create smoke-data
docker run --rm -it -v smoke-data:/data smoke-tracker create-resident <username>
docker run -d -p 8080:8080 -v smoke-data:/data -e APP_BASE_URL=http://localhost:8080 smoke-tracker
```

## Documentation

- [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
- [Decision log](docs/DECISIONS.md)
