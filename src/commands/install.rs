use anyhow::{anyhow, bail, Result};
use dialoguer::{theme::ColorfulTheme, MultiSelect};

use crate::{
    commands::ui,
    tools::{deps, installer, status::Status, survey::Survey, EmbeddedTool, Registry},
};

/// What to install. Kept separate from the CLI arguments so the guided front
/// door can ask for the same work without synthesising fake argv.
pub enum Request {
    All,
    /// Only tools that are not installed at all.
    Missing,
    Ids(Vec<String>),
    /// Offer a checklist; fall back to `Missing` when there is no terminal.
    Pick,
}

impl Request {
    pub fn missing() -> Self {
        Self::Missing
    }

    pub fn from_args(ids: Vec<String>, all: bool) -> Self {
        if all {
            Self::All
        } else if ids.is_empty() {
            Self::Pick
        } else {
            Self::Ids(ids)
        }
    }
}

pub fn run(registry: &Registry, request: &Request) -> Result<()> {
    let requested = resolve(registry, request)?;

    if requested.is_empty() {
        println!("{}", ui::dim("Nothing to install."));
        return Ok(());
    }

    let ordered = deps::resolve_install_order(registry, &requested)?;
    let mut failures = Vec::new();
    let mut installed = Vec::new();

    for id in ordered {
        let tool = registry
            .get(&id)
            .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;
        let status = Status::detect(&tool.definition)?;

        if matches!(status, Status::Installed) {
            ui::print_status_line('✓', &id, Some(ui::action_suffix(status)), true);
            continue;
        }

        match installer::install(tool, false) {
            Ok(()) => {
                ui::print_status_line('✓', &id, Some(ui::action_suffix(status)), true);
                installed.push(tool);
            }
            Err(err) => {
                ui::print_status_line('✗', &id, Some("failed"), false);
                if let Some(output) = ui::indented(err.detail_output().unwrap_or("")) {
                    print!("{output}");
                }
                failures.push(id);
            }
        }
    }

    print_next_steps(&installed);

    if !failures.is_empty() {
        bail!(
            "{} tool(s) failed to install: {}",
            failures.len(),
            failures.join(", ")
        );
    }

    Ok(())
}

/// The whole point of `next_steps`: after installing, say the one thing to do,
/// not the tool's entire manual. The manual is one command away.
fn print_next_steps(installed: &[&EmbeddedTool]) {
    let steps: Vec<(&str, &str)> = installed
        .iter()
        .filter_map(|tool| {
            tool.definition
                .next_steps
                .as_deref()
                .map(|next| (tool.definition.id.as_str(), next))
        })
        .collect();

    if steps.is_empty() {
        return;
    }

    let id_width = steps.iter().map(|(id, _)| id.len()).max().unwrap_or(0);
    let room = ui::width().saturating_sub(id_width + 5);

    println!();
    println!("  {}", ui::heading("Next steps"));
    for (id, next) in steps {
        println!(
            "    {:<id_width$}  {}",
            ui::tool_id(id),
            ui::truncate(next, room)
        );
    }

    println!();
    println!("  {}", ui::dim("Full docs: tt usage <tool>"));
}

fn resolve(registry: &Registry, request: &Request) -> Result<Vec<String>> {
    match request {
        Request::All => Ok(registry.tool_ids()),
        Request::Ids(ids) => Ok(ids.clone()),
        Request::Missing => Ok(not_installed_ids(registry)?),
        Request::Pick => {
            if !ui::is_interactive() {
                return not_installed_ids(registry);
            }
            pick(registry)
        }
    }
}

fn not_installed_ids(registry: &Registry) -> Result<Vec<String>> {
    let survey = Survey::run(registry)?;
    Ok(survey
        .not_installed()
        .iter()
        .map(|state| state.id().to_owned())
        .collect())
}

/// A checklist, pre-ticked with everything not yet installed. This is the
/// "interactive menu" a picker earns — one screen, no modes to learn.
fn pick(registry: &Registry) -> Result<Vec<String>> {
    let survey = Survey::run(registry)?;

    let mut labels = Vec::new();
    let mut ids = Vec::new();
    let mut checked = Vec::new();

    for state in &survey.tools {
        let marker = match state.status {
            Status::Installed => "installed",
            Status::NeedsUpdate => "update available",
            Status::NotInstalled => "not installed",
        };
        labels.push(format!(
            "{:<14} {:<18} {}",
            state.id(),
            marker,
            ui::truncate(&state.tool.definition.description, 46)
        ));
        ids.push(state.id().to_owned());
        checked.push(!state.status.is_installed());
    }

    let chosen = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Space to toggle, Enter to confirm")
        .items(&labels)
        .defaults(&checked)
        .interact_opt()?;

    match chosen {
        Some(indexes) => Ok(indexes
            .into_iter()
            .map(|index| ids[index].clone())
            .collect()),
        None => bail!("cancelled"),
    }
}

#[cfg(test)]
mod tests {
    use super::Request;

    #[test]
    fn args_map_to_requests() {
        assert!(matches!(Request::from_args(vec![], true), Request::All));
        assert!(matches!(Request::from_args(vec![], false), Request::Pick));
        assert!(matches!(
            Request::from_args(vec!["a".to_owned()], false),
            Request::Ids(ids) if ids == vec!["a".to_owned()]
        ));
    }

    #[test]
    fn all_beats_explicit_ids() {
        assert!(matches!(
            Request::from_args(vec!["a".to_owned()], true),
            Request::All
        ));
    }
}
