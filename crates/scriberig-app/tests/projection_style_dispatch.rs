//! Proves one host style adapter routes each projection kind to its own domain.

#[path = "projection_style_fixture.rs"]
mod fixture;

use std::sync::Arc;

use scriberig_app::{
    ApplicationContext, InMemoryProjectionStyleStore, KindDispatchStyleAdapter,
    ProjectionStyleBackend, ProjectionStyleConfiguration, StyleScopeKind,
};
use sr_projection::{ProjectionFidelity, ProjectionKind, ProjectionPurpose};

use fixture::{DocumentStyleAdapter, context_for, field, fields, kind, store_style};

/// Builds one service whose style adapter dispatches by projection kind.
///
/// # Returns
/// The service and the shared style configuration store.
fn dispatching_service() -> (
    scriberig_app::ApplicationService<fixture::RecordingBackend>,
    Arc<InMemoryProjectionStyleStore>,
) {
    let store = Arc::new(InMemoryProjectionStyleStore::new());
    let dispatcher = KindDispatchStyleAdapter::new().with_adapter(
        fixture::style_support::KIND,
        Arc::new(DocumentStyleAdapter) as Arc<dyn sr_projection::ProjectionStyleAdapter>,
    );
    let service = scriberig_app::ApplicationService::new(fixture::RecordingBackend::default())
        .with_projection_style_configuration(ProjectionStyleConfiguration::new(
            Arc::new(dispatcher),
            store.clone(),
        ));
    (service, store)
}

/// Verifies an unregistered kind is refused rather than generically styled.
#[test]
fn an_unregistered_kind_is_refused_rather_than_generically_styled() {
    let (service, _store) = dispatching_service();
    let unregistered = ProjectionKind::new("unregistered-kind").expect("kind label is valid");

    let error = service
        .projection_style_effective(
            &context_for("alice"),
            &scriberig_app::StyleResolutionRequest::new(
                unregistered.clone(),
                "proj-1",
                "ws-1",
                ProjectionPurpose::InteractiveView,
            ),
        )
        .expect_err("no domain owns this kind");

    assert!(
        matches!(error, scriberig_app::ApplicationError::UnsupportedOperation { operation } if operation == "projection style default")
    );
    assert!(
        service
            .projection_style_schema(
                &ApplicationContext::empty(),
                &scriberig_app::ProjectionStyleSchemaRequest {
                    projection_kind: unregistered.clone()
                },
            )
            .expect("schema query")
            .is_none()
    );
    assert_eq!(
        service
            .projection_style_fidelity(
                &ApplicationContext::empty(),
                &scriberig_app::ProjectionStyleFidelityRequest {
                    projection_kind: unregistered,
                    resolved: sr_projection::ResolvedStyle::inline(sr_projection::StyleValue::new(
                        b"anything".to_vec()
                    )),
                    purpose: ProjectionPurpose::InteractiveView,
                },
            )
            .expect("fidelity query")
            .fidelity,
        ProjectionFidelity::ReadOnlyLossy
    );
}

/// Verifies a registered kind resolves through its own domain rules.
#[test]
fn a_registered_kind_resolves_through_its_own_domain() {
    let (service, store) = dispatching_service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);

    let effective = service
        .projection_style_effective(
            &context_for("alice"),
            &scriberig_app::StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                ProjectionPurpose::InteractiveView,
            ),
        )
        .expect("registered kind resolves");

    assert_eq!(
        field(&fields(effective.resolved.value()), "width"),
        Some("100")
    );
    assert_eq!(
        effective.fidelity.fidelity,
        ProjectionFidelity::LossAwareEditable,
        "the fixture domain reports width 100 as loss-aware for this exact style"
    );
}

/// Verifies a shared purpose still excludes personal state through the dispatcher.
#[test]
fn the_dispatcher_keeps_shared_purpose_free_of_personal_state() {
    let (service, store) = dispatching_service();
    store_style(&store, StyleScopeKind::User, "alice", &[("width", "100")]);

    let effective = service
        .projection_style_effective(
            &context_for("alice"),
            &scriberig_app::StyleResolutionRequest::new(
                kind(),
                "proj-1",
                "ws-1",
                ProjectionPurpose::SharedMaterialization,
            ),
        )
        .expect("shared style resolves");

    assert_eq!(
        field(&fields(effective.resolved.value()), "width"),
        Some("80")
    );
}
