#!/usr/bin/env bash
set -euo pipefail

RC="${ZDOTDIR:-$HOME}/.zshrc"
MARKER="# toms-tools:jsut-alias"
LINE="alias jsut='just' $MARKER"

touch "$RC"

if [ "$(grep -F "$MARKER" "$RC" || true)" = "$LINE" ]; then
  echo "✓ jsut alias already up to date" >&2
  exit 0
fi

# Back up before editing
TS=$(date +%Y%m%d-%H%M%S)
cp "$RC" "$RC.bak.$TS"
echo "Backed up $RC to $RC.bak.$TS" >&2

# Drop any previous managed line, then append the current one
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
grep -vF "$MARKER" "$RC" >"$TMP" || true
printf '%s\n' "$LINE" >>"$TMP"
cat "$TMP" >"$RC"

echo "✓ jsut alias installed (restart your shell or run: source $RC)" >&2
