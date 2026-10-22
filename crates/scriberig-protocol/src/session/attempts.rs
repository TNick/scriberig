//! Defines bounded plan-attempt history reads.

use crate::{SessionPlanEvidenceReference, SessionPlanNodeStatus, SessionReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests one bounded page of attempts for a plan node.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanAttemptsRequest {
    /// Session containing the plan.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Stable plan-node identity.
    pub node: String,
    /// Zero-based attempt offset.
    #[serde(default)]
    pub offset: u32,
    /// Requested attempt count, capped by the server.
    #[serde(default)]
    pub limit: u32,
}

/// One safe, transcript-free worker attempt record.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanAttempt {
    /// Durable branch used by this attempt.
    pub branch: String,
    /// Monotonic node-local attempt generation.
    pub claim_generation: u64,
    /// Worker claim associated with the attempt, when retained.
    #[serde(default)]
    pub claim: Option<String>,
    /// Latest lifecycle outcome recorded for this attempt.
    pub status: SessionPlanNodeStatus,
    /// Accepted result reference, when this attempt completed.
    #[serde(default)]
    pub result: Option<String>,
    /// Exact failure evidence reference, when this attempt failed.
    #[serde(default)]
    pub failure: Option<SessionPlanEvidenceReference>,
}

/// One bounded page of safe plan-attempt history.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanAttemptsResponse {
    /// Stable plan identity.
    pub plan: String,
    /// Stable plan-node identity.
    pub node: String,
    /// Total attempts retained for this node.
    pub total: u32,
    /// Offset represented by this page.
    pub offset: u32,
    /// Attempts in stable claim-generation/branch order.
    pub attempts: Vec<SessionPlanAttempt>,
    /// Offset for another page, when more attempts remain.
    #[serde(default)]
    pub next_offset: Option<u32>,
}
