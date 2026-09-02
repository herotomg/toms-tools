# toms-tools

`tt` installs and maintains Tom's helper tools — CLIs, shell aliases, and Claude
Code / Codex skills — from the bundled `tools/` registry.

## Start here

```sh
curl -fsSL https://raw.githubusercontent.com/herotomg/toms-tools/main/install.sh | bash
tt
```

That is the whole thing. **Run `tt` with no arguments** and it tells you what
state you are in and offers to fix it — missing tools, outdated ones, a
dependency that is not installed, a `PATH` that will not find what you install.

```console
$ tt

  tt v0.1.23 · 6 of 7 tools installed

  Installed, but missing something
    artifacts
      uv not found — runs the art CLI
      brew install uv

  Not installed
    ○ gh-unresolved    A `gh unresolved` command that lists unresolved review comments…

  ? What would you like to do? ›
  ❯ Run `brew install uv` to fix what is missing
    Install the 1 tool I do not have
    Show me what my tools do
    Nothing right now
```

Everything below is a shortcut for something that menu already offers.

## Commands

| | |
|---|---|
| `tt` | show what needs doing, and offer to do it |
| `tt install [ids]` | install tools; no arguments opens a checklist |
| `tt update [ids]` | update `tt` itself and any outdated tools |
| `tt remove <ids>` | remove tools, showing what will be deleted first |
| `tt list` | every tool and its status |
| `tt usage [id]` | what a tool does; with an id, its full page |
| `tt completions install` | shell completions |

`tt install --all` takes the lot. `tt update --self` updates only the binary.
`TT_NO_UPDATE_CHECK=1` silences the once-a-day release check.

## The tools

Descriptions live in each `tools/<id>/tool.toml` and are printed by `tt list` —
this table is deliberately not a second copy of them.

- **`artifacts`** — publish Markdown or HTML to your tailnet as a page teammates
  can read and comment on. Bundles the `art` CLI and two agent skills.
- **`preview-gif`** — record preview GIFs of a UI with a visible cursor and
  clicks. Bundles the `screengif` recorder.
- **`agent-matrix`** — the general-purpose Claude Code subagent in 15 model ×
  effort variants.
- **`pr-fixer`** — a `/fix-pr` slash command that fixes unresolved PR review
  comments.
- **`gh-unresolved`** — a `gh unresolved` command listing unresolved review
  threads.
- **`gtms-alias`**, **`jsut-alias`** — small zsh aliases.

## Where things go

| | |
|---|---|
| `~/.local/bin/` | binaries (`art`, `screengif`), symlinked |
| `~/.local/share/toms-tools/<id>/` | each tool's payload |
| `~/.local/share/toms-tools/installed.toml` | recorded versions |
| `~/.local/share/toms-tools/backups/` | anything replaced during an install |
| `~/.claude/skills/`, `~/.codex/skills/` | skills, symlinked to the payload |

Skills are symlinked rather than copied, so both agent hosts read the same files.

## Development

```sh
cargo install --path .    # local install
cargo run -- list
cargo test                # includes validation of every bundled manifest
```

Adding a tool, and the `tool.toml` schema, are documented in [AGENTS.md](AGENTS.md).

## Releasing

```sh
git tag v0.1.23 && git push origin v0.1.23
```

The release workflow builds four targets and creates the release; notes follow
`.github/RELEASE_NOTES_TEMPLATE.md`.
