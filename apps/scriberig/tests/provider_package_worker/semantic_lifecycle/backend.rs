//! Implements a deterministic store-backed application path for a plugin semantic operation.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use scriberig_app::{
    ApplicationBackend, ApplicationContext, ApplicationError, ApplicationRequest,
    ApplicationResult, ApplyRequest, CapabilityExecution, OperationProvider,
    StoreOperationProvider, ValidationStatus,
};
use scriberig_core::EncodingVersion;
use scriberig_query::QueryValue;
use scriberig_store::{Revision, RevisionStore, SnapshotChange, SnapshotStore};
use scriberig_store_memory::MemoryStore;
use sr_change::{EntityDelta, Migration, Provenance, SourceKind};

/// Composes the plugin operation with a store-backed candidate lifecycle.
pub(super) struct LifecycleOperationBackend {
    /// Application requests received in execution order.
    requests: Arc<Mutex<Vec<ApplicationRequest>>>,
    /// Immutable store receiving the semantic candidate and promotion.
    store: Arc<MemoryStore>,
    /// Store-backed public operations for candidate application and promotion.
    operations: StoreOperationProvider<Arc<MemoryStore>>,
    /// Candidate handles that passed semantic migration validation.
    validated: Mutex<BTreeSet<String>>,
}

impl LifecycleOperationBackend {
    /// Creates a backend sharing the seeded semantic store and request recorder.
    ///
    /// - `store`: Immutable store used by application operations.
    /// - `requests`: Shared ordered request recorder.
    ///
    /// A fixture backend that registers plugin candidates and promotes them
    /// through the canonical store operation provider.
    ///
    /// # Returns
    /// A backend sharing the provided semantic store and request recorder.
    pub(super) fn new(
        store: Arc<MemoryStore>,
        requests: Arc<Mutex<Vec<ApplicationRequest>>>,
    ) -> Self {
        Self {
            requests,
            operations: StoreOperationProvider::new(Arc::clone(&store)),
            store,
            validated: Mutex::new(BTreeSet::new()),
        }
    }

    /// Checks that the applied snapshot contains the requested semantic rename.
    ///
    /// - `requests`: Earlier operation request context.
    /// - `apply`: Candidate application input.
    /// - `candidate`: Snapshot returned from the shared Apply operation.
    ///
    /// `true` when the candidate revision contains the exact requested value.
    ///
    /// # Returns
    /// Whether the immutable candidate contains the requested entity value.
    ///
    /// # Errors
    /// Returns an application error when the captured request or stored data is
    /// unavailable or malformed.
    fn validates_requested_rename(
        &self,
        requests: &[ApplicationRequest],
        apply: &ApplyRequest,
        candidate: scriberig_core::SnapshotId,
    ) -> Result<bool, ApplicationError> {
        let operation = requests.iter().find_map(|request| match request {
            ApplicationRequest::DomainOperation(operation) => Some(operation),
            _ => None,
        });
        let Some(operation) = operation else {
            return Err(ApplicationError::InvalidRequest {
                operation: "apply".to_owned(),
                message: "candidate lacks its source semantic operation".to_owned(),
            });
        };
        let Some(("name", QueryValue::Text(expected_name))) = operation
            .arguments
            .iter()
            .find(|(key, _)| key == "name")
            .map(|(key, value)| (key.as_str(), value))
        else {
            return Err(ApplicationError::InvalidRequest {
                operation: "apply".to_owned(),
                message: "candidate source lacks a typed name argument".to_owned(),
            });
        };
        if operation.subject.snapshot != apply.base_snapshot {
            return Ok(false);
        }
        let before = self
            .store
            .resolve(apply.base_snapshot, operation.subject.entity)
            .map_err(|error| ApplicationError::Internal {
                message: format!("fixture base resolution failed: {error:?}"),
            })?;
        let after = self
            .store
            .resolve(candidate, operation.subject.entity)
            .map_err(|error| ApplicationError::Internal {
                message: format!("fixture candidate resolution failed: {error:?}"),
            })?;
        if before == after {
            return Ok(false);
        }
        let Some(after) = after else {
            return Ok(false);
        };
        let revision = self
            .store
            .get(after)
            .map_err(|error| ApplicationError::Internal {
                message: format!("fixture candidate revision read failed: {error:?}"),
            })?
            .ok_or_else(|| ApplicationError::NotFound {
                subject: after.to_string(),
            })?;
        let payload: serde_json::Value =
            serde_json::from_slice(revision.payload()).map_err(|error| {
                ApplicationError::Internal {
                    message: format!("fixture candidate payload is invalid: {error}"),
                }
            })?;
        Ok(payload["name"] == expected_name.as_str())
    }
}

impl ApplicationBackend for LifecycleOperationBackend {
    /// Applies plugin operations to a store-backed migration lifecycle.
    ///
    /// # Parameters
    /// - `self`: Lifecycle recording backend.
    /// - `_context`: Trusted application context.
    /// - `request`: Public application request received from the caller.
    ///
    /// # Returns
    #[expect(
        clippy::too_many_lines,
        reason = "test backend enforces one complete operation lifecycle"
    )]
    /// The semantic candidate, validated apply result, or promoted commit.
    ///
    /// - `context`: Host authority context for the operation.
    /// - `request`: Exact semantic operation requested by the fixture.
    ///
    /// # Returns
    /// The operation-specific result accepted by the test workflow.
    ///
    /// # Errors
    /// Returns a safe error for an unexpected request or lifecycle order.
    fn execute(
        &self,
        context: &ApplicationContext,
        request: &ApplicationRequest,
    ) -> Result<ApplicationResult, ApplicationError> {
        let mut requests = self.requests.lock().expect("lifecycle request lock");
        requests.push(request.clone());
        match request {
            ApplicationRequest::DomainOperation(operation) => {
                let Some(("name", QueryValue::Text(name))) = operation
                    .arguments
                    .iter()
                    .find(|(key, _)| key == "name")
                    .map(|(key, value)| (key.as_str(), value))
                else {
                    return Err(ApplicationError::InvalidRequest {
                        operation: "domain_operation".to_owned(),
                        message: "fixture rename requires a typed name argument".to_owned(),
                    });
                };
                let before = self
                    .store
                    .resolve(operation.subject.snapshot, operation.subject.entity)
                    .map_err(|error| ApplicationError::Internal {
                        message: format!("fixture entity resolution failed: {error:?}"),
                    })?
                    .ok_or_else(|| ApplicationError::NotFound {
                        subject: operation.subject.entity.to_string(),
                    })?;
                let previous = self
                    .store
                    .get(before)
                    .map_err(|error| ApplicationError::Internal {
                        message: format!("fixture revision read failed: {error:?}"),
                    })?
                    .ok_or_else(|| ApplicationError::NotFound {
                        subject: before.to_string(),
                    })?;
                let mut payload: serde_json::Value = serde_json::from_slice(previous.payload())
                    .map_err(|error| ApplicationError::Internal {
                        message: format!("fixture semantic payload is invalid: {error}"),
                    })?;
                payload["name"] = serde_json::Value::String(name.clone());
                let after = Revision::new(
                    EncodingVersion::V1,
                    previous.object_type().clone(),
                    serde_json::to_vec(&payload).expect("fixture payload serializes"),
                )
                .map_err(|error| ApplicationError::Internal {
                    message: format!("fixture semantic revision is invalid: {error:?}"),
                })?;
                let after_id =
                    self.store
                        .put(after)
                        .map_err(|error| ApplicationError::Internal {
                            message: format!("fixture revision write failed: {error:?}"),
                        })?;
                let candidate = self
                    .store
                    .apply_changes(
                        operation.subject.snapshot,
                        &[SnapshotChange::Set(operation.subject.entity, after_id)],
                    )
                    .map_err(|error| ApplicationError::Internal {
                        message: format!("fixture candidate creation failed: {error:?}"),
                    })?;
                let migration = Migration::new(
                    operation.subject.snapshot,
                    candidate,
                    vec![
                        EntityDelta::new(operation.subject.entity, Some(before), Some(after_id))
                            .map_err(|error| ApplicationError::Internal {
                                message: format!("fixture migration delta is invalid: {error:?}"),
                            })?,
                    ],
                    Vec::new(),
                    Provenance {
                        source: SourceKind::Agent,
                        actor: None,
                        workspace: None,
                        projection: None,
                        validation: Vec::new(),
                        parents: Vec::new(),
                        metadata: std::collections::BTreeMap::new(),
                    },
                )
                .map_err(|error| ApplicationError::Internal {
                    message: format!("fixture migration is invalid: {error:?}"),
                })?;
                if !self
                    .operations
                    .register_migration("candidate:plugin-domain-operation", migration)
                {
                    return Err(ApplicationError::InvalidRequest {
                        operation: "domain_operation".to_owned(),
                        message: "fixture candidate handle already exists".to_owned(),
                    });
                }
                Ok(ApplicationResult::DomainOperation(CapabilityExecution {
                    change: "candidate:plugin-domain-operation".to_owned(),
                }))
            }
            ApplicationRequest::Apply(apply)
                if apply.change == "candidate:plugin-domain-operation" =>
            {
                let mut summary = self.operations.apply(context, apply)?;
                let valid = match summary.snapshot {
                    Some(candidate) => {
                        self.validates_requested_rename(&requests, apply, candidate)?
                    }
                    None => false,
                };
                summary.validation = Some(if valid {
                    ValidationStatus::Passed
                } else {
                    ValidationStatus::Failed
                });
                summary.validation_profile = Some("fixture-semantic-rename".to_owned());
                if valid {
                    self.validated
                        .lock()
                        .expect("validation lock")
                        .insert(apply.change.clone());
                }
                Ok(ApplicationResult::Apply(summary))
            }
            ApplicationRequest::Commit(commit)
                if commit.change == "candidate:plugin-domain-operation" =>
            {
                if !self
                    .validated
                    .lock()
                    .expect("validation lock")
                    .contains(&commit.change)
                {
                    return Err(ApplicationError::InvalidRequest {
                        operation: "commit".to_owned(),
                        message: "candidate was not validated through Apply".to_owned(),
                    });
                }
                self.operations
                    .commit(context, commit)
                    .map(ApplicationResult::Commit)
            }
            _ => Err(ApplicationError::InvalidRequest {
                operation: "fixture".to_owned(),
                message: "unexpected semantic lifecycle request".to_owned(),
            }),
        }
    }
}
