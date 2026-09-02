pub mod deps;
pub mod installer;
pub mod paths;
pub mod remover;
pub mod status;
pub mod survey;
pub mod usage;

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Deserialize;

static TOOLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tools");

/// An external program a tool needs in order to work. Declared here rather than
/// checked inside install.sh so that `tt` can report — and offer to fix — a
/// missing dependency at any time, not just once during an install that has
/// already scrolled off the screen.
#[derive(Debug, Clone, Deserialize)]
pub struct Requirement {
    /// The command to look for on `$PATH`.
    pub command: String,
    /// The exact command a user should run to get it. Shown verbatim.
    pub fix: Option<String>,
    /// What the tool needs it for, as a sentence fragment: "runs the art CLI".
    pub why: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    /// Other bundled tools that must be installed first.
    #[serde(default)]
    pub depends: Vec<String>,
    /// External commands this tool needs.
    #[serde(default)]
    pub requires: Vec<Requirement>,
    /// Paths the tool creates unconditionally. Their presence *is* the install
    /// status, and `tt remove` deletes them. Prefer this to `status_check`.
    #[serde(default)]
    pub installs: Vec<String>,
    /// Extra paths to delete on removal if they happen to exist — things
    /// created conditionally, like a skill linked only when Codex is present.
    /// Never consulted for status.
    #[serde(default)]
    pub cleans: Vec<String>,
    /// Escape hatch for tools whose installed state is not a set of paths — a
    /// `gh` alias, a line in `.zshrc`. Takes precedence over `installs`.
    pub status_check: Option<String>,
    /// The one thing to do next, in a single line. Shown after install instead
    /// of the tool's whole manual.
    pub next_steps: Option<String>,
}

#[cfg(test)]
impl Tool {
    /// A minimal valid tool, for tests that only care about one field.
    pub fn fixture(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: id.to_owned(),
            description: format!("the {id} tool"),
            version: "1".to_owned(),
            depends: Vec::new(),
            requires: Vec::new(),
            installs: Vec::new(),
            cleans: Vec::new(),
            status_check: Some("true".to_owned()),
            next_steps: None,
        }
    }
}

impl Tool {
    /// Every path this tool is responsible for, in removal order.
    pub fn owned_paths(&self) -> impl Iterator<Item = &str> {
        self.installs
            .iter()
            .chain(self.cleans.iter())
            .map(String::as_str)
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddedTool {
    pub definition: Tool,
    pub(crate) dir: &'static Dir<'static>,
}

impl EmbeddedTool {
    pub fn dir(&self) -> &'static Dir<'static> {
        self.dir
    }
}

#[derive(Debug, Clone)]
pub struct Registry {
    pub(crate) tools: BTreeMap<String, EmbeddedTool>,
}

impl Registry {
    pub fn load() -> Result<Self> {
        let mut tools = BTreeMap::new();

        for dir in TOOLS_DIR.dirs() {
            let dir_name = dir
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or_else(|| anyhow!("invalid embedded tool directory name"))?;
            let manifest = dir
                .get_file(dir.path().join("tool.toml"))
                .context("missing tool.toml")?
                .contents_utf8()
                .context("tool.toml is not valid UTF-8")?;
            let tool: Tool = toml::from_str(manifest)
                .with_context(|| format!("failed to parse tool.toml for {dir_name}"))?;

            if tool.id != dir_name {
                return Err(anyhow!(
                    "tool id '{}' does not match directory name '{dir_name}'",
                    tool.id
                ));
            }

            for required in ["install.sh", "usage.md"] {
                if dir.get_file(dir.path().join(required)).is_none() {
                    return Err(anyhow!(
                        "tool '{}' missing required file {required}",
                        tool.id
                    ));
                }
            }

            if tool.status_check.is_none() && tool.installs.is_empty() {
                return Err(anyhow!(
                    "tool '{}' must declare `installs` paths or a `status_check`; \
                     without one there is no way to tell whether it is installed",
                    tool.id
                ));
            }

            tools.insert(
                tool.id.clone(),
                EmbeddedTool {
                    definition: tool,
                    dir,
                },
            );
        }

        Ok(Self { tools })
    }

    pub fn embedded_tool_ids() -> Vec<&'static str> {
        let mut ids = TOOLS_DIR
            .dirs()
            .filter_map(|dir| dir.path().file_name().and_then(|name| name.to_str()))
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub fn get(&self, id: &str) -> Option<&EmbeddedTool> {
        self.tools.get(id)
    }

    pub fn tool_ids(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn tools(&self) -> impl Iterator<Item = &EmbeddedTool> {
        self.tools.values()
    }
}

/// These tests load the *real* bundled registry. Without them a malformed
/// tool.toml — a bad id, a dangling dependency, a status check that will not
/// parse — ships green, because every other test builds its own fixtures and
/// `Registry::load` is otherwise only ever called from `cli::run`.
#[cfg(test)]
mod registry_tests {
    use std::{
        io::Write,
        process::{Command, Stdio},
    };

    use super::{deps, Registry};

    fn registry() -> Registry {
        Registry::load().expect("the bundled registry must load")
    }

    fn parses_as_bash(script: &str) -> Result<(), String> {
        let bash = which::which("bash").expect("bash is required to run the test suite");
        let mut child = Command::new(bash)
            .arg("-n")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn bash");
        child
            .stdin
            .take()
            .expect("stdin is piped")
            .write_all(script.as_bytes())
            .expect("failed to write script to bash");
        let output = child.wait_with_output().expect("failed to wait for bash");

        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
        }
    }

    #[test]
    fn bundled_registry_loads_and_is_not_empty() {
        assert!(!registry().tool_ids().is_empty());
    }

    #[test]
    fn embedded_ids_match_loaded_ids() {
        let loaded = registry().tool_ids();
        let mut embedded = Registry::embedded_tool_ids();
        embedded.sort_unstable();

        assert_eq!(loaded, embedded);
    }

    #[test]
    fn every_declared_dependency_exists() {
        let registry = registry();
        for tool in registry.tools() {
            for dependency in &tool.definition.depends {
                assert!(
                    registry.get(dependency).is_some(),
                    "{} depends on unknown tool '{dependency}'",
                    tool.definition.id
                );
            }
        }
    }

    #[test]
    fn dependency_graph_is_acyclic() {
        let registry = registry();
        deps::resolve_install_order(&registry, &registry.tool_ids())
            .expect("the bundled dependency graph must be acyclic");
    }

    #[test]
    fn every_status_check_parses_as_bash() {
        for tool in registry().tools() {
            let Some(check) = &tool.definition.status_check else {
                continue;
            };
            if let Err(error) = parses_as_bash(check) {
                panic!(
                    "{} has an unparseable status_check: {error}",
                    tool.definition.id
                );
            }
        }
    }

    #[test]
    fn every_install_script_parses_as_bash() {
        for tool in registry().tools() {
            let path = tool.dir().path().join("install.sh");
            let script = tool
                .dir()
                .get_file(&path)
                .unwrap_or_else(|| panic!("{} is missing install.sh", tool.definition.id))
                .contents_utf8()
                .unwrap_or_else(|| panic!("{}'s install.sh is not UTF-8", tool.definition.id));

            if let Err(error) = parses_as_bash(script) {
                panic!(
                    "{} has an unparseable install.sh: {error}",
                    tool.definition.id
                );
            }
        }
    }

    /// A `\|` inside a table cell is valid Markdown but reaches the terminal
    /// with its backslash intact. Write the cell without a pipe instead.
    #[test]
    fn usage_tables_do_not_escape_pipes() {
        for tool in registry().tools() {
            let usage = super::usage::read(tool).unwrap();
            for (number, line) in usage.lines().enumerate() {
                let line = line.trim();
                if line.starts_with('|') && line.contains("\\|") {
                    panic!(
                        "{}: usage.md line {} escapes a pipe inside a table; \
                         rewrite the cell without one",
                        tool.definition.id,
                        number + 1
                    );
                }
            }
        }
    }

    #[test]
    fn every_tool_can_report_whether_it_is_installed() {
        for tool in registry().tools() {
            let tool = &tool.definition;
            assert!(
                tool.status_check.is_some() || !tool.installs.is_empty(),
                "{} declares neither installs nor status_check",
                tool.id
            );
        }
    }

    #[test]
    fn owned_paths_are_absolute_or_home_relative() {
        for tool in registry().tools() {
            for path in tool.definition.owned_paths() {
                assert!(
                    path.starts_with('~') || path.starts_with('/'),
                    "{}: '{path}' must be absolute or start with ~",
                    tool.definition.id
                );
            }
        }
    }

    /// A requirement with no `fix` is just a complaint — we would be telling
    /// the user something is wrong without telling them what to run.
    #[test]
    fn declared_requirements_are_actionable() {
        for tool in registry().tools() {
            for requirement in &tool.definition.requires {
                assert!(
                    !requirement.command.trim().is_empty(),
                    "{} has a requirement with no command",
                    tool.definition.id
                );
                assert!(
                    requirement.fix.is_some(),
                    "{}: requirement '{}' needs a `fix`",
                    tool.definition.id,
                    requirement.command
                );
            }
        }
    }

    #[test]
    fn every_tool_has_a_titled_usage_doc() {
        for tool in registry().tools() {
            let usage = super::usage::read(tool)
                .unwrap_or_else(|error| panic!("{}: {error:#}", tool.definition.id));
            assert!(
                usage.starts_with("# "),
                "{}'s usage.md must open with a '# Title' heading",
                tool.definition.id
            );
        }
    }

    #[test]
    fn metadata_is_present_and_sane() {
        for tool in registry().tools() {
            let tool = &tool.definition;
            assert!(
                !tool.name.trim().is_empty(),
                "{} has an empty name",
                tool.id
            );
            assert!(
                !tool.description.trim().is_empty(),
                "{} has an empty description",
                tool.id
            );
            assert!(
                !tool.version.trim().is_empty(),
                "{} has an empty version",
                tool.id
            );
            assert!(
                tool.id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "{} must be lowercase kebab-case",
                tool.id
            );
        }
    }
}
