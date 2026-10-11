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
//! to ordinary library consumers. A `SnapshotId` answers whether canonical
//! scope state changed; a `DerivationKey` answers whether a specific result
//! still applies. Queries asserting “all”, “none”, or completeness require a
//! dependency on the relevant scope so newly matching items invalidate the
//! materialization. None of these derived operations advances snapshots.

/// Defines exact universal dependencies for derivation validity.
pub mod dependency;
/// Defines domain-neutral derived-state lifecycle event values.
pub mod event;
/// Explains exact key matches and dependency mismatches.
pub mod explain;
/// Defines generic freshness behavior for derivation registrations.
pub mod freshness;
/// Finds results affected by explicit semantic dependency changes.
pub mod invalidation;
/// Defines exact derivation request keys.
pub mod key;
/// Defines derivation identities, schedules, and configuration references.
pub mod profile;
/// Defines domain-neutral derivation registration metadata and lookup.
pub mod registry;
/// Defines deterministic exact-profile registry operations.
mod registry_api;
/// Defines the immutable derived-result repository contract.
pub mod repository;
/// Defines immutable result records and structured status.
pub mod result;
/// Defines generic retention behavior for derivation registrations.
pub mod retention;
/// Provides a usable in-memory repository and cache resolution runtime.
pub mod runtime;
/// Selects among existing immutable result records.
pub mod selection;

// Re-export the stable event, profile, policy, and registration API.
pub use dependency::{Dependency, DependencySet};
pub use event::{DerivationEvent, DerivationEventKind, DerivedResultHandle, EventReason};
pub use explain::{ExplanationReason, ReuseExplanation, explain_key_comparison};
pub use freshness::{FreshnessObservation, FreshnessPolicy, FreshnessState, evaluate_freshness};
pub use invalidation::{SemanticChange, affected_transitively, is_affected};
pub use key::DerivationKey;
pub use profile::{DerivationProfile, DerivationSchedule};
pub use registry::{
    Applicability, CostHint, DependencyDiscovery, DerivationRegistration, DerivationRegistry,
    ProducerIdentity, RegistrationError,
};
pub use repository::{DerivedRepository, RepositoryError};
pub use result::{DerivedResultRecord, DerivedResultStatus, FailureReason};
pub use retention::{RetentionPolicy, RetentionStrength};
pub use runtime::{DerivedRuntime, InMemoryDerivedRepository, Resolution, RuntimeError};
pub use selection::{SelectionOutcome, SelectionPolicy, SelectionReason, select};
