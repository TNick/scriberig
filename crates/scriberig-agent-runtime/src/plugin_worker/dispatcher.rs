//! Applies `ScribeRig` semantic authorization to isolated plugin host calls.

use std::sync::Arc;

use rpi_extensions::isolated::WorkerHostHandler;
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationRequest, ApplicationResult,
    ApplicationService, BranchContext, CapabilityAction, CapabilityId, CapabilityScope,
    DomainOperationRequest, OperationKind,
};
use scriberig_core::{EntityId, RootName};
use scriberig_query::{QueryValue, Selection};
use serde_json::Value;

use super::PluginComponentDispatcher;

const MAX_DOMAIN_ARGUMENT_BYTES: usize = 65_536;
const MAX_CHANGE_HANDLE_BYTES: usize = 16_384;

/// Host-authorized semantic operation dispatcher used by plugin workers.
///
/// Runtime-action identifiers and other upstream host capabilities do not
/// imply `ScribeRig` authority. Implementations must map each admitted name to an
/// existing application operation and deny every unmapped name.
pub trait PluginSemanticDispatcher: Send + Sync {
    /// Authorizes and executes one semantic operation through host tools.
    ///
    /// # Parameters
    /// - `operation`: Stable semantic operation identifier.
    /// - `arguments`: Operation arguments.
    ///
    /// # Returns
    /// A bounded JSON result.
    ///
    /// # Errors
    /// Returns a safe denial or execution failure.
    fn dispatch(&self, operation: &str, arguments: &Value) -> Result<Value, String>;
}

/// Executes plugin requests through one trusted `ScribeRig` application context.
///
/// Only registered domain operations are admitted. The dispatcher derives the
/// base snapshot, branch, and grants from host context; plugins cannot supply
/// those values. The backend remains responsible for candidate construction
/// and returns only a change handle. Validation, application, and promotion
/// require separate host operations and are denied here.
///
/// # Public members
/// None.
///
/// # Private members
/// - `service`: Application service owning semantic operation execution.
/// - `context`: Immutable host capability context for this branch.
/// - `branch`: Trusted branch context for candidate creation.
/// - `components`: Optional upstream component mailbox adapter.
pub struct ApplicationDomainDispatcher<B> {
    /// Application service that routes through `ScribeRig` semantic operations.
    service: Arc<ApplicationService<B>>,
    /// Immutable trusted context and grants for the owning branch.
    context: ApplicationContext,
    /// Host-selected branch context for candidate creation.
    branch: Option<RootName>,
    /// Optional upstream component mailbox adapter.
    components: Option<PluginComponentDispatcher>,
}

impl<B> ApplicationDomainDispatcher<B> {
    /// Creates a dispatcher bound to a trusted branch context.
    ///
    /// # Parameters
    /// - `service`: Application service that owns the semantic backend.
    /// - `context`: Host-authored snapshot and capability grants.
    /// - `branch`: Host-authored candidate branch, when applicable.
    ///
    /// # Returns
    /// A dispatcher that admits only authorized domain-operation requests.
    #[must_use]
    pub fn new(
        service: Arc<ApplicationService<B>>,
        context: ApplicationContext,
        branch: Option<RootName>,
    ) -> Self {
        Self {
            service,
            context,
            branch,
            components: None,
        }
    }

    /// Routes isolated plugin component updates to the upstream TUI mailbox.
    ///
    /// # Parameters
    /// - `mailbox`: Session-scoped upstream component registry.
    ///
    /// # Returns
    /// The dispatcher configured to accept `ui_component` host calls.
    #[must_use]
    pub fn with_extension_components(
        mut self,
        mailbox: rpi_extensions::ExtensionStatusMailbox,
    ) -> Self {
        self.components = Some(PluginComponentDispatcher::new(mailbox));
        self
    }
}

impl<B> PluginSemanticDispatcher for ApplicationDomainDispatcher<B>
where
    B: ApplicationBackend + Send + Sync,
{
    /// Authorizes and executes one exact-target `ScribeRig` domain operation.
    ///
    /// `arguments` must contain only `subject` and typed `arguments`. The exact
    /// snapshot, branch, and authority come from the trusted host context.
    ///
    /// # Parameters
    /// - `operation`: Must be `domain_operation`.
    /// - `arguments`: Exact semantic subject and typed domain arguments.
    ///
    /// # Returns
    /// JSON containing the resulting candidate change handle.
    ///
    /// # Errors
    /// Returns a safe denial or semantic-operation failure.
    fn dispatch(&self, operation: &str, arguments: &Value) -> Result<Value, String> {
        if operation == "ui_component" {
            return self.dispatch_ui_component(arguments);
        }
        if operation != "domain_operation" {
            return Err("plugin host operation is not authorized".to_owned());
        }

        let snapshot = self
            .context
            .snapshot
            .ok_or_else(|| "plugin semantic context has no exact snapshot".to_owned())?;
        let encoded_arguments = serde_json::to_vec(arguments)
            .map_err(|_| "plugin semantic arguments are invalid".to_owned())?;
        if encoded_arguments.len() > MAX_DOMAIN_ARGUMENT_BYTES {
            return Err("plugin semantic arguments exceed the size limit".to_owned());
        }
        let request: PluginDomainOperationArguments = serde_json::from_slice(&encoded_arguments)
            .map_err(|_| "plugin semantic arguments are invalid".to_owned())?;
        if request.subject.snapshot != snapshot {
            return Err("plugin semantic subject is outside its snapshot".to_owned());
        }

        let capability = CapabilityId::new(request.capability)
            .map_err(|_| "plugin semantic capability is invalid".to_owned())?;
        if !self.authorized(&capability, request.subject.entity, snapshot) {
            return Err("plugin semantic capability is not authorized".to_owned());
        }

        let request = DomainOperationRequest {
            capability,
            subject: request.subject,
            context: BranchContext {
                base_snapshot: snapshot,
                branch: self.branch.clone(),
            },
            arguments: request.arguments,
        };
        let result = self
            .service
            .execute(&self.context, &ApplicationRequest::DomainOperation(request))
            .map_err(|_| "ScribeRig semantic operation failed".to_owned())?;
        let ApplicationResult::DomainOperation(result) = result else {
            return Err("ScribeRig returned an unexpected semantic result".to_owned());
        };
        if result.change.len() > MAX_CHANGE_HANDLE_BYTES {
            return Err("ScribeRig change handle exceeds the size limit".to_owned());
        }
        Ok(serde_json::json!({ "change": result.change }))
    }
}

impl<B> ApplicationDomainDispatcher<B> {
    /// Applies one worker UI update to its host-scoped mailbox owner.
    ///
    /// # Parameters
    /// - `arguments`: Worker-local owner and component operation payload.
    ///
    /// # Returns
    /// The upstream mailbox acknowledgement.
    ///
    /// # Errors
    /// Returns a denial for malformed input, missing mailbox, or invalid update.
    fn dispatch_ui_component(&self, arguments: &Value) -> Result<Value, String> {
        self.components
            .as_ref()
            .ok_or_else(|| "plugin UI components are unavailable in this host".to_owned())?
            .dispatch(arguments)
    }
}

impl<B> ApplicationDomainDispatcher<B> {
    /// Applies `ScribeRig`'s named and exact scoped domain-operation grants.
    ///
    /// # Parameters
    /// - `capability`: Registered domain capability requested by the plugin.
    /// - `entity`: Exact semantic entity selected by the request.
    /// - `snapshot`: Exact trusted base snapshot.
    ///
    /// # Returns
    /// `true` when the host context permits this operation on the entity.
    fn authorized(
        &self,
        capability: &CapabilityId,
        entity: EntityId,
        snapshot: scriberig_core::SnapshotId,
    ) -> bool {
        let named = self.context.capabilities.allows(capability)
            || self
                .context
                .capabilities
                .granted
                .iter()
                .any(|granted| granted.as_str() == "domain_operation");
        let action = CapabilityAction::Custom(capability.to_string());
        let generic_action = CapabilityAction::Custom("domain_operation".to_owned());
        let scope = CapabilityScope::Entity(entity);
        let exact = self.context.capabilities.permits(
            &action,
            &scope,
            Some(snapshot),
            Some(OperationKind::DomainOperation),
        );
        let generic = self.context.capabilities.permits(
            &generic_action,
            &scope,
            Some(snapshot),
            Some(OperationKind::DomainOperation),
        );
        named || exact || generic
    }
}

/// Wire arguments accepted for the one supported plugin semantic operation.
///
/// # Public members
/// None.
///
/// # Private members
/// - `capability`: Registered `ScribeRig` domain operation identifier.
/// - `subject`: Exact selected semantic entity and snapshot.
/// - `arguments`: Typed domain operation values.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct PluginDomainOperationArguments {
    /// Registered semantic capability identifier.
    capability: String,
    /// Exact semantic subject supplied by the plugin.
    subject: Selection,
    /// Typed domain operation arguments.
    arguments: Vec<(String, QueryValue)>,
}

/// Host dispatch policy for operations admitted by upstream's worker allowlist.
///
/// # Public members
/// None.
///
/// # Private members
/// - `dispatcher`: Host-owned operation route.
pub struct PluginHostPolicy {
    /// Host-owned dispatcher; it must retain the `ScribeRig` operation path.
    dispatcher: Arc<dyn PluginSemanticDispatcher>,
}

impl PluginHostPolicy {
    /// Creates an upstream host handler backed by `ScribeRig` semantic policy.
    ///
    /// # Parameters
    /// - `dispatcher`: Host-owned `ScribeRig` operation route.
    ///
    /// # Returns
    /// A policy handler that delegates only after upstream allowlist checks.
    pub fn new(dispatcher: Arc<dyn PluginSemanticDispatcher>) -> Self {
        Self { dispatcher }
    }
}

impl WorkerHostHandler for PluginHostPolicy {
    /// Dispatches an operation after upstream allowlist authorization.
    ///
    /// # Parameters
    /// - `capability`: Worker-requested operation identifier.
    /// - `arguments`: Operation arguments.
    ///
    /// # Returns
    /// The host dispatcher result.
    ///
    /// # Errors
    /// Returns a safe denial or execution failure from the host dispatcher.
    fn call(&self, capability: &str, arguments: &Value) -> Result<Value, String> {
        self.dispatcher.dispatch(capability, arguments)
    }
}
