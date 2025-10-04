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
}

impl Cli {
    pub fn should_use_stdin(&self) -> bool {
        self.file.is_none() && !std::io::stdin().is_terminal()
    }
}
