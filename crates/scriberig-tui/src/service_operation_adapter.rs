//! Adapts application operations to the TUI service contract.

#![allow(clippy::wildcard_imports)]

use super::*;

/// Adapts the application backend to operation-oriented TUI services.
pub struct ApplicationOperationAdapter<B> {
    service: ApplicationService<B>,
}

/// Adapts the public protocol client to the TUI inspection boundary.
impl<B> ApplicationOperationAdapter<B> {
    /// Creates an operation adapter around an application service.
    #[must_use]
    pub const fn new(service: ApplicationService<B>) -> Self {
        Self { service }
    }
}

impl<B: ApplicationBackend> TuiService for ApplicationOperationAdapter<B> {
    /// Reports the semantic operations exposed by the embedded application.
    ///
    /// # Parameters
    /// - `self`: Embedded application adapter.
    /// - `_context`: Authorization context used by the surrounding runtime.
    ///
    /// # Returns
    /// Capability descriptors used to keep the command palette honest.
    ///
    /// # Errors
    /// This adapter has no separate capability-discovery failure path.
    fn runtime_capabilities(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<RuntimeCapability>, ApplicationError> {
        let capabilities = [
            ("inspect_entity", "Inspect an exact semantic entity."),
            ("explorer_list", "List entities in an exact snapshot."),
            ("semantic_find", "Search entities in an exact snapshot."),
            ("history", "Read immutable snapshot history."),
            ("diff", "Compare two exact snapshots."),
            ("session_branch_tree", "Read session and branch state."),
            ("operation_status", "Observe operation lifecycle state."),
            ("operation_cancel", "Cancel a cancellable operation."),
            ("agent_start", "Start provider-neutral agent work."),
            ("agent_continue", "Continue work on a session branch."),
        ];
        capabilities
            .into_iter()
            .map(|(id, description)| {
                Ok(RuntimeCapability::new(
                    scriberig_app::CapabilityId::new(id).map_err(|error| {
                        ApplicationError::Internal {
                            message: error.to_owned(),
                        }
                    })?,
                    description,
                    true,
                ))
            })
            .collect()
    }

    /// Delegates inspection through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `entity`: Entity to inspect.
    /// - `snapshot`: Optional exact snapshot.
    ///
    /// # Returns
    /// The inspection result.
    ///
    /// # Errors
    /// Returns the application inspection error.
    fn inspect_entity(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: Option<SnapshotId>,
    ) -> Result<InspectResult, ApplicationError> {
        self.service.inspect(
            context,
            &InspectRequest {
                entity,
                snapshot: snapshot.map_or(SnapshotResolution::Current, SnapshotResolution::At),
                detail: scriberig_app::DetailLevel::Detailed,
            },
        )
    }

    /// Lists the shared snapshot entity collection for Explorer navigation.
    ///
    /// # Parameters
    /// - `context`: Application context.
    /// - `entity`: Entity owning the collection.
    /// - `snapshot`: Exact snapshot context.
    /// - `continuation`: Optional continuation token.
    ///
    /// # Returns
    /// A bounded Explorer page.
    ///
    /// # Errors
    /// Returns the application listing error.
    fn list_explorer(
        &self,
        context: &ApplicationContext,
        entity: EntityId,
        snapshot: SnapshotId,
        continuation: Option<scriberig_query::ContinuationToken>,
    ) -> Result<crate::explorer::ExplorerPage, ApplicationError> {
        let result = self.service.execute(
            context,
            &ApplicationRequest::List(scriberig_query::ListRequest {
                subject: scriberig_query::EntityInSnapshot { entity, snapshot },
                collection: scriberig_query::CollectionRef {
                    owner: entity,
                    collection_type: "snapshot.entities".to_owned(),
                },
                page: scriberig_query::PageRequest {
                    continuation,
                    ..scriberig_query::PageRequest::default()
                },
                detail: scriberig_query::DetailLevel::Compact,
            }),
        )?;
        let ApplicationResult::List(result) = result else {
            return Err(ApplicationError::Internal {
                message: "explorer listing returned a different result kind".to_owned(),
            });
        };
        let rows = result
            .page
            .items
            .into_iter()
            .filter_map(|entry| match entry.target {
                scriberig_query::TargetRef::Snapshot(target) => {
                    Some(crate::explorer::ExplorerRow {
                        selection: crate::selection::EntitySelection {
                            entity: target.entity,
                            revision: None,
                            snapshot: Some(target.snapshot),
                        },
                        label: entry.label.unwrap_or_else(|| target.entity.to_string()),
                        inspectable: true,
                    })
                }
                scriberig_query::TargetRef::Revision(_) => None,
            })
            .filter(|row| row.selection.entity != entity)
            .collect();
        Ok(crate::explorer::ExplorerPage {
            rows,
            next: result.page.next,
        })
    }

    /// Executes exact-text discovery through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context.
    /// - `snapshot`: Exact discovery scope.
    /// - `query`: Search text.
    ///
    /// # Returns
    /// A bounded find model.
    ///
    /// # Errors
    /// Returns the application discovery error.
    fn find_entities(
        &self,
        context: &ApplicationContext,
        snapshot: SnapshotId,
        query: &str,
    ) -> Result<crate::view::find::FindModel, ApplicationError> {
        let result = self.service.execute(
            context,
            &ApplicationRequest::Find(scriberig_query::FindRequest {
                scope: scriberig_query::QueryScope::Snapshot(snapshot),
                query: query.to_owned(),
                strategy: scriberig_query::FindStrategy::ExactText,
                page: scriberig_query::PageRequest::default(),
                detail: scriberig_query::DetailLevel::Compact,
            }),
        )?;
        let ApplicationResult::Find(result) = result else {
            return Err(ApplicationError::Internal {
                message: "find returned a different result kind".to_owned(),
            });
        };
        Ok(crate::view::find::FindModel::from_result(query, &result))
    }

    /// Delegates shared operations through the public application facade.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `request`: Public operation request.
    ///
    /// # Returns
    /// The operation result.
    ///
    /// # Errors
    /// Returns the application operation error.
    fn execute_operation(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        self.service.execute(context, request)
    }

    /// Reads an operation record from the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the lookup.
    /// - `operation`: Operation identity to resolve.
    ///
    /// # Returns
    /// The current operation record.
    ///
    /// # Errors
    /// Returns a lookup error when the operation is unknown.
    fn operation_status(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().get(operation)
    }

    /// Cancels an operation through the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for the cancellation.
    /// - `operation`: Operation identity to cancel.
    ///
    /// # Returns
    /// The cancelled operation record.
    ///
    /// # Errors
    /// Returns a cancellation error when the operation is unknown.
    fn cancel_operation(
        &self,
        _context: &ApplicationContext,
        operation: &scriberig_core::OperationId,
    ) -> Result<scriberig_app::OperationRecord, ApplicationError> {
        self.service.operations().cancel(operation)
    }

    /// Returns lifecycle events retained by the shared application registry.
    ///
    /// # Parameters
    /// - `_context`: Application context for event access.
    ///
    /// # Returns
    /// Retained application events.
    ///
    /// # Errors
    /// This adapter does not return an event access error.
    fn application_events(
        &self,
        _context: &ApplicationContext,
    ) -> Result<Vec<scriberig_app::ApplicationEvent>, ApplicationError> {
        Ok(self.service.operations().events())
    }

    /// Reads a session through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity to read.
    ///
    /// # Returns
    /// The requested session.
    ///
    /// # Errors
    /// Returns the application session error.
    fn read_session(
        &self,
        context: &ApplicationContext,
        session: EntityId,
    ) -> Result<scriberig_session::Session, ApplicationError> {
        scriberig_app::SessionBackend::read_session(
            &self.service,
            context,
            &scriberig_app::SessionRequest { session },
        )
    }

    /// Submits an agent command through the shared application boundary.
    ///
    /// # Parameters
    /// - `context`: Application context for the request.
    /// - `session`: Session identity receiving the prompt.
    /// - `branch`: Optional branch identity.
    /// - `prompt`: Agent prompt to submit.
    ///
    /// # Returns
    /// The accepted operation identity.
    ///
    /// # Errors
    /// Returns the application agent error.
    fn submit_agent_command(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_core::OperationId, ApplicationError> {
        scriberig_app::SessionBackend::submit_agent_command(
            &self.service,
            context,
            &scriberig_app::AgentCommandRequest {
                session,
                branch,
                prompt: prompt.to_owned(),
            },
        )
    }

    /// Submits local agent work and preserves the accepted branch identity.
    ///
    /// # Parameters
    /// - `context`: Application context for the operation.
    /// - `session`: Stable session identity.
    /// - `branch`: Optional continuation branch.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// Stable operation, session, and branch handles.
    ///
    /// # Errors
    /// Returns the application submission or session-read error.
    fn submit_agent_handle(
        &self,
        context: &ApplicationContext,
        session: EntityId,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<super::AgentHandle, ApplicationError> {
        let operation = self.submit_agent_command(context, session, branch, prompt)?;
        let branch = branch.or_else(|| {
            scriberig_app::SessionBackend::read_session(
                &self.service,
                context,
                &scriberig_app::SessionRequest { session },
            )
            .ok()
            .and_then(|value| value.branches.last().map(|value| value.id))
        });
        Ok(super::AgentHandle {
            operation,
            session,
            branch,
        })
    }
}
