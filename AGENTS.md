# AGENTS

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