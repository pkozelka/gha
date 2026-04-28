use clap::ValueEnum;
use clap_complete::generate;
use std::io;

/// Shell type for completion generation
#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// Powershell
    Powershell,
    /// Elvish shell
    Elvish,
}

impl From<Shell> for clap_complete::shells::Shell {
    fn from(shell: Shell) -> Self {
        match shell {
            Shell::Bash => clap_complete::shells::Shell::Bash,
            Shell::Zsh => clap_complete::shells::Shell::Zsh,
            Shell::Fish => clap_complete::shells::Shell::Fish,
            Shell::Powershell => clap_complete::shells::Shell::PowerShell,
            Shell::Elvish => clap_complete::shells::Shell::Elvish,
        }
    }
}

/// Generate shell completions
pub fn generate_completion(shell: Shell) -> io::Result<()> {
    use clap::CommandFactory;
    let mut cmd = crate::Cli::command();
    let shell: clap_complete::shells::Shell = shell.into();
    generate(shell, &mut cmd, "gha", &mut io::stdout());
    Ok(())
}


