//! Proves the generic resolved-style, purpose, fidelity, schema, and provenance
//! contracts stay domain-neutral.

use scriberig_core::{EntityId, RevisionId, SnapshotId};
use sr_projection::{
    FidelityAssessment, OwnershipClass, ProjectionClass, ProjectionFidelity, ProjectionKind,
    ProjectionPurpose, ProjectionSpec, ProjectionSpecRef, RenderCacheKey, RenderRequest,
    RenderedProjection, RendererVersion, RenderingContext, ResolvedStyle, ResolvedStyleProvenance,
    SemanticScopeRef, SemanticStateRef, StyleApplicability, StylePropertyDescriptor, StyleRef,
    StyleResolutionError, StyleSchema, StyleSchemaError, StyleSchemaRef, StyleSource,
    StyleSourceKind, StyleValue, StyleValueKind,
};

/// Builds one projection kind for these generic contract tests.
///
/// - `name`: Kind label used by the test.
///
/// # Returns
/// The validated projection kind.
fn kind(name: &str) -> ProjectionKind {
    ProjectionKind::new(name).expect("test kind")
}

/// Builds one exact render request consuming a resolved style.
///
/// - `style`: Exact resolved style the renderer must consume.
///
/// # Returns
/// The render request.
fn request_with(style: ResolvedStyle) -> RenderRequest {
    RenderRequest {
        specification: ProjectionSpecRef(EntityId::from_bytes([1; 16])),
        scope: SemanticScopeRef {
            entity: EntityId::from_bytes([2; 16]),
            revision: None,
        },
        base_state: SemanticStateRef(SnapshotId::new([3; 32])),
        kind: kind("test.generic"),
        style,
        renderer: RendererVersion::new("1"),
        context: RenderingContext::default(),
    }
}

/// Builds one neutral integer style descriptor for schema tests.
///
/// - `key`: Property key under test.
/// - `minimum`: Inclusive lower bound.
/// - `maximum`: Inclusive upper bound.
///
/// # Returns
/// The descriptor.
fn integer_descriptor(key: &str, minimum: i64, maximum: i64) -> StylePropertyDescriptor {
    StylePropertyDescriptor {
        key: key.to_owned(),
        label: "Beta".to_owned(),
        description: None,
        kind: StyleValueKind::Integer,
        allowed_values: Vec::new(),
        minimum: Some(minimum),
        maximum: Some(maximum),
        default: b"4".to_vec(),
        applicability: StyleApplicability::MaterializationOnly,
        affects_fidelity: false,
        required_for_materialization: true,
    }
}

/// Verifies a revision-backed resolution keeps reference, value, and schema.
#[test]
fn resolved_style_retains_reference_value_and_schema() {
    let entity = EntityId::from_bytes([10; 16]);
    let revision = RevisionId::new([11; 32]);
    let style = ResolvedStyle::new(
        StyleRef::Revision { entity, revision },
        StyleValue::new(b"canonical-a".to_vec()),
        Some(StyleSchemaRef::new("test.generic", "1").expect("test schema reference")),
    )
    .expect("revision resolution");

    assert_eq!(style.reference(), &StyleRef::Revision { entity, revision });
    assert_eq!(style.value().as_bytes(), b"canonical-a");
    assert_eq!(style.schema().map(StyleSchemaRef::version), Some("1"));
}

/// Verifies the exact canonical value, not just the reference, drives identity.
#[test]
fn render_identity_follows_the_exact_resolved_value() {
    let entity = EntityId::from_bytes([12; 16]);
    let revision = RevisionId::new([13; 32]);
    let reference = StyleRef::Revision { entity, revision };
    let first =
        ResolvedStyle::new(reference.clone(), StyleValue::new(b"a".to_vec()), None).expect("first");
    let second = ResolvedStyle::new(reference.clone(), StyleValue::new(b"b".to_vec()), None)
        .expect("second");
    let repeat =
        ResolvedStyle::new(reference, StyleValue::new(b"a".to_vec()), None).expect("repeat");

    let first_key = RenderCacheKey::from_request(&request_with(first));
    assert_ne!(
        first_key,
        RenderCacheKey::from_request(&request_with(second))
    );
    assert_eq!(
        first_key,
        RenderCacheKey::from_request(&request_with(repeat)),
        "identical canonical style bytes resolve to identical identity"
    );
}

/// Verifies the resolved style an adapter echoes back is the exact one consumed.
#[test]
fn rendered_output_reports_the_resolved_style() {
    let style = ResolvedStyle::inline(StyleValue::new(b"inline-style".to_vec()));
    let request = request_with(style.clone());
    let output = RenderedProjection::from_bytes(b"out".to_vec(), &request, None);

    assert_eq!(output.style, style);
    assert_eq!(output.style.value_digest(), style.value_digest());
}

/// Verifies a bare revision reference cannot be handed to a renderer valueless.
#[test]
fn bare_revision_reference_is_refused() {
    let reference = StyleRef::Revision {
        entity: EntityId::from_bytes([14; 16]),
        revision: RevisionId::new([15; 32]),
    };

    assert_eq!(
        ResolvedStyle::from_reference(&reference),
        Err(StyleResolutionError::RevisionValueRequired)
    );
}

/// Verifies an inline reference whose bytes disagree with the value is refused.
#[test]
fn inconsistent_inline_pair_is_refused() {
    assert_eq!(
        ResolvedStyle::new(
            StyleRef::Value(StyleValue::new(b"declared".to_vec())),
            StyleValue::new(b"different".to_vec()),
            None,
        ),
        Err(StyleResolutionError::ReferenceValueMismatch)
    );
}

/// Verifies the three fidelity states stay distinct and drive editability.
#[test]
fn fidelity_states_remain_distinguishable() {
    assert!(ProjectionFidelity::LosslessEditable.is_editable());
    assert!(ProjectionFidelity::LossAwareEditable.is_editable());
    assert!(!ProjectionFidelity::ReadOnlyLossy.is_editable());
    assert!(ProjectionFidelity::LossAwareEditable.requires_loss_awareness());
    assert!(!ProjectionFidelity::LosslessEditable.requires_loss_awareness());
    assert_ne!(
        FidelityAssessment::lossless(),
        FidelityAssessment::loss_aware(vec!["category-b".to_owned()], None)
    );
    assert_eq!(
        FidelityAssessment::conservative("unclassified").fidelity,
        ProjectionFidelity::ReadOnlyLossy
    );
}

/// Verifies purpose separates personal from shared and explicit use.
#[test]
fn purpose_separates_personal_and_shared_resolution() {
    assert!(ProjectionPurpose::InteractiveView.admits_personal_preference());
    assert!(!ProjectionPurpose::SharedMaterialization.admits_personal_preference());
    assert!(ProjectionPurpose::ExplicitExport.admits_personal_preference());
    assert!(!ProjectionPurpose::InteractiveView.requires_explicit_personal_source());
    assert!(ProjectionPurpose::ExplicitExport.requires_explicit_personal_source());
    assert_ne!(
        ProjectionPurpose::InteractiveView.to_string(),
        ProjectionPurpose::SharedMaterialization.to_string()
    );
}

/// Verifies a style schema can drive structured client presentation.
#[test]
fn style_schema_describes_properties_without_owning_domain_meaning() {
    let schema = StyleSchema::new(
        "test.generic",
        "1",
        vec![kind("test.generic")],
        vec![
            StylePropertyDescriptor {
                key: "option-alpha".to_owned(),
                label: "Alpha".to_owned(),
                description: Some("Adapter-owned option".to_owned()),
                kind: StyleValueKind::Enum,
                allowed_values: vec!["first".to_owned(), "second".to_owned()],
                minimum: None,
                maximum: None,
                default: b"first".to_vec(),
                applicability: StyleApplicability::Both,
                affects_fidelity: true,
                required_for_materialization: false,
            },
            integer_descriptor("option-beta", 1, 8),
        ],
    )
    .expect("valid schema");

    assert!(schema.covers(&kind("test.generic")));
    assert!(!schema.covers(&kind("other.kind")));
    assert_eq!(
        schema.property("option-alpha").map(|item| item.kind),
        Some(StyleValueKind::Enum)
    );
    assert!(schema.property("missing-option").is_none());
    assert_eq!(schema.properties.len(), 2);
}

/// Verifies inconsistent schema descriptors are rejected structurally.
#[test]
fn schema_validation_reports_structured_defects() {
    let empty_key = StyleSchema::new(
        "test.generic",
        "1",
        Vec::new(),
        vec![StylePropertyDescriptor {
            key: " ".to_owned(),
            label: "Label".to_owned(),
            description: None,
            kind: StyleValueKind::Text,
            allowed_values: Vec::new(),
            minimum: None,
            maximum: None,
            default: Vec::new(),
            applicability: StyleApplicability::Both,
            affects_fidelity: false,
            required_for_materialization: false,
        }],
    )
    .expect_err("empty key");
    assert_eq!(empty_key, StyleSchemaError::EmptyPropertyKey);

    let empty_enum = StyleSchema::new(
        "test.generic",
        "1",
        Vec::new(),
        vec![StylePropertyDescriptor {
            key: "option-gamma".to_owned(),
            label: "Gamma".to_owned(),
            description: None,
            kind: StyleValueKind::Enum,
            allowed_values: Vec::new(),
            minimum: None,
            maximum: None,
            default: Vec::new(),
            applicability: StyleApplicability::Both,
            affects_fidelity: false,
            required_for_materialization: false,
        }],
    )
    .expect_err("empty enum domain");
    assert_eq!(empty_enum, StyleSchemaError::EmptyEnumDomain);

    let reversed = StyleSchema::new(
        "test.generic",
        "1",
        Vec::new(),
        vec![integer_descriptor("option-delta", 9, 2)],
    )
    .expect_err("reversed range");
    assert_eq!(reversed, StyleSchemaError::ReversedRange);

    let duplicate = StyleSchema::new(
        "test.generic",
        "1",
        Vec::new(),
        vec![
            integer_descriptor("option-beta", 1, 8),
            integer_descriptor("option-beta", 1, 4),
        ],
    )
    .expect_err("duplicate key");
    assert_eq!(duplicate, StyleSchemaError::DuplicatePropertyKey);

    let empty_adapter =
        StyleSchema::new(" ", "1", Vec::new(), Vec::new()).expect_err("empty adapter identifier");
    assert_eq!(empty_adapter, StyleSchemaError::EmptySchemaIdentifier);
}

/// Verifies provenance records the applied source chain and winners.
#[test]
fn provenance_names_applied_sources_and_winners() {
    let resolved = ResolvedStyle::inline(StyleValue::new(b"effective".to_vec()));
    let mut provenance = ResolvedStyleProvenance::new(
        resolved,
        vec![
            StyleSource::new(
                StyleSourceKind::DomainDefault,
                "domain",
                None,
                StyleValue::new(b"base".to_vec()),
            ),
            StyleSource::new(
                StyleSourceKind::UserPreference,
                "principal-1",
                None,
                StyleValue::new(b"effective".to_vec()),
            ),
        ],
    );
    provenance
        .property_outcomes
        .push(sr_projection::StylePropertyOutcome {
            property: "option-alpha".to_owned(),
            winner: 1,
        });

    assert!(provenance.applied_from(StyleSourceKind::UserPreference));
    assert!(!provenance.applied_from(StyleSourceKind::WorkspaceMaterializationPolicy));
    assert_eq!(
        provenance
            .winner_for("option-alpha")
            .map(|source| source.identity.as_str()),
        Some("principal-1")
    );
    assert_eq!(provenance.winner_for("unrecorded-option"), None);
}

/// Verifies a specification still stores only the style reference boundary.
#[test]
fn specification_stores_style_reference_not_resolved_value() {
    let specification = ProjectionSpec {
        scope: SemanticScopeRef {
            entity: EntityId::from_bytes([16; 16]),
            revision: None,
        },
        kind: kind("test.generic"),
        class: ProjectionClass::Editable,
        style: StyleRef::Value(StyleValue::new(b"reference-only".to_vec())),
        adapter_id: "test".to_owned(),
        adapter_version: "1".to_owned(),
        renderer: RendererVersion::new("renderer-1"),
        configuration: std::collections::BTreeMap::new(),
        ownership: OwnershipClass::Managed,
        location: None,
    };

    assert_eq!(
        specification.style,
        StyleRef::Value(StyleValue::new(b"reference-only".to_vec()))
    );
}
