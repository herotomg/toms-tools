# toms-tools

`tt` is a small Rust CLI for discovering, installing, and looking up usage for Tom's curated helper tools from the bundled `tools/` registry.

## Quick start

Install every bundled tool in one go:

```sh
tt tools install --all
```

If you do not have `tt` yet, install it first:

```sh
curl -fsSL https://raw.githubusercontent.com/herotomg/toms-tools/main/install.sh | bash
```

Useful follow-up commands:

```sh
tt tools list
tt tools update
tt tools usage --all
tt update
```

Use `tt update` to update the CLI itself to the latest release.

When a newer release exists, `tt` says so once a day and offers to install it right
there (just press Enter). Set `TT_NO_UPDATE_CHECK=1` to turn the check off.

## Tools

| Tool | Description |
| --- | --- |
| `gh-unresolved` | Install the `gh unresolved` command to list unresolved CR comments on a PR. |
| `jsut-alias` | Install a zsh alias so a `jsut` typo runs `just`. |
| `gtms-alias` | Install a zsh `gtms` alias for `gt modify && gt submit --stack`. |
| `pr-fixer` | Install the pr-fixer Claude Code agent and `/fix-pr` slash command. |
| `agent-matrix` | Install the general-purpose Claude Code subagent as 15 model x effort variants (sonnet/opus/fable x low/medium/high/xhigh/max). |
| `artifacts` | Publish Markdown/HTML to your tailnet as a commentable page: the `art` CLI plus the artifact-design and publish-artifact skills. |
| `preview-gif` | Record preview GIFs of a UI with a visible cursor and clicks: the preview-gif skill plus the `screengif` recorder. |

Install a single tool by id:

```sh
tt tools install gh-unresolved
```

## Usage

```sh
tt tools list
tt tools update [id]
tt tools update --all
tt tools usage
tt tools usage --all
tt tools install [id]
tt tools install --all
tt completions print zsh
tt completions install zsh
tt completions install
```

## Local development

```sh
cargo install --path .
cargo run -- tools list
```

## Adding a new tool

1. Create `tools/<id>/tool.toml` with the tool metadata.
2. Add `tools/<id>/install.sh` to perform the installation.
3. Add `tools/<id>/usage.md` with concise usage notes.

## Releasing

```sh
git tag v0.1.11 && git push origin v0.1.11
```
