use color_eyre::Result;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

pub fn init() -> Result<()> {
    let log_file = std::fs::File::create("app-logs/lazylog.log")?;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .try_init()?;

    Ok(())
}
