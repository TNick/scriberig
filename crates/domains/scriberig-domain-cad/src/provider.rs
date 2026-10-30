//! Defines the optional provider boundary for external CAD formats.

mod import;

pub use import::{CadProviderEntityIdentifier, CadProviderIdentity, CadProviderImport};

use crate::{CadEntity, Drawing};

/// Identifies an external CAD package format.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CadFormat {
    /// Drawing Exchange Format.
    Dxf,
    /// Native Drawing format.
    Dwg,
    /// Provider-defined format identifier.
    Other(String),
}

/// Describes one provider version without exposing native handles or secrets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadProviderDescriptor {
    /// Stable provider identifier.
    pub id: String,
    /// Provider version string.
    pub version: String,
    /// Supported format capabilities.
    pub formats: Vec<CadProviderFormatCapability>,
    /// Closed deployment requirements without paths, keys, or free-form data.
    pub deployment_requirements: Vec<CadProviderDeploymentRequirement>,
}

/// Describes a deployment prerequisite without carrying provider-supplied text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CadProviderDeploymentRequirement {
    /// A provider-owned native runtime must be installed by the host operator.
    NativeRuntimeInstalled,
    /// Licensing must be configured out of band by the host operator.
    LicenseConfiguredOutOfBand,
    /// A provider-owned remote service must be reachable by the host.
    RemoteServiceAvailable,
}

/// Identifies one format feature a provider can preserve or emit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CadProviderFeature {
    /// CAD line entity.
    Line,
    /// CAD polyline entity.
    Polyline,
    /// CAD circle entity.
    Circle,
    /// CAD arc entity.
    Arc,
    /// CAD text entity.
    Text,
    /// CAD dimension entity.
    Dimension,
    /// CAD block-reference entity.
    BlockReference,
    /// Drawing block definitions.
    BlockDefinition,
    /// Drawing layouts.
    Layout,
    /// Layout viewports.
    Viewport,
    /// External drawing references.
    ExternalReference,
}

/// Describes independent read, write, and validation support for one format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadProviderFormatCapability {
    /// Supported external CAD format.
    pub format: CadFormat,
    /// Provider can import supported semantic structures.
    pub read: bool,
    /// Provider can render candidate semantic structures.
    pub write: bool,
    /// Provider can reopen and validate candidate output.
    pub validate: bool,
    /// Provider guarantee for source-significant content outside its semantic model.
    pub opaque_preservation: CadProviderOpaquePreservation,
    /// Provider-supported native classes/features.
    pub supported_features: Vec<CadProviderFeature>,
}

impl CadProviderFormatCapability {
    /// Checks that imported semantics fit the provider's advertised read features.
    ///
    /// # Parameters
    /// - `self`: Capability being checked.
    /// - `drawing`: Provider-imported shared semantic drawing.
    ///
    /// # Returns
    /// Unit when every represented feature is covered by the provider descriptor.
    ///
    /// # Errors
    /// Returns `ReadOnly` when reading is disabled or `UnsupportedFeature` when
    /// imported semantics exceed the advertised feature set.
    pub fn validate_read_support(&self, drawing: &Drawing) -> Result<(), CadProviderError> {
        if !self.read {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::ReadOnly,
            });
        }
        if required_features(drawing)
            .iter()
            .any(|feature| !self.supported_features.contains(feature))
        {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::UnsupportedFeature,
            });
        }
        Ok(())
    }

    /// Checks that advertised write support covers every modeled drawing feature.
    ///
    /// Composite structures also require block-definition, layout, viewport, or
    /// external-reference support when those structures occur.
    ///
    /// # Parameters
    /// - `self`: Capability being checked.
    /// - `drawing`: Candidate semantic drawing.
    ///
    /// # Returns
    /// Unit when the capability can represent every modeled feature in the drawing.
    ///
    /// # Errors
    /// Returns `ReadOnly` when writing is disabled, or `UnsupportedFeature` when any
    /// modeled feature is absent from the provider's advertised feature set.
    pub fn validate_write_support(&self, drawing: &Drawing) -> Result<(), CadProviderError> {
        if !self.write {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::ReadOnly,
            });
        }

        let required = required_features(drawing);
        if required
            .iter()
            .any(|feature| !self.supported_features.contains(feature))
        {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::UnsupportedFeature,
            });
        }
        Ok(())
    }
}

/// Lists every modeled feature present in a shared drawing.
///
/// # Parameters
/// - `drawing`: Shared semantic drawing to inspect.
///
/// # Returns
/// Feature tags required to read or write the drawing.
fn required_features(drawing: &Drawing) -> Vec<CadProviderFeature> {
    let mut required = Vec::new();
    for entity in &drawing.entities {
        required.push(match &entity.geometry {
            CadEntity::Line { .. } => CadProviderFeature::Line,
            CadEntity::Polyline { .. } => CadProviderFeature::Polyline,
            CadEntity::Circle { .. } => CadProviderFeature::Circle,
            CadEntity::Arc { .. } => CadProviderFeature::Arc,
            CadEntity::Text { .. } => CadProviderFeature::Text,
            CadEntity::Dimension { .. } => CadProviderFeature::Dimension,
            CadEntity::BlockReference(_) => CadProviderFeature::BlockReference,
        });
    }
    if !drawing.blocks.is_empty() {
        required.push(CadProviderFeature::BlockDefinition);
    }
    if !drawing.layouts.is_empty() {
        required.push(CadProviderFeature::Layout);
    }
    if drawing
        .layouts
        .iter()
        .any(|layout| !layout.viewports.is_empty())
    {
        required.push(CadProviderFeature::Viewport);
    }
    if !drawing.external_references.is_empty() {
        required.push(CadProviderFeature::ExternalReference);
    }

    required
}

impl CadProviderFormatCapability {
    /// Checks that provider output can be validated and preserves a non-empty baseline.
    ///
    /// # Parameters
    /// - `self`: Capability being checked.
    /// - `drawing`: Exact candidate semantic drawing.
    /// - `baseline`: Original package bytes, empty only for a new export.
    ///
    /// # Returns
    /// Unit when rendering is supported, verifiable, and safe for this baseline.
    ///
    /// # Errors
    /// Returns a categorized error when writing, validation, feature coverage, or
    /// opaque-content preservation is insufficient.
    pub fn validate_render_support(
        &self,
        drawing: &Drawing,
        baseline: &[u8],
    ) -> Result<(), CadProviderError> {
        self.validate_write_support(drawing)?;
        if !self.validate {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::ValidationUnavailable,
            });
        }
        if !baseline.is_empty()
            && self.opaque_preservation != CadProviderOpaquePreservation::Preserved
        {
            return Err(CadProviderError {
                kind: CadProviderErrorKind::OpaquePreservationUnproven,
            });
        }
        Ok(())
    }
}

/// Classifies opaque source content preservation claimed by a CAD provider.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CadProviderOpaquePreservation {
    /// Provider proves unrelated source content survives candidate edits.
    Preserved,
    /// Provider has not established a preservation guarantee.
    Unproven,
    /// Provider is known to discard opaque source content.
    NotPreserved,
}

/// Reports whether an optional provider is installed for one format.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CadProviderStatus {
    /// Provider is present and reports this descriptor.
    Available(CadProviderDescriptor),
    /// No provider is installed for the requested format.
    Unavailable(CadProviderUnavailable),
    /// A configured provider failed before it could report capabilities.
    Failed {
        /// Format whose configured provider failed to initialize.
        format: CadFormat,
        /// Provider-neutral failure category without diagnostic secrets.
        kind: CadProviderErrorKind,
    },
}

/// Describes the normal unavailable state for an optional format provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadProviderUnavailable {
    /// Format for which no provider is present.
    pub format: CadFormat,
    /// Safe human-readable reason without secret deployment material.
    pub reason: String,
}

/// Represents a safe provider operation failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadProviderError {
    /// Provider-neutral failure category that cannot contain provider secrets.
    pub kind: CadProviderErrorKind,
}

/// Classifies provider failures without carrying provider-controlled text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CadProviderErrorKind {
    /// Provider initialization failed.
    InitializationFailed,
    /// Requested input could not be read.
    ReadFailed,
    /// Requested operation uses an unsupported feature.
    UnsupportedFeature,
    /// Provider does not support output generation.
    ReadOnly,
    /// Candidate output could not be generated.
    RenderFailed,
    /// Candidate output did not pass provider validation.
    ValidationFailed,
    /// Provider does not advertise reopening and validating output.
    ValidationUnavailable,
    /// Provider cannot prove preservation of opaque source content for this baseline.
    OpaquePreservationUnproven,
    /// Provider import contains invalid or semantically leaked native identifiers.
    InvalidImportProvenance,
}

/// Imports, renders, and validates CAD data behind a provider-neutral boundary.
pub trait CadExternalProvider: Send + Sync {
    /// Returns provider identity and independent format capabilities.
    ///
    /// The descriptor must not include license keys or secret paths.
    ///
    /// # Parameters
    /// - `self`: Provider being described.
    ///
    /// # Returns
    /// Provider identity and supported capabilities.
    fn descriptor(&self) -> CadProviderDescriptor;

    /// Imports exact package bytes into the shared semantic drawing model.
    ///
    /// # Parameters
    /// - `self`: Provider performing the import.
    /// - `format`: Requested source format.
    /// - `bytes`: Exact source package bytes.
    ///
    /// # Returns
    /// Semantic drawing and provider identifiers in a separate provenance envelope.
    ///
    /// # Errors
    /// Returns a categorized provider failure without provider-controlled diagnostic text.
    fn import(
        &self,
        format: &CadFormat,
        bytes: &[u8],
    ) -> Result<CadProviderImport, CadProviderError>;

    /// Checks advertised capability, renders output, and reopens it for validation.
    ///
    /// # Parameters
    /// - `self`: Provider being called.
    /// - `format`: Requested output format.
    /// - `drawing`: Exact candidate semantic state.
    /// - `baseline`: Exact original package bytes required for preservation.
    ///
    /// # Returns
    /// Candidate package bytes after provider validation.
    ///
    /// # Errors
    /// Returns a categorized error before returning bytes when capabilities are
    /// insufficient, rendering fails, or reopened output fails validation.
    fn render(
        &self,
        format: &CadFormat,
        drawing: &Drawing,
        baseline: &[u8],
    ) -> Result<Vec<u8>, CadProviderError> {
        let descriptor = self.descriptor();
        let capability = descriptor
            .formats
            .iter()
            .find(|capability| capability.format == *format)
            .ok_or(CadProviderError {
                kind: CadProviderErrorKind::ReadOnly,
            })?;
        capability.validate_render_support(drawing, baseline)?;
        let bytes = self.render_candidate(format, drawing, baseline)?;
        self.validate(format, &bytes, drawing)?;
        Ok(bytes)
    }

    /// Generates candidate bytes after the shared render entry point checks capabilities.
    ///
    /// Provider integrations should call [`CadExternalProvider::render`] so unsupported
    /// candidate features are rejected before provider code runs.
    ///
    /// # Parameters
    /// - `self`: Provider rendering the candidate.
    /// - `format`: Requested output format.
    /// - `drawing`: Exact candidate semantic state.
    /// - `baseline`: Exact original package bytes required for preservation.
    ///
    /// # Returns
    /// Candidate package bytes for shared validation/materialization.
    ///
    /// # Errors
    /// Returns a categorized error when the provider cannot preserve or represent the candidate.
    fn render_candidate(
        &self,
        format: &CadFormat,
        drawing: &Drawing,
        baseline: &[u8],
    ) -> Result<Vec<u8>, CadProviderError>;

    /// Reopens candidate bytes and verifies them against candidate semantics.
    ///
    /// # Parameters
    /// - `self`: Provider validating the candidate.
    /// - `format`: Candidate package format.
    /// - `bytes`: Staged candidate package bytes.
    /// - `expected`: Candidate semantic state to verify.
    ///
    /// # Returns
    /// Unit when the provider confirms candidate output.
    ///
    /// # Errors
    /// Returns a categorized error when parsing or semantic validation fails.
    fn validate(
        &self,
        format: &CadFormat,
        bytes: &[u8],
        expected: &Drawing,
    ) -> Result<(), CadProviderError>;
}
