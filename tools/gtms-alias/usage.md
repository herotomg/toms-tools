# gtms (Graphite modify + submit)

Adds a zsh `gtms` alias that amends the current commit and submits the stack in one go.

## What it does

- Appends `alias gtms='gt modify && gt submit --stack'` to your `~/.zshrc` (idempotently, with a backup).

## After install

- Run `source ~/.zshrc` or open a new shell, then `gtms` runs `gt modify && gt submit --stack`.
