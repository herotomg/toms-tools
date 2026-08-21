# PR Fixer (Claude Code)

Installs a Claude Code slash command that fixes unresolved PR review comments — running inline
in your main session for a single PR, or fanning out to subagents when fixing a stack of PRs.

## What it installs

- `~/.claude/commands/fix-pr.md` — the `/fix-pr` slash command. It carries the full
  instructions and runs directly in your session, so it uses your current model and context.

Existing files are backed up before being overwritten. A legacy `pr-fixer` agent from
earlier versions, if present, is retired (backed up and removed).

## Usage

- In Claude Code, run `/fix-pr` on the PR branch you want to fix (optionally pass a PR number).
- It fetches unresolved comments via `gh unresolved`, fixes them, runs tests, then submits, replies, and resolves each addressed comment thread.
- Pass multiple PRs (or ask to fix "the whole stack") and it processes them bottom to top,
  one PR fully at a time (fix → submit → reply+resolve), dispatching a subagent per PR so the
  work doesn't pile up in your context.
