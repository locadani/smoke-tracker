#!/usr/bin/env bash
# Regenerates the committed sqlx query metadata (`.sqlx/`) of the storage crates from freshly
# migrated databases. With --check, fails instead when the committed metadata is out of date.
# Needs sqlx-cli 0.9 (see README).
set -euo pipefail

mode=prepare
if [[ "${1:-}" == "--check" ]]; then mode=check; fi

root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export SQLX_OFFLINE=false

prepare() {
  if [[ "$mode" == check ]]; then cargo sqlx prepare --check; else cargo sqlx prepare; fi
}

cd "$root/crates/storage-sqlite"
export SQLITE_DATABASE_URL="sqlite://$work/prepare.db"
export DATABASE_URL="$SQLITE_DATABASE_URL"
sqlx database create
sqlx migrate run
prepare
