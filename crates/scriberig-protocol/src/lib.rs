//! Defines stable serializable transport contracts without server business logic.

pub mod agent;
pub mod anchor;
pub mod artifact;
pub mod cad;
pub mod capability;
pub mod document;
pub mod error;
pub mod event;
pub mod geo;
pub mod model;
pub mod operation;
pub mod presentation;
pub mod projection_style;
pub mod qgis;
pub mod refs;
pub mod request;
pub mod response;
pub mod session;
pub mod spreadsheet;
pub mod transport;
pub mod trust;
pub mod version;

pub use agent::{
    AgentAcceptedResponse, AgentContinueAtRequest, AgentContinueRequest, AgentNewSessionRequest,
    AgentResult, AgentSelection, AgentStartRequest, BranchReference, CandidateChangeReference,
    SessionReference,
};
pub use anchor::ProjectionAnchor;
pub use artifact::{
    ArtifactEditCapability, ArtifactInspectRequest, ArtifactInspectResponse, ArtifactMemberSummary,
    ArtifactMembersRequest, ArtifactMembersResponse, ArtifactReference, ArtifactSummary,
};
pub use cad::{
    CadBounds2D, CadBoundsQueryRequest, CadBoundsQueryResponse, CadCandidateValidation,
    CadCoordinate, CadDrawingInspectRequest, CadDrawingInspectResponse, CadDrawingSpace,
    CadEntitiesRequest, CadEntitiesResponse, CadEntityStyleRef, CadEntitySummary, CadGeometry,
    CadLayerChangeRequest, CadLayerChangeResponse, CadLayerSummary, CadLineEditRequest,
    CadLineEditResponse, CadTextFindRequest, CadTextFindResponse, CadTextMatch,
};
pub use capability::{
    CadOpaquePreservation, CadProviderAvailability, CadProviderCapabilityInfo, CadProviderFormat,
    CapabilitiesResponse, CapabilityInfo, RuntimeFeature,
};
pub use document::{
    DocumentBlockView, DocumentCandidateValidation, DocumentCellView, DocumentInlineRunView,
    DocumentInspectRequest, DocumentInspectResponse, DocumentParagraphEditRequest,
    DocumentParagraphEditResponse, DocumentParagraphFindRequest, DocumentParagraphFindResponse,
    DocumentParagraphInsertRequest, DocumentParagraphInsertResponse, DocumentParagraphMatch,
    DocumentParagraphView, DocumentRowView, DocumentSectionView, DocumentStyleView,
    DocumentTableCellEditRequest, DocumentTableView, DocumentTextStyleRef,
};
pub use error::{ProtocolError, ProtocolErrorCode};
pub use event::{Event, EventKind};
pub use geo::{
    GeoAttributeEditRequest, GeoAttributeEditResponse, GeoCatalogRequest, GeoCatalogResponse,
    GeoCoordinate, GeoCrsView, GeoFeatureBounds, GeoFeaturePredicate, GeoFeatureQueryRequest,
    GeoFeatureQueryResponse, GeoFeatureView, GeoFeaturesRequest, GeoFeaturesResponse, GeoFieldType,
    GeoFieldView, GeoGeometry, GeoGeometryEditRequest, GeoGeometryEditResponse, GeoGeometryType,
    GeoLayerView, GeoValue,
};
pub use model::{
    ModelCostSnapshot as PublicModelCostSnapshot,
    ModelCostTierSnapshot as PublicModelCostTierSnapshot, ModelDescriptor as PublicModelDescriptor,
    ModelInputModality as PublicModelInputModality, ModelLibraryResponse,
    ModelMetadataSnapshot as PublicModelMetadataSnapshot, ProviderAuthenticationStatus,
    ProviderCredentialReadiness, ProviderReadiness, model_row_id,
};
pub use operation::{Operation, OperationProgress, OperationStatus};
pub use presentation::{
    AgentPresentationPreferences as PublicAgentPresentationPreferences,
    AgentPresentationPreferencesReadRequest, AgentPresentationPreferencesResponse,
    AgentPresentationPreferencesSetRequest, PromptVisibility as PublicPromptVisibility,
    ReasoningVisibility as PublicReasoningVisibility,
    ToolInvocationDetail as PublicToolInvocationDetail,
    ToolOutputVisibility as PublicToolOutputVisibility,
};
pub use projection_style::{
    ProjectionStyleApplicability, ProjectionStyleFidelity, ProjectionStyleFidelityReport,
    ProjectionStyleProperty, ProjectionStylePurpose, ProjectionStyleReference,
    ProjectionStyleResolveRequest, ProjectionStyleResolveResponse, ProjectionStyleSchemaReference,
    ProjectionStyleSchemaRequest, ProjectionStyleSchemaResponse, ProjectionStyleScope,
    ProjectionStyleSettingRequest, ProjectionStyleSettingResponse, ProjectionStyleSource,
    ProjectionStyleValueKind,
};
pub use qgis::{
    QgisLayerGroupView, QgisLayerOrderRequest, QgisLayerOrderResponse, QgisLayerReferenceView,
    QgisLayerRenameRequest, QgisLayerRenameResponse, QgisProjectRequest, QgisProjectResponse,
};
pub use refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
pub use request::{
    CancelOperationRequest, CommandRequest, DomainOperationCommand, FindRequest,
    InspectEntityRequest, LegacyExecutionMappingRequest, LegacyExecutionRecordsRequest,
    LegacyPlanAssignmentMappingRequest, ListRequest, ModelLibraryRequest, OperationStatusRequest,
    ProjectRequest, QueryRequest, RuntimeCapabilitiesRequest, SessionModelSelectionRequest,
    SessionThinkingSelectionRequest, SnapshotSelector, SubscribeRequest,
};
pub use response::{
    FindResponse, InspectEntityResponse, ListResponse, OperationStatusResponse, ProjectResponse,
    QueryResponse,
};
pub use session::{
    LegacyExecutionRecordView, LegacyExecutionTargetView, ModelContextRevisionReference,
    ModelRequestDetailFragment, ModelRequestDetailRequest, ModelRequestDetailResponse,
    ModelRequestMessageDetail, ModelRequestToolDetail, SessionActivityItem, SessionActivityKind,
    SessionActivityState, SessionBranch, SessionBranchResult, SessionBranchStatus,
    SessionContinuationAnchor, SessionCoordinatorClaim, SessionExecutionChange,
    SessionExecutionSelection, SessionExpectedResult, SessionModelAssignment, SessionPlan,
    SessionPlanAbandonReason, SessionPlanAction, SessionPlanAssignmentRequest, SessionPlanAttempt,
    SessionPlanAttemptsRequest, SessionPlanAttemptsResponse, SessionPlanCreateRequest,
    SessionPlanDependency, SessionPlanDetailsRequest, SessionPlanDetailsResponse,
    SessionPlanEvidenceAssociation, SessionPlanEvidenceKind, SessionPlanEvidenceOwner,
    SessionPlanEvidenceReference, SessionPlanEvidenceRole, SessionPlanGate, SessionPlanGateCounts,
    SessionPlanGatePhase, SessionPlanGateRequirement, SessionPlanGateWaiver, SessionPlanGraphView,
    SessionPlanLifecycleRequest, SessionPlanMutation, SessionPlanMutationRequest, SessionPlanNode,
    SessionPlanNodeDraft, SessionPlanNodeReference, SessionPlanNodeStatus, SessionPlanNodeView,
    SessionPlanReplanRequest, SessionPlanRevision, SessionPlanSemanticPredicate, SessionPlanStatus,
    SessionPlanStatusCounts, SessionPlanSummary, SessionPlanView, SessionPlanViewRequest,
    SessionReadRequest, SessionReadResponse, SessionSemanticReference, SessionTurn,
    SessionWorkerClaim, ToolActivityKind, ToolResultDetailRequest, ToolResultDetailResponse,
};
pub use spreadsheet::{
    SpreadsheetCandidateValidation, SpreadsheetCellEdit, SpreadsheetCellFindRequest,
    SpreadsheetCellFindResponse, SpreadsheetCellInspectRequest, SpreadsheetCellInspectResponse,
    SpreadsheetCellMatch, SpreadsheetCellStyle, SpreadsheetCellValue, SpreadsheetCellView,
    SpreadsheetCellsRequest, SpreadsheetCellsResponse, SpreadsheetEditRequest,
    SpreadsheetEditResponse, SpreadsheetRange, SpreadsheetRangeEditRequest,
    SpreadsheetRenameRequest, SpreadsheetTableSummary, SpreadsheetWorkbookRequest,
    SpreadsheetWorkbookResponse, SpreadsheetWorksheetSummary,
};
pub use transport::{ProtocolTransport, Request, RequestEnvelope, Response, ResponseEnvelope};
pub use trust::{
    ApprovalSummary, CapabilityDenial, ConflictSummary, ExternalEffectState, ExternalEffectSummary,
    PolicyEvaluationSummary, PromotionResult, PromotionState, ProvenanceInput, TrustDetails,
    TrustSelection, TrustSource, TrustSummary, ValidationDiagnostic, ValidationStatus,
    ValidationSummary,
};
pub use version::{ProtocolVersion, VersionExchange};

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
