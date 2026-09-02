//! Shared presentation for every command.
//!
//! These used to live in `commands::install`, which meant `commands::update`
//! imported its sibling to print a line. Everything a command needs to say
//! about a tool now comes from here.

use std::io::{self, IsTerminal};

use owo_colors::{OwoColorize, Stream, Style};

use crate::tools::status::Status;

/// Terminal width, honoured consistently everywhere. `$COLUMNS` wins so output
/// stays reproducible in tests and scripts.
pub fn width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width > 0)
        .or_else(|| {
            terminal_size::terminal_size()
                .map(|(terminal_size::Width(width), _)| usize::from(width))
        })
        .unwrap_or(80)
        .clamp(40, 100)
}

pub fn is_interactive() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal()
}

pub fn tick() -> String {
    "✓"
        .if_supports_color(Stream::Stdout, |text| text.green().to_string())
        .to_string()
}

pub fn cross() -> String {
    "✗"
        .if_supports_color(Stream::Stdout, |text| text.red().to_string())
        .to_string()
}

pub fn tool_id(id: &str) -> String {
    id.if_supports_color(Stream::Stdout, |text| text.cyan())
        .to_string()
}

pub fn dim(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |text| text.dimmed())
        .to_string()
}

pub fn bold(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |text| text.bold())
        .to_string()
}

pub fn heading(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |text| {
        text.style(Style::new().bold().bright_white())
    })
    .to_string()
}

/// A command the user could run, styled so it reads as copy-pasteable.
pub fn command(text: &str) -> String {
    text.if_supports_color(Stream::Stdout, |text| {
        text.style(Style::new().bright_magenta().bold())
    })
    .to_string()
}

pub fn status_dot(status: Status) -> String {
    let (glyph, style) = match status {
        Status::Installed => ("●", Style::new().green()),
        Status::NeedsUpdate => ("●", Style::new().yellow()),
        Status::NotInstalled => ("○", Style::new().bright_black()),
    };

    glyph
        .if_supports_color(Stream::Stdout, |text| text.style(style))
        .to_string()
}

pub fn action_suffix(status: Status) -> &'static str {
    match status {
        Status::Installed => "already current",
        Status::NotInstalled => "installed",
        Status::NeedsUpdate => "updated",
    }
}

pub fn print_status_line(symbol: char, id: &str, suffix: Option<&str>, success: bool) {
    let mark = if success { tick() } else { cross() };
    let mark = if symbol == '✓' || symbol == '✗' {
        mark
    } else {
        symbol.to_string()
    };
    let id = tool_id(id);

    match suffix {
        Some(suffix) => println!("{mark} {id} {}", dim(suffix)),
        None => println!("{mark} {id}"),
    }
}

pub fn indented(output: &str) -> Option<String> {
    let trimmed = output.trim_end();
    if trimmed.is_empty() {
        return None;
    }

    Some(
        trimmed
            .lines()
            .map(|line| format!("    {line}"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
}

/// Truncate to `max` display columns, ellipsising rather than wrapping. Used
/// for one-line-per-tool listings where a wrapped cell destroys the alignment.
pub fn truncate(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    if max <= 1 {
        return "…".to_owned();
    }

    chars[..max - 1]
        .iter()
        .collect::<String>()
        .trim_end()
        .to_owned()
        + "…"
}

#[cfg(test)]
mod tests {
    use super::truncate;

    #[test]
    fn leaves_short_text_alone() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("exactly10!", 10), "exactly10!");
    }

    #[test]
    fn ellipsises_long_text_to_the_limit() {
        let out = truncate("a much longer description than fits", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn does_not_leave_a_space_before_the_ellipsis() {
        assert_eq!(truncate("hello world", 7), "hello…");
    }

    #[test]
    fn handles_multibyte_text_by_character_not_byte() {
        assert_eq!(truncate("héllo wörld", 6), "héllo…");
        assert_eq!(truncate("日本語のテキスト", 4), "日本語…");
    }
}
