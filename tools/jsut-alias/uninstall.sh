#!/usr/bin/env bash
# Drop the managed alias line. Every line we add carries the marker below, so
# removing by marker cannot touch anything the user wrote themselves.
set -euo pipefail

RC="${ZDOTDIR:-$HOME}/.zshrc"
MARKER="# toms-tools:jsut-alias"

[ -f "$RC" ] || { echo "no $RC; nothing to remove" >&2; exit 0; }

if ! grep -Fq "$MARKER" "$RC"; then
  echo "no managed jsut-alias line in $RC" >&2
  exit 0
fi

TS=$(date +%Y%m%d-%H%M%S)
cp "$RC" "$RC.bak.$TS"

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
grep -vF "$MARKER" "$RC" >"$TMP" || true
cat "$TMP" >"$RC"

echo "removed the jsut-alias line from $RC (backup: $RC.bak.$TS)" >&2
