//! Defines the storage-independent structured query expression vocabulary.

use crate::{DetailLevel, InformationCost, Page, PageRequest, QueryError, QueryScope, TargetRef};
use scriberig_core::ObjectType;

/// Domain-neutral scalar value used by structural predicates.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum QueryValue {
    /// UTF-8 scalar value.
    Text(String),
    /// Signed integer value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
}

/// Direction for typed relation predicates.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum QueryDirection {
    /// Traverse relation targets from the subject.
    Outgoing,
    /// Traverse reverse relation targets when supported.
    Incoming,
}

/// Declarative structural query expression.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum QueryExpr {
    /// Matches every entity in the scope.
    All,
    /// Matches entities with the exact semantic type.
    TypeIs(ObjectType),
    /// Matches entities whose property equals a scalar value.
    PropertyEq {
        /// Property key to compare.
        key: String,
        /// Expected property value.
        value: QueryValue,
    },
    /// Matches entities that expose a property key.
    PropertyExists {
        /// Property key to require.
        key: String,
    },
    /// Matches entities related to a target expression.
    Related {
        /// Stable semantic relation identifier.
        relation: String,
        /// Relation traversal direction.
        direction: QueryDirection,
        /// Predicate applied to related entities.
        target: Box<QueryExpr>,
    },
    /// Matches entities contained in a typed collection.
    MemberOf {
        /// Stable semantic collection identifier.
        collection: String,
    },
    /// Requires every nested expression to match.
    And(Vec<QueryExpr>),
    /// Requires at least one nested expression to match.
    Or(Vec<QueryExpr>),
    /// Negates one nested expression.
    Not(Box<QueryExpr>),
}

/// Orders exact structural query results using domain-independent semantics.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum QueryOrdering {
    /// Provider-defined stable ordering.
    StableDefault,
    /// Orders by a semantic property key.
    Property {
        /// Property key used for ordering.
        key: String,
        /// Ordering direction.
        direction: SortDirection,
    },
}

/// Selects ascending or descending property ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum SortDirection {
    /// Lowest values first.
    Ascending,
    /// Highest values first.
    Descending,
}

/// Requests exact structural selection within one snapshot-bound scope.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct QuerySpec {
    /// Exact semantic scope to query.
    pub scope: QueryScope,
    /// Declarative structural predicate.
    pub expression: QueryExpr,
    /// Stable result ordering.
    pub ordering: QueryOrdering,
    /// Page bounds and opaque continuation.
    pub page: PageRequest,
    /// Detail level for returned rows.
    pub detail: DetailLevel,
}

/// One semantic row returned by an exact structural query.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct QueryRow {
    /// Semantic handle reusable by inspect/list/follow operations.
    pub target: TargetRef,
    /// Entity type when known.
    pub entity_type: Option<ObjectType>,
    /// Optional compact metadata.
    pub metadata: Option<String>,
}

/// Returns exact-scope structural query rows.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct QueryResult {
    /// Exact scope used by evaluation.
    pub scope: QueryScope,
    /// Bounded semantic rows.
    pub page: Page<QueryRow>,
    /// Advisory cost estimate.
    pub information_cost: Option<InformationCost>,
}

/// Provides declarative structural query execution.
pub trait StructuredQueryService {
    /// Evaluates one query specification without changing its snapshot scope.
    ///
    /// # Parameters
    /// - `self`: Query provider.
    /// - `specification`: Exact-scope structural query.
    ///
    /// # Returns
    /// A bounded page of semantic rows.
    ///
    /// # Errors
    /// Returns a structured error for unsupported expressions or invalid queries.
    fn query(&self, specification: &QuerySpec) -> Result<QueryResult, QueryError>;
}
