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
    AgentAcceptedResponse, AgentContinueRequest, AgentResult, AgentSelection, AgentStartRequest,
    BranchReference, CandidateChangeReference, SessionReference,
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
    CancelOperationRequest, CommandRequest, FindRequest, InspectEntityRequest, ListRequest,
    ModelLibraryRequest, OperationStatusRequest, ProjectRequest, QueryRequest,
    RuntimeCapabilitiesRequest, SessionModelSelectionRequest, SessionThinkingSelectionRequest,
    SnapshotSelector, SubscribeRequest,
};
pub use response::{
    FindResponse, InspectEntityResponse, ListResponse, OperationStatusResponse, ProjectResponse,
    QueryResponse,
};
pub use session::{
    ModelRequestDetailRequest, ModelRequestDetailResponse, ModelRequestMessageDetail,
    ModelRequestToolDetail, SessionActivityItem, SessionActivityKind, SessionActivityState,
    SessionBranch, SessionBranchResult, SessionBranchStatus, SessionExecutionChange,
    SessionExecutionSelection, SessionReadRequest, SessionReadResponse, SessionTurn,
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
mod tests {
    use super::*;
    use serde::Serialize;
    use serde::de::DeserializeOwned;

    /// Verifies a value survives JSON encoding without semantic loss.
    fn round_trip<T>(value: &T)
    where
        T: DeserializeOwned + std::fmt::Debug + PartialEq + Serialize,
    {
        let encoded = serde_json::to_string(value).expect("serialize protocol value");
        let decoded: T = serde_json::from_str(&encoded).expect("deserialize protocol value");
        assert_eq!(decoded, *value);
    }

    /// Verifies stable references and snapshot-aware requests round trip.
    #[test]
    fn references_and_requests_round_trip() {
        let entity = EntityReference {
            id: "entity-1".to_owned(),
        };
        round_trip(&entity);
        round_trip(&InspectEntityRequest {
            entity,
            snapshot: SnapshotSelector::Current,
            detail: request::DetailLevel::Deep,
        });
        round_trip(&QueryRequest {
            specification: serde_json::json!({"scope": "snapshot"}),
        });
        round_trip(&ListRequest {
            specification: serde_json::json!({"collection": "snapshot.entities"}),
        });
        round_trip(&CommandRequest {
            kind: "apply".to_owned(),
            payload: serde_json::json!({"change": "change-1"}),
        });
        round_trip(&AgentStartRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            prompt: "inspect the selected branch".to_owned(),
            selection: None,
        });
        round_trip(&AgentContinueRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            branch: BranchReference {
                id: "branch-1".to_owned(),
            },
            prompt: "continue".to_owned(),
            selection: None,
        });
        round_trip(&SessionReadResponse {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            branches: Vec::new(),
            execution: SessionExecutionSelection {
                revision: 7,
                model_profile: Some("profile-1".to_owned()),
                model: Some("model-1".to_owned()),
                thinking_level: Some("careful".to_owned()),
                history: Vec::new(),
            },
        });
        model_contracts_round_trip();
    }

    /// Verifies model, prompt, and presentation contracts round trip.
    fn model_contracts_round_trip() {
        round_trip(&ModelRequestDetailRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            branch: BranchReference {
                id: "branch-1".to_owned(),
            },
            invocation: "invocation-1".to_owned(),
        });
        round_trip(&ModelRequestDetailResponse {
            invocation: "invocation-1".to_owned(),
            model: "model-1".to_owned(),
            thinking_level: Some("careful".to_owned()),
            messages: vec![ModelRequestMessageDetail {
                role: "user".to_owned(),
                content: "safe prompt".to_owned(),
            }],
            tools: vec![ModelRequestToolDetail {
                name: "inspect".to_owned(),
                description: "Inspect an entity".to_owned(),
                parameters: Some("{}".to_owned()),
            }],
            truncated: false,
        });
        round_trip(&ModelLibraryRequest::default());
        round_trip(&SessionModelSelectionRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            expected_revision: 7,
            model_profile: "profile-1".to_owned(),
        });
        round_trip(&SessionThinkingSelectionRequest {
            session: SessionReference {
                id: "session-1".to_owned(),
            },
            expected_revision: 8,
            thinking_level: "extended".to_owned(),
        });
        round_trip(&ModelLibraryResponse {
            models: vec![PublicModelDescriptor {
                id: "model-1".to_owned(),
                display_name: "Safe Model Name".to_owned(),
                provider_profile: "provider-1".to_owned(),
                provider_model_id: "provider-model-1".to_owned(),
                capabilities: PublicModelCapabilities {
                    streaming: true,
                    tool_calls: true,
                    structured_output: false,
                },
                thinking_levels: vec![PublicThinkingLevelDescriptor {
                    id: "deep-dive".to_owned(),
                    label: "Deep dive".to_owned(),
                    description: None,
                    order: 0,
                }],
                default_thinking_level: Some("deep-dive".to_owned()),
            }],
            profiles: vec![PublicModelProfile {
                id: "profile-1".to_owned(),
                display_name: "Primary".to_owned(),
                model: "model-1".to_owned(),
                connection: "connection-1".to_owned(),
                credential: Some(ModelCredentialRef {
                    id: "secret-ref-1".to_owned(),
                }),
                default_thinking_level: Some("deep-dive".to_owned()),
                enabled: true,
            }],
        });
    }

    /// Verifies errors preserve every machine-readable category field.
    #[test]
    fn structured_error_round_trip() {
        round_trip(&ProtocolError {
            code: ProtocolErrorCode::PermissionDenied,
            message: "denied".to_owned(),
            details: Some(serde_json::json!({"action": "inspect"})),
            correlation_id: Some("request-1".to_owned()),
        });
    }

    /// Verifies operation, event, and version values round trip together.
    #[test]
    fn operation_event_and_version_round_trip() {
        round_trip(&VersionExchange {
            active: ProtocolVersion::default(),
            supported: vec![ProtocolVersion::default()],
        });
        round_trip(&Operation {
            reference: OperationReference {
                id: "op-1".to_owned(),
            },
            kind: operation::OperationKind::Inspect,
            status: operation::OperationStatus::Completed,
            progress: Some(operation::OperationProgress {
                percent: 100,
                message: Some("done".to_owned()),
            }),
            result_snapshot: None,
            result_handles: Vec::new(),
            agent_result: None,
            error: None,
        });
        round_trip(&Event {
            sequence: 1,
            kind: EventKind::EntityChanged,
            entity: Some(EntityReference {
                id: "entity-1".to_owned(),
            }),
            revision: None,
            snapshot: None,
            operation: None,
            activity_kind: None,
            activity_state: None,
            metadata: vec![("source".to_owned(), "test".to_owned())],
        });
    }

    /// Verifies trust outcomes and event references round trip distinctly.
    #[test]
    fn trust_outcomes_and_events_round_trip() {
        let selection = TrustSelection {
            entity: EntityReference {
                id: "entity-1".to_owned(),
            },
            snapshot: SnapshotReference {
                id: "snapshot-1".to_owned(),
            },
            revision: None,
        };
        round_trip(&CapabilityDenial {
            action: "promote".to_owned(),
            scope: "entity-1".to_owned(),
            reason: "approval required".to_owned(),
            alternatives: vec!["inspect".to_owned()],
            approval_possible: true,
        });
        round_trip(&TrustDetails {
            summary: TrustSummary {
                source: TrustSource::ModelGenerated,
                validation: ValidationStatus::Current,
                accepted_via: Some("approval-1".to_owned()),
                restricted: false,
            },
            provenance: vec![ProvenanceInput {
                selection: selection.clone(),
                relation: "derived-from".to_owned(),
            }],
            validation: Some(ValidationSummary {
                status: ValidationStatus::Failed,
                diagnostics: vec![ValidationDiagnostic {
                    code: "schema".to_owned(),
                    message: "invalid value".to_owned(),
                    location: Some("field".to_owned()),
                }],
                operation: None,
            }),
            policy: None,
            conflict: None,
        });
        round_trip(&Event {
            sequence: 2,
            kind: EventKind::SnapshotPromoted,
            entity: Some(selection.entity),
            revision: None,
            snapshot: Some(selection.snapshot),
            operation: Some(OperationReference {
                id: "op-1".to_owned(),
            }),
            activity_kind: None,
            activity_state: None,
            metadata: vec![("candidate".to_owned(), "snapshot-2".to_owned())],
        });
        round_trip(&PromotionResult {
            state: PromotionState::Stale,
            candidate: Some(SnapshotReference {
                id: "candidate-1".to_owned(),
            }),
            promoted_snapshot: None,
            reason: Some("base changed".to_owned()),
        });
        round_trip(&ExternalEffectSummary {
            handle: "receipt-1".to_owned(),
            state: ExternalEffectState::Unknown,
            operation: None,
        });
    }
}
