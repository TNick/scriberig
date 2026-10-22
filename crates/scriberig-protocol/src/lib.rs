//! Defines stable serializable transport contracts without server business logic.

pub mod agent;
pub mod capability;
pub mod error;
pub mod event;
pub mod model;
pub mod operation;
pub mod presentation;
pub mod refs;
pub mod request;
pub mod response;
pub mod session;
pub mod transport;
pub mod trust;
pub mod version;

pub use agent::{
    AgentAcceptedResponse, AgentContinueAtRequest, AgentContinueRequest, AgentNewSessionRequest,
    AgentResult, AgentSelection, AgentStartRequest, BranchReference, CandidateChangeReference,
    SessionReference,
};
pub use capability::{CapabilitiesResponse, CapabilityInfo, RuntimeFeature};
pub use error::{ProtocolError, ProtocolErrorCode};
pub use event::{Event, EventKind};
pub use model::{
    ModelCostSnapshot as PublicModelCostSnapshot,
    ModelCostTierSnapshot as PublicModelCostTierSnapshot, ModelDescriptor as PublicModelDescriptor,
    ModelInputModality as PublicModelInputModality, ModelLibraryResponse,
    ModelMetadataSnapshot as PublicModelMetadataSnapshot, ProviderCredentialReadiness,
    ProviderReadiness, model_row_id,
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
pub use refs::{EntityReference, OperationReference, RevisionReference, SnapshotReference};
pub use request::{
    CancelOperationRequest, CommandRequest, DomainOperationCommand, FindRequest,
    InspectEntityRequest, ListRequest, ModelLibraryRequest, OperationStatusRequest, ProjectRequest,
    QueryRequest, RuntimeCapabilitiesRequest, SessionModelSelectionRequest,
    SessionThinkingSelectionRequest, SnapshotSelector, SubscribeRequest,
};
pub use response::{
    FindResponse, InspectEntityResponse, ListResponse, OperationStatusResponse, ProjectResponse,
    QueryResponse,
};
pub use session::{
    ModelContextRevisionReference, ModelRequestDetailFragment, ModelRequestDetailRequest,
    ModelRequestDetailResponse, ModelRequestMessageDetail, ModelRequestToolDetail,
    SessionActivityItem, SessionActivityKind, SessionActivityState, SessionBranch,
    SessionBranchResult, SessionBranchStatus, SessionContinuationAnchor, SessionCoordinatorClaim,
    SessionExecutionChange, SessionExecutionSelection, SessionExpectedResult,
    SessionModelAssignment, SessionPlan, SessionPlanAbandonReason, SessionPlanAction,
    SessionPlanAssignmentRequest, SessionPlanAttempt, SessionPlanAttemptsRequest,
    SessionPlanAttemptsResponse, SessionPlanCreateRequest, SessionPlanDependency,
    SessionPlanDetailsRequest, SessionPlanDetailsResponse, SessionPlanEvidenceAssociation,
    SessionPlanEvidenceKind, SessionPlanEvidenceOwner, SessionPlanEvidenceReference,
    SessionPlanEvidenceRole, SessionPlanGate, SessionPlanGateCounts, SessionPlanGatePhase,
    SessionPlanGateRequirement, SessionPlanGateWaiver, SessionPlanGraphView,
    SessionPlanLifecycleRequest, SessionPlanMutation, SessionPlanMutationRequest, SessionPlanNode,
    SessionPlanNodeDraft, SessionPlanNodeReference, SessionPlanNodeStatus, SessionPlanNodeView,
    SessionPlanReplanRequest, SessionPlanRevision, SessionPlanSemanticPredicate, SessionPlanStatus,
    SessionPlanStatusCounts, SessionPlanSummary, SessionPlanView, SessionPlanViewRequest,
    SessionReadRequest, SessionReadResponse, SessionSemanticReference, SessionTurn,
    SessionWorkerClaim, ToolActivityKind, ToolResultDetailRequest, ToolResultDetailResponse,
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
