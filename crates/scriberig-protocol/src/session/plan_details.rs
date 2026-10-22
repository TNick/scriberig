//! Defines bounded plan gate and evidence detail reads.

use crate::{SessionPlanEvidenceAssociation, SessionPlanGate, SessionReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests bounded gate and evidence pages from one plan revision.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanDetailsRequest {
    /// Session containing the plan.
    pub session: SessionReference,
    /// Stable plan identity.
    pub plan: String,
    /// Plan mutation revision observed by the caller.
    pub expected_revision: u64,
    /// Exact historical snapshot to read, or the current plan when absent.
    #[serde(default)]
    pub snapshot: Option<String>,
    /// Zero-based gate offset.
    #[serde(default)]
    pub gate_offset: u32,
    /// Zero-based evidence offset.
    #[serde(default)]
    pub evidence_offset: u32,
    /// Requested item count for each collection, capped by the server.
    #[serde(default)]
    pub limit: u32,
}

/// One independently paged set of canonical plan requirements.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionPlanDetailsResponse {
    /// Stable plan identity.
    pub plan: String,
    /// Plan mutation revision represented by these pages.
    pub revision: u64,
    /// Exact universal snapshot when the plan is durably loaded.
    #[serde(default)]
    pub snapshot: Option<String>,
    /// Total gates retained by this plan revision.
    pub gate_total: u32,
    /// Gate offset represented by this page.
    pub gate_offset: u32,
    /// Gates in stable identity order.
    pub gates: Vec<SessionPlanGate>,
    /// Offset for another gate page, when more gates remain.
    #[serde(default)]
    pub gate_next_offset: Option<u32>,
    /// Total evidence associations retained by this plan revision.
    pub evidence_total: u32,
    /// Evidence offset represented by this page.
    pub evidence_offset: u32,
    /// Evidence associations in stable identity order.
    pub evidence: Vec<SessionPlanEvidenceAssociation>,
    /// Offset for another evidence page, when more evidence remains.
    #[serde(default)]
    pub evidence_next_offset: Option<u32>,
}
