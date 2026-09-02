# Preview GIF

Record a polished preview GIF of a UI for a README, PR, or docs page — with a
visible cursor and visible clicks.

## What it installs

- `~/.local/bin/screengif` — symlink to the recorder
  (`screencapture` → `gifski` → `gifsicle`); `screengif --help` for options.
- `~/.local/share/toms-tools/preview-gif/` — the payload: `bin/screengif` and
  `skills/preview-gif/` (SKILL.md plus `scripts/record-template.mjs` and
  `scripts/demo-cursor.mjs`).
- The `preview-gif` skill, symlinked into `~/.claude/skills` and `~/.codex/skills`
  for whichever hosts are present.

An earlier `~/.claude/scripts/screengif` is backed up and retired — the skill now
calls `screengif` from your PATH, so it works the same under both hosts.

## Dependencies

`node` (Playwright), `ffmpeg`, `gifski`, `gifsicle`. The install reports any that
are missing rather than failing; `screengif --app` additionally needs peekaboo
with Screen Recording and Accessibility granted.

## The two paths

|  | Browser-driven (preferred) | Screen capture (fallback) |
|---|---|---|
| Tool | Playwright video + injected cursor | `screengif` |
| Cursor | Drawn by us, always visible | Real OS cursor, only if a human drives |
| Determinism | Re-runnable, identical every time | One-shot performance |
| Privacy | Only the viewport | Whole window/display, notifications included |

Default to browser-driven. Playwright's video is Chromium's compositor surface,
which never contains the OS cursor, so the skill injects its own — an arrow that
follows the pointer, dips on press, and leaves a ring at each click.

For terminal/CLI demos use neither; use VHS (charmbracelet/vhs).

## Usage

In Claude Code or Codex, ask for a preview GIF and the skill loads itself. Or
drive the recorder directly:

```sh
screengif --list                                    # discover displays/windows
screengif --app Paseo -d 12 out.gif                 # record an app window
screengif -f take.mp4 -w 1000 --fps 20 --lossy 30 out.gif   # convert a recording
```

`--lossy 30` rather than the default 65: on dark UIs, 65 leaves visible mottling
and ghost text in flat backgrounds, for about 15% less size.

The skill carries the rest — gliding the cursor so travel is visible, dwelling
long enough to read, the two-pass setup, privacy staging, and how to verify a
take before showing it to anyone.
