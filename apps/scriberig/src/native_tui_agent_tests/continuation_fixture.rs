//! Composes the durable service, native runner, and continuation provider used
//! by the root continuation lifecycle tests.

use std::path::PathBuf;
use std::sync::Arc;

use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;
use scriberig_app::{
    ApplicationBackendComposition, ApplicationContext, ApplicationError, ApplicationService,
    CapabilityContext, CapabilityId, SessionBackend, SessionExecutionSelection,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_session::{BranchStatus, Session, SessionStore, Turn};
use scriberig_store_sqlite::SqliteStore;
use scriberig_tui::AgentCommandRunner;

use super::super::{NativeScenario, NativeTuiAgentRunner};
use super::model_catalog_from_models;
use crate::native_tui_agent::continuation_scenario::ContinuationScenarioProvider;

/// Backend composition used by the fixture service.
type FixtureBackend = ApplicationBackendComposition<
    StoreInspectionBackend<Arc<SqliteStore>>,
    StoreOperationProvider<Arc<SqliteStore>>,
>;

/// Grants sufficient to submit and continue a root agent branch.
const BASE_GRANTS: [&str; 5] = [
    "agent_start",
    "agent_continue",
    "agent_plan",
    "inspect.entity",
    "session_branch_tree",
];

/// One composed root-continuation fixture: service, runner, provider, session.
pub(super) struct LifecycleFixture {
    /// Temporary directory holding the durable stores.
    pub(super) temp_root: PathBuf,
    /// Shared application service under test.
    pub(super) service: ApplicationService<FixtureBackend>,
    /// Durable session store for assertions.
    pub(super) session_store: Arc<scriberig_session_sqlite::SqliteSessionStore>,
    /// Scenario provider registered for the `"local"` connection.
    pub(super) provider: Arc<ContinuationScenarioProvider>,
    /// Native root runner whose cache is under test.
    pub(super) runner: NativeTuiAgentRunner,
    /// Canonical session identity used by every turn.
    pub(super) session: EntityId,
    /// Provider registrations keyed by host connection identity.
    registrations: Vec<(String, Arc<ContinuationScenarioProvider>)>,
    /// Upstream catalog rows the runner may resolve.
    catalog: Vec<rpi_ai::Model>,
    /// Provider-native model id configured by the runner.
    runtime_model: String,
    /// Shared durable project store used by application tools.
    project_store: Arc<SqliteStore>,
}

/// Builds one fixture with an empty session and the supplied provider bindings.
///
/// # Parameters
/// - `catalog`: Upstream catalog rows the runner may resolve.
/// - `connections`: Connection identity to scenario provider registrations.
/// - `runtime_model`: Provider-native model id configured by the runner.
///
/// # Returns
/// A composed fixture with isolated durable stores.
pub(super) fn lifecycle_fixture(
    catalog: Vec<rpi_ai::Model>,
    connections: Vec<(String, Arc<ContinuationScenarioProvider>)>,
    runtime_model: &str,
) -> LifecycleFixture {
    let temp_root =
        std::env::temp_dir().join(format!("scriberig-continuation-{0}", EntityId::new()));
    std::fs::create_dir_all(&temp_root).expect("continuation fixture root");
    let project_store = Arc::new(
        SqliteStore::open(temp_root.join("project.sqlite")).expect("continuation project store"),
    );
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&project_store)),
        StoreOperationProvider::new(Arc::clone(&project_store)),
    );
    let session_store = Arc::new(
        scriberig_session_sqlite::SqliteSessionStore::open(temp_root.join("session.sqlite"))
            .expect("continuation session store"),
    );
    let session_repository: Arc<dyn SessionStore> =
        Arc::clone(&session_store) as Arc<dyn SessionStore>;
    let session = EntityId::from_bytes([77; 16]);
    let service = ApplicationService::new(backend)
        .with_session_store(session_repository)
        .with_upstream_models(catalog.clone());
    let log_connection = connections
        .iter()
        .find(|(connection, _)| connection == "local")
        .or_else(|| connections.first())
        .expect("at least one provider registration");
    let scenario_provider = Arc::clone(&log_connection.1);
    let providers = connections
        .iter()
        .map(|(connection, provider)| {
            (
                connection.clone(),
                NativeScenario::Continuation(Arc::clone(provider)),
            )
        })
        .collect();
    let registrations = connections;
    let runner = NativeTuiAgentRunner {
        providers,
        model: runtime_model.to_owned(),
        upstream_credentials: None,
        upstream_enabled: false,
        model_catalog: model_catalog_from_models(catalog.clone()).clone(),
        provider_extension_runtime: scriberig_app::ProviderExtensionRuntime::with_model_catalog(
            model_catalog_from_models(catalog.clone()).clone(),
        ),
        plugin_command_worker: std::sync::Mutex::new(None),
        extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox::new(),
        provider_lifecycle:
            crate::native_tui_agent::provider_readiness::ProviderLifecycleState::new(),
        model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
        project_store: Arc::clone(&project_store),
        active_coordinators: std::sync::Mutex::default(),
        continuations: EphemeralContinuationOwner::default(),
    };
    let context = LifecycleFixture::context(&[]);
    SessionBackend::save_session(
        &service,
        &context,
        &Session {
            id: session,
            branches: Vec::new(),
            goals: Vec::new(),
            tasks: Vec::new(),
            plans: Vec::new(),
            execution: SessionExecutionSelection::default(),
        },
    )
    .expect("persist empty session");
    LifecycleFixture {
        temp_root,
        service,
        session_store,
        provider: scenario_provider,
        runner,
        session,
        registrations,
        catalog,
        runtime_model: runtime_model.to_owned(),
        project_store: Arc::clone(&project_store),
    }
}

impl LifecycleFixture {
    /// Builds a second runner with an empty continuation cache.
    ///
    /// This models a process restart: the same durable session and provider
    /// configuration must seed from canonical `ScribeRig` context.
    ///
    /// # Returns
    /// A native runner sharing the fixture's stores and provider bindings.
    pub(super) fn fresh_runner(&self) -> NativeTuiAgentRunner {
        NativeTuiAgentRunner {
            providers: self
                .registrations
                .iter()
                .map(|(connection, provider)| {
                    (
                        connection.clone(),
                        NativeScenario::Continuation(Arc::clone(provider)),
                    )
                })
                .collect(),
            model: self.runtime_model.clone(),
            upstream_credentials: None,
            upstream_enabled: false,
            model_catalog: model_catalog_from_models(self.catalog.clone()).clone(),
            provider_extension_runtime: scriberig_app::ProviderExtensionRuntime::with_model_catalog(
                model_catalog_from_models(self.catalog.clone()).clone(),
            ),
            plugin_command_worker: std::sync::Mutex::new(None),
            extension_component_mailbox: rpi_extensions::ExtensionStatusMailbox::new(),
            provider_lifecycle:
                crate::native_tui_agent::provider_readiness::ProviderLifecycleState::new(),
            model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
            project_store: Arc::clone(&self.project_store),
            active_coordinators: std::sync::Mutex::default(),
            continuations: EphemeralContinuationOwner::default(),
        }
    }

    /// Builds one authorized context, adding any extra inert grants.
    ///
    /// # Parameters
    /// - `extra`: Additional capability ids that do not authorize the turn.
    ///
    /// # Returns
    /// An application context carrying the base and extra grants.
    pub(super) fn context(extra: &[&str]) -> ApplicationContext {
        let mut grants = BASE_GRANTS
            .iter()
            .map(|grant| (*grant).to_owned())
            .collect::<Vec<_>>();
        grants.extend(extra.iter().map(|grant| (*grant).to_owned()));
        Self::authorized_context(&grants)
    }

    /// Builds one authorized context from explicit grant ids.
    ///
    /// # Parameters
    /// - `grants`: Capability ids granted to the caller.
    ///
    /// # Returns
    /// An application context carrying the supplied grants.
    fn authorized_context(grants: &[String]) -> ApplicationContext {
        ApplicationContext {
            capabilities: CapabilityContext::new(
                grants
                    .iter()
                    .map(|grant| CapabilityId::new(grant).expect("capability id")),
            ),
            ..ApplicationContext::empty()
        }
    }

    /// Submits one prompt through the production root runner path.
    ///
    /// # Parameters
    /// - `context`: Trusted caller authorization for the turn.
    /// - `branch`: Existing branch to continue, when any.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// The accepted handle, or the application failure.
    /// # Errors
    /// Returns an authorization, provider, or application failure from the turn.
    pub(super) fn submit(
        &self,
        context: &ApplicationContext,
        branch: Option<EntityId>,
        prompt: &str,
    ) -> Result<scriberig_tui::AgentHandle, ApplicationError> {
        self.runner
            .submit(&self.service, context, self.session, branch, prompt)
    }

    /// Returns the branch identities currently cached by the runner.
    ///
    /// # Returns
    /// Sorted canonical root-branch keys holding an ephemeral handle.
    pub(super) fn cached_branches(&self) -> Vec<EntityId> {
        self.runner.continuations.branches()
    }

    /// Replaces the durable session execution selection.
    ///
    /// # Parameters
    /// - `provider_id`: Selected provider identity, when any.
    /// - `model_id`: Selected provider-native model id, when any.
    /// - `thinking_level`: Selected thinking id, when any.
    pub(super) fn set_selection(
        &self,
        provider_id: Option<&str>,
        model_id: Option<&str>,
        thinking_level: Option<&str>,
    ) {
        let mut session = self.durable();
        session.execution = SessionExecutionSelection {
            revision: session.execution.revision + 1,
            provider_id: provider_id.map(str::to_owned),
            model_id: model_id.map(str::to_owned),
            thinking_level: thinking_level.map(str::to_owned),
            history: Vec::new(),
        };
        SessionBackend::save_session(&self.service, &Self::context(&[]), &session)
            .expect("persist execution selection");
    }

    /// Re-opens a finished branch so its canonical root identity can re-enter.
    ///
    /// The product creates an anchored child branch for a follow-up prompt on a
    /// completed branch, so the runner's same-root identity is exercised by
    /// re-entering a branch that is still active.
    ///
    /// # Parameters
    /// - `branch`: Canonical root branch identity to re-open.
    pub(super) fn reactivate(&self, branch: EntityId) {
        let mut session = self.durable();
        let target = session
            .branches
            .iter_mut()
            .find(|candidate| candidate.id == branch)
            .expect("root branch exists");
        target.status = BranchStatus::Active;
        SessionBackend::save_session(&self.service, &Self::context(&[]), &session)
            .expect("persist re-opened branch");
    }

    /// Appends one completed turn to a branch, changing its durable context.
    ///
    /// # Parameters
    /// - `branch`: Canonical branch receiving the external turn.
    /// - `human`: Human instruction text.
    /// - `agent`: Assistant output text.
    pub(super) fn append_turn(&self, branch: EntityId, human: &str, agent: &str) {
        let mut session = self.durable();
        let ordinal = {
            let target = session
                .branches
                .iter()
                .find(|candidate| candidate.id == branch)
                .expect("root branch exists");
            target.turns.len().saturating_add(1)
        };
        let target = session
            .branches
            .iter_mut()
            .find(|candidate| candidate.id == branch)
            .expect("root branch exists");
        target.turns.push(Turn {
            id: EntityId::from_bytes([78; 16]),
            ordinal: u64::try_from(ordinal).expect("turn ordinal"),
            human: Some(scriberig_session::HumanInput {
                text: human.to_owned(),
            }),
            agent: Some(scriberig_session::AgentOutput {
                text: agent.to_owned(),
            }),
            tool_invocation: None,
            tool_result: None,
        });
        SessionBackend::save_session(&self.service, &Self::context(&[]), &session)
            .expect("persist external turn");
    }

    /// Reloads the durable session.
    ///
    /// # Returns
    /// The authoritative session state.
    pub(super) fn durable(&self) -> Session {
        self.session_store
            .load(self.session)
            .expect("load continuation session")
            .expect("continuation session exists")
    }

    /// Removes the fixture's temporary durable stores.
    pub(super) fn cleanup(self) {
        drop(self.service);
        drop(self.session_store);
        drop(self.runner);
        std::fs::remove_dir_all(&self.temp_root).expect("remove continuation fixture root");
    }
}

/// Builds one catalog row for the scenario provider family.
///
/// # Parameters
/// - `provider_id`: Provider identity owning the row.
/// - `model_id`: Provider-native model id.
///
/// # Returns
/// An upstream catalog row.
pub(super) fn catalog_row(provider_id: &str, model_id: &str) -> rpi_ai::Model {
    rpi_ai::Model::new(
        model_id,
        model_id,
        rpi_ai::Api::OpenaiCompletions,
        provider_id,
        "https://continuation.invalid",
    )
}
