//! Verifies HTTP project routes retain the host-injected DWG provider.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_domain_cad::{
    CadExternalProvider, CadFormat, CadProviderDescriptor, CadProviderError, CadProviderErrorKind,
    CadProviderFormatCapability, CadProviderIdentity, CadProviderImport,
    CadProviderOpaquePreservation, CadProviderRegistry, Drawing,
};

/// Counts calls to the fake provider so the route test can prove import used it.
struct ImportProvider(Arc<AtomicUsize>);

impl CadExternalProvider for ImportProvider {
    /// Describes a read-only DWG provider used by this transport test.
    ///
    /// # Returns
    /// A safe provider descriptor advertising read-only DWG support.
    fn descriptor(&self) -> CadProviderDescriptor {
        CadProviderDescriptor {
            id: "http-fixture.dwg".to_owned(),
            version: "1".to_owned(),
            formats: vec![CadProviderFormatCapability {
                format: CadFormat::Dwg,
                read: true,
                write: false,
                validate: false,
                opaque_preservation: CadProviderOpaquePreservation::Unproven,
                supported_features: Vec::new(),
            }],
            deployment_requirements: Vec::new(),
        }
    }

    /// Counts provider import and returns an empty shared CAD drawing.
    ///
    /// # Parameters
    /// - `format`: Requested source format.
    /// - `_bytes`: Exact bytes scanned by the project route.
    ///
    /// # Returns
    /// Provider-neutral drawing data and provider provenance.
    ///
    /// # Errors
    /// Returns `ReadFailed` when called for a format other than DWG.
    fn import(
        &self,
        format: &CadFormat,
        _bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError> {
        if format != &CadFormat::Dwg {
            return Err(provider_error(CadProviderErrorKind::ReadFailed));
        }
        self.0.fetch_add(1, Ordering::SeqCst);
        CadProviderImport::new(
            CadProviderIdentity {
                id: "http-fixture.dwg".to_owned(),
                version: "1".to_owned(),
            },
            Drawing {
                id: scriberig_core::EntityId::new(),
                units: "unitless".to_owned(),
                coordinate_reference: None,
                layers: Vec::new(),
                entities: Vec::new(),
                blocks: Vec::new(),
                layouts: Vec::new(),
                external_references: Vec::new(),
            },
            Vec::new(),
        )
    }

    /// Refuses rendering because this provider is read-only.
    ///
    /// # Parameters
    /// - `_format`: Requested output format.
    /// - `_drawing`: Candidate shared drawing.
    /// - `_baseline`: Exact original bytes.
    ///
    /// # Returns
    /// No output bytes.
    ///
    /// # Errors
    /// Always returns `ReadOnly`.
    fn render_candidate(
        &self,
        _format: &CadFormat,
        _drawing: &Drawing,
        _baseline: &[u8],
    ) -> Result<Vec<u8>, CadProviderError> {
        Err(provider_error(CadProviderErrorKind::ReadOnly))
    }

    /// Refuses validation because this provider cannot produce candidate output.
    ///
    /// # Parameters
    /// - `_format`: Requested output format.
    /// - `_bytes`: Candidate bytes.
    /// - `_expected`: Expected shared drawing.
    ///
    /// # Returns
    /// No successful validation result.
    ///
    /// # Errors
    /// Always returns `ValidationFailed`.
    fn validate(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
        _expected: &Drawing,
    ) -> Result<(), CadProviderError> {
        Err(provider_error(CadProviderErrorKind::ValidationFailed))
    }
}

/// Creates a provider-neutral categorized error.
///
/// # Parameters
/// - `kind`: Closed provider failure category.
///
/// # Returns
/// Safe provider error without provider-controlled diagnostic text.
fn provider_error(kind: CadProviderErrorKind) -> CadProviderError {
    CadProviderError { kind }
}

/// Confirms HTTP scan/import routes use the application host's provider registry.
#[test]
fn project_routes_import_dwg_with_host_injected_provider() {
    use super::HttpState;
    use super::direct;
    use crate::development::DevelopmentFixture;

    let root = std::env::temp_dir().join(format!(
        "scriberig-http-dwg-{}",
        scriberig_core::EntityId::new()
    ));
    std::fs::create_dir_all(&root).expect("create project directory");
    std::fs::write(root.join("model.dwg"), b"http DWG fixture").expect("write drawing");
    let database = root.with_extension("sqlite");
    let store = Arc::new(scriberig_store_sqlite::SqliteStore::open(&database).expect("database"));
    let imported = Arc::new(AtomicUsize::new(0));
    let registry = CadProviderRegistry::new()
        .register(Arc::new(ImportProvider(Arc::clone(&imported))))
        .expect("register provider");
    let project_service = scriberig_project::ProjectService::new(Arc::clone(&store))
        .with_cad_provider_registry(registry.clone());
    let service = scriberig_app::ApplicationService::new(
        scriberig_app::StoreInspectionBackend::new(Arc::clone(&store)),
    )
    .with_cad_provider_registry(registry)
    .with_project_service(project_service);
    let state = HttpState {
        service,
        principal: scriberig_app::Principal::new("http-dwg-test"),
        fixture: DevelopmentFixture {
            root_entity: scriberig_core::EntityId::new(),
            child_entities: Vec::new(),
            snapshot: scriberig_core::SnapshotId::new([0; 32]),
        },
        allowed_origin: None,
        project_store: Some(Arc::clone(&store)),
        allowed_project_roots: vec![root.clone()],
        plan_lifecycle_handler: None,
        agent_execution_handler: None,
        provider_readiness_handler: None,
    };

    let scan = direct(
        &state,
        &serde_json::json!({
            "operation": "scan",
            "payload": {"path": root, "name": "DWG route fixture"}
        })
        .to_string(),
    )
    .expect("scan DWG through HTTP project route");
    let project = scan["project"]["id"].as_str().expect("project ID");
    assert_eq!(scan["files"][0]["adapter"], "dwg");

    let result = direct(
        &state,
        &serde_json::json!({
            "operation": "import",
            "payload": {"project": project}
        })
        .to_string(),
    )
    .expect("import DWG through HTTP project route");
    assert_eq!(imported.load(Ordering::SeqCst), 1);
    assert_eq!(result["workspace"]["project"]["id"], project);
    assert!(result["entities"].as_u64().unwrap_or_default() > 0);
    let project_id = project.parse().expect("typed project ID");
    assert!(
        store
            .project_entities(project_id)
            .expect("project semantic records")
            .iter()
            .any(|entity| entity.kind == "cad.drawing")
    );

    drop(state);
    drop(store);
    std::fs::remove_file(database).expect("remove test database");
    std::fs::remove_dir_all(root).expect("remove project directory");
}
