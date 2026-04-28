use anyhow::Result;
use clap::{
    builder::PossibleValuesParser, Args, CommandFactory, FromArgMatches, Parser, Subcommand,
};
use owo_colors::{OwoColorize, Stream};

use crate::{commands, tools::Registry, update};

#[derive(Debug, Parser)]
#[command(name = "tt")]
#[command(version)]
#[command(about = "Tom's Tools CLI")]
pub struct Cli {
    #[arg(long, hide = true, global = true)]
    no_update_check: bool,
    #[arg(long, global = true, conflicts_with = "no_update_check")]
    /// Force a fresh update check instead of using the cached result
    check_update: bool,
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
    /// List bundled tools and their install status
    List,

    /// Install one or more tools
    Install(InstallArgs),

    /// Show usage notes for installed tools or selected tool ids
    Usage(UsageArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InstallArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser())]
    pub ids: Vec<String>,
    #[arg(short, long)]
    pub all: bool,
    #[arg(short, long)]
    pub verbose: bool,
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UsageArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser())]
    pub ids: Vec<String>,
    #[arg(short, long, conflicts_with = "ids")]
    pub all: bool,
}

pub fn run() -> Result<()> {
    let cli = parse();
    update::maybe_check(cli.no_update_check, cli.check_update);

    match cli.command {
        Some(Commands::Tools(args)) => {
            let registry = Registry::load()?;

            match args.command {
                ToolsCommand::List => commands::list::run(&registry),
                ToolsCommand::Install(args) => commands::install::run(&registry, &args),
                ToolsCommand::Usage(args) => commands::usage::run(&registry, &args),
            }
        }
        Some(Commands::Completions(args)) => commands::completions::run(args),
        None => {
            let mut command = command();
            command.print_long_help()?;
            println!();
            Ok(())
        }
    }
}

pub fn command() -> clap::Command {
    <Cli as CommandFactory>::command().after_help(after_help())
}

fn parse() -> Cli {
    let matches = command().get_matches();
    Cli::from_arg_matches(&matches).expect("clap matched arguments should parse")
}

fn after_help() -> String {
    format!(
        "Tip: run {} to install every tool in one go.",
        "tt tools install --all".if_supports_color(Stream::Stdout, |text| text.cyan())
    )
}

fn tool_id_value_parser() -> PossibleValuesParser {
    PossibleValuesParser::new(Registry::embedded_tool_ids())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{after_help, Cli, InstallArgs};

    #[test]
    fn allows_running_without_a_subcommand() {
        assert!(Cli::try_parse_from(["tt"]).is_ok());
    }

    #[test]
    fn after_help_tip_uses_plain_command_text() {
        let help = after_help();
        assert!(help.contains("tt tools install --all"));
        assert!(!help.contains('`'));
    }

    #[test]
    fn tools_help_lists_usage_subcommand() {
        let mut command = super::command();
        let tools = command.find_subcommand_mut("tools").unwrap();
        let mut buffer = Vec::new();
        tools.write_long_help(&mut buffer).unwrap();

        let help = String::from_utf8(buffer).unwrap();
        assert!(help.contains("usage"));
    }

    #[test]
    fn help_documents_force_update_check_flag() {
        let mut command = super::command();
        let mut buffer = Vec::new();
        command.write_long_help(&mut buffer).unwrap();

        let help = String::from_utf8(buffer).unwrap();
        assert!(help.contains("--check-update"));
        assert!(help.contains("Force a fresh update check"));
    }

    #[test]
    fn install_args_support_verbose_flag() {
        let cli = Cli::try_parse_from(["tt", "tools", "install", "--all", "-v"]).unwrap();

        let args = match cli.command.unwrap() {
            super::Commands::Tools(tools) => match tools.command {
                super::ToolsCommand::Install(args) => args,
                _ => panic!("expected install command"),
            },
            _ => panic!("expected tools command"),
        };

        assert!(matches!(
            args,
            InstallArgs {
                all: true,
                verbose: true,
                ..
            }
        ));
    }
}
