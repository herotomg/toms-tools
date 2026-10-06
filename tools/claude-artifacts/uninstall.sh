#!/usr/bin/env bash
# Turn the Artifact flag back off, but only if tt is what turned it on.
#
# install.sh leaves an already-set flag alone and records nothing, so a flag the
# user set themselves has no marker and is not ours to remove. Everything else
# in `agents.providers` is written back untouched.
set -euo pipefail

PASEO_DIR="${PASEO_HOME:-$HOME/.paseo}"
MARKER="$HOME/.local/share/toms-tools/claude-artifacts/enabled-flag"

if [ ! -f "$MARKER" ]; then
  echo "CLAUDE_CODE_ARTIFACT was not set by tt, so Paseo's config was left alone." >&2
  exit 0
fi

if ! command -v paseo >/dev/null 2>&1; then
  echo "paseo is not installed; nothing to remove" >&2
  exit 0
fi

providers="$(paseo daemon config get agents.providers --home "$PASEO_DIR" --json | jq -c '.value // {}')"

if ! jq -e '.claude.env | has("CLAUDE_CODE_ARTIFACT")' <<<"$providers" >/dev/null 2>&1; then
  echo "CLAUDE_CODE_ARTIFACT is already gone from Paseo's config" >&2
  exit 0
fi

merged="$(jq -c 'del(.claude.env.CLAUDE_CODE_ARTIFACT)
  | if .claude.env == {} then del(.claude.env) else . end' <<<"$providers")"
paseo daemon config set agents.providers "$merged" --home "$PASEO_DIR" --json >/dev/null

echo "removed CLAUDE_CODE_ARTIFACT from Paseo's Claude provider" >&2
echo "  sessions already running keep the Artifact tool until reloaded" >&2
