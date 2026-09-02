use std::{fs, path::Path};

use anyhow::{Context, Result};

use super::{installer, paths, status, EmbeddedTool};

/// What removing a tool actually did, so the caller can report it honestly
/// rather than claiming success for a no-op.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Removal {
    /// Paths that existed and are now gone.
    pub removed: Vec<String>,
    /// Paths the manifest claims but which were not there. Not an error — a
    /// skill is only linked into Codex if Codex is installed.
    pub absent: Vec<String>,
    /// Whether the tool's own uninstall.sh ran.
    pub ran_hook: bool,
}

impl Removal {
    pub fn touched_nothing(&self) -> bool {
        self.removed.is_empty() && !self.ran_hook
    }
}

/// Remove a tool: run its uninstall hook if it has one, delete every path it
/// declares, then forget its recorded version.
///
/// The hook runs *first*, while the tool's files are still in place, because a
/// hook may need them (reading a manifest, calling a binary we are about to
/// delete).
pub fn remove(tool: &EmbeddedTool, verbose: bool) -> Result<Removal> {
    let mut removal = Removal::default();

    if has_uninstall_hook(tool) {
        installer::run_hook(tool, "uninstall.sh", verbose)
            .with_context(|| format!("uninstall.sh failed for {}", tool.definition.id))?;
        removal.ran_hook = true;
    }

    for declared in tool.definition.owned_paths() {
        let path = paths::expand(declared);

        if !paths::exists(&path) {
            removal.absent.push(declared.to_owned());
            continue;
        }

        remove_path(&path).with_context(|| format!("failed to remove {}", path.display()))?;
        removal.removed.push(declared.to_owned());
    }

    status::forget_installed_version(&tool.definition.id)?;
    Ok(removal)
}

pub fn has_uninstall_hook(tool: &EmbeddedTool) -> bool {
    tool.dir()
        .get_file(tool.dir().path().join("uninstall.sh"))
        .is_some()
}

/// `symlink_metadata` so we act on the link itself: a symlink to a directory
/// must be unlinked, never recursed into and deleted through.
fn remove_path(path: &Path) -> Result<()> {
    let metadata = path.symlink_metadata()?;

    if metadata.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{env, fs};

    use super::remove_path;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = env::temp_dir().join(format!("tt-remover-{}-{name}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn removes_files_and_directories() {
        let dir = scratch("basic");
        let file = dir.join("file");
        let nested = dir.join("nested");
        fs::write(&file, "x").unwrap();
        fs::create_dir_all(nested.join("deep")).unwrap();
        fs::write(nested.join("deep/f"), "x").unwrap();

        remove_path(&file).unwrap();
        remove_path(&nested).unwrap();

        assert!(!file.exists());
        assert!(!nested.exists());
        fs::remove_dir_all(&dir).ok();
    }

    /// The important one: `~/.claude/skills/preview-gif` is a symlink into the
    /// payload directory. Deleting through it would take the payload with it —
    /// and, for a skills directory, potentially far more.
    #[test]
    fn unlinks_a_symlinked_directory_without_touching_its_target() {
        let dir = scratch("symlink");
        let target = dir.join("payload");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("important"), "keep me").unwrap();

        let link = dir.join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();

        remove_path(&link).unwrap();

        assert!(!link.symlink_metadata().is_ok(), "the link is gone");
        assert!(target.join("important").exists(), "the target survived");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn removes_a_dangling_symlink() {
        let dir = scratch("dangling");
        let link = dir.join("dangling");
        std::os::unix::fs::symlink(dir.join("nothing"), &link).unwrap();

        remove_path(&link).unwrap();

        assert!(link.symlink_metadata().is_err());
        fs::remove_dir_all(&dir).ok();
    }
}
