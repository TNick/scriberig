//! Verifies safe model request provenance survives durable session reopening.

use super::*;
use scriberig_session::{ModelContextRevisionRef, ModelRequestMessage, ModelRequestProjection};

/// Preserves safe request provenance when a durable session is reopened.
#[test]
#[allow(clippy::too_many_lines)] // Keeps the durable reopen scenario end to end.
fn model_request_provenance_survives_reopen() {
    let path = std::env::temp_dir().join(format!(
        "scriberig-request-provenance-{}.sqlite",
        EntityId::new()
    ));
    let session_id = EntityId::from_bytes([51; 16]);
    let branch_id = EntityId::from_bytes([52; 16]);
    let profile = ModelContextRevisionRef {
        identity: "agent-profile:reviewer".to_owned(),
        revision: "revision:profile-7".to_owned(),
    };
    let active_components = vec![
        ModelContextRevisionRef {
            identity: "context-component:core".to_owned(),
            revision: "revision:core-3".to_owned(),
        },
        ModelContextRevisionRef {
            identity: "context-component:handoff".to_owned(),
            revision: "revision:handoff-2".to_owned(),
        },
    ];
    let request = ModelRequestProjection {
        ordinal: 1,
        model: "model-a".to_owned(),
        provider_id: Some("provider-a".to_owned()),
        provider_profile: Some("provider-profile-a".to_owned()),
        thinking_level: Some("careful".to_owned()),
        profile: Some(profile.clone()),
        active_components: active_components.clone(),
        context_fingerprint: Some("context-fingerprint-51".to_owned()),
        request_fingerprint: Some("request-fingerprint-52".to_owned()),
        catalog_revision: None,
        upstream_runtime_revision: None,
        provider_response_id: None,
        model_execution_configuration: None,
        messages: vec![ModelRequestMessage {
            role: "user".to_owned(),
            content: "Review the bounded request.".to_owned(),
        }],
        full_messages: None,
        tools: Vec::new(),
        full_tools: None,
        content_complete: false,
        applied_settings: None,
        applied_settings_fingerprint: None,
        truncated: false,
    };
    let mut branch = Branch::new(branch_id, "root").expect("root branch");
    branch.model_invocations.push(ModelInvocation {
        id: EntityId::from_bytes([53; 16]),
        branch: branch_id,
        operation: None,
        ordinal: 1,
        agent_profile: Some(format!("{}@{}", profile.identity, profile.revision)),
        context_fingerprint: request.context_fingerprint.clone(),
        provider_profile: request.provider_profile.clone(),
        provider_id: request.provider_id.clone(),
        model: Some(request.model.clone()),
        model_profile: None,
        provider_model_id: Some("native-model-a".to_owned()),
        thinking_level: request.thinking_level.clone(),
        request_fingerprint: request.request_fingerprint.clone(),
        cache: CacheDisposition::NotUsed,
        output: None,
        request: Some(request.clone()),
    });
    let session = Session {
        id: session_id,
        branches: vec![branch],
        goals: Vec::new(),
        tasks: Vec::new(),
        plans: Vec::new(),
        execution: scriberig_session::SessionExecutionSelection::default(),
    };
    {
        let store = SqliteSessionStore::open(&path).expect("open store");
        store.save(&session).expect("save session");
    }
    let reopened = SqliteSessionStore::open(&path).expect("reopen store");
    let loaded = reopened
        .load(session_id)
        .expect("load session")
        .expect("stored session");
    let loaded_invocation = &loaded.branches[0].model_invocations[0];
    let loaded_request = loaded_invocation
        .request
        .as_ref()
        .expect("durable request projection");
    assert_eq!(loaded_request.profile, Some(profile));
    assert_eq!(loaded_request.active_components, active_components);
    assert_eq!(
        loaded_request.context_fingerprint.as_deref(),
        Some("context-fingerprint-51")
    );
    assert_eq!(
        loaded_request.request_fingerprint.as_deref(),
        Some("request-fingerprint-52")
    );
    assert_eq!(loaded_request, &request);
    assert_eq!(
        loaded_invocation.agent_profile.as_deref(),
        Some("agent-profile:reviewer@revision:profile-7")
    );
    assert_eq!(
        loaded_invocation.context_fingerprint.as_deref(),
        Some("context-fingerprint-51")
    );
    assert_eq!(
        loaded_invocation.request_fingerprint.as_deref(),
        Some("request-fingerprint-52")
    );
    drop(reopened);
    let _ = std::fs::remove_file(path);
}
