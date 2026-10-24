//! Composes the durable service, native root runner, and scripted provider used
//! by the upstream execution activity and cancellation tests.
//!
//! Only the upstream provider is replaced; branch creation, context
//! compilation, semantic tool authorization, child delegation, and durable
//! settlement all run through the production composition.

use std::path::PathBuf;
use std::sync::Arc;

use scriberig_agent_runtime::continuation::EphemeralContinuationOwner;
use scriberig_app::{
    AgentCommandRequest, ApplicationBackendComposition, ApplicationContext, ApplicationError,
    ApplicationService, CapabilityContext, CapabilityId, LocalAgentHandle, SessionBackend,
    SessionExecutionSelection, StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_session::{Session, SessionStore};
use scriberig_store_sqlite::SqliteStore;
use scriberig_tui::{AgentCommandRunner, AgentHandle};

use super::provider::{ScriptedProvider, Step};
use crate::native_scenario::NativeScenario;
use crate::native_tui_agent::NativeTuiAgentRunner;

/// Grants sufficient to submit and continue a root agent branch.
const BASE_GRANTS: [&str; 7] = [
    "agent_start",
    "agent_continue",
    "agent_plan",
    "agent.delegate",
    "inspect.entity",
    "runtime_capabilities",
    "session_branch_tree",
];

/// Backend composition used by the fixture service.
type FixtureBackend = ApplicationBackendComposition<
    StoreInspectionBackend<Arc<SqliteStore>>,
    StoreOperationProvider<Arc<SqliteStore>>,
>;

/// One composed fixture: durable service, native runner, and scripted provider.
pub(crate) struct UpstreamFixture {
    /// Temporary directory holding the durable stores.
    pub(crate) temp_root: PathBuf,
    /// Shared application service under test.
    pub(crate) service: ApplicationService<FixtureBackend>,
    /// Durable session store for assertions.
    pub(crate) session_store: Arc<scriberig_session_sqlite::SqliteSessionStore>,
    /// Native runner executing the root branch.
    pub(crate) runner: NativeTuiAgentRunner,
    /// Canonical session identity used by every turn.
    pub(crate) session: EntityId,
    /// Scripted provider registered for the `"local"` connection.
    pub(crate) provider: Arc<ScriptedProvider>,
}

impl UpstreamFixture {
    /// Builds one fixture with isolated durable stores and one provider.
    ///
    /// # Parameters
    /// - `steps`: Provider steps consumed in request order.
    ///
    /// # Returns
    /// A composed fixture ready to submit a root prompt.
    pub(crate) fn new(steps: Vec<Step>) -> Self {
        let temp_root =
            std::env::temp_dir().join(format!("scriberig-upstream-{0}", EntityId::new()));
        std::fs::create_dir_all(&temp_root).expect("fixture root");
        let project_store =
            Arc::new(SqliteStore::open(temp_root.join("project.sqlite")).expect("project store"));
        let backend = ApplicationBackendComposition::new(
            StoreInspectionBackend::new(Arc::clone(&project_store)),
            StoreOperationProvider::new(Arc::clone(&project_store)),
        );
        let session_store = Arc::new(
            scriberig_session_sqlite::SqliteSessionStore::open(temp_root.join("session.sqlite"))
                .expect("session store"),
        );
        let session_repository: Arc<dyn SessionStore> =
            Arc::clone(&session_store) as Arc<dyn SessionStore>;
        let session = EntityId::from_bytes([93; 16]);
        let service = ApplicationService::new(backend)
            .with_session_store(session_repository)
            .with_upstream_models(Self::catalog());
        let model = Self::catalog()
            .into_iter()
            .next()
            .expect("catalog row exists");
        let provider = Arc::new(ScriptedProvider::new("local", model, steps));
        let runner = NativeTuiAgentRunner {
            providers: std::collections::BTreeMap::from([(
                "local".to_owned(),
                NativeScenario::Provider(Arc::clone(&provider) as Arc<dyn rpi_ai::Provider>),
            )]),
            model: "fake-model".to_owned(),
            upstream_credentials: None,
            upstream_enabled: false,
            model_catalog: super::super::model_catalog_from_models(Self::catalog()),
            model_config: std::sync::Mutex::new((rpi_model_runtime::ModelsConfig::default(), None)),
            project_store,
            active_coordinators: std::sync::Mutex::default(),
            continuations: EphemeralContinuationOwner::default(),
        };
        SessionBackend::save_session(
            &service,
            &Self::context(&[]),
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
        Self {
            temp_root,
            service,
            session_store,
            runner,
            session,
            provider,
        }
    }

    /// Returns the fixture's `ScribeRig` model catalog rows.
    ///
    /// # Returns
    /// The upstream catalog rows the runner may resolve.
    pub(crate) fn catalog() -> Vec<rpi_ai::Model> {
        super::super::plan::upstream_model_catalog("local", &["fake-model"])
    }

    /// Builds one authorized context, adding any extra grants.
    ///
    /// # Parameters
    /// - `extra`: Additional capability ids.
    ///
    /// # Returns
    /// An application context carrying the base and extra grants.
    ///
    /// # Panics
    /// Panics when a capability id is invalid.
    pub(crate) fn context(extra: &[&str]) -> ApplicationContext {
        let mut grants = BASE_GRANTS
            .iter()
            .map(|grant| (*grant).to_owned())
            .collect::<Vec<_>>();
        grants.extend(extra.iter().map(|grant| (*grant).to_owned()));
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
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// The accepted handle, or the application failure.
    ///
    /// # Errors
    /// Returns an authorization, provider, or application failure from the turn.
    pub(crate) fn submit(
        &self,
        context: &ApplicationContext,
        prompt: &str,
    ) -> Result<AgentHandle, ApplicationError> {
        self.runner
            .submit(&self.service, context, self.session, None, prompt)
    }

    /// Begins one durable root operation without executing its turn.
    ///
    /// # Parameters
    /// - `context`: Trusted caller authorization for the turn.
    /// - `prompt`: User-authored instruction.
    ///
    /// # Returns
    /// The accepted application handle.
    ///
    /// # Errors
    /// Returns an authorization or persistence failure.
    pub(crate) fn begin(
        &self,
        context: &ApplicationContext,
        prompt: &str,
    ) -> Result<LocalAgentHandle, ApplicationError> {
        let handle = self.service.begin_agent_execution(
            context,
            &AgentCommandRequest {
                session: self.session,
                branch: None,
                anchor: None,
                prompt: prompt.to_owned(),
                selection: None,
            },
        )?;
        Ok(handle)
    }

    /// Executes one accepted operation through the production root runner path.
    ///
    /// # Parameters
    /// - `context`: Trusted caller authorization for the turn.
    /// - `handle`: Accepted operation and root branch.
    ///
    /// # Returns
    /// The settled handle, or the application failure.
    ///
    /// # Errors
    /// Returns the provider, cancellation, or application failure.
    pub(crate) fn execute(
        &self,
        context: &ApplicationContext,
        handle: &LocalAgentHandle,
    ) -> Result<AgentHandle, ApplicationError> {
        self.runner.execute_accepted(&self.service, context, handle)
    }

    /// Signals cancellation for one accepted operation.
    ///
    /// - `handle`: Accepted operation whose execution must stop.
    pub(crate) fn cancel(&self, handle: &LocalAgentHandle) {
        self.service
            .operations()
            .cancellation_flag(&handle.operation)
            .expect("operation cancellation flag")
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Returns the number of provider requests already dispatched.
    ///
    /// # Returns
    /// The fixture provider request count.
    pub(crate) fn request_count(&self) -> usize {
        self.provider.request_count()
    }

    /// Reloads the durable session.
    ///
    /// # Returns
    /// The authoritative session state.
    pub(crate) fn durable(&self) -> Session {
        self.session_store
            .load(self.session)
            .expect("load session")
            .expect("session exists")
    }

    /// Removes the fixture's temporary durable stores.
    pub(crate) fn cleanup(self) {
        drop(self.service);
        drop(self.session_store);
        drop(self.runner);
        std::fs::remove_dir_all(&self.temp_root).expect("remove fixture root");
    }
}
