---
name: preview-gif
description: Record a polished preview GIF of a UI for a README, PR, or docs page — with a visible cursor and visible clicks. Use when asked to record, capture, or make a demo/preview GIF or screencast of an app, a feature, or a UI flow, or to replace an existing README GIF. Covers browser-driven recording (preferred, deterministic) and raw screen capture (fallback).
---

# Preview GIFs

Two ways to get a GIF. Pick deliberately — they fail in different places.

| | Browser-driven (preferred) | Screen capture (fallback) |
|---|---|---|
| Tool | Playwright video + injected cursor | `screengif` |
| Cursor | Drawn by us, always visible | Real OS cursor, only if a human drives |
| Determinism | Re-runnable, identical every time | One-shot performance |
| Privacy | Only the viewport, nothing bleeds in | Whole window/display, notifications included |
| Use when | Target is a web app you can drive | Native app, terminal, or a human must drive |

**Default to browser-driven.** Reach for screen capture only for a native/Electron
UI you cannot drive, or when the user wants to perform the demo themselves.

For terminal/CLI demos use neither — use VHS (charmbracelet/vhs), which builds
terminal GIFs from a text script, deterministically.

## The cursor problem

Playwright's video is Chromium's compositor surface, which **never contains the OS
cursor**. And Playwright drives input via CDP `Input.dispatchMouseEvent`, which
synthesises events inside the renderer — **the hardware cursor never moves**. So:

- Recording a *headed* Playwright browser with `screengif` gives a frozen arrow
  wherever the user last left it, and no click highlights (macOS click
  highlighting only fires on real OS clicks).
- The fix is to draw our own cursor in the page. `scripts/demo-cursor.mjs` does
  this: an arrow that follows `mousemove`, dips to 85% on press, and leaves an
  expanding ring at each click point.

Playwright *does* have native annotation — `video: { show: { actions: { cursor:
'pointer' } } }` in a **playwright.config** (test-runner only, not
`browser.newContext`). It draws an arrow plus a red dot, but only for ~500ms per
action with no travel between targets, and it also stamps element outlines and a
`Click get_by_role(...)` caption you cannot turn off. That reads as a test
artifact. Use the injected cursor for anything user-facing.

## Recording

1. Copy `scripts/record-template.mjs` to `/tmp`, edit the two marked sections.
2. Run it with `node`. Iterate — it is cheap and repeatable.

Rules that matter, each learned the hard way:

- **Glide, then click.** `locator.click()` teleports the pointer. Always
  `glideClick(page, locator)` (or `glideTo` + `click`) so there is visible travel.
- **Dwell on the target.** `glideClick` rests 650ms before pressing, longer for a
  commit action. Below ~500ms it reads as an instant jump at 20fps and the viewer
  cannot see what was clicked.
- **Hover and click can be different actions.** Check what hovering alone does
  before scripting a click. A tooltip/preview that appears on hover may be
  *toggled off* by the click that follows — so the click cancels the very thing
  the beat exists to show. When hover is the behaviour, `glideTo` and hold; do
  not click.
- **Never end on a locator.** A final `glideTo(someLocator)` that fails to match
  blocks for the full 30s timeout and silently pads dead frames onto the take.
  Drift away with `page.mouse.move(x, y, { steps })`.
- **Two passes.** Pass 1 puts the app into its opening state (collapse sidebars,
  dismiss onboarding nags, select the target) and is discarded; pass 2 is the
  take. `localStorage` persists between them in the same context.
- **Reset app state between takes.** Anything a take creates (a queued item, a
  row, a toast) changes what the *next* take sees, and will silently alter
  behaviour — see the hover/click trap above. Clear it before re-recording.
- **Wait on an element, not a timeout**, for the first interaction — an SPA
  reconnecting to a backend can take many seconds.
- `findChromium()` handles the usual `npx playwright` complaint that the cached
  browser revision is older than expected. Do not run `npx playwright install`.

Sizing: `1000x660` matches a GitHub README column well. Match an existing GIF's
dimensions exactly when replacing one (`ffprobe` it first).

## Encoding

```bash
ffmpeg -v error -i take.webm -c:v libx264 -pix_fmt yuv420p -crf 16 take.mp4 -y
screengif -f take.mp4 -w 1000 --fps 20 -q 95 --lossy 30 out.gif
```

- **`--lossy 30`, not the default 65.** On dark UIs, 65 leaves visible mottling
  and ghost text in flat backgrounds. 30 costs ~15% size and looks clean.
- **20fps for cursor motion**, 15fps if size matters more. Dwell frames are
  static and compress well, so the cost of dwelling is small.
- Budget: ~1MB for a 20s 1000px take at 20fps.

## Verify before showing anyone

Never hand over a GIF you have not looked at.

```bash
ffprobe -v error -show_entries format=duration -of default=nw=1:nk=1 take.webm
ffmpeg -v error -ss 5 -i out.gif -frames:v 1 /tmp/f.png -y   # then Read it
```

- Check the **duration matches the scripted waits**. A big overshoot means
  something blocked — usually a locator.
- Sample frames at each beat and confirm the state actually changed. Clicks fire
  ~0.5s ripples, so sample at 0.2-0.3s intervals to catch one.
- Stack frames into a contact sheet to review several at once:
  `ffmpeg -i a.png -i b.png -filter_complex "[0][1]vstack=inputs=2,scale=700:-1" sheet.png`

## Privacy — do this first, not last

A GIF headed for a public README will contain whatever is on screen. Real
project names, ticket titles, session lists and queued messages are a leak.

- **Stage a throwaway target.** Create a scratch project/workspace/session with
  neutral names and record against that. Far safer than cropping real data out.
- Collapse or crop sidebars and pickers that enumerate real work.
- Give the demo a plausible, generic script ("Summarise the diff and open a PR"),
  not visible test scaffolding ("reply with one sentence and do nothing").
- Show the user before it is committed, and say what is in frame.

## Recording the Paseo web app

Paseo's daemon is loopback-only and its own web UI is force-disabled by the
desktop app (`PASEO_WEB_UI_ENABLED=false` in the daemon's environment, which
overrides `features.webUi.enabled` in `~/.paseo/config.json` — do not bother
setting it). Serve Paseo's own bundle over loopback instead:

```bash
nohup python3 -m http.server 8787 --bind 127.0.0.1 \
  --directory "/Applications/Paseo.app/Contents/Resources/app-dist" \
  > /tmp/paseo-webui.log 2>&1 &
```

`http://127.0.0.1:8787` is **already a permanent entry** in
`daemon.cors.allowedOrigins`, so no config editing or `paseo reload` is needed.
Do not add and remove it per session.

- `http.server` has no SPA fallback, so **deep links 404** — always load `/` and
  click through. Paseo restores the last workspace, and sidebar rows carry
  testids like `sidebar-workspace-row-<host>:<workspaceId>`.
- Never restart the Paseo daemon; it can kill the agent doing the work.
- Stop the static server when done; leave the CORS entry alone.

## Screen-capture fallback

`screengif` (screencapture -> gifski -> gifsicle). `--help` for
options; `-f FILE` converts an existing recording, which is how the
browser-driven path reuses it.

- `--app NAME` needs peekaboo to resolve a window id, which needs **Screen
  Recording and Accessibility granted to peekaboo's bridge host**. Check with
  `peekaboo permissions`; if not granted, `--app`/`--window-id` silently list
  nothing and you must fall back to `--display N` or `-R x,y,w,h`.
- Still capture can work while `-V` video capture fails; test with a real
  `screencapture -V 2` before concluding permissions are missing.
- Never use `-i` — it blocks waiting for a human.
- Recording a display captures notifications and every other window. Check what
  is actually on each display first (`screencapture -x -D 2 /tmp/d2.png`).
