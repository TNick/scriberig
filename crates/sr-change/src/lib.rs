//! Provides the backend-independent change model for Scriberig.
//!
//! Snapshots and revisions are immutable. Change is represented as transitions
//! between snapshots. This crate is backend- and domain-independent.

/// Defines branch metadata and branch references.
pub mod branch;
/// Defines structured semantic conflicts.
pub mod conflict;
/// Defines comparisons between exact snapshots.
pub mod diff;
/// Defines explicit change-history topology.
pub mod history;
/// Defines three-way merge orchestration.
pub mod merge;
/// Defines exact snapshot transitions and entity deltas.
pub mod migration;
/// Defines generic and domain-extensible semantic operations.
pub mod operation;
/// Defines optimistic named-root advancement.
pub mod promotion;
/// Defines source and provenance metadata.
pub mod provenance;
/// Defines history and change queries.
pub mod query;
/// Defines replay against a new base.
pub mod rebase;
/// Defines compact structural summaries.
pub mod summary;
/// Defines isolated speculative work.
pub mod workspace;

pub use migration::{EntityDelta, Migration, MigrationError};
pub use operation::{CollectionPosition, Operation};
pub use provenance::{Provenance, SourceKind};
