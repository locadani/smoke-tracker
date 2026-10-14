#!/usr/bin/env bash
# Shows which exercises are done: runs each exercise's tests and counts how many pass.
# Usage: ./progress.sh            (from anywhere; honours CARGO_TARGET_DIR)
set -euo pipefail

cd "$(dirname "$0")"

echo "Building the exercises..."
if ! cargo test --no-run --quiet 2>/dev/null; then
    echo "The exercises do not compile. Run 'cargo test --no-run' to see the errors." >&2
    exit 1
fi
echo

total=0
done_count=0
for test_file in tests/ex*.rs; do
    name=$(basename "$test_file" .rs)
    total=$((total + 1))

    summary=$(cargo test --quiet --test "$name" 2>/dev/null | grep '^test result:' || true)
    passed=$(echo "$summary" | sed -E 's/.* ([0-9]+) passed.*/\1/')
    failed=$(echo "$summary" | sed -E 's/.* ([0-9]+) failed.*/\1/')
    count=$((passed + failed))

    if [ "$failed" -eq 0 ] && [ "$count" -gt 0 ]; then
        status="✅ done"
        done_count=$((done_count + 1))
    elif [ "$passed" -gt 0 ]; then
        status="🟡 partial"
    else
        status="⬜ not started"
    fi
    printf '%-22s %-15s (%d/%d tests)\n' "$name" "$status" "$passed" "$count"
done

echo
echo "Done: $done_count of $total"
