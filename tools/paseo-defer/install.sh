#!/usr/bin/env bash
# Install paseo-defer as a trusted Paseo plugin, pinned to its newest release.
#
# There is no payload of ours to copy: Paseo clones the repository itself,
# compiles it and starts it. So this script drives `paseo plugin`, and its real
# work is deciding which of four states it is in without making any of them
# worse.
set -euo pipefail

REPO="tomgrin10/paseo-defer"
ID="paseo-defer"

PASEO_DIR="${PASEO_HOME:-$HOME/.paseo}"
CONFIG="$PASEO_DIR/config.json"
SOURCES="$PASEO_DIR/plugins/sources.json"

if ! command -v paseo >/dev/null 2>&1; then
  echo "paseo is not installed, so there is no plugin host to install into." >&2
  echo "  brew install --cask paseo" >&2
  exit 1
fi

# Plugins off in Settings means `plugin add` succeeds and then nothing ever
# runs. Stop, rather than report a success the user cannot see the effect of.
if [ -f "$CONFIG" ] && grep -q '"pluginsEnabled": *false' "$CONFIG"; then
  echo "Paseo has plugins turned off, so this would install but never run." >&2
  echo "Turn them on in Paseo under Settings -> Plugins, then run this again." >&2
  exit 1
fi

# The newest release tag, read from the redirect that /releases/latest issues.
# Deliberately not the GitHub API, which rate-limits unauthenticated callers to
# 60 requests an hour — shared between everyone behind one IP.
latest_tag() {
  curl -fsSLI --connect-timeout 2 --max-time 5 -o /dev/null -w '%{url_effective}' \
    "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##'
}

# The ref Paseo has cloned, empty when Paseo is not managing a clone at all.
# config.json cannot answer this: it records `source: "directory"` for a clone
# and for somebody's local checkout alike. Only sources.json tells them apart.
tracked_ref() {
  [ -f "$SOURCES" ] || return 0
  # Whitespace is stripped first so this does not depend on the file being
  # pretty-printed. A line-oriented parse looked fine against today's output
  # and would have read "no ref" from a compact file — which install.sh takes
  # to mean "somebody's local checkout", so it would refuse to ever update.
  tr -d ' \t\n' < "$SOURCES" \
    | sed -n 's/.*"'"$ID"'":{\([^}]*\)}.*/\1/p' \
    | sed -n 's/.*"requestedRef":"\([^"]*\)".*/\1/p'
}

configured() {
  [ -f "$CONFIG" ] && grep -q "\"$ID\": *{" "$CONFIG"
}

paseo_plugin() {
  if ! paseo plugin "$@"; then
    echo >&2
    echo "\`paseo plugin $*\` failed. If Paseo is not running, start it (or run" >&2
    echo "\`paseo start\`) and try again — plugins are managed by the daemon." >&2
    return 1
  fi
}

TAG="$(latest_tag)"
if [ -z "$TAG" ] || [ "$TAG" = "latest" ]; then
  echo "Could not work out the newest $REPO release. Check your connection," >&2
  echo "or install it yourself with: paseo plugin add $REPO" >&2
  exit 1
fi

REF="$(tracked_ref)"

if [ -n "$REF" ] && [ "$REF" = "$TAG" ]; then
  echo "$ID is already pinned to $TAG" >&2
elif [ -n "$REF" ]; then
  # A pinned ref never moves — `paseo plugin update` reports 0 commits against
  # a tag forever — and `plugin add` refuses an id that is already configured.
  # Re-pinning therefore means remove then add. The deferred queue lives in
  # $PASEO_HOME/plugin-data/defer, outside the checkout, so it survives.
  echo "moving $ID from $REF to $TAG" >&2
  paseo_plugin remove "$ID"
  # Removing first is forced on us, which leaves a window with nothing
  # installed. If the add fails, say that plainly rather than let the user
  # assume the old version is still there.
  if ! paseo_plugin add "$REPO" --ref "$TAG"; then
    echo "$REF was removed but $TAG could not be installed." >&2
    echo "Run \`tt install $ID\` to try again." >&2
    exit 1
  fi
elif configured; then
  # Somebody is developing against a checkout. Re-pointing that at GitHub
  # would quietly swap their working copy for a clone of the release.
  echo "$ID is already installed from a local checkout, so it was left alone." >&2
  echo "To switch to the released plugin: paseo plugin remove $ID" >&2
else
  paseo_plugin add "$REPO" --ref "$TAG"
fi

echo "✓ paseo-defer is installed as a Paseo plugin" >&2
