use std::{
    env,
    io::{self, IsTerminal},
};

use anyhow::Result;
use comfy_table::{
    presets::{ASCII_FULL, UTF8_FULL},
    ColumnConstraint::UpperBoundary,
    ContentArrangement, Table,
    Width::{Fixed, Percentage},
};
use owo_colors::{OwoColorize, Stream, Style};
use terminal_size::{terminal_size, Width};

use crate::tools::{status::Status, Registry};

pub fn run(registry: &Registry) -> Result<()> {
    let mut table = build_table(terminal_width(), io::stdout().is_terminal());
    table.set_header(vec![
        "ID".if_supports_color(Stream::Stdout, |text| text.bold())
            .to_string(),
        "Name"
            .if_supports_color(Stream::Stdout, |text| text.bold())
            .to_string(),
        "Description"
            .if_supports_color(Stream::Stdout, |text| text.bold())
            .to_string(),
        "Status"
            .if_supports_color(Stream::Stdout, |text| text.bold())
            .to_string(),
    ]);

    for tool in registry.tools() {
        let status = Status::detect(&tool.definition)?;
        table.add_row(vec![
            tool.definition
                .id
                .as_str()
                .if_supports_color(Stream::Stdout, |text| {
                    text.style(Style::new().cyan().bold())
                })
                .to_string(),
            tool.definition.name.clone(),
            tool.definition.description.clone(),
            status.label(),
        ]);
    }

    println!("{table}");
    Ok(())
}

fn build_table(width: u16, is_terminal: bool) -> Table {
    let mut table = Table::new();
    table.load_preset(table_preset(is_terminal));
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_width(width);
    table.set_constraints(vec![
        UpperBoundary(Fixed(17)),
        UpperBoundary(Fixed(17)),
        UpperBoundary(Percentage(100)),
        UpperBoundary(Fixed(14)),
    ]);
    table
}

fn table_preset(is_terminal: bool) -> &'static str {
    if is_terminal {
        UTF8_FULL
    } else {
        ASCII_FULL
    }
}

fn terminal_width() -> u16 {
    env_width(env::var("COLUMNS").ok().as_deref())
        .or_else(|| terminal_size().map(|(Width(width), _)| width))
        .unwrap_or(80)
}

fn env_width(value: Option<&str>) -> Option<u16> {
    value
        .and_then(|value| value.parse::<u16>().ok())
        .filter(|width| *width > 0)
}

#[cfg(test)]
mod tests {
    use super::{build_table, env_width, table_preset};

    #[test]
    fn parses_columns_from_env() {
        assert_eq!(env_width(Some("60")), Some(60));
        assert_eq!(env_width(Some("0")), None);
        assert_eq!(env_width(Some("wide")), None);
    }

    #[test]
    fn keeps_ansi_sequences_intact_when_wrapping_tty_output() {
        let mut table = build_table(60, true);
        table.set_header(vec![
            "\u{1b}[1mID\u{1b}[0m".to_string(),
            "\u{1b}[1mName\u{1b}[0m".to_string(),
            "\u{1b}[1mDescription\u{1b}[0m".to_string(),
            "\u{1b}[1mStatus\u{1b}[0m".to_string(),
        ]);
        table.add_row(vec![
            "\u{1b}[36;1mgh-unresolved\u{1b}[0m".to_string(),
            "gh unresolved".to_string(),
            "Install the `gh unresolved` command to list unresolved CR comments on a PR."
                .to_string(),
            "\u{1b}[32mInstalled\u{1b}[39m".to_string(),
        ]);

        let rendered = table.to_string();

        assert!(rendered.contains("\u{1b}[1mStatus\u{1b}[0m"));
        assert!(rendered.contains("\u{1b}[36;1mgh-unresolved\u{1b}[0m"));
        assert!(rendered.contains("\u{1b}[32mInstalled\u{1b}[39m"));
    }

    #[test]
    fn uses_ascii_borders_for_non_tty_output() {
        let mut table = build_table(60, false);
        table.set_header(vec!["ID", "Name", "Description", "Status"]);
        table.add_row(vec![
            "gh-unresolved",
            "gh unresolved",
            "Install the tool.",
            "Installed",
        ]);

        let rendered = table.to_string();

        assert_eq!(table_preset(false), comfy_table::presets::ASCII_FULL);
        assert!(rendered.contains('+'));
        assert!(rendered.contains('|'));
        assert!(!rendered.contains('┌'));
    }
}
