---
name: publish-artifact
description: Publish a Markdown or HTML file to the user's Tailscale tailnet and return a URL teammates can open, and read the comments they leave on it. Use when the user asks to publish, share, or "send someone" a document, report, dashboard, runbook, or page; when a deliverable would be more useful as a link than as terminal output or a local file; or when they ask what feedback a published page has received.
---

# Publish to the tailnet

`art` writes a page into the local artifact store and Tailscale serves it over
HTTPS to everyone on the tailnet. The result is a link, not a file.

## Publish

```bash
art publish REPORT.md --slug weekly-status --favicon 📊 --desc "Week of Aug 18"
```

Prints the URL on stdout. That is the thing to hand back to the user.

- Input may be `.md` or `.html`. Markdown is rendered through the same design
  system, so it looks the same as hand-authored HTML — publish Markdown unless
  the page needs layout that Markdown cannot express.
- An HTML **fragment** (no `<html>` tag) gets wrapped in the standard shell.
  A **full document** is served exactly as written.
- `--slug` is the URL segment and **is the identity of the page**. Republishing
  the same slug updates the page in place, so a link sent last week still works.
  Omit it and it is derived from the title — fine for one-offs, but pass it
  explicitly for anything that will be updated.
- `--asset path` copies a file or directory next to the page; reference it
  relatively (`<img src="diagram.png">`).
- `--favicon` takes one emoji. Keep it stable across republishes — people find
  the tab by its icon.
- `--no-comments` publishes without the commenting sidebar. Default is on.

## Writing the page first

If you are authoring the HTML rather than publishing a file the user already has,
**load the `artifact-design` skill before writing it.** It carries the template,
the shared token set, and the calibration for how much design the page warrants.

## Comments

Readers can select any passage and comment on it. Threads are attributed
automatically from Tailscale identity, and they survive republishing — they
re-anchor to their quoted text.

**Read them before revising a published page.** If the user asks you to update an
artifact that has open comments, the comments are the requirements:

```bash
art comments <slug>                                  # open threads + deep links
art comments <slug> --json                           # same, machine-readable
art comments reply <slug> <thread-id> "…"            # answer in the thread
art comments resolve <slug> <thread-id>              # once you have acted on it
```

Resolve a thread when you have actually addressed it — made the change, or
established none was needed. Do not resolve feedback you are ignoring; leave it
open and say so. A short reply saying what you changed is worth more than a
silent resolve.

When you republish after acting on comments, anchored threads reattach on their
own. A thread whose quoted text you rewrote becomes *text no longer on the page*
and moves to the top of the rail — that is expected, not a failure, but it is a
good reason to reply before you rewrite the passage.

Turn commenting off for an artifact that does not want it:

```bash
art comments off <slug>     # or publish with --no-comments
```

## Other commands

```bash
art list            # every artifact with its URL, plus comment counts
art status          # store path, node, base URL, whether serving is configured
art open <slug>     # open in the browser
art url <slug>      # just the URL
art rm <slug>       # delete
art vendor          # download mermaid locally, so diagrams work offline
```

## When it is not set up

If `art publish` warns that Tailscale is not serving the store, run:

```bash
art serve                 # directory backend — no process to keep running
art serve --mode proxy    # fallback: local static server + launchd agent
```

Use `--mode proxy` if the directory backend fails; the sandboxed macOS Tailscale
app cannot always read files under the home directory.

## Tell the user the truth about the link

Say these plainly when handing over a URL, once per session:

- It works **only for people on the same tailnet**, and only while this machine
  is awake and online. It is not a public link and not a durable one.
- Nothing here is access-controlled beyond tailnet membership — anyone on the
  tailnet can read any artifact. Do not publish secrets, credentials, or customer
  data on that basis.

## Do not expose it publicly

`tailscale funnel` would put an artifact on the public internet. `art` will not
do that, and neither should you — not even when it seems convenient. If the user
wants it, they run `tailscale funnel` themselves, deliberately.
