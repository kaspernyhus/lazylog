use clap::Parser;
use std::io::IsTerminal;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Log file path (if not provided, reads from stdin)
    pub file: Option<String>,

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
        self.file.is_none() && !std::io::stdin().is_terminal()
    }
}
