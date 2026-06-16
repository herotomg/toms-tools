<!--
Release notes template for `tt`. Every GitHub release MUST follow this shape so
they all look the same.

Rules:
- GitHub release title: "vX.Y.Z — <short summary>"
- Body starts with "## tt vX.Y.Z" and a one-line summary.
- Include only the sections that apply; delete the ones you don't use.
- ALWAYS keep the "📦 Upgrade" section exactly as written below (it is identical
  across every release).
- Set the notes with: gh release edit vX.Y.Z --notes-file <file> --title "..."
-->
## tt vX.Y.Z

<one-line summary of what this release does>

### ✨ New tools

- **`tool-id`** — what it does, in one line.

### 🔧 Changes

- What changed and why it matters to the user.

### 🗄️ Removed

- What was removed (and where it went, if archived).

### 📦 Upgrade

```sh
tt update              # update the tt CLI itself
tt tools install --all # (re)install / update all bundled tools
```
