//! Defines work requested by the reducer but executed by the runtime.

/// A presentation or future semantic effect emitted by state reduction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Effect {
    /// No work is required outside the reducer.
    None,
    /// Ask the owning service adapter to refresh presentation data.
    RequestRefresh,
    /// Ask the owning service adapter to execute a find request.
    RequestFind {
        /// Monotonic request identifier.
        request_id: u64,
        /// Query text supplied by the user.
        text: String,
        /// Exact snapshot scope for discovery.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Ask the owning service adapter to execute a contextual action later.
    RequestSemanticAction {
        /// Stable semantic action identifier.
        id: String,
        /// Explicit action inputs.
        inputs: Vec<(String, String)>,
    },
    /// Request runtime termination.
    RequestQuit,
    /// Request semantic inspection through the TUI service port.
    InspectEntity {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity to inspect.
        entity: scriberig_core::EntityId,
        /// Optional exact snapshot.
        snapshot: Option<scriberig_core::SnapshotId>,
    },
    /// Request the bounded semantic children of one snapshot entity.
    ListExplorer {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity whose snapshot collection is listed.
        entity: scriberig_core::EntityId,
        /// Exact snapshot containing the collection.
        snapshot: scriberig_core::SnapshotId,
        /// Opaque continuation for the next page.
        continuation: Option<scriberig_query::ContinuationToken>,
    },
    /// Read one bounded page of logical artifacts for an exact project.
    InspectArtifactPage {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable project identity.
        project: scriberig_core::ProjectId,
        /// Zero-based artifact offset.
        offset: usize,
        /// Maximum number of summaries to return.
        limit: usize,
    },
    /// Read a bounded page of members for one exact artifact.
    InspectArtifactMembers {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Stable semantic artifact identity.
        artifact: scriberig_core::EntityId,
        /// Zero-based member offset.
        offset: usize,
        /// Maximum number of descriptors to return.
        limit: usize,
    },
    /// Read one exact-snapshot CAD drawing and a bounded entity page.
    InspectCadPage {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project and drawing identity.
        drawing: scriberig_protocol::CadDrawingInspectRequest,
        /// Exact drawing scope and entity page bounds.
        entities: scriberig_protocol::CadEntitiesRequest,
    },
    /// Propose an exact-base CAD entity layer change through shared semantics.
    ProposeCadLayerChange {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, entity, and destination layer.
        request: scriberig_protocol::CadLayerChangeRequest,
    },
    /// Read a bounded semantic worksheet page at an exact snapshot.
    InspectSpreadsheetPage {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Exact immutable snapshot to inspect.
        snapshot: scriberig_core::SnapshotId,
        /// Stable semantic worksheet identity.
        worksheet: scriberig_core::EntityId,
        /// Optional inclusive worksheet coordinate bounds.
        range: Option<scriberig_domain_spreadsheet::CellRange>,
        /// Zero-based sparse-cell page offset.
        offset: usize,
        /// Maximum number of cells to return.
        limit: usize,
    },
    /// Inspect one exact worksheet coordinate.
    InspectSpreadsheetCell {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, worksheet, and coordinate.
        request: scriberig_app::SpreadsheetCellQuery,
    },
    /// Propose one spreadsheet value/formula change as a shared candidate.
    ProposeSpreadsheetEdit {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, worksheet, coordinate, and authored value.
        request: scriberig_app::SpreadsheetCellEditQuery,
    },
    /// Read an exact-snapshot semantic document tree.
    InspectDocument {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, and document scope.
        request: scriberig_app::DocumentTreeQuery,
    },
    /// Propose one document paragraph replacement through the shared API.
    ProposeDocumentParagraphEdit {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, base snapshot, target paragraph, and replacement text.
        request: scriberig_app::DocumentParagraphEditQuery,
    },
    /// Propose one document table-cell replacement through the shared API.
    ProposeDocumentTableCellEdit {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, base snapshot, target cell, and replacement text.
        request: scriberig_app::DocumentTableCellEditQuery,
    },
    /// Read a bounded exact-snapshot `GeoPackage` layer catalog.
    InspectGeoCatalog {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Exact project-relative source path.
        path: String,
        /// Exact immutable semantic snapshot.
        snapshot: scriberig_core::SnapshotId,
        /// Zero-based layer offset.
        offset: usize,
    },
    /// Read an exact-snapshot QGIS project and resolved layer references.
    InspectQgisProject {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project and source path.
        request: scriberig_app::QgisProjectQuery,
    },
    /// Propose an exact-snapshot QGIS layer-reference rename.
    ProposeQgisLayerRename {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, reference, and new display name.
        request: scriberig_protocol::QgisLayerRenameRequest,
    },
    /// Propose an exact-snapshot QGIS project layer order.
    ProposeQgisLayerOrder {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, snapshot, path, and complete layer order.
        request: scriberig_protocol::QgisLayerOrderRequest,
    },
    /// Set or clear the caller's personal projection-style preference.
    SetProjectionStylePreference {
        /// Monotonic request generation.
        request_id: u64,
        /// User-scoped setting write or clear request.
        request: scriberig_protocol::ProjectionStyleSettingRequest,
    },
    /// Resolve the exact effective Rust source style for the selected view.
    ResolveProjectionStyle {
        /// Monotonic request generation.
        request_id: u64,
        /// Project, workspace, view, and purpose used by the shared resolver.
        request: scriberig_protocol::ProjectionStyleResolveRequest,
    },
    /// Read a bounded exact-snapshot `GeoPackage` feature page.
    InspectGeoFeatures {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Exact project-relative source path.
        path: String,
        /// Exact immutable semantic snapshot.
        snapshot: scriberig_core::SnapshotId,
        /// Stable semantic layer identity.
        layer: scriberig_core::EntityId,
        /// Zero-based feature offset.
        offset: usize,
    },
    /// Execute a bounded typed geospatial query at an exact snapshot.
    QueryGeoFeatures {
        /// Monotonic request generation.
        request_id: u64,
        /// Typed exact-snapshot query request.
        request: scriberig_app::GeoFeatureQuery,
    },
    /// Propose a typed exact-base feature attribute change through shared semantics.
    ProposeGeoAttributeEdit {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, base, layer, feature, field, and replacement value.
        edit: scriberig_project::GeoPackageFeatureAttributeEdit,
    },
    /// Propose a typed exact-base feature geometry change through shared semantics.
    ProposeGeoGeometryEdit {
        /// Monotonic request generation.
        request_id: u64,
        /// Exact project, path, snapshot, layer, feature, and geometry.
        request: scriberig_protocol::GeoGeometryEditRequest,
    },
    /// Request the current runtime capability descriptors.
    RequestCapabilities,
    /// Read exact semantic history.
    History {
        /// Request generation.
        request_id: u64,
        /// Exact snapshot context.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Compute a structured two-sided semantic diff.
    Diff {
        /// Request generation.
        request_id: u64,
        /// Public exact diff request.
        request: scriberig_app::DiffRequest,
    },
    /// Render through a shared projection operation.
    Render {
        /// Request generation.
        request_id: u64,
        /// Public render request.
        request: scriberig_app::RenderRequest,
    },
    /// Apply a candidate change without promotion.
    Apply {
        /// Request generation.
        request_id: u64,
        /// Public apply request.
        request: scriberig_app::ApplyRequest,
    },
    /// Commit a reviewed change with its exact base context.
    Commit {
        /// Request generation.
        request_id: u64,
        /// Public commit request.
        request: scriberig_app::CommitRequest,
    },
    /// Read the authoritative lifecycle record for one operation.
    OperationStatus {
        /// Operation identity to inspect.
        operation: scriberig_core::OperationId,
    },
    /// Ask the application to cancel one operation.
    CancelOperation {
        /// Operation identity to cancel.
        operation: scriberig_core::OperationId,
    },
    /// Refresh the tracked application operation records.
    ListOperations,
    /// Read a public session tree.
    ReadSession {
        /// Request generation.
        request_id: u64,
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional branch whose activity is loaded on demand.
        branch: Option<scriberig_core::EntityId>,
    },
    /// List the caller's durable conversation sessions.
    ListSessions,
    /// Read and select one exact durable session.
    SwitchSession {
        /// Stable session identity to load.
        session: scriberig_core::EntityId,
    },
    /// Remove one exact idle durable session.
    RemoveSession {
        /// Stable session identity to remove.
        session: scriberig_core::EntityId,
    },
    /// Read safe upstream-derived model display rows.
    ReadModelLibrary {
        /// Request generation.
        request_id: u64,
    },
    /// Read shared presentation defaults for this trusted application user.
    ReadPresentationPreferences,
    /// Read expanded safe detail for one durable tool result.
    ReadToolResultDetail {
        /// Session containing the result.
        session: scriberig_core::EntityId,
        /// Branch containing the result.
        branch: scriberig_core::EntityId,
        /// Stable tool-result identity.
        result: scriberig_core::EntityId,
    },
    /// Read captured safe request detail for one model invocation.
    ReadModelRequestDetail {
        /// Session containing the invocation.
        session: scriberig_core::EntityId,
        /// Branch containing the invocation.
        branch: scriberig_core::EntityId,
        /// Stable model-invocation identity.
        invocation: scriberig_core::EntityId,
    },
    /// List preserved legacy execution choices for the active session.
    LegacyRecoveryList {
        /// Session whose sanitized recovery records are requested.
        session: scriberig_core::EntityId,
        /// Session-selection revision to use for an `/execution` record.
        selection_revision: u64,
        /// Current plan identities and revisions visible to the operator.
        plan_revisions: Vec<(scriberig_core::EntityId, u64)>,
    },
    /// Map one preserved legacy execution choice to an active upstream model.
    LegacyRecoveryMap {
        /// Session containing the preserved choice.
        session: scriberig_core::EntityId,
        /// Exact path returned by the recovery listing.
        path: String,
        /// Selection or plan revision observed by the operator.
        expected_revision: u64,
        /// Active upstream provider identity supplied by the operator.
        provider_id: String,
        /// Provider-scoped model identity supplied by the operator.
        model_id: String,
        /// Optional advertised thinking level supplied by the operator.
        thinking_level: Option<String>,
    },
    /// Select a model profile for future invocations in one session.
    SetSessionModel(scriberig_app::SessionModelSelectionRequest),
    /// Select a thinking level for future invocations in one session.
    SetSessionThinkingLevel(scriberig_app::SessionThinkingSelectionRequest),
    /// Submit a command to a session branch through the application boundary.
    SubmitAgent {
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional target branch.
        branch: Option<scriberig_core::EntityId>,
        /// User-authored command text.
        prompt: String,
    },
    /// Starts a new session through shared application semantics.
    StartNewConversation {
        /// First user-authored instruction.
        prompt: String,
        /// Exact semantic project selection for the initial model context.
        selection: Option<scriberig_app::AgentSelection>,
    },
    /// Continues at one exact durable conversation anchor.
    ContinueConversation {
        /// Owning session identity.
        session: scriberig_core::EntityId,
        /// Stable semantic continuation target.
        anchor: scriberig_session::ContinuationAnchor,
        /// User-authored follow-up instruction.
        prompt: String,
    },
    /// Creates a durable plan through the shared application plan operation.
    CreatePlan(scriberig_app::PlanCreateRequest),
    /// Applies a complete structured revision to a durable plan.
    Replan(scriberig_app::PlanReplanRequest),
    /// Changes a durable plan's lifecycle through shared application semantics.
    PlanLifecycle {
        /// Plan identity and expected durable revision.
        request: scriberig_app::PlanLifecycleRequest,
        /// Requested lifecycle transition.
        action: scriberig_app::PlanLifecycleAction,
    },
    /// Applies one typed plan-domain mutation through shared application semantics.
    PlanMutation(scriberig_app::PlanMutationRequest),
    /// Changes one plan node's credential-safe model-library assignment.
    SetPlanNodeAssignment(scriberig_app::PlanAssignmentRequest),
    /// Run the deterministic semantic project agent for the selected entity.
    ProjectAgent {
        /// Monotonic request identifier.
        request_id: u64,
        /// Selected semantic entity.
        entity: scriberig_core::EntityId,
        /// Exact base snapshot.
        snapshot: scriberig_core::SnapshotId,
        /// User instruction sent to the provider.
        prompt: String,
    },
}
