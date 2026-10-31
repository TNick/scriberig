//! Defines format-independent drawing, geometry, layer, and block semantics.

mod operation;
mod provider;
mod provider_registry;
mod registration;
mod schema;

pub use operation::{CadOperationError, move_entity_to_layer};
pub use provider::{
    CadExternalProvider, CadFormat, CadProviderDeploymentRequirement, CadProviderDescriptor,
    CadProviderEntityIdentifier, CadProviderError, CadProviderErrorKind, CadProviderFeature,
    CadProviderFormatCapability, CadProviderIdentity, CadProviderImport,
    CadProviderOpaquePreservation, CadProviderStatus, CadProviderUnavailable,
};
pub use provider_registry::{CadProviderRegistry, CadProviderRegistryError};
pub use registration::cad_schema;
pub use schema::{
    BlockDefinition, BlockReference, CadEntity, Coordinate, Drawing, DrawingEntity, DrawingSpace,
    EntityStyleRef, ExternalReference, Layer, Layout, Viewport,
};

#[cfg(test)]
mod provider_import_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod tests;
