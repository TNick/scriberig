//! Proves purpose-scoped projection style resolution across product surfaces.

#[path = "projection_style_fixture.rs"]
mod fixture;

use std::collections::BTreeMap;

use scriberig_app::{
    ApplicationError, ProjectionStyleBackend, StyleResolutionRequest, StyleScopeKind,
};
use sr_projection::{ProjectionPurpose, StyleRef, StyleSourceKind};

use fixture::{context_for, effective_fields, field, fields, kind, service, store_style};

/// Verifies a personal preference applies to an interactive view.
#[test]
fn interactive_view_includes_the_personal_preference() {
    let (service, store, _probe) = service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);

    let resolved = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );

    assert_eq!(field(&resolved, "width"), Some("100"));
    assert_eq!(field(&resolved, "indent"), Some("spaces"));
}

/// Verifies shared materialization ignores a personal preference.
#[test]
fn shared_materialization_excludes_the_personal_preference() {
    let (service, store, _probe) = service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);

    let resolved = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::SharedMaterialization,
        None,
    );

    assert_eq!(field(&resolved, "width"), Some("80"));
}

/// Verifies a view override changes only the view it names.
#[test]
fn a_view_override_applies_only_to_its_own_view() {
    let (service, store, _probe) = service();
    store_style(
        &store,
        StyleScopeKind::View,
        "view-a",
        &[("emphasis", "italic")],
    );

    let in_view_a = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        Some("view-a"),
    );
    let in_view_b = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        Some("view-b"),
    );

    assert_eq!(field(&in_view_a, "emphasis"), Some("italic"));
    assert_eq!(field(&in_view_b, "emphasis"), Some("bold"));
}

/// Verifies two principals resolve distinct interactive styles for one scope.
#[test]
fn two_users_resolve_different_interactive_styles_for_one_scope() {
    let (service, store, _probe) = service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);
    store_style(
        &store,
        StyleScopeKind::User,
        "bob",
        &[("emphasis", "italic")],
    );

    let alice = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );
    let bob = effective_fields(
        &service,
        &context_for("bob"),
        ProjectionPurpose::InteractiveView,
        None,
    );

    assert_eq!(field(&alice, "width"), Some("100"));
    assert_eq!(field(&alice, "emphasis"), Some("bold"));
    assert_eq!(field(&bob, "width"), Some("80"));
    assert_eq!(field(&bob, "emphasis"), Some("italic"));
}

/// Verifies both principals resolve one shared style under a workspace policy.
#[test]
fn the_same_users_resolve_one_shared_materialization_style() {
    let (service, store, _probe) = service();
    for principal in ["alice", "bob"] {
        store_style(
            &store,
            StyleScopeKind::User,
            principal,
            &[("emphasis", "italic")],
        );
    }
    store_style(
        &store,
        StyleScopeKind::Workspace,
        "ws-1",
        &[("indent", "tabs"), ("width", "100")],
    );

    let alice = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::SharedMaterialization,
        None,
    );
    let bob = effective_fields(
        &service,
        &context_for("bob"),
        ProjectionPurpose::SharedMaterialization,
        None,
    );

    assert_eq!(alice, bob);
    assert_eq!(field(&alice, "indent"), Some("tabs"));
    assert_eq!(field(&alice, "emphasis"), Some("bold"));
}

/// Verifies a style the domain rejects surfaces as a structured diagnostic.
#[test]
fn an_invalid_domain_style_produces_a_structured_diagnostic() {
    let (service, store, _probe) = service();
    store_style(
        &store,
        StyleScopeKind::User,
        "alice",
        &[("indentation", "spaces")],
    );

    let error = service
        .projection_style_effective(
            &context_for("alice"),
            &StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                ProjectionPurpose::InteractiveView,
            ),
        )
        .expect_err("the domain must reject the unknown field");

    match error {
        ApplicationError::SchemaViolation { message } => {
            assert!(message.contains("indentation"), "diagnostic: {message}");
        }
        other => panic!("expected a schema violation, got {other:?}"),
    }
}

/// Verifies resolution records each applied source in precedence order.
#[test]
fn effective_provenance_names_every_applied_source() {
    let (service, store, _probe) = service();
    store_style(
        &store,
        StyleScopeKind::Project,
        "proj-1",
        &[("emphasis", "italic")],
    );
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "80")]);
    store_style(
        &store,
        StyleScopeKind::View,
        "view-a",
        &[("indent", "tabs")],
    );

    let mut request =
        StyleResolutionRequest::new(kind(), "proj-1", "ws-1", ProjectionPurpose::InteractiveView);
    request.view_id = Some("view-a".to_owned());
    let effective = service
        .projection_style_effective(&context_for("alice"), &request)
        .expect("effective style resolves");

    let applied: Vec<StyleSourceKind> =
        effective.applied.iter().map(|source| source.kind).collect();
    assert_eq!(
        applied,
        vec![
            StyleSourceKind::DomainDefault,
            StyleSourceKind::ProjectPolicy,
            StyleSourceKind::UserPreference,
            StyleSourceKind::ViewOverride,
        ]
    );
    assert_eq!(
        effective
            .applied
            .iter()
            .map(|source| source.identity.as_str())
            .collect::<Vec<_>>(),
        vec!["adapter-domain-default", "proj-1", "alice", "view-a"]
    );
    assert_eq!(
        effective.resolved.reference(),
        &StyleRef::Value(effective.resolved.value().clone())
    );
    assert_eq!(
        effective
            .resolved
            .schema()
            .map(sr_projection::StyleSchemaRef::adapter_id),
        Some("document-style")
    );
    assert_eq!(
        field(&fields(effective.resolved.value()), "indent"),
        Some("tabs")
    );
}

/// Verifies shared provenance never names a personal source.
#[test]
fn shared_materialization_never_lists_a_personal_source() {
    let (service, store, _probe) = service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);
    store_style(
        &store,
        StyleScopeKind::View,
        "view-a",
        &[("emphasis", "italic")],
    );

    let mut request = StyleResolutionRequest::new(
        kind(),
        "proj-1",
        "ws-1",
        ProjectionPurpose::SharedMaterialization,
    );
    request.view_id = Some("view-a".to_owned());
    request.principal = Some("alice".to_owned());
    let effective = service
        .projection_style_effective(&context_for("alice"), &request)
        .expect("effective style resolves");

    assert!(!effective.applied.iter().any(|source| matches!(
        source.kind,
        StyleSourceKind::UserPreference | StyleSourceKind::ViewOverride
    )));
    assert_eq!(
        effective
            .applied
            .iter()
            .map(|source| source.identity.as_str())
            .collect::<Vec<_>>(),
        vec!["adapter-domain-default"]
    );
}

/// Verifies a named personal style requires the export purpose.
#[test]
fn an_explicit_personal_style_requires_the_export_purpose() {
    let (service, _store, _probe) = service();
    let mut request =
        StyleResolutionRequest::new(kind(), "proj-1", "ws-1", ProjectionPurpose::InteractiveView);
    request.explicit_personal = Some(fixture::style(&[("emphasis", "italic")]));

    let error = service
        .projection_style_effective(&context_for("alice"), &request)
        .expect_err("personal state may not be borrowed implicitly");

    assert!(
        matches!(error, ApplicationError::InvalidRequest { operation, .. } if operation == "projection style resolve")
    );
}

/// Verifies an explicit export applies and records a named personal style.
#[test]
fn an_explicit_export_may_opt_into_personal_state_and_records_it() {
    let (service, _store, _probe) = service();
    let mut request =
        StyleResolutionRequest::new(kind(), "proj-1", "ws-1", ProjectionPurpose::ExplicitExport);
    request.explicit_personal = Some(fixture::style(&[("emphasis", "italic")]));

    let effective = service
        .projection_style_effective(&context_for("alice"), &request)
        .expect("export style resolves");

    assert_eq!(
        field(&fields(effective.resolved.value()), "emphasis"),
        Some("italic")
    );
    assert_eq!(
        effective.applied.last().map(|source| source.kind),
        Some(StyleSourceKind::ExplicitExportOverride)
    );
}

/// Verifies a cleared scope leaves the domain default untouched.
#[test]
fn an_unconfigured_scope_resolves_the_domain_default() {
    let (service, _store, _probe) = service();

    let resolved: BTreeMap<String, String> = effective_fields(
        &service,
        &context_for("alice"),
        ProjectionPurpose::InteractiveView,
        None,
    );

    assert_eq!(field(&resolved, "indent"), Some("spaces"));
    assert_eq!(field(&resolved, "width"), Some("80"));
    assert_eq!(field(&resolved, "emphasis"), Some("bold"));
}
