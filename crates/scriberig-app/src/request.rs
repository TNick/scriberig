//! Defines the transport-independent universal application request model.

use crate::DomainOperationRequest;
use scriberig_core::{RootName, SnapshotId};
use scriberig_query::{FindRequest, FollowRequest, InspectRequest, ListRequest, QuerySpec};

/// Resolves an application request to an exact snapshot before execution.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum SnapshotSelector {
    /// Use this exact immutable snapshot.
    Exact(SnapshotId),
    /// Resolve this named root through application state.
    NamedRoot(RootName),
}

/// Identifies a semantic diff request.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DiffRequest {
    /// Earlier exact snapshot.
    pub from: SnapshotId,
    /// Later exact snapshot.
    pub to: SnapshotId,
}

/// Identifies exact candidates for an explicit three-way semantic merge.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MergeRequest {
    /// Exact common candidate base.
    pub base: SnapshotId,
    /// Exact candidate produced by the first dependency.
    pub left: SnapshotId,
    /// Exact candidate produced by the second dependency.
    pub right: SnapshotId,
}

/// Identifies a history request for a semantic target.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct HistoryRequest {
    /// Snapshot or named root from which history is resolved.
    pub snapshot: SnapshotSelector,
}

/// Identifies a rendering request for a semantic target.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RenderRequest {
    /// Exact snapshot context for rendering.
    pub snapshot: SnapshotSelector,
    /// Registered projection/view identifier.
    pub view: String,
    /// How this render selects the exact style it will apply.
    pub style: RenderStyleSelection,
}

/// Why one render is being produced, from the caller's point of view.
///
/// The application layer owns this label so the request model stays independent
/// of the projection contracts crate, and the purpose decides which style
/// sources may contribute.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RenderStylePurpose {
    /// A person is looking at the projection.
    InteractiveView,
    /// The projection is being written out as shared materialization.
    SharedMaterialization,
    /// A named export that may carry personal style.
    ExplicitExport,
}

impl RenderStylePurpose {
    /// Converts this purpose into the projection contract's purpose.
    ///
    /// - `self`: Purpose to convert.
    ///
    /// # Returns
    /// The equivalent projection purpose.
    #[must_use]
    pub fn projection_purpose(self) -> sr_projection::ProjectionPurpose {
        match self {
            Self::InteractiveView => sr_projection::ProjectionPurpose::InteractiveView,
            Self::SharedMaterialization => sr_projection::ProjectionPurpose::SharedMaterialization,
            Self::ExplicitExport => sr_projection::ProjectionPurpose::ExplicitExport,
        }
    }
}

/// How one render selects the exact style it will apply.
///
/// A render never inherits a style from ambient state: the caller either asks the
/// application to resolve one for a stated purpose, supplies one exactly, or
/// carries the value the application already resolved. Providers refuse anything
/// but the resolved case, so no surface can silently invent a style.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum RenderStyleSelection {
    /// Resolve from configuration for one purpose and scope.
    ///
    /// - `projection_kind`: Kind whose domain owns the style meaning.
    /// - `project_id`: Stable project identity whose policy and conventions apply.
    /// - `workspace_id`: Stable workspace identity whose materialization policy applies.
    /// - `purpose`: Why the render is being produced.
    /// - `view_id`: Specific view session a view override may apply to.
    /// - `personal_style`: Personal style named explicitly, export purpose only.
    Configured {
        /// Kind whose domain owns the style meaning.
        projection_kind: String,
        /// Stable project identity whose policy applies.
        project_id: String,
        /// Stable workspace identity whose materialization policy applies.
        workspace_id: String,
        /// Why the render is being produced.
        purpose: RenderStylePurpose,
        /// Specific view session a view override may apply to.
        view_id: Option<String>,
        /// Personal style named explicitly, export purpose only.
        personal_style: Option<Vec<u8>>,
    },
    /// Apply one exact caller-supplied style; configuration is not consulted.
    Exact {
        /// Canonical adapter-owned style bytes.
        value: Vec<u8>,
    },
    /// The exact style the application resolved, with the sources that produced it.
    /// This is the only variant a provider may render with.
    Applied {
        /// Canonical adapter-owned style bytes.
        value: Vec<u8>,
        /// Ordered labels of the sources that contributed.
        provenance: Vec<String>,
    },
}

impl RenderStyleSelection {
    /// Builds a configured selection for one kind, scope, and purpose.
    ///
    /// - `projection_kind`: Kind whose domain owns the style meaning.
    /// - `project_id`: Stable project identity whose policy applies.
    /// - `workspace_id`: Stable workspace identity whose materialization policy applies.
    /// - `purpose`: Why the render is being produced.
    ///
    /// # Returns
    /// The configured selection with no view override or personal style.
    #[must_use]
    pub fn configured(
        projection_kind: impl Into<String>,
        project_id: impl Into<String>,
        workspace_id: impl Into<String>,
        purpose: RenderStylePurpose,
    ) -> Self {
        Self::Configured {
            projection_kind: projection_kind.into(),
            project_id: project_id.into(),
            workspace_id: workspace_id.into(),
            purpose,
            view_id: None,
            personal_style: None,
        }
    }

    /// Builds an exact selection from canonical style bytes.
    ///
    /// - `value`: Canonical adapter-owned style bytes.
    ///
    /// # Returns
    /// The exact selection.
    #[must_use]
    pub fn exact(value: Vec<u8>) -> Self {
        Self::Exact { value }
    }

    /// Reports whether this selection is already an exact resolved style.
    ///
    /// - `self`: Selection to inspect.
    ///
    /// # Returns
    /// `true` only for the resolved variant.
    #[must_use]
    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }

    /// Reads the exact style bytes this selection will render with.
    ///
    /// - `self`: Selection to read.
    ///
    /// # Returns
    /// The exact style bytes, when the selection is already resolved.
    #[must_use]
    pub fn applied_value(&self) -> Option<&[u8]> {
        match self {
            Self::Applied { value, .. } => Some(value),
            Self::Configured { .. } | Self::Exact { .. } => None,
        }
    }
}

/// Identifies a branch operation without implementing branch mutation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct BranchRequest {
    /// Root from which the branch is created.
    pub base: SnapshotSelector,
    /// Requested branch name.
    pub name: RootName,
}

/// Identifies an application change-application request.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ApplyRequest {
    /// Exact base snapshot required by the change.
    pub base_snapshot: SnapshotId,
    /// Optional branch receiving the candidate context.
    pub branch: Option<RootName>,
    /// Stable migration/change handle.
    pub change: String,
}

/// Identifies a commit request for an already validated change.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CommitRequest {
    /// Exact base snapshot being committed.
    pub base_snapshot: SnapshotId,
    /// Named branch whose root is promoted, or `main` when omitted.
    pub branch: Option<RootName>,
    /// Stable migration/change handle.
    pub change: String,
}

/// Selects one universal application operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationRequest {
    /// Inspect an exact semantic target.
    Inspect(InspectRequest),
    /// List a snapshot-bound collection.
    List(ListRequest),
    /// Follow a snapshot-bound relation.
    Follow(FollowRequest),
    /// Discover semantic entities in an exact scope.
    Find(FindRequest),
    /// Evaluate a structural query in an exact scope.
    Query(QuerySpec),
    /// Compute a semantic diff.
    Diff(DiffRequest),
    /// Merge exact candidate snapshots without promoting a root.
    Merge(MergeRequest),
    /// Read semantic history.
    History(HistoryRequest),
    /// Render through a registered view.
    Render(RenderRequest),
    /// Create or update a branch context.
    Branch(BranchRequest),
    /// Apply a proposed change.
    Apply(ApplyRequest),
    /// Commit a validated change.
    Commit(CommitRequest),
    /// Execute a registered exact-target semantic domain operation.
    DomainOperation(DomainOperationRequest),
}
