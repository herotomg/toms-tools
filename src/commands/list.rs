use std::env;

use anyhow::Result;
use comfy_table::{
    presets::UTF8_FULL,
    ColumnConstraint::UpperBoundary,
    ContentArrangement, Table,
    Width::{Fixed, Percentage},
};
use terminal_size::{terminal_size, Width};

use crate::tools::{status::Status, Registry};

pub fn run(registry: &Registry) -> Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_width(terminal_width());
    table.set_header(["ID", "Name", "Description", "Status"]);
    table.set_constraints(vec![
        UpperBoundary(Fixed(17)),
        UpperBoundary(Fixed(17)),
        UpperBoundary(Percentage(100)),
        UpperBoundary(Fixed(14)),
    ]);

    for tool in registry.tools() {
        let status = Status::detect(&tool.definition)?;
        table.add_row([
            tool.definition.id.clone(),
            tool.definition.name.clone(),
            tool.definition.description.clone(),
            status.label().to_string(),
        ]);
    }

    println!("{table}");
    Ok(())
}

impl Status {
    fn label(self) -> String {
        match self {
            Status::Installed => "Installed".to_owned(),
            Status::NotInstalled => "Not installed".to_owned(),
            Status::NeedsUpdate => "Needs update".to_owned(),
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
