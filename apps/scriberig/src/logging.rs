//! Configures concise stderr or explicitly requested file logging for the product.

/// Installs the process tracing subscriber before product or worker startup.
///
/// `RUST_LOG` can raise or narrow the default `warn` filter. Setting
/// `SCRIBE_LOG_FILE` sends diagnostics to that append-only file instead of
/// stderr. Invalid filters fall back to warnings so startup diagnostics remain
/// visible.
pub(crate) fn initialize() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    if let Some(path) = std::env::var_os("SCRIBE_LOG_FILE") {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_target(false)
                .with_writer(file)
                .try_init();
            return;
        }
    }
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
