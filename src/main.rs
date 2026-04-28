use owo_colors::{OwoColorize, Stream, Style};

mod cli;
mod commands;
mod tools;
mod update;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!(
            "{} {err:#}",
            "Error:".if_supports_color(Stream::Stderr, |text| {
                text.style(Style::new().red().bold())
            })
        );
        std::process::exit(1);
    }
}
