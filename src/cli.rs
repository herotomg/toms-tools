use anyhow::Result;
use clap::{
    builder::PossibleValuesParser, Args, CommandFactory, FromArgMatches, Parser, Subcommand,
};

use crate::{
    commands,
    tools::{upstream, Registry},
    update,
};

#[derive(Debug, Parser)]
#[command(name = "tt")]
#[command(version)]
#[command(about = "Tom's Tools — run `tt` on its own and it will tell you what to do")]
pub struct Cli {
    #[arg(long, hide = true, global = true)]
    no_update_check: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Install tools (no arguments opens a checklist)
    Install(InstallArgs),

    /// Update tt itself and any outdated tools
    Update(UpdateArgs),

    /// Remove installed tools
    Remove(RemoveArgs),

    /// List every tool and its status
    List,

    /// Show what a tool does and how to use it
    Usage(UsageArgs),

    /// Install shell completions
    Completions(commands::completions::CompletionsArgs),

    /// Former command layout, kept working. Prefer `tt install` and friends.
    #[command(hide = true)]
    Tools(ToolsArgs),
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
    Update(UpdateArgs),
    Remove(RemoveArgs),
    Usage(UsageArgs),
}

#[derive(Debug, Clone, Args)]
pub struct InstallArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser())]
    pub ids: Vec<String>,
    /// Install every bundled tool
    #[arg(short, long)]
    pub all: bool,
    #[arg(short, long, hide = true)]
    pub verbose: bool,
    #[arg(short = 'y', long, hide = true)]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UpdateArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser(), conflicts_with_all = ["all", "self_only"])]
    pub ids: Vec<String>,
    /// Reinstall every tool, current or not
    #[arg(short, long)]
    pub all: bool,
    /// Only update the tt binary, leave tools alone
    #[arg(long = "self", name = "self_only")]
    pub self_only: bool,
    #[arg(short, long, hide = true)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Args)]
pub struct RemoveArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser(), required = true)]
    pub ids: Vec<String>,
    /// Skip the confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,
}

#[derive(Debug, Clone, Args)]
pub struct UsageArgs {
    #[arg(value_name = "IDS", value_parser = tool_id_value_parser())]
    pub ids: Vec<String>,
    /// Include tools you have not installed
    #[arg(short, long, conflicts_with = "ids")]
    pub all: bool,
}

pub fn run() -> Result<()> {
    let cli = parse();

    let is_update = matches!(cli.command.as_ref(), Some(Commands::Update(_)));

    // The update command does its own checking; everywhere else this is the
    // once-a-day nudge.
    if !is_update {
        update::maybe_check(cli.no_update_check, false);
    }

    let command = match cli.command {
        Some(Commands::Tools(tools)) => Some(flatten(tools.command)),
        other => other,
    };

    let registry = Registry::load()?;

    // Tools that track someone else's releases can only be told they are
    // behind by asking. This is the one place that costs a network call, and
    // it is rate-limited to once a day per tool — `tt update` forces it,
    // because typing it is a request to look now.
    if !cli.no_update_check && !update_check_disabled_by_env() {
        upstream::refresh(&registry, is_update);
    }

    match command {
        Some(Commands::Install(args)) => commands::install::run(
            &registry,
            &commands::install::Request::from_args(args.ids, args.all),
        ),
        Some(Commands::Update(args)) => update_command(&registry, args),
        Some(Commands::Remove(args)) => commands::remove::run(&registry, &args.ids, args.yes),
        Some(Commands::List) => commands::list::run(&registry),
        Some(Commands::Usage(args)) => commands::usage::run(
            &registry,
            &commands::usage::Request::from_args(args.ids, args.all),
        ),
        Some(Commands::Completions(args)) => commands::completions::run(args),
        Some(Commands::Tools(_)) => unreachable!("flattened above"),
        None => commands::home::run(&registry),
    }
}

/// `tt update` means "make everything current" — the binary and the tools.
/// Splitting those across two commands was a distinction only the author cared
/// about.
fn update_command(registry: &Registry, args: UpdateArgs) -> Result<()> {
    if args.self_only {
        return update::run();
    }

    let targeted = args.all || !args.ids.is_empty();
    if !targeted {
        update::update_self_if_newer();
    }

    commands::tools_update::run(
        registry,
        &commands::tools_update::Request::from_args(args.ids, args.all),
    )
}

fn flatten(command: ToolsCommand) -> Commands {
    match command {
        ToolsCommand::List => Commands::List,
        ToolsCommand::Install(args) => Commands::Install(args),
        ToolsCommand::Update(args) => Commands::Update(args),
        ToolsCommand::Remove(args) => Commands::Remove(args),
        ToolsCommand::Usage(args) => Commands::Usage(args),
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
    "Run tt with no arguments and it will show you what needs doing.".to_owned()
}

fn update_check_disabled_by_env() -> bool {
    matches!(std::env::var("TT_NO_UPDATE_CHECK").as_deref(), Ok("1"))
}

fn tool_id_value_parser() -> PossibleValuesParser {
    PossibleValuesParser::new(Registry::embedded_tool_ids())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands, ToolsCommand};

    #[test]
    fn allows_running_without_a_subcommand() {
        let cli = Cli::try_parse_from(["tt"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn exposes_the_flat_commands() {
        for (argv, matches) in [
            (vec!["tt", "install"], "install"),
            (vec!["tt", "update"], "update"),
            (vec!["tt", "list"], "list"),
            (vec!["tt", "usage"], "usage"),
            (vec!["tt", "remove", "jsut-alias"], "remove"),
        ] {
            let cli = Cli::try_parse_from(&argv)
                .unwrap_or_else(|error| panic!("{matches} should parse: {error}"));
            assert!(cli.command.is_some(), "{matches} produced no command");
        }
    }

    /// Muscle memory and any script written against the old layout must keep
    /// working, so the nested form stays — just hidden from help.
    #[test]
    fn the_old_tools_subcommands_still_parse() {
        for argv in [
            vec!["tt", "tools", "list"],
            vec!["tt", "tools", "install", "--all"],
            vec!["tt", "tools", "update"],
            vec!["tt", "tools", "usage"],
        ] {
            let cli = Cli::try_parse_from(&argv)
                .unwrap_or_else(|error| panic!("{argv:?} should still parse: {error}"));
            assert!(matches!(cli.command, Some(Commands::Tools(_))));
        }
    }

    #[test]
    fn old_and_new_forms_resolve_to_the_same_command() {
        let old = Cli::try_parse_from(["tt", "tools", "list"]).unwrap();
        let Some(Commands::Tools(tools)) = old.command else {
            panic!("expected the nested form");
        };
        assert!(matches!(super::flatten(tools.command), Commands::List));
    }

    #[test]
    fn update_accepts_self_only() {
        let cli = Cli::try_parse_from(["tt", "update", "--self"]).unwrap();
        let Some(Commands::Update(args)) = cli.command else {
            panic!("expected update");
        };
        assert!(args.self_only);
    }

    #[test]
    fn remove_requires_at_least_one_id() {
        assert!(Cli::try_parse_from(["tt", "remove"]).is_err());
    }

    #[test]
    fn unknown_tool_ids_are_rejected_before_anything_runs() {
        assert!(Cli::try_parse_from(["tt", "install", "no-such-tool"]).is_err());
    }

    #[test]
    fn help_leads_with_the_flat_commands() {
        let mut command = super::command();
        let mut buffer = Vec::new();
        command.write_long_help(&mut buffer).unwrap();
        let help = String::from_utf8(buffer).unwrap();

        for expected in ["install", "update", "remove", "list", "usage"] {
            assert!(help.contains(expected), "help should mention {expected}");
        }
        // The legacy layout is hidden, not advertised.
        assert!(!help.contains("Former command layout"));
    }

    #[test]
    fn tools_command_variants_are_exhaustively_flattened() {
        // A compile-time reminder: adding a ToolsCommand variant forces a
        // matching flat command.
        fn _exhaustive(command: ToolsCommand) -> Commands {
            super::flatten(command)
        }
    }
}
