# Send to Paseo

Press **Send to Paseo** on any pull request, type what you want done, and a new
agent starts in the Paseo workspace that belongs to that PR — with a worktree
checked out to the PR if you do not already have one. Works on github.com and on
Graphite.

Every send starts a *new* agent and nothing happens silently: the composer shows
the target it picked and every alternative, and waits for you to press **Send**.

## Two halves, one release

| Half | Where it goes |
|---|---|
| Paseo plugin | cloned and run by Paseo, pinned to the newest release |
| Chrome extension | `~/.local/share/toms-tools/send-to-paseo/extension` |

The two speak a frozen contract and refuse each other when their versions
differ, so `tt` installs both from the *same* release rather than letting them
drift apart.

That directory is a permanent address: Chrome derives an unpacked extension's ID
from the path it was loaded from, and the pairing token is tied to that ID. An
update replaces the contents in place and never moves it, so you load and pair
the extension exactly once.

## Setup, once

1. `chrome://extensions` (or `edge://`, `brave://`, `arc://`) → **Developer
   mode** → **Load unpacked** → pick
   `~/.local/share/toms-tools/send-to-paseo/extension`.
2. In Paseo, open **Send to Paseo** in the sidebar and copy the **pairing
   token**.
3. Click the extension's toolbar icon — or the cog in the composer header —
   paste the token, and press **Test connection**.

After an update, press the extension's **reload** button on
`chrome://extensions` so Chrome picks up the new files.

## How it picks a workspace

The page URL is the only source of PR identity; everything else is resolved on
the daemon side. Candidates are ranked: an exact branch match, then another
branch in the same stack (nearest first), then any workspace in the project,
then a synthetic *create* option. Stacked PRs resolve to the workspace you
already have, and the agent's prompt names the branch the change belongs on.

## `gh` is optional

Without the GitHub CLI, sending still works — Paseo checks the PR out with its
own credentials. What you lose is the PR title, the branch names and stack
detection, so every workspace ranks as "same project" and the default becomes
*create*. The target picker names the reason. `gh auth login` is what makes
titles and stacks work.

## Security

The bridge can start agents that execute code, so it binds `127.0.0.1` only,
requires a bearer token on every endpoint except a liveness ping, rejects any
request whose `Origin` is not a `chrome-extension://` one, validates the `Host`
header, and caps bodies at 64 KiB. The token lives only in the extension's
service worker, never in the content script.

## Removing

`tt remove send-to-paseo` removes the Paseo plugin and deletes the extension
directory. Chrome will then show the extension as broken — remove it from
`chrome://extensions` yourself. The pairing token in
`$PASEO_HOME/plugin-data/send-to-paseo/` is left alone.

## Requires

Paseo 0.7.0 or newer with plugins enabled, and `git`. `gh` is optional but
recommended.
