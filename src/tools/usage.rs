use std::io::{self, IsTerminal};

use anyhow::{Context, Result};
use owo_colors::{OwoColorize, Style};
use terminal_size::{terminal_size, Width};

use super::{status::Status, EmbeddedTool};

const MAX_CARD_WIDTH: usize = 80;
const MIN_CARD_WIDTH: usize = 32;

pub fn read(tool: &EmbeddedTool) -> Result<&'static str> {
    tool.dir()
        .get_file(tool.dir().path().join("usage.md"))
        .context("usage.md missing")?
        .contents_utf8()
        .context("usage.md is not valid UTF-8")
}

pub fn render_post_install(tool: &EmbeddedTool) -> Result<String> {
    render_card(tool, Status::Installed)
}

pub fn render_card(tool: &EmbeddedTool, status: Status) -> Result<String> {
    Ok(render_for_terminal(
        read(tool)?,
        &tool.definition.id,
        status,
        io::stdout().is_terminal(),
    ))
}

fn render_for_terminal(markdown: &str, id: &str, status: Status, is_terminal: bool) -> String {
    render_card_with_width(markdown, id, status, is_terminal, card_width())
}

fn render_card_with_width(
    markdown: &str,
    id: &str,
    status: Status,
    is_terminal: bool,
    width: usize,
) -> String {
    let (title, body) = split_title(markdown);
    let mut rendered = vec![
        render_title(&title, is_terminal),
        render_metadata(id, status, is_terminal),
        render_rule(is_terminal, width),
    ];

    if !body.iter().all(|line| line.trim().is_empty()) {
        rendered.push(String::new());
        rendered.extend(render_markdown_lines(&body, is_terminal, width));
    }

    ensure_trailing_newline(rendered.join("\n"))
}

fn render_markdown_lines(lines: &[&str], is_terminal: bool, width: usize) -> Vec<String> {
    let mut rendered = Vec::new();
    let mut in_code_block = false;

    for line in lines {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            if !is_terminal {
                rendered.push((*line).to_owned());
            }
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            rendered.push(render_code_line(line, is_terminal));
            continue;
        }

        rendered.push(render_markdown_line(line, is_terminal, width));
    }

    rendered
}

fn render_markdown_line(line: &str, is_terminal: bool, width: usize) -> String {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    if let Some((level, text)) = heading(trimmed) {
        return render_heading(level, text, is_terminal);
    }

    if is_rule(trimmed) {
        return render_rule(is_terminal, width);
    }

    if let Some(text) = trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
    {
        if is_terminal {
            return format!(
                "{indent}{} {}",
                paint("•", Style::new().bright_cyan().bold(), true),
                render_inline(text, true)
            );
        }
    }

    if let Some((marker, text)) = numbered_item(trimmed) {
        if is_terminal {
            return format!(
                "{indent}{} {}",
                paint(marker, Style::new().bright_yellow().bold(), true),
                render_inline(text, true)
            );
        }
    }

    render_inline(line, is_terminal)
}

fn render_title(title: &str, is_terminal: bool) -> String {
    if is_terminal {
        paint(title, Style::new().bright_cyan().bold(), true)
    } else {
        format!("# {title}")
    }
}

fn render_heading(level: usize, text: &str, is_terminal: bool) -> String {
    if !is_terminal {
        return format!("{} {text}", "#".repeat(level));
    }

    match level {
        1 => paint(text, Style::new().bright_cyan().bold(), true),
        2 => paint(text, Style::new().bright_yellow().bold(), true),
        _ => paint(text, Style::new().bright_blue().bold().dimmed(), true),
    }
}

fn render_metadata(id: &str, status: Status, is_terminal: bool) -> String {
    let id = paint(id, Style::new().bright_cyan(), is_terminal);
    let separator = paint(" · ", Style::new().bright_black().dimmed(), is_terminal);
    let status = render_status(status, is_terminal);

    format!("{id}{separator}{status}")
}

fn render_status(status: Status, is_terminal: bool) -> String {
    let style = match status {
        Status::Installed => Style::new().bright_green().bold(),
        Status::NeedsUpdate => Style::new().bright_yellow().bold(),
        Status::NotInstalled => Style::new().bright_red().dimmed(),
    };

    paint(status.plain_label(), style, is_terminal)
}

fn render_rule(is_terminal: bool, width: usize) -> String {
    if is_terminal {
        paint(
            &"─".repeat(width),
            Style::new().bright_black().dimmed(),
            true,
        )
    } else {
        "---".to_owned()
    }
}

fn render_code_line(line: &str, is_terminal: bool) -> String {
    if !is_terminal {
        return line.to_owned();
    }

    format!(
        "{} {}",
        paint("│", Style::new().bright_magenta().bold(), true),
        paint(line, Style::new().bright_white(), true)
    )
}

fn render_inline(line: &str, is_terminal: bool) -> String {
    if !is_terminal {
        return line.to_owned();
    }

    let mut rendered = String::new();
    let mut remaining = line;

    while let Some(start) = remaining.find('`') {
        rendered.push_str(&remaining[..start]);
        remaining = &remaining[start + 1..];

        match remaining.find('`') {
            Some(end) => {
                let code = &remaining[..end];
                rendered.push_str(&paint(
                    code,
                    Style::new()
                        .bright_magenta()
                        .bold()
                        .on_truecolor(36, 24, 44),
                    true,
                ));
                remaining = &remaining[end + 1..];
            }
            None => {
                rendered.push('`');
                rendered.push_str(remaining);
                return rendered;
            }
        }
    }

    rendered.push_str(remaining);
    rendered
}

fn split_title(markdown: &str) -> (String, Vec<&str>) {
    let mut lines = markdown.lines();

    match lines.next() {
        Some(first) => {
            let title = first.strip_prefix("# ").unwrap_or(first).to_owned();
            let body = lines.skip_while(|line| line.trim().is_empty()).collect();
            (title, body)
        }
        None => ("Usage".to_owned(), Vec::new()),
    }
}

fn heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|char| *char == '#').count();
    if level == 0 || level > 6 || !line[level..].starts_with(' ') {
        return None;
    }

    Some((level, line[level + 1..].trim()))
}

fn is_rule(line: &str) -> bool {
    line.len() >= 3 && line.chars().all(|char| matches!(char, '-' | '*' | '_'))
}

fn numbered_item(line: &str) -> Option<(&str, &str)> {
    let (marker, text) = line.split_once(". ")?;
    if marker.is_empty() || !marker.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }

    Some((&line[..marker.len() + 1], text))
}

fn paint(text: &str, style: Style, enabled: bool) -> String {
    if enabled {
        text.style(style).to_string()
    } else {
        text.to_owned()
    }
}

fn card_width() -> usize {
    terminal_size()
        .map(|(Width(width), _)| usize::from(width))
        .unwrap_or(MAX_CARD_WIDTH)
        .clamp(MIN_CARD_WIDTH, MAX_CARD_WIDTH)
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
    use super::{render_card_with_width, render_for_terminal};
    use crate::tools::status::Status;

    #[test]
    fn non_tty_output_has_no_ansi_escapes() {
        let rendered = render_card_with_width(
            "# Demo\n\n- `tt demo`",
            "demo",
            Status::NotInstalled,
            false,
            80,
        );

        assert!(!rendered.contains("\u{1b}["));
        assert_eq!(
            rendered,
            "# Demo\ndemo · Not installed\n---\n\n- `tt demo`\n"
        );
    }

    #[test]
    fn tty_output_formats_markdown_with_color() {
        let rendered = render_card_with_width(
            "# Demo\n\n## Commands\n\n- `tt demo`",
            "demo",
            Status::Installed,
            true,
            40,
        );

        assert!(rendered.contains("\u{1b}["));
        assert!(rendered.contains('•'));
        assert!(!rendered.contains("`tt demo`"));
    }

    #[test]
    fn tty_title_is_left_aligned() {
        let rendered = render_for_terminal("# Demo\n\n- one", "demo", Status::NeedsUpdate, true);
        let plain = strip_ansi(&rendered);

        assert!(plain.starts_with("Demo\n"));
        assert!(!plain.starts_with(' '));
    }

    fn strip_ansi(input: &str) -> String {
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
}
