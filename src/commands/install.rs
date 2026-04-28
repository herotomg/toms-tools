use std::io::{self, IsTerminal, Write};

use anyhow::{bail, Result};
use dialoguer::Confirm;
use owo_colors::OwoColorize;

use crate::{
    cli::InstallArgs,
    tools::{deps, installer, status::Status, Registry},
};

pub fn run(registry: &Registry, args: &InstallArgs) -> Result<()> {
    let requested = resolve_requested_ids(registry, args)?;
    let ordered = deps::resolve_install_order(registry, &requested)?;

    for id in ordered {
        let tool = registry
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("unknown tool id: {id}"))?;

        match Status::detect(&tool.definition)? {
            Status::Installed => {
                println!("{} {}", "Skipping".cyan().bold(), tool.definition.id);
            }
            Status::NotInstalled | Status::NeedsUpdate => {
                println!("{} {}", "Installing".green().bold(), tool.definition.id);
                installer::install(tool)?;
            }
        }
    }

    Ok(())
}

fn resolve_requested_ids(registry: &Registry, args: &InstallArgs) -> Result<Vec<String>> {
    if args.all {
        return Ok(registry.tool_ids());
    }

    if !args.ids.is_empty() {
        return Ok(args.ids.clone());
    }

    if !args.yes {
        let confirmed = if io::stdin().is_terminal() && io::stdout().is_terminal() {
            Confirm::new()
                .with_prompt("Install all tools?")
                .default(true)
                .interact()?
        } else {
            print!("Install all tools? [Y/n] ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            matches!(input.trim().to_ascii_lowercase().as_str(), "" | "y" | "yes")
        };

        if !confirmed {
            bail!("installation cancelled");
        }
    }

    Ok(registry.tool_ids())
}