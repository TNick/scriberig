//! Encodes and assembles independently revisioned semantic plan objects.

use scriberig_core::{EntityId, RevisionId, TypeId};

#[path = "semantic/decode.rs"]
mod decode;
#[path = "semantic/encode.rs"]
mod encode;

/// One immutable plan-domain object payload and its content revision identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanSemanticRevision {
    /// Stable entity identity represented by this object.
    pub entity: EntityId,
    /// Stable semantic object type.
    pub object_type: TypeId,
    /// Content-derived immutable revision identity.
    pub revision: RevisionId,
    /// Versioned deterministic JSON payload used to derive `revision`.
    pub payload: Vec<u8>,
}

/// The current semantic objects that assemble one plan projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanSemanticSnapshot {
    /// Stable identity of the plan represented by these objects.
    pub plan: EntityId,
    /// Current plan, node, membership, structure, and dependency objects.
    pub objects: Vec<PlanSemanticRevision>,
}

/// Reports invalid or incomplete semantic plan object payloads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlanSemanticError {
    /// An object payload could not be serialized or decoded.
    Serialization(String),
    /// The object set lacks a required plan, node, or relation object.
    InvalidObject(String),
}

/// Creates and reads current immutable plan object sets.
impl PlanSemanticSnapshot {
    /// Splits one aggregate plan view into independently revisioned objects.
    ///
    /// The returned objects are ready to be written into a universal snapshot.
    /// Historical plan revisions are intentionally not encoded in the current
    /// plan object; the migration layer must retain them as historical
    /// snapshots or migration provenance records.
    ///
    /// # Parameters
    /// - `plan`: Current in-memory plan projection to encode.
    ///
    /// # Returns
    /// A deterministically ordered set of plan-domain immutable objects.
    ///
    /// # Errors
    /// Returns `Serialization` when a typed payload cannot be encoded.
    pub fn from_plan(plan: &super::Plan) -> Result<Self, PlanSemanticError> {
        encode::from_plan(plan)
    }

    /// Assembles a plan projection from independently loaded objects.
    ///
    /// # Parameters
    /// - `self`: Semantic object set to assemble.
    /// - `history`: Historical compatibility records retained by the caller.
    ///
    /// # Returns
    /// A provider-neutral plan projection with stable identities and ordering.
    ///
    /// # Errors
    /// Returns `InvalidObject` for missing, duplicated, mismatched, or
    /// unsupported object payloads and `Serialization` for invalid bytes.
    pub fn into_plan(
        &self,
        history: Vec<super::PlanRevision>,
    ) -> Result<super::Plan, PlanSemanticError> {
        decode::into_plan(self, history)
    }
}
