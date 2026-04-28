use std::env;

use anyhow::Result;
use comfy_table::{
    presets::UTF8_FULL,
    ColumnConstraint::UpperBoundary,
    ContentArrangement, Table,
    Width::{Fixed, Percentage},
};
use owo_colors::{OwoColorize, Stream, Style};
use terminal_size::{terminal_size, Width};

use crate::tools::{status::Status, Registry};

pub fn run(registry: &Registry) -> Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_width(terminal_width());
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
    table.set_constraints(vec![
        UpperBoundary(Fixed(17)),
        UpperBoundary(Fixed(17)),
        UpperBoundary(Percentage(100)),
        UpperBoundary(Fixed(14)),
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

impl Status {
    fn label(self) -> String {
        match self {
            Status::Installed => "Installed"
                .if_supports_color(Stream::Stdout, |text| text.green())
                .to_string(),
            Status::NotInstalled => "Not installed"
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
                .to_string(),
            Status::NeedsUpdate => "Needs update"
                .if_supports_color(Stream::Stdout, |text| text.yellow())
                .to_string(),
        }
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
    use super::env_width;

    #[test]
    fn parses_columns_from_env() {
        assert_eq!(env_width(Some("60")), Some(60));
        assert_eq!(env_width(Some("0")), None);
        assert_eq!(env_width(Some("wide")), None);
    }
}
