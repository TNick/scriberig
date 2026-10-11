//! Provides universal identifiers and domain-neutral core values for Scriberig.

// Expose canonical encoding and identity primitives to consumers.
/// Defines deterministic canonical byte encoding primitives.
pub mod encoding;
/// Defines strongly typed universal identifiers and names.
pub mod ids;
/// Defines actor identity independently from authority.
pub mod principal;
/// Defines origin and exact-input provenance records.
pub mod provenance;
/// Defines domain-neutral property values and canonical/derived classification.
pub mod value;

// Re-export the stable identifier API at the crate root.
pub use encoding::{
    CanonicalEncode, CanonicalWriter, EncodingError, EncodingVersion, revision_envelope,
    revision_id,
};
pub use ids::{
    EntityId, IdParseError, ObjectType, OperationId, RevisionId, RootName, SnapshotId, TypeId,
};
pub use principal::{PrincipalId, PrincipalKind};
pub use provenance::{EntityRevisionRef, Provenance, ProvenanceSource};
pub use value::{KnowledgeClass, KnowledgeValue};

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        EntityId, EntityRevisionRef, KnowledgeClass, KnowledgeValue, Provenance, ProvenanceSource,
        RevisionId,
    };

    /// Checks nested values preserve list order and deterministic records.
    #[test]
    fn knowledge_values_preserve_structure() {
        let mut record = BTreeMap::new();
        record.insert("name".to_owned(), KnowledgeValue::String("item".to_owned()));
        let value = KnowledgeValue::List(vec![
            KnowledgeValue::UnsignedInteger(1),
            KnowledgeValue::Record(record.clone()),
        ]);
        assert_eq!(
            value,
            KnowledgeValue::List(vec![
                KnowledgeValue::UnsignedInteger(1),
                KnowledgeValue::Record(record),
            ])
        );
    }

    /// Checks canonical and derived classes remain explicit vocabulary.
    #[test]
    fn knowledge_classes_are_distinct() {
        assert_ne!(KnowledgeClass::Canonical, KnowledgeClass::Derived);
    }

    /// Checks provenance supports empty, multiple, and custom source inputs.
    #[test]
    fn provenance_preserves_exact_inputs_and_custom_sources() {
        let empty = Provenance {
            source: ProvenanceSource::User,
            producer: None,
            producer_version: None,
            inputs: Vec::new(),
            profile: None,
            note: None,
        };
        assert_eq!(empty.inputs.len(), 0);
        let populated = Provenance {
            source: ProvenanceSource::Custom("review-bot".to_owned()),
            producer: Some("bot-v1".to_owned()),
            producer_version: None,
            inputs: vec![
                EntityRevisionRef {
                    entity_id: EntityId::from_bytes([1; 16]),
                    revision_id: RevisionId::new([2; 32]),
                },
                EntityRevisionRef {
                    entity_id: EntityId::from_bytes([3; 16]),
                    revision_id: RevisionId::new([4; 32]),
                },
            ],
            profile: Some("review".to_owned()),
            note: Some("derived from exact revisions".to_owned()),
        };
        assert_eq!(populated.inputs.len(), 2);
        assert!(matches!(populated.source, ProvenanceSource::Custom(_)));
    }

    /// Checks records sort keys independently from insertion order.
    #[test]
    fn records_have_deterministic_key_order() {
        let mut record = BTreeMap::new();
        record.insert("zeta".to_owned(), KnowledgeValue::Null);
        record.insert("alpha".to_owned(), KnowledgeValue::Bool(true));
        let keys: Vec<_> = record.keys().cloned().collect();
        assert_eq!(keys, vec!["alpha".to_owned(), "zeta".to_owned()]);
    }

    /// Checks semantic references stay in provenance instead of property values.
    #[test]
    fn semantic_references_use_typed_provenance_inputs() {
        let reference = EntityRevisionRef {
            entity_id: EntityId::from_bytes([5; 16]),
            revision_id: RevisionId::new([6; 32]),
        };
        let property = KnowledgeValue::Record(BTreeMap::from([(
            "label".to_owned(),
            KnowledgeValue::String("related".to_owned()),
        )]));
        let provenance = Provenance {
            source: ProvenanceSource::System,
            producer: None,
            producer_version: None,
            inputs: vec![reference],
            profile: None,
            note: None,
        };
        assert_eq!(provenance.inputs, vec![reference]);
        assert!(matches!(property, KnowledgeValue::Record(_)));
    }
}
