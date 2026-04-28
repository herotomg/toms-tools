use std::io::{self, IsTerminal, Write};

use anyhow::{anyhow, bail, Result};
use dialoguer::Confirm;
use owo_colors::{OwoColorize, Stream, Style};

use crate::{
    cli::InstallArgs,
    tools::{deps, installer, status::Status, usage, Registry},
};

pub fn run(registry: &Registry, args: &InstallArgs) -> Result<()> {
    let requested = resolve_requested_ids(registry, args)?;
    let ordered = deps::resolve_install_order(registry, &requested)?;
    let mut failures = Vec::new();

    for id in ordered {
        let tool = registry
            .get(&id)
            .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;

        match Status::detect(&tool.definition)? {
            Status::Installed => {
                print_status_line('✓', &tool.definition.id, Some("already installed"), true);
            }
            Status::NotInstalled | Status::NeedsUpdate => {
                match installer::install(tool, args.verbose) {
                    Ok(()) => {
                        print_status_line('✓', &tool.definition.id, None, true);
                        print!("{}", usage::render_post_install(tool)?);
                    }
                    Err(err) => {
                        print_status_line('✗', &tool.definition.id, None, false);
                        if !args.verbose {
                            if let Some(output) = indented(err.detail_output().unwrap_or("")) {
                                print!("{output}");
                            }
                        }

                        if args.all {
                            failures.push(tool.definition.id.clone());
                        } else {
                            return Err(anyhow!(err));
                        }
                    }
                }
            }
        }
    }

    if !failures.is_empty() {
        bail!(
            "{} tool install(s) failed: {}",
            failures.len(),
            failures.join(", ")
        );
    }

    Ok(())
}

fn print_status_line(symbol: char, id: &str, suffix: Option<&str>, success: bool) {
    let symbol = if success {
        symbol
            .to_string()
            .if_supports_color(Stream::Stdout, |text| {
                text.style(Style::new().green().bold())
            })
            .to_string()
    } else {
        symbol
            .to_string()
            .if_supports_color(Stream::Stdout, |text| text.style(Style::new().red().bold()))
            .to_string()
    };
    let id = id.if_supports_color(Stream::Stdout, |text| text.cyan());

    match suffix {
        Some(suffix) => println!("{symbol} {id} {suffix}"),
        None => println!("{symbol} {id}"),
    }
}

fn indented(output: &str) -> Option<String> {
    let trimmed = output.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    Some(
        trimmed
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
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

#[cfg(test)]
mod tests {
    use super::indented;

    #[test]
    fn indented_prefixes_each_line() {
        assert_eq!(
            indented("first\nsecond").unwrap(),
            "    first\n    second\n"
        );
    }

    #[test]
    fn indented_skips_empty_output() {
        assert_eq!(indented("\n"), None);
    }
}
