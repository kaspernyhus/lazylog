use color_eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize debug logging to a file using tracing.
///
/// Uses RUST_LOG environment variable for filtering, or defaults to INFO level.
/// Examples:
///   RUST_LOG=lazylog::viewport=debug  - Only debug viewport module
pub fn init(path: &str) -> Result<()> {
    let log_file = std::fs::File::create(path)?;

    let env_filter = EnvFilter::builder()
        .with_default_directive(tracing::Level::INFO.into())
        .try_from_env()
        .or_else(|_| EnvFilter::try_new("info"))?;

    let file_subscriber = fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(log_file)
        .with_target(false)
        .with_ansi(false)
        .with_filter(env_filter);

    tracing_subscriber::registry()
        .with(file_subscriber)
        .with(ErrorLayer::default())
        .try_init()?;

    Ok(())
}
