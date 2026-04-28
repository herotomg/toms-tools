use anyhow::Result;
use comfy_table::{presets::UTF8_FULL, ContentArrangement, Table};
use owo_colors::OwoColorize;

use crate::tools::{status::Status, Registry};

pub fn run(registry: &Registry) -> Result<()> {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(["ID", "Name", "Description", "Status"]);

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
            Status::Installed => format!("{}", "Installed".green()),
            Status::NotInstalled => format!("{}", "NotInstalled".yellow()),
            Status::NeedsUpdate => format!("{}", "NeedsUpdate".magenta()),
        }
    }
}