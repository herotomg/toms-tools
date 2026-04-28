use anyhow::{anyhow, Result};
use owo_colors::{OwoColorize, Stream};

use crate::{
    cli::UsageArgs,
    tools::{status::Status, usage as tool_usage, EmbeddedTool, Registry},
};

pub fn run(registry: &Registry, args: &UsageArgs) -> Result<()> {
    let selected = resolve_selected_tools(registry, args)?;

    if selected.is_empty() {
        println!(
            "{}",
            "No installed tools found.".if_supports_color(Stream::Stdout, |text| text.dimmed())
        );
        println!(
            "{}",
            "Tip: run tt tools usage --all to show usage for every bundled tool."
                .if_supports_color(Stream::Stdout, |text| text.dimmed())
        );
        return Ok(());
    }

    for (index, (tool, status)) in selected.iter().enumerate() {
        if index > 0 {
            println!();
        }

        print_section(tool, *status)?;
    }

    Ok(())
}

fn resolve_selected_tools<'a>(
    registry: &'a Registry,
    args: &UsageArgs,
) -> Result<Vec<(&'a EmbeddedTool, Status)>> {
    let mut selected = Vec::new();

    if args.all {
        for tool in registry.tools() {
            selected.push((tool, Status::detect(&tool.definition)?));
        }
        return Ok(selected);
    }

    if !args.ids.is_empty() {
        for id in &args.ids {
            let tool = registry
                .get(id)
                .ok_or_else(|| anyhow!("unknown tool id: {id}"))?;
            selected.push((tool, Status::detect(&tool.definition)?));
        }
        return Ok(selected);
    }

    for tool in registry.tools() {
        let status = Status::detect(&tool.definition)?;
        if status.is_installed() {
            selected.push((tool, status));
        }
    }

    Ok(selected)
}

fn print_section(tool: &EmbeddedTool, status: Status) -> Result<()> {
    print!("{}", tool_usage::render_card(tool, status)?);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_help_includes_usage_subcommand() {
        let mut command = crate::cli::command();
        let tools = command.find_subcommand_mut("tools").unwrap();
        let mut buffer = Vec::new();
        tools.write_long_help(&mut buffer).unwrap();

        let help = String::from_utf8(buffer).unwrap();
        assert!(help.contains("usage"));
    }
}
