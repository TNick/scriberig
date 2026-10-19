//! Defines execution values for durable session transport.

use crate::{SessionActivityKind, SessionBranch, SessionPlan, SessionReference};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Session-scoped model selection used by future agent invocations.
#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionExecutionSelection {
    /// Mutation revision for optimistic concurrency.
    pub revision: u64,
    /// Curated model profile identity, when selected.
    #[serde(default)]
    pub model_profile: Option<String>,
    /// Provider-neutral model identity, when selected.
    #[serde(default)]
    pub model: Option<String>,
    /// Model-advertised thinking level, when selected.
    #[serde(default)]
    pub thinking_level: Option<String>,
    /// Bounded replayable model and thinking selection provenance.
    #[serde(default)]
    pub history: Vec<SessionExecutionChange>,
}

/// One durable model or thinking selection change.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionExecutionChange {
    /// Selection mutation revision that produced this record.
    pub revision: u64,
    /// Semantic category of the changed selection.
    pub kind: SessionActivityKind,
    /// Safe concise description of the resulting selection.
    pub summary: String,
    /// Resolved model identity after the change, when selected.
    pub model: Option<String>,
    /// Resolved thinking level after the change, when selected.
    pub thinking_level: Option<String>,
}

/// Compact session tree response that avoids provider transcript details.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SessionReadResponse {
    /// Session identity returned by the server.
    pub session: SessionReference,
    /// Deterministically ordered branch summaries.
    pub branches: Vec<SessionBranch>,
    /// Durable plans and result-based dependencies.
    #[serde(default)]
    pub plans: Vec<SessionPlan>,
    /// Current session selection for future invocations.
    #[serde(default)]
    pub execution: SessionExecutionSelection,
}
