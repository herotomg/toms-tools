use anyhow::{anyhow, bail, Result};

use crate::{
    commands::ui,
    tools::{deps, installer, status::Status, survey::Survey, Registry},
};

pub enum Request {
    All,
    /// Only tools whose recorded version is behind the bundled one.
    Outdated,
    Ids(Vec<String>),
}

impl Request {
    pub fn outdated() -> Self {
        Self::Outdated
    }

    pub fn from_args(ids: Vec<String>, all: bool) -> Self {
        if all {
            Self::All
        } else if ids.is_empty() {
            Self::Outdated
        } else {
            Self::Ids(ids)
        }
    }
}

pub fn run(registry: &Registry, request: &Request) -> Result<()> {
    let requested = resolve(registry, request)?;

    if requested.is_empty() {
        println!("{}", ui::dim("All tools are already current."));
        return Ok(());
    }

    let ordered = deps::resolve_install_order(registry, &requested)?;
    let mut failures = Vec::new();

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
            Ok(()) => ui::print_status_line('✓', &id, Some(ui::action_suffix(status)), true),
            Err(err) => {
                ui::print_status_line('✗', &id, Some("failed"), false);
                if let Some(output) = ui::indented(err.detail_output().unwrap_or("")) {
                    print!("{output}");
                }
                failures.push(id);
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "{} tool update(s) failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }

    Ok(())
}

fn resolve(registry: &Registry, request: &Request) -> Result<Vec<String>> {
    match request {
        Request::All => Ok(registry.tool_ids()),
        Request::Ids(ids) => Ok(ids.clone()),
        Request::Outdated => {
            let survey = Survey::run(registry)?;
            Ok(survey
                .outdated()
                .iter()
                .map(|state| state.id().to_owned())
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Request;

    #[test]
    fn no_arguments_means_whatever_is_outdated() {
        assert!(matches!(
            Request::from_args(vec![], false),
            Request::Outdated
        ));
    }

    #[test]
    fn explicit_ids_are_taken_literally() {
        assert!(matches!(
            Request::from_args(vec!["pr-fixer".to_owned()], false),
            Request::Ids(ids) if ids == vec!["pr-fixer".to_owned()]
        ));
    }

    #[test]
    fn all_wins_over_ids() {
        assert!(matches!(
            Request::from_args(vec!["pr-fixer".to_owned()], true),
            Request::All
        ));
    }
}
