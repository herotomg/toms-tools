#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

AGENTS_DIR="$HOME/.claude/agents"
COMMANDS_DIR="$HOME/.claude/commands"
mkdir -p "$COMMANDS_DIR"

TS=$(date +%Y%m%d-%H%M%S)

# Back up the command before overwriting
if [ -f "$COMMANDS_DIR/fix-pr.md" ]; then
  cp "$COMMANDS_DIR/fix-pr.md" "$COMMANDS_DIR/fix-pr.md.bak.$TS"
  echo "Backed up $COMMANDS_DIR/fix-pr.md to $COMMANDS_DIR/fix-pr.md.bak.$TS" >&2
fi

# Retire the old subagent from previous versions (instructions now live in the command itself)
if [ -f "$AGENTS_DIR/pr-fixer.md" ]; then
  mv "$AGENTS_DIR/pr-fixer.md" "$AGENTS_DIR/pr-fixer.md.bak.$TS"
  echo "Removed legacy pr-fixer agent (backed up to $AGENTS_DIR/pr-fixer.md.bak.$TS)" >&2
fi

cp "$SCRIPT_DIR/command.md" "$COMMANDS_DIR/fix-pr.md"

echo "✓ /fix-pr command installed" >&2
