//! Defines the trusted model-visible tool catalog and authorization policy.

use std::fmt;

use rpi_ai::types::Tool;
use scriberig_app::{CapabilityAction, CapabilityContext, CapabilityScope};

/// Classifies how a trusted tool executes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolExecutionMode {
    /// Executes one application operation in the current branch.
    Atomic,
    /// Starts a separate reasoning branch.
    ChildBranch,
}

/// Shared durable semantic category for one trusted tool.
pub type ToolCategory = scriberig_session::ToolActivityKind;

/// Complete trusted descriptor exposed to provider request compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    /// Provider-neutral function definition.
    pub definition: Tool,
    /// Trusted provider-neutral semantic category used for exposure ordering.
    pub category: ToolCategory,
    /// Bounded result-shape description.
    pub result_shape: String,
    /// Optional trusted command-like label for compact invocation display.
    pub invocation_label: Option<String>,
    /// Whether the operation can change canonical state.
    pub state_changing: bool,
    /// Execution mode selected by the application runtime.
    pub execution_mode: ToolExecutionMode,
    /// Capability required before the tool may be exposed or invoked.
    pub required_capability: Option<String>,
}

/// Trusted descriptors available to one branch execution.
#[derive(Clone, Debug, Default)]
pub struct ToolCatalog {
    descriptors: Vec<ToolDescriptor>,
}

impl ToolCatalog {
    /// Creates a deterministic catalog sorted by stable tool identifier.
    ///
    /// - `descriptors`: Trusted application descriptors.
    ///
    /// A catalog with deterministic provider exposure order.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(mut descriptors: Vec<ToolDescriptor>) -> Self {
        for descriptor in &mut descriptors {
            descriptor.invocation_label = descriptor
                .invocation_label
                .as_deref()
                .map(bounded_invocation_label)
                .filter(|label| !label.is_empty());
        }
        descriptors.sort_by(|left, right| {
            left.category
                .cmp(&right.category)
                .then_with(|| left.definition.name.cmp(&right.definition.name))
        });
        Self { descriptors }
    }

    /// Returns provider definitions in deterministic order.
    ///
    /// Definitions suitable for an upstream provider context.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn definitions(&self) -> Vec<Tool> {
        self.descriptors
            .iter()
            .map(|descriptor| descriptor.definition.clone())
            .collect()
    }

    /// Returns descriptors authorized by one branch capability context.
    ///
    /// - `context`: Capabilities granted to the current branch.
    ///
    /// Authorized descriptors in stable catalog order.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn descriptors_for(&self, context: &CapabilityContext) -> Vec<ToolDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| descriptor_visible(descriptor, context))
            .cloned()
            .collect()
    }

    /// Returns a catalog that also holds host-authorized tool additions.
    ///
    /// A base catalog never mutates in place, so an unauthorized caller cannot
    /// widen a branch's tool set through a shared handle. Every addition must
    /// name an explicit required capability: the branch capability snapshot is
    /// what decides visibility, and an addition without one would inherit the
    /// unconditional visibility of a registered semantic tool. A descriptor may
    /// be added only when the caller can reauthorize that exact name at
    /// invocation time.
    ///
    /// # Parameters
    /// - `additions`: Trusted descriptors contributed by a host-owned executor.
    ///
    /// # Returns
    /// A new catalog holding the base and added descriptors in stable order.
    ///
    /// # Errors
    /// Returns an error when an addition lacks an explicit required capability
    /// or when a name is already registered or repeated.
    pub fn merge_authorized(
        &self,
        additions: &[ToolDescriptor],
    ) -> Result<Self, ToolRegistrationError> {
        for (index, addition) in additions.iter().enumerate() {
            let name = addition.definition.name.clone();
            if addition.required_capability.is_none() {
                return Err(ToolRegistrationError::MissingCapability(name));
            }
            if self
                .descriptors
                .iter()
                .any(|registered| registered.definition.name == addition.definition.name)
                || additions[..index]
                    .iter()
                    .any(|earlier| earlier.definition.name == addition.definition.name)
            {
                return Err(ToolRegistrationError::DuplicateTool(name));
            }
        }
        let mut descriptors = self.descriptors.clone();
        descriptors.extend(additions.iter().cloned());
        Ok(Self::new(descriptors))
    }

    /// Resolves a provider-selected tool name against trusted descriptors.
    ///
    /// JSON Schema argument validation belongs to `rpi-agent`; this method
    /// enforces only `ScribeRig`'s trusted tool registry.
    ///
    /// - `tool_name`: Provider-selected tool name.
    ///
    /// The trusted descriptor for the selected tool.
    /// # Returns
    /// The matching descriptor.
    /// # Errors
    /// Returns an error when the name is not registered.
    pub fn authorize(&self, tool_name: &str) -> Result<&ToolDescriptor, ToolAuthorizationError> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.definition.name == tool_name)
            .ok_or_else(|| ToolAuthorizationError::UnknownTool(tool_name.to_owned()))?;
        Ok(descriptor)
    }

    /// Authorizes a provider-selected tool under branch capabilities.
    ///
    /// JSON Schema argument validation belongs to `rpi-agent`; this method
    /// enforces the trusted registry and `ScribeRig` branch capability policy.
    ///
    /// - `tool_name`: Provider-selected tool name.
    /// - `context`: Capabilities granted to the current branch.
    ///
    /// The trusted descriptor when authorization succeeds.
    ///
    /// # Returns
    /// The descriptor authorized for this branch.
    ///
    /// # Errors
    /// Returns an authorization or capability-denial error.
    pub fn authorize_for_context<'a>(
        &'a self,
        tool_name: &str,
        context: &CapabilityContext,
    ) -> Result<&'a ToolDescriptor, ToolAuthorizationError> {
        let descriptor = self.authorize(tool_name)?;
        if !descriptor_visible(descriptor, context) {
            return Err(ToolAuthorizationError::CapabilityDenied(
                descriptor
                    .required_capability
                    .clone()
                    .unwrap_or_else(|| "opaque_fallback".to_owned()),
            ));
        }
        Ok(descriptor)
    }
}

/// Applies the same branch-policy predicate to exposure and invocation.
///
/// Opaque fallback tools stay hidden unless they carry an explicit capability
/// requirement that this branch satisfies.
///
/// # Parameters
/// - `descriptor`: Trusted tool descriptor being checked.
/// - `context`: Capabilities granted to the current branch.
///
/// # Returns
/// `true` when the descriptor is permitted for this branch.
fn descriptor_visible(descriptor: &ToolDescriptor, context: &CapabilityContext) -> bool {
    match descriptor.required_capability.as_deref() {
        Some(required) => capability_granted(context, required),
        None => descriptor.category != ToolCategory::OpaqueFallback,
    }
}

/// Sanitizes and bounds one trusted invocation label for client presentation.
///
/// - `label`: Descriptor-provided human-readable label.
///
/// At most 128 non-control Unicode scalar values.
/// # Returns
/// The operation result.
fn bounded_invocation_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| !character.is_control())
        .take(128)
        .collect()
}

/// Checks both named grants and scoped application grants.
///
/// - `context`: Capability context to inspect.
/// - `required`: Stable required capability name.
///
/// `true` when the capability is granted.
/// # Returns
/// The operation result.
fn capability_granted(context: &CapabilityContext, required: &str) -> bool {
    if context
        .granted
        .iter()
        .any(|capability| capability.as_str() == required)
    {
        return true;
    }
    let action = match required {
        "inspect" | "inspect.entity" => CapabilityAction::Inspect,
        name => CapabilityAction::Custom(name.to_owned()),
    };
    context.permits(&action, &CapabilityScope::Project, None, None)
}

/// Safe error returned when a tool call fails `ScribeRig` authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolAuthorizationError {
    /// Provider selected an unregistered tool identifier.
    UnknownTool(String),
    /// The branch lacks the capability required by a trusted descriptor.
    CapabilityDenied(String),
}

/// Safe error returned when host-authorized additions cannot join a catalog.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolRegistrationError {
    /// An addition repeats a tool name already fixed by the catalog.
    DuplicateTool(String),
    /// An addition names no explicit required capability.
    MissingCapability(String),
}

impl fmt::Display for ToolRegistrationError {
    /// Formats a safe tool registration error.
    ///
    /// - `self`: Registration failure.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// The formatter error when output cannot be written.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTool(tool) => {
                write!(formatter, "tool name is already registered: {tool}")
            }
            Self::MissingCapability(tool) => {
                write!(
                    formatter,
                    "tool addition names no required capability: {tool}"
                )
            }
        }
    }
}

impl std::error::Error for ToolRegistrationError {}

impl fmt::Display for ToolAuthorizationError {
    /// Formats a safe tool authorization error.
    ///
    /// - `self`: Authorization failure.
    /// - `formatter`: Destination formatter.
    ///
    /// Formatting success or the formatter error.
    ///
    /// Returns the formatter error when output cannot be written.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(tool) => write!(formatter, "unknown trusted tool: {tool}"),
            Self::CapabilityDenied(capability) => {
                write!(formatter, "required capability denied: {capability}")
            }
        }
    }
}

impl std::error::Error for ToolAuthorizationError {}
