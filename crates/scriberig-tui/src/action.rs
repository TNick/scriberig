//! Defines typed local interaction intents for the TUI reducer.

/// Direction for cursor movement in a list-like presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Direction {
    /// Move toward the previous item.
    Previous,
    /// Move toward the next item.
    Next,
}

/// A typed presentation action independent of terminal or service handles.
#[derive(Clone, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Action {
    /// Records that an effect had no stateful result.
    Noop,
    /// Displays a safe result from an explicit semantic command.
    StatusMessage(String),
    /// Replaces the multiline legacy recovery listing in semantic views.
    RecoveryRecordsLoaded(String),
    /// Request normal runtime termination.
    QuitRequested,
    /// Close the active overlay, if any.
    OverlayClosed,
    /// Move a local cursor.
    CursorMoved(Direction),
    /// Activate the focused row or overlay item.
    Activated,
    /// Navigate to the previous semantic presentation location.
    Back,
    /// Navigate to the next semantic presentation location.
    Forward,
    /// Request a presentation refresh.
    RefreshRequested,
    /// Open the help overlay.
    HelpOpened,
    /// Open the local command palette.
    PaletteOpened,
    /// Open the find input overlay.
    FindOpened,
    /// Open the exact snapshot chooser for semantic comparison.
    DiffChooserOpened,
    /// Open a source projection for the exact selected snapshot.
    ProjectionOpened,
    /// Update the active text input.
    InputChanged(String),
    /// Submit the active text input.
    InputSubmitted,
    /// Applies an accepted conversation submission with authoritative handles.
    ConversationAgentAccepted(crate::service::AgentHandle),
    /// Enters or leaves conversation-tree browse mode.
    ConversationBrowsing(bool),
    /// Pins the current navigable conversation source as the reply target.
    ConversationPinCursor,
    /// Resumes local follow of the shared plan coordinator cursor.
    ConversationFollowCoordinator,
    /// Expands or collapses the currently selected branch in the local tree.
    ConversationToggleExpansion,
    /// Insert one paste payload into the active input.
    Paste(String),
    /// Activate a concrete semantic location.
    ActivateLocation(crate::selection::NavigationLocation),
    /// Mark an inspect request as outstanding.
    InspectRequested {
        /// Monotonic request identifier.
        request_id: u64,
        /// Entity being inspected.
        entity: scriberig_core::EntityId,
        /// Optional exact snapshot.
        snapshot: Option<scriberig_core::SnapshotId>,
        /// Whether this request should switch the primary view to Overview.
        switch_view: bool,
    },
    /// Apply a successful inspect result.
    InspectSucceeded {
        /// Request that produced the result.
        request_id: u64,
        /// Presentation projection of the result.
        model: crate::inspector::InspectorModel,
    },
    /// Apply a failed inspect result.
    InspectFailed {
        /// Request that failed.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Applies a successful bounded Explorer listing.
    ExplorerSucceeded {
        /// Request that produced the rows.
        request_id: u64,
        /// Snapshot-bound semantic rows.
        rows: Vec<crate::explorer::ExplorerRow>,
        /// Opaque continuation for another bounded page.
        next: Option<scriberig_query::ContinuationToken>,
    },
    /// Requests a bounded logical artifact page for one stable project identity.
    ArtifactsRequested {
        /// Project whose persisted scan is summarized.
        project: scriberig_core::ProjectId,
        /// Zero-based artifact offset.
        offset: usize,
    },
    /// Applies a successful bounded artifact page.
    ArtifactsLoaded {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Terminal-safe artifact metadata.
        model: crate::view::artifacts::ArtifactPageModel,
    },
    /// Applies a failed artifact-page request.
    ArtifactsFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a bounded page of members from one exact artifact.
    ArtifactMembersRequested {
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Stable semantic artifact identity.
        artifact: scriberig_core::EntityId,
        /// Zero-based member offset.
        offset: usize,
    },
    /// Applies one successful exact artifact-member page.
    ArtifactMembersLoaded {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Terminal-safe exact member page.
        model: crate::view::ArtifactMembersModel,
    },
    /// Applies one failed artifact-member request.
    ArtifactMembersFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests the selected semantic CAD drawing and an exact entity page.
    CadRequested {
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Exact immutable snapshot to inspect.
        snapshot: scriberig_core::SnapshotId,
        /// Stable semantic drawing identity.
        drawing: scriberig_core::EntityId,
        /// Zero-based entity page offset.
        offset: usize,
    },
    /// Applies an exact-snapshot CAD drawing and entity page.
    CadLoaded {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Typed drawing summary and semantic layers.
        drawing: scriberig_protocol::CadDrawingInspectResponse,
        /// Bounded semantic entity page.
        entities: scriberig_protocol::CadEntitiesResponse,
    },
    /// Applies a failed CAD inspection request.
    CadFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Selects an entity only when it occurs in the loaded exact CAD page.
    CadEntitySelected {
        /// Stable semantic CAD entity identity.
        entity: scriberig_core::EntityId,
    },
    /// Requests a shared exact-base CAD layer-change candidate.
    CadLayerChangeRequested(scriberig_protocol::CadLayerChangeRequest),
    /// Applies a CAD layer candidate and opens common change review.
    CadLayerChangeProposed {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Durable candidate metadata from the shared app operation.
        candidate: scriberig_protocol::CadLayerChangeResponse,
    },
    /// Applies a failed CAD layer-change candidate request.
    CadLayerChangeFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a bounded worksheet range or page.
    SpreadsheetRequested {
        /// Stable owning project identity.
        project: scriberig_core::ProjectId,
        /// Exact immutable snapshot to inspect.
        snapshot: scriberig_core::SnapshotId,
        /// Stable semantic worksheet identity.
        worksheet: scriberig_core::EntityId,
        /// Optional inclusive coordinate bounds.
        range: Option<scriberig_domain_spreadsheet::CellRange>,
        /// Zero-based sparse-cell page offset.
        offset: usize,
    },
    /// Applies a successful bounded spreadsheet page.
    SpreadsheetLoaded {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Typed semantic cell page normalized from local or remote service.
        page: scriberig_protocol::SpreadsheetCellsResponse,
    },
    /// Applies a failed spreadsheet page request.
    SpreadsheetFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests inspection of one exact worksheet coordinate.
    SpreadsheetCellRequested(scriberig_app::SpreadsheetCellQuery),
    /// Applies a successful exact-coordinate inspection.
    SpreadsheetCellLoaded {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Typed cell data, including sparse blank coordinates.
        cell: scriberig_protocol::SpreadsheetCellInspectResponse,
    },
    /// Applies a failed exact-coordinate inspection.
    SpreadsheetCellFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a shared exact-base spreadsheet candidate.
    SpreadsheetEditRequested(scriberig_app::SpreadsheetCellEditQuery),
    /// Applies a created spreadsheet candidate and opens shared review.
    SpreadsheetEditProposed {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Reviewable candidate returned by the shared operation.
        candidate: scriberig_app::SpreadsheetEditCandidate,
    },
    /// Applies a failed spreadsheet candidate request.
    SpreadsheetEditFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests an exact-snapshot semantic document tree.
    DocumentRequested(scriberig_app::DocumentTreeQuery),
    /// Applies a successful semantic document read.
    DocumentLoaded {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Typed section, paragraph, and table structure.
        tree: scriberig_protocol::DocumentInspectResponse,
    },
    /// Applies a failed semantic document read.
    DocumentFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Selects a verified paragraph or table cell from the loaded document tree.
    DocumentPartSelected {
        /// Exact semantic part identity found in the loaded tree.
        entity: scriberig_core::EntityId,
    },
    /// Selects a bounded document outline page already present in the loaded tree.
    DocumentPageSelected {
        /// Zero-based record offset into the flattened semantic outline.
        offset: usize,
    },
    /// Requests an exact-base semantic paragraph replacement candidate.
    DocumentEditRequested(scriberig_app::DocumentParagraphEditQuery),
    /// Request an exact-base table-cell replacement candidate.
    DocumentTableCellEditRequested(scriberig_app::DocumentTableCellEditQuery),
    /// Applies a created paragraph candidate and opens shared review.
    DocumentEditProposed {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Reviewable candidate returned by the shared application API.
        candidate: scriberig_app::DocumentParagraphCandidate,
    },
    /// Carries a completed table-cell candidate response.
    DocumentCellEditProposed {
        /// Monotonic request generation.
        request_id: u64,
        /// Durable candidate metadata.
        candidate: scriberig_app::DocumentParagraphCandidate,
    },
    /// Carries a failed table-cell candidate response.
    DocumentCellEditFailed {
        /// Monotonic request generation.
        request_id: u64,
        /// Stable failure category.
        category: String,
        /// Safe failure message.
        message: String,
    },
    /// Applies a failed paragraph candidate request.
    DocumentEditFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests one exact-snapshot QGIS project and its resolved data references.
    QgisProjectRequested(scriberig_app::QgisProjectQuery),
    /// Applies one exact-snapshot QGIS project result.
    QgisProjectLoaded {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Provider-neutral project and linked-layer details.
        project: scriberig_protocol::QgisProjectResponse,
    },
    /// Applies a failed QGIS project request.
    QgisProjectFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe user-facing message.
        message: String,
    },
    /// Requests a shared exact-base QGIS layer-reference rename candidate.
    QgisLayerRenameRequested(scriberig_protocol::QgisLayerRenameRequest),
    /// Requests a complete exact-snapshot QGIS project layer order.
    QgisLayerOrderRequested(scriberig_protocol::QgisLayerOrderRequest),
    /// Applies a QGIS rename candidate and opens common change review.
    QgisLayerRenameProposed {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Durable candidate metadata from the shared app operation.
        candidate: scriberig_protocol::QgisLayerRenameResponse,
    },
    /// Applies a QGIS layer-order candidate and opens common change review.
    QgisLayerOrderProposed {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Durable candidate metadata from the shared app operation.
        candidate: scriberig_protocol::QgisLayerOrderResponse,
    },
    /// Applies a failed QGIS project layer-order proposal.
    QgisLayerOrderFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Applies a failed QGIS rename proposal.
    QgisLayerRenameFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a personal source-style preference update or clear.
    ProjectionStylePreferenceRequested(scriberig_protocol::ProjectionStyleSettingRequest),
    /// Applies the authoritative result of a personal source-style preference update.
    ProjectionStylePreferenceSet {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Stored value and compare-and-swap revision.
        setting: scriberig_protocol::ProjectionStyleSettingResponse,
    },
    /// Applies a failed personal source-style preference update.
    ProjectionStylePreferenceFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Applies the exact effective projection style resolution.
    ProjectionStyleResolved {
        /// Request generation used to reject stale results.
        request_id: u64,
        /// Effective style, provenance, and fidelity from the shared resolver.
        resolution: scriberig_protocol::ProjectionStyleResolveResponse,
    },
    /// Applies a failed effective projection style request.
    ProjectionStyleResolveFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests effective style inspection for the selected source view.
    ProjectionStyleResolveRequested(scriberig_protocol::ProjectionStyleResolveRequest),
    /// Requests one exact-snapshot `GeoPackage` catalog page.
    GeoCatalogRequested {
        /// Owning project.
        project: scriberig_core::ProjectId,
        /// Exact source path in the project artifact manifest.
        path: String,
        /// Exact immutable semantic snapshot.
        snapshot: scriberig_core::SnapshotId,
        /// Zero-based layer page offset.
        offset: usize,
    },
    /// Applies one exact-snapshot `GeoPackage` catalog page.
    GeoCatalogLoaded {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Typed dataset and layer catalog metadata.
        catalog: scriberig_protocol::GeoCatalogResponse,
    },
    /// Applies a failed `GeoPackage` catalog request.
    GeoCatalogFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a bounded feature page for a catalog layer.
    GeoFeaturesRequested {
        /// Stable semantic layer identity.
        layer: scriberig_core::EntityId,
        /// Zero-based feature page offset.
        offset: usize,
    },
    /// Applies one bounded feature page.
    GeoFeaturesLoaded {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Typed provider-neutral feature data.
        page: scriberig_protocol::GeoFeaturesResponse,
    },
    /// Applies a failed feature page request.
    GeoFeaturesFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a bounded typed geospatial predicate query.
    GeoFeatureQueryRequested(scriberig_app::GeoFeatureQuery),
    /// Applies one query result with source and CRS provenance.
    GeoFeatureQueryLoaded {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Typed query matches and provenance.
        result: scriberig_protocol::GeoFeatureQueryResponse,
    },
    /// Applies a failed geospatial query request.
    GeoFeatureQueryFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured failure category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Selects a verified feature from the loaded exact-snapshot page.
    GeoFeatureSelected {
        /// Stable semantic feature identity.
        feature: scriberig_core::EntityId,
    },
    /// Requests a shared exact-base feature attribute candidate.
    GeoAttributeEditRequested(scriberig_project::GeoPackageFeatureAttributeEdit),
    /// Applies a created geospatial candidate and opens the shared Changes review.
    GeoAttributeEditProposed {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Reviewable candidate returned by the shared service.
        candidate: scriberig_app::GeoAttributeCandidate,
    },
    /// Applies a failed geospatial candidate request.
    GeoAttributeEditFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured error category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Requests a shared exact-base feature geometry candidate.
    GeoGeometryEditRequested(scriberig_protocol::GeoGeometryEditRequest),
    /// Applies a created geometry candidate and opens the shared Changes review.
    GeoGeometryEditProposed {
        /// Request generation used to reject stale responses.
        request_id: u64,
        /// Reviewable candidate returned by the shared service.
        candidate: scriberig_app::GeoGeometryCandidate,
    },
    /// Applies a failed geometry candidate request.
    GeoGeometryEditFailed {
        /// Request generation used to reject stale failures.
        request_id: u64,
        /// Safe structured error category.
        category: String,
        /// Safe presentation message.
        message: String,
    },
    /// Apply runtime capability metadata returned by the application.
    CapabilitiesLoaded(Vec<scriberig_app::RuntimeCapability>),
    /// Replace observed lifecycle state with a current application snapshot.
    OperationsLoaded(Vec<scriberig_app::OperationRecord>),
    /// Switches the primary view without changing semantic selection.
    ViewSelected(crate::selection::PrimaryView),
    /// Applies a history operation result.
    HistorySucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected history.
        model: crate::view::history::HistoryModel,
    },
    /// Applies a diff operation result.
    DiffSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected diff.
        model: crate::view::diff::DiffModel,
    },
    /// Applies a projection operation result.
    ProjectionSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected rendered content.
        model: crate::view::projection::ProjectionModel,
    },
    /// Applies a candidate review result.
    ReviewSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected review.
        model: crate::view::changes::ChangeReviewModel,
    },
    /// Records a committed candidate and refreshes the active semantic selection.
    CommitSucceeded {
        /// Request generation.
        request_id: u64,
        /// Reviewed candidate result.
        model: crate::view::changes::ChangeReviewModel,
        /// Authoritative successor snapshot returned by the application.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Applies a failed operation result without retrying it.
    OperationFailed {
        /// Request generation.
        request_id: u64,
        /// Stable application error category.
        category: String,
        /// Safe error message.
        message: String,
    },
    /// Opens the explicit review confirmation overlay.
    ConfirmReview,
    /// Cancels review confirmation without a service call.
    CancelReview,
    /// Confirms the reviewed canonical commit.
    CommitConfirmed(crate::view::changes::ConfirmedCommit),
    /// Requests history for an exact snapshot.
    HistoryRequested {
        /// Exact snapshot whose history is requested.
        snapshot: scriberig_core::SnapshotId,
    },
    /// Requests snapshot-scoped semantic discovery.
    FindRequested(String),
    /// Applies a successful semantic discovery result.
    FindSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected discovery rows.
        model: crate::view::find::FindModel,
    },
    /// Requests a two-sided semantic diff.
    DiffRequested {
        /// Public exact diff request.
        request: scriberig_app::DiffRequest,
    },
    /// Requests a shared projection render.
    ProjectionRequested {
        /// Public render request.
        request: scriberig_app::RenderRequest,
    },
    /// Requests candidate construction for review.
    ReviewRequested {
        /// Public apply request.
        request: scriberig_app::ApplyRequest,
    },
    /// Applies an authoritative operation status observation.
    OperationObserved(crate::view::operations::OperationModel),
    /// Requests an authoritative operation status refresh.
    OperationStatusRequested(scriberig_core::OperationId),
    /// Requests application-owned cancellation of an operation.
    OperationCancelRequested(scriberig_core::OperationId),
    /// Applies a public session tree result.
    SessionSucceeded {
        /// Request generation.
        request_id: u64,
        /// Projected session tree.
        model: crate::view::session::SessionModel,
        /// Tree-aware conversation projection over the same durable snapshot.
        conversation: crate::view::conversation::ConversationModel,
        /// Shared session snapshot retained for presentation-only rebuilding.
        session_snapshot: Option<scriberig_session::Session>,
    },
    /// Applies a durable list of sessions to the session-list overlay.
    SessionsListed(Vec<scriberig_session::Session>),
    /// Applies an exact session switch to the conversation projection.
    SessionSwitched(scriberig_session::Session),
    /// Applies removal of one exact session.
    SessionRemoved(scriberig_core::EntityId),
    /// Applies an authoritative plan result and rebuilds the conversation tree.
    PlanUpdated(scriberig_session::Plan),
    /// Applies a successful model-library read.
    ModelLibraryLoaded {
        /// Request generation.
        request_id: u64,
        /// Credential-safe model and profile descriptors.
        library: scriberig_protocol::ModelLibraryResponse,
    },
    /// Applies shared presentation defaults without replacing local overrides.
    PresentationPreferencesLoaded(scriberig_protocol::AgentPresentationPreferencesResponse),
    /// Applies expanded safe detail for one durable tool result.
    ToolResultDetailLoaded(scriberig_protocol::ToolResultDetailResponse),
    /// Store one captured provider-neutral model request detail response.
    ModelRequestDetailLoaded(scriberig_protocol::ModelRequestDetailResponse),
    /// Applies an authoritative future-invocation execution selection.
    SessionExecutionSelectionUpdated(scriberig_session::SessionExecutionSelection),
    /// Tracks the operation returned by an agent command submission.
    AgentSubmitted(crate::service::AgentHandle),
    /// Requests a public session tree.
    SessionRequested {
        /// Stable session identity.
        session: scriberig_core::EntityId,
        /// Optional branch whose activity is loaded on demand.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Opens the agent composer for an explicit session branch.
    ComposerOpened {
        /// Session receiving the command.
        session: scriberig_core::EntityId,
        /// Optional branch receiving the new turn.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Opens the persistent composer on the session surface.
    SessionComposerOpened {
        /// Session receiving the command.
        session: scriberig_core::EntityId,
        /// Optional branch receiving the new turn.
        branch: Option<scriberig_core::EntityId>,
    },
    /// Changes keyboard focus for the persistent session composer.
    SessionComposerFocused(bool),
    /// Records successful contact after a remote disconnect.
    ConnectionRestored,
    /// Records bounded agent output under its shared operation identity.
    AgentOutputObserved {
        /// Operation producing this output.
        operation: scriberig_core::OperationId,
        /// Output fragment or complete output value.
        text: String,
        /// Whether the shared event contains the complete output value.
        complete: bool,
    },
}
