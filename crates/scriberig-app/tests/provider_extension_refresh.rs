//! Verifies staged extension refresh uses the shared upstream publication gate.

use std::sync::Arc;

use rpi_ai::providers::faux::{FauxProvider, FauxScript};
use rpi_ai::{Api, Model, Provider};
use rpi_extensions::{ExtensionSession, GenerationCandidate};
use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, BranchContext, Capability, CapabilityAction,
    CapabilityConstraints, CapabilityContext, CapabilityId, CapabilityRegistry, CapabilityScope,
    CompositeBackend, DomainOperationRequest, OperationKind,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_query::{QueryValue, Selection};

#[path = "universal_operation_support.rs"]
mod operation_support;

/// Backend fixture that leaves service lifecycle behavior to the application.
struct Fixture;

impl ApplicationBackend for Fixture {
    /// Rejects semantic operations because this fixture exercises refresh only.
    ///
    /// - `_context`: Application authorization context, unused by the fixture.
    /// - `_request`: Application operation, unsupported by the fixture.
    ///
    /// # Returns
    /// Always returns an unsupported-operation error.
    ///
    /// # Errors
    /// Returns `UnsupportedOperation` for every operation.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "provider extension refresh fixture".to_owned(),
        })
    }
}

/// Verifies paired publication and preservation when host staging fails.
#[test]
fn staged_refresh_pairs_catalog_and_preserves_the_live_pair_on_failure() {
    let service = ApplicationService::new(Fixture);
    let peer = ApplicationService::new(Fixture)
        .with_provider_extension_runtime(service.provider_extension_runtime());
    let before = service.extension_catalog_snapshot();
    assert_eq!(before.extension.catalog_revision(), None);

    let published = service
        .refresh_extension_catalog_generation(|| {
            Ok::<_, String>((
                GenerationCandidate::new(ExtensionSession::none()),
                vec![model_row()],
            ))
        })
        .expect("publish staged extension generation");
    let live = service.extension_catalog_snapshot();
    assert_eq!(live.extension.id(), published.current.id());
    assert_eq!(
        live.extension.catalog_revision(),
        Some(live.catalog.revision)
    );
    assert_eq!(live.catalog.models.len(), 1);
    assert_eq!(live.catalog.models[0].provider, "provider-stage");
    assert_eq!(
        peer.extension_catalog_snapshot().extension.id(),
        live.extension.id()
    );
    assert_eq!(peer.upstream_models(), live.catalog.models.clone());

    let before_catalog_reload = service.extension_catalog_snapshot();
    let reloaded = service.reload_model_catalog(vec![Model::new(
        "model-reloaded",
        "model-reloaded",
        Api::OpenaiCompletions,
        "provider-reloaded",
        "https://provider-reloaded.example.test/v1",
    )]);
    let after_catalog_reload = service.extension_catalog_snapshot();
    assert!(after_catalog_reload.extension.id() > before_catalog_reload.extension.id());
    assert_eq!(
        after_catalog_reload.extension.catalog_revision(),
        Some(after_catalog_reload.catalog.revision)
    );
    assert_eq!(reloaded.revision(), after_catalog_reload.catalog.revision);
    assert_eq!(peer.upstream_models(), after_catalog_reload.catalog.models);

    let before_failed_stage = service.extension_catalog_snapshot();

    let failure = service.refresh_extension_catalog_generation(|| {
        Err::<(GenerationCandidate, Vec<Model>), _>("staging refused".to_owned())
    });
    assert!(matches!(
        failure,
        Err(rpi_model_runtime::ExtensionCatalogRefreshError::Stage(reason))
            if reason == "staging refused"
    ));
    let unchanged = service.extension_catalog_snapshot();
    assert_eq!(unchanged.extension.id(), before_failed_stage.extension.id());
    assert_eq!(
        unchanged.extension.catalog_revision(),
        before_failed_stage.extension.catalog_revision()
    );
    assert_eq!(unchanged.catalog, before_failed_stage.catalog);
}

/// Verifies an injected prepopulated catalog starts with a paired generation.
#[test]
fn injected_catalog_is_paired_before_application_service_reads_it() {
    let catalog = rpi_model_runtime::ModelCatalog::new();
    catalog.reconcile_models_snapshot(vec![model_row()]);
    let runtime = scriberig_app::ProviderExtensionRuntime::with_model_catalog(catalog.clone());
    let snapshot = runtime.extension_catalog_snapshot();
    assert_eq!(
        snapshot.extension.catalog_revision(),
        Some(snapshot.catalog.revision)
    );
    assert_eq!(snapshot.catalog.models, catalog.snapshot().models);
}

/// Verifies provider adapters publish and fail atomically with their model rows.
#[test]
fn provider_adapters_publish_with_their_catalog_generation() {
    let service = ApplicationService::new(Fixture);
    let peer = ApplicationService::new(Fixture)
        .with_provider_extension_runtime(service.provider_extension_runtime());
    let provider = FauxProvider::with_models(FauxScript::new(), vec![model_row()]);
    let provider: Arc<dyn Provider> = provider;
    let published = service
        .provider_extension_runtime()
        .refresh_provider_extension_generation(|| {
            Ok::<_, String>((
                GenerationCandidate::new(ExtensionSession::none()),
                vec![model_row()],
                vec![provider],
            ))
        })
        .expect("publish provider generation");
    assert_eq!(
        published.current.catalog_revision(),
        Some(service.model_catalog_generation().revision())
    );
    assert!(peer.provider_extension_runtime().provider("faux").is_some());
    let (generation, provider_ids) = service
        .provider_extension_runtime()
        .provider_generation_snapshot();
    assert_eq!(provider_ids, vec!["faux"]);
    assert_eq!(
        generation.extension.catalog_revision(),
        Some(generation.catalog.revision)
    );

    let before = service.extension_catalog_snapshot();
    let failure = service
        .provider_extension_runtime()
        .refresh_provider_extension_generation(|| {
            Err::<_, String>("provider staging refused".to_owned())
        });
    assert!(matches!(
        failure,
        Err(rpi_model_runtime::ExtensionCatalogRefreshError::Stage(reason))
            if reason == "provider staging refused"
    ));
    let after = service.extension_catalog_snapshot();
    assert_eq!(after.extension.id(), before.extension.id());
    assert_eq!(after.catalog, before.catalog);
    assert!(
        service
            .provider_extension_runtime()
            .provider("faux")
            .is_some()
    );
}

/// Keeps Scriberig domain operations live across provider-package generations.
#[test]
fn domain_capability_remains_available_across_provider_extension_refresh() {
    let snapshot = SnapshotId::new([43; 32]);
    let entity = EntityId::from_bytes([44; 16]);
    let mut capabilities = CapabilityRegistry::new();
    capabilities
        .register(operation_support::InspectionCapabilityProvider)
        .expect("register semantic domain capability");
    let backend = CompositeBackend::new(
        operation_support::InspectingReadFixture,
        operation_support::OperationFixture::new(),
    )
    .with_capability_registry(capabilities);
    let service = ApplicationService::new(backend);
    let model = model_row();
    let provider: Arc<dyn Provider> =
        FauxProvider::with_models(FauxScript::new(), vec![model.clone()]);
    service
        .provider_extension_runtime()
        .refresh_provider_extension_generation(|| {
            Ok::<_, String>((
                GenerationCandidate::new(ExtensionSession::none()),
                vec![model],
                vec![provider],
            ))
        })
        .expect("publish provider package generation");
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        capabilities: CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Entity(entity),
            actions: [CapabilityAction::Custom("operation.rename".to_owned())]
                .into_iter()
                .collect(),
            constraints: CapabilityConstraints {
                snapshot: Some(snapshot),
                operation: Some(OperationKind::DomainOperation),
                ..CapabilityConstraints::unrestricted()
            },
        }]),
        ..ApplicationContext::empty()
    };
    let operation = ApplicationRequest::DomainOperation(DomainOperationRequest {
        capability: CapabilityId::new("operation.rename").expect("capability id"),
        subject: Selection {
            snapshot,
            entity,
            revision: None,
            relation_path: Vec::new(),
            projection_anchor: None,
        },
        context: BranchContext {
            base_snapshot: snapshot,
            branch: None,
        },
        arguments: vec![("name".to_owned(), QueryValue::Text("refreshed".to_owned()))],
    });
    let result = service
        .execute(&context, &operation)
        .expect("domain operation remains available after provider refresh");
    let ApplicationResult::DomainOperation(candidate) = result else {
        panic!("expected a semantic candidate result");
    };
    let applied = service
        .execute(
            &context,
            &ApplicationRequest::Apply(scriberig_app::ApplyRequest {
                base_snapshot: snapshot,
                branch: None,
                change: candidate.change,
            }),
        )
        .expect("semantic candidate uses the shared apply path");
    assert!(matches!(applied, ApplicationResult::Apply(_)));
}

/// Builds the model row staged with the replacement extension session.
///
/// # Returns
/// A credential-free upstream provider/model row.
fn model_row() -> Model {
    Model::new(
        "model-stage",
        "model-stage",
        Api::OpenaiCompletions,
        "provider-stage",
        "https://provider-stage.example.test/v1",
    )
}
