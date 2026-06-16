#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

AGENTS_DIR="$HOME/.claude/agents"
COMMANDS_DIR="$HOME/.claude/commands"
mkdir -p "$AGENTS_DIR" "$COMMANDS_DIR"

# Back up existing files before overwriting
TS=$(date +%Y%m%d-%H%M%S)
for target in "$AGENTS_DIR/pr-fixer.md" "$COMMANDS_DIR/fix-pr.md"; do
  if [ -f "$target" ]; then
    cp "$target" "$target.bak.$TS"
    echo "Backed up $target to $target.bak.$TS" >&2
  fi
done

cp "$SCRIPT_DIR/agent.md" "$AGENTS_DIR/pr-fixer.md"
cp "$SCRIPT_DIR/command.md" "$COMMANDS_DIR/fix-pr.md"

echo "✓ pr-fixer agent and /fix-pr command installed" >&2
