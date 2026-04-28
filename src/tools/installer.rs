use std::{
    fs,
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use include_dir::Dir;
use owo_colors::{OwoColorize, Stream, Style};

use super::{status, usage, EmbeddedTool};

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
    println!(
        "{} Installed {}",
        "✓".if_supports_color(Stream::Stdout, |text| {
            text.style(Style::new().green().bold())
        }),
        tool.definition.id
    );
    usage::print(tool)?;
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
