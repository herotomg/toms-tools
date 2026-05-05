use std::io::{self, IsTerminal};

use anyhow::{anyhow, bail, Result};
use dialoguer::Confirm;
use owo_colors::{OwoColorize, Stream};

use crate::{
    cli::UpdateArgs,
    tools::{deps, installer, status::Status, Registry, Tool},
};

use super::install::{action_suffix, indented, print_status_line};

pub fn run(registry: &Registry, args: &UpdateArgs) -> Result<()> {
    let requested = resolve_requested_ids(registry, args, Status::detect)?;
    if requested.is_empty() {
        println!(
            "{}",
            "All bundled tools are already current."
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
        );
        return Ok(());
    }

    let ordered = deps::resolve_install_order(registry, &requested)?;
    let continue_on_error = ordered.len() > 1;
    let mut failures = Vec::new();

    for id in ordered {
        let tool = registry
            .get(&id)
            .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;

        let status = Status::detect(&tool.definition)?;
        if matches!(status, Status::Installed) {
            print_status_line('✓', &tool.definition.id, Some(action_suffix(status)), true);
            continue;
        }

        match installer::install(tool, args.verbose) {
            Ok(()) => {
                print_status_line('✓', &tool.definition.id, Some(action_suffix(status)), true);
            }
            Err(err) => {
                print_status_line('✗', &tool.definition.id, Some("failed"), false);
                if !args.verbose {
                    if let Some(output) = indented(err.detail_output().unwrap_or("")) {
                        print!("{output}");
                    }
                }

                if continue_on_error {
                    failures.push(tool.definition.id.clone());
                } else {
                    return Err(anyhow!(err));
                }
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

fn resolve_requested_ids<F>(
    registry: &Registry,
    args: &UpdateArgs,
    detect_status: F,
) -> Result<Vec<String>>
where
    F: Fn(&Tool) -> Result<Status>,
{
    if args.all {
        return Ok(registry.tool_ids());
    }

    if !args.ids.is_empty() {
        return Ok(args.ids.clone());
    }

    let mut outdated = Vec::new();
    for tool in registry.tools() {
        if matches!(detect_status(&tool.definition)?, Status::NeedsUpdate) {
            outdated.push(tool.definition.id.clone());
        }
    }

    if outdated.is_empty() || !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Ok(outdated);
    }

    let confirmed = Confirm::new()
        .with_prompt(format!("Update {} outdated tool(s)?", outdated.len()))
        .default(true)
        .interact()?;

    if confirmed {
        Ok(outdated)
    } else {
        bail!("update cancelled");
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use include_dir::Dir;

    use crate::{
        cli::UpdateArgs,
        tools::{status::Status, EmbeddedTool, Registry, Tool},
    };

    use super::resolve_requested_ids;

    static EMPTY_DIR: Dir<'_> = Dir::new(".", &[]);

    fn registry(ids: &[&str]) -> Registry {
        let tools = ids
            .iter()
            .map(|id| {
                (
                    (*id).to_owned(),
                    EmbeddedTool {
                        definition: Tool {
                            id: (*id).to_owned(),
                            name: (*id).to_owned(),
                            description: String::new(),
                            version: "1.0.0".to_owned(),
                            depends: Vec::new(),
                            status_check: "true".to_owned(),
                        },
                        dir: &EMPTY_DIR,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();

        Registry { tools }
    }

    #[test]
    fn defaults_to_outdated_tools() {
        let registry = registry(&["a", "b", "c"]);
        let args = UpdateArgs {
            ids: Vec::new(),
            all: false,
            verbose: false,
        };

        let selected = resolve_requested_ids(&registry, &args, |tool| {
            Ok(match tool.id.as_str() {
                "b" => Status::NeedsUpdate,
                _ => Status::Installed,
            })
        })
        .unwrap();

        assert_eq!(selected, vec!["b"]);
    }

    #[test]
    fn all_selects_every_tool() {
        let registry = registry(&["a", "b"]);
        let args = UpdateArgs {
            ids: Vec::new(),
            all: true,
            verbose: false,
        };

        let selected = resolve_requested_ids(&registry, &args, |_| Ok(Status::Installed)).unwrap();

        assert_eq!(selected, vec!["a", "b"]);
    }

    #[test]
    fn explicit_ids_take_priority() {
        let registry = registry(&["a", "b"]);
        let args = UpdateArgs {
            ids: vec!["b".to_owned()],
            all: false,
            verbose: false,
        };

        let selected = resolve_requested_ids(&registry, &args, |_| Ok(Status::Installed)).unwrap();

        assert_eq!(selected, vec!["b"]);
    }
}