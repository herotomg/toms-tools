# Paseo Defer

Queue a message to a Paseo agent and have it delivered later — after a delay, at
a time you name, or the moment your Claude usage window resets. When the message
comes due it waits for the agent to go idle, so it arrives as a new message
instead of steering a turn that is already running.

The queue lives on the Paseo daemon, so it survives plugin reloads, restarts and
every client you connect from.

## Where it shows up

- **A pill above the composer**, on every session. It reads `Defer` when nothing
  is waiting and becomes the status (`in 12m`, `2 deferred`) once something is —
  amber if a message is overdue because the session is mid-turn.
- **The Defer panel**, as a workspace tab or in Explorer. A waiting message can
  still be edited, text and timing both, until delivery starts.
- **The Deferred sidebar surface**, which does the same across every session.
- **⌘K / Ctrl+K** → *Defer a message*.

## When it can send

| You pick | You get |
|---|---|
| `15m` / `1h` / `3h` | the preset delay |
| **In…** | any wait you type: `3`, `45m`, `2h`, `1h 30m`, up to 30 days |
| **At…** | a local time, `21:30` or `9:30 pm` |
| **Usage reset** | the moment the Claude rolling window resets |

Both typed fields say what they resolved to — *Sends today at 9:30 PM · in
4h 12m* — before anything is queued.

## What this installs

Nothing of ours. Paseo clones
[tomgrin10/paseo-defer](https://github.com/tomgrin10/paseo-defer) on the daemon
machine, compiles it and runs it, so there is no package manager step and no
dependency to install. `tt` pins the clone to the newest release, and checks once
a day whether a newer one exists — that is what `tt update` acts on.

If you already have the plugin installed from a local checkout, `tt` leaves it
alone rather than swapping your working copy for a clone.

## Data

Queued message text is stored on the daemon machine, at
`$PASEO_HOME/plugin-data/defer/` (`~/.paseo` by default). It survives
`tt remove`, so removing and reinstalling does not lose the queue.

## Requires

Paseo 0.7.0 or newer, with plugins enabled in **Settings → Plugins**. If they
are off, the install stops and says so rather than reporting a success you
cannot see the effect of.
