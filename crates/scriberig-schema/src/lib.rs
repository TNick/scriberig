//! Defines universal schema and structural validation vocabulary for semantic domains.

pub mod capability;
pub mod domain;
pub mod registry;

pub use capability::{CapabilityCategory, CapabilityDescriptor, CapabilityId, ProviderMetadata};
pub use domain::{DomainDescriptor, DomainId};
pub use registry::{DomainRegistry, RegistryError};
