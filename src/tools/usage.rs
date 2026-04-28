use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Stream, Style};
use termimad::MadSkin;

use super::EmbeddedTool;

pub fn read(tool: &EmbeddedTool) -> Result<&'static str> {
    tool.dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")
}

pub fn render_post_install(tool: &EmbeddedTool) -> Result<String> {
    Ok(format!(
        "{}\n{}",
        post_install_header(&tool.definition.name),
        render(read(tool)?)
    ))
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

fn post_install_header(name: &str) -> String {
    format!("━━━ How to use: {name} ━━━")
        .if_supports_color(Stream::Stdout, |text| {
            text.style(Style::new().cyan().bold())
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::{post_install_header, render_for_terminal};

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

    #[test]
    fn post_install_header_uses_expected_text() {
        assert_eq!(
            post_install_header("Intent PR Fixer"),
            "━━━ How to use: Intent PR Fixer ━━━"
        );
    }
}
