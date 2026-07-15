# General-Purpose Agent Matrix (Claude Code)

Installs 15 variants of the built-in `general-purpose` Claude Code subagent, one
per (model x effort) combination, so you can pick reasoning depth and cost
explicitly when spawning an agent.

## What it installs

- `~/.claude/agents/general-purpose-<model>-<effort>.md` for:
  - model: `sonnet`, `opus`, `fable`
  - effort: `low`, `medium`, `high`, `xhigh`, `max`

Existing files are backed up (`.bak.<timestamp>`) before being overwritten.

## Usage

In Claude Code, spawn one of the 15 agents by name, e.g.:

- `general-purpose-opus-low` — cheap opus-quality pass
- `general-purpose-sonnet-high` — sonnet at high reasoning effort
- `general-purpose-fable-max` — fable at maximum reasoning effort

Use the Agent tool's `subagent_type` field, or ask Claude to use one of these agents directly.
