use clap::Parser;
use lazylog::{app::App, cli::Cli, debug_log};
use ratatui::{
    Terminal, backend,
    crossterm::{execute, terminal::*},
};
use std::io::{LineWriter, stderr};
use tracing::info;

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

    // Use line-buffered stderr for better terminal I/O performance
    // LineWriter flushes on newlines, which matches terminal escape sequence behavior
    let backend = backend::CrosstermBackend::new(LineWriter::new(stderr()));
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
