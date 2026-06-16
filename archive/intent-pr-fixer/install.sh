#!/usr/bin/env bash
set -euo pipefail

mkdir -p "$HOME/.augment/specialists"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cp "$SCRIPT_DIR/specialist.md" "$HOME/.augment/specialists/custom-toms-tools-pr-fixer.md"