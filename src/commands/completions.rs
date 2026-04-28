use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Context, Result};
use clap::{Args, CommandFactory, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};

use crate::cli::Cli;

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct CompletionsArgs {
    #[arg(value_enum)]
    pub shell: Option<Shell>,
    #[command(subcommand)]
    pub command: Option<CompletionsCommand>,
}

#[derive(Debug, Subcommand)]
pub enum CompletionsCommand {
    Install(CompletionInstallArgs),
}

#[derive(Debug, Clone, Args)]
pub struct CompletionInstallArgs {
    #[arg(value_enum)]
    pub shell: Option<Shell>,
}

pub fn run(args: CompletionsArgs) -> Result<()> {
    match args.command {
        Some(CompletionsCommand::Install(args)) => install(args.shell),
        None => print(args.shell),
    }
}

fn print(shell: Option<Shell>) -> Result<()> {
    let shell = shell.ok_or_else(missing_shell_error)?;
    print!("{}", script_with_header(shell)?);
    Ok(())
}

fn install(shell: Option<Shell>) -> Result<()> {
    let shell = resolve_shell(shell)?;
    let config = shell_config(shell);
    let path = completion_path(config.relative_path())?;
    let content = script_with_header(shell)?;

    if fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
        println!("✓ already installed at {}", path.display());
        println!("Paste this into {}:", config.rc_file());
        println!("{}", config.append_one_liner());
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create {parent:?}"))?;
    }
    fs::write(&path, content).with_context(|| format!("failed to write {path:?}"))?;

    println!("✓ installed completions to {}", path.display());
    println!("Paste this into {}:", config.rc_file());
    println!("{}", config.append_one_liner());
    Ok(())
}

fn script_with_header(shell: Shell) -> Result<String> {
    let mut script = Vec::new();
    let mut command = Cli::command();
    generate(shell, &mut command, "tt", &mut script);

    Ok(format!(
        "{}\n{}",
        shell_config(shell).header_comment(),
        String::from_utf8(script).context("generated completion script was not valid UTF-8")?
    ))
}

fn resolve_shell(shell: Option<Shell>) -> Result<Shell> {
    shell
        .or_else(|| env::var("SHELL").ok().as_deref().and_then(parse_shell_name))
        .ok_or_else(|| {
            anyhow!(
                "could not detect shell from $SHELL. Supported shells: {}",
                supported_shells()
            )
        })
}

fn missing_shell_error() -> anyhow::Error {
    anyhow!("shell required. Supported shells: {}", supported_shells())
}

fn supported_shells() -> String {
    Shell::value_variants()
        .iter()
        .filter_map(|shell| {
            shell
                .to_possible_value()
                .map(|value| value.get_name().to_owned())
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn parse_shell_name(shell: &str) -> Option<Shell> {
    let name = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())?
        .trim_end_matches(".exe")
        .to_ascii_lowercase();

    match name.as_str() {
        "bash" => Some(Shell::Bash),
        "elvish" => Some(Shell::Elvish),
        "fish" => Some(Shell::Fish),
        "pwsh" | "powershell" => Some(Shell::PowerShell),
        "zsh" => Some(Shell::Zsh),
        _ => None,
    }
}

fn completion_path(relative_path: &str) -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| anyhow!("HOME is not set"))?;
    Ok(Path::new(&home).join(relative_path))
}

fn shell_config(shell: Shell) -> ShellConfig {
    match shell {
        Shell::Bash => ShellConfig::new(
            "bash",
            ".config/tt/completions.bash",
            "~/.bashrc",
            "source <(tt completions bash)",
            "source ~/.config/tt/completions.bash",
            "echo 'source ~/.config/tt/completions.bash' >> ~/.bashrc",
        ),
        Shell::Elvish => ShellConfig::new(
            "elvish",
            ".config/tt/completions.elvish",
            "~/.elvish/rc.elv",
            "eval (tt completions elvish | slurp)",
            "eval (cat ~/.config/tt/completions.elvish | slurp)",
            "echo 'eval (cat ~/.config/tt/completions.elvish | slurp)' >> ~/.elvish/rc.elv",
        ),
        Shell::Fish => ShellConfig::new(
            "fish",
            ".config/tt/completions.fish",
            "~/.config/fish/config.fish",
            "tt completions fish | source",
            "source ~/.config/tt/completions.fish",
            "echo 'source ~/.config/tt/completions.fish' >> ~/.config/fish/config.fish",
        ),
        Shell::PowerShell => ShellConfig::new(
            "powershell",
            ".config/tt/completions.powershell",
            "$PROFILE",
            "tt completions powershell | Out-String | Invoke-Expression",
            ". ~/.config/tt/completions.powershell",
            "Add-Content -Path $PROFILE -Value '. ~/.config/tt/completions.powershell'",
        ),
        Shell::Zsh => ShellConfig::new(
            "zsh",
            ".config/tt/completions.zsh",
            "~/.zshrc",
            "source <(tt completions zsh)",
            "source ~/.config/tt/completions.zsh",
            "echo 'source ~/.config/tt/completions.zsh' >> ~/.zshrc",
        ),
        _ => unreachable!("unsupported shell variant"),
    }
}

struct ShellConfig {
    name: &'static str,
    relative_path: &'static str,
    rc_file: &'static str,
    session_command: &'static str,
    file_source_command: &'static str,
    append_one_liner: &'static str,
}

impl ShellConfig {
    const fn new(
        name: &'static str,
        relative_path: &'static str,
        rc_file: &'static str,
        session_command: &'static str,
        file_source_command: &'static str,
        append_one_liner: &'static str,
    ) -> Self {
        Self {
            name,
            relative_path,
            rc_file,
            session_command,
            file_source_command,
            append_one_liner,
        }
    }

    fn header_comment(&self) -> String {
        format!(
            concat!(
                "# tt completions for {shell}\n",
                "# Load in the current shell:\n",
                "#   {session}\n",
                "# Persist across sessions:\n",
                "#   {install}\n",
                "# Or add this to {rc_file}:\n",
                "#   {source_file}\n"
            ),
            shell = self.name,
            session = self.session_command,
            install = format!("tt completions install {}", self.name),
            rc_file = self.rc_file,
            source_file = self.file_source_command,
        )
    }

    const fn relative_path(&self) -> &'static str {
        self.relative_path
    }

    const fn rc_file(&self) -> &'static str {
        self.rc_file
    }

    const fn append_one_liner(&self) -> &'static str {
        self.append_one_liner
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_shell_name, shell_config};
    use clap_complete::Shell;

    #[test]
    fn parses_shell_from_env_basename() {
        assert_eq!(parse_shell_name("/bin/zsh"), Some(Shell::Zsh));
        assert_eq!(
            parse_shell_name("/opt/homebrew/bin/pwsh"),
            Some(Shell::PowerShell)
        );
    }

    #[test]
    fn zsh_install_line_points_at_shared_completion_file() {
        assert_eq!(
            shell_config(Shell::Zsh).append_one_liner(),
            "echo 'source ~/.config/tt/completions.zsh' >> ~/.zshrc"
        );
    }
}
