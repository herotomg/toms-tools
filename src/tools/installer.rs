use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use include_dir::Dir;
use owo_colors::OwoColorize;

use super::{status, EmbeddedTool};

pub fn install(tool: &EmbeddedTool) -> Result<()> {
    let temp_dir = create_temp_dir(&tool.definition.id)?;
    let result = install_inner(tool, &temp_dir);
    let cleanup = fs::remove_dir_all(&temp_dir);

    if let Err(err) = result {
        cleanup.ok();
        return Err(err);
    }

    cleanup.with_context(|| format!("failed to clean up {temp_dir:?}"))?;
    Ok(())
}

fn install_inner(tool: &EmbeddedTool, temp_dir: &Path) -> Result<()> {
    extract_dir(tool.dir(), temp_dir)?;

    let bash = which::which("bash").context("bash is required")?;
    let install_script = temp_dir.join("install.sh");
    Command::new(bash)
        .arg(&install_script)
        .current_dir(temp_dir)
        .status()
        .with_context(|| format!("failed to run {install_script:?}"))
        .and_then(|status| {
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("install.sh failed for {}", tool.definition.id)
            }
        })?;

    status::write_installed_version(&tool.definition.id, &tool.definition.version)?;
    print_usage(tool)?;
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
    fs::create_dir_all(destination)
        .with_context(|| format!("failed to create {destination:?}"))?;

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

fn print_usage(tool: &EmbeddedTool) -> Result<()> {
    let usage = tool
        .dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")?;

    for line in usage.lines() {
        if let Some(heading) = line.strip_prefix("# ") {
            println!("{}", heading.bold().cyan());
        } else if let Some(heading) = line.strip_prefix("## ") {
            println!("{}", heading.bold().yellow());
        } else {
            println!("{line}");
        }
    }

    Ok(())
}