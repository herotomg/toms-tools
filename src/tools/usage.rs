use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use termimad::MadSkin;

use super::EmbeddedTool;

pub fn read(tool: &EmbeddedTool) -> Result<&'static str> {
    tool.dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")
}

pub fn print(tool: &EmbeddedTool) -> Result<()> {
    print!("{}", render(read(tool)?));

    Ok(())
}

pub fn render(markdown: &str) -> String {
    render_for_terminal(markdown, io::stdout().is_terminal())
}

fn render_for_terminal(markdown: &str, is_terminal: bool) -> String {
    let rendered = if is_terminal {
        MadSkin::default().term_text(markdown).to_string()
    } else {
        markdown.to_owned()
    };

    if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{rendered}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::render_for_terminal;

    #[test]
    fn non_tty_output_keeps_plain_markdown() {
        assert_eq!(
            render_for_terminal("# Demo\n\n- `tt demo`", false),
            "# Demo\n\n- `tt demo`\n"
        );
    }

    #[test]
    fn tty_output_formats_markdown() {
        let rendered = render_for_terminal("# Demo\n\n- `tt demo`", true);

        assert_ne!(rendered, "# Demo\n\n- `tt demo`\n");
        assert!(!rendered.contains("`tt demo`"));
    }
}
