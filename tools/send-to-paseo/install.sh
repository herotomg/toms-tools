#!/usr/bin/env bash
# Install send-to-paseo: the Paseo plugin, plus the browser extension it pairs
# with, both taken from the same release.
#
# The two halves speak a frozen contract and refuse each other when their
# versions differ, so they must not be sourced independently. The extension is
# only published as a release asset, so the plugin is pinned to that same
# release rather than tracking the default branch.
set -euo pipefail

REPO="tomgrin10/send-to-paseo"
ID="send-to-paseo"

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
  echo "or install it yourself with: paseo plugin add $REPO --path plugin" >&2
  exit 1
fi

REF="$(tracked_ref)"

if [ -n "$REF" ] && [ "$REF" = "$TAG" ]; then
  echo "$ID is already pinned to $TAG" >&2
elif [ -n "$REF" ]; then
  # A pinned ref never moves — `paseo plugin update` reports 0 commits against
  # a tag forever — and `plugin add` refuses an id that is already configured.
  # Re-pinning therefore means remove then add. The pairing token lives in
  # $PASEO_HOME/plugin-data/send-to-paseo, outside the checkout, so it
  # survives and nobody has to pair the extension again.
  echo "moving $ID from $REF to $TAG" >&2
  paseo_plugin remove "$ID"
  # Removing first is forced on us, which leaves a window with nothing
  # installed. If the add fails, say that plainly rather than let the user
  # assume the old version is still there.
  if ! paseo_plugin add "$REPO" --path plugin --ref "$TAG"; then
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
  paseo_plugin add "$REPO" --path plugin --ref "$TAG"
fi


# ----------------------------------------------------------------- extension
# Chrome derives an unpacked extension's ID from the *path* it was loaded from,
# and the pairing token is tied to that ID. So this directory is a permanent
# address: updates replace its contents in place and never move it, or the user
# would have to load and pair the extension again every time.
PAYLOAD="$HOME/.local/share/toms-tools/send-to-paseo"
EXT="$PAYLOAD/extension"
ASSET="https://github.com/$REPO/releases/download/$TAG/send-to-paseo-extension.zip"

for dep in curl unzip; do
  command -v "$dep" >/dev/null 2>&1 || { echo "$dep is required to fetch the extension" >&2; exit 1; }
done

STAGING="$(mktemp -d "${TMPDIR:-/tmp}/send-to-paseo.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT

if ! curl -fsSL -o "$STAGING/extension.zip" "$ASSET"; then
  echo "Could not download the extension for $TAG:" >&2
  echo "  $ASSET" >&2
  exit 1
fi

unzip -qq "$STAGING/extension.zip" -d "$STAGING/unpacked"

# Loading a directory without a manifest is a confusing Chrome error, so prove
# we have a real extension before putting it where the user will be sent.
if [ ! -f "$STAGING/unpacked/manifest.json" ]; then
  echo "The downloaded archive has no manifest.json at its root; refusing to" >&2
  echo "install it, because Chrome could not load it." >&2
  exit 1
fi

mkdir -p "$EXT"
find "$EXT" -mindepth 1 -delete
cp -R "$STAGING/unpacked/." "$EXT/"
printf '%s\n' "$TAG" > "$PAYLOAD/extension-version"

echo "✓ send-to-paseo $TAG: plugin installed, extension unpacked into" >&2
echo "  $EXT" >&2
