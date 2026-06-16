# PR Fixer (Claude Code)

Installs a Claude Code agent and slash command that fix unresolved PR review comments in one pass.

## What it installs

- `~/.claude/agents/pr-fixer.md` — the `pr-fixer` agent (fixes the comments itself; does not delegate).
- `~/.claude/commands/fix-pr.md` — the `/fix-pr` slash command that runs the agent.

Existing files are backed up before being overwritten.

## Usage

- In Claude Code, run `/fix-pr` on the PR branch you want to fix (optionally pass a PR number).
- It fetches unresolved comments via `gh unresolved`, fixes them, runs tests, then submits and replies.
