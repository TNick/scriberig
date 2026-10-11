//! Cross-layer conformance tests for the universal semantic model.
use scriberig_core::{
    EntityId, EntityRevisionRef, KnowledgeValue, ObjectType, Provenance, ProvenanceSource,
    RevisionId, SnapshotId,
};
use scriberig_graph::{
    CollectionValue, DerivedArtifact, DerivedStatus, Description, RelationValue, RevisionRecord,
};
use scriberig_schema::ValidationSubject;
use std::collections::BTreeMap;

/// Verifies conceptual identity remains stable across exact revisions.
#[test]
fn entity_continuity_and_derived_dependencies() {
    let entity = EntityId::from_bytes([1; 16]);
    let first = RevisionId::new([2; 32]);
    let second = RevisionId::new([3; 32]);
    assert_ne!(first, second);
    let input = EntityRevisionRef {
        entity_id: entity,
        revision_id: first,
    };
    let provenance = Provenance {
        source: ProvenanceSource::User,
        producer: None,
        producer_version: None,
        inputs: vec![input],
        profile: None,
        note: None,
    };
    let description = Description {
        source: input,
        profile: "summary".to_owned(),
        text: "text".to_owned(),
        provenance: provenance.clone(),
        information_cost: None,
    };
    let artifact = DerivedArtifact {
        kind: "summary".to_owned(),
        dependencies: vec![input],
        provenance,
        result: Some(KnowledgeValue::String(description.text)),
        status: DerivedStatus::Complete,
    };
    assert_eq!(artifact.dependencies[0].revision_id, first);
    assert_eq!(description.source.entity_id, entity);
    assert_ne!(SnapshotId::new([4; 32]), SnapshotId::new([5; 32]));
}

/// Verifies collection order and relation targets are semantic identities.
#[test]
fn collection_and_relation_semantics() {
    let p1 = EntityId::from_bytes([1; 16]);
    let p2 = EntityId::from_bytes([2; 16]);
    assert_ne!(
        CollectionValue::Ordered(vec![p1, p2]),
        CollectionValue::Ordered(vec![p2, p1])
    );
    assert_eq!(
        CollectionValue::unordered([p1, p2]),
        CollectionValue::unordered([p2, p1])
    );
    let relation_type = ObjectType::new("related").expect("type");
    let relation = RelationValue::new(relation_type.clone(), vec![p2]);
    assert_eq!(relation.relation_type, relation_type);
}

/// Verifies graph revisions expose schema validation properties without parents.
#[test]
fn revision_record_has_no_parent_requirement() {
    let entity_type = ObjectType::new("paragraph").expect("type");
    let mut properties = BTreeMap::new();
    properties.insert(
        "text".to_owned(),
        KnowledgeValue::String("hello".to_owned()),
    );
    let record = RevisionRecord {
        entity: EntityId::from_bytes([7; 16]),
        entity_type: entity_type.clone(),
        properties,
        relations: BTreeMap::new(),
        collections: BTreeMap::new(),
    };
    assert_eq!(record.entity_type(), &entity_type);
    assert!(record.properties().contains_key("text"));
}
