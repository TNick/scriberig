//! Verifies configured CAD providers cross the shared application boundary safely.

use std::sync::Arc;

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplicationService, CadProviderRegistry,
};
use scriberig_domain_cad::{
    CadExternalProvider, CadFormat, CadProviderDeploymentRequirement, CadProviderDescriptor,
    CadProviderError, CadProviderErrorKind, CadProviderFeature, CadProviderFormatCapability,
    CadProviderImport, CadProviderOpaquePreservation, CadProviderStatus, Drawing,
};

/// Supplies an initialized fake DWG provider with read-only fidelity.
struct FakeDwgProvider {
    /// Safe or deliberately unsafe provider identifier under test.
    id: &'static str,
    /// Deployment requirement tokens returned by the fake provider.
    deployment_requirements: Vec<CadProviderDeploymentRequirement>,
    /// Supported feature tokens returned by the fake provider.
    supported_features: Vec<CadProviderFeature>,
}

impl CadExternalProvider for FakeDwgProvider {
    /// Returns one safe fake DWG descriptor.
    ///
    /// # Returns
    /// The fake provider's read-only DWG descriptor.
    fn descriptor(&self) -> CadProviderDescriptor {
        CadProviderDescriptor {
            id: self.id.to_owned(),
            version: "1.0".to_owned(),
            formats: vec![CadProviderFormatCapability {
                format: CadFormat::Dwg,
                read: true,
                write: false,
                validate: false,
                opaque_preservation: CadProviderOpaquePreservation::Unproven,
                supported_features: self.supported_features.clone(),
            }],
            deployment_requirements: self.deployment_requirements.clone(),
        }
    }

    /// Rejects import because the registry test does not exercise native parsing.
    ///
    /// # Parameters
    /// - `_format`: Requested CAD format.
    /// - `_bytes`: Exact source bytes.
    ///
    /// # Returns
    /// An import result, always categorized as a read failure in this fixture.
    ///
    /// # Errors
    /// Returns `ReadFailed` because parsing is outside this registry test.
    fn import(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError> {
        Err(CadProviderError {
            kind: CadProviderErrorKind::ReadFailed,
        })
    }

    /// Returns no output because the fixture provider is read-only.
    ///
    /// # Parameters
    /// - `_format`: Requested output format.
    /// - `_drawing`: Candidate semantic drawing.
    /// - `_baseline`: Exact source bytes.
    ///
    /// # Returns
    /// Candidate package bytes when rendering is supported.
    ///
    /// # Errors
    /// Returns `ReadOnly` for this fake provider.
    fn render_candidate(
        &self,
        _format: &CadFormat,
        _drawing: &Drawing,
        _baseline: &[u8],
    ) -> Result<Vec<u8>, CadProviderError> {
        Err(CadProviderError {
            kind: CadProviderErrorKind::ReadOnly,
        })
    }

    /// Accepts no output in this read-only fixture.
    ///
    /// # Parameters
    /// - `_format`: Candidate package format.
    /// - `_bytes`: Staged candidate bytes.
    /// - `_expected`: Expected semantic drawing.
    ///
    /// # Returns
    /// Unit when candidate bytes pass validation.
    ///
    /// # Errors
    /// Returns an error if validation fails.
    fn validate(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
        _expected: &Drawing,
    ) -> Result<(), CadProviderError> {
        Ok(())
    }
}

/// Supplies the minimal backend needed to construct the shared application service.
struct EmptyBackend;

impl ApplicationBackend for EmptyBackend {
    /// Rejects semantic execution because this registry test does not invoke it.
    ///
    /// # Parameters
    /// - `_context`: Trusted application caller context.
    /// - `_request`: Universal operation that was requested.
    ///
    /// # Returns
    /// A result if execution were supported by the test backend.
    ///
    /// # Errors
    /// Always returns unsupported-operation for this test fixture.
    fn execute(
        &self,
        _context: &ApplicationContext,
        _request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        Err(ApplicationError::UnsupportedOperation {
            operation: "registry test".to_owned(),
        })
    }
}

/// Exposes initialized provider metadata while keeping DWG writes disabled.
#[test]
fn application_reports_configured_provider_descriptor_and_fidelity() {
    let registry = CadProviderRegistry::new()
        .register(Arc::new(FakeDwgProvider {
            id: "fixture.dwg",
            deployment_requirements: Vec::new(),
            supported_features: vec![CadProviderFeature::Line],
        }))
        .expect("register provider");
    let app = ApplicationService::new(EmptyBackend).with_cad_provider_registry(registry);

    let CadProviderStatus::Available(descriptor) = app.cad_provider_status(&CadFormat::Dwg) else {
        panic!("configured provider should be available");
    };
    assert_eq!(descriptor.id, "fixture.dwg");
    assert_eq!(descriptor.version, "1.0");
    assert_eq!(descriptor.formats.len(), 1);
    assert!(descriptor.formats[0].read);
    assert!(!descriptor.formats[0].write);
    assert_eq!(
        descriptor.formats[0].opaque_preservation,
        CadProviderOpaquePreservation::Unproven
    );
}

/// Rejects provider identifiers that could smuggle deployment paths to clients.
#[test]
fn registry_rejects_path_like_provider_metadata() {
    let Err(error) = CadProviderRegistry::new().register(Arc::new(FakeDwgProvider {
        id: "/etc/license.key",
        deployment_requirements: Vec::new(),
        supported_features: vec![CadProviderFeature::Line],
    })) else {
        panic!("path-like identifier must be rejected");
    };
    assert_eq!(
        error,
        scriberig_app::CadProviderRegistryError::InvalidDescriptor
    );
}

/// Bounds provider deployment and feature descriptors before they are exposed.
#[test]
fn registry_rejects_unbounded_deployment_and_feature_metadata() {
    let providers = [
        FakeDwgProvider {
            id: "fixture.dwg",
            deployment_requirements: vec![
                CadProviderDeploymentRequirement::LicenseConfiguredOutOfBand;
                129
            ],
            supported_features: vec![CadProviderFeature::Line],
        },
        FakeDwgProvider {
            id: "fixture.dwg",
            deployment_requirements: Vec::new(),
            supported_features: vec![CadProviderFeature::Line; 129],
        },
    ];

    for provider in providers {
        assert!(matches!(
            CadProviderRegistry::new().register(Arc::new(provider)),
            Err(scriberig_app::CadProviderRegistryError::InvalidDescriptor)
        ));
    }
}

/// Keeps configured initialization failure separate from normal provider absence.
#[test]
fn application_reports_provider_initialization_failure_without_secret_text() {
    let registry = CadProviderRegistry::new()
        .register_initialization_failure(CadFormat::Dwg)
        .expect("register initialization failure");
    let app = ApplicationService::new(EmptyBackend).with_cad_provider_registry(registry);

    assert!(matches!(
        app.cad_provider_status(&CadFormat::Dwg),
        CadProviderStatus::Failed {
            format: CadFormat::Dwg,
            kind: CadProviderErrorKind::InitializationFailed,
        }
    ));
    assert!(matches!(
        app.cad_provider_status(&CadFormat::Dxf),
        CadProviderStatus::Unavailable(_)
    ));
}
