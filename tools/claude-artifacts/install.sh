#!/usr/bin/env bash
# Turn on Claude's native Artifact tool for every Claude session Paseo starts.
#
# Claude Code exposes the tool when CLAUDE_CODE_ARTIFACT=1 is in its
# environment, and Paseo passes `agents.providers.claude.env` to every Claude
# process it spawns. So the whole install is one value in Paseo's config.
#
# `paseo daemon config set` validates the config and reloads a running daemon,
# so it is the only writer used here. It cannot address a dynamic key such as an
# env var directly, so the whole `agents.providers` object is read, merged and
# written back — every other provider and env var goes back exactly as found.
set -euo pipefail

PASEO_DIR="${PASEO_HOME:-$HOME/.paseo}"
STATE="$HOME/.local/share/toms-tools/claude-artifacts"

providers="$(paseo daemon config get agents.providers --home "$PASEO_DIR" --json | jq -c '.value // {}')"

if jq -e '.claude.env.CLAUDE_CODE_ARTIFACT == "1"' <<<"$providers" >/dev/null; then
  echo "Claude sessions in Paseo already get the Artifact tool; nothing changed." >&2
  exit 0
fi

# Paseo requires env values to be strings: "1", never 1.
merged="$(jq -c '.claude.env.CLAUDE_CODE_ARTIFACT = "1"' <<<"$providers")"
paseo daemon config set agents.providers "$merged" --home "$PASEO_DIR" --json >/dev/null

# Record that tt turned this on, so `tt remove` turns off only what tt did.
mkdir -p "$STATE"
touch "$STATE/enabled-flag"

echo "✓ new Claude sessions in Paseo get the native Artifact tool" >&2
echo "  sessions already running keep their old environment until reloaded" >&2
