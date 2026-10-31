//! Defines runtime capability metadata without implementing authorization policy.

use crate::ApplicationContext;
use crate::operation::OperationKind;
use std::collections::BTreeSet;
#[path = "capability_descriptor.rs"]
mod capability_descriptor;

pub use capability_descriptor::{
    BranchContext, CapabilityDescriptor, CapabilityId, CapabilityKind, DomainOperationRequest,
    RuntimeCapability,
};

use scriberig_core::{EntityId, ObjectType, PrincipalId, RootName, SnapshotId};

/// Stable capability required by the inspect agent tool.
pub const INSPECT_CAPABILITY: &str = "inspect.entity";

/// Reports whether a trusted context exposes the inspect tool surface.
///
/// - `context`: Trusted application context.
///
/// # Returns
/// `true` when the inspect capability is granted.
#[must_use]
pub fn inspect_available(context: &ApplicationContext) -> bool {
    CapabilityId::new(INSPECT_CAPABILITY)
        .is_ok_and(|capability| context.capabilities.allows(&capability))
}

/// Names an action that a capability can authorize.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CapabilityAction {
    /// Inspect semantic state.
    Inspect,
    /// Execute a structural or discovery query.
    Query,
    /// Render through a registered view.
    Render,
    /// Propose a change without applying it.
    Propose,
    /// Modify a branch or workspace.
    ModifyBranch,
    /// Run validation.
    Validate,
    /// Promote a candidate to canonical state.
    Promote,
    /// Invoke an external destination.
    InvokeExternal,
    /// An extensible domain action.
    Custom(String),
}

/// Scope over which a capability applies.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum CapabilityScope {
    /// The project and its semantic descendants.
    Project,
    /// One session.
    Session(String),
    /// One named branch.
    Branch(RootName),
    /// One entity only.
    Entity(EntityId),
    /// One typed relation from an entity.
    Relation {
        /// Entity from which the relation is traversed.
        entity: EntityId,
        /// Relation name.
        relation: String,
    },
    /// One projection for an entity.
    Projection {
        /// Entity being projected.
        entity: EntityId,
        /// Projection name.
        projection: String,
    },
    /// One declared external destination.
    ExternalDestination(String),
    /// An extensible application scope.
    Custom(String),
}

/// Restricts a capability to specific semantic contexts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityConstraints {
    /// Optional exact snapshot binding.
    pub snapshot: Option<SnapshotId>,
    /// Optional branch binding.
    pub branch: Option<RootName>,
    /// Optional domain/type binding.
    pub domain: Option<ObjectType>,
    /// Optional universal operation binding.
    pub operation: Option<OperationKind>,
    /// Optional external destination binding.
    pub external_destination: Option<String>,
    /// Optional lifetime label supplied by the authority issuer.
    pub lifetime: Option<String>,
}

impl CapabilityConstraints {
    /// Creates unconstrained capability context.
    #[must_use]
    pub const fn unrestricted() -> Self {
        Self {
            snapshot: None,
            branch: None,
            domain: None,
            operation: None,
            external_destination: None,
            lifetime: None,
        }
    }
}

/// Represents explicit authority without embedding authentication state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capability {
    /// Scope covered by this capability.
    pub scope: CapabilityScope,
    /// Actions granted within the scope.
    pub actions: BTreeSet<CapabilityAction>,
    /// Additional semantic constraints.
    pub constraints: CapabilityConstraints,
}

impl Capability {
    /// Reports whether this capability covers an action and request context.
    ///
    /// - `self`: Capability being checked.
    /// - `action`: Requested action.
    /// - `scope`: Requested scope.
    /// - `snapshot`: Exact snapshot requested, when applicable.
    /// - `operation`: Universal operation requested, when applicable.
    ///
    /// `true` only when action, scope, and constraints all match.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn covers(
        &self,
        action: &CapabilityAction,
        scope: &CapabilityScope,
        snapshot: Option<SnapshotId>,
        operation: Option<OperationKind>,
    ) -> bool {
        let external_scope = matches!(scope, CapabilityScope::ExternalDestination(_));
        self.actions.contains(action)
            && (!matches!(action, CapabilityAction::InvokeExternal) || external_scope)
            && scope_covers(&self.scope, scope)
            && self
                .constraints
                .snapshot
                .is_none_or(|expected| Some(expected) == snapshot)
            && self
                .constraints
                .branch
                .as_ref()
                .is_none_or(|expected| matches!(scope, CapabilityScope::Branch(actual) if actual == expected))
            && self
                .constraints
                .external_destination
                .as_ref()
                .is_none_or(|expected| {
                    matches!(scope, CapabilityScope::ExternalDestination(actual) if actual == expected)
                })
            && self
                .constraints
                .operation
                .is_none_or(|expected| Some(expected) == operation)
    }

    /// Reports whether a delegated capability is no broader than this parent.
    ///
    /// - `self`: Parent capability.
    /// - `child`: Delegated capability to check.
    ///
    /// `true` when the child grants only a subset of the parent's authority.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn can_delegate(&self, child: &Self) -> bool {
        child.actions.is_subset(&self.actions)
            && scope_covers(&self.scope, &child.scope)
            && constraints_cover(&self.constraints, &child.constraints)
    }
}

/// Describes a denied capability request without exposing restricted content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthorizationDenial {
    /// Principal that requested the action, when known.
    pub principal: Option<PrincipalId>,
    /// Action requested.
    pub action: CapabilityAction,
    /// Scope requested.
    pub scope: CapabilityScope,
    /// Structured reason for denial.
    pub reason: String,
    /// Narrower alternatives, when available.
    pub alternatives: Vec<CapabilityAction>,
    /// Whether an approval process may grant the action.
    pub approval_possible: bool,
}

/// Reports whether a parent scope contains a requested child scope.
///
/// - `parent`: Scope granted by the existing capability.
/// - `child`: Scope requested by the operation or delegation.
///
/// `true` when the parent scope contains the child scope.
/// # Returns
/// The operation result.
fn scope_covers(parent: &CapabilityScope, child: &CapabilityScope) -> bool {
    match (parent, child) {
        (CapabilityScope::Project, CapabilityScope::ExternalDestination(_)) => false,
        (CapabilityScope::Project, _) => true,
        (CapabilityScope::Session(parent), CapabilityScope::Session(child))
        | (
            CapabilityScope::ExternalDestination(parent),
            CapabilityScope::ExternalDestination(child),
        )
        | (CapabilityScope::Custom(parent), CapabilityScope::Custom(child)) => parent == child,
        (CapabilityScope::Branch(parent), CapabilityScope::Branch(child)) => parent == child,
        (CapabilityScope::Entity(parent), CapabilityScope::Entity(child)) => parent == child,
        (
            CapabilityScope::Relation {
                entity: parent_entity,
                relation: parent_relation,
            },
            CapabilityScope::Relation {
                entity: child_entity,
                relation: child_relation,
            },
        ) => parent_entity == child_entity && parent_relation == child_relation,
        (
            CapabilityScope::Projection {
                entity: parent_entity,
                projection: parent_projection,
            },
            CapabilityScope::Projection {
                entity: child_entity,
                projection: child_projection,
            },
        ) => parent_entity == child_entity && parent_projection == child_projection,
        _ => false,
    }
}

/// Reports whether child constraints are no broader than parent constraints.
///
/// - `parent`: Constraints granted by the existing capability.
/// - `child`: Constraints requested by the delegated capability.
///
/// `true` when every child constraint remains within the parent bounds.
/// # Returns
/// The operation result.
fn constraints_cover(parent: &CapabilityConstraints, child: &CapabilityConstraints) -> bool {
    parent
        .snapshot
        .is_none_or(|value| child.snapshot == Some(value))
        && parent
            .branch
            .as_ref()
            .is_none_or(|value| child.branch.as_ref() == Some(value))
        && parent
            .domain
            .as_ref()
            .is_none_or(|value| child.domain.as_ref() == Some(value))
        && parent
            .operation
            .is_none_or(|value| child.operation == Some(value))
        && parent
            .external_destination
            .as_ref()
            .is_none_or(|value| child.external_destination.as_ref() == Some(value))
        && parent
            .lifetime
            .as_ref()
            .is_none_or(|value| child.lifetime.as_ref() == Some(value))
}
