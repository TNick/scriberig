//! Provides storage-backed exact inspection for the application service.

use crate::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest, ApplicationResult,
};
use scriberig_query::{
    CollectionRef, EntityInSnapshot, Inspection, NavigationEntry, Page, TargetRef,
};
use scriberig_store::{RevisionStore, RootStore, SnapshotStore};

/// Provides exact inspection over canonical revision, snapshot, and root stores.
pub struct StoreInspectionBackend<S> {
    store: S,
    current_root: scriberig_core::RootName,
}

impl<S> StoreInspectionBackend<S> {
    /// Creates an inspection backend using the conventional `main` root.
    ///
    /// # Parameters
    /// - `store`: Revision, snapshot, and root backend.
    ///
    /// # Returns
    /// A storage-backed inspection provider.
    ///
    /// # Panics
    /// Panics only if the fixed root name is invalid.
    #[must_use]
    pub fn new(store: S) -> Self {
        Self {
            store,
            current_root: scriberig_core::RootName::new("main").expect("main is a valid root name"),
        }
    }

    /// Returns the underlying storage backend.
    ///
    /// # Parameters
    /// - `self`: Storage-backed provider.
    ///
    /// # Returns
    /// The backend used for reads.
    #[must_use]
    pub const fn store(&self) -> &S {
        &self.store
    }
}

impl<S> ApplicationBackend for StoreInspectionBackend<S>
where
    S: RevisionStore + SnapshotStore + RootStore,
{
    /// Executes an exact snapshot or revision inspection.
    ///
    /// # Parameters
    /// - `self`: Storage-backed inspection provider.
    /// - `_context`: Trusted application context.
    /// - `request`: Universal application request.
    ///
    /// # Returns
    /// Compact inspection metadata from canonical storage.
    ///
    /// # Errors
    /// Returns a structured not-found or backend failure.
    #[allow(clippy::too_many_lines)]
    fn execute(
        &self,
        _context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let ApplicationRequest::Inspect(request) = request else {
            let ApplicationRequest::List(request) = request else {
                if let ApplicationRequest::Find(request) = request {
                    return find_snapshot_entities(&self.store, request);
                }
                return Err(ApplicationError::UnsupportedOperation {
                    operation: "storage backend read operation".to_owned(),
                });
            };
            if request.collection.collection_type != "snapshot.entities" {
                return Err(ApplicationError::UnsupportedCapability {
                    operation: "list".to_owned(),
                    capability: request.collection.collection_type.clone(),
                });
            }
            let entities = self
                .store
                .iter(request.subject.snapshot)
                .map_err(|error| storage_snapshot_error(&error))?;
            let start = request
                .page
                .continuation
                .as_ref()
                .map_or(Some(0), |token| {
                    token.offset_for("list", request.subject.snapshot)
                })
                .ok_or_else(|| ApplicationError::InvalidRequest {
                    operation: "list".to_owned(),
                    message: "invalid snapshot entity continuation".to_owned(),
                })?;
            let end = start.saturating_add(request.page.limit).min(entities.len());
            if start > end {
                return Err(ApplicationError::InvalidRequest {
                    operation: "list".to_owned(),
                    message: "snapshot entity continuation is out of range".to_owned(),
                });
            }
            let items = entities
                .iter()
                .skip(start)
                .take(end.saturating_sub(start))
                .copied()
                .map(|(entity, revision)| {
                    let entity_type = self
                        .store
                        .get(revision)
                        .ok()
                        .flatten()
                        .map(|value| value.object_type().clone());
                    NavigationEntry {
                        target: TargetRef::Snapshot(EntityInSnapshot {
                            entity,
                            snapshot: request.subject.snapshot,
                        }),
                        entity_type,
                        label: None,
                        description: None,
                        metadata: Some("snapshot entity".to_owned()),
                    }
                })
                .collect();
            return Ok(ApplicationResult::List(scriberig_query::ListResult {
                subject: request.subject,
                snapshot: request.subject.snapshot,
                collection: CollectionRef {
                    owner: request.collection.owner,
                    collection_type: request.collection.collection_type.clone(),
                },
                page: Page {
                    items,
                    next: (end < entities.len()).then(|| {
                        scriberig_query::ContinuationToken::for_page(
                            "list",
                            request.subject.snapshot,
                            end,
                        )
                    }),
                },
            }));
        };
        let (subject, revision_id, snapshot) = match request.target {
            TargetRef::Snapshot(target) => {
                let revision = self
                    .store
                    .resolve(target.snapshot, target.entity)
                    .map_err(|error| storage_snapshot_error(&error))?
                    .ok_or_else(|| ApplicationError::NotFound {
                        subject: target.entity.to_string(),
                    })?;
                (request.target, revision, Some(target.snapshot))
            }
            TargetRef::Revision(target) => (request.target, target.revision, None),
        };
        let revision = self
            .store
            .get(revision_id)
            .map_err(|error| storage_revision_error(&error))?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: revision_id.to_string(),
            })?;
        Ok(ApplicationResult::Inspect(Inspection {
            subject,
            snapshot,
            revision: Some(revision.id()),
            entity_type: Some(revision.object_type().clone()),
            label: None,
            description: None,
            properties: Vec::new(),
            counts: Vec::new(),
            relations: Vec::new(),
            collections: Vec::new(),
            views: Vec::new(),
            operations: Vec::new(),
            provenance: Vec::new(),
            information_cost: None,
            detail: request.detail,
        }))
    }

    /// Resolves the conventional current `main` root.
    ///
    /// # Parameters
    /// - `self`: Storage-backed inspection provider.
    /// - `_context`: Trusted application context.
    ///
    /// # Returns
    /// The exact snapshot promoted on `main`.
    ///
    /// # Errors
    /// Returns not-found when `main` has not been promoted.
    fn current_snapshot(
        &self,
        _context: &ApplicationContext,
    ) -> Result<scriberig_core::SnapshotId, ApplicationError> {
        self.store
            .read_root(&self.current_root)
            .ok_or_else(|| ApplicationError::NotFound {
                subject: self.current_root.to_string(),
            })
    }
}

/// Finds entities by exact text over stable identity and type metadata.
///
/// # Parameters
/// - `store`: Revision and snapshot storage backend.
/// - `request`: Snapshot-scoped exact-text discovery request.
///
/// # Returns
/// A bounded public find result.
///
/// # Errors
/// Returns a structured application error for unsupported scopes or pages.
fn find_snapshot_entities<S>(
    store: &S,
    request: &scriberig_query::FindRequest,
) -> Result<ApplicationResult, ApplicationError>
where
    S: RevisionStore + SnapshotStore,
{
    let scriberig_query::QueryScope::Snapshot(snapshot) = request.scope else {
        return Err(ApplicationError::InvalidRequest {
            operation: "find".to_owned(),
            message: "storage find requires a snapshot scope".to_owned(),
        });
    };
    if request.page.continuation.is_some() {
        return Err(ApplicationError::InvalidRequest {
            operation: "find".to_owned(),
            message: "storage find has no next page".to_owned(),
        });
    }
    let query = request.query.to_lowercase();
    let hits = store
        .iter(snapshot)
        .map_err(|error| storage_snapshot_error(&error))?
        .into_iter()
        .filter_map(|(entity, revision)| {
            let value = store.get(revision).ok().flatten()?;
            let entity_text = entity.to_string();
            let type_text = value.object_type().to_string();
            (entity_text.to_lowercase().contains(&query)
                || type_text.to_lowercase().contains(&query))
            .then_some(scriberig_query::FindHit {
                target: TargetRef::Snapshot(EntityInSnapshot { entity, snapshot }),
                entity_type: Some(value.object_type().clone()),
                label: None,
                preview: Some(type_text),
                score: None,
                anchor: None,
                provenance: None,
            })
        })
        .take(request.page.limit)
        .collect();
    Ok(ApplicationResult::Find(scriberig_query::FindResult {
        scope: request.scope.clone(),
        strategy: request.strategy.clone(),
        page: Page {
            items: hits,
            next: None,
        },
        information_cost: None,
    }))
}

/// Maps a revision backend failure to a safe application error.
///
/// # Parameters
/// - `error`: Revision backend failure.
///
/// # Returns
/// Safe application error preserving no backend internals.
fn storage_revision_error(error: &scriberig_store::StoreError) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("revision backend failure: {error:?}"),
    }
}

/// Maps a snapshot backend failure to a safe application error.
///
/// # Parameters
/// - `error`: Snapshot backend failure.
///
/// # Returns
/// Safe application error preserving no backend internals.
fn storage_snapshot_error(error: &scriberig_store::SnapshotError) -> ApplicationError {
    ApplicationError::Internal {
        message: format!("snapshot backend failure: {error:?}"),
    }
}
