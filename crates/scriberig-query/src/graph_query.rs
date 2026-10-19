//! Executes find and structured query operations over explicit graph scopes.

use crate::{
    ContinuationToken, EntityInSnapshot, FindHit, FindRequest, FindResult, FindService,
    FindStrategy, GraphReadService, InformationCost, Page, QueryError, QueryExpr, QueryOrdering,
    QueryResult, QueryRow, QueryScope, QuerySpec, StructuredQueryService, TargetRef,
};
use scriberig_core::EntityId;
use scriberig_graph::{KnowledgeView, RevisionRecord};
use std::cmp::Ordering;

#[path = "graph_query_pagination.rs"]
mod graph_query_pagination;
use graph_query_pagination::page_bounds;

impl<V: KnowledgeView> FindService for GraphReadService<V> {
    /// Executes bounded discovery over explicitly supplied graph entities.
    ///
    /// - `self`: Graph-backed read service.
    /// - `request`: Discovery request with an explicit entity scope.
    ///
    /// A bounded page of matching semantic handles.
    ///
    /// Returns an unsupported-capability error for whole-snapshot scopes or
    /// semantic strategies requiring an index.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn find(&self, request: &FindRequest) -> Result<FindResult, QueryError> {
        if request.query.trim().is_empty() {
            return Err(QueryError::InvalidQuery {
                message: "find query must not be empty".to_owned(),
            });
        }
        let entities = scope_entities(self.view(), &request.scope, crate::ReadOperation::Find)?;
        if !matches!(
            request.strategy,
            FindStrategy::ExactText | FindStrategy::StructuredName
        ) {
            return Err(QueryError::UnsupportedCapability {
                operation: crate::ReadOperation::Find,
                capability: format!("find:{:?}", request.strategy),
            });
        }
        let snapshot = request.scope.snapshot();
        let mut hits = Vec::new();
        for entity in entities {
            let record = self.read_record(snapshot, entity)?;
            let type_name = record.entity_type.to_string();
            if type_name.contains(&request.query)
                || record
                    .properties
                    .keys()
                    .any(|key| key.contains(&request.query))
            {
                hits.push(FindHit {
                    target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
                    entity_type: Some(record.entity_type),
                    label: None,
                    preview: None,
                    score: None,
                    anchor: None,
                    provenance: None,
                });
            }
        }
        let (start, end) = page_bounds(
            request.page.continuation.as_ref(),
            request.page.limit,
            "find",
            snapshot,
        )?;
        let next = (end < hits.len()).then(|| ContinuationToken::for_page("find", snapshot, end));
        Ok(FindResult {
            scope: request.scope.clone(),
            strategy: request.strategy.clone(),
            page: Page {
                items: hits.into_iter().skip(start).take(end - start).collect(),
                next,
            },
            information_cost: Some(InformationCost::default()),
        })
    }
}

impl<V: KnowledgeView> StructuredQueryService for GraphReadService<V> {
    /// Evaluates basic structural predicates over explicitly supplied entities.
    ///
    /// - `self`: Graph-backed read service.
    /// - `specification`: Query request with an explicit entity scope.
    ///
    /// A bounded page of matching semantic rows.
    ///
    /// Returns an unsupported-capability error for whole-snapshot scopes or
    /// predicates requiring relation indexes.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn query(&self, specification: &QuerySpec) -> Result<QueryResult, QueryError> {
        let entities = scope_entities(
            self.view(),
            &specification.scope,
            crate::ReadOperation::Query,
        )?;
        let snapshot = specification.scope.snapshot();
        let mut rows = Vec::new();
        for entity in entities {
            let record = self.read_record(snapshot, entity)?;
            if matches_query(self, snapshot, &record, &specification.expression)? {
                rows.push((entity, record));
            }
        }
        if let QueryOrdering::Property { key, direction } = &specification.ordering {
            rows.sort_by(|left, right| {
                let ordering = compare_property_values(
                    left.1.properties.get(key),
                    right.1.properties.get(key),
                );
                match direction {
                    crate::SortDirection::Ascending => ordering,
                    crate::SortDirection::Descending => ordering.reverse(),
                }
            });
        }
        let (start, end) = page_bounds(
            specification.page.continuation.as_ref(),
            specification.page.limit,
            "query",
            snapshot,
        )?;
        let next = (end < rows.len()).then(|| ContinuationToken::for_page("query", snapshot, end));
        Ok(QueryResult {
            scope: specification.scope.clone(),
            page: Page {
                items: rows
                    .into_iter()
                    .skip(start)
                    .take(end - start)
                    .map(|(entity, record)| QueryRow {
                        target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
                        entity_type: Some(record.entity_type),
                        metadata: None,
                    })
                    .collect(),
                next,
            },
            information_cost: Some(InformationCost::default()),
        })
    }
}

/// Compares optional property values for deterministic query ordering.
///
/// - `left`: First optional property value.
/// - `right`: Second optional property value.
///
/// A stable ordering with missing values before present values.
/// # Returns
/// The operation result.
fn compare_property_values(
    left: Option<&scriberig_core::KnowledgeValue>,
    right: Option<&scriberig_core::KnowledgeValue>,
) -> Ordering {
    match (left, right) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(left), Some(right)) => compare_values(left, right),
    }
}

/// Compares property values by semantic variant and value rather than display text.
///
/// - `left`: First canonical property value.
/// - `right`: Second canonical property value.
///
/// A deterministic total ordering for supported property values.
/// # Returns
/// The operation result.
fn compare_values(
    left: &scriberig_core::KnowledgeValue,
    right: &scriberig_core::KnowledgeValue,
) -> Ordering {
    let variant_order = value_variant(left).cmp(&value_variant(right));
    if variant_order != Ordering::Equal {
        return variant_order;
    }
    match (left, right) {
        (scriberig_core::KnowledgeValue::Null, scriberig_core::KnowledgeValue::Null) => {
            Ordering::Equal
        }
        (
            scriberig_core::KnowledgeValue::Bool(left),
            scriberig_core::KnowledgeValue::Bool(right),
        ) => left.cmp(right),
        (
            scriberig_core::KnowledgeValue::SignedInteger(left),
            scriberig_core::KnowledgeValue::SignedInteger(right),
        ) => left.cmp(right),
        (
            scriberig_core::KnowledgeValue::UnsignedInteger(left),
            scriberig_core::KnowledgeValue::UnsignedInteger(right),
        ) => left.cmp(right),
        (
            scriberig_core::KnowledgeValue::Float(left),
            scriberig_core::KnowledgeValue::Float(right),
        ) => left.partial_cmp(right).unwrap_or(Ordering::Equal),
        (
            scriberig_core::KnowledgeValue::String(left),
            scriberig_core::KnowledgeValue::String(right),
        ) => left.cmp(right),
        (
            scriberig_core::KnowledgeValue::Bytes(left),
            scriberig_core::KnowledgeValue::Bytes(right),
        ) => left.cmp(right),
        (
            scriberig_core::KnowledgeValue::List(left),
            scriberig_core::KnowledgeValue::List(right),
        ) => left
            .iter()
            .zip(right)
            .map(|(left, right)| compare_values(left, right))
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        (
            scriberig_core::KnowledgeValue::Record(left),
            scriberig_core::KnowledgeValue::Record(right),
        ) => left
            .iter()
            .zip(right)
            .map(|((left_key, left), (right_key, right))| {
                left_key
                    .cmp(right_key)
                    .then_with(|| compare_values(left, right))
            })
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or_else(|| left.len().cmp(&right.len())),
        _ => Ordering::Equal,
    }
}

/// Assigns canonical ordering ranks to property value variants.
///
/// - `value`: Property value to classify.
///
/// The stable variant rank used before comparing values.
const fn value_variant(value: &scriberig_core::KnowledgeValue) -> u8 {
    match value {
        scriberig_core::KnowledgeValue::Null => 0,
        scriberig_core::KnowledgeValue::Bool(_) => 1,
        scriberig_core::KnowledgeValue::SignedInteger(_) => 2,
        scriberig_core::KnowledgeValue::UnsignedInteger(_) => 3,
        scriberig_core::KnowledgeValue::Float(_) => 4,
        scriberig_core::KnowledgeValue::String(_) => 5,
        scriberig_core::KnowledgeValue::Bytes(_) => 6,
        scriberig_core::KnowledgeValue::List(_) => 7,
        scriberig_core::KnowledgeValue::Record(_) => 8,
    }
}

/// Resolves scopes that can be evaluated without a graph-wide index.
///
/// - `scope`: Exact query scope.
///
/// Explicit entity identities to evaluate.
///
/// Returns unsupported capability for a complete-snapshot scope.
fn scope_entities<V: KnowledgeView>(
    view: &V,
    scope: &QueryScope,
    operation: crate::ReadOperation,
) -> Result<Vec<EntityId>, QueryError> {
    match scope {
        QueryScope::Entity { entity, .. } => Ok(vec![*entity]),
        QueryScope::ExplicitEntities { entities, .. } if entities.is_empty() => {
            Err(QueryError::InvalidScope {
                operation,
                scope: scope.clone(),
            })
        }
        QueryScope::ExplicitEntities { entities, .. } => Ok(entities.clone()),
        QueryScope::Snapshot(snapshot) => {
            view.entities(*snapshot)
                .map_err(|_| QueryError::SnapshotUnavailable {
                    operation,
                    snapshot: *snapshot,
                })
        }
    }
}

/// Evaluates the graph predicates supported by the direct view.
///
/// - `record`: Exact graph record under evaluation.
/// - `expression`: Structural predicate.
///
/// `true` when the record matches.
///
/// Returns unsupported capability for predicates requiring indexes or values
/// not exposed by the graph record contract.
fn matches_query<V: KnowledgeView>(
    service: &GraphReadService<V>,
    snapshot: scriberig_core::SnapshotId,
    record: &RevisionRecord,
    expression: &QueryExpr,
) -> Result<bool, QueryError> {
    match expression {
        QueryExpr::All => Ok(true),
        QueryExpr::TypeIs(expected) => Ok(&record.entity_type == expected),
        QueryExpr::PropertyExists { key } => Ok(record.properties.contains_key(key)),
        QueryExpr::And(expressions) => expressions
            .iter()
            .map(|expression| matches_query(service, snapshot, record, expression))
            .try_fold(true, |matched, value| Ok(matched && value?)),
        QueryExpr::Or(expressions) => expressions
            .iter()
            .map(|expression| matches_query(service, snapshot, record, expression))
            .try_fold(false, |matched, value| Ok(matched || value?)),
        QueryExpr::Not(expression) => Ok(!matches_query(service, snapshot, record, expression)?),
        QueryExpr::PropertyEq { key, value } => Ok(record
            .properties
            .get(key)
            .is_some_and(|candidate| query_value_matches(candidate, value))),
        QueryExpr::MemberOf { collection } => Ok(record.collections.contains_key(collection)),
        QueryExpr::Related {
            relation,
            direction,
            target,
        } => {
            if matches!(direction, crate::QueryDirection::Incoming) {
                return Err(QueryError::UnsupportedCapability {
                    operation: crate::ReadOperation::Query,
                    capability: "reverse relation query".to_owned(),
                });
            }
            record
                .relations
                .get(relation)
                .map(|value| value.targets.clone())
                .unwrap_or_default()
                .into_iter()
                .try_fold(false, |matched, entity| {
                    let related = service.read_record(snapshot, entity)?;
                    Ok(matched || matches_query(service, snapshot, &related, target)?)
                })
        }
    }
}

/// Compares a canonical scalar property with a query scalar.
///
/// - `candidate`: Canonical property value.
/// - `expected`: Query scalar value.
///
/// `true` when both values have equal supported scalar meaning.
/// # Returns
/// The operation result.
fn query_value_matches(
    candidate: &scriberig_core::KnowledgeValue,
    expected: &crate::QueryValue,
) -> bool {
    match (candidate, expected) {
        (scriberig_core::KnowledgeValue::String(actual), crate::QueryValue::Text(expected)) => {
            actual == expected
        }
        (
            scriberig_core::KnowledgeValue::SignedInteger(actual),
            crate::QueryValue::Integer(expected),
        ) => actual == expected,
        (scriberig_core::KnowledgeValue::Bool(actual), crate::QueryValue::Boolean(expected)) => {
            actual == expected
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "graph_query_tests.rs"]
mod tests;
