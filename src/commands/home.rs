//! `tt` with no arguments: say what the state is, then offer to fix it.
//!
//! This is the only surface most people should ever need. Everything it can do
//! is also a named command, but nobody should have to learn those to get a
//! working setup.

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

use crate::{
    commands::{install, tools_update, ui, usage},
    tools::{
        survey::{Survey, ToolState, BIN_DIR},
        Registry,
    },
};

pub fn run(registry: &Registry) -> Result<()> {
    let survey = Survey::run(registry)?;

    print_report(&survey);

    if survey.is_all_well() {
        println!("  {}", ui::dim("Everything is set up. Nothing to do."));
        println!();
        return Ok(());
    }

    if !ui::is_interactive() {
        print_commands_to_run(&survey);
        return Ok(());
    }

    match choose(&survey)? {
        Some(action) => perform(action, registry),
        None => Ok(()),
    }
}

// ----------------------------------------------------------------- reporting

fn print_report(survey: &Survey<'_>) {
    let total = survey.tools.len();
    let installed = survey.installed_count();

    println!();
    println!(
        "  {} {}",
        ui::bold(&format!("tt v{}", env!("CARGO_PKG_VERSION"))),
        ui::dim(&format!("· {installed} of {total} tools installed"))
    );
    println!();

    if !survey.bin_dir_on_path {
        section("Not on your PATH");
        println!(
            "    {} is missing from $PATH, so installed commands will not be found.",
            ui::command(BIN_DIR)
        );
        // `$HOME`, not `~`: a tilde inside double quotes is not expanded by the
        // shell, so the copy-pasteable form must not use one.
        println!(
            "    Add to your shell profile: {}",
            ui::command("export PATH=\"$HOME/.local/bin:$PATH\"")
        );
        println!();
    }

    let blocked = survey.blocked();
    if !blocked.is_empty() {
        section("Installed, but missing something");
        for state in &blocked {
            println!("    {}", ui::tool_id(state.id()));
            for requirement in &state.missing {
                let why = requirement
                    .why
                    .as_deref()
                    .map(|why| format!(" — {why}"))
                    .unwrap_or_default();
                println!(
                    "      {} not found{}",
                    ui::bold(&requirement.command),
                    ui::dim(&why)
                );
                if let Some(fix) = &requirement.fix {
                    println!("      {}", ui::command(fix));
                }
            }
        }
        println!();
    }

    let outdated = survey.outdated();
    if !outdated.is_empty() {
        section("Updates available");
        for state in &outdated {
            print_tool_line(state);
        }
        println!();
    }

    let missing = survey.not_installed();
    if !missing.is_empty() {
        section("Not installed");
        for state in &missing {
            print_tool_line(state);
        }
        println!();
    }
}

fn section(title: &str) {
    println!("  {}", ui::heading(title));
}

fn print_tool_line(state: &ToolState<'_>) {
    let width = ui::width();
    let description_width = width.saturating_sub(26).max(20);

    println!(
        "    {} {:<16} {}",
        ui::status_dot(state.status),
        ui::tool_id(state.id()),
        ui::dim(&ui::truncate(state.detail(), description_width))
    );
}

/// Non-interactive fallback: no menu, just the exact commands.
fn print_commands_to_run(survey: &Survey<'_>) {
    println!("  {}", ui::heading("What to run"));
    for fix in survey.fix_commands() {
        println!("    {}", ui::command(&fix));
    }
    if !survey.not_installed().is_empty() {
        println!("    {}", ui::command("tt install"));
    }
    if !survey.outdated().is_empty() {
        println!("    {}", ui::command("tt update"));
    }
    println!();
}

// ------------------------------------------------------------------- actions

enum Action {
    InstallDependencies(Vec<String>),
    InstallMissing,
    UpdateOutdated,
    ShowUsage,
}

fn choose(survey: &Survey<'_>) -> Result<Option<Action>> {
    let mut labels = Vec::new();
    let mut actions = Vec::new();

    let fixes = survey.fix_commands();
    if !fixes.is_empty() {
        labels.push(match fixes.len() {
            1 => format!("Run `{}` to fix what is missing", fixes[0]),
            n => format!("Install {n} missing dependencies"),
        });
        actions.push(Action::InstallDependencies(fixes));
    }

    let missing = survey.not_installed().len();
    if missing > 0 {
        labels.push(format!(
            "Install the {missing} tool{} I do not have",
            plural(missing)
        ));
        actions.push(Action::InstallMissing);
    }

    let outdated = survey.outdated().len();
    if outdated > 0 {
        labels.push(format!("Update {outdated} tool{}", plural(outdated)));
        actions.push(Action::UpdateOutdated);
    }

    if survey.installed_count() > 0 {
        labels.push("Show me what my tools do".to_owned());
        actions.push(Action::ShowUsage);
    }

    labels.push("Nothing right now".to_owned());

    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&labels)
        .default(0)
        .interact_opt()?;

    Ok(match choice {
        // The trailing "Nothing right now", or Esc.
        Some(index) if index < actions.len() => Some(actions.swap_remove(index)),
        _ => None,
    })
}

fn perform(action: Action, registry: &Registry) -> Result<()> {
    match action {
        Action::InstallDependencies(commands) => install_dependencies(&commands),
        Action::InstallMissing => install::run(registry, &install::Request::missing()),
        Action::UpdateOutdated => tools_update::run(registry, &tools_update::Request::outdated()),
        Action::ShowUsage => usage::run(registry, &usage::Request::installed()),
    }
}

/// Run the fix commands the manifests declared, one at a time, showing each
/// before it runs. These are third-party installers, so the user sees exactly
/// what is about to happen and can stop after any of them.
fn install_dependencies(commands: &[String]) -> Result<()> {
    let bash = which::which("bash")?;

    for command in commands {
        println!();
        println!("  {} {}", ui::dim("running"), ui::command(command));

        let status = std::process::Command::new(&bash)
            .arg("-lc")
            .arg(command)
            .status()?;

        if status.success() {
            println!("  {} {}", ui::tick(), ui::dim(command));
        } else {
            println!("  {} {} failed", ui::cross(), ui::command(command));
            println!("  {}", ui::dim("Run it yourself, then try `tt` again."));
            return Ok(());
        }
    }

    println!();
    println!(
        "  {} Dependencies installed. Run `tt` again to check.",
        ui::tick()
    );
    Ok(())
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
    use super::plural;

    #[test]
    fn pluralises_counts() {
        assert_eq!(plural(1), "");
        assert_eq!(plural(0), "s");
        assert_eq!(plural(2), "s");
    }
}
