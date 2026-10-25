//! Catalog registration and adapter construction for one authorized worker.

use std::sync::Arc;

use rpi_ai::types::{Schema, Tool};
use rpi_extensions::isolated::WorkerToolSpec;
use scriberig_app::{CapabilityContext, ToolExecutionEvidence};
use scriberig_core::EntityId;
use serde_json::Value;

use crate::orchestration::ActivitySequence;
use crate::plugin_worker::AuthorizedPluginWorker;
use crate::tools::{ToolCatalog, ToolCategory, ToolDescriptor, ToolExecutionMode};
use crate::upstream_tool::SemanticAgentTool;

use super::bounded::{bounded_description, is_safe_identifier};
use super::dispatch::{call_worker, plugin_evidence};

/// Reason one worker tool could not be composed into a branch tool set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PluginCompositionError {
    /// The worker process is not attested as OS-isolated.
    SandboxUnavailable(String),
    /// The worker tool identifier is not safe to advertise.
    InvalidToolName(String),
    /// The worker tool schema is not a JSON object argument schema.
    InvalidToolSchema(String),
    /// The merged catalog refused the worker descriptor.
    Registration(String),
    /// The admitted worker could not start under the injected launcher.
    WorkerStart(String),
    /// The plugin capability name is not a safe identifier.
    InvalidCapability(String),
    /// The upstream adapter refused the composed definition.
    Adapter(String),
}

impl std::fmt::Display for PluginCompositionError {
    /// Formats a safe plugin composition error.
    ///
    /// - `self`: Composition failure.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// The formatter error when output cannot be written.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SandboxUnavailable(reason) => {
                write!(formatter, "plugin worker is not OS-isolated: {reason}")
            }
            Self::InvalidToolName(name) => write!(formatter, "plugin tool name is invalid: {name}"),
            Self::InvalidToolSchema(name) => {
                write!(formatter, "plugin tool schema is invalid: {name}")
            }
            Self::Registration(reason) => {
                write!(formatter, "plugin tool registration refused: {reason}")
            }
            Self::WorkerStart(reason) => {
                write!(formatter, "plugin worker start refused: {reason}")
            }
            Self::InvalidCapability(capability) => {
                write!(formatter, "plugin capability name is invalid: {capability}")
            }
            Self::Adapter(reason) => write!(formatter, "plugin tool adapter refused: {reason}"),
        }
    }
}

impl std::error::Error for PluginCompositionError {}

/// Trusted inputs for one branch's plugin tool composition.
///
/// # Public members
/// - `catalog`: Branch tool catalog the plugin descriptors join.
/// - `capabilities`: Immutable branch capability snapshot.
/// - `branch`: Canonical `ScribeRig` branch that owns the adapters.
/// - `activity`: Branch activity sequence that orders durable tool evidence.
/// - `evidence`: Durable evidence sink owned by the branch executor.
///
/// # Private members
/// None.
pub struct PluginToolRequest {
    /// Branch tool catalog the plugin descriptors join.
    pub catalog: ToolCatalog,
    /// Immutable branch capability snapshot.
    pub capabilities: CapabilityContext,
    /// Canonical `ScribeRig` branch that owns the adapters.
    pub branch: EntityId,
    /// Branch activity sequence that orders durable tool evidence.
    pub activity: Arc<ActivitySequence>,
    /// Durable evidence sink owned by the branch executor.
    pub evidence: Arc<dyn Fn(ToolExecutionEvidence) + Send + Sync>,
}

/// Plugin tools composed for one branch from one authorized worker.
///
/// # Public members
/// - `catalog`: Catalog containing both semantic and plugin descriptors.
/// - `plugin_descriptors`: Registered plugin descriptors in stable order.
/// - `tools`: Upstream adapters for every branch-visible plugin tool.
///
/// # Private members
/// None.
pub struct ComposedPluginTools {
    /// Catalog containing both semantic and plugin descriptors.
    catalog: ToolCatalog,
    /// Registered plugin descriptors in deterministic order.
    plugin_descriptors: Vec<ToolDescriptor>,
    /// Upstream adapters for every plugin tool visible to this branch.
    tools: Vec<Arc<SemanticAgentTool>>,
}

impl ComposedPluginTools {
    /// Returns the merged branch catalog.
    ///
    /// # Returns
    /// The catalog that authorizes both semantic and plugin tool calls.
    #[must_use]
    pub fn catalog(&self) -> &ToolCatalog {
        &self.catalog
    }

    /// Returns the plugin descriptors registered for this branch.
    ///
    /// # Returns
    /// Plugin descriptors in deterministic catalog order.
    #[must_use]
    pub fn plugin_descriptors(&self) -> &[ToolDescriptor] {
        &self.plugin_descriptors
    }

    /// Returns the composed upstream tool adapters.
    ///
    /// # Returns
    /// One adapter per plugin tool visible to this branch.
    #[must_use]
    pub fn tools(&self) -> &[Arc<SemanticAgentTool>] {
        &self.tools
    }
}

/// One authorized worker composed against one branch tool boundary.
///
/// # Public members
/// - `new`: Validates the worker's sandbox attestation and plugin capability.
/// - `capability`: Plugin capability required by every composed tool.
/// - `worker`: Authorized worker retained by this composition.
/// - `compose`: Builds the merged catalog and branch tool adapters.
///
/// # Private members
/// - `worker`: Authorized worker whose tools may be composed.
/// - `capability`: Required plugin capability name.
pub struct PluginToolBridge {
    /// Authorized worker whose tools may be composed.
    worker: Arc<AuthorizedPluginWorker>,
    /// Required plugin capability name.
    capability: String,
}

impl PluginToolBridge {
    /// Binds one authorized worker to one plugin capability name.
    ///
    /// Fails closed before any tool is advertised when the running worker is
    /// not attested as OS-isolated or when the capability name is unusable.
    ///
    /// # Parameters
    /// - `worker`: Authorized worker retained by this composition.
    /// - `capability`: Capability the branch must hold to see these tools.
    ///
    /// # Returns
    /// A composition boundary for one branch.
    ///
    /// # Errors
    /// Returns `PluginCompositionError` when isolation is not attested or the
    /// capability name is not safe.
    pub fn new(
        worker: Arc<AuthorizedPluginWorker>,
        capability: &str,
    ) -> Result<Self, PluginCompositionError> {
        let attestation = worker.sandbox_attestation();
        if !attestation.enforces_os_isolation() {
            return Err(PluginCompositionError::SandboxUnavailable(format!(
                "launcher `{}`",
                attestation.platform
            )));
        }
        if !is_safe_identifier(capability) {
            return Err(PluginCompositionError::InvalidCapability(
                capability.to_owned(),
            ));
        }
        Ok(Self {
            worker,
            capability: capability.to_owned(),
        })
    }

    /// Returns the plugin capability required by every composed tool.
    ///
    /// # Returns
    /// The capability name recorded on each plugin descriptor.
    #[must_use]
    pub fn capability(&self) -> &str {
        &self.capability
    }

    /// Returns the authorized worker retained by this composition.
    ///
    /// # Returns
    /// The worker used for every composed tool call.
    #[must_use]
    pub fn worker(&self) -> &Arc<AuthorizedPluginWorker> {
        &self.worker
    }

    /// Registers the worker's authorized tools in one branch catalog.
    ///
    /// A tool the branch capability snapshot does not grant is never adapted, so
    /// an unavailable plugin capability stays out of provider-visible context. A
    /// worker tool that collides with a registered semantic name refuses the
    /// whole composition, and each adapted adapter reauthorizes that descriptor
    /// on every call before the worker is reached.
    ///
    /// # Parameters
    /// - `request`: Trusted catalog, capability snapshot, branch identity, and
    ///   durable evidence route for this branch.
    ///
    /// # Returns
    /// The merged catalog plus one adapter per branch-visible plugin tool.
    ///
    /// # Errors
    /// Returns a composition error when isolation, a tool specification, the
    /// merged catalog, or the upstream adapter refuses the composition.
    pub fn compose(
        &self,
        request: PluginToolRequest,
    ) -> Result<ComposedPluginTools, PluginCompositionError> {
        self.worker
            .sandbox_attestation()
            .enforces_os_isolation()
            .then_some(())
            .ok_or_else(|| {
                PluginCompositionError::SandboxUnavailable("worker process".to_owned())
            })?;
        let specs = self
            .worker
            .advertised_tools()
            .map_err(|error| PluginCompositionError::SandboxUnavailable(error.to_string()))?;
        let descriptors = specs
            .iter()
            .map(|spec| self.descriptor(spec))
            .collect::<Result<Vec<_>, _>>()?;
        let PluginToolRequest {
            catalog,
            capabilities,
            branch,
            activity,
            evidence,
        } = request;
        let catalog = catalog
            .merge_authorized(&descriptors)
            .map_err(|error| PluginCompositionError::Registration(error.to_string()))?;
        let mut tools = Vec::new();
        for descriptor in &descriptors {
            if catalog
                .authorize_for_context(&descriptor.definition.name, &capabilities)
                .is_err()
            {
                continue;
            }
            let adapter = self.adapter(
                descriptor,
                &catalog,
                &capabilities,
                branch,
                &activity,
                &evidence,
            );
            let adapter = adapter.map_err(PluginCompositionError::Adapter)?;
            tools.push(Arc::new(adapter));
        }
        Ok(ComposedPluginTools {
            catalog,
            plugin_descriptors: descriptors,
            tools,
        })
    }

    /// Builds the trusted descriptor for one authorized worker tool.
    ///
    /// # Parameters
    /// - `spec`: Host-filtered tool specification from the worker.
    ///
    /// # Returns
    /// A descriptor that names the plugin capability explicitly.
    ///
    /// # Errors
    /// Returns an error when the identifier or argument schema is unusable.
    fn descriptor(&self, spec: &WorkerToolSpec) -> Result<ToolDescriptor, PluginCompositionError> {
        if !is_safe_identifier(&spec.name) {
            return Err(PluginCompositionError::InvalidToolName(spec.name.clone()));
        }
        if !spec.parameters.is_object()
            || spec.parameters.get("type").and_then(Value::as_str) != Some("object")
        {
            return Err(PluginCompositionError::InvalidToolSchema(spec.name.clone()));
        }
        Ok(ToolDescriptor {
            definition: Tool {
                name: spec.name.clone(),
                description: bounded_description(&spec.description),
                parameters: Schema::new(spec.parameters.clone()),
                constrained_sampling: None,
            },
            category: ToolCategory::OpaqueFallback,
            result_shape: "bounded plugin worker result".to_owned(),
            invocation_label: Some(spec.name.clone()),
            // A plugin tool can reach a semantic operation through the host
            // capability route, so it is never assumed read-only.
            state_changing: true,
            execution_mode: ToolExecutionMode::Atomic,
            required_capability: Some(self.capability.clone()),
        })
    }

    /// Adapts one registered plugin descriptor into an upstream tool.
    ///
    /// # Parameters
    /// - `descriptor`: Registered plugin descriptor.
    /// - `catalog`: Merged catalog used for invocation-time authorization.
    /// - `capabilities`: Immutable branch capability snapshot.
    /// - `branch`: Canonical `ScribeRig` branch that owns the adapter.
    /// - `activity`: Branch activity sequence ordering durable evidence.
    /// - `evidence`: Durable evidence sink for settled calls.
    ///
    /// # Returns
    /// The upstream adapter, or the adapter's own refusal.
    ///
    /// # Errors
    /// Returns the upstream refusal when the definition is not branch-visible.
    fn adapter(
        &self,
        descriptor: &ToolDescriptor,
        catalog: &ToolCatalog,
        capabilities: &CapabilityContext,
        branch: EntityId,
        activity: &Arc<ActivitySequence>,
        evidence: &Arc<dyn Fn(ToolExecutionEvidence) + Send + Sync>,
    ) -> Result<SemanticAgentTool, String> {
        let worker = Arc::clone(&self.worker);
        let name = descriptor.definition.name.clone();
        let sink = Arc::clone(evidence);
        let sequence = Arc::clone(activity);
        SemanticAgentTool::new(
            &descriptor.definition,
            descriptor
                .invocation_label
                .clone()
                .unwrap_or_else(|| descriptor.definition.name.clone()),
            branch,
            catalog.clone(),
            capabilities.clone(),
            move |_tool_call_id, arguments, cancellation, _on_update| {
                let worker = Arc::clone(&worker);
                let name = name.clone();
                let sink = Arc::clone(&sink);
                let sequence = Arc::clone(&sequence);
                async move {
                    let result = call_worker(worker, &name, arguments.clone(), cancellation).await;
                    let ordinal = sequence.next();
                    sink(plugin_evidence(ordinal, &name, &arguments, &result));
                    result
                }
            },
        )
    }
}
