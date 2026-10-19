//! Defines durable intent, receipt, and reconciliation state for external effects.

use scriberig_core::{OperationId, PrincipalId, RevisionId, SnapshotId};

use crate::{AuthorizationDenial, Capability, CapabilityAction, CapabilityScope};

/// Immutable request to perform one external side effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEffectIntent {
    /// Principal responsible for the request.
    pub principal: PrincipalId,
    /// Connector-defined operation kind.
    pub operation_kind: String,
    /// Exact external destination identifier.
    pub destination: String,
    /// Stable retry key, when supported by the destination.
    pub idempotency_key: Option<String>,
    /// Semantic snapshot used as input.
    pub snapshot: SnapshotId,
    /// Exact revisions read by the operation.
    pub revisions: Vec<RevisionId>,
    /// Created application operation identity.
    pub operation: OperationId,
}

/// Explicit state of an external effect attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalEffectState {
    /// Intent has been recorded but no request was sent.
    Requested,
    /// A connector attempt was made.
    Attempted,
    /// Remote confirmation was received.
    ConfirmedExternally,
    /// Remote confirmation was reflected in local semantic state.
    ReflectedLocally,
    /// Failure was established before remote acceptance.
    Failed,
    /// The local process cannot determine the remote outcome.
    Unknown,
}

/// Non-secret evidence returned by an external connector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEffectReceipt {
    /// External operation identity, when supplied.
    pub external_operation_id: Option<String>,
    /// Remote resource reference, when supplied.
    pub resource: Option<String>,
    /// Non-secret response metadata.
    pub metadata: Vec<(String, String)>,
    /// Connector confirmation observation.
    pub confirmation: String,
    /// Idempotency key used for the request.
    pub idempotency_key: Option<String>,
}

/// One append-only external effect state transition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEffectEvent {
    /// Resulting state.
    pub state: ExternalEffectState,
    /// Optional receipt attached to the transition.
    pub receipt: Option<ExternalEffectReceipt>,
    /// Operation retaining provenance for this transition.
    pub operation: OperationId,
}

/// Append-only history for one external intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEffectHistory {
    /// Immutable external intent.
    pub intent: ExternalEffectIntent,
    /// State transitions in creation order.
    pub events: Vec<ExternalEffectEvent>,
}

impl ExternalEffectHistory {
    /// Creates a history in the requested state.
    ///
    /// - `intent`: Immutable effect intent.
    ///
    /// A history beginning in `Requested`.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new(intent: ExternalEffectIntent) -> Self {
        Self {
            events: vec![ExternalEffectEvent {
                operation: intent.operation.clone(),
                state: ExternalEffectState::Requested,
                receipt: None,
            }],
            intent,
        }
    }

    /// Appends a connector attempt.
    pub fn attempt(&mut self) {
        self.push(ExternalEffectState::Attempted, None);
    }

    /// Records an externally confirmed receipt.
    ///
    /// - `receipt`: Non-secret connector evidence.
    pub fn confirm(&mut self, receipt: ExternalEffectReceipt) {
        self.push(ExternalEffectState::ConfirmedExternally, Some(receipt));
    }

    /// Records that confirmation was reflected in local state.
    pub fn reflect_locally(&mut self) {
        self.push(ExternalEffectState::ReflectedLocally, None);
    }

    /// Records a definite pre-acceptance failure.
    pub fn fail(&mut self) {
        self.push(ExternalEffectState::Failed, None);
    }

    /// Records an outcome that requires connector reconciliation.
    pub fn unknown(&mut self) {
        self.push(ExternalEffectState::Unknown, None);
    }

    /// Returns the latest known state.
    ///
    /// The most recent state in the append-only history.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn state(&self) -> ExternalEffectState {
        self.events
            .last()
            .map_or(ExternalEffectState::Requested, |event| event.state)
    }

    /// Returns whether a confirmed operation must not be resent blindly.
    ///
    /// `true` when retry or reconciliation is still required.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn should_retry(&self) -> bool {
        matches!(
            self.state(),
            ExternalEffectState::Failed | ExternalEffectState::Unknown
        )
    }

    /// Appends one transition while retaining all prior provenance.
    ///
    /// - `state`: New effect state.
    /// - `receipt`: Optional connector evidence.
    fn push(&mut self, state: ExternalEffectState, receipt: Option<ExternalEffectReceipt>) {
        self.events.push(ExternalEffectEvent {
            state,
            receipt,
            operation: self.intent.operation.clone(),
        });
    }
}

/// Checks exact destination authority before connector invocation.
///
/// - `capabilities`: Capabilities presented by the principal.
/// - `destination`: Exact destination identifier.
/// - `snapshot`: Input snapshot bound to the request.
///
/// Unit when an external invocation capability covers the destination.
///
/// Returns a structured denial when no exact destination capability exists.
/// # Returns
/// The operation result.
/// # Errors
/// Returns an error when the operation fails.
pub fn authorize_external_effect(
    capabilities: &[Capability],
    destination: &str,
    snapshot: SnapshotId,
) -> Result<(), Box<AuthorizationDenial>> {
    let scope = CapabilityScope::ExternalDestination(destination.to_owned());
    if capabilities.iter().any(|capability| {
        capability.covers(
            &CapabilityAction::InvokeExternal,
            &scope,
            Some(snapshot),
            None,
        )
    }) {
        return Ok(());
    }
    Err(Box::new(AuthorizationDenial {
        principal: None,
        action: CapabilityAction::InvokeExternal,
        scope,
        reason: "destination capability is required".to_owned(),
        alternatives: Vec::new(),
        approval_possible: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CapabilityConstraints, CapabilityScope};

    /// Creates a deterministic effect intent fixture.
    ///
    /// A secret-free external effect intent.
    /// # Returns
    /// The operation result.
    fn intent() -> ExternalEffectIntent {
        ExternalEffectIntent {
            principal: PrincipalId::new("test").expect("principal"),
            operation_kind: "publish".to_owned(),
            destination: "remote://one".to_owned(),
            idempotency_key: Some("same-request".to_owned()),
            snapshot: SnapshotId::new([1; 32]),
            revisions: vec![RevisionId::new([2; 32])],
            operation: OperationId::new("effect").expect("operation"),
        }
    }

    /// Verifies unknown outcomes remain retryable and history is retained.
    #[test]
    fn unknown_outcome_is_reconcilable() {
        let mut history = ExternalEffectHistory::new(intent());
        history.attempt();
        history.unknown();
        assert_eq!(history.state(), ExternalEffectState::Unknown);
        assert!(history.should_retry());
        assert_eq!(history.events.len(), 3);
    }

    /// Verifies external authority cannot be granted by a project capability.
    #[test]
    fn authorization_requires_exact_destination() {
        let capability = Capability {
            scope: CapabilityScope::ExternalDestination("remote://one".to_owned()),
            actions: [CapabilityAction::InvokeExternal].into_iter().collect(),
            constraints: CapabilityConstraints::unrestricted(),
        };
        assert!(
            authorize_external_effect(
                std::slice::from_ref(&capability),
                "remote://one",
                SnapshotId::new([1; 32]),
            )
            .is_ok()
        );
        assert!(
            authorize_external_effect(&[capability], "remote://two", SnapshotId::new([1; 32]),)
                .is_err()
        );
    }
}
