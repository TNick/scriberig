//! Dispatches upstream tool calls through `ScribeRig` authorization.

use std::sync::{Arc, Mutex};

use rpi_agent::{AgentError, AgentToolResult};
use scriberig_agent_runtime::ChildExecutionScope;
use scriberig_agent_runtime::tools::ToolDescriptor;
use scriberig_agent_runtime::upstream_invocation::UpstreamBranchInvocation;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationService, ToolExecutionEvidence,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::agent_tool::LocalAgentTool;

use super::{MAX_TOOL_ARGUMENT_BYTES, ToolRequest, delegation, evidence};

/// Signals the cancellation monitor when semantic dispatch exits or unwinds.
struct CancellationMonitorGuard {
    /// Shared completion flag polled by the monitor thread.
    finished: Arc<std::sync::atomic::AtomicBool>,
}

impl Drop for CancellationMonitorGuard {
    /// Stops the monitor thread during ordinary return or panic unwinding.
    fn drop(&mut self) {
        self.finished
            .store(true, std::sync::atomic::Ordering::Release);
    }
}

/// Dispatches branch tool requests and settles recursive child invocations.
///
/// # Parameters
/// - `service`: Borrowed host application boundary.
/// - `context`: Trusted caller context for application operations.
/// - `session`: Canonical session identity for durable children.
/// - `scope`: Current `ScribeRig` branch scope.
/// - `project_store`: Durable project store for application tools.
/// - `invocation`: Provider/model values inherited by child branches.
/// - `catalog`: Trusted tool catalog.
/// - `descriptors`: Branch-visible tool metadata.
/// - `receiver`: Scoped host broker request queue.
/// - `evidence`: Shared ordered evidence sink.
/// - `cancellation`: Branch cancellation token.
/// - `output_callback`: Provider-visible assistant text sink.
///
/// # Returns
/// Success after all upstream tools have been dropped and the queue closes.
///
/// # Errors
/// Returns a safe broker settlement error.
#[allow(clippy::too_many_arguments)]
pub(super) async fn dispatch_tools<B: ApplicationBackend + Sync>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    scope: ChildExecutionScope,
    project_store: Arc<SqliteStore>,
    invocation: UpstreamBranchInvocation,
    descriptors: Vec<ToolDescriptor>,
    mut receiver: mpsc::UnboundedReceiver<ToolRequest>,
    evidence: Arc<Mutex<Vec<ToolExecutionEvidence>>>,
    cancellation: CancellationToken,
    output_callback: Option<scriberig_agent_runtime::OutputCallback>,
) -> Result<(), String> {
    let local_tool = LocalAgentTool::new(service).with_project_store(Arc::clone(&project_store));
    let mut ordinal = 0_u64;
    let mut tool_calls = 0_usize;
    let mut child_branches = 0_usize;
    while let Some(request) = receiver.recv().await {
        tool_calls = tool_calls.saturating_add(1);
        if tool_calls
            > scriberig_agent_runtime::orchestration::RuntimeLimits::default().max_tool_calls
        {
            let _ = request
                .response
                .send(Err(AgentError::tool("tool call limit reached")));
            continue;
        }
        if serde_json::to_vec(&request.arguments)
            .is_ok_and(|arguments| arguments.len() > MAX_TOOL_ARGUMENT_BYTES)
        {
            let _ = request.response.send(Err(AgentError::tool(
                "tool arguments exceed the branch limit",
            )));
            continue;
        }
        if cancellation.is_cancelled() || request.cancellation.is_cancelled() {
            let _ = request
                .response
                .send(Err(AgentError::tool("semantic operation cancelled")));
            continue;
        }
        let descriptor = descriptors
            .iter()
            .find(|descriptor| descriptor.definition.name == request.name)
            .ok_or_else(|| "tool descriptor disappeared from branch catalog".to_owned())?;
        let result = if request.name == "delegate" {
            child_branches = child_branches.saturating_add(1);
            if child_branches
                > scriberig_agent_runtime::orchestration::RuntimeLimits::default()
                    .max_child_branches
            {
                let _ = request
                    .response
                    .send(Err(AgentError::tool("child branch limit reached")));
                continue;
            }
            delegation::dispatch_child(
                service,
                context,
                session,
                &scope,
                &invocation,
                &request,
                Arc::clone(&project_store),
                cancellation.child_token(),
                output_callback.clone(),
            )
            .await
        } else {
            invoke_semantic_tool(&local_tool, context, &request)
        };
        let failed = result.is_err();
        let evidence_record = evidence::tool_evidence(
            ordinal,
            descriptor,
            &request.arguments,
            result.as_ref().ok(),
            failed,
        );
        ordinal = ordinal.saturating_add(1);
        if let Ok(mut records) = evidence.lock() {
            records.push(evidence_record);
        }
        let response = result.map(|(value, _)| AgentToolResult::text(value));
        let _ = request.response.send(response);
    }
    Ok(())
}

/// Invokes an authorized non-delegation semantic operation synchronously.
///
/// # Parameters
/// - `tool`: Local shared application operation adapter.
/// - `context`: Trusted caller context.
/// - `request`: Validated upstream tool call.
///
/// # Returns
/// Compact safe JSON output and an optional child branch identity.
fn invoke_semantic_tool<B: ApplicationBackend + Sync>(
    tool: &LocalAgentTool<'_, B>,
    context: &ApplicationContext,
    request: &ToolRequest,
) -> Result<(String, Option<EntityId>), AgentError> {
    let cancelled = Arc::new(std::sync::atomic::AtomicBool::new(
        request.cancellation.is_cancelled(),
    ));
    let finished = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let token = request.cancellation.clone();
    let monitor_cancelled = Arc::clone(&cancelled);
    let monitor_finished = Arc::clone(&finished);
    let result = std::thread::scope(|scope| {
        let monitor = scope.spawn(move || {
            while !monitor_finished.load(std::sync::atomic::Ordering::Acquire) {
                if token.is_cancelled() {
                    monitor_cancelled.store(true, std::sync::atomic::Ordering::Release);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });
        let _completion_guard = CancellationMonitorGuard {
            finished: Arc::clone(&finished),
        };
        let result = (|| -> Result<_, AgentError> {
            let operation = if request.name == "inspect" {
                let entity = request.arguments["entity"]
                    .as_str()
                    .ok_or_else(|| AgentError::tool("invalid entity argument"))?
                    .parse::<EntityId>()
                    .map_err(|_| AgentError::tool("invalid entity argument"))?;
                let snapshot = request.arguments["snapshot"]
                    .as_str()
                    .map(str::parse::<scriberig_core::SnapshotId>)
                    .transpose()
                    .map_err(|_| AgentError::tool("invalid snapshot argument"))?;
                let detail = match request.arguments["detail"].as_str() {
                    Some("detailed") => scriberig_agent_core::ToolDetail::Detailed,
                    _ => scriberig_agent_core::ToolDetail::Compact,
                };
                scriberig_agent_core::Tool::invoke(
                    tool,
                    context,
                    scriberig_agent_core::ToolInput {
                        entity,
                        snapshot,
                        detail,
                    },
                )
            } else {
                scriberig_agent_core::Tool::invoke_named_cancellable(
                    tool,
                    context,
                    &request.name,
                    &request.arguments,
                    cancelled.as_ref(),
                )
            };
            operation.map_err(|error| {
                let message = match error {
                    scriberig_agent_core::ToolError::CapabilityDenied { action, scope } => {
                        format!("capability denied: {action} ({scope})")
                    }
                    scriberig_agent_core::ToolError::Application { category, message } => {
                        format!("{category}: {message}")
                    }
                };
                AgentError::tool(message.chars().take(500).collect::<String>())
            })
        })();
        finished.store(true, std::sync::atomic::Ordering::Release);
        let _ = monitor.join();
        result
    })?;
    let mut handles = result.handles;
    if handles.len() > 32 {
        if let Some(index) = handles
            .iter()
            .position(|handle| handle.starts_with("next_offset:"))
        {
            let continuation = handles.remove(index);
            handles.truncate(31);
            handles.push(continuation);
        } else {
            handles.truncate(32);
        }
    }
    let output = serde_json::json!({
        "entity": result.entity.to_string(),
        "revision": result.revision.map(|revision| revision.to_string()),
        "snapshot": result.snapshot.map(|snapshot| snapshot.to_string()),
        "object_type": result.object_type,
        "summary": result.summary.map(|summary| evidence::bounded_text(&summary, 2_048)),
        "handles": handles,
    });
    Ok((output.to_string(), None))
}
