use std::{
    error::Error as StdError,
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use include_dir::Dir;

use super::{status, upstream, EmbeddedTool};

#[derive(Debug)]
pub enum InstallError {
    ScriptFailed {
        tool_id: String,
        stdout: String,
        stderr: String,
    },
    Other(anyhow::Error),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ScriptFailed { tool_id, .. } => write!(f, "install.sh failed for {tool_id}"),
            Self::Other(err) => write!(f, "{err:#}"),
        }
    }
}

impl StdError for InstallError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ScriptFailed { .. } => None,
            Self::Other(err) => err.source(),
        }
    }
}

impl From<anyhow::Error> for InstallError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

impl InstallError {
    pub fn detail_output(&self) -> Option<&str> {
        match self {
            Self::ScriptFailed { stderr, .. } if !stderr.trim().is_empty() => Some(stderr),
            Self::ScriptFailed { stdout, .. } if !stdout.trim().is_empty() => Some(stdout),
            _ => None,
        }
    }
}

pub fn install(tool: &EmbeddedTool, verbose: bool) -> std::result::Result<(), InstallError> {
    run_script(tool, "install.sh", verbose)?;
    status::write_installed_version(&tool.definition.id, &tool.definition.version)?;
    // Ask again now that the install has happened. Whatever the daily check
    // recorded was measured before this ran, and is about to be wrong.
    upstream::recheck(tool);
    Ok(())
}

/// Run one of a tool's own scripts — `install.sh`, `uninstall.sh` — from a
/// throwaway copy of its directory, so the script can read its payload with
/// relative paths and we leave nothing behind.
pub fn run_hook(
    tool: &EmbeddedTool,
    script_name: &str,
    verbose: bool,
) -> std::result::Result<(), InstallError> {
    run_script(tool, script_name, verbose)
}

/// Run a hook for its *output* rather than its effect, returning stdout.
///
/// A non-zero exit yields `None`, not an error: `update-check.sh` is allowed to
/// fail — the network is down, a host is unreachable — and "we could not tell"
/// must never read as "an update is waiting".
pub fn capture_hook(tool: &EmbeddedTool, script_name: &str) -> Option<String> {
    let temp_dir = create_temp_dir(&tool.definition.id).ok()?;
    let output = capture_hook_inner(tool, script_name, &temp_dir);
    let _ = fs::remove_dir_all(&temp_dir);
    output
}

fn capture_hook_inner(tool: &EmbeddedTool, script_name: &str, temp_dir: &Path) -> Option<String> {
    extract_dir(tool.dir(), temp_dir).ok()?;

    let bash = which::which("bash").ok()?;
    let output = Command::new(&bash)
        .arg(temp_dir.join(script_name))
        .current_dir(temp_dir)
        .output()
        .ok()?;

    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn has_hook(tool: &EmbeddedTool, script_name: &str) -> bool {
    tool.dir()
        .get_file(tool.dir().path().join(script_name))
        .is_some()
}

fn run_script(
    tool: &EmbeddedTool,
    script_name: &str,
    verbose: bool,
) -> std::result::Result<(), InstallError> {
    let temp_dir = create_temp_dir(&tool.definition.id)?;
    let result = run_script_inner(tool, script_name, &temp_dir, verbose);
    let cleanup = fs::remove_dir_all(&temp_dir);

    if let Err(err) = result {
        cleanup.ok();
        return Err(err);
    }

    cleanup.with_context(|| format!("failed to clean up {temp_dir:?}"))?;
    Ok(())
}

fn run_script_inner(
    tool: &EmbeddedTool,
    script_name: &str,
    temp_dir: &Path,
    verbose: bool,
) -> std::result::Result<(), InstallError> {
    extract_dir(tool.dir(), temp_dir)?;

    let bash = which::which("bash").context("bash is required")?;
    let script = temp_dir.join(script_name);

    let failed = |stdout: String, stderr: String| InstallError::ScriptFailed {
        tool_id: tool.definition.id.clone(),
        stdout,
        stderr,
    };

    if verbose {
        let status = Command::new(&bash)
            .arg(&script)
            .current_dir(temp_dir)
            .status()
            .with_context(|| format!("failed to run {script:?}"))?;
        if !status.success() {
            return Err(failed(String::new(), String::new()));
        }
    } else {
        let output = Command::new(&bash)
            .arg(&script)
            .current_dir(temp_dir)
            .output()
            .with_context(|| format!("failed to run {script:?}"))?;
        if !output.status.success() {
            return Err(failed(
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
    }

    Ok(())
}

fn create_temp_dir(id: &str) -> Result<std::path::PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is before UNIX_EPOCH")?
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!("toms-tools-{id}-{nanos}"));
    fs::create_dir_all(&temp_dir).with_context(|| format!("failed to create {temp_dir:?}"))?;
    Ok(temp_dir)
}

fn extract_dir(dir: &Dir<'_>, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination).with_context(|| format!("failed to create {destination:?}"))?;

    for file in dir.files() {
        let name = file
            .path()
            .file_name()
            .and_then(|value| value.to_str())
            .context("invalid embedded file name")?;
        let path = destination.join(name);
        fs::write(&path, file.contents()).with_context(|| format!("failed to write {path:?}"))?;
    }

    for child in dir.dirs() {
        let name = child
            .path()
            .file_name()
            .and_then(|value| value.to_str())
            .context("invalid embedded directory name")?;
        extract_dir(child, &destination.join(name))?;
    }

    Ok(())
}
