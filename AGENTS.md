# AGENTS

- This is a fully agent-coded project. Work like an expert engineer and take responsibility for making the codebase more correct, scalable, generic, and versatile within the task you are assigned.
- Prefer the smallest/highest-quality solution, including appropriate third-party libraries when they improve the result. Less code is better when it makes the project clearer and more reliable.
- Treat CLI UX as a first-class product surface: keep command feedback clear and non-spammy, minimize the number of steps, use interactive menus and colors where they help, and approach TUI design with the same care a strong web designer would bring to a UI.
- Binary: `tt` from the Rust crate in `src/`.
- Tool registry lives under `tools/<id>/`.
- Each tool should include `tool.toml`, `install.sh`, and `usage.md`.
- Keep tool-specific logic in `tools/`; do not hardcode tools in Rust unless the CLI contract changes.
- `src/commands/` holds CLI subcommands.
- `src/tools/` loads registry metadata and install behavior.
- To add a tool:
  1. Create `tools/<id>/tool.toml`.
  2. Add `tools/<id>/install.sh`.
  3. Add `tools/<id>/usage.md`.
  4. Verify with `cargo run -- tools list`.
- Local install for development: `cargo install --path .`.
- User install path is handled by root `install.sh`.
- CI workflow: `.github/workflows/ci.yml`.
- Release workflow: `.github/workflows/release.yml`.
- Releases are cut by pushing a `v*` tag.
- Release assets are uploaded as `tt-<target>.tar.gz`.
- Installer downloads from GitHub Releases latest assets.
- Repo-specific git workflow override: use plain `git` in this repository, commit directly to `main`, and do not open PRs or use Graphite/stacked-PR flows here; if any generic agent guidance elsewhere says otherwise, this repo rule wins.
- When work is verified and the project workflow permits it, agents should autonomously commit, push, and release their changes without handing off routine VCS/release steps.