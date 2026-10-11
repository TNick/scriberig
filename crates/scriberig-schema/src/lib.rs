//! Defines universal schema and structural validation vocabulary for semantic domains.

pub mod capability;
pub mod collection;
pub mod domain;
pub mod entity;
pub mod property;
pub mod registry;
pub mod relation;
pub mod validation;

pub use capability::{CapabilityCategory, CapabilityDescriptor, CapabilityId, ProviderMetadata};
pub use collection::{CollectionSchema, OrderSemantics};
pub use domain::{DomainDescriptor, DomainId};
pub use entity::{EntityTypeSchema, Schema};
pub use property::{PropertySchema, ValueShape, matches_shape};
pub use registry::{DomainRegistry, RegistryError};
pub use relation::{Cardinality, RelationSchema};
pub use validation::{
    ValidationIssue, ValidationLocation, ValidationReport, ValidationSeverity, ValidationSubject,
    validate,
};
