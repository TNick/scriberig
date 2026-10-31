//! Deterministic isolated plugin worker used only by the composition tests.
//!
//! The binary serves the upstream `rpi-plugin-worker-v1` protocol over piped
//! stdio through `rpi_extensions::isolated::serve_worker_stdio`, which is the
//! same worker-side driver the product worker mode uses. It advertises the
//! fixture tools named in `SCRIG_FIXTURE_TOOLS` and reports only what the
//! worker process can actually observe: its own environment, the arguments it
//! received, and whether the host sent a cancel frame.

use std::collections::BTreeMap;
use std::env;
use std::sync::Arc;

use rpi_extensions::isolated::{WorkerHandler, WorkerToolSpec};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

/// Tool that echoes the worker's environment and call arguments.
const ECHO_TOOL: &str = "fixture.echo";
/// Tool that returns a small fixed result.
const REPORT_TOOL: &str = "fixture.report";
/// Tool that waits for a host cancel frame.
const WAIT_TOOL: &str = "fixture.wait";
/// Tool that always reports a worker-side failure.
const FAIL_TOOL: &str = "fixture.fail";

/// Fixture tool set served by one worker process.
struct FixtureWorker {
    /// Tool identifiers this worker advertises.
    tools: Vec<String>,
}

impl WorkerHandler for FixtureWorker {
    /// Advertises the fixture tools selected by the host environment.
    ///
    /// # Returns
    /// One `WorkerToolSpec` per selected fixture tool.
    fn tools(&self) -> Vec<WorkerToolSpec> {
        self.tools
            .iter()
            .map(|name| WorkerToolSpec {
                name: name.clone(),
                description: "Deterministic plugin worker fixture".to_owned(),
                parameters: json!({"type": "object", "additionalProperties": true}),
            })
            .collect()
    }

    /// Executes one fixture tool inside the worker process.
    ///
    /// # Parameters
    /// - `tool`: Advertised fixture tool identifier.
    /// - `arguments`: Arguments sent by the host.
    /// - `cancellation`: Worker-side cancellation token driven by the host
    ///   cancel frame.
    ///
    /// # Returns
    /// A bounded JSON result for the host.
    ///
    /// # Errors
    /// Returns a safe failure for an unadvertised tool, a cancelled wait, or the
    /// always-failing tool.
    fn call_tool(
        &self,
        tool: &str,
        arguments: Value,
        cancellation: CancellationToken,
    ) -> Result<Value, String> {
        match tool {
            ECHO_TOOL => Ok(json!({
                "environment": observed_environment(),
                "arguments": arguments,
            })),
            REPORT_TOOL => Ok(json!({"report": "fixture result"})),
            WAIT_TOOL => {
                while !cancellation.is_cancelled() {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                note_cancellation();
                Err("fixture wait cancelled".to_owned())
            }
            FAIL_TOOL => Err("fixture tool failure".to_owned()),
            other => Err(format!("unadvertised fixture tool: {other}")),
        }
    }
}

/// Returns the environment this worker process actually received.
///
/// # Returns
/// Every visible environment name and value, sorted by name.
fn observed_environment() -> BTreeMap<String, String> {
    env::vars().collect()
}

/// Records that the worker observed a host cancel frame.
///
/// The marker path is host-declared so the test can prove the cancel frame
/// crossed the process boundary rather than only the host-side deadline.
fn note_cancellation() {
    if let Ok(marker) = env::var("SCRIG_FIXTURE_CANCEL_MARKER")
        && let Err(error) = std::fs::write(marker, "cancelled")
    {
        eprintln!("fixture could not write the cancellation marker: {error}");
    }
}

/// Serves the fixture worker protocol on this process' standard streams.
///
/// # Returns
/// `Ok(()))` after the host closes the stream or requests shutdown.
///
/// # Errors
/// Returns the worker driver's failure text.
fn main() -> Result<(), String> {
    let tools = env::var("SCRIG_FIXTURE_TOOLS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    rpi_extensions::isolated::serve_worker_stdio(Arc::new(FixtureWorker { tools }))
        .map_err(|error| error.to_string())
}
