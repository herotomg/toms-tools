use anyhow::{anyhow, Result};

use crate::{
    commands::ui,
    tools::{status::Status, usage as tool_usage, EmbeddedTool, Registry},
};

pub enum Request {
    Installed,
    All,
    Ids(Vec<String>),
}

impl Request {
    pub fn installed() -> Self {
        Self::Installed
    }

    pub fn from_args(ids: Vec<String>, all: bool) -> Self {
        if all {
            Self::All
        } else if ids.is_empty() {
            Self::Installed
        } else {
            Self::Ids(ids)
        }
    }
}

pub fn run(registry: &Registry, request: &Request) -> Result<()> {
    let selected = resolve(registry, request)?;

    if selected.is_empty() {
        println!("{}", ui::dim("No installed tools yet."));
        println!("{}", ui::dim("Run `tt install` to pick some."));
        return Ok(());
    }

    // One tool: show it. Several: summarise, because dumping every manual is
    // how this command became 245 lines of scrollback.
    if let [(tool, status)] = selected.as_slice() {
        print!("{}", tool_usage::render_card(tool, *status)?);
        return Ok(());
    }

    print_summary(&selected);
    Ok(())
}

fn print_summary(selected: &[(&EmbeddedTool, Status)]) {
    // This is an index, not the manual — keep every entry to one line each so
    // it stays scannable. `tt usage <id>` renders the full page.
    let room = ui::width().saturating_sub(4);

    println!();
    for (tool, status) in selected {
        let tool = &tool.definition;
        println!("  {} {}", ui::status_dot(*status), ui::bold(&tool.name));
        println!("    {}", ui::dim(&ui::truncate(&tool.description, room)));
        if let Some(next) = &tool.next_steps {
            for line in next.lines() {
                println!("    {}", ui::truncate(line, room));
            }
        }
        println!(
            "    {}",
            ui::dim(&format!("tt usage {} for the full page", tool.id))
        );
        println!();
    }
}

fn resolve<'a>(
    registry: &'a Registry,
    request: &Request,
) -> Result<Vec<(&'a EmbeddedTool, Status)>> {
    let mut selected = Vec::new();

    match request {
        Request::All => {
            for tool in registry.tools() {
                selected.push((tool, Status::detect(&tool.definition)?));
            }
        }
        Request::Ids(ids) => {
            for id in ids {
                let tool = registry
                    .get(id)
                    .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;
                selected.push((tool, Status::detect(&tool.definition)?));
            }
        }
        Request::Installed => {
            for tool in registry.tools() {
                let status = Status::detect(&tool.definition)?;
                if status.is_installed() {
                    selected.push((tool, status));
                }
            }
        }
    }

    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::Request;

    #[test]
    fn bare_usage_means_what_is_installed() {
        assert!(matches!(
            Request::from_args(vec![], false),
            Request::Installed
        ));
    }

    #[test]
    fn a_single_id_is_requested_directly() {
        assert!(matches!(
            Request::from_args(vec!["artifacts".to_owned()], false),
            Request::Ids(ids) if ids == vec!["artifacts".to_owned()]
        ));
    }
}
