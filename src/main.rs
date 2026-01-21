use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use lazylog::{app::App, cli::Cli, debug_log};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{LineWriter, stderr, stdout};
use tracing::info;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args = Cli::parse();

    if let Some(ref debug_path) = args.debug {
        debug_log::init(debug_path)?;
    }

    info!("Starting lazylog with args: {:?}", args);

    if args.should_use_stdin() {
        run_streaming_mode(args).await
    } else {
        run_file_mode(args).await
    }
}

async fn run_streaming_mode(args: Cli) -> color_eyre::Result<()> {
    info!("Drawing to stderr");
    set_panic_hook_stderr();
    enable_raw_mode()?;
    execute!(stderr(), EnterAlternateScreen)?;

    // Use line-buffered stderr for better terminal I/O performance
    // LineWriter flushes on newlines, which matches terminal escape sequence behavior
    let backend = CrosstermBackend::new(LineWriter::new(stderr()));
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let app = App::new(args);
    let result = app.run(terminal).await;

    disable_raw_mode()?;
    execute!(stderr(), LeaveAlternateScreen)?;
    result
}

async fn run_file_mode(args: Cli) -> color_eyre::Result<()> {
    info!("Drawing to stdout");
    set_panic_hook_stdout();
    enable_raw_mode()?;

    execute!(stdout(), EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    terminal.clear()?;

    let app = App::new(args);
    let result = app.run(terminal).await;

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    result
}

fn set_panic_hook_stderr() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stderr(), LeaveAlternateScreen);
        hook(panic_info);
    }));
}

fn set_panic_hook_stdout() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
        hook(panic_info);
    }));
}
