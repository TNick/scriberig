//! Defines the trusted model-visible tool catalog and call validation.

use std::fmt;

use scriberig_app::{CapabilityAction, CapabilityContext, CapabilityScope};

use crate::provider::{ToolCall, ToolDefinition};

/// Classifies how a trusted tool executes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToolExecutionMode {
    /// Executes one application operation in the current branch.
    Atomic,
    /// Starts a separate reasoning branch.
    ChildBranch,
}

/// Complete trusted descriptor exposed to provider request compilation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDescriptor {
    /// Provider-neutral function definition.
    pub definition: ToolDefinition,
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
    /// # Parameters
    /// - `descriptors`: Trusted application descriptors.
    ///
    /// # Returns
    /// A catalog with deterministic provider exposure order.
    #[must_use]
    pub fn new(mut descriptors: Vec<ToolDescriptor>) -> Self {
        for descriptor in &mut descriptors {
            descriptor.invocation_label = descriptor
                .invocation_label
                .as_deref()
                .map(bounded_invocation_label)
                .filter(|label| !label.is_empty());
        }
        descriptors.sort_by(|left, right| left.definition.name.cmp(&right.definition.name));
        Self { descriptors }
    }

    /// Returns provider definitions in deterministic order.
    ///
    /// # Returns
    /// The definitions suitable for a normalized model request.
    #[must_use]
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.descriptors
            .iter()
            .map(|descriptor| descriptor.definition.clone())
            .collect()
    }

    /// Returns descriptors authorized by one branch capability context.
    ///
    /// # Parameters
    /// - `context`: Capabilities granted to the current branch.
    ///
    /// # Returns
    /// Authorized descriptors in stable catalog order.
    #[must_use]
    pub fn descriptors_for(&self, context: &CapabilityContext) -> Vec<ToolDescriptor> {
        self.descriptors
            .iter()
            .filter(|descriptor| {
                descriptor
                    .required_capability
                    .as_deref()
                    .is_none_or(|required| capability_granted(context, required))
            })
            .cloned()
            .collect()
    }

    /// Validates a provider-selected call against trusted descriptors.
    ///
    /// # Parameters
    /// - `call`: Normalized provider tool call.
    ///
    /// # Returns
    /// The trusted descriptor for the call.
    ///
    /// # Errors
    /// Returns a safe validation error for unknown tools or invalid arguments.
    pub fn validate(&self, call: &ToolCall) -> Result<&ToolDescriptor, ToolValidationError> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.definition.name == call.name)
            .ok_or_else(|| ToolValidationError::UnknownTool(call.name.clone()))?;
        if !call.arguments.is_object() {
            return Err(ToolValidationError::ArgumentsNotObject {
                tool: call.name.clone(),
            });
        }
        if call.name == "inspect"
            && call
                .arguments
                .get("entity")
                .and_then(serde_json::Value::as_str)
                .is_none()
        {
            return Err(ToolValidationError::MissingArgument {
                tool: call.name.clone(),
                argument: "entity".to_owned(),
            });
        }
        if call.name == "delegate"
            && (call
                .arguments
                .get("purpose")
                .and_then(serde_json::Value::as_str)
                .is_none()
                || call
                    .arguments
                    .get("prompt")
                    .and_then(serde_json::Value::as_str)
                    .is_none()
                || call
                    .arguments
                    .get("capabilities")
                    .and_then(serde_json::Value::as_array)
                    .is_none()
                || call
                    .arguments
                    .get("expected_result")
                    .and_then(serde_json::Value::as_str)
                    .is_none())
        {
            return Err(ToolValidationError::MissingArgument {
                tool: call.name.clone(),
                argument: "purpose, prompt, capabilities, and expected_result".to_owned(),
            });
        }
        Ok(descriptor)
    }

    /// Validates a provider call and its required branch capability.
    ///
    /// # Parameters
    /// - `call`: Normalized provider tool call.
    /// - `context`: Capabilities granted to the current branch.
    ///
    /// # Returns
    /// The trusted descriptor when validation and authorization succeed.
    ///
    /// # Errors
    /// Returns a validation or capability-denial error.
    pub fn validate_for_context<'a>(
        &'a self,
        call: &ToolCall,
        context: &CapabilityContext,
    ) -> Result<&'a ToolDescriptor, ToolValidationError> {
        let descriptor = self.validate(call)?;
        if let Some(required) = descriptor.required_capability.as_deref()
            && !capability_granted(context, required)
        {
            return Err(ToolValidationError::CapabilityDenied(required.to_owned()));
        }
        Ok(descriptor)
    }
}

/// Sanitizes and bounds one trusted invocation label for client presentation.
///
/// # Parameters
/// - `label`: Descriptor-provided human-readable label.
///
/// # Returns
/// At most 128 non-control Unicode scalar values.
fn bounded_invocation_label(label: &str) -> String {
    label
        .chars()
        .filter(|character| !character.is_control())
        .take(128)
        .collect()
}

/// Checks both named grants and scoped application grants.
///
/// # Parameters
/// - `context`: Capability context to inspect.
/// - `required`: Stable required capability name.
///
/// # Returns
/// `true` when the capability is granted.
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

/// Safe error returned when a model call fails trusted validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ToolValidationError {
    /// Provider selected an unregistered tool identifier.
    UnknownTool(String),
    /// Provider supplied a non-object argument value.
    ArgumentsNotObject {
        /// Tool being invoked.
        tool: String,
    },
    /// Provider omitted a required argument.
    MissingArgument {
        /// Tool being invoked.
        tool: String,
        /// Required argument name.
        argument: String,
    },
    /// The branch lacks the capability required by a trusted descriptor.
    CapabilityDenied(String),
}

impl fmt::Display for ToolValidationError {
    /// Formats a safe tool validation error.
    ///
    /// # Parameters
    /// - `self`: Validation failure.
    /// - `formatter`: Destination formatter.
    ///
    /// # Returns
    /// Formatting success or the formatter error.
    ///
    /// # Errors
    /// Returns the formatter error when output cannot be written.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTool(tool) => write!(formatter, "unknown trusted tool: {tool}"),
            Self::ArgumentsNotObject { tool } => {
                write!(formatter, "tool {tool} arguments must be an object")
            }
            Self::MissingArgument { tool, argument } => {
                write!(formatter, "tool {tool} requires argument {argument}")
            }
            Self::CapabilityDenied(capability) => {
                write!(formatter, "required capability denied: {capability}")
            }
        }
    }
}

impl std::error::Error for ToolValidationError {}
