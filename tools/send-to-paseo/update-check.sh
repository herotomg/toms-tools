#!/usr/bin/env bash
# Are either half of send-to-paseo behind the newest release?
#
# The contract with tt: print one line when an update is waiting, print nothing
# when it is current, and exit non-zero only if we genuinely could not tell.
# Both halves are checked, because they are pinned to a release together and
# either one lagging blocks sends on their frozen contract.
set -euo pipefail

REPO="tomgrin10/send-to-paseo"
ID="send-to-paseo"
SOURCES="${PASEO_HOME:-$HOME/.paseo}/plugins/sources.json"
EXTVERSION="$HOME/.local/share/toms-tools/send-to-paseo/extension-version"

REF=""
if [ -f "$SOURCES" ]; then
  REF="$(tr -d ' \t\n' < "$SOURCES" \
    | sed -n 's/.*"'"$ID"'":{\([^}]*\)}.*/\1/p' \
    | sed -n 's/.*"requestedRef":"\([^"]*\)".*/\1/p')"
fi

EXT=""
[ -f "$EXTVERSION" ] && EXT="$(tr -d '[:space:]' < "$EXTVERSION")"

# Nothing we manage, so nothing to compare. A plugin installed from a local
# checkout records no ref, and is not ours to call outdated.
if { [ -z "$REF" ] || [ "$REF" = null ]; } && [ -z "$EXT" ]; then
  exit 0
fi

TAG="$(curl -fsSLI --connect-timeout 2 --max-time 5 -o /dev/null -w '%{url_effective}' \
  "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')"

case "$TAG" in
  "" | latest) exit 0 ;;
esac

BEHIND=""
case "$REF" in
  "" | null | "$TAG") ;;
  *) BEHIND="plugin" ;;
esac
if [ -n "$EXT" ] && [ "$EXT" != "$TAG" ]; then
  BEHIND="${BEHIND:+$BEHIND and }extension"
fi

[ -z "$BEHIND" ] || echo "$BEHIND → $TAG"
