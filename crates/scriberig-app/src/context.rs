//! Defines transport-neutral context passed to application operations.

use crate::capability::{Capability, CapabilityAction, CapabilityId, CapabilityScope};
use crate::operation::OperationKind;
use scriberig_core::SnapshotId;
use std::path::PathBuf;

/// Identifies the principal on whose behalf an operation runs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Principal {
    /// Stable caller identity used for provenance and audit.
    pub id: String,
}

impl Principal {
    /// Creates a principal from a stable identity string.
    ///
    /// # Parameters
    /// - `id`: Stable principal identity.
    ///
    /// # Returns
    /// The principal descriptor.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

/// Carries the capabilities granted to one application request.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CapabilityContext {
    /// Stable capabilities granted by the trusted runtime boundary.
    pub granted: Vec<CapabilityId>,
    /// Scoped capability grants supplied by the trusted runtime boundary.
    pub grants: Vec<Capability>,
}

impl CapabilityContext {
    /// Creates a capability context from granted identifiers.
    ///
    /// # Parameters
    /// - `granted`: Capabilities granted to the caller.
    ///
    /// # Returns
    /// The request capability context.
    #[must_use]
    pub fn new(granted: impl IntoIterator<Item = CapabilityId>) -> Self {
        Self {
            granted: granted.into_iter().collect(),
            grants: Vec::new(),
        }
    }

    /// Creates a capability context from complete scoped grants.
    ///
    /// # Parameters
    /// - `grants`: Explicit capabilities to evaluate for this request.
    ///
    /// # Returns
    /// A context containing the supplied grants.
    #[must_use]
    pub fn with_grants(grants: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            granted: Vec::new(),
            grants: grants.into_iter().collect(),
        }
    }

    /// Checks whether a named capability is granted.
    ///
    /// # Parameters
    /// - `self`: Capability context to inspect.
    /// - `capability`: Capability to find.
    ///
    /// # Returns
    /// `true` when the capability is present.
    #[must_use]
    pub fn allows(&self, capability: &CapabilityId) -> bool {
        self.granted.iter().any(|candidate| candidate == capability)
    }

    /// Reports whether any scoped grant covers one application read.
    ///
    /// # Parameters
    /// - `self`: Capability context to inspect.
    /// - `action`: Action required by the read operation.
    /// - `scope`: Semantic scope exposed by the operation.
    /// - `snapshot`: Exact snapshot bound to the operation.
    /// - `operation`: Universal operation kind, when known.
    ///
    /// # Returns
    /// `true` when a complete scoped grant covers the request.
    #[must_use]
    pub fn permits(
        &self,
        action: &CapabilityAction,
        scope: &CapabilityScope,
        snapshot: Option<SnapshotId>,
        operation: Option<OperationKind>,
    ) -> bool {
        self.grants
            .iter()
            .any(|grant| grant.covers(action, scope, snapshot, operation))
    }
}

/// Carries immutable request context into shared application operations.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApplicationContext {
    /// Optional exact snapshot under which the request executes.
    pub snapshot: Option<SnapshotId>,
    /// Optional caller identity for provenance and audit.
    pub principal: Option<Principal>,
    /// Trusted user identity that owns shared display preferences.
    pub user_preferences_principal: Option<String>,
    /// Capabilities supplied by the trusted runtime boundary.
    pub capabilities: CapabilityContext,
    /// Optional correlation identifier for tracing one request.
    pub correlation_id: Option<String>,
    /// Trusted local bootstrap environment, distinct from semantic project scope.
    pub bootstrap: Option<BootstrapEnvironment>,
}

/// Supplies bounded local environment data for authorized bootstrap operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BootstrapEnvironment {
    /// Working directory selected by the trusted local product composition.
    pub working_directory: PathBuf,
}

impl ApplicationContext {
    /// Creates an empty context for an embedded operation.
    ///
    /// # Returns
    /// A context with no snapshot, principal, or capabilities.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Binds the context to an exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Context to update.
    /// - `snapshot`: Exact snapshot identity.
    ///
    /// # Returns
    /// The updated context.
    #[must_use]
    pub const fn with_snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.snapshot = Some(snapshot);
        self
    }

    /// Attaches trusted local bootstrap context to an application request.
    ///
    /// # Parameters
    /// - `self`: Context to update.
    /// - `bootstrap`: Environment supplied by a trusted local composition root.
    ///
    /// # Returns
    /// The updated context.
    #[must_use]
    pub fn with_bootstrap(mut self, bootstrap: BootstrapEnvironment) -> Self {
        self.bootstrap = Some(bootstrap);
        self
    }

    /// Binds shared preferences independently from the execution actor.
    ///
    /// # Parameters
    /// - `self`: Context to update.
    /// - `principal`: Trusted stable user identity from the composition root.
    ///
    /// # Returns
    /// The updated request context.
    #[must_use]
    pub fn with_user_preferences_principal(mut self, principal: impl Into<String>) -> Self {
        self.user_preferences_principal = Some(principal.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{ApplicationContext, BootstrapEnvironment};
    use std::path::PathBuf;

    /// Confirms bootstrap paths remain separate from semantic project scope.
    #[test]
    fn bootstrap_environment_does_not_create_semantic_scope() {
        let path = PathBuf::from("/trusted/project");
        let context = ApplicationContext::empty().with_bootstrap(BootstrapEnvironment {
            working_directory: path.clone(),
        });

        assert_eq!(context.bootstrap.unwrap().working_directory, path);
        assert_eq!(context.snapshot, None);
    }
}
