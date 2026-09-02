# Artifacts

Publish a Markdown or HTML file to your Tailscale tailnet and get back a URL your
teammates can open and comment on. A self-hosted stand-in for Claude's Artifacts,
built out of a CLI and two skills, so it works anywhere that can run bash.

```console
$ art publish q3-threat-model.md --favicon 🛡️ --desc "Where the rewrite changes our exposure"
published: Q3 Threat Model
https://your-laptop.tailnet-name.ts.net:8443/q3-threat-model/
```

Requires [uv](https://astral.sh/uv) and Tailscale.

## How it works

There is no server to run in the `dir` backend: `tailscale serve` publishes a
local directory over HTTPS and tailscaled handles TLS, auth and static files.
"Publishing" is writing into that directory.

```text
~/.local/share/artifacts/
├── index.html                   gallery, rebuilt on every publish
├── _assets/                     base.css, code.css, topbar.*, comments.*
├── _comments/<slug>.json        threads, kept outside the page
└── <slug>/index.html + meta.json
```

## What it installs

- `~/.local/bin/art` — symlink to the CLI.
- `~/.local/share/toms-tools/artifacts/` — the payload: `bin/art`, `assets/`
  (the shared design system and commenting UI), and `skills/`.
- `artifact-design` and `publish-artifact` skills, symlinked into
  `~/.claude/skills` and `~/.codex/skills` for whichever hosts are present.

The skills are symlinked rather than copied, so both hosts read the same files.
A real directory left by an earlier install is backed up (`.bak.<timestamp>`)
before being replaced.

## One-time setup

```sh
art serve      # point tailscale at the store
art status     # store path, node, base URL, serve state
```

There is no server to run in the `dir` backend — `tailscale serve` publishes the
directory itself. On macOS `art serve` picks `proxy` instead, because the
Tailscale network extension is sandboxed and 403s every file under `dir`.
Comments need `proxy`. Force either with `art serve --mode dir|proxy`, stop with
`art serve --off`.

## Commands

| Command | What it does |
|---|---|
| `art publish <file>` | publish `.md` or `.html`; prints the URL |
| `art list [--json]` | every artifact with its URL |
| `art status` | store, node, base URL, serve state |
| `art open <slug>` | open in a browser |
| `art url <slug>` | print the URL |
| `art rm <slug>...` | delete |
| `art serve [--mode auto/dir/proxy] [--off]` | configure Tailscale serving |
| `art comments [slug]` | show open threads (`--resolved`, `--json`) |
| `art comments reply <slug> <thread> "…"` | reply from the terminal |
| `art comments resolve <slug> <thread>` | resolve (`--reopen` to undo) |
| `art comments off` / `on <slug>` | turn commenting off or back on |
| `art vendor` | download mermaid locally so diagrams work offline |

`publish` flags: `--title` `--slug` `--desc` `--favicon` `--asset` (repeatable),
`--no-comments`.

Environment: `ART_HOME` (store path, default `~/.local/share/artifacts`),
`ART_PORT` (default `8443`), `ART_HOST` (override the node's DNS name).

**Slugs are stable.** Republishing the same slug updates the page at the URL you
already sent — the property that makes the link worth sending.

## The skills

- **`publish-artifact`** — thin wrapper over the CLI. Teaches an agent to publish
  and, importantly, to tell the user the truth about the link's limits.
- **`artifact-design`** — the substantive one. Calibrates how much design a page
  warrants, then hands over `template.html` and the shared token set.

Both are plain directories with a `SKILL.md`, symlinked into every host found so
editing one takes effect in both at once. Set `ART_SKILL_DIR` (Claude) or
`CODEX_HOME` (Codex) to override, or to install for a host not present yet; for
any other harness, symlink the payload's `skills/*` wherever it looks.

Verify Codex picked them up:

```sh
codex debug prompt-input | grep -o 'publish-artifact\|artifact-design'
```

## The header

Every artifact gets a sticky header — back to the gallery, the title, when it was
last updated, and **Share**, which copies the link. It is injected at serve time
from `meta.json`, so hand-authored pages get it too and it cannot go stale.
Opening a thread writes `?comment=<id>` into the URL, so sharing with a thread
open hands over a link that opens on that comment.

## Comments

Readers select any passage and comment on it. Identity comes from Tailscale —
`tailscale serve` stamps the user headers onto every proxied request and the
server binds `127.0.0.1`, so nobody signs in and nobody can post as someone else.

Threads are keyed to a quoted passage plus surrounding context, not a DOM
position, so they survive republishing. When the quoted text is genuinely gone
the thread moves to the top of the rail labelled *text no longer on the page*.

Asset URLs are stamped with each file's mtime, so editing `base.css` reaches
readers on their next load rather than when a cache lapses. Also included:
threaded replies, resolve/reopen, live updates over SSE, drafts in
`localStorage`, and `⌘↵` to post.

`art comments off <slug>` means the widget is not injected **and** the API
refuses writes for that slug — not merely hidden. Existing threads are kept and
return if you turn it back on, and republishing does not silently re-enable it.

## Authoring pages

Markdown goes through the same design system as hand-authored HTML, so publish
Markdown unless you need layout it cannot express. Fenced `mermaid` blocks become
diagrams, and Python-Markdown's `admonition`, `attr_list`, `toc` and table
extensions are all on.

For HTML, start from the `artifact-design` skill's `template.html` and link
`/_assets/base.css` rather than writing your own styles. There is no CSP, so
external scripts, relative assets and shared files under `/_assets/` all work —
the main thing this has over hosted artifacts.

## Limits, stated plainly

- The URL resolves **only for people on the same tailnet**, and only while this
  machine is awake and online. Not public, not durable.
- Nothing is access-controlled beyond tailnet membership. Anyone on the tailnet
  can read any artifact — do not publish secrets or customer data.
- Reachability depends on your tailnet ACLs. If your policy restricts ports,
  teammates may not reach `:8443` on your node — test with one person first.
- Exposing an artifact to the public internet is not automated: `art public`
  deliberately refuses, so `tailscale funnel` stays something you type yourself.
- For something durable, run the same store and `tailscale serve` on an always-on
  host. Nothing else changes.
