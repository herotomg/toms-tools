//! `tt list` — one line per tool, grouped by what you would do about it.
//!
//! This was a bordered table. Seven tools rendered as 48 lines, with names
//! truncated to 17 characters and every description wrapped across six rows.
//! A list of seven short things does not need column separators.

use anyhow::Result;

use crate::{
    commands::ui,
    tools::{
        status::Status,
        survey::{Survey, ToolState},
        Registry,
    },
};

pub fn run(registry: &Registry) -> Result<()> {
    let survey = Survey::run(registry)?;

    let groups: [(&str, Vec<&ToolState<'_>>); 3] = [
        ("Updates available", survey.outdated()),
        (
            "Installed",
            survey
                .tools
                .iter()
                .filter(|state| matches!(state.status, Status::Installed))
                .collect(),
        ),
        ("Not installed", survey.not_installed()),
    ];

    println!();
    for (title, states) in groups {
        if states.is_empty() {
            continue;
        }

        println!(
            "  {} {}",
            ui::heading(title),
            ui::dim(&format!("({})", states.len()))
        );
        for state in states {
            print_row(state);
        }
        println!();
    }

    print_footer(&survey);
    Ok(())
}

fn print_row(state: &ToolState<'_>) {
    let id_width = 16;
    let description_width = ui::width().saturating_sub(id_width + 8).max(24);

    let mut line = format!(
        "    {} {:<id_width$} {}",
        ui::status_dot(state.status),
        ui::tool_id(state.id()),
        ui::dim(&ui::truncate(state.detail(), description_width)),
    );

    // A tool that is installed but cannot run is the thing most worth knowing,
    // so it goes on the row rather than in a footnote.
    if state.is_blocked() {
        let names: Vec<&str> = state
            .missing
            .iter()
            .map(|requirement| requirement.command.as_str())
            .collect();
        line.push_str(&format!(
            " {}",
            ui::dim(&format!("· needs {}", names.join(", ")))
        ));
    }

    println!("{line}");
}

fn print_footer(survey: &Survey<'_>) {
    let mut hints = Vec::new();

    if !survey.not_installed().is_empty() {
        hints.push("tt install");
    }
    if !survey.outdated().is_empty() {
        hints.push("tt update");
    }
    if !survey.blocked().is_empty() || !survey.bin_dir_on_path {
        hints.push("tt");
    }

    if !hints.is_empty() {
        println!("  {}", ui::dim(&format!("Next: {}", hints.join("  ·  "))));
        println!();
    }
}
