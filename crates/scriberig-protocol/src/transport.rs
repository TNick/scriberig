//! Defines the transport-neutral in-process protocol envelope boundary.

use crate::ArtifactMembersRequest;
use crate::{
    AgentContinueAtRequest, AgentContinueRequest, AgentNewSessionRequest,
    AgentPresentationPreferencesReadRequest, AgentPresentationPreferencesSetRequest,
    AgentStartRequest, ArtifactInspectRequest, CadBoundsQueryRequest, CadDrawingInspectRequest,
    CadEntitiesRequest, CadLayerChangeRequest, CadLineEditRequest, CadTextFindRequest,
    CancelOperationRequest, CommandRequest, DocumentInspectRequest, DocumentParagraphEditRequest,
    DocumentParagraphFindRequest, DocumentParagraphInsertRequest, DocumentTableCellEditRequest,
    Event, FindRequest, GeoAttributeEditRequest, GeoCatalogRequest, GeoFeatureQueryRequest,
    GeoFeaturesRequest, GeoGeometryEditRequest, LegacyExecutionMappingRequest,
    LegacyExecutionRecordsRequest, LegacyPlanAssignmentMappingRequest, ListRequest,
    ModelLibraryRequest, ModelRequestDetailRequest, ProjectRequest, ProjectionStyleResolveRequest,
    ProjectionStyleSchemaRequest, ProjectionStyleSettingRequest, QueryRequest,
    SessionModelSelectionRequest, SessionPlanAssignmentRequest, SessionPlanAttemptsRequest,
    SessionPlanCreateRequest, SessionPlanDetailsRequest, SessionPlanLifecycleRequest,
    SessionPlanMutationRequest, SessionPlanReplanRequest, SessionPlanViewRequest,
    SessionReadRequest, SessionThinkingSelectionRequest, SpreadsheetCellFindRequest,
    SpreadsheetCellInspectRequest, SpreadsheetCellsRequest, SpreadsheetEditRequest,
    SpreadsheetRangeEditRequest, SpreadsheetRenameRequest, SpreadsheetWorkbookRequest,
    SubscribeRequest, ToolResultDetailRequest,
};
use crate::{
    CapabilitiesResponse, InspectEntityRequest, InspectEntityResponse, OperationStatusRequest,
    OperationStatusResponse, ProjectResponse, ProtocolError, ProtocolVersion,
    RuntimeCapabilitiesRequest,
};
use crate::{QgisLayerOrderRequest, QgisLayerRenameRequest, QgisProjectRequest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Initial requests supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum Request {
    /// Read one bounded `GeoPackage` dataset/layer catalog page.
    GeoCatalog(GeoCatalogRequest),
    /// Read one bounded `GeoPackage` feature page.
    GeoFeatures(GeoFeaturesRequest),
    /// Execute a bounded attribute or explicit-CRS spatial query.
    GeoFeatureQuery(GeoFeatureQueryRequest),
    /// Read one exact-snapshot QGIS project and its linked layer references.
    QgisProject(QgisProjectRequest),
    /// Propose one exact-base QGIS layer-reference display-name change.
    QgisLayerRename(QgisLayerRenameRequest),
    /// Propose one exact-base QGIS project layer-order change.
    QgisLayerOrder(QgisLayerOrderRequest),
    /// Create a typed exact-base `GeoPackage` feature geometry candidate.
    GeoGeometryEdit(GeoGeometryEditRequest),
    /// Create a typed exact-base `GeoPackage` feature attribute candidate.
    GeoAttributeEdit(GeoAttributeEditRequest),
    /// Read an exact-snapshot CAD drawing summary.
    CadDrawingInspect(CadDrawingInspectRequest),
    /// Read one bounded page of CAD drawing entities.
    CadEntities(CadEntitiesRequest),
    /// Finds authored text in an exact-snapshot CAD drawing.
    CadTextFind(CadTextFindRequest),
    /// Query CAD entities intersecting bounds in one exact drawing space.
    CadBoundsQuery(CadBoundsQueryRequest),
    /// Create an exact-base semantic CAD line edit candidate.
    CadLineEdit(CadLineEditRequest),
    /// Create an exact-base semantic CAD layer-change candidate.
    CadLayerChange(CadLayerChangeRequest),
    /// Read one exact-snapshot semantic document tree.
    DocumentInspect(DocumentInspectRequest),
    /// Find paragraph text at one exact document snapshot.
    DocumentParagraphFind(DocumentParagraphFindRequest),
    /// Read one bounded sparse worksheet cell page.
    SpreadsheetCells(SpreadsheetCellsRequest),
    /// Read one exact-coordinate spreadsheet cell.
    SpreadsheetCellInspect(SpreadsheetCellInspectRequest),
    /// Reads workbook and worksheet summaries at an exact snapshot.
    SpreadsheetWorkbook(SpreadsheetWorkbookRequest),
    /// Find authored values in a workbook at an exact snapshot.
    SpreadsheetCellFind(SpreadsheetCellFindRequest),
    /// Create a durable exact-base spreadsheet cell edit candidate.
    SpreadsheetEdit(SpreadsheetEditRequest),
    /// Create one bounded exact-base candidate containing multiple spreadsheet cell updates.
    SpreadsheetRangeEdit(SpreadsheetRangeEditRequest),
    /// Create a durable exact-base spreadsheet worksheet rename candidate.
    SpreadsheetRename(SpreadsheetRenameRequest),
    /// Create a durable exact-base document paragraph edit candidate.
    DocumentParagraphEdit(DocumentParagraphEditRequest),
    /// Create a durable exact-base DOCX table-cell edit candidate.
    DocumentTableCellEdit(DocumentTableCellEditRequest),
    /// Create a durable exact-base document paragraph insertion candidate.
    DocumentParagraphInsert(DocumentParagraphInsertRequest),
    /// Read a bounded page of artifact summaries.
    ArtifactInspect(ArtifactInspectRequest),
    /// Read one bounded page of exact artifact-member evidence.
    ArtifactMembers(ArtifactMembersRequest),
    /// Inspect one semantic entity.
    Inspect(InspectEntityRequest),
    /// Discover runtime capabilities.
    Capabilities(RuntimeCapabilitiesRequest),
    /// Read one operation status.
    OperationStatus(OperationStatusRequest),
    /// Cancel one tracked operation.
    CancelOperation(CancelOperationRequest),
    /// Read events currently available to a subscription.
    Subscribe(SubscribeRequest),
    /// Evaluate one typed structural query.
    Query(QueryRequest),
    /// Read one bounded semantic collection page.
    List(ListRequest),
    /// Discover semantic entities within an exact scope.
    Find(FindRequest),
    /// Execute one typed application command.
    Command(CommandRequest),
    /// Start one provider-neutral agent operation.
    AgentStart(AgentStartRequest),
    /// Start a session with application-allocated identities.
    AgentNewSession(AgentNewSessionRequest),
    /// Continue one provider-neutral agent operation.
    AgentContinue(AgentContinueRequest),
    /// Continue from an exact durable conversation anchor.
    AgentContinueAt(AgentContinueAtRequest),
    /// Read one compact durable session tree.
    SessionRead(SessionReadRequest),
    /// Create a durable structured plan.
    PlanCreate(SessionPlanCreateRequest),
    /// Apply one plan lifecycle transition.
    PlanLifecycle(SessionPlanLifecycleRequest),
    /// Change one safe node assignment.
    PlanAssignment(SessionPlanAssignmentRequest),
    /// Apply one complete durable replan revision.
    PlanReplan(SessionPlanReplanRequest),
    /// Apply a typed gate, evidence, retry, or structure mutation.
    PlanMutation(SessionPlanMutationRequest),
    /// Read a bounded page of one node's worker attempts.
    PlanAttempts(SessionPlanAttemptsRequest),
    /// Read bounded gate and evidence pages from one plan revision.
    PlanDetails(SessionPlanDetailsRequest),
    /// Read a compact plan graph without history or detail collections.
    PlanView(SessionPlanViewRequest),
    /// Read a bounded safe model request projection.
    ModelRequestDetail(ModelRequestDetailRequest),
    /// Read bounded safe detail for one tool result.
    ToolResultDetail(ToolResultDetailRequest),
    /// Read the safe model library.
    ModelLibrary(ModelLibraryRequest),
    /// Select a model profile for future invocations.
    SessionModelSelection(SessionModelSelectionRequest),
    /// Select a thinking level for future invocations.
    SessionThinkingSelection(SessionThinkingSelectionRequest),
    /// Inspect preserved legacy execution values for explicit recovery.
    LegacyExecutionRecords(LegacyExecutionRecordsRequest),
    /// Resolve one preserved session-level legacy execution selection.
    LegacyExecutionMapping(LegacyExecutionMappingRequest),
    /// Map one preserved current plan-node assignment to active upstream IDs.
    LegacyPlanAssignmentMapping(LegacyPlanAssignmentMappingRequest),
    /// Read shared presentation defaults for the authenticated principal.
    AgentPresentationPreferencesRead(AgentPresentationPreferencesReadRequest),
    /// Replace shared presentation defaults using an expected revision.
    AgentPresentationPreferencesSet(AgentPresentationPreferencesSetRequest),
    /// Read the domain-owned style schema for one projection kind.
    ProjectionStyleSchema(ProjectionStyleSchemaRequest),
    /// Resolve effective style, provenance, and fidelity for one purpose.
    ProjectionStyleResolve(ProjectionStyleResolveRequest),
    /// Set or clear a style value in one explicit configuration scope.
    ProjectionStyleSetting(ProjectionStyleSettingRequest),
    /// Execute one project onboarding operation.
    Project(ProjectRequest),
}

/// Initial typed responses supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub enum Response {
    /// Bounded exact-snapshot `GeoPackage` catalog page.
    GeoCatalog(crate::GeoCatalogResponse),
    /// Bounded snapshot-bound provider-neutral geospatial features.
    GeoFeatures(crate::GeoFeaturesResponse),
    /// Bounded exact-snapshot geospatial query result and provenance.
    GeoFeatureQuery(crate::GeoFeatureQueryResponse),
    /// Exact-snapshot provider-neutral QGIS project and layer-reference view.
    QgisProject(crate::QgisProjectResponse),
    /// Candidate handle from a QGIS layer-reference rename.
    QgisLayerRename(crate::QgisLayerRenameResponse),
    /// Candidate handle from a QGIS project layer-order edit.
    QgisLayerOrder(crate::QgisLayerOrderResponse),
    /// Candidate handle for a typed feature geometry operation.
    GeoGeometryEdit(crate::GeoGeometryEditResponse),
    /// Candidate handle for a typed feature attribute operation.
    GeoAttributeEdit(crate::GeoAttributeEditResponse),
    /// Exact-snapshot semantic CAD drawing summary.
    CadDrawingInspect(crate::CadDrawingInspectResponse),
    /// Bounded CAD entity page.
    CadEntities(crate::CadEntitiesResponse),
    /// Bounded CAD annotation search results.
    CadTextFind(crate::CadTextFindResponse),
    /// Bounded CAD bounds query result.
    CadBoundsQuery(crate::CadBoundsQueryResponse),
    /// Reviewable candidate handle from a CAD line edit.
    CadLineEdit(crate::CadLineEditResponse),
    /// Reviewable candidate handle from a CAD layer change.
    CadLayerChange(crate::CadLayerChangeResponse),
    /// Typed exact-snapshot semantic document tree.
    DocumentInspect(crate::DocumentInspectResponse),
    /// Bounded exact-snapshot semantic paragraph matches.
    DocumentParagraphFind(crate::DocumentParagraphFindResponse),
    /// Bounded snapshot-bound sparse worksheet cells.
    SpreadsheetCells(crate::SpreadsheetCellsResponse),
    /// Typed cell result or explicit sparse blank coordinate.
    SpreadsheetCellInspect(crate::SpreadsheetCellInspectResponse),
    /// Workbook and ordered worksheet summaries from an exact snapshot.
    SpreadsheetWorkbook(crate::SpreadsheetWorkbookResponse),
    /// Bounded exact-snapshot semantic cell search results.
    SpreadsheetCellFind(crate::SpreadsheetCellFindResponse),
    /// Reviewable candidate handle from a spreadsheet cell edit.
    SpreadsheetEdit(crate::SpreadsheetEditResponse),
    /// Reviewable candidate handle for a bounded multi-cell edit.
    SpreadsheetRangeEdit(crate::SpreadsheetEditResponse),
    /// Reviewable candidate handle from a worksheet rename.
    SpreadsheetRename(crate::SpreadsheetEditResponse),
    /// Reviewable candidate handle from a document paragraph edit.
    DocumentParagraphEdit(crate::DocumentParagraphEditResponse),
    /// Reviewable candidate handle from a semantic table-cell edit.
    DocumentTableCellEdit(crate::DocumentParagraphEditResponse),
    /// Reviewable candidate handle from a document paragraph insertion.
    DocumentParagraphInsert(crate::DocumentParagraphInsertResponse),
    /// Bounded logical artifact summary page.
    ArtifactInspect(crate::ArtifactInspectResponse),
    /// Bounded physical and container-internal artifact members.
    ArtifactMembers(crate::ArtifactMembersResponse),
    /// Inspection result.
    Inspect(InspectEntityResponse),
    /// Runtime capability result.
    Capabilities(CapabilitiesResponse),
    /// Operation status result.
    OperationStatus(OperationStatusResponse),
    /// Resulting state after cancellation.
    OperationCancelled(OperationStatusResponse),
    /// Events currently available to the subscriber.
    Events(Vec<Event>),
    /// Query result page.
    Query(crate::response::QueryResponse),
    /// Collection result page.
    List(crate::response::ListResponse),
    /// Semantic discovery result.
    Find(crate::response::FindResponse),
    /// Accepted command operation.
    CommandAccepted(OperationStatusResponse),
    /// Accepted agent operation.
    AgentAccepted(crate::AgentAcceptedResponse),
    /// Compact durable session tree.
    Session(crate::SessionReadResponse),
    /// Current durable plan projection.
    Plan(crate::SessionPlan),
    /// Bounded safe plan-attempt history page.
    PlanAttempts(crate::SessionPlanAttemptsResponse),
    /// Bounded gate and evidence pages from one exact plan revision.
    PlanDetails(crate::SessionPlanDetailsResponse),
    /// Compact plan graph with exact semantic revision references.
    PlanView(crate::SessionPlanGraphView),
    /// Safe normalized request captured for one model invocation.
    ModelRequestDetail(crate::ModelRequestDetailResponse),
    /// Bounded safe detail for one tool result.
    ToolResultDetail(crate::ToolResultDetailResponse),
    /// Safe model library.
    ModelLibrary(crate::ModelLibraryResponse),
    /// Updated session execution selection.
    SessionExecution(crate::SessionExecutionSelection),
    /// Preserved, sanitized legacy execution values.
    LegacyExecutionRecords(Vec<crate::LegacyExecutionRecordView>),
    /// Shared presentation defaults for the authenticated principal.
    AgentPresentationPreferences(crate::AgentPresentationPreferencesResponse),
    /// Domain-owned projection style schema, when available.
    ProjectionStyleSchema(Option<crate::ProjectionStyleSchemaResponse>),
    /// Effective style with exact provenance and fidelity.
    ProjectionStyleResolve(crate::ProjectionStyleResolveResponse),
    /// Authoritative result of a projection style setting mutation.
    ProjectionStyleSetting(crate::ProjectionStyleSettingResponse),
    /// Project onboarding result.
    Project(ProjectResponse),
    /// Structured protocol failure.
    Error(ProtocolError),
}

/// Correlated protocol request envelope.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct RequestEnvelope {
    /// Protocol version requested by the client.
    pub version: ProtocolVersion,
    /// Correlation identity retained by the server.
    pub correlation_id: String,
    /// Typed semantic request.
    pub request: Request,
}

/// Correlated protocol response envelope.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct ResponseEnvelope {
    /// Protocol version used by the server.
    pub version: ProtocolVersion,
    /// Correlation identity copied from the request.
    pub correlation_id: String,
    /// Typed response or structured error.
    pub response: Response,
}

/// Transport boundary independent of TCP/HTTP/WebSocket details.
pub trait ProtocolTransport {
    /// Sends one correlated protocol request.
    ///
    /// - `request`: Request envelope to send.
    ///
    /// Correlated typed response envelope.
    /// # Returns
    /// The operation result.
    fn send(&self, request: RequestEnvelope) -> ResponseEnvelope;

    /// Returns and clears a transport failure from the most recent send.
    ///
    /// In-process transports never report one. Network transports may use this
    /// optional side channel because the protocol response itself is semantic.
    ///
    /// The most recent transport failure, if any.
    /// # Returns
    /// The operation result.
    fn take_error(&self) -> Option<String> {
        None
    }
}
