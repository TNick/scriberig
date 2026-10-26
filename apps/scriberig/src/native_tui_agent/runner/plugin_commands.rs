//! Exposes explicitly authorized upstream package commands to the native TUI.

use super::super::NativeTuiAgentRunner;
use super::plugin_command_logging::{
    log_plugin_command_failure, log_plugin_command_refresh_failure,
};
/// Returns current allowlisted command descriptors from the upstream worker.
///
/// - `runner`: Native runner that owns the package worker lifecycle.
///
/// # Returns
/// Commands available from the current healthy worker, or an empty list.
pub(super) fn plugin_commands(
    runner: &NativeTuiAgentRunner,
) -> Vec<rpi_extensions::isolated::WorkerCommandSpec> {
    let is_closed = runner
        .plugin_command_worker
        .lock()
        .ok()
        .and_then(|worker| worker.as_ref().map(|worker| worker.is_closed()))
        .unwrap_or(false);
    if is_closed && let Err(error) = runner.refresh_provider_packages() {
        log_plugin_command_refresh_failure(&error);
    }
    runner
        .plugin_command_worker
        .lock()
        .ok()
        .and_then(|worker| {
            let worker = worker.as_ref()?;
            (!worker.is_closed())
                .then(|| worker.advertised_commands().ok())
                .flatten()
        })
        .unwrap_or_default()
}

/// Invokes one allowlisted command through its upstream isolated worker.
///
/// # Parameters
/// - `runner`: Native runner that owns the package worker lifecycle.
/// - `command`: Exact normalized command identity.
/// - `arguments`: Raw text following the command name.
///
/// # Returns
/// The structured plugin response.
///
/// # Errors
/// Returns a safe error when the command is unavailable or execution fails.
pub(super) fn invoke_plugin_command(
    runner: &NativeTuiAgentRunner,
    command: &str,
    arguments: &str,
) -> Result<serde_json::Value, String> {
    let worker = runner
        .plugin_command_worker
        .lock()
        .map_err(|_| "plugin command worker is unavailable".to_owned())?;
    let worker = worker
        .as_ref()
        .ok_or_else(|| "plugin commands are unavailable".to_owned())?;
    worker.call_command(command, arguments).map_err(|error| {
        log_plugin_command_failure(command, &error);
        "upstream plugin command failed".to_owned()
    })
}

/// Continues an allowlisted command with a structured upstream UI action.
///
/// # Parameters
/// - `runner`: Native runner that owns the package worker lifecycle.
/// - `command`: Exact normalized command identity.
/// - `arguments`: Raw text following the command name.
/// - `action`: Structured selection or text submitted to the plugin.
///
/// # Returns
/// The next structured plugin response.
///
/// # Errors
/// Returns a safe error when the command is unavailable or execution fails.
pub(super) fn invoke_plugin_command_action(
    runner: &NativeTuiAgentRunner,
    command: &str,
    arguments: &str,
    action: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let worker = runner
        .plugin_command_worker
        .lock()
        .map_err(|_| "plugin command worker is unavailable".to_owned())?;
    let worker = worker
        .as_ref()
        .ok_or_else(|| "plugin commands are unavailable".to_owned())?;
    worker
        .call_command_action(command, arguments, Some(action))
        .map_err(|error| {
            log_plugin_command_failure(command, &error);
            "upstream plugin command failed".to_owned()
        })
}
