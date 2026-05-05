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
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Update tt to the latest released version
    Update,

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

    /// Update outdated bundled tools, or install/update explicit tool ids
    Update(UpdateArgs),

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
pub struct UpdateArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser(), conflicts_with = "all")]
    pub ids: Vec<String>,
    #[arg(short, long)]
    pub all: bool,
    #[arg(short, long)]
    pub verbose: bool,
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
    if !matches!(cli.command.as_ref(), Some(Commands::Update)) {
        update::maybe_check(cli.no_update_check, false);
    }

    match cli.command {
        Some(Commands::Update) => update::run(),
        Some(Commands::Tools(args)) => {
            let registry = Registry::load()?;

            match args.command {
                ToolsCommand::List => commands::list::run(&registry),
                ToolsCommand::Install(args) => commands::install::run(&registry, &args),
                ToolsCommand::Update(args) => commands::tools_update::run(&registry, &args),
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

    use super::{after_help, Cli, InstallArgs, UpdateArgs};

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
        assert!(help.contains("update"));
        assert!(help.contains("usage"));
    }

    #[test]
    fn help_lists_update_subcommand_and_hides_old_flag() {
        let mut command = super::command();
        let mut buffer = Vec::new();
        command.write_long_help(&mut buffer).unwrap();

        let help = String::from_utf8(buffer).unwrap();
        assert!(help.contains("update"));
        assert!(!help.contains("--check-update"));
    }

    #[test]
    fn parses_update_subcommand() {
        let cli = Cli::try_parse_from(["tt", "update"]).unwrap();

        assert!(matches!(cli.command, Some(super::Commands::Update)));
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

    #[test]
    fn tools_update_args_support_verbose_flag() {
        let cli = Cli::try_parse_from(["tt", "tools", "update", "--all", "-v"]).unwrap();

        let args = match cli.command.unwrap() {
            super::Commands::Tools(tools) => match tools.command {
                super::ToolsCommand::Update(args) => args,
                _ => panic!("expected update command"),
            },
            _ => panic!("expected tools command"),
        };

        assert!(matches!(
            args,
            UpdateArgs {
                all: true,
                verbose: true,
                ..
            }
        ));
    }
}
