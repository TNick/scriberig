//! Implements universal reads over a snapshot-bound graph view.

use crate::{
    ContinuationToken, DetailLevel, EntityInSnapshot, FollowDirection, FollowRequest, FollowResult,
    InspectRequest, InspectService, Inspection, ListRequest, ListResult, NavigationEntry, Page,
    QueryError, ReadCapability, TargetRef, TraverseService,
};
use scriberig_core::{EntityId, SnapshotId};
use scriberig_graph::{CollectionValue, KnowledgeView};

/// Provides inspect, list, and follow operations over one graph view.
pub struct GraphReadService<V> {
    view: V,
}

impl<V> GraphReadService<V> {
    /// Creates a read service around a storage-independent graph view.
    ///
    /// # Parameters
    /// - `view`: Snapshot-bound graph provider.
    ///
    /// # Returns
    /// A universal read service.
    #[must_use]
    pub const fn new(view: V) -> Self {
        Self { view }
    }

    /// Returns the wrapped graph view.
    ///
    /// # Parameters
    /// - `self`: Read service to inspect.
    ///
    /// # Returns
    /// The graph provider.
    #[must_use]
    pub const fn view(&self) -> &V {
        &self.view
    }
}

impl<V: KnowledgeView> InspectService for GraphReadService<V> {
    /// Inspects one entity at an exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact snapshot or revision inspection request.
    ///
    /// # Returns
    /// Compact or structural inspection metadata.
    ///
    /// # Errors
    /// Returns a structured query error when the target cannot be resolved.
    fn inspect(&self, request: &InspectRequest) -> Result<Inspection, QueryError> {
        let (snapshot, revision, record) = match request.target {
            TargetRef::Snapshot(target) => {
                let revision = self
                    .view
                    .resolve(target.snapshot, target.entity)
                    .map_err(|_| QueryError::SnapshotUnavailable {
                        operation: crate::ReadOperation::Inspect,
                        snapshot: target.snapshot,
                    })?
                    .ok_or(QueryError::NotFound {
                        operation: crate::ReadOperation::Inspect,
                        target: request.target,
                    })?;
                let record = self
                    .view
                    .read_revision(target.snapshot, target.entity)
                    .map_err(|_| QueryError::Internal {
                        operation: crate::ReadOperation::Inspect,
                        message: "graph view failed to read revision".to_owned(),
                    })?
                    .ok_or(QueryError::NotFound {
                        operation: crate::ReadOperation::Inspect,
                        target: request.target,
                    })?;
                (Some(target.snapshot), revision, record)
            }
            TargetRef::Revision(target) => {
                let record = self
                    .view
                    .read_exact_revision(target.entity, target.revision)
                    .map_err(|_| QueryError::Internal {
                        operation: crate::ReadOperation::Inspect,
                        message: "graph view failed to read exact revision".to_owned(),
                    })?
                    .ok_or(QueryError::NotFound {
                        operation: crate::ReadOperation::Inspect,
                        target: request.target,
                    })?;
                (None, target.revision, record)
            }
        };
        let relations: Vec<ReadCapability> = record
            .relations
            .keys()
            .map(|relation| ReadCapability {
                id: relation.clone(),
                description: "follow relation".to_owned(),
            })
            .collect();
        let collections: Vec<ReadCapability> = record
            .collections
            .keys()
            .map(|collection| ReadCapability {
                id: collection.clone(),
                description: "list collection".to_owned(),
            })
            .collect();
        Ok(Inspection {
            subject: request.target,
            snapshot,
            revision: Some(revision),
            entity_type: Some(record.entity_type),
            label: None,
            description: None,
            properties: if request.detail == DetailLevel::Identity {
                Vec::new()
            } else {
                record
                    .properties
                    .keys()
                    .map(|key| (key.clone(), "present".to_owned()))
                    .collect()
            },
            counts: vec![("relations".to_owned(), relations.len() as u64)],
            relations,
            collections,
            views: Vec::new(),
            operations: Vec::new(),
            provenance: Vec::new(),
            information_cost: None,
            detail: request.detail,
        })
    }
}

impl<V: KnowledgeView> TraverseService for GraphReadService<V> {
    /// Lists one named collection from an exact snapshot.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact collection listing request.
    ///
    /// # Returns
    /// One bounded page of semantic navigation entries.
    ///
    /// # Errors
    /// Returns a structured query error for unavailable collections or views.
    fn list(&self, request: &ListRequest) -> Result<ListResult, QueryError> {
        let record = self.read_record(request.subject.snapshot, request.subject.entity)?;
        let collection = record
            .collections
            .get(&request.collection.collection_type)
            .ok_or_else(|| QueryError::InvalidCollection {
                operation: crate::ReadOperation::List,
                collection: request.collection.clone(),
            })?;
        let members = match collection {
            CollectionValue::Ordered(values) => values.clone(),
            CollectionValue::Unordered(values) => values.iter().copied().collect(),
        };
        let (start, end) = page_bounds(
            request.page.continuation.as_ref(),
            request.page.limit,
            "list",
            request.subject.snapshot,
        )?;
        let end = end.min(members.len());
        if start > end {
            return Err(QueryError::InvalidContinuation {
                operation: crate::ReadOperation::List,
            });
        }
        let items = members[start..end]
            .iter()
            .copied()
            .map(|entity| self.navigation(entity, request.subject.snapshot))
            .collect();
        Ok(ListResult {
            subject: request.subject,
            snapshot: request.subject.snapshot,
            collection: request.collection.clone(),
            page: Page {
                items,
                next: (end < members.len())
                    .then(|| ContinuationToken::for_page("list", request.subject.snapshot, end)),
            },
        })
    }

    /// Follows one outgoing or indexed incoming relation.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact relation traversal request.
    ///
    /// # Returns
    /// One bounded page of semantic navigation entries.
    ///
    /// # Errors
    /// Returns a structured query error when the relation or reverse index is
    /// unavailable.
    fn follow(&self, request: &FollowRequest) -> Result<FollowResult, QueryError> {
        let targets = match request.direction {
            FollowDirection::Outgoing => {
                let record = self.read_record(request.subject.snapshot, request.subject.entity)?;
                record
                    .relations
                    .get(&request.relation.relation_type)
                    .ok_or_else(|| QueryError::InvalidRelation {
                        operation: crate::ReadOperation::Follow,
                        relation: request.relation.clone(),
                    })?
                    .targets
                    .clone()
            }
            FollowDirection::Incoming => self
                .view
                .reverse_relations(request.subject.snapshot, request.subject.entity)
                .map_err(|_| QueryError::DerivedDataUnavailable {
                    operation: crate::ReadOperation::Follow,
                    entity: request.subject.entity,
                })?
                .into_iter()
                .filter(|relation| {
                    relation.relation_type.as_str() == request.relation.relation_type
                })
                .map(|relation| relation.source)
                .collect(),
        };
        let (start, end) = page_bounds(
            request.page.continuation.as_ref(),
            request.page.limit,
            "follow",
            request.subject.snapshot,
        )?;
        let end = end.min(targets.len());
        if start > end {
            return Err(QueryError::InvalidContinuation {
                operation: crate::ReadOperation::Follow,
            });
        }
        let items = targets[start..end]
            .iter()
            .copied()
            .map(|entity| self.navigation(entity, request.subject.snapshot))
            .collect();
        Ok(FollowResult {
            subject: request.subject,
            snapshot: request.subject.snapshot,
            relation: request.relation.clone(),
            direction: request.direction,
            metadata: None,
            page: Page {
                items,
                next: (end < targets.len())
                    .then(|| ContinuationToken::for_page("follow", request.subject.snapshot, end)),
            },
        })
    }
}

impl<V: KnowledgeView> GraphReadService<V> {
    /// Builds a compact navigation entry from one exact revision.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `entity`: Entity identity for the navigation target.
    /// - `snapshot`: Exact snapshot containing the target.
    ///
    /// # Returns
    /// A navigation entry with reusable identity and available metadata.
    ///
    /// This method intentionally does not require a full target revision;
    /// listing remains useful when only generic type metadata is available.
    fn navigation(&self, entity: EntityId, snapshot: SnapshotId) -> NavigationEntry {
        let record = self.view.read_revision(snapshot, entity).ok().flatten();
        let entity_type = record
            .as_ref()
            .map(|record| record.entity_type.clone())
            .or_else(|| self.view.entity_type(snapshot, entity).ok().flatten());
        let label = record.as_ref().and_then(|record| {
            property_text(record, "label")
                .or_else(|| property_text(record, "title"))
                .or_else(|| property_text(record, "name"))
        });
        let description = record
            .as_ref()
            .and_then(|record| property_text(record, "description"));
        let metadata = record.as_ref().map(|record| {
            format!(
                "relations:{} collections:{}",
                record.relations.len(),
                record.collections.len()
            )
        });
        NavigationEntry {
            target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
            entity_type,
            label,
            description,
            metadata,
        }
    }

    /// Reads one exact graph record and maps backend failures to query errors.
    ///
    /// # Parameters
    /// - `self`: Graph-backed read service.
    /// - `snapshot`: Exact snapshot to read.
    /// - `entity`: Entity identity to resolve.
    /// - `crate`: Internal visibility marker.
    ///
    /// # Returns
    /// The exact graph record.
    ///
    /// # Errors
    /// Returns a structured query error when the snapshot or entity is absent.
    pub(crate) fn read_record(
        &self,
        snapshot: SnapshotId,
        entity: EntityId,
    ) -> Result<scriberig_graph::RevisionRecord, QueryError> {
        self.view
            .read_revision(snapshot, entity)
            .map_err(|_| QueryError::SnapshotUnavailable {
                operation: crate::ReadOperation::Inspect,
                snapshot,
            })?
            .ok_or(QueryError::NotFound {
                operation: crate::ReadOperation::Inspect,
                target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
            })
    }
}

/// Reads a text property when its canonical value is a string.
///
/// # Parameters
/// - `entity`: Entity identity for the navigation target.
/// - `snapshot`: Exact snapshot containing the target.
///
/// # Returns
/// A compact navigation entry.
/// - `record`: Argument named `record`.
/// - `key`: Argument named `key`.
fn property_text(record: &scriberig_graph::RevisionRecord, key: &str) -> Option<String> {
    match record.properties.get(key) {
        Some(scriberig_core::KnowledgeValue::String(value)) => Some(value.clone()),
        _ => None,
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
/// - `operation`: Argument named `operation`.
/// - `snapshot`: Argument named `snapshot`.
fn page_bounds(
    continuation: Option<&ContinuationToken>,
    limit: usize,
    operation: &str,
    snapshot: SnapshotId,
) -> Result<(usize, usize), QueryError> {
    if !(1..=crate::MAX_PAGE_LIMIT).contains(&limit) {
        return Err(QueryError::InvalidQuery {
            message: format!("page limit must be between 1 and {}", crate::MAX_PAGE_LIMIT),
        });
    }
    let start = continuation.map_or(0, |token| {
        token.offset_for(operation, snapshot).unwrap_or(usize::MAX)
    });
    if start == usize::MAX {
        return Err(QueryError::InvalidContinuation {
            operation: match operation {
                "follow" => crate::ReadOperation::Follow,
                _ => crate::ReadOperation::List,
            },
        });
    }
    Ok((start, start.saturating_add(limit)))
}
