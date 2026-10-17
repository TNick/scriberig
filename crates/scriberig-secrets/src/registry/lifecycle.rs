//! Orders metadata and OS-store mutations to fail with authorization disabled.

use super::RegistryGuard;
use crate::{Binding, SecretError, SecretRef, SecretStore, SecretValue, normalize_endpoint};

impl RegistryGuard {
    /// Creates or explicitly replaces a managed OS credential.
    ///
    /// # Parameters
    /// - `store`: Native or deterministic test store.
    /// - `name`: Managed alias.
    /// - `value`: Protected input value.
    /// - `replace`: Whether an existing item may be replaced.
    ///
    /// # Returns
    /// Unit after the store and metadata agree.
    ///
    /// # Errors
    /// Returns safe conflicts, backend errors, or partial-operation errors.
    pub fn set(
        &mut self,
        store: &dyn SecretStore,
        name: &str,
        value: &SecretValue,
        replace: bool,
    ) -> Result<(), SecretError> {
        let reference = SecretRef::Keyring {
            name: name.to_owned(),
        };
        reference.validate()?;
        let exists = store.exists(name)?;
        if (exists || self.registry.names.contains(name)) && !replace {
            return Err(SecretError::Conflict);
        }
        let original = self.registry.policy.bindings.clone();
        self.registry
            .policy
            .bindings
            .retain(|binding| binding.credential != reference);
        self.save()?;
        store.set(name, value).map_err(|_| SecretError::Partial)?;
        self.registry.names.insert(name.to_owned());
        self.registry.policy.bindings = original;
        self.save().map_err(|_| SecretError::Partial)
    }

    /// Removes authorization before deleting a managed credential.
    ///
    /// # Parameters
    /// - `store`: Native or deterministic test store.
    /// - `name`: Managed alias.
    ///
    /// # Returns
    /// Unit after idempotent deletion.
    ///
    /// # Errors
    /// Returns a safe metadata or partial backend failure.
    pub fn delete(&mut self, store: &dyn SecretStore, name: &str) -> Result<(), SecretError> {
        let reference = SecretRef::Keyring {
            name: name.to_owned(),
        };
        reference.validate()?;
        self.registry
            .policy
            .bindings
            .retain(|binding| binding.credential != reference);
        self.registry.names.remove(name);
        self.save()?;
        match store.delete(name) {
            Ok(()) | Err(SecretError::Missing) => Ok(()),
            Err(_) => Err(SecretError::Partial),
        }
    }

    /// Adds an exact destination/project grant without reading a credential.
    ///
    /// # Parameters
    /// - `binding`: Explicitly requested local administrative grant.
    ///
    /// # Returns
    /// Unit after idempotently saving the grant.
    ///
    /// # Errors
    /// Returns safe validation or persistence failures.
    pub fn bind(&mut self, mut binding: Binding) -> Result<(), SecretError> {
        binding.credential.validate()?;
        if let SecretRef::Keyring { name } = &binding.credential {
            if !self.registry.names.contains(name) {
                return Err(SecretError::Missing);
            }
        }
        binding.base_url = normalize_endpoint(&binding.base_url)?;
        if let SecretRef::File { path } = &binding.credential {
            self.registry.protected_files.insert(path.clone());
        }
        binding.project_root = binding
            .project_root
            .canonicalize()
            .map_err(|_| SecretError::Invalid)?;
        if binding.purpose != "model_provider" || !binding.project_root.is_dir() {
            return Err(SecretError::Invalid);
        }
        if !self.registry.policy.bindings.contains(&binding) {
            self.registry.policy.bindings.push(binding);
        }
        self.save()
    }

    /// Removes an exact destination/project grant.
    ///
    /// # Parameters
    /// - `binding`: Exact binding to remove.
    ///
    /// # Returns
    /// Unit after idempotently saving the removal.
    ///
    /// # Errors
    /// Returns safe normalization or persistence failures.
    pub fn unbind(&mut self, mut binding: Binding) -> Result<(), SecretError> {
        binding.base_url = normalize_endpoint(&binding.base_url)?;
        binding.project_root = binding
            .project_root
            .canonicalize()
            .map_err(|_| SecretError::Invalid)?;
        self.registry
            .policy
            .bindings
            .retain(|existing| existing != &binding);
        self.save()
    }
}
