use crate::app::App;
use clap::Parser;
use cli::Cli;
use ratatui::{
    Terminal, backend,
    crossterm::{execute, terminal::*},
};
use std::io::stderr;
use tracing::info;

pub mod app;
pub mod cli;
pub mod colors;
pub mod command;
pub mod completion;
pub mod config;
pub mod debug_log;
pub mod event;
pub mod event_mark_view;
pub mod filter;
pub mod help;
pub mod highlighter;
pub mod history;
pub mod keybindings;
pub mod list_view_state;
pub mod log;
pub mod log_event;
pub mod log_processor;
pub mod marking;
pub mod options;
pub mod persistence;
pub mod processing;
pub mod search;
pub mod ui;
pub mod utils;
pub mod viewport;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let args = Cli::parse();

    if let Some(ref debug_path) = args.debug {
        debug_log::init(debug_path)?;
    }

    info!("Starting lazylog with args: {:?}", args);

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
