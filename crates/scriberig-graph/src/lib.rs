//! Defines domain-neutral semantic graph records and snapshot-bound graph views.

pub mod collection;
pub mod derived;
pub mod description;
pub mod relation;
pub mod revision;
pub mod view;

pub use collection::CollectionValue;
pub use derived::{DerivedArtifact, DerivedStatus};
pub use description::{Description, InformationCost};
pub use relation::RelationValue;
pub use revision::RevisionRecord;
pub use view::{KnowledgeView, ReverseRelation};

#[cfg(test)]
mod tests {
    use super::{CollectionValue, RelationValue};
    use scriberig_core::{EntityId, ObjectType};

    /// Ordered values retain sequence while unordered values ignore insertion order.
    #[test]
    fn collection_semantics_are_distinct() {
        let first = EntityId::from_bytes([1; 16]);
        let second = EntityId::from_bytes([2; 16]);
        assert_ne!(
            CollectionValue::Ordered(vec![first, second]),
            CollectionValue::Ordered(vec![second, first])
        );
        assert_eq!(
            CollectionValue::unordered([first, second]),
            CollectionValue::unordered([second, first])
        );
    }

    /// Relations retain semantic target identities and typed relation identity.
    #[test]
    fn relation_targets_are_entity_ids() {
        let relation_type = ObjectType::new("depends_on").expect("type");
        let relation =
            RelationValue::new(relation_type.clone(), vec![EntityId::from_bytes([7; 16])]);
        assert_eq!(relation.relation_type, relation_type);
        assert_eq!(relation.targets.len(), 1);
    }
}
