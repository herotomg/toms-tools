# Claude Artifacts

Every Claude session Paseo starts gets Claude Code's native **Artifact** tool —
publish a page to claude.ai, update it in place, and read any artifact link you
are handed — without passing an environment flag per session.

## What this installs

One value in Paseo's config:

```json
"agents": { "providers": { "claude": { "env": { "CLAUDE_CODE_ARTIFACT": "1" } } } }
```

It is written with `paseo daemon config set`, which validates the config and
reloads a running daemon, so nothing needs restarting. The rest of
`agents.providers` — other providers, other env vars — is merged back exactly
as it was. Installing again when the flag is already on changes nothing.

Installing does not publish, fetch or open any artifact, and does not touch
Claude's sign-in or permission settings.

## Which sessions get it

- **New sessions** and sessions you **reload** pick it up.
- **Sessions already running** keep the environment they started with. Reload
  them to get the tool.

## Using it

Ask the agent to make a page, or hand it a link. To read an existing artifact,
the agent calls the tool with an explicit read action rather than fetching the
URL:

```text
Artifact(action="read", url="https://claude.ai/artifact/…")
```

Pages start private to your claude.ai account; sharing them is a separate,
deliberate step in claude.ai.

## Removing it

`tt remove claude-artifacts` deletes the flag only if `tt` set it. If it was
already on before you installed, it is yours and is left alone. Running sessions
keep the tool until they are reloaded.
