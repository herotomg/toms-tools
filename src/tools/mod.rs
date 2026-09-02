pub mod deps;
pub mod installer;
pub mod status;
pub mod usage;

use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use include_dir::{include_dir, Dir};
use serde::Deserialize;

static TOOLS_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/tools");

#[derive(Debug, Clone, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub depends: Vec<String>,
    pub status_check: String,
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
            if let Err(error) = parses_as_bash(&tool.definition.status_check) {
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
