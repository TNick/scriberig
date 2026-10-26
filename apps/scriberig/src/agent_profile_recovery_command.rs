//! Implements the explicit local workflow for recovering a legacy profile selection.

use std::collections::BTreeSet;
use std::str::FromStr;
use std::sync::Arc;

use crate::{agent_profile_composition, native_tui_agent};
use scriberig_app::{
    AgentConsoleBackend, AgentProfileDescriptor, ApplicationBackendComposition, ApplicationContext,
    ApplicationService, Capability, CapabilityAction, CapabilityConstraints, CapabilityId,
    CapabilityScope, LegacyExecutionMappingRequest, LegacyProfileMappingRequest,
    StoreInspectionBackend, StoreOperationProvider,
};
use scriberig_core::EntityId;
use scriberig_store_sqlite::SqliteStore;

#[expect(
    clippy::too_many_lines,
    reason = "recovery command keeps profile promotion and legacy mapping order explicit"
)]
/// Recovers one legacy session profile using an explicit operator template and current selection.
///
/// # Parameters
/// - `args`: Profile recovery command options.
///
/// # Returns
/// Success after inspection, or profile promotion and exact legacy-record resolution.
///
/// # Errors
/// Returns an error for invalid input, authorization, catalog, stale revision, confirmation, or storage failures.
pub(crate) fn run(
    args: &scriberig_cli::RecoverProfileArgs,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let session = EntityId::from_str(&args.session)?;
    let store = Arc::new(SqliteStore::open(&args.db)?);
    let backend = ApplicationBackendComposition::new(
        StoreInspectionBackend::new(Arc::clone(&store)),
        StoreOperationProvider::new(Arc::clone(&store)),
    );
    let sessions = Arc::new(scriberig_session_sqlite::SqliteSessionStore::open(
        &args.db,
    )?);
    let shared_sessions: Arc<dyn scriberig_session::SessionStore> = sessions.clone();
    let service = ApplicationService::new(backend)
        .with_session_store(shared_sessions)
        .with_agent_profile_provider(agent_profile_composition::provider(Arc::clone(&store)));
    let actions = [
        CapabilityAction::Inspect,
        CapabilityAction::Custom("session_model_selection".to_owned()),
        CapabilityAction::Custom("agent_profile.propose".to_owned()),
        CapabilityAction::Custom("agent_profile.promote".to_owned()),
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    let mut context = ApplicationContext {
        capabilities: scriberig_app::CapabilityContext::with_grants([Capability {
            scope: CapabilityScope::Project,
            actions,
            constraints: CapabilityConstraints::unrestricted(),
        }]),
        principal: Some(scriberig_app::Principal::new("local-profile-recovery")),
        ..ApplicationContext::empty()
    };
    context
        .capabilities
        .granted
        .push(CapabilityId::new("session_model_selection").expect("fixed capability id"));
    if args.inspect {
        return inspect(&service, &context, session, sessions.as_ref());
    }
    let revision = args
        .revision
        .ok_or("--revision is required unless --inspect is used")?;
    let profile_path = args
        .profile
        .as_ref()
        .ok_or("--profile is required unless --inspect is used")?;
    let provider_id = args
        .provider
        .as_deref()
        .ok_or("--provider is required unless --inspect is used")?;
    let model_id = args
        .model
        .as_deref()
        .ok_or("--model is required unless --inspect is used")?;
    let bytes = std::fs::read(profile_path)?;
    let profile: AgentProfileDescriptor = serde_json::from_slice(&bytes)?;
    let runner = native_tui_agent::NativeTuiAgentRunner::from_environment(Arc::clone(&store))?;
    let service = service.with_provider_extension_runtime(runner.provider_extension_runtime());
    let base = service.current_agent_profile_snapshot()?;
    let candidate = service.propose_legacy_profile_mapping(
        &context,
        LegacyProfileMappingRequest {
            session,
            expected_revision: revision,
            legacy_path: "/execution".to_owned(),
            base_profile_snapshot: base,
            profile,
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            thinking_level: args.thinking_level.clone(),
        },
    )?;
    println!(
        "Profile candidate: {} revision {} at snapshot {} (base {})",
        candidate.profile_identity(),
        candidate.revision(),
        candidate.snapshot(),
        candidate.base()
    );
    if !args.yes && !confirm()? {
        println!("Recovery cancelled; candidate was not promoted.");
        return Ok(());
    }
    match service.promote_agent_profile_candidate(&context, &candidate)? {
        scriberig_app::AgentProfileStoreResult::Promoted { .. } => {}
        scriberig_app::AgentProfileStoreResult::Stale { actual } => {
            return Err(format!("profile root changed before promotion: {actual:?}").into());
        }
    }
    service.map_legacy_execution_selection(
        &context,
        &LegacyExecutionMappingRequest {
            session,
            expected_revision: revision,
            legacy_path: "/execution".to_owned(),
            provider_id: provider_id.to_owned(),
            model_id: model_id.to_owned(),
            thinking_level: args.thinking_level.clone(),
        },
    )?;
    println!(
        "Recovered profile {} and resolved session {} legacy execution record.",
        candidate.profile_identity(),
        session
    );
    Ok(())
}

/// Prints the exact legacy profile identity and observed session-selection revision.
///
/// # Parameters
/// - `service`: Shared application facade that reads the authorized recovery record.
/// - `context`: Local operator capabilities for reading the session record.
/// - `session`: Exact session containing the legacy selection.
/// - `sessions`: SQLite session store used to read the observed selection revision.
///
/// # Returns
/// Success after the recovery details are written to stdout.
///
/// # Errors
/// Returns an authorization, missing-session, storage, or output error.
fn inspect<B: scriberig_app::ApplicationBackend>(
    service: &ApplicationService<B>,
    context: &ApplicationContext,
    session: EntityId,
    sessions: &scriberig_session_sqlite::SqliteSessionStore,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use scriberig_session::SessionStore;

    let records = service.legacy_execution_records(context, session)?;
    let record = records
        .iter()
        .find(|record| record.path == "/execution")
        .ok_or("session has no preserved /execution recovery record")?;
    let identity = record
        .value
        .get("model_profile")
        .and_then(serde_json::Value::as_str)
        .ok_or("preserved /execution record has no legacy profile identity")?;
    let loaded = sessions
        .load(session)
        .map_err(std::io::Error::other)?
        .ok_or("session does not exist")?;
    println!("Session: {session}");
    println!(
        "Execution selection revision: {}",
        loaded.execution.revision
    );
    println!("Legacy profile identity: {identity}");
    println!(
        "Preserved selection: {}",
        serde_json::to_string(&record.value)?
    );
    println!(
        "Supply a complete semantic profile template and an explicit current upstream selection to apply recovery."
    );
    Ok(())
}

/// Requests explicit confirmation before promoting a recovery candidate.
///
/// # Returns
/// Whether the operator entered `yes`.
///
/// # Errors
/// Returns an input or output error when the prompt cannot be completed.
fn confirm() -> Result<bool, std::io::Error> {
    use std::io::{self, Write};
    print!("Promote this profile and resolve the legacy session record? [y/N] ");
    io::stdout().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}
