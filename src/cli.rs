use clap::Parser;
use std::io::IsTerminal;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Log file path(s). If not provided, reads from stdin.
    pub files: Vec<String>,

    /// Path to config file
    #[arg(short, long)]
    pub config: Option<String>,

    /// Path to filters file (TOML file containing predefined filters)
    #[arg(short, long)]
    pub filters: Option<String>,

    /// Clear all persisted state files
    #[arg(long)]
    pub clear_state: bool,

    /// Disable persistence
    #[arg(long)]
    pub no_persist: bool,

    /// Enable debug logging to file
    #[arg(long)]
    pub debug: Option<String>,
}

impl Cli {
    pub fn should_use_stdin(&self) -> bool {
        self.files.is_empty() && !std::io::stdin().is_terminal()
    }
}
