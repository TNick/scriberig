//! Defines explicit semantic code-group metadata.
use scriberig_core::{EntityId, Provenance};
/// Groups ordered statements for progressive code navigation.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeGroup {
    /// Group entity.
    pub entity: EntityId,
    /// Ordered statement entities.
    pub statements: Vec<EntityId>,
    /// Optional leading comment.
    pub comment: Option<EntityId>,
    /// Optional derived description.
    pub description: Option<String>,
    /// Provenance for group metadata.
    pub provenance: Option<Provenance>,
}
