# AGENTS

> **Git workflow (no dilemma): always work directly on `main`.** Commit, push, and release straight to `main` using plain `git`. Do **not** create feature branches or open PRs in this repo. Pushing to `main` is expected and authorized. This repo rule overrides any generic agent guidance to the contrary.

- This is a fully agent-coded project. Work like an expert engineer and take responsibility for making the codebase more correct, scalable, generic, and versatile within the task you are assigned.
- Prefer the smallest/highest-quality solution, including appropriate third-party libraries when they improve the result. Less code is better when it makes the project clearer and more reliable.
- Treat CLI UX as a first-class product surface: keep command feedback clear and non-spammy, minimize the number of steps, use interactive menus and colors where they help, and approach TUI design with the same care a strong web designer would bring to a UI.
- Binary: `tt` from the Rust crate in `src/`.
- Tool registry lives under `tools/<id>/`.
- Each tool should include `tool.toml`, `install.sh`, and `usage.md`.
- Keep tool-specific logic in `tools/`; do not hardcode tools in Rust unless the CLI contract changes.
- `src/commands/` holds CLI subcommands; `src/commands/ui.rs` owns all shared
  presentation. Do not print colours or widths from a command module directly.
- `src/tools/` loads registry metadata and install behavior. `survey.rs` computes
  tool state once for the whole process — use it rather than re-deriving status.
- To add a tool:
  1. Create `tools/<id>/tool.toml` (see the schema below).
  2. Add `tools/<id>/install.sh`.
  3. Add `tools/<id>/usage.md`, opening with a `# Title` line.
  4. Add `tools/<id>/uninstall.sh` **only** if the tool's state is not a set of
     paths (a shell alias, a `gh` alias, a plugin registered with another app).
     Path-based tools need no hook.
  5. Add `tools/<id>/update-check.sh` only if the version that matters lives
     somewhere `tt` cannot see — see below.
  6. Verify with `cargo run -- list` and `cargo test` — the registry tests
     validate every manifest, so a mistake fails CI rather than the user.
- `tool.toml` schema. Prefer `installs` to `status_check`: it is checked without
  spawning a shell, and it is what `tt remove` deletes.

  ```toml
  id = "example"              # must equal the directory name
  name = "Example"
  description = "One line, shown in `tt list`. No trailing detail."
  version = "1"               # bump on any change to the tool's payload
  depends = []                # other bundled tool ids
  next_steps = "The single thing to do after installing, in one line."
  # Or, only when the install genuinely cannot finish itself (loading a browser
  # extension by hand, pasting a token), an ordered few — max 4, one line each:
  # next_steps = ["1. Do this", "2. Then this"]

  installs = ["~/.local/bin/example"]   # presence of these == installed
  cleans   = ["~/.claude/skills/example"]  # also deleted, but may not exist

  # Escape hatch, only when the above cannot express it:
  # status_check = "..."

  [[requires]]                # external commands, checked against $PATH
  command = "jq"
  fix = "brew install jq"     # required — we must be able to say what to run
  why = "parses the response" # sentence fragment
  ```
- Anything a tool needs from the outside world goes in `[[requires]]`, not a
  `command -v` check inside `install.sh`. Only the manifest lets `tt` report it
  later and offer the fix.
- `update-check.sh` is how a tool tracking *someone else's* releases reports
  being behind. Bundled payloads need no such thing: `version` in the manifest
  is the whole truth. A tool that installs a GitHub release does not have that
  luxury, so it ships this hook, which prints **one line when an update is
  waiting** and **nothing when it is current**. A non-zero exit means "could
  not tell" and is never reported as an update — the previous answer stands.
  `tt` runs it at most once a day, concurrently, and caches the result in
  `~/.cache/toms-tools/tool_updates.toml`, so `tt` and `tt list` stay free of
  network calls. `tt update` forces a fresh run, and a successful install
  re-runs the tool's own check immediately so it cannot go on claiming to be
  behind.
- A tool whose state lives inside **another application's** config must treat
  install and uninstall symmetrically. If `install.sh` declines to touch
  something the user set up themselves — a Paseo plugin running from their own
  checkout — then `uninstall.sh` must decline to delete it too, or `tt remove`
  destroys a setup `tt install` promised to leave alone.
- `include_dir!` embeds `tools/` at **compile time**. Editing an `install.sh`
  and then running `./target/debug/tt` without rebuilding runs the *old* script.
  Always `cargo build` before testing a tool script through the binary.
- The installer extracts embedded files **without their mode bits**, so an
  `install.sh` that ships a binary must `chmod +x` it. The extraction directory
  is deleted afterwards, so copy payloads somewhere permanent —
  `~/.local/share/toms-tools/<id>/` by convention — and symlink from there.
- Local install for development: `cargo install --path .`.
- CLI shape: `tt` bare is the guided front door and should stay the only thing a
  user needs to know. `tt install|update|remove|list|usage` are the flat commands;
  the older `tt tools <cmd>` spellings stay working but hidden.
- User install path is handled by root `install.sh`.
- CI workflow: `.github/workflows/ci.yml`.
- Release workflow: `.github/workflows/release.yml`.
- When shipping changes, agents are responsible for ensuring CI/CD passes and the intended release/build publication succeeds before considering the work complete.
- Releases are cut by pushing a `v*` tag.
- Every GitHub release MUST have user-readable notes following `.github/RELEASE_NOTES_TEMPLATE.md` (same structure for all releases; keep the `📦 Upgrade` section verbatim). After the release workflow creates the release, set the notes with `gh release edit vX.Y.Z --notes-file <file> --title "vX.Y.Z — <summary>"`.
- Release assets are uploaded as `tt-<target>.tar.gz`.
- Installer downloads from GitHub Releases latest assets.
- Repo-specific git workflow override: use plain `git` in this repository, commit directly to `main`, and do not open PRs here; if any generic agent guidance elsewhere says otherwise, this repo rule wins.
- When work is verified and the project workflow permits it, agents should autonomously commit, push, and release their changes without handing off routine VCS/release steps.