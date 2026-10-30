//! Verifies injected DWG providers participate in shared project onboarding.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_domain_cad::{
    BlockDefinition, CadEntity, CadExternalProvider, CadFormat, CadProviderDescriptor,
    CadProviderEntityIdentifier, CadProviderError, CadProviderErrorKind, CadProviderFeature,
    CadProviderFormatCapability, CadProviderIdentity, CadProviderImport,
    CadProviderOpaquePreservation, Coordinate, Drawing, DrawingEntity, DrawingSpace,
    ExternalReference, Layer, Layout, Viewport,
};
/// Supplies deterministic read-only DWG semantics for the project adapter.
pub(super) struct FakeDwgProvider {
    /// Safe provider ID written to import provenance.
    pub(super) provider_id: &'static str,
    /// Safe provider version written to import provenance.
    pub(super) provider_version: &'static str,
    /// Optional deliberately mismatched identity returned only for a boundary test.
    pub(super) import_identity_override: Option<(&'static str, &'static str)>,
    /// Provider-native handle returned for the first semantic entity.
    pub(super) native_handle: &'static str,
    /// Whether the fake emits a changed line for structural reconciliation tests.
    pub(super) change_line_geometry: bool,
    /// Whether the provider advertises semantic import for DWG.
    pub(super) read: bool,
    /// Number of provider source objects intentionally omitted from the fixture.
    pub(super) unmapped_object_count: usize,
    /// Number of import calls made by the project adapter.
    pub(super) import_calls: Arc<AtomicUsize>,
}

impl CadExternalProvider for FakeDwgProvider {
    /// Reports provider identity and read-only DWG capability.
    ///
    /// # Returns
    /// A provider-neutral fake DWG descriptor.
    fn descriptor(&self) -> CadProviderDescriptor {
        CadProviderDescriptor {
            id: self.provider_id.to_owned(),
            version: self.provider_version.to_owned(),
            formats: vec![CadProviderFormatCapability {
                format: CadFormat::Dwg,
                read: self.read,
                write: false,
                validate: false,
                opaque_preservation: CadProviderOpaquePreservation::Unproven,
                supported_features: vec![
                    CadProviderFeature::Line,
                    CadProviderFeature::Text,
                    CadProviderFeature::BlockDefinition,
                    CadProviderFeature::Layout,
                    CadProviderFeature::Viewport,
                    CadProviderFeature::ExternalReference,
                ],
            }],
            deployment_requirements: Vec::new(),
        }
    }

    /// Imports one deterministic line while retaining its provider identifier separately.
    ///
    /// # Parameters
    /// - `format`: Requested format, which is DWG for this fixture.
    /// - `_bytes`: Exact package bytes owned by the project source record.
    ///
    /// # Returns
    /// Shared drawing semantics and detached native-handle provenance.
    ///
    /// # Errors
    /// Returns a structured read error for a non-DWG request.
    fn import(
        &self,
        format: &CadFormat,
        _bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError> {
        self.import_calls.fetch_add(1, Ordering::SeqCst);
        if format != &CadFormat::Dwg {
            return Err(provider_error(CadProviderErrorKind::ReadFailed));
        }
        let drawing_id = scriberig_core::EntityId::from_bytes([1; 16]);
        let layer_id = scriberig_core::EntityId::from_bytes([2; 16]);
        let entity_id = scriberig_core::EntityId::from_bytes([3; 16]);
        let drawing = Drawing {
            id: drawing_id,
            units: "millimeter".to_owned(),
            coordinate_reference: None,
            layers: vec![Layer {
                id: layer_id,
                name: "Roads".to_owned(),
                visible: true,
                locked: false,
            }],
            entities: vec![
                DrawingEntity {
                    id: entity_id,
                    layer: layer_id,
                    space: DrawingSpace::Model,
                    geometry: if self.change_line_geometry {
                        CadEntity::Line {
                            start: Coordinate::new("100", "0", "0"),
                            end: Coordinate::new("110", "0", "0"),
                        }
                    } else {
                        CadEntity::Line {
                            start: Coordinate::new("0", "0", "0"),
                            end: Coordinate::new("10", "0", "0"),
                        }
                    },
                    style: None,
                },
                DrawingEntity {
                    id: scriberig_core::EntityId::from_bytes([8; 16]),
                    layer: layer_id,
                    space: DrawingSpace::Model,
                    geometry: CadEntity::Text {
                        position: Coordinate::new("2", "3", "0"),
                        value: "North Gate".to_owned(),
                        height: "2.5".to_owned(),
                    },
                    style: None,
                },
            ],
            blocks: vec![BlockDefinition {
                id: scriberig_core::EntityId::from_bytes([4; 16]),
                name: "RoadMarker".to_owned(),
                entities: vec![entity_id],
            }],
            layouts: vec![Layout {
                id: scriberig_core::EntityId::from_bytes([5; 16]),
                name: "Sheet A".to_owned(),
                viewports: vec![Viewport {
                    id: scriberig_core::EntityId::from_bytes([6; 16]),
                    center: Coordinate::new("5", "5", "0"),
                    height: "50".to_owned(),
                }],
            }],
            external_references: vec![ExternalReference {
                id: scriberig_core::EntityId::from_bytes([7; 16]),
                target: "survey.dwg".to_owned(),
                drawing: None,
            }],
        };
        let (provider_id, provider_version) = self
            .import_identity_override
            .unwrap_or((self.provider_id, self.provider_version));
        CadProviderImport::new(
            CadProviderIdentity {
                id: provider_id.to_owned(),
                version: provider_version.to_owned(),
            },
            drawing,
            vec![CadProviderEntityIdentifier {
                entity: entity_id,
                value: self.native_handle.to_owned(),
            }],
        )
        .map(|imported| imported.with_unmapped_object_count(self.unmapped_object_count))
    }

    /// Refuses candidate rendering because this fixture advertises read-only support.
    ///
    /// # Parameters
    /// - `_format`: Requested format.
    /// - `_drawing`: Candidate shared semantic drawing.
    /// - `_baseline`: Exact original package bytes.
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

    /// Accepts no candidate output for the read-only fixture.
    ///
    /// # Parameters
    /// - `_format`: Candidate package format.
    /// - `_bytes`: Candidate package bytes.
    /// - `_expected`: Expected semantic drawing.
    ///
    /// # Returns
    /// Unit when validation succeeds.
    ///
    /// # Errors
    /// Returns `ValidationFailed` because this provider has no write capability.
    fn validate(
        &self,
        _format: &CadFormat,
        _bytes: &[u8],
        _expected: &Drawing,
    ) -> Result<(), CadProviderError> {
        Err(provider_error(CadProviderErrorKind::ValidationFailed))
    }
}

/// Creates a provider-neutral categorized failure.
///
/// # Parameters
/// - `kind`: Closed provider error category.
///
/// # Returns
/// Safe provider error with no provider-controlled message.
fn provider_error(kind: CadProviderErrorKind) -> CadProviderError {
    CadProviderError { kind }
}

#[path = "dwg_provider/import_tests.rs"]
mod import_tests;
