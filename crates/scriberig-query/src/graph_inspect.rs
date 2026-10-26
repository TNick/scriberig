//! Implements cancellable exact-target inspection over a snapshot-bound graph view.

use crate::{
    DetailLevel, InspectRequest, InspectService, Inspection, QueryError, ReadCapability, TargetRef,
};
use scriberig_graph::KnowledgeView;

use super::GraphReadService;

impl<V: KnowledgeView> InspectService for GraphReadService<V> {
    /// Inspects one entity at an exact snapshot.
    ///
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact snapshot or revision inspection request.
    ///
    /// Compact or structural inspection metadata.
    ///
    /// Returns a structured query error when the target cannot be resolved.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn inspect(&self, request: &InspectRequest) -> Result<Inspection, QueryError> {
        self.inspect_with_cancellation(request, None)
    }

    /// Inspects a target with cancellation checkpoints between graph reads.
    ///
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact target and detail policy.
    /// - `cancelled`: Caller-owned cancellation state.
    ///
    /// Exact inspection metadata when all required reads complete.
    ///
    /// # Returns
    /// The semantic inspection result.
    ///
    /// # Errors
    /// Returns `Cancelled` when the caller cancels between graph reads, or a
    /// structured query failure when graph state is unavailable.
    fn inspect_cancellable(
        &self,
        request: &InspectRequest,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Inspection, QueryError> {
        self.inspect_with_cancellation(request, Some(cancelled))
    }
}

impl<V: KnowledgeView> GraphReadService<V> {
    /// Inspects a target and checks cancellation around each graph read.
    ///
    /// - `self`: Graph-backed read service.
    /// - `request`: Exact target and detail policy.
    /// - `cancelled`: Optional caller-owned cancellation state.
    ///
    /// Exact inspection metadata when all required reads complete.
    ///
    /// # Returns
    /// The semantic inspection result.
    ///
    /// # Errors
    /// Returns `Cancelled` when cancellation is observed between graph reads,
    /// or a structured query failure when graph state is unavailable.
    fn inspect_with_cancellation(
        &self,
        request: &InspectRequest,
        cancelled: Option<&std::sync::atomic::AtomicBool>,
    ) -> Result<Inspection, QueryError> {
        ensure_inspection_active(cancelled)?;
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
                ensure_inspection_active(cancelled)?;
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
                ensure_inspection_active(cancelled)?;
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
                ensure_inspection_active(cancelled)?;
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
        ensure_inspection_active(cancelled)?;
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

/// Returns a cancellation error when a graph inspection should stop.
///
/// - `cancelled`: Optional cancellation state for this inspection.
///
/// `Ok(())` while active, otherwise an inspect cancellation error.
///
/// # Returns
/// The current cancellation status.
///
/// # Errors
/// Returns `Cancelled` when the supplied flag is set.
fn ensure_inspection_active(
    cancelled: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(), QueryError> {
    if cancelled.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire)) {
        return Err(QueryError::Cancelled {
            operation: crate::ReadOperation::Inspect,
        });
    }
    Ok(())
}
