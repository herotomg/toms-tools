use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};
use serde::{Deserialize, Serialize};

use super::Tool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Installed,
    NotInstalled,
    NeedsUpdate,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct InstalledState {
    #[serde(default)]
    tools: BTreeMap<String, String>,
}

impl Status {
    pub fn plain_label(self) -> &'static str {
        match self {
            Status::Installed => "Installed",
            Status::NotInstalled => "Not installed",
            Status::NeedsUpdate => "Needs update",
        }
    }

    pub fn detect(tool: &Tool) -> Result<Self> {
        if !run_status_check(&tool.status_check)? {
            return Ok(Self::NotInstalled);
        }

        let installed = read_installed_state()?;
        match installed.tools.get(&tool.id) {
            Some(version) if version == &tool.version => Ok(Self::Installed),
            _ => Ok(Self::NeedsUpdate),
        }
    }

    pub fn is_installed(self) -> bool {
        !matches!(self, Self::NotInstalled)
    }

    pub fn label(self) -> String {
        match self {
            Status::Installed => self
                .plain_label()
                .if_supports_color(Stream::Stdout, |text| text.green())
                .to_string(),
            Status::NotInstalled => self
                .plain_label()
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
                .to_string(),
            Status::NeedsUpdate => self
                .plain_label()
                .if_supports_color(Stream::Stdout, |text| text.yellow())
                .to_string(),
        }
    }
}

pub fn write_installed_version(id: &str, version: &str) -> Result<()> {
    let path = installed_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create {parent:?}"))?;
    }

    let mut installed = read_installed_state()?;
    installed.tools.insert(id.to_owned(), version.to_owned());

    let content = toml::to_string_pretty(&installed)?;
    fs::write(&path, content).with_context(|| format!("failed to write {path:?}"))?;
    Ok(())
}

fn run_status_check(command: &str) -> Result<bool> {
    let bash = which::which("bash").context("bash is required")?;
    let status = Command::new(bash)
        .arg("-lc")
        .arg(command)
        .status()
        .with_context(|| format!("failed to run status check: {command}"))?;
    Ok(status.success())
}

fn read_installed_state() -> Result<InstalledState> {
    let path = installed_file_path()?;
    if !path.exists() {
        return Ok(InstalledState::default());
    }

    let content = fs::read_to_string(&path).with_context(|| format!("failed to read {path:?}"))?;
    let installed =
        toml::from_str(&content).with_context(|| format!("failed to parse {path:?}"))?;
    Ok(installed)
}

fn installed_file_path() -> Result<PathBuf> {
    let home = env::var_os("HOME").context("HOME is not set")?;
    Ok(Path::new(&home)
        .join(".local")
        .join("share")
        .join("toms-tools")
        .join("installed.toml"))
}
