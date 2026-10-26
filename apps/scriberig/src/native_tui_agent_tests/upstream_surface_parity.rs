//! Proves local and remote TUI adapters persist equivalent upstream agent turns.

use std::sync::Arc;

use scriberig_app::ApplicationContext;
use scriberig_client::Client;
use scriberig_server::{AgentExecutionHandler, InProcessTransport, LocalApplicationBackend};
use scriberig_session::SessionStore;
use scriberig_tui::{AgentCommandRunner, RemoteAdapter, TuiService};

use super::upstream_fixture::fixture::UpstreamFixture;
use super::upstream_fixture::provider::{answer, step};

/// Verifies local and remote adapters persist the same deterministic agent turn.
#[test]
fn local_and_remote_agent_turns_persist_equivalent_session_activity() {
    let local = fixture();
    let remote = fixture();
    let context = UpstreamFixture::context(&[]);
    let local_adapter =
        scriberig_tui::service::ApplicationOperationAdapter::new(local.service.clone())
            .with_agent_runner(local.runner);
    let prompt = "Explain the fixture result.";
    let local_result = local_adapter
        .start_new_session_handle(&context, prompt, None)
        .expect("local adapter executes the turn");

    let remote_runner = remote.runner;
    let handler: Arc<AgentExecutionHandler<scriberig_server::LocalApplicationService>> = Arc::new(
        move |service, request_context, request, accepted_response| {
            let mut runtime_context = UpstreamFixture::context(&[]);
            runtime_context.principal = request_context.principal.clone();
            let accepted = accepted_response.map_or_else(
                || service.begin_agent_execution(&runtime_context, request),
                |response| crate::main_agent_execution::from_protocol_acceptance(request, response),
            )?;
            let _execution = AgentCommandRunner::<LocalApplicationBackend>::execute_accepted(
                &remote_runner,
                service,
                &runtime_context,
                &accepted,
            )?;
            scriberig_server::ApplicationService::operation_status(
                service,
                &accepted.operation.to_string(),
            )
        },
    );
    let transport =
        InProcessTransport::new(remote.service.clone()).with_agent_execution_handler(handler);
    let remote_adapter = RemoteAdapter::new(Client::new(transport));
    let remote_result = remote_adapter
        .start_new_session_handle(&ApplicationContext::empty(), prompt, None)
        .expect("remote adapter executes the turn");

    assert_eq!(local_result.session, remote_result.session);
    assert_eq!(local_result.branch, remote_result.branch);
    assert_eq!(local_result.operation, remote_result.operation);
    let anchor = scriberig_session::ContinuationAnchor::BranchHead {
        branch: local_result.branch.expect("root branch exists"),
    };
    let local_continuation = local_adapter
        .continue_agent_at_handle(
            &context,
            local_result.session,
            anchor,
            "Continue the result.",
        )
        .expect("local adapter executes exact-anchor continuation");
    let remote_continuation = remote_adapter
        .continue_agent_at_handle(
            &ApplicationContext::empty(),
            remote_result.session,
            anchor,
            "Continue the result.",
        )
        .expect("remote adapter executes exact-anchor continuation");
    assert_eq!(local_continuation.session, remote_continuation.session);
    assert_eq!(local_continuation.branch, remote_continuation.branch);
    assert_eq!(local_continuation.operation, remote_continuation.operation);
    let local_session = local
        .session_store
        .load(local_result.session)
        .expect("load local session")
        .expect("local session exists");
    let remote_session = remote
        .session_store
        .load(remote_result.session)
        .expect("load remote session")
        .expect("remote session exists");
    assert_eq!(local_session, remote_session);
    assert_eq!(local.provider.request_count(), 2);
    assert_eq!(remote.provider.request_count(), 2);
    assert_eq!(local_session.branches.len(), 2);
    assert_eq!(
        local_session
            .branches
            .iter()
            .map(|branch| branch.model_invocations.len())
            .sum::<usize>(),
        2
    );
    drop(remote_adapter);
    drop(local_adapter);
    drop(local.service);
    drop(local.session_store);
    drop(remote.service);
    drop(remote.session_store);
    std::fs::remove_dir_all(&local.temp_root).expect("remove local fixture root");
    std::fs::remove_dir_all(&remote.temp_root).expect("remove remote fixture root");
}

/// Verifies one-shot CLI and TUI runs preserve the same durable branch contract.
#[test]
fn one_shot_cli_and_tui_write_equivalent_agent_branch_outcomes() {
    let prompt = "Explain the fixture result.";
    let cli = fixture_with_answer(prompt);
    let tui = fixture_with_answer("No provider endpoint configured.");
    let cli_session = execute_cli_turn(&cli, prompt);
    let context = UpstreamFixture::context(&[]);
    let tui_adapter = scriberig_tui::service::ApplicationOperationAdapter::new(tui.service.clone())
        .with_agent_runner(tui.runner);
    let tui_handle = tui_adapter
        .start_new_session_handle(&context, prompt, None)
        .expect("TUI adapter executes the turn");
    let tui_session = tui
        .session_store
        .load(tui_handle.session)
        .expect("load TUI session")
        .expect("TUI session exists");

    assert_eq!(cli_session.id, tui_session.id);
    assert_eq!(cli_session.branches.len(), 1);
    assert_eq!(tui_session.branches.len(), 1);
    let cli_branch = &cli_session.branches[0];
    let tui_branch = &tui_session.branches[0];
    assert_eq!(cli_branch.id, tui_branch.id);
    assert_eq!(cli_branch.parent, tui_branch.parent);
    assert_eq!(cli_branch.status, tui_branch.status);
    assert_eq!(cli_branch.turns, tui_branch.turns);
    assert_eq!(cli_branch.result, tui_branch.result);
    assert_eq!(cli_branch.activity, tui_branch.activity);
    assert_eq!(cli_branch.model_invocations.len(), 1);
    assert_eq!(tui_branch.model_invocations.len(), 1);
    assert_eq!(
        cli_branch.model_invocations[0].operation,
        tui_branch.model_invocations[0].operation
    );
    assert_eq!(
        cli_branch.model_invocations[0]
            .request
            .as_ref()
            .map(|request| request.ordinal),
        tui_branch.model_invocations[0]
            .request
            .as_ref()
            .map(|request| request.ordinal)
    );

    drop(tui_adapter);
    drop(tui.service);
    drop(tui.session_store);
    drop(cli.service);
    drop(cli.session_store);
    drop(cli.runner);
    std::fs::remove_dir_all(&tui.temp_root).expect("remove TUI fixture root");
    std::fs::remove_dir_all(&cli.temp_root).expect("remove CLI fixture root");
}

/// Builds one fixture whose provider produces the same durable assistant result.
///
/// # Returns
/// An isolated store and runner ready for a single root turn.
fn fixture() -> UpstreamFixture {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("fixture model exists");
    UpstreamFixture::new(vec![
        step(answer(&model, "surface-parity", "Fixture answer.")),
        step(answer(
            &model,
            "surface-parity-continuation",
            "Continued fixture answer.",
        )),
    ])
}

/// Builds one fixture whose model returns the same text as the CLI echo provider.
///
/// # Parameters
/// - `text`: User prompt and deterministic assistant output.
///
/// # Returns
/// An isolated store and runner ready for a single root turn.
fn fixture_with_answer(text: &str) -> UpstreamFixture {
    let model = UpstreamFixture::catalog()
        .into_iter()
        .next()
        .expect("fixture model exists");
    UpstreamFixture::new(vec![step(answer(&model, "surface-parity", text))])
}

/// Executes one one-shot CLI runtime turn and reads its durable session.
///
/// - `fixture`: Isolated store and provider-independent session service.
/// - `prompt`: User instruction sent through the CLI branch operation.
///
/// The completed durable session after CLI execution.
/// # Returns
/// The operation result.
/// # Panics
/// Panics when the CLI runtime or durable settlement fails.
fn execute_cli_turn(fixture: &UpstreamFixture, prompt: &str) -> scriberig_session::Session {
    let context = UpstreamFixture::context(&[]);
    let accepted = fixture
        .service
        .begin_new_session_agent(
            &context,
            scriberig_app::NewSessionAgentRequest {
                prompt: prompt.to_owned(),
                selection: None,
            },
        )
        .expect("accept CLI branch operation");
    let store = Arc::new(
        scriberig_store_sqlite::SqliteStore::open(fixture.temp_root.join("project.sqlite"))
            .expect("open CLI project store"),
    );
    let args = scriberig_cli::RunArgs {
        prompt: prompt.to_owned(),
        db: fixture.temp_root.join("project.sqlite"),
        format: scriberig_cli::RunFormat::Json,
        no_stream: true,
        provider: "missing-provider".to_owned(),
        model: "fake-model".to_owned(),
        base_url: None,
        api_key_env: None,
        credential_ref: None,
        credential: None,
        explicit_cli: std::collections::BTreeSet::default(),
    };
    let cancellation = std::sync::atomic::AtomicBool::new(false);
    let result = crate::run_composed_model(
        &args,
        &fixture.service,
        &store,
        &context,
        &scriberig_agent_runtime::ChildExecutionScope {
            session: accepted.session,
            parent_branch: accepted.root_branch,
            depth: 0,
            active_branches: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            semantic_scope: None,
            provider_resolver: None,
        },
        &cancellation,
    )
    .unwrap_or_else(|failure| panic!("CLI runtime failed: {:?}", failure.error));
    fixture
        .service
        .complete_local_agent_with_evidence(
            &accepted,
            result.output,
            Vec::new(),
            result.tool_evidence,
            result.model_requests,
        )
        .expect("settle CLI branch result");
    fixture
        .session_store
        .load(accepted.session)
        .expect("load CLI session")
        .expect("CLI session exists")
}
