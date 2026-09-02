use std::{
    env,
    path::{Path, PathBuf},
};

/// Expand a registry path like `~/.local/bin/art` against `$HOME`.
///
/// Registry manifests are written by hand, so they use `~`. Everything else is
/// taken literally — no globbing, no environment interpolation. A tool that
/// cannot name its files literally keeps a `status_check` instead.
pub fn expand(path: &str) -> PathBuf {
    let trimmed = path.trim();

    let Some(rest) = trimmed.strip_prefix('~') else {
        return PathBuf::from(trimmed);
    };

    let Some(home) = env::var_os("HOME") else {
        return PathBuf::from(trimmed);
    };

    match rest.strip_prefix('/') {
        Some(rest) => Path::new(&home).join(rest),
        // Bare `~`, or another user's home (`~alice`) which we do not resolve.
        None if rest.is_empty() => PathBuf::from(home),
        None => PathBuf::from(trimmed),
    }
}

/// `true` when the path exists, including a symlink whose target does not.
/// A dangling symlink is still something we installed and still something
/// `tt remove` must clean up.
pub fn exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use super::{exists, expand};

    #[test]
    fn expands_a_leading_tilde_to_home() {
        let home = env::var("HOME").expect("HOME is set in the test environment");
        assert_eq!(
            expand("~/.local/bin/art"),
            PathBuf::from(format!("{home}/.local/bin/art"))
        );
        assert_eq!(expand("~"), PathBuf::from(&home));
    }

    #[test]
    fn leaves_other_paths_alone() {
        assert_eq!(
            expand("/usr/local/bin/art"),
            PathBuf::from("/usr/local/bin/art")
        );
        assert_eq!(expand("relative/path"), PathBuf::from("relative/path"));
        // We do not resolve another user's home directory.
        assert_eq!(expand("~alice/bin"), PathBuf::from("~alice/bin"));
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(expand("  /tmp/x  "), PathBuf::from("/tmp/x"));
    }

    #[test]
    fn a_dangling_symlink_counts_as_existing() {
        let dir = env::temp_dir().join(format!("tt-paths-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let link = dir.join("dangling");
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(dir.join("nothing-here"), &link).unwrap();

        assert!(exists(&link), "a broken symlink is still installed state");
        assert!(!link.exists(), "std::path::Path::exists follows the link");

        std::fs::remove_dir_all(&dir).ok();
    }
}
