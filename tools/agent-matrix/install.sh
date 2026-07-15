#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENTS_DIR="$HOME/.claude/agents"
mkdir -p "$AGENTS_DIR"

TS=$(date +%Y%m%d-%H%M%S)
installed=0

for src in "$SCRIPT_DIR"/agents/*.md; do
  name="$(basename "$src")"
  dest="$AGENTS_DIR/$name"
  if [ -f "$dest" ] && ! cmp -s "$src" "$dest"; then
    cp "$dest" "$dest.bak.$TS"
  fi
  cp "$src" "$dest"
  installed=$((installed + 1))
done

echo "✓ Installed $installed general-purpose agent variants to $AGENTS_DIR" >&2
