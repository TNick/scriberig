//! Defines extensible semantic discovery requests and results.

use crate::{
    DetailLevel, InformationCost, Page, PageRequest, ProjectionAnchor, QueryError, QueryScope,
    TargetRef,
};
use scriberig_core::ObjectType;

/// Selects a discovery strategy without promising provider support.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum FindStrategy {
    /// Matches exact text.
    ExactText,
    /// Matches tokenized terms.
    Token,
    /// Matches structured names.
    StructuredName,
    /// Uses a semantic provider when installed.
    Semantic,
    /// Uses a domain-defined stable strategy identifier.
    DomainSpecific(String),
}

/// Requests ranked or approximate discovery inside one exact snapshot scope.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FindRequest {
    /// Exact semantic scope to search.
    pub scope: QueryScope,
    /// Search text or provider-defined expression.
    pub query: String,
    /// Discovery strategy requested by the caller.
    pub strategy: FindStrategy,
    /// Page bounds and opaque continuation.
    pub page: PageRequest,
    /// Detail level for returned hits.
    pub detail: DetailLevel,
}

/// One semantic discovery hit with optional ranking/provenance metadata.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FindHit {
    /// Semantic handle that can be passed to inspection.
    pub target: TargetRef,
    /// Entity type when known.
    pub entity_type: Option<ObjectType>,
    /// Optional short label or description.
    pub label: Option<String>,
    /// Optional bounded preview.
    pub preview: Option<String>,
    /// Optional provider ranking score.
    pub score: Option<f32>,
    /// Optional projection/source anchor.
    pub anchor: Option<ProjectionAnchor>,
    /// Optional provenance or derivation handle.
    pub provenance: Option<String>,
}

/// Returns discovery hits with the exact scope and strategy used.
#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct FindResult {
    /// Exact scope used by discovery.
    pub scope: QueryScope,
    /// Strategy executed by the provider.
    pub strategy: FindStrategy,
    /// Bounded discovery page.
    pub page: Page<FindHit>,
    /// Advisory cost estimate.
    pub information_cost: Option<InformationCost>,
}

/// Provides semantic discovery without exposing backend search implementation.
pub trait FindService {
    /// Executes one exact-scope discovery request.
    ///
    /// # Parameters
    /// - `self`: Discovery provider.
    /// - `request`: Exact-scope find request.
    ///
    /// # Returns
    /// A bounded page of semantic hits.
    ///
    /// # Errors
    /// Returns `UnsupportedCapability` when the requested strategy is unavailable.
    fn find(&self, request: &FindRequest) -> Result<FindResult, QueryError>;
}
