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
tt tools usage --all
tt --check-update
```

Use `tt --check-update` to force a fresh update check instead of waiting for the cached daily check.

## Tools

| Tool | Description |
| --- | --- |
| `gh-unresolved` | Install the `gh unresolved` command to list unresolved CR comments on a PR. |
| `intent-pr-fixer` | Install the PR Fixer Intent specialist agent for one-shot CR comment fixing. |

Install a single tool by id:

```sh
tt tools install gh-unresolved
```

## Usage

```sh
tt tools list
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
