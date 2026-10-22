//! Defines the transport-neutral in-process protocol envelope boundary.

use crate::{
    AgentContinueAtRequest, AgentContinueRequest, AgentNewSessionRequest,
    AgentPresentationPreferencesReadRequest, AgentPresentationPreferencesSetRequest,
    AgentStartRequest, CancelOperationRequest, CommandRequest, Event, FindRequest, ListRequest,
    ModelLibraryRequest, ModelRequestDetailRequest, ProjectRequest, QueryRequest,
    SessionModelSelectionRequest, SessionPlanAssignmentRequest, SessionPlanCreateRequest,
    SessionPlanLifecycleRequest, SessionPlanMutationRequest, SessionPlanReplanRequest,
    SessionReadRequest, SessionThinkingSelectionRequest, SubscribeRequest, ToolResultDetailRequest,
};
use crate::{
    CapabilitiesResponse, InspectEntityRequest, InspectEntityResponse, OperationStatusRequest,
    OperationStatusResponse, ProjectResponse, ProtocolError, ProtocolVersion,
    RuntimeCapabilitiesRequest,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Initial requests supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub enum Request {
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
    /// Read shared presentation defaults for the authenticated principal.
    AgentPresentationPreferencesRead(AgentPresentationPreferencesReadRequest),
    /// Replace shared presentation defaults using an expected revision.
    AgentPresentationPreferencesSet(AgentPresentationPreferencesSetRequest),
    /// Execute one project onboarding operation.
    Project(ProjectRequest),
}

/// Initial typed responses supported by the remote vertical slice.
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub enum Response {
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
    /// Safe normalized request captured for one model invocation.
    ModelRequestDetail(crate::ModelRequestDetailResponse),
    /// Bounded safe detail for one tool result.
    ToolResultDetail(crate::ToolResultDetailResponse),
    /// Safe model library.
    ModelLibrary(crate::ModelLibraryResponse),
    /// Updated session execution selection.
    SessionExecution(crate::SessionExecutionSelection),
    /// Shared presentation defaults for the authenticated principal.
    AgentPresentationPreferences(crate::AgentPresentationPreferencesResponse),
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
