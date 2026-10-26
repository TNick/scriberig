//! Projects upstream credential readiness through `ScribeRig` host authorization.
//!
//! The shared runtime projection owns the credential-source classification and
//! binds each row to the upstream catalog generation it resolved. This module
//! only supplies the runner's shared catalog handle and its host-authorized
//! credential authority.

use super::NativeTuiAgentRunner;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

/// Owns the host-facing provider lifecycle source and its safe auth projection.
///
/// # Public members
/// None.
///
/// # Private members
/// - `events`: Upstream provider lifecycle event source.
/// - `receiver`: Subscriber used to consume lifecycle updates.
/// - `authentication`: Latest provider-auth result by provider/model and catalog.
/// - `credential_revision`: Monotonic host revision for credential changes.
/// - `registry_revision`: Last shared secret-registry revision observed.
pub(super) struct ProviderLifecycleState {
    /// Upstream provider lifecycle event source.
    events: rpi_model_runtime::ProviderLifecycleEvents,
    /// Subscriber used to consume lifecycle updates.
    receiver: Mutex<std::sync::mpsc::Receiver<rpi_model_runtime::ProviderLifecycleEvent>>,
    /// Latest provider-auth result by provider/model and catalog revision.
    authentication:
        Mutex<BTreeMap<(String, String), (u64, rpi_model_runtime::ProviderAuthenticationOutcome)>>,
    /// Monotonic host revision for credential changes.
    credential_revision: AtomicU64,
    /// Last shared secret-registry revision observed by this process.
    registry_revision: Mutex<Option<u64>>,
}

impl ProviderLifecycleState {
    /// Creates a lifecycle source and subscribes the readiness projection.
    ///
    /// # Returns
    /// A fresh host lifecycle state with no cached authentication results.
    pub(super) fn new() -> Self {
        let events = rpi_model_runtime::ProviderLifecycleEvents::new();
        let receiver = events.subscribe();
        Self {
            events,
            receiver: Mutex::new(receiver),
            authentication: Mutex::new(BTreeMap::new()),
            credential_revision: AtomicU64::new(0),
            registry_revision: Mutex::new(None),
        }
    }

    /// Creates an invocation sink bound to one observed catalog revision.
    ///
    /// # Parameters
    /// - `catalog_revision`: Catalog revision used to resolve the invocation.
    ///
    /// # Returns
    /// An upstream lifecycle context for the provider request.
    pub(super) fn context(
        &self,
        catalog_revision: u64,
    ) -> scriberig_agent_runtime::upstream_invocation::ProviderLifecycleContext {
        scriberig_agent_runtime::upstream_invocation::ProviderLifecycleContext {
            events: self.events.clone(),
            catalog_revision,
        }
    }

    /// Notifies subscribers and invalidates cached auth after a host credential change.
    ///
    /// # Parameters
    /// - `provider_id`: Upstream provider whose credential changed.
    pub(super) fn notify_credential_changed(&self, provider_id: &str) {
        let revision = self.credential_revision.fetch_add(1, Ordering::Relaxed) + 1;
        self.events
            .notify_credential_changed(rpi_model_runtime::ProviderCredentialChange {
                provider_id: provider_id.to_owned(),
                credential_revision: revision,
            });
        self.consume_events();
    }

    /// Invalidates provider auth after another process changes managed credentials.
    ///
    /// # Parameters
    /// - `revision`: Persisted revision read from the shared protected registry.
    /// - `providers`: Provider IDs present in the current catalog snapshot.
    pub(super) fn observe_registry_revision(
        &self,
        revision: u64,
        providers: impl IntoIterator<Item = String>,
    ) {
        let changed = {
            let mut observed = self
                .registry_revision
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let changed = observed.is_some_and(|previous| previous != revision);
            *observed = Some(revision);
            changed
        };
        if changed {
            for provider_id in providers
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
            {
                self.notify_credential_changed(&provider_id);
            }
        }
    }

    /// Adds current-generation upstream auth outcomes to safe readiness rows.
    ///
    /// # Parameters
    /// - `projection`: Credential readiness from the shared upstream catalog.
    ///
    /// # Returns
    /// Readiness rows augmented only with current-generation auth outcomes.
    pub(super) fn apply(
        &self,
        mut projection: scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection,
    ) -> scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
        self.consume_events();
        let revision = projection.generation.revision();
        let mut authentication = self
            .authentication
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        authentication.retain(|_, (observed_revision, _)| *observed_revision == revision);
        for row in &mut projection.rows {
            row.authentication = authentication
                .get(&(row.provider_id.clone(), row.model_id.clone()))
                .map(|(_, outcome)| match outcome {
                    rpi_model_runtime::ProviderAuthenticationOutcome::Accepted => {
                        scriberig_protocol::ProviderAuthenticationStatus::Accepted
                    }
                    rpi_model_runtime::ProviderAuthenticationOutcome::Rejected => {
                        scriberig_protocol::ProviderAuthenticationStatus::Rejected
                    }
                });
        }
        projection
    }

    /// Drains auth results and credential-change invalidations from upstream.
    fn consume_events(&self) {
        let receiver = self.receiver.lock().unwrap_or_else(PoisonError::into_inner);
        let mut authentication = self
            .authentication
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        loop {
            match receiver.try_recv() {
                Ok(rpi_model_runtime::ProviderLifecycleEvent::AuthenticationResult(result)) => {
                    authentication.insert(
                        (result.provider_id, result.model_id),
                        (result.catalog_revision, result.outcome),
                    );
                }
                Ok(rpi_model_runtime::ProviderLifecycleEvent::CredentialChanged(change)) => {
                    authentication.retain(|(provider, _), _| provider != &change.provider_id);
                }
                Err(
                    std::sync::mpsc::TryRecvError::Empty
                    | std::sync::mpsc::TryRecvError::Disconnected,
                ) => {
                    return;
                }
            }
        }
    }
}

/// Builds credential-safe status rows for each exact provider model endpoint.
///
/// - `runner`: Provider catalog and host-authorized credential source.
///
/// Per-model readiness rows with their catalog generation, and no endpoint or
/// credential values.
/// # Returns
/// The safe readiness projection.
pub(super) fn project(
    runner: &NativeTuiAgentRunner,
) -> scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
    let projection = scriberig_agent_runtime::provider_readiness::project_readiness_with(
        &runner.model_catalog,
        |model| {
            let connection_credentials =
                crate::agent_config::authorized_connection_credential_provider(model);
            let credentials = match connection_credentials {
                Ok(Some(credentials)) => Some(credentials),
                Ok(None) => runner.upstream_credentials.clone(),
                Err(_) => return Err(()),
            };
            match credentials.as_ref() {
                Some(credentials) => {
                    scriberig_agent_runtime::protected_credential::credential_source_for_model(
                        credentials.as_ref(),
                        model,
                    )
                    .map_err(|_| ())
                }
                None => Ok(rpi_model_runtime::readiness_from_catalog(
                    &model.provider,
                    rpi_model_runtime::CredentialSource::Missing,
                    std::slice::from_ref(model),
                )),
            }
        },
    );
    if let Ok(locations) = scriberig_secrets::registry::RegistryLocations::from_process()
        && let Ok(registry) = locations.load()
    {
        runner.provider_lifecycle.observe_registry_revision(
            registry.credential_revision,
            projection.rows.iter().map(|row| row.provider_id.clone()),
        );
    }
    runner.provider_lifecycle.apply(projection)
}

#[cfg(test)]
mod tests {

    use super::ProviderLifecycleState;
    use rpi_ai::ProviderAuthenticationResult;
    use rpi_model_runtime::ProviderAuthenticationOutcome;
    use scriberig_app::ModelCatalogGeneration;
    use scriberig_protocol::{
        ProviderAuthenticationStatus, ProviderCredentialReadiness, ProviderReadiness,
    };

    /// Builds one readiness projection at an exact catalog revision.
    ///
    /// # Parameters
    /// - `revision`: Upstream catalog revision to bind to the row.
    ///
    /// # Returns
    /// A single safe provider/model readiness row.
    fn projection(
        revision: u64,
    ) -> scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
        scriberig_agent_runtime::provider_readiness::ProviderReadinessProjection {
            generation: ModelCatalogGeneration::from_revision(revision),
            rows: vec![ProviderReadiness {
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                credential: ProviderCredentialReadiness::Available,
                authentication: None,
            }],
        }
    }

    /// Projects upstream auth evidence only for its generation and clears it on rotation.
    #[test]
    fn authentication_projection_is_generation_bound_and_credential_changes_invalidate_it() {
        let lifecycle = ProviderLifecycleState::new();
        lifecycle.observe_registry_revision(1, vec!["provider-a".to_owned()]);
        lifecycle.context(4).events.report_agent_result(
            ProviderAuthenticationResult {
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                outcome: ProviderAuthenticationOutcome::Rejected,
            },
            4,
        );

        let projected = lifecycle.apply(projection(4));
        assert_eq!(
            projected.rows[0].authentication,
            Some(ProviderAuthenticationStatus::Rejected)
        );
        assert_eq!(
            lifecycle.apply(projection(5)).rows[0].authentication,
            None,
            "a newer catalog generation must not inherit old auth evidence"
        );

        lifecycle.context(5).events.report_agent_result(
            ProviderAuthenticationResult {
                provider_id: "provider-a".to_owned(),
                model_id: "model-a".to_owned(),
                outcome: ProviderAuthenticationOutcome::Accepted,
            },
            5,
        );
        assert_eq!(
            lifecycle.apply(projection(5)).rows[0].authentication,
            Some(ProviderAuthenticationStatus::Accepted)
        );
        lifecycle.observe_registry_revision(2, vec!["provider-a".to_owned()]);
        assert_eq!(
            lifecycle.apply(projection(5)).rows[0].authentication,
            None,
            "credential changes must invalidate provider-auth results"
        );
    }

    /// Invalidates authentication after another process advances the registry revision.
    #[test]
    fn observed_registry_revision_notifies_each_provider_once_per_change() {
        let lifecycle = ProviderLifecycleState::new();
        let receiver = lifecycle.events.subscribe();
        let providers = || vec!["provider-a".to_owned(), "provider-b".to_owned()];

        lifecycle.observe_registry_revision(1, providers());
        lifecycle.observe_registry_revision(1, providers());
        assert!(
            receiver.try_recv().is_err(),
            "initial revision is a baseline"
        );

        lifecycle.observe_registry_revision(2, providers());
        let mut changed = [
            receiver.try_recv().expect("provider-a notification"),
            receiver.try_recv().expect("provider-b notification"),
        ];
        changed.sort_by_key(|event| match event {
            rpi_model_runtime::ProviderLifecycleEvent::CredentialChanged(change) => {
                change.provider_id.clone()
            }
            rpi_model_runtime::ProviderLifecycleEvent::AuthenticationResult(_) => String::new(),
        });
        assert!(matches!(
            &changed[0],
            rpi_model_runtime::ProviderLifecycleEvent::CredentialChanged(change)
                if change.provider_id == "provider-a"
        ));
        assert!(matches!(
            &changed[1],
            rpi_model_runtime::ProviderLifecycleEvent::CredentialChanged(change)
                if change.provider_id == "provider-b"
        ));
        assert!(receiver.try_recv().is_err(), "one event per provider");
    }
}
