use anyhow::Result;
use clap::{builder::PossibleValuesParser, Args, CommandFactory, Parser, Subcommand};

use crate::{commands, tools::Registry, update};

#[derive(Debug, Parser)]
#[command(name = "tt")]
#[command(version)]
#[command(about = "Tom's Tools CLI")]
#[command(after_help = "Tip: run `tt tools install --all` to install every tool in one go.")]
pub struct Cli {
    #[arg(long, hide = true, global = true)]
    no_update_check: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Tools(ToolsArgs),
    Completions(commands::completions::CompletionsArgs),
}

#[derive(Debug, Args)]
struct ToolsArgs {
    #[command(subcommand)]
    command: ToolsCommand,
}

#[derive(Debug, Subcommand)]
enum ToolsCommand {
    List,
    Install(InstallArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InstallArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser())]
    pub ids: Vec<String>,
    #[arg(short, long)]
    pub all: bool,
    #[arg(short = 'y', long)]
    pub yes: bool,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    update::maybe_check(cli.no_update_check);

    match cli.command {
        Some(Commands::Tools(args)) => {
            let registry = Registry::load()?;

            match args.command {
                ToolsCommand::List => commands::list::run(&registry),
                ToolsCommand::Install(args) => commands::install::run(&registry, &args),
            }
        }
        Some(Commands::Completions(args)) => commands::completions::run(args),
        None => {
            let mut command = Cli::command();
            command.print_long_help()?;
            println!();
            Ok(())
        }
    }
}

fn tool_id_value_parser() -> PossibleValuesParser {
    PossibleValuesParser::new(Registry::embedded_tool_ids())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::Cli;

    #[test]
    fn allows_running_without_a_subcommand() {
        assert!(Cli::try_parse_from(["tt"]).is_ok());
    }
}
