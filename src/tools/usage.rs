//! Rendering a tool's `usage.md`.
//!
//! This used to be ~300 lines of hand-rolled Markdown: headings, bullets,
//! inline code, code fences. It could not render a table — every `| a | b |`
//! came out as literal pipes — and it never wrapped, so any line longer than
//! the terminal spilled. `termimad` was already in Cargo.toml, unused.

use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use termimad::{
    crossterm::style::{Attribute, Color},
    FmtText, MadSkin,
};

use super::{status::Status, EmbeddedTool};
use crate::commands::ui;

pub fn read(tool: &EmbeddedTool) -> Result<&'static str> {
    tool.dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")
}

pub fn render_card(tool: &EmbeddedTool, status: Status) -> Result<String> {
    Ok(render(
        read(tool)?,
        &tool.definition.id,
        status,
        io::stdout().is_terminal(),
        ui::width(),
    ))
}

fn render(markdown: &str, id: &str, status: Status, is_terminal: bool, width: usize) -> String {
    // Piped or redirected: hand back the source Markdown untouched. It is
    // already a good document, and `tt usage artifacts > notes.md` should
    // produce one rather than a de-styled approximation.
    if !is_terminal {
        return ensure_trailing_newline(markdown.to_owned());
    }

    let header = format!(
        "{} {} {}\n",
        ui::tool_id(id),
        ui::dim("·"),
        status_label(status)
    );

    let skin = skin();
    let body = FmtText::from(&skin, markdown, Some(width));
    ensure_trailing_newline(format!("\n{header}\n{body}"))
}

fn status_label(status: Status) -> String {
    let label = status.plain_label();
    match status {
        Status::Installed | Status::NeedsUpdate => ui::bold(label),
        Status::NotInstalled => ui::dim(label),
    }
}

/// Tuned to sit alongside the rest of `tt`'s output rather than termimad's
/// defaults, which centre headers and draw them on a filled background.
fn skin() -> MadSkin {
    let mut skin = MadSkin::default();

    skin.set_headers_fg(Color::AnsiValue(75));
    skin.headers[0].align = termimad::Alignment::Left;
    skin.headers[1].align = termimad::Alignment::Left;
    skin.headers[2].align = termimad::Alignment::Left;
    skin.bold.set_fg(Color::AnsiValue(255));
    skin.bold.add_attr(Attribute::Bold);
    skin.italic.set_fg(Color::AnsiValue(250));
    skin.inline_code.set_fg(Color::AnsiValue(213));
    skin.code_block.set_fg(Color::AnsiValue(252));
    skin.bullet.set_fg(Color::AnsiValue(75));
    skin.table.set_fg(Color::AnsiValue(240));

    skin
}

fn ensure_trailing_newline(rendered: String) -> String {
    if rendered.ends_with('\n') {
        rendered
    } else {
        format!("{rendered}\n")
    }
}

#[cfg(test)]
mod tests {
    use super::{render, strip_ansi_for_test as strip_ansi};
    use crate::tools::status::Status;

    const TABLE: &str = "\
# Demo

Some prose that is quite a lot longer than a narrow terminal could ever hope to
display on one single line without wrapping it somewhere sensible.

| Command | What it does |
|---|---|
| `art publish` | publish a file |
| `art list` | list everything |
";

    #[test]
    fn piped_output_is_the_source_markdown() {
        let rendered = render(TABLE, "demo", Status::Installed, false, 80);
        assert_eq!(rendered, TABLE);
        assert!(!rendered.contains('\u{1b}'));
    }

    /// The bug that motivated the swap: a Markdown table used to reach the
    /// terminal as literal `|---|---|` pipes.
    #[test]
    fn tty_output_draws_a_table_instead_of_printing_pipes() {
        let rendered = render(TABLE, "demo", Status::Installed, true, 80);
        let plain = strip_ansi(&rendered);

        assert!(!plain.contains("|---|"), "separator row leaked:\n{plain}");
        assert!(
            plain.contains('─') || plain.contains('│'),
            "expected box drawing:\n{plain}"
        );
        assert!(plain.contains("art publish"));
    }

    /// The other bug: long lines were emitted at source length and the
    /// terminal hard-wrapped them mid-word.
    #[test]
    fn tty_output_wraps_to_the_given_width() {
        for width in [40usize, 60, 100] {
            let rendered = render(TABLE, "demo", Status::Installed, true, width);
            let longest = strip_ansi(&rendered)
                .lines()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(0);

            assert!(
                longest <= width,
                "width {width}: emitted a {longest}-char line"
            );
        }
    }

    #[test]
    fn tty_output_names_the_tool_and_its_status() {
        let rendered = render(TABLE, "demo", Status::NeedsUpdate, true, 80);
        let plain = strip_ansi(&rendered);

        assert!(plain.contains("demo"));
        assert!(plain.contains("Needs update"));
    }

    #[test]
    fn always_ends_with_exactly_one_newline() {
        for is_terminal in [true, false] {
            let rendered = render("# T\n\nbody", "demo", Status::Installed, is_terminal, 80);
            assert!(rendered.ends_with('\n'));
            assert!(!rendered.ends_with("\n\n"));
        }
    }
}

#[cfg(test)]
pub(crate) fn strip_ansi_for_test(input: &str) -> String {
    let mut stripped = String::new();
    let mut chars = input.chars().peekable();

    while let Some(char) = chars.next() {
        if char == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for code_char in chars.by_ref() {
                if code_char.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            stripped.push(char);
        }
    }

    stripped
}
