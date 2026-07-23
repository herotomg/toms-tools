# PR Fixer (Claude Code)

Installs a Claude Code slash command that fixes unresolved PR review comments in one pass — running inline in your main session (no subagent).

## What it installs

- `~/.claude/commands/fix-pr.md` — the `/fix-pr` slash command. It carries the full
  instructions and runs directly in your session, so it uses your current model and context.

Existing files are backed up before being overwritten. A legacy `pr-fixer` agent from
earlier versions, if present, is retired (backed up and removed).

## Usage

- In Claude Code, run `/fix-pr` on the PR branch you want to fix (optionally pass a PR number).
- It fetches unresolved comments via `gh unresolved`, fixes them, runs tests, then submits, replies, and resolves each addressed comment thread.
