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

gh alias delete unresolved 2>/dev/null || true

gh alias set unresolved --shell '
REPO=$(gh repo view --json nameWithOwner -q .nameWithOwner)
OWNER=${REPO%%/*}
NAME=${REPO##*/}
PR=${1:-$(gh pr view --json number -q .number)}
gh api graphql -f query="
query(\\$owner: String!, \\$repo: String!, \\$number: Int!) {
  repository(owner: \\$owner, name: \\$repo) {
    pullRequest(number: \\$number) {
      reviewThreads(first: 100) {
        nodes {
          isResolved
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
}" -F owner="$OWNER" -F repo="$NAME" -F number="$PR" \
  --jq ".data.repository.pullRequest.reviewThreads.nodes[] | select(.isResolved == false) | .comments.nodes[] | \"\(.path):\(.line) @\(.author.login)\n\(.body)\n\""
'