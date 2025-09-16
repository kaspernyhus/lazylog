use crate::app::App;
use clap::Parser;
use cli::Cli;

pub mod app;
pub mod cli;
pub mod event;
pub mod log;
pub mod ui;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let args = Cli::parse();
    let terminal = ratatui::init();
    let app = App::new(args);
    let result = app.run(terminal).await;
    ratatui::restore();
    result
}
