//! Executes find and structured query operations over explicit graph scopes.

use crate::{
    ContinuationToken, EntityInSnapshot, FindHit, FindRequest, FindResult, FindService,
    FindStrategy, GraphReadService, InformationCost, Page, QueryError, QueryExpr, QueryResult,
    QueryRow, QueryScope, QuerySpec, StructuredQueryService, TargetRef,
};
use scriberig_core::EntityId;
use scriberig_graph::{KnowledgeView, RevisionRecord};

impl<V: KnowledgeView> FindService for GraphReadService<V> {
    /// Executes bounded discovery over explicitly supplied graph entities.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `request`: Discovery request with an explicit entity scope.
    ///
    /// # Returns
    /// A bounded page of matching semantic handles.
    ///
    /// # Errors
    /// Returns an unsupported-capability error for whole-snapshot scopes or
    /// semantic strategies requiring an index.
    fn find(&self, request: &FindRequest) -> Result<FindResult, QueryError> {
        let entities = scope_entities(self.view(), &request.scope)?;
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
        let (start, end) = page_bounds(request.page.continuation.as_ref(), request.page.limit)?;
        let next = (end < hits.len()).then(|| ContinuationToken::new(end.to_string()));
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
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `specification`: Query request with an explicit entity scope.
    ///
    /// # Returns
    /// A bounded page of matching semantic rows.
    ///
    /// # Errors
    /// Returns an unsupported-capability error for whole-snapshot scopes or
    /// predicates requiring relation indexes.
    fn query(&self, specification: &QuerySpec) -> Result<QueryResult, QueryError> {
        let entities = scope_entities(self.view(), &specification.scope)?;
        let snapshot = specification.scope.snapshot();
        let mut rows = Vec::new();
        for entity in entities {
            let record = self.read_record(snapshot, entity)?;
            if matches_query(self, snapshot, &record, &specification.expression)? {
                rows.push(QueryRow {
                    target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
                    entity_type: Some(record.entity_type),
                    metadata: None,
                });
            }
        }
        let (start, end) = page_bounds(
            specification.page.continuation.as_ref(),
            specification.page.limit,
        )?;
        let next = (end < rows.len()).then(|| ContinuationToken::new(end.to_string()));
        Ok(QueryResult {
            scope: specification.scope.clone(),
            page: Page {
                items: rows.into_iter().skip(start).take(end - start).collect(),
                next,
            },
            information_cost: Some(InformationCost::default()),
        })
    }
}

/// Resolves scopes that can be evaluated without a graph-wide index.
///
/// # Parameters
/// - `scope`: Exact query scope.
///
/// # Returns
/// Explicit entity identities to evaluate.
///
/// # Errors
/// Returns unsupported capability for a complete-snapshot scope.
fn scope_entities<V: KnowledgeView>(
    view: &V,
    scope: &QueryScope,
) -> Result<Vec<EntityId>, QueryError> {
    match scope {
        QueryScope::Entity { entity, .. } => Ok(vec![*entity]),
        QueryScope::ExplicitEntities { entities, .. } => Ok(entities.clone()),
        QueryScope::Snapshot(snapshot) => {
            view.entities(*snapshot)
                .map_err(|_| QueryError::SnapshotUnavailable {
                    operation: crate::ReadOperation::Query,
                    snapshot: *snapshot,
                })
        }
    }
}

/// Evaluates the graph predicates supported by the direct view.
///
/// # Parameters
/// - `record`: Exact graph record under evaluation.
/// - `expression`: Structural predicate.
///
/// # Returns
/// `true` when the record matches.
///
/// # Errors
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
/// # Parameters
/// - `candidate`: Canonical property value.
/// - `expected`: Query scalar value.
///
/// # Returns
/// `true` when both values have equal supported scalar meaning.
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

/// Computes bounded page offsets from a provider-owned continuation token.
///
/// # Parameters
/// - `continuation`: Optional provider continuation token.
/// - `limit`: Maximum number of items to return.
///
/// # Returns
/// Start and exclusive end offsets.
///
/// # Errors
/// Returns an invalid-continuation error when the token is malformed.
fn page_bounds(
    continuation: Option<&ContinuationToken>,
    limit: usize,
) -> Result<(usize, usize), QueryError> {
    let start = continuation.map_or(0, |token| {
        token.as_str().parse::<usize>().unwrap_or(usize::MAX)
    });
    if start == usize::MAX {
        return Err(QueryError::InvalidContinuation {
            operation: crate::ReadOperation::Query,
        });
    }
    Ok((start, start.saturating_add(limit)))
}
