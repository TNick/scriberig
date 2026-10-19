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
    CredentialRef as ModelCredentialRef, ModelCapabilities as PublicModelCapabilities,
    ModelDescriptor as PublicModelDescriptor, ModelLibraryResponse,
    ModelProfile as PublicModelProfile, ThinkingLevelDescriptor as PublicThinkingLevelDescriptor,
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
    ModelRequestDetailRequest, ModelRequestDetailResponse, ModelRequestMessageDetail,
    ModelRequestToolDetail, SessionActivityItem, SessionActivityKind, SessionActivityState,
    SessionBranch, SessionBranchResult, SessionBranchStatus, SessionContinuationAnchor,
    SessionCoordinatorClaim, SessionExecutionChange, SessionExecutionSelection,
    SessionExpectedResult, SessionModelAssignment, SessionPlan, SessionPlanAction,
    SessionPlanAssignmentRequest, SessionPlanCreateRequest, SessionPlanDependency,
    SessionPlanLifecycleRequest, SessionPlanNode, SessionPlanNodeDraft, SessionPlanNodeReference,
    SessionPlanNodeStatus, SessionPlanReplanRequest, SessionPlanRevision, SessionPlanStatus,
    SessionReadRequest, SessionReadResponse, SessionTurn, SessionWorkerClaim, ToolActivityKind,
    ToolResultDetailRequest, ToolResultDetailResponse,
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
