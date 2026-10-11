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

pub use branch::Branch;
pub use conflict::{ConflictKind, MergeConflict};
pub use diff::{DiffEnricher, DiffSummary, SemanticDiff, compare};
pub use history::{HistoryGraph, LineageRecord, MigrationId, MigrationReference};
pub use merge::{
    DomainMergeOutcome, DomainMergeStrategy, EntityMergeContext, MergeError, MergeInput,
    MergeResult, MergeStoreError, merge_snapshots, merge_snapshots_into_store,
};
pub use migration::{EntityDelta, Migration, MigrationError};
pub use operation::{CollectionPosition, Operation};
pub use promotion::{PromotionResult, promote, promote_checked};
pub use provenance::{Provenance, SourceKind};
pub use query::{
    QueryPage, branches_descending_from, changed_between, common_ancestor, entity_history,
    introduced_by, merge_conflicts, migrations_touching, page, workspace_migrations,
};
pub use rebase::{
    DomainReplayOutcome, DomainReplayStrategy, RebaseError, RebaseInput, RebaseResult,
    RebasedMigration, Relation, ReplayContext, ReplayState, rebase,
};
pub use summary::{BranchSummary, MergeSummary, MigrationSummary};
pub use workspace::{Workspace, WorkspaceStatus};
