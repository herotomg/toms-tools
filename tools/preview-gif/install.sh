#!/usr/bin/env bash
# Install the preview-gif skill and the `screengif` recorder it falls back to.
#
# Same shape as the artifacts tool: one payload directory, symlinked into every
# agent host found, so the skill has a single source of truth on disk.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

PAYLOAD="$HOME/.local/share/toms-tools/preview-gif"
BIN="${TT_BIN_DIR:-$HOME/.local/bin}"
SKILLS=(preview-gif)
BACKUPS="$HOME/.local/share/toms-tools/backups"
TS=$(date +%Y%m%d-%H%M%S)

# ---------------------------------------------------------------- payload
[ -n "$PAYLOAD" ] || { echo "preview-gif: empty payload path" >&2; exit 1; }
rm -rf "$PAYLOAD"
mkdir -p "$PAYLOAD"
cp -R "$SCRIPT_DIR/bin" "$SCRIPT_DIR/skills" "$PAYLOAD/"
# tt extracts embedded files without their mode bits, so set it here.
chmod +x "$PAYLOAD/bin/screengif"

mkdir -p "$BIN"
ln -sfn "$PAYLOAD/bin/screengif" "$BIN/screengif"
echo "linked $BIN/screengif" >&2

# Retire the pre-tt location the skill used to hardcode.
LEGACY="$HOME/.claude/scripts/screengif"
if [ -e "$LEGACY" ] && [ ! -L "$LEGACY" ]; then
  mkdir -p "$BACKUPS"
  mv "$LEGACY" "$BACKUPS/screengif.$TS"
  echo "backed up $LEGACY -> $BACKUPS/screengif.$TS" >&2
fi

# ---------------------------------------------------------------- skills
link_skills() {
  local dest="$1" label="$2" target bak
  mkdir -p "$dest"
  for s in "${SKILLS[@]}"; do
    target="$dest/$s"
    # A real directory here would make `ln` drop the link inside it. Move it
    # aside --- and out of the skills directory, or the host would load the
    # backup as a second copy of the same skill.
    if [ -e "$target" ] && [ ! -L "$target" ]; then
      bak="$BACKUPS/${label// /-}"
      mkdir -p "$bak"
      mv "$target" "$bak/$s.$TS"
      echo "backed up $target -> $bak/$s.$TS" >&2
    fi
    ln -sfn "$PAYLOAD/skills/$s" "$target"
  done
  echo "linked ${#SKILLS[@]} skill into $dest ($label)" >&2
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
  echo "note: found neither Claude Code nor Codex; the skill was not linked." >&2
  echo "      \`screengif\` works on its own, or set TT_CLAUDE_SKILL_DIR and reinstall." >&2
fi

# ---------------------------------------------------------------- follow-ups
case ":$PATH:" in
  *":$BIN:"*) ;;
  *) echo "note: $BIN is not on your PATH — add to ~/.zshrc: export PATH=\"$BIN:\$PATH\"" >&2 ;;
esac

# These are what the two recording paths shell out to. Missing ones degrade the
# skill rather than break the install, so report instead of failing.
missing=()
for dep in node ffmpeg gifski gifsicle; do
  command -v "$dep" >/dev/null 2>&1 || missing+=("$dep")
done
if [ ${#missing[@]} -gt 0 ]; then
  echo "note: missing recording dependencies: ${missing[*]}" >&2
  echo "      brew install ${missing[*]}" >&2
fi

echo "✓ preview-gif skill + screengif installed" >&2
