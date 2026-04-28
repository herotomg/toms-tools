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
    let usage = tool_usage::read(tool)?;
    print!("{}", render_section(usage, status));

    Ok(())
}

fn render_section(usage: &str, status: Status) -> String {
    let mut rendered = Vec::new();
    let mut lines = usage.lines();

    match lines.next() {
        Some(first_line) => {
            rendered.push(tool_usage::style_line(first_line));
            rendered.push(status.label());
            rendered.extend(lines.map(tool_usage::style_line));
        }
        None => rendered.push(status.label()),
    }

    rendered.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::render_section;
    use crate::tools::status::Status;

    #[test]
    fn default_help_includes_usage_subcommand() {
        let mut command = crate::cli::command();
        let tools = command.find_subcommand_mut("tools").unwrap();
        let mut buffer = Vec::new();
        tools.write_long_help(&mut buffer).unwrap();

        let help = String::from_utf8(buffer).unwrap();
        assert!(help.contains("usage"));
    }

    #[test]
    fn render_section_inserts_status_after_heading() {
        let rendered = render_section("# Demo\n\n- one", Status::NeedsUpdate);

        assert!(rendered.starts_with("# Demo\nNeeds update\n\n- one\n"));
    }
}
