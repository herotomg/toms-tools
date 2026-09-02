//! One pass over the registry that answers every question the front door asks.
//!
//! Commands used to each re-derive status in their own loop. Gathering it once
//! keeps a single definition of "needs attention" and means the summary you see
//! and the action you pick can never disagree.

use std::{env, path::PathBuf};

use anyhow::Result;

use super::{status::Status, EmbeddedTool, Registry, Requirement};

/// Where we link binaries. If this is not on `$PATH`, tools install fine and
/// then appear not to exist, which is the most confusing failure available.
pub const BIN_DIR: &str = "~/.local/bin";

pub struct ToolState<'a> {
    pub tool: &'a EmbeddedTool,
    pub status: Status,
    /// Declared external commands that are not on `$PATH`.
    pub missing: Vec<&'a Requirement>,
}

impl ToolState<'_> {
    pub fn id(&self) -> &str {
        &self.tool.definition.id
    }

    /// Installed, but unable to actually run.
    pub fn is_blocked(&self) -> bool {
        self.status.is_installed() && !self.missing.is_empty()
    }
}

pub struct Survey<'a> {
    pub tools: Vec<ToolState<'a>>,
    pub bin_dir_on_path: bool,
}

impl<'a> Survey<'a> {
    pub fn run(registry: &'a Registry) -> Result<Self> {
        let mut tools = Vec::new();

        for tool in registry.tools() {
            let status = Status::detect(&tool.definition)?;
            // Only report missing dependencies for tools the user actually has.
            // Listing what an uninstalled tool would need is noise.
            let missing = if status.is_installed() {
                super::status::missing_requirements(&tool.definition)
            } else {
                Vec::new()
            };

            tools.push(ToolState {
                tool,
                status,
                missing,
            });
        }

        Ok(Self {
            tools,
            bin_dir_on_path: bin_dir_on_path(),
        })
    }

    pub fn installed(&self) -> impl Iterator<Item = &ToolState<'a>> {
        self.tools
            .iter()
            .filter(|state| state.status.is_installed())
    }

    pub fn outdated(&self) -> Vec<&ToolState<'a>> {
        self.filter(|state| matches!(state.status, Status::NeedsUpdate))
    }

    pub fn not_installed(&self) -> Vec<&ToolState<'a>> {
        self.filter(|state| matches!(state.status, Status::NotInstalled))
    }

    pub fn blocked(&self) -> Vec<&ToolState<'a>> {
        self.filter(|state| state.is_blocked())
    }

    fn filter(&self, predicate: impl Fn(&&ToolState<'a>) -> bool) -> Vec<&ToolState<'a>> {
        self.tools.iter().filter(|state| predicate(state)).collect()
    }

    pub fn installed_count(&self) -> usize {
        self.installed().count()
    }

    /// Every distinct fix command needed across all blocked tools, deduplicated
    /// so `brew install ffmpeg` is offered once even if two tools want it.
    pub fn fix_commands(&self) -> Vec<String> {
        let mut commands = Vec::new();
        for state in self.blocked() {
            for requirement in &state.missing {
                if let Some(fix) = &requirement.fix {
                    if !commands.contains(fix) {
                        commands.push(fix.clone());
                    }
                }
            }
        }
        commands
    }

    /// True when there is nothing at all for the user to do.
    pub fn is_all_well(&self) -> bool {
        self.outdated().is_empty()
            && self.not_installed().is_empty()
            && self.blocked().is_empty()
            && self.bin_dir_on_path
    }
}

fn bin_dir_on_path() -> bool {
    let target = super::paths::expand(BIN_DIR);
    let Some(path) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&path).any(|entry| normalise(&entry) == normalise(&target))
}

/// Compare by canonical path where possible so `/Users/x/.local/bin` and a
/// symlinked or trailing-slash variant of it are recognised as the same place.
fn normalise(path: &std::path::Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::normalise;

    #[test]
    fn normalise_is_stable_for_a_path_that_exists() {
        let tmp = std::env::temp_dir();
        assert_eq!(normalise(&tmp), normalise(&tmp.join(".")));
    }

    #[test]
    fn normalise_falls_back_to_the_literal_path() {
        let missing = PathBuf::from("/definitely/not/here/xyzzy");
        assert_eq!(normalise(&missing), missing);
    }
}
