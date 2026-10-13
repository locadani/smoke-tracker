#!/usr/bin/env bash
# Replaces each argument with *** in stdin, literally. Used before posting a plan publicly.
set -euo pipefail

text=$(cat)
for value in "$@"; do
  [ -n "$value" ] || continue
  text=${text//"$value"/***}
done
printf '%s\n' "$text"
