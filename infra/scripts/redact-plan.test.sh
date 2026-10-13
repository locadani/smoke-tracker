#!/usr/bin/env bash
set -euo pipefail

here=$(cd "$(dirname "$0")" && pwd)

fail() { printf 'FAIL: %s\n--- got ---\n%s\n' "$1" "$2"; exit 1; }

got=$(printf 'email = "deploy@my-proj-123.iam.gserviceaccount.com"\nname = "projects/987654/locations/global"\n' | "$here/redact-plan.sh" my-proj-123 987654)
want=$(printf 'email = "deploy@***.iam.gserviceaccount.com"\nname = "projects/***/locations/global"\n')
[ "$got" = "$want" ] || fail "replaces every value" "$got"

got=$(printf 'a.b*c\n' | "$here/redact-plan.sh" 'a.b*c')
[ "$got" = "***" ] || fail "treats values literally, not as patterns" "$got"

got=$(printf 'unchanged\n' | "$here/redact-plan.sh" "")
[ "$got" = "unchanged" ] || fail "ignores empty values" "$got"

echo "redact-plan: all tests passed"
