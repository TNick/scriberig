//! Covers additional focused cases for the parent test module.

use super::*;

/// Verifies fingerprint excludes credentials and changes for model inputs.
#[test]
fn fingerprint_excludes_credentials_and_changes_for_model_inputs() {
    let input = RequestFingerprintInput::new("profile", "model", "compiled", "tools");
    let first = input.clone().fingerprint();
    let changed = input.with_model("other").fingerprint();

    assert_ne!(first, changed);
    assert!(!first.contains("one"));
}

/// Verifies exact workspace references enter bounded context and dependencies.
#[test]
fn project_scope_is_explicit_and_snapshot_bound() {
    let scope = ProjectSemanticScope {
        project: "prj-scriberig".to_owned(),
        snapshot: "snp-base".to_owned(),
        root: "ent-root".to_owned(),
        selection: Some("ent-code-domain".to_owned()),
    };
    let context = BranchExecutionContext::new("session", "branch", "discover", "map adapter")
        .with_project_scope(scope);
    let compiled = ContextCompiler::new(ContextPolicy::default()).compile(&context);

    assert!(compiled.canonical.contains("snapshot=snp-base"));
    assert!(
        compiled
            .dependencies
            .contains(&"project:prj-scriberig".to_owned())
    );
    assert!(
        compiled
            .dependencies
            .contains(&"snapshot:snp-base".to_owned())
    );
    assert!(compiled.dependencies.contains(&"root:ent-root".to_owned()));
    assert!(compiled.canonical.contains("selection=ent-code-domain"));
    assert!(!compiled.canonical.contains("fn parse()"));
    assert!(
        compiled
            .dependencies
            .contains(&"selection:ent-code-domain".to_owned())
    );
}

/// Verifies exact cache reuse avoids a second provider invocation.
#[test]
fn reuse_exact_preserves_original_provenance() {
    let cache = ModelCache::new();
    let calls = AtomicUsize::new(0);
    let response = || {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(ModelResponse {
            text: Some("done".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        })
    };

    let first = cache
        .execute(CachePolicy::RecordOnly, "fp", "invocation-1", response)
        .expect("first response");
    let second = cache
        .execute(CachePolicy::ReuseExact, "fp", "invocation-2", || {
            panic!("provider should not be called on cache hit")
        })
        .expect("cached response");

    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(first.original_invocation, "invocation-1");
    assert_eq!(second.invocation, "invocation-2");
    assert_eq!(second.original_invocation, "invocation-1");
    assert!(second.reused);
}

/// Verifies branch namespaces isolate otherwise identical model requests.
#[test]
fn cache_namespace_isolates_branches() {
    let cache = ModelCache::new();
    let calls = AtomicUsize::new(0);
    let response = || {
        calls.fetch_add(1, Ordering::Relaxed);
        Ok(ModelResponse {
            text: Some("done".to_owned()),
            tool_calls: Vec::new(),
            finish_reason: FinishReason::Stop,
            usage: None,
            metadata: ProviderMetadata::default(),
        })
    };

    cache
        .execute(CachePolicy::RecordOnly, "branch-a:request", "a-1", response)
        .expect("branch a response");
    let branch_b = cache
        .execute(CachePolicy::ReuseExact, "branch-b:request", "b-1", || {
            Ok(ModelResponse {
                text: Some("branch b".to_owned()),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
                metadata: ProviderMetadata::default(),
            })
        })
        .expect("branch b response");

    assert!(!branch_b.reused);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

/// Verifies a changed request identity misses an exact cache entry.
#[test]
fn changed_request_fingerprint_invalidates_exact_reuse() {
    let cache = ModelCache::new();
    let first = cache
        .execute(
            CachePolicy::RecordOnly,
            "request-v1",
            "invocation-1",
            || {
                Ok(ModelResponse {
                    text: Some("v1".to_owned()),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    usage: None,
                    metadata: ProviderMetadata::default(),
                })
            },
        )
        .expect("first response");
    let second = cache
        .execute(
            CachePolicy::ReuseExact,
            "request-v2",
            "invocation-2",
            || {
                Ok(ModelResponse {
                    text: Some("v2".to_owned()),
                    tool_calls: Vec::new(),
                    finish_reason: FinishReason::Stop,
                    usage: None,
                    metadata: ProviderMetadata::default(),
                })
            },
        )
        .expect("changed response");

    assert!(!second.reused);
    assert_ne!(first.response, second.response);
}

/// Verifies failed provider responses never become successful cache hits.
#[test]
fn failed_response_is_not_cached_as_success() {
    let cache = ModelCache::new();
    let first = cache.execute(CachePolicy::ReuseExact, "failed", "invocation-1", || {
        Err("provider failed".to_owned())
    });
    assert_eq!(first, Err("provider failed".to_owned()));

    let second = cache
        .execute(CachePolicy::ReuseExact, "failed", "invocation-2", || {
            Ok(ModelResponse {
                text: Some("recovered".to_owned()),
                tool_calls: Vec::new(),
                finish_reason: FinishReason::Stop,
                usage: None,
                metadata: ProviderMetadata::default(),
            })
        })
        .expect("recovery response");
    assert!(!second.reused);
}
