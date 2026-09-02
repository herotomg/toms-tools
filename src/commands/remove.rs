use anyhow::{anyhow, bail, Result};
use dialoguer::{theme::ColorfulTheme, Confirm};

use crate::{
    commands::ui,
    tools::{paths, remover, Registry},
};

pub fn run(registry: &Registry, ids: &[String], assume_yes: bool) -> Result<()> {
    if ids.is_empty() {
        bail!("name at least one tool to remove, e.g. `tt remove jsut-alias`");
    }

    // Show exactly what will happen before anything happens. Removal touches
    // paths outside our own directory, so it is never a surprise.
    println!();
    for id in ids {
        let tool = registry
            .get(id)
            .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;

        println!("  {} {}", ui::bold(&tool.definition.name), ui::tool_id(id));

        for declared in tool.definition.owned_paths() {
            let expanded = paths::expand(declared);
            let note = if paths::exists(&expanded) {
                ui::dim("will be deleted")
            } else {
                ui::dim("not present")
            };
            println!("    {declared:<48} {note}");
        }

        if remover::has_uninstall_hook(tool) {
            println!("    {}", ui::dim("runs the tool's own uninstall script"));
        }
        println!();
    }

    if !assume_yes && ui::is_interactive() {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Remove {}?", describe(ids)))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("{}", ui::dim("Nothing removed."));
            return Ok(());
        }
    }

    let mut failures = Vec::new();

    for id in ids {
        let tool = registry
            .get(id)
            .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;

        match remover::remove(tool, false) {
            Ok(removal) if removal.touched_nothing() => {
                ui::print_status_line('✓', id, Some("was not installed"), true);
            }
            Ok(removal) => {
                let count = removal.removed.len();
                // A tool whose state is an alias or a shell line removes
                // nothing from disk; "(0 paths)" would read like a failure.
                let detail = if count == 0 {
                    "removed".to_owned()
                } else {
                    format!("removed ({count} path{})", plural(count))
                };
                ui::print_status_line('✓', id, Some(&detail), true);
            }
            Err(err) => {
                ui::print_status_line('✗', id, Some("failed"), false);
                println!("{}", ui::indented(&format!("{err:#}")).unwrap_or_default());
                failures.push(id.clone());
            }
        }
    }

    if !failures.is_empty() {
        bail!("failed to remove: {}", failures.join(", "));
    }

    report_shell_restart(registry, ids)?;
    Ok(())
}

/// The alias tools edit `.zshrc`; their removal only takes effect in a new
/// shell. Saying so is the difference between "it worked" and "it is broken".
fn report_shell_restart(registry: &Registry, ids: &[String]) -> Result<()> {
    let needs_restart = ids.iter().any(|id| {
        registry
            .get(id)
            .map(|tool| {
                tool.definition
                    .status_check
                    .as_deref()
                    .is_some_and(|check| check.contains(".zshrc"))
            })
            .unwrap_or(false)
    });

    if needs_restart {
        println!();
        println!(
            "  {}",
            ui::dim("Restart your shell (or `source ~/.zshrc`) for the alias to disappear.")
        );
    }

    Ok(())
}

fn describe(ids: &[String]) -> String {
    match ids {
        [one] => one.clone(),
        _ => format!("these {} tools", ids.len()),
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::{describe, plural};

    #[test]
    fn describes_one_tool_by_name() {
        assert_eq!(describe(&["jsut-alias".to_owned()]), "jsut-alias");
    }

    #[test]
    fn describes_several_by_count() {
        assert_eq!(describe(&["a".to_owned(), "b".to_owned()]), "these 2 tools");
    }

    #[test]
    fn pluralises_paths() {
        assert_eq!(plural(1), "");
        assert_eq!(plural(3), "s");
    }
}
