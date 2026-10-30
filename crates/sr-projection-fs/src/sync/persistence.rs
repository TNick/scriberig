//! Persists exact synchronization baselines and collects unreachable output blobs.

use serde::{Deserialize, Serialize};

use super::{MemorySyncStore, SyncState};
use crate::{FileSystem, IoError, LocalFileSystem};

/// Stores typed synchronization metadata alongside one content-addressed output reference.
#[derive(Clone, Debug, Deserialize, Serialize)]
struct PersistedSyncState {
    /// Synchronization metadata with the large output bytes removed.
    state: SyncState,
    /// BLAKE3 content address of the exact rendered baseline bytes.
    baseline_blob: String,
}

impl MemorySyncStore {
    /// Opens a synchronization store that atomically persists its current records.
    ///
    /// - `path`: File used to retain exact synchronization baselines across restart.
    ///
    /// # Returns
    /// A store loaded from the file, or an empty store when the file is absent.
    ///
    /// # Errors
    /// Returns an IO error for unreadable data or invalid serialized state.
    pub fn open(path: impl Into<std::path::PathBuf>) -> Result<Self, IoError> {
        let path = path.into();
        let persisted: Vec<PersistedSyncState> = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|error| {
                IoError::Read(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    error.to_string(),
                ))
            })?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(IoError::Read(error)),
        };
        let blob_directory = Self::blob_directory(&path);
        let mut states = Vec::with_capacity(persisted.len());
        for mut item in persisted {
            if !Self::valid_blob_name(&item.baseline_blob) {
                return Err(IoError::BaselineMismatch);
            }
            let blob_path = blob_directory.join(&item.baseline_blob);
            let output = std::fs::read(blob_path).map_err(IoError::Read)?;
            if Self::blob_name(&output) != item.baseline_blob {
                return Err(IoError::BaselineMismatch);
            }
            item.state.baseline.rendered.bytes = output;
            states.push(item.state);
        }
        for state in &mut states {
            let evidence = state
                .baseline
                .evidence()
                .map_err(|_| IoError::BaselineMismatch)?
                .cloned();
            state.baseline = sr_projection::ReconciliationBaseline::new_with_evidence(
                state.baseline.rendered.clone(),
                state.baseline.adapter.clone(),
                state.baseline.fidelity.clone(),
                evidence,
            )
            .map_err(|_| IoError::BaselineMismatch)?;
        }
        Ok(Self {
            states,
            persistence_path: Some(path),
        })
    }

    /// Atomically persists all current synchronization records when configured.
    ///
    /// # Returns
    /// Success when no persistence path is configured or the state was written.
    ///
    /// # Errors
    /// Returns an IO error when the target directory or atomic replacement fails.
    pub fn persist(&self) -> Result<(), IoError> {
        let Some(path) = &self.persistence_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            LocalFileSystem.create_dir_all(parent)?;
        }
        let blob_directory = Self::blob_directory(path);
        LocalFileSystem.create_dir_all(&blob_directory)?;
        let mut referenced = std::collections::BTreeSet::new();
        let mut persisted = Vec::with_capacity(self.states.len());
        for state in &self.states {
            let evidence = state
                .baseline
                .evidence()
                .map_err(|_| IoError::BaselineMismatch)?
                .cloned();
            sr_projection::ReconciliationBaseline::new_with_evidence(
                state.baseline.rendered.clone(),
                state.baseline.adapter.clone(),
                state.baseline.fidelity.clone(),
                evidence,
            )
            .map_err(|_| IoError::BaselineMismatch)?;
            let blob = Self::blob_name(&state.baseline.rendered.bytes);
            referenced.insert(blob.clone());
            let blob_path = blob_directory.join(&blob);
            match std::fs::read(&blob_path) {
                Ok(existing) if existing == state.baseline.rendered.bytes => {}
                Ok(_) => return Err(IoError::BaselineMismatch),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    LocalFileSystem.write_atomic(&blob_path, &state.baseline.rendered.bytes)?;
                }
                Err(error) => return Err(IoError::Read(error)),
            }
            let mut metadata = state.clone();
            metadata.baseline.rendered.bytes.clear();
            persisted.push(PersistedSyncState {
                state: metadata,
                baseline_blob: blob,
            });
        }
        let manifest = serde_json::to_vec(&persisted)
            .map_err(|error| IoError::Write(std::io::Error::other(error.to_string())))?;
        LocalFileSystem.write_atomic(path, &manifest)?;
        Self::collect_unreferenced_blobs(&blob_directory, &referenced)?;
        Ok(())
    }

    /// Locates the content-addressed output directory for one manifest.
    ///
    /// - `manifest`: Manifest path supplied by the application host.
    ///
    /// # Returns
    /// A sibling directory holding exact baseline output blobs.
    fn blob_directory(manifest: &std::path::Path) -> std::path::PathBuf {
        let mut name = manifest.file_name().map_or_else(
            || "sync-state".to_owned(),
            |value| value.to_string_lossy().into_owned(),
        );
        name.push_str(".blobs");
        manifest
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .join(name)
    }

    /// Computes the lowercase BLAKE3 content-addressed filename.
    ///
    /// - `bytes`: Exact output representation.
    ///
    /// # Returns
    /// The stable hexadecimal digest filename.
    fn blob_name(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    /// Checks the exact filename grammar accepted for stored baseline blobs.
    ///
    /// - `name`: Candidate content-addressed filename.
    ///
    /// # Returns
    /// Whether the name is one lowercase 256-bit hexadecimal digest.
    fn valid_blob_name(name: &str) -> bool {
        name.len() == 64
            && name
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    }

    /// Removes content blobs unreachable from the newly committed manifest.
    ///
    /// - `directory`: Content-addressed blob directory.
    /// - `referenced`: Blob names included in the committed manifest.
    ///
    /// # Returns
    /// Unit after every unreferenced file has been removed.
    ///
    /// # Errors
    /// Returns an IO error when the directory cannot be read or a blob cannot be removed.
    fn collect_unreferenced_blobs(
        directory: &std::path::Path,
        referenced: &std::collections::BTreeSet<String>,
    ) -> Result<(), IoError> {
        for entry in std::fs::read_dir(directory).map_err(IoError::Read)? {
            let entry = entry.map_err(IoError::Read)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if !referenced.contains(&name) && entry.file_type().map_err(IoError::Read)?.is_file() {
                std::fs::remove_file(entry.path()).map_err(IoError::Remove)?;
            }
        }
        Ok(())
    }
}
