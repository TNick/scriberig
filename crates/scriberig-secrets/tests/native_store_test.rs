//! Explicitly selected native-store smoke test using an isolated fake credential.

#![cfg(feature = "native-tests")]

use scriberig_secrets::{OsSecretStore, SecretStore, SecretValue};

/// Creates, reads, rotates, and removes a unique test-only OS credential.
#[test]
fn native_store_round_trip() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let name = format!("test/{}-{nonce}", std::process::id());
    let store = OsSecretStore;
    let result = (|| {
        assert!(!store.exists(&name)?);
        store.set(
            &name,
            &SecretValue::new("SCRIBERIG_FAKE_NATIVE_ONE".to_owned())?,
        )?;
        assert!(store.exists(&name)?);
        assert_eq!(store.get(&name)?.expose(), "SCRIBERIG_FAKE_NATIVE_ONE");
        store.set(
            &name,
            &SecretValue::new("SCRIBERIG_FAKE_NATIVE_TWO".to_owned())?,
        )?;
        assert_eq!(store.get(&name)?.expose(), "SCRIBERIG_FAKE_NATIVE_TWO");
        Ok::<(), scriberig_secrets::SecretError>(())
    })();
    let cleanup = store.delete(&name);
    result.expect("native-store round trip");
    cleanup.expect("remove isolated test item");
    assert!(!store.exists(&name).expect("metadata after deletion"));
}
