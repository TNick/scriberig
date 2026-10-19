//! Supplies a deterministic protected in-memory store for boundary tests.

use scriberig_secrets::{SecretError, SecretRef, SecretResolver, SecretStore, SecretValue};
use std::collections::BTreeMap;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// Stores fake secrets with explicit failure injection and read counts.
#[derive(Default)]
pub struct MemoryStore {
    /// Protected fake values.
    values: Mutex<BTreeMap<String, SecretValue>>,
    /// Whether deletion should fail for partial-operation tests.
    pub fail_delete: AtomicBool,
    /// Number of actual secret reads.
    pub reads: AtomicUsize,
}

impl SecretStore for MemoryStore {
    /// Reads a protected fake credential.
    ///
    /// - `name`: Fake alias.
    ///
    /// A protected copy for this request.
    ///
    /// Returns `Missing` for absent items.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn get(&self, name: &str) -> Result<SecretValue, SecretError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        let values = self.values.lock().expect("fake-store lock");
        SecretValue::new(
            values
                .get(name)
                .ok_or(SecretError::Missing)?
                .expose()
                .to_owned(),
        )
    }

    /// Replaces a fake credential.
    ///
    /// - `name`: Fake alias.
    /// - `value`: Protected input.
    ///
    /// Unit after the test-store write.
    ///
    /// Returns invalid secret errors.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn set(&self, name: &str, value: &SecretValue) -> Result<(), SecretError> {
        self.values.lock().expect("fake-store lock").insert(
            name.to_owned(),
            SecretValue::new(value.expose().to_owned())?,
        );
        Ok(())
    }

    /// Deletes a fake credential or injects a backend failure.
    ///
    /// - `name`: Fake alias.
    ///
    /// Unit after deletion.
    ///
    /// Returns `Unavailable` when requested by the test.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn delete(&self, name: &str) -> Result<(), SecretError> {
        if self.fail_delete.load(Ordering::Relaxed) {
            return Err(SecretError::Unavailable);
        }
        self.values.lock().expect("fake-store lock").remove(name);
        Ok(())
    }

    /// Checks fake metadata without reading the value.
    ///
    /// - `name`: Fake alias.
    ///
    /// Whether the alias exists.
    ///
    /// This fake implementation does not fail.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn exists(&self, name: &str) -> Result<bool, SecretError> {
        Ok(self
            .values
            .lock()
            .expect("fake-store lock")
            .contains_key(name))
    }
}

impl SecretResolver for MemoryStore {
    /// Resolves only fake keyring references.
    ///
    /// - `reference`: Authorized fake reference.
    ///
    /// A protected fake credential.
    ///
    /// Returns `Unsupported` for other source types.
    /// # Returns
    /// The operation result.
    /// # Errors
    /// Returns an error when the operation fails.
    fn resolve(&self, reference: &SecretRef) -> Result<SecretValue, SecretError> {
        match reference {
            SecretRef::Keyring { name } => self.get(name),
            _ => Err(SecretError::Unsupported),
        }
    }
}
