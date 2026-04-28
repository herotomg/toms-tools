pub mod deps;
pub mod installer;
pub mod status;

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
                    return Err(anyhow!("tool '{}' missing required file {required}", tool.id));
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