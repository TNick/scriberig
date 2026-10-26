//! Configures concise stderr logging for the integrated product executable.

/// Installs the process tracing subscriber before product or worker startup.
///
/// `RUST_LOG` can raise or narrow the default `warn` filter. Invalid filter
/// values fall back to warnings so startup diagnostics remain visible.
pub(crate) fn initialize() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_writer(std::io::stderr)
        .try_init();
}
