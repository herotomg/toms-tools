#!/usr/bin/env bash
# The alias lives in gh's own config, not in a file we own, so removing it
# needs gh itself. A missing gh means the alias is already unreachable.
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "gh is not installed; nothing to remove" >&2
  exit 0
fi

if gh alias delete unresolved 2>/dev/null; then
  echo "removed the gh unresolved alias" >&2
else
  echo "no gh unresolved alias to remove" >&2
fi
