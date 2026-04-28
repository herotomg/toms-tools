use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream};

use super::EmbeddedTool;

pub fn read(tool: &EmbeddedTool) -> Result<&'static str> {
    tool.dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")
}

pub fn print(tool: &EmbeddedTool) -> Result<()> {
    for line in read(tool)?.lines() {
        println!("{}", style_line(line));
    }

    Ok(())
}

pub fn style_line(line: &str) -> String {
    if line.starts_with('#') {
        line.if_supports_color(Stream::Stdout, |text| text.bold())
            .to_string()
    } else {
        line.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::style_line;

    #[test]
    fn non_heading_lines_are_unchanged() {
        assert_eq!(style_line("- use this command"), "- use this command");
    }
}
