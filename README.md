# toms-tools

## What it is

`tt` is a small Rust CLI for discovering and installing Tom's curated helper tools from the bundled `tools/` registry. It gives you a single command for listing available tools, installing one tool by id, or installing everything at once.

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/herotomg/toms-tools/main/install.sh | bash
```

## Usage

```sh
tt tools list
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
git tag v0.1.1 && git push origin v0.1.1
```
