#!/usr/bin/env bash
# Install the `art` CLI, its shared assets, and the two authoring skills.
#
# Everything lands in one payload directory and is symlinked from wherever each
# host looks. That keeps a single source of truth on disk: Claude Code and Codex
# read the same SKILL.md files, and `art` finds its assets relative to the real
# path of the binary (it resolves symlinks), so the link in ~/.local/bin works.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Note: this is the *program*, not the artifact store. The store is ART_HOME,
# which defaults to ~/.local/share/artifacts and is never touched here.
PAYLOAD="$HOME/.local/share/toms-tools/artifacts"
BIN="${TT_BIN_DIR:-$HOME/.local/bin}"
SKILLS=(artifact-design publish-artifact)
BACKUPS="$HOME/.local/share/toms-tools/backups"
TS=$(date +%Y%m%d-%H%M%S)

command -v uv >/dev/null 2>&1 || {
  echo "artifacts: uv is required to run \`art\` — see https://astral.sh/uv" >&2
  exit 1
}

# ---------------------------------------------------------------- payload
[ -n "$PAYLOAD" ] || { echo "artifacts: empty payload path" >&2; exit 1; }
rm -rf "$PAYLOAD"
mkdir -p "$PAYLOAD"
cp -R "$SCRIPT_DIR/bin" "$SCRIPT_DIR/assets" "$SCRIPT_DIR/skills" "$PAYLOAD/"
# tt extracts embedded files without their mode bits, so set it here.
chmod +x "$PAYLOAD/bin/art"

mkdir -p "$BIN"
ln -sfn "$PAYLOAD/bin/art" "$BIN/art"
echo "linked $BIN/art" >&2

# ---------------------------------------------------------------- skills
link_skills() {
  local dest="$1" label="$2" target bak
  mkdir -p "$dest"
  for s in "${SKILLS[@]}"; do
    target="$dest/$s"
    # A real directory here (an older copy-based install) would make `ln` drop
    # the link *inside* it rather than replace it. Move it aside first --- and
    # out of the skills directory, or the host would load the backup as a
    # second copy of the same skill.
    if [ -e "$target" ] && [ ! -L "$target" ]; then
      bak="$BACKUPS/${label// /-}"
      mkdir -p "$bak"
      mv "$target" "$bak/$s.$TS"
      echo "backed up $target -> $bak/$s.$TS" >&2
    fi
    ln -sfn "$PAYLOAD/skills/$s" "$target"
  done
  echo "linked ${#SKILLS[@]} skills into $dest ($label)" >&2
}

installed_any=0

CLAUDE_SKILLS="${TT_CLAUDE_SKILL_DIR:-$HOME/.claude/skills}"
if [ -n "${TT_CLAUDE_SKILL_DIR:-}" ] || command -v claude >/dev/null 2>&1 || [ -d "$HOME/.claude" ]; then
  link_skills "$CLAUDE_SKILLS" "Claude Code"
  installed_any=1
fi

CODEX_SKILLS="${CODEX_HOME:-$HOME/.codex}/skills"
if command -v codex >/dev/null 2>&1 || [ -d "${CODEX_HOME:-$HOME/.codex}" ]; then
  link_skills "$CODEX_SKILLS" "Codex"
  installed_any=1
fi

if [ "$installed_any" -eq 0 ]; then
  echo "note: found neither Claude Code nor Codex; skills were not linked." >&2
  echo "      \`art\` works on its own, or set TT_CLAUDE_SKILL_DIR and reinstall." >&2
fi

# ---------------------------------------------------------------- follow-ups
case ":$PATH:" in
  *":$BIN:"*) ;;
  *) echo "note: $BIN is not on your PATH — add to ~/.zshrc: export PATH=\"$BIN:\$PATH\"" >&2 ;;
esac

command -v tailscale >/dev/null 2>&1 ||
  echo "note: tailscale not found — install it, then run \`art serve\`." >&2

echo "✓ art CLI + ${#SKILLS[@]} artifact skills installed" >&2
echo "  next: art serve   # one-time, points tailscale at the store" >&2
