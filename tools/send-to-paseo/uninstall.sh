#!/usr/bin/env bash
# Remove the Paseo plugin, but only the one we installed.
#
# Symmetry with install.sh matters here: that script refuses to touch a plugin
# somebody is running from a local checkout, so this must not delete it either.
# Only a clone recorded in sources.json is ours to remove.
set -euo pipefail

ID="send-to-paseo"
PASEO_DIR="${PASEO_HOME:-$HOME/.paseo}"
SOURCES="$PASEO_DIR/plugins/sources.json"

if ! command -v paseo >/dev/null 2>&1; then
  echo "paseo is not installed; nothing to remove" >&2
  exit 0
fi

managed=0
if [ -f "$SOURCES" ] && tr -d ' \t\n' < "$SOURCES" | grep -q "\"$ID\":{"; then
  managed=1
fi

if [ "$managed" -eq 0 ]; then
  echo "$ID is not a clone tt installed (a local checkout, most likely)," >&2
  echo "so the Paseo plugin was left in place. Remove it yourself with:" >&2
  echo "  paseo plugin remove $ID" >&2
  exit 0
fi

if paseo plugin remove "$ID" >/dev/null 2>&1; then
  echo "removed the $ID plugin from Paseo" >&2
else
  echo "could not remove the $ID plugin; try: paseo plugin remove $ID" >&2
fi

# the pairing token in $PASEO_DIR/plugin-data is deliberately left alone: it is
# user data, and reinstalling should not start from scratch.
