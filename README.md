# Smoke Tracker

A small, installable web app (PWA) for logging cigarette smoke that drifts into an apartment
from neighbours. It records how strong the smell is, which rooms it reached, any symptoms, and
PM2.5 readings, and builds a detailed, tamper-evident record over time.

Written in Rust (Axum, Askama, htmx, SQLite).

> **Status:** early development. See the [roadmap](docs/plans/ROADMAP.md).

## Development

Requirements: [rustup](https://rustup.rs) and [gitleaks](https://github.com/gitleaks/gitleaks)
(`brew install gitleaks`).

```sh
git config core.hooksPath .githooks   # once per clone: blocks commits containing secrets
cargo test --workspace
```

## Documentation

- [Design spec](docs/specs/2026-10-13-smoke-tracker-design.md)
- [Decision log](docs/DECISIONS.md)
