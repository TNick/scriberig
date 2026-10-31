//! Tests provider contracts and deterministic fake implementations.

use scriberig_core::EntityId;

use crate::{
    CadEntity, CadExternalProvider, CadFormat, CadProviderDescriptor, CadProviderError,
    CadProviderErrorKind, CadProviderFeature, CadProviderFormatCapability, CadProviderIdentity,
    CadProviderImport, CadProviderOpaquePreservation, CadProviderStatus, CadProviderUnavailable,
    Coordinate, Drawing, DrawingEntity, DrawingSpace,
};

/// Supplies a deterministic native format provider for boundary checks.
struct ReadOnlyFakeProvider;

impl CadExternalProvider for ReadOnlyFakeProvider {
    /// Reports one read-only native format.
    ///
    /// # Returns
    /// The fake provider's read-only DWG capability.
    fn descriptor(&self) -> CadProviderDescriptor {
        CadProviderDescriptor {
            id: "fake.dwg".to_owned(),
            version: "1".to_owned(),
            formats: vec![CadProviderFormatCapability {
                format: CadFormat::Dwg,
                read: true,
                write: false,
                validate: false,
                opaque_preservation: CadProviderOpaquePreservation::Unproven,
                supported_features: vec![CadProviderFeature::Line],
            }],
            deployment_requirements: Vec::new(),
        }
    }

    /// Imports test bytes into an empty semantic drawing.
    ///
    /// # Parameters
    /// - `_format`: Requested source format.
    /// - `_bytes`: Exact test source bytes.
    ///
    /// # Returns
    /// An empty semantic drawing.
    ///
    /// # Errors
    /// This fake provider does not fail to import.
    fn import(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError> {
        CadProviderImport::new(
            CadProviderIdentity {
                id: "fake.dwg".to_owned(),
                version: "1".to_owned(),
            },
            empty_drawing(),
            Vec::new(),
        )
    }

    /// Refuses candidate generation because this provider is explicitly read-only.
    ///
    /// # Parameters
    /// - `_format`: Requested output format.
    /// - `_drawing`: Candidate semantic drawing.
    /// - `_baseline`: Exact source package bytes.
    ///
    /// # Returns
    /// No bytes because this provider is read-only.
    ///
    /// # Errors
    /// Returns the safe `ReadOnly` category.
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

    /// Refuses candidate validation because it does not support output.
    ///
    /// # Parameters
    /// - `_format`: Candidate output format.
    /// - `_bytes`: Staged candidate bytes.
    /// - `_expected`: Expected candidate semantics.
    ///
    /// # Returns
    /// No success value because output is unsupported.
    ///
    /// # Errors
    /// Returns the safe `ValidationFailed` category.
    fn validate(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
        _expected: &Drawing,
    ) -> Result<(), CadProviderError> {
        Err(CadProviderError {
            kind: CadProviderErrorKind::ValidationFailed,
        })
    }
}

/// Supplies a deterministic provider with write support for line drawings only.
struct LineWriterFakeProvider;

impl CadExternalProvider for LineWriterFakeProvider {
    /// Reports write support only for line geometry.
    ///
    /// # Returns
    /// The fake provider's line-only capability.
    fn descriptor(&self) -> CadProviderDescriptor {
        CadProviderDescriptor {
            id: "fake.line-writer".to_owned(),
            version: "1".to_owned(),
            formats: vec![CadProviderFormatCapability {
                format: CadFormat::Dwg,
                read: true,
                write: true,
                validate: true,
                opaque_preservation: CadProviderOpaquePreservation::Preserved,
                supported_features: vec![CadProviderFeature::Line],
            }],
            deployment_requirements: Vec::new(),
        }
    }

    /// Imports test bytes as an empty drawing.
    ///
    /// # Parameters
    /// - `format`: Requested source format.
    /// - `bytes`: Exact test source bytes.
    ///
    /// # Returns
    /// An empty semantic drawing.
    ///
    /// # Errors
    /// Propagates the read-only fake's structured failure, if any.
    fn import(
        &self,
        format: &CadFormat,
        bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError> {
        ReadOnlyFakeProvider.import(format, bytes)
    }

    /// Renders supported line drawings into deterministic test bytes.
    ///
    /// # Parameters
    /// - `_format`: Requested output format.
    /// - `_drawing`: Candidate semantic drawing.
    /// - `baseline`: Exact source package bytes.
    ///
    /// # Returns
    /// Deterministic fake package bytes.
    ///
    /// # Errors
    /// Returns a safe render failure for the dedicated failure fixture.
    fn render_candidate(
        &self,
        _format: &CadFormat,
        _drawing: &Drawing,
        baseline: &[u8],
    ) -> Result<Vec<u8>, CadProviderError> {
        if baseline == b"render-failure" {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::RenderFailed,
            });
        }
        Ok(if baseline == b"validation-failure" {
            b"reject-validation".to_vec()
        } else {
            b"fake-dwg".to_vec()
        })
    }

    /// Accepts generated test bytes for supported line drawings.
    ///
    /// # Parameters
    /// - `_format`: Candidate output format.
    /// - `bytes`: Staged candidate bytes.
    /// - `_expected`: Expected candidate semantics.
    ///
    /// # Returns
    /// Unit when the fake output is accepted.
    ///
    /// # Errors
    /// This fake validator does not fail.
    fn validate(
        &self,
        _format: &CadFormat,
        bytes: &[u8],
        _expected: &Drawing,
    ) -> Result<(), CadProviderError> {
        if bytes == b"reject-validation" {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::ValidationFailed,
            });
        }
        Ok(())
    }
}

/// Keeps provider error data structural so license material cannot leak in diagnostics.
#[test]
fn provider_errors_contain_only_safe_categories() {
    let error = CadProviderError {
        kind: CadProviderErrorKind::InitializationFailed,
    };
    assert_eq!(error.kind, CadProviderErrorKind::InitializationFailed);
}

/// Ensures fake read/write capability advertises its supported feature set.
#[test]
fn fake_writer_advertises_write_only_for_supported_features() {
    let descriptor = LineWriterFakeProvider.descriptor();
    let capability = &descriptor.formats[0];
    assert!(capability.read && capability.write && capability.validate);
    assert_eq!(capability.supported_features, [CadProviderFeature::Line]);
    assert_eq!(
        capability.opaque_preservation,
        CadProviderOpaquePreservation::Preserved
    );
    assert!(capability.validate_write_support(&empty_drawing()).is_ok());
    let mut unsupported = empty_drawing();
    unsupported.entities.push(DrawingEntity {
        id: EntityId::new(),
        layer: EntityId::new(),
        space: DrawingSpace::Model,
        geometry: CadEntity::Circle {
            center: Coordinate::new("0", "0", "0"),
            radius: "1".to_owned(),
        },
        style: None,
    });
    assert_eq!(
        LineWriterFakeProvider
            .render(&CadFormat::Dwg, &unsupported, b"baseline")
            .unwrap_err()
            .kind,
        CadProviderErrorKind::UnsupportedFeature
    );
}

/// Rejects imported semantics the provider did not advertise as readable.
#[test]
fn provider_import_cannot_exceed_advertised_read_features() {
    let capability = ReadOnlyFakeProvider.descriptor().formats.remove(0);
    let mut unsupported = empty_drawing();
    unsupported.entities.push(DrawingEntity {
        id: EntityId::new(),
        layer: EntityId::new(),
        space: DrawingSpace::Model,
        geometry: CadEntity::Circle {
            center: Coordinate::new("0", "0", "0"),
            radius: "1".to_owned(),
        },
        style: None,
    });

    assert_eq!(
        capability
            .validate_read_support(&unsupported)
            .unwrap_err()
            .kind,
        CadProviderErrorKind::UnsupportedFeature
    );
}

/// Blocks rendering when output validation or baseline preservation is unproven.
#[test]
fn provider_render_requires_validation_and_opaque_baseline_preservation() {
    let mut capability = LineWriterFakeProvider.descriptor().formats.remove(0);
    assert!(
        capability
            .validate_render_support(&empty_drawing(), b"baseline")
            .is_ok()
    );

    capability.validate = false;
    assert_eq!(
        capability
            .validate_render_support(&empty_drawing(), b"baseline")
            .unwrap_err()
            .kind,
        CadProviderErrorKind::ValidationUnavailable
    );

    capability.validate = true;
    capability.opaque_preservation = CadProviderOpaquePreservation::Unproven;
    assert_eq!(
        capability
            .validate_render_support(&empty_drawing(), b"baseline")
            .unwrap_err()
            .kind,
        CadProviderErrorKind::OpaquePreservationUnproven
    );
    assert!(
        capability
            .validate_render_support(&empty_drawing(), b"")
            .is_ok()
    );
}

/// Returns no candidate bytes when a provider rejects its reopened output.
#[test]
fn provider_render_reopens_candidate_and_propagates_validation_failure() {
    assert_eq!(
        LineWriterFakeProvider
            .render(&CadFormat::Dwg, &empty_drawing(), b"validation-failure")
            .unwrap_err()
            .kind,
        CadProviderErrorKind::ValidationFailed
    );
}

/// Returns no candidate bytes when provider rendering fails before validation.
#[test]
fn provider_render_failure_returns_no_candidate_bytes() {
    assert_eq!(
        LineWriterFakeProvider
            .render(&CadFormat::Dwg, &empty_drawing(), b"render-failure")
            .unwrap_err()
            .kind,
        CadProviderErrorKind::RenderFailed
    );
}

/// Builds an empty semantic drawing for provider capability tests.
///
/// # Returns
/// An empty drawing with no layers, entities, or compound structures.
fn empty_drawing() -> Drawing {
    Drawing {
        id: EntityId::new(),
        units: "millimeter".to_owned(),
        coordinate_reference: None,
        layers: Vec::new(),
        entities: Vec::new(),
        blocks: Vec::new(),
        layouts: Vec::new(),
        external_references: Vec::new(),
    }
}

/// Makes provider absence discoverable and read/write capabilities distinct.
#[test]
fn optional_provider_status_and_read_only_capability_are_explicit() {
    let unavailable = CadProviderStatus::Unavailable(CadProviderUnavailable {
        format: CadFormat::Dwg,
        reason: "no DWG provider installed".to_owned(),
    });
    assert!(matches!(unavailable, CadProviderStatus::Unavailable(_)));
    let descriptor = ReadOnlyFakeProvider.descriptor();
    assert!(descriptor.formats[0].read);
    assert!(!descriptor.formats[0].write);
    assert_eq!(
        descriptor.formats[0].supported_features,
        [CadProviderFeature::Line]
    );
    assert!(
        ReadOnlyFakeProvider
            .render(
                &CadFormat::Dwg,
                ReadOnlyFakeProvider
                    .import(&CadFormat::Dwg, b"test")
                    .unwrap()
                    .drawing(),
                b"test"
            )
            .is_err()
    );
}

/// Keeps provider initialization failure distinct from normal provider absence.
#[test]
fn provider_initialization_failure_is_structured_and_secret_free() {
    let status = CadProviderStatus::Failed {
        format: CadFormat::Dwg,
        kind: CadProviderErrorKind::InitializationFailed,
    };

    assert!(matches!(
        status,
        CadProviderStatus::Failed {
            format: CadFormat::Dwg,
            kind: CadProviderErrorKind::InitializationFailed,
        }
    ));
}
