# jsut → just

Adds a zsh alias so the common `jsut` typo just works.

## What it does

- Appends `alias jsut='just'` to your `~/.zshrc` (idempotently, with a backup).

## After install

- Run `source ~/.zshrc` or open a new shell, then `jsut <recipe>` runs `just <recipe>`.
