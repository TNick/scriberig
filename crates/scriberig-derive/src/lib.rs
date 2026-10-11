//! Models derived knowledge without changing canonical semantic snapshots.
//!
//! Derived results may share storage with canonical knowledge, but do not
//! participate in snapshot identity unless explicitly promoted through the
//! canonical change model. Exact reuse is based on declared derivation
//! dependencies and semantics, not entity identity alone or file timestamps.
//! Snapshot identity answers whether canonical scope state changed; derivation
//! keys answer whether a particular result still applies. Completeness claims
//! such as “all” or “none” need explicit scope dependencies. Freshness and
//! retention are independent: external observations can become stale for
//! current requests while remaining historical records, and retention does
//! not establish validity. This crate has no agent dependency and is useful
//! to ordinary library consumers.

/// Defines exact universal dependencies for derivation validity.
pub mod dependency;
/// Defines domain-neutral derived-state lifecycle event values.
pub mod event;
/// Defines generic freshness behavior for derivation registrations.
pub mod freshness;
/// Defines exact derivation request keys.
pub mod key;
/// Defines derivation identities, schedules, and configuration references.
pub mod profile;
/// Defines domain-neutral derivation registration metadata and lookup.
pub mod registry;
/// Defines deterministic exact-profile registry operations.
mod registry_api;
/// Defines generic retention behavior for derivation registrations.
pub mod retention;

// Re-export the stable event, profile, policy, and registration API.
pub use dependency::{Dependency, DependencySet};
pub use event::{DerivationEvent, DerivationEventKind, DerivedResultHandle, EventReason};
pub use freshness::FreshnessPolicy;
pub use key::DerivationKey;
pub use profile::{DerivationProfile, DerivationSchedule};
pub use registry::{
    Applicability, CostHint, DependencyDiscovery, DerivationRegistration, DerivationRegistry,
    ProducerIdentity, RegistrationError,
};
pub use retention::RetentionPolicy;
