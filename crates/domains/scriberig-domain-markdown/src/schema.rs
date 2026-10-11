//! Builds the deterministic shared schema descriptor for Markdown.

use crate::types::{CONTAINMENT_RELATIONS, ENTITY_TYPES, entity_type};
use scriberig_schema::{
    CapabilityCategory, CapabilityDescriptor, CapabilityId, DomainDescriptor, DomainId,
    ProviderMetadata, SchemaVersion,
};

/// Describes one ordered Markdown containment relation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OrderedRelation {
    /// Stable relation identifier.
    pub id: String,
    /// Parent entity type namespace.
    pub parent: String,
    /// Child entity type namespace.
    pub child: String,
    /// Whether child order is semantically meaningful.
    pub ordered: bool,
}

/// Returns the Markdown domain descriptor and ordered relation metadata.
///
/// # Returns
/// A deterministic shared domain descriptor plus relation details.
///
/// # Panics
/// Panics only if the static Markdown domain identifier is invalid.
#[must_use]
pub fn markdown_schema() -> (DomainDescriptor, Vec<OrderedRelation>) {
    let entity_types = ENTITY_TYPES.iter().map(|name| entity_type(name)).collect();
    let relations = vec![
        relation(
            CONTAINMENT_RELATIONS[0],
            "markdown.document",
            "markdown.section",
        ),
        relation(
            CONTAINMENT_RELATIONS[1],
            "markdown.section",
            "markdown.paragraph",
        ),
        relation(
            CONTAINMENT_RELATIONS[2],
            "markdown.section",
            "markdown.section",
        ),
        relation(
            CONTAINMENT_RELATIONS[3],
            "markdown.list",
            "markdown.list_item",
        ),
        relation(
            CONTAINMENT_RELATIONS[4],
            "markdown.table",
            "markdown.table_row",
        ),
        relation(
            CONTAINMENT_RELATIONS[5],
            "markdown.table_row",
            "markdown.table_cell",
        ),
    ];
    let descriptor = DomainDescriptor {
        id: DomainId::new("markdown").expect("static domain id"),
        name: "Markdown".to_owned(),
        schema_version: SchemaVersion::new(1),
        entity_types,
        relation_types: CONTAINMENT_RELATIONS
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
        capabilities: vec![CapabilityDescriptor {
            id: CapabilityId::new("projection.markdown").expect("static capability"),
            entity_type: entity_type("document"),
            category: CapabilityCategory::Projection,
            description: "Parse and render Markdown projections".to_owned(),
            provider: Some(
                ProviderMetadata::new("scriberig.markdown", "1", true)
                    .with_profile("commonmark-tables"),
            ),
        }],
    };
    (descriptor, relations)
}

/// Creates one ordered relation descriptor.
///
/// # Parameters
/// - `id`: Stable relation identifier.
/// - `parent`: Parent entity type.
/// - `child`: Child entity type.
///
/// # Returns
/// An ordered relation descriptor.
fn relation(id: &str, parent: &str, child: &str) -> OrderedRelation {
    OrderedRelation {
        id: id.to_owned(),
        parent: parent.to_owned(),
        child: child.to_owned(),
        ordered: true,
    }
}

#[cfg(test)]
mod tests {
    use super::markdown_schema;
    use crate::classification::{InformationClass, classify};

    /// Checks the complete Markdown entity vocabulary is registered.
    #[test]
    fn schema_contains_required_entities_and_ordered_relations() {
        let (descriptor, relations) = markdown_schema();
        assert_eq!(descriptor.entity_types.len(), 14);
        assert_eq!(descriptor.relation_types.len(), 6);
        assert_eq!(relations.len(), 6);
        assert!(relations.iter().all(|relation| relation.ordered));
    }

    /// Checks canonical ownership classes distinguish semantic and formatting data.
    #[test]
    fn classification_keeps_projection_and_derived_data_out_of_canonical_state() {
        assert_eq!(classify("text"), Some(InformationClass::Canonical));
        assert_eq!(
            classify("table_padding"),
            Some(InformationClass::ProjectionOnly)
        );
        assert_eq!(classify("summary"), Some(InformationClass::Derived));
    }

    /// Checks schema construction is deterministic.
    #[test]
    fn schema_registration_is_deterministic() {
        assert_eq!(markdown_schema(), markdown_schema());
    }
}
