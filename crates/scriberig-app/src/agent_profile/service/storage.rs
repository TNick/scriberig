//! Encodes and validates profile semantic objects in universal immutable revisions.

use scriberig_core::{EncodingVersion, EntityId, ObjectType, RevisionId, SnapshotId};
use scriberig_store::Revision;

use super::model::AgentProfileError;
use super::{PROFILE_PROMOTE, PROFILE_PROPOSE};
use crate::{AgentProfileDescriptor, ApplicationContext, CapabilityAction, CapabilityScope};

/// Serializes one stable profile field set without embedding its own revision.
#[derive(serde::Deserialize, serde::Serialize)]
pub(super) struct StoredProfile {
    pub(super) identity: String,
    pub(super) name: String,
    pub(super) components: Vec<crate::ContextComponentRevisionRef>,
    pub(super) preferred_execution: Option<crate::AgentProfileExecutionPreference>,
}

impl From<&AgentProfileDescriptor> for StoredProfile {
    /// Converts the expanded descriptor into exact profile metadata.
    ///
    /// # Parameters
    /// - `profile`: Expanded descriptor to serialize.
    ///
    /// # Returns
    /// Stable profile metadata without an embedded revision.
    fn from(profile: &AgentProfileDescriptor) -> Self {
        Self {
            identity: profile.reference.identity.clone(),
            name: profile.name.clone(),
            components: profile
                .components
                .iter()
                .map(|component| component.reference.clone())
                .collect(),
            preferred_execution: profile.preferred_execution.clone(),
        }
    }
}

/// Serializes one context component independently from the profile composition.
#[derive(serde::Deserialize, serde::Serialize)]
pub(super) struct StoredComponent {
    pub(super) identity: String,
    pub(super) kind: crate::ContextComponentKind,
    pub(super) content: String,
    pub(super) activation: crate::ContextActivationRule,
    pub(super) priority: i32,
}

impl From<&crate::ContextComponentDescriptor> for StoredComponent {
    /// Converts a component descriptor into its independent semantic revision value.
    ///
    /// # Parameters
    /// - `component`: Context component to serialize.
    ///
    /// # Returns
    /// Component content and activation metadata.
    fn from(component: &crate::ContextComponentDescriptor) -> Self {
        Self {
            identity: component.reference.identity.clone(),
            kind: component.kind,
            content: component.content.clone(),
            activation: component.activation,
            priority: component.priority,
        }
    }
}

impl StoredComponent {
    /// Reconstructs a descriptor with the exact profile-pinned component reference.
    ///
    /// # Parameters
    /// - `self`: Stored component fields to restore.
    /// - `reference`: Exact immutable component revision reference.
    ///
    /// # Returns
    /// The reconstructed component descriptor.
    pub(super) fn into_descriptor(
        self,
        reference: crate::ContextComponentRevisionRef,
    ) -> crate::ContextComponentDescriptor {
        crate::ContextComponentDescriptor {
            reference,
            kind: self.kind,
            content: self.content,
            activation: self.activation,
            priority: self.priority,
        }
    }
}

/// Validates required profile identity, names, component uniqueness, and content.
///
/// # Parameters
/// - `profile`: Profile descriptor to validate.
///
/// # Returns
/// Success when all profile and component fields satisfy the contract.
///
/// # Errors
/// Returns an invalid-profile error when required values are empty or duplicated.
pub(super) fn validate_profile(profile: &AgentProfileDescriptor) -> Result<(), AgentProfileError> {
    if profile.reference.identity.trim().is_empty() || profile.name.trim().is_empty() {
        return Err(AgentProfileError::Invalid(
            "profile identity and display name must be non-empty".to_owned(),
        ));
    }
    let mut identities = std::collections::BTreeSet::new();
    for component in &profile.components {
        if component.reference.identity.trim().is_empty()
            || component.content.trim().is_empty()
            || !identities.insert(component.reference.identity.as_str())
        {
            return Err(AgentProfileError::Invalid(
                "component identities must be unique and content must be non-empty".to_owned(),
            ));
        }
    }
    if let Some(preference) = &profile.preferred_execution
        && (preference.provider_id.trim().is_empty() || preference.model_id.trim().is_empty())
    {
        return Err(AgentProfileError::Invalid(
            "preferred provider and model identities must be non-empty".to_owned(),
        ));
    }
    Ok(())
}

/// Authorizes one profile action against an exact entity and snapshot.
///
/// # Parameters
/// - `context`: Caller capabilities used for authorization.
/// - `action`: Requested profile action.
/// - `entity`: Exact profile entity being accessed.
/// - `snapshot`: Exact immutable snapshot being accessed.
///
/// # Returns
/// Whether the action is permitted for the entity and snapshot.
pub(super) fn permits(
    context: &ApplicationContext,
    action: &CapabilityAction,
    entity: EntityId,
    snapshot: SnapshotId,
) -> bool {
    let custom = match action {
        CapabilityAction::Propose => PROFILE_PROPOSE,
        CapabilityAction::Promote => PROFILE_PROMOTE,
        CapabilityAction::Inspect => {
            return context.capabilities.permits(
                action,
                &CapabilityScope::Entity(entity),
                Some(snapshot),
                None,
            );
        }
        _ => return false,
    };
    context.capabilities.permits(
        &CapabilityAction::Custom(custom.to_owned()),
        &CapabilityScope::Entity(entity),
        Some(snapshot),
        None,
    )
}

/// Builds a deterministic entity identity from a semantic namespace and stable name.
///
/// # Parameters
/// - `namespace`: Stable entity namespace.
/// - `identity`: Stable domain identity within the namespace.
///
/// # Returns
/// A deterministic entity ID derived from the namespace and identity.
///
/// # Errors
/// Returns a profile error when the identity revision cannot be encoded.
pub(super) fn identity_entity(
    namespace: &str,
    identity: &str,
) -> Result<EntityId, AgentProfileError> {
    let kind = object_type("agent.profile.identity")?;
    let revision = Revision::new(
        EncodingVersion::V1,
        kind,
        [namespace.as_bytes(), b"\0", identity.as_bytes()].concat(),
    )
    .map_err(store_error)?;
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&revision.id().as_bytes()[..16]);
    Ok(EntityId::from_bytes(bytes))
}

/// Derives a snapshot entity identity for one exact component revision.
///
/// # Parameters
/// - `identity`: Stable component identity.
/// - `revision`: Exact component revision ID.
///
/// # Returns
/// An entity ID bound to this component revision.
///
/// # Errors
/// Returns a profile error when the identity revision cannot be encoded.
pub(super) fn component_revision_entity(
    identity: &str,
    revision: RevisionId,
) -> Result<EntityId, AgentProfileError> {
    let kind = object_type("agent.context_component.identity")?;
    let bytes = [identity.as_bytes(), b"\0", revision.as_bytes()].concat();
    let identity_revision = Revision::new(EncodingVersion::V1, kind, bytes).map_err(store_error)?;
    let mut entity_bytes = [0; 16];
    entity_bytes.copy_from_slice(&identity_revision.id().as_bytes()[..16]);
    Ok(EntityId::from_bytes(entity_bytes))
}

/// Constructs a validated universal object type.
///
/// # Parameters
/// - `value`: Object type name to validate.
///
/// # Returns
/// The validated object type.
///
/// # Errors
/// Returns a profile error when the name is invalid.
pub(super) fn object_type(value: &str) -> Result<ObjectType, AgentProfileError> {
    ObjectType::new(value).map_err(|error| AgentProfileError::Invalid(error.to_string()))
}

/// Encodes a profile value as deterministic JSON bytes.
pub(super) fn encode_payload<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, AgentProfileError> {
    serde_json::to_vec(value).map_err(|error| AgentProfileError::Encoding(error.to_string()))
}

/// Decodes a retained profile value from canonical JSON bytes.
pub(super) fn decode_payload<T: for<'de> serde::Deserialize<'de>>(
    payload: &[u8],
) -> Result<T, AgentProfileError> {
    serde_json::from_slice(payload).map_err(|error| AgentProfileError::Encoding(error.to_string()))
}

/// Converts a universal store error into the profile application error.
///
/// # Parameters
/// - `error`: Store error to retain in the application error.
///
/// # Returns
/// A profile store error with diagnostic context.
pub(super) fn store_error(error: impl std::fmt::Debug) -> AgentProfileError {
    AgentProfileError::Store(format!("{error:?}"))
}
