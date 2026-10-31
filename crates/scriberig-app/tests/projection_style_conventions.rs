//! Proves the deterministic repository-convention source sits in shared precedence.

#[allow(dead_code)] // This binary uses a subset of the shared style fixtures.
#[path = "projection_style_fixture.rs"]
mod fixture;

use std::sync::Arc;

use scriberig_app::{
    ApplicationService, InMemoryProjectionStyleStore, ProjectionStyleBackend,
    ProjectionStyleConfiguration, ProjectionStyleSettingRequest, StaticRepositoryConvention,
    StyleResolutionRequest, StyleScopeKind,
};
use sr_projection::{ProjectionPurpose, StyleSourceKind};

use fixture::{
    DocumentStyleAdapter, RecordingBackend, context_for, effective_fields, field, kind,
    store_style, style,
};

/// Builds one service with deterministic repository-convention sources installed.
///
/// - `conventions`: Convention sources in application order.
///
/// # Returns
/// The service and its style configuration store.
fn service_with_conventions(
    conventions: Vec<Arc<dyn scriberig_app::RepositoryConventionSource>>,
) -> (
    ApplicationService<RecordingBackend>,
    Arc<InMemoryProjectionStyleStore>,
) {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let service = ApplicationService::new(RecordingBackend::default())
        .with_projection_style_configuration(
            ProjectionStyleConfiguration::new(Arc::new(DocumentStyleAdapter), store.clone())
                .with_conventions(conventions),
        );
    (service, store)
}

/// Reports the ordered source classes resolution applied for one purpose.
///
/// - `service`: Application service resolving the style.
/// - `purpose`: Why the style is being resolved.
///
/// # Returns
/// The applied source classes in exact application order.
fn applied_kinds(
    service: &ApplicationService<RecordingBackend>,
    purpose: ProjectionPurpose,
) -> Vec<StyleSourceKind> {
    service
        .projection_style_effective(
            &context_for("alice"),
            &StyleResolutionRequest::new(kind(), "proj-1", "ws-1", purpose),
        )
        .expect("effective style resolves")
        .applied
        .iter()
        .map(|source| source.kind)
        .collect()
}

/// Verifies a repository convention outranks the domain default and loses to
/// shared project policy, with the contribution recorded in provenance.
#[test]
fn a_repository_convention_sits_between_the_default_and_project_policy() {
    let (service, store) = service_with_conventions(vec![Arc::new(
        StaticRepositoryConvention::new("proj-1", kind(), style(&[("emphasis", "italic")])),
    )]);

    let from_convention = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    assert_eq!(
        field(&from_convention, "emphasis"),
        Some("italic"),
        "the convention replaces the domain default"
    );
    assert_eq!(
        field(&from_convention, "indent"),
        Some("spaces"),
        "the convention leaves unset fields at the domain default"
    );

    store_style(
        &store,
        StyleScopeKind::Project,
        "proj-1",
        &[("emphasis", "bold")],
    );

    let after_policy = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    assert_eq!(
        field(&after_policy, "emphasis"),
        Some("bold"),
        "project policy outranks the repository convention"
    );

    let effective = service
        .projection_style_effective(
            &context_for("alice"),
            &StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                ProjectionPurpose::InteractiveView,
            ),
        )
        .expect("effective style resolves");
    assert_eq!(
        effective.applied.iter().map(|s| s.kind).collect::<Vec<_>>(),
        vec![
            StyleSourceKind::DomainDefault,
            StyleSourceKind::RepositoryConvention,
            StyleSourceKind::ProjectPolicy,
        ]
    );
    assert_eq!(
        effective
            .applied
            .iter()
            .find(|source| source.kind == StyleSourceKind::RepositoryConvention)
            .map(|source| source.value.clone()),
        Some(style(&[("emphasis", "italic")])),
        "provenance carries the exact convention contribution"
    );
}

/// Verifies a repository convention also feeds shared materialization and stays
/// below the workspace materialization policy.
#[test]
fn a_repository_convention_applies_under_the_workspace_materialization_policy() {
    let (service, _store) = service_with_conventions(vec![Arc::new(
        StaticRepositoryConvention::new("proj-1", kind(), style(&[("emphasis", "italic")])),
    )]);

    assert_eq!(
        field(
            &effective_fields(
                &service,
                &context_for("alice"),
                ProjectionPurpose::SharedMaterialization,
                None,
            ),
            "emphasis"
        ),
        Some("italic"),
        "a shared convention reaches a shared purpose"
    );

    service
        .set_projection_style_materialization_policy(
            &context_for("alice"),
            &ProjectionStyleSettingRequest::new(
                kind(),
                "ws-1",
                Some(style(&[("emphasis", "bold")])),
            ),
        )
        .expect("materialization policy stored");

    assert_eq!(
        field(
            &effective_fields(
                &service,
                &context_for("alice"),
                ProjectionPurpose::SharedMaterialization,
                None,
            ),
            "emphasis"
        ),
        Some("bold"),
        "the workspace materialization policy outranks the convention"
    );
    assert_eq!(
        applied_kinds(&service, ProjectionPurpose::SharedMaterialization),
        vec![
            StyleSourceKind::DomainDefault,
            StyleSourceKind::RepositoryConvention,
            StyleSourceKind::WorkspaceMaterializationPolicy,
        ]
    );
}

/// Verifies a convention source contributes only to the project and kind it names.
#[test]
fn a_repository_convention_contributes_only_to_its_own_project_and_kind() {
    let (service, _store) = service_with_conventions(vec![Arc::new(
        StaticRepositoryConvention::new("other-project", kind(), style(&[("emphasis", "italic")])),
    )]);

    assert_eq!(
        applied_kinds(&service, ProjectionPurpose::InteractiveView),
        vec![StyleSourceKind::DomainDefault],
        "a convention for another project contributes nothing"
    );
}
