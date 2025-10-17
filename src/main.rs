use crate::app::App;
use clap::Parser;
use cli::Cli;
use ratatui::{
    Terminal, backend,
    crossterm::{execute, terminal::*},
};
use std::io::stderr;

pub mod app;
pub mod cli;
pub mod command;
pub mod config;
pub mod display_options;
pub mod event;
pub mod filter;
pub mod help;
pub mod highlighter;
pub mod keybindings;
pub mod log;
pub mod log_event;
pub mod marking;
pub mod search;
pub mod ui;
pub mod viewport;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Cli::parse();

    set_panic_hook();
    color_eyre::install()?;

    execute!(stderr(), EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = backend::CrosstermBackend::new(stderr());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let app = App::new(args);
    let result = app.run(terminal).await;

    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen)?;

    result
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stderr(), LeaveAlternateScreen);
        hook(panic_info);
    }));
}
