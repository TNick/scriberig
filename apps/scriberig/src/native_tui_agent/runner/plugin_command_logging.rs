//! Logs safe classifications for isolated plugin command failures.

use rpi_extensions::isolated::WorkerError;
use scriberig_agent_runtime::host_configuration::HostConfigurationError;

/// Logs a safe upstream worker failure classification for one plugin command.
///
/// The upstream error's detailed text may originate from plugin-controlled
/// output, so only its stable variant category and the public command name are
/// recorded.
///
/// - `command`: Public upstream command identity.
/// - `error`: Worker failure to classify.
pub(super) fn log_plugin_command_failure(command: &str, error: &WorkerError) {
    let kind = match error {
        WorkerError::Spawn(_) => "spawn",
        WorkerError::StartupTimeout => "startup_timeout",
        WorkerError::RequestTimeout => "request_timeout",
        WorkerError::Closed(_) => "closed",
        WorkerError::Tool(_) => "tool",
        WorkerError::Command(_) => "command",
        WorkerError::Unauthorized(_) => "unauthorized",
        WorkerError::Cancelled => "cancelled",
        WorkerError::Sandbox(_) => "sandbox",
        WorkerError::Protocol(_) => "protocol",
    };
    if let WorkerError::Closed(reason) = error {
        tracing::warn!(
            command,
            error_kind = kind,
            reason,
            "upstream plugin command invocation failed"
        );
    } else {
        tracing::warn!(
            command,
            error_kind = kind,
            "upstream plugin command invocation failed"
        );
    }
}

/// Logs a safe classification when reloading a closed command worker fails.
///
/// - `error`: Host-side package refresh failure.
pub(super) fn log_plugin_command_refresh_failure(error: &HostConfigurationError) {
    let kind = match error {
        HostConfigurationError::Configuration(_) => "configuration",
        HostConfigurationError::Scenario(_) => "scenario",
    };
    tracing::warn!(
        error_kind = kind,
        "upstream plugin command worker refresh failed"
    );
}
