#!/usr/bin/env bash
set -euo pipefail

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI (gh) is required. Install it with: brew install gh" >&2
  exit 1
fi

if ! gh auth status >/dev/null 2>&1; then
  echo "GitHub CLI is not authenticated. Run: gh auth login" >&2
  exit 1
fi

# Backup existing gh config before making changes
if [ -f "$HOME/.config/gh/config.yml" ]; then
  TS=$(date +%Y%m%d-%H%M%S)
  cp "$HOME/.config/gh/config.yml" "$HOME/.config/gh/config.yml.bak.$TS"
  echo "Backed up gh config to ~/.config/gh/config.yml.bak.$TS" >&2
fi

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

cat >"$TMP" <<'GH_UNRESOLVED_BODY_EOF'
!
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
OWNER=${REPO%%/*}
NAME=${REPO##*/}
PR=${1:-$(gh pr view --json number -q .number)}
QUERY=$(cat <<'GRAPHQL'
query($owner: String!, $repo: String!, $number: Int!) {
  repository(owner: $owner, name: $repo) {
    pullRequest(number: $number) {
      reviewThreads(first: 100) {
        nodes {
          isResolved
          isOutdated
          comments(first: 100) {
            nodes {
              author { login }
              body
              path
              line
            }
          }
        }
      }
    }
  }
}
GRAPHQL
)
JQ_FILTER=$(cat <<'JQ'
.data.repository.pullRequest.reviewThreads.nodes
| map(select(.isResolved == false))
| to_entries
| map(
    . as $thread_entry
    | $thread_entry.value as $thread
    | ($thread.comments.nodes[0] // {}) as $first_comment
    | (
        "========== PR Review Thread \($thread_entry.key + 1) ==========\n"
        + "Location: \($first_comment.path // "Unknown"):\(
            if $first_comment.line == null then
              (if $thread.isOutdated then "Stale" else "NoLine" end)
            else
              ($first_comment.line | tostring)
            end
          )\n"
        + "Outdated: \($thread.isOutdated)\n"
        + "Comments: \($thread.comments.nodes | length)\n"
        + "--------------------------------------------------\n"
        + (
            $thread.comments.nodes
            | to_entries
            | map(
                "[\(.key + 1)] @\(.value.author.login)\n"
                + (
                    .value.body
                    | gsub("\\[!\\[Fix This in Augment\\]\\([^\\n]*\\)\\]\\([^\\n]*\\)"; "")
                    | gsub("\\n{3,}"; "\n\n")
                  )
              )
            | join("\n\n")
          )
      )
  )
| join("\n\n")
JQ
)
COMMENTS=$(gh api graphql -f query="$QUERY" -F owner="$OWNER" -F repo="$NAME" -F number="$PR" --jq "$JQ_FILTER")
if [ -z "$COMMENTS" ]; then
  echo "No unresolved review comments."
else
  printf "%s\n" "$COMMENTS"
fi
GH_UNRESOLVED_BODY_EOF

EXISTING=$(gh alias list 2>/dev/null | awk '
  /^unresolved:/ { found=1; next }
  found && /^[^[:space:]].*:/ { exit }
  found { sub(/^    /, ""); print }
' || true)
NEW=$(cat "$TMP")

if [ "${EXISTING:-}" = "$NEW" ]; then
  echo "✓ gh unresolved alias already up to date" >&2
  exit 0
fi

gh alias delete unresolved 2>/dev/null || true

gh alias set unresolved --shell "$NEW"
echo "✓ gh unresolved installed" >&2