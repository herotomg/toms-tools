#!/usr/bin/env bash
# Is Paseo's clone of this plugin behind its newest release?
#
# The contract with tt: print one line when an update is waiting, print nothing
# when it is current, and exit non-zero only if we genuinely could not tell.
# "Could not tell" is never reported as an update. tt runs this at most once a
# day and caches the answer, so it stays off the path of `tt` and `tt list`.
set -euo pipefail

REPO="tomgrin10/paseo-defer"
ID="paseo-defer"
SOURCES="${PASEO_HOME:-$HOME/.paseo}/plugins/sources.json"

# No clone recorded means either not installed, or installed from a local
# checkout. Neither is ours to call outdated.
[ -f "$SOURCES" ] || exit 0

REF="$(tr -d ' \t\n' < "$SOURCES" \
  | sed -n 's/.*"'"$ID"'":{\([^}]*\)}.*/\1/p' \
  | sed -n 's/.*"requestedRef":"\([^"]*\)".*/\1/p')"

case "$REF" in
  "" | null) exit 0 ;;
esac

TAG="$(curl -fsSLI --connect-timeout 2 --max-time 5 -o /dev/null -w '%{url_effective}' \
  "https://github.com/$REPO/releases/latest" | sed 's#.*/tag/##')"

case "$TAG" in
  "" | latest) exit 0 ;;
esac

[ "$REF" = "$TAG" ] || echo "$REF → $TAG"
