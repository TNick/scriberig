//! Proves shared projection style settings, compatibility, and fidelity behavior.

#[path = "projection_style_fixture.rs"]
mod fixture;

use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, ApplicationError, ApplicationService, CompatibilityStyleAdapter,
    InMemoryProjectionStyleStore, ProjectionStyleBackend, ProjectionStyleConfiguration,
    ProjectionStyleFidelityRequest, ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest,
};
use sr_projection::{ProjectionFidelity, ResolvedStyle};

use fixture::{
    context_for, effective_fields, field, fields, kind, service, set_style, store_style,
};

/// Verifies style setting writes never run a semantic operation.
#[test]
fn style_settings_never_dispatch_a_semantic_migration() {
    let (service, _store, probe) = service();

    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &set_style(&[("width", "100")]),
        )
        .expect("preference written");
    assert_eq!(
        field(
            &effective_fields(
                &service,
                &context_for("alice"),
                sr_projection::ProjectionPurpose::InteractiveView,
                None,
            ),
            "width"
        ),
        Some("100"),
        "the written preference is the effective interactive style"
    );
    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &ProjectionStyleSettingRequest::new(kind(), "caller-ignored", None),
        )
        .expect("preference cleared");

    assert_eq!(
        probe.seen(),
        0,
        "style settings must not dispatch semantic operations"
    );
}

/// Verifies anonymous callers cannot write personal style settings.
#[test]
fn style_settings_require_a_trusted_personal_principal() {
    let (service, _store, _probe) = service();

    let error = service
        .set_projection_style_user_preference(
            &ApplicationContext::empty(),
            &set_style(&[("width", "100")]),
        )
        .expect_err("anonymous callers own no personal preference");

    assert!(
        matches!(error, ApplicationError::PermissionDenied { action, .. } if action == "set projection style preference")
    );
}

/// Verifies a stale style revision is rejected as a conflict.
#[test]
fn a_stale_style_revision_is_rejected_as_a_conflict() {
    let (service, _store, _probe) = service();
    let stored = service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &set_style(&[("width", "100")]),
        )
        .expect("first write");
    assert_eq!(stored.as_ref().map(|entry| entry.revision), Some(1));

    let mut stale = set_style(&[("width", "80")]);
    stale.expected_revision = Some(7);
    let error = service
        .set_projection_style_user_preference(&context_for("alice"), &stale)
        .expect_err("a stale revision must conflict");

    match &error {
        ApplicationError::SemanticConflict { subject } => {
            assert_eq!(subject, "projection style user:alice:document");
        }
        other => panic!("expected a revision conflict, got {other:?}"),
    }
}

/// Verifies clearing a preference restores the shared effective style.
#[test]
fn clearing_a_preference_falls_back_to_shared_layers() {
    let (service, _store, _probe) = service();
    service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &set_style(&[("width", "100")]),
        )
        .expect("preference written");
    assert_eq!(
        effective_fields(
            &service,
            &context_for("alice"),
            sr_projection::ProjectionPurpose::InteractiveView,
            None,
        )
        .get("width")
        .map(String::as_str),
        Some("100")
    );

    let cleared = service
        .set_projection_style_user_preference(
            &context_for("alice"),
            &ProjectionStyleSettingRequest::new(kind(), "caller-ignored", None),
        )
        .expect("preference cleared");
    assert_eq!(cleared.as_ref().map(|entry| entry.revision), None);
    assert_eq!(
        effective_fields(
            &service,
            &context_for("alice"),
            sr_projection::ProjectionPurpose::InteractiveView,
            None,
        )
        .get("width")
        .map(String::as_str),
        Some("80")
    );
}

/// Verifies project and workspace policy settings reach the right scopes.
#[test]
fn shared_policy_settings_apply_to_their_own_scope() {
    let (service, _store, _probe) = service();

    service
        .set_projection_style_project_policy(
            &context_for("maintainer"),
            &scriberig_app::ProjectionStyleSettingRequest::new(
                kind(),
                "proj-1",
                Some(fixture::style(&[("width", "100")])),
            ),
        )
        .expect("project policy written");
    service
        .set_projection_style_materialization_policy(
            &context_for("maintainer"),
            &scriberig_app::ProjectionStyleSettingRequest::new(
                kind(),
                "other-workspace",
                Some(fixture::style(&[("indent", "tabs")])),
            ),
        )
        .expect("workspace policy written");

    let shared = effective_fields(
        &service,
        &context_for("alice"),
        sr_projection::ProjectionPurpose::SharedMaterialization,
        None,
    );
    assert_eq!(shared.get("width").map(String::as_str), Some("100"));
    assert_eq!(shared.get("indent").map(String::as_str), Some("spaces"));

    let in_workspace = {
        let mut request = scriberig_app::StyleResolutionRequest::new(
            kind(),
            "proj-1",
            "other-workspace",
            sr_projection::ProjectionPurpose::SharedMaterialization,
        );
        request.principal = Some("alice".to_owned());
        service
            .projection_style_effective(&context_for("alice"), &request)
            .expect("effective style resolves")
    };
    assert_eq!(
        fields(in_workspace.resolved.value())
            .get("indent")
            .map(String::as_str),
        Some("tabs")
    );
}

/// Verifies an unmigrated adapter refuses generic style layering.
#[test]
fn an_unmigrated_adapter_refuses_generic_style_layering() {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let service = ApplicationService::new(fixture::RecordingBackend::default())
        .with_projection_style_configuration(ProjectionStyleConfiguration::new(
            Arc::new(CompatibilityStyleAdapter),
            store.clone(),
        ));
    store_style(
        &store,
        scriberig_app::StyleScopeKind::Project,
        "proj-1",
        &[("width", "100")],
    );

    let error = service
        .projection_style_effective(
            &context_for("alice"),
            &scriberig_app::StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                sr_projection::ProjectionPurpose::InteractiveView,
            ),
        )
        .expect_err("generic merge must be refused");

    assert!(
        matches!(error, ApplicationError::UnsupportedOperation { operation } if operation == "projection style normalize")
    );
}

/// Verifies an unmigrated adapter still resolves its fixed default style.
#[test]
fn an_unmigrated_adapter_resolves_one_default_with_read_only_fidelity() {
    let service = ApplicationService::new(fixture::RecordingBackend::default())
        .with_projection_style_configuration(ProjectionStyleConfiguration::default());

    let effective = service
        .projection_style_effective(
            &context_for("alice"),
            &scriberig_app::StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                sr_projection::ProjectionPurpose::InteractiveView,
            ),
        )
        .expect("compatibility style resolves");

    assert_eq!(effective.resolved.value().as_bytes(), Vec::<u8>::new());
    assert_eq!(
        effective.fidelity.fidelity,
        ProjectionFidelity::ReadOnlyLossy
    );
    assert_eq!(effective.resolved.schema(), None);
}

/// Verifies fidelity is assessed per exact resolved style rather than per kind.
#[test]
fn fidelity_is_answered_per_exact_resolved_style() {
    let (service, _store, _probe) = service();
    let assess = |pairs: &[(&str, &str)]| {
        service
            .projection_style_fidelity(
                &ApplicationContext::empty(),
                &ProjectionStyleFidelityRequest {
                    projection_kind: kind(),
                    resolved: ResolvedStyle::inline(fixture::style(pairs)),
                    purpose: sr_projection::ProjectionPurpose::InteractiveView,
                },
            )
            .expect("fidelity resolves")
    };

    let default_fidelity = assess(&[("indent", "spaces"), ("width", "80")]);
    let wide_fidelity = assess(&[("indent", "spaces"), ("width", "100")]);
    let tab_fidelity = assess(&[("indent", "tabs"), ("width", "80")]);

    assert_eq!(
        default_fidelity.fidelity,
        ProjectionFidelity::LosslessEditable
    );
    assert!(default_fidelity.fidelity.is_editable());
    assert_eq!(default_fidelity.omitted_categories, Vec::<String>::new());
    assert_eq!(
        wide_fidelity.fidelity,
        ProjectionFidelity::LossAwareEditable
    );
    assert_eq!(
        wide_fidelity.omitted_categories,
        vec!["line-width".to_owned()]
    );
    assert_eq!(tab_fidelity.fidelity, ProjectionFidelity::ReadOnlyLossy);
    assert!(!tab_fidelity.fidelity.is_editable());
}

/// Verifies the installed adapter's style schema reaches callers.
#[test]
fn style_schema_is_reported_from_the_installed_adapter() {
    let (service, _store, _probe) = service();
    let schema = service
        .projection_style_schema(
            &ApplicationContext::empty(),
            &ProjectionStyleSchemaRequest {
                projection_kind: kind(),
            },
        )
        .expect("schema resolves");

    assert_eq!(
        schema.as_ref().map(|schema| schema.properties.len()),
        Some(3)
    );
    assert_eq!(
        schema
            .as_ref()
            .map(|schema| schema.reference.adapter_id().to_owned()),
        Some("document-style".to_owned())
    );
}
