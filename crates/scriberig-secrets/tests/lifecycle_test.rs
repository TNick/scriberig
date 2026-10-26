//! Verifies lifecycle safety, policy denial, and per-request rotation.

mod common;

use common::MemoryStore;
use scriberig_secrets::registry::{AuthorizedCredential, RegistryLocations};
use scriberig_secrets::{
    Binding, CredentialProvider, ExecutionMode, SecretError, SecretPolicy, SecretRef,
    SecretUseContext, SecretValue,
};
use std::sync::{Arc, atomic::Ordering};

/// Rotates on the next request, denies wrong destinations before reading, and revokes.
#[test]
fn lifecycle_authorizes_before_read_and_reloads_rotation() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let project = temporary.path().canonicalize().expect("canonical project");
    let locations = RegistryLocations {
        user_dir: project.join("secrets"),
        system_policy: None,
    };
    let store = Arc::new(MemoryStore::default());
    let reference = SecretRef::Keyring {
        name: "work".to_owned(),
    };
    let context = SecretUseContext {
        project_root: project.clone(),
        purpose: "model_provider".to_owned(),
        mode: ExecutionMode::Interactive,
    };
    let binding = Binding {
        credential: reference.clone(),
        base_url: "https://example.com/v1".to_owned(),
        project_root: project,
        purpose: context.purpose.clone(),
        mode: context.mode,
    };
    {
        let mut guard = locations.lock().expect("protected registry");
        guard
            .set(
                &*store,
                "work",
                &SecretValue::new("CANARY_ONE".to_owned()).expect("secret"),
                false,
            )
            .expect("initial set");
        assert_eq!(
            guard.set(
                &*store,
                "work",
                &SecretValue::new("CANARY_TWO".to_owned()).expect("secret"),
                false
            ),
            Err(SecretError::Conflict)
        );
        guard.bind(binding).expect("binding");
    }
    let source = AuthorizedCredential {
        reference,
        context,
        locations: locations.clone(),
        invocation_binding: None,
        resolver: store.clone(),
    };
    assert_eq!(
        source
            .credential("https://attacker.example/v1")
            .expect_err("destination denied"),
        SecretError::Denied
    );
    assert_eq!(store.reads.load(Ordering::Relaxed), 0);
    assert_eq!(
        source
            .credential("https://example.com/v1")
            .expect("authorized")
            .expect("credential")
            .expose(),
        "CANARY_ONE"
    );
    locations
        .lock()
        .expect("registry")
        .set(
            &*store,
            "work",
            &SecretValue::new("CANARY_TWO".to_owned()).expect("secret"),
            true,
        )
        .expect("rotation");
    assert_eq!(
        source
            .credential("https://example.com/v1")
            .expect("authorized")
            .expect("credential")
            .expose(),
        "CANARY_TWO"
    );
    store.fail_delete.store(true, Ordering::Relaxed);
    assert_eq!(
        locations.lock().expect("registry").delete(&*store, "work"),
        Err(SecretError::Partial)
    );
    assert_eq!(
        source
            .credential("https://example.com/v1")
            .expect_err("revoked despite backend failure"),
        SecretError::Denied
    );
}

/// Advances safe registry metadata when credentials rotate without storing values.
#[test]
fn credential_rotation_advances_revision_without_serializing_secret_values() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let locations = RegistryLocations {
        user_dir: temporary.path().join("secrets"),
        system_policy: None,
    };
    let store = MemoryStore::default();
    locations
        .lock()
        .expect("registry")
        .set(
            &store,
            "work",
            &SecretValue::new("CANARY_ONE".to_owned()).expect("secret"),
            false,
        )
        .expect("initial credential");
    let before = locations
        .load()
        .expect("initial credential metadata")
        .credential_revision;

    locations
        .lock()
        .expect("registry")
        .set(
            &store,
            "work",
            &SecretValue::new("CANARY_TWO".to_owned()).expect("secret"),
            true,
        )
        .expect("credential rotation");

    let after = locations
        .load()
        .expect("rotated credential metadata")
        .credential_revision;
    let serialized = std::fs::read(locations.user_dir.join("registry.json"))
        .expect("serialized registry metadata");
    assert!(after > before, "credential rotation advances its revision");
    assert!(!String::from_utf8_lossy(&serialized).contains("CANARY"));
}

/// Administrator restrictions cannot be broadened by a process-local grant.
#[test]
fn invocation_binding_cannot_override_system_denial() {
    let temporary = tempfile::tempdir().expect("temporary directory");
    let root = temporary.path().canonicalize().expect("canonical root");
    let directory = root.join("secrets");
    scriberig_secrets::files::create_private_dir(&directory).expect("protected directory");
    let policy_path = directory.join("policy.json");
    let mut file =
        scriberig_secrets::files::create_private_file(&policy_path).expect("protected policy");
    serde_json::to_writer(&mut file, &SecretPolicy::default()).expect("empty deny policy");
    drop(file);
    let store = Arc::new(MemoryStore::default());
    let reference = SecretRef::Keyring {
        name: "work".to_owned(),
    };
    let context = SecretUseContext {
        project_root: root.clone(),
        purpose: "model_provider".to_owned(),
        mode: ExecutionMode::Interactive,
    };
    let binding = Binding {
        credential: reference.clone(),
        base_url: "https://example.com".to_owned(),
        project_root: root,
        purpose: context.purpose.clone(),
        mode: context.mode,
    };
    let source = AuthorizedCredential {
        reference,
        context,
        locations: RegistryLocations {
            user_dir: directory,
            system_policy: Some(policy_path),
        },
        invocation_binding: Some(binding),
        resolver: store.clone(),
    };
    assert_eq!(
        source
            .credential("https://example.com")
            .expect_err("system denial"),
        SecretError::Denied
    );
    assert_eq!(store.reads.load(Ordering::Relaxed), 0);
}
