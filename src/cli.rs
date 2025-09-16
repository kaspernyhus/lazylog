use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
pub struct Cli {
    /// Log file path
    pub file: Option<String>,
}
