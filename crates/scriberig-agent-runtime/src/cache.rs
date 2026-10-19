//! Provides policy-controlled in-memory normalized model-result caching.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::provider::ModelResponse;

/// Selects how one model invocation interacts with the cache.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CachePolicy {
    /// Never read or write cache entries.
    Disabled,
    /// Invoke the provider and record a complete response.
    RecordOnly,
    /// Reuse an exact successful response when available.
    ReuseExact,
}

/// Normalized cache result with reuse provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CachedResponse {
    /// Complete normalized response.
    pub response: ModelResponse,
    /// Current invocation identity observing this response.
    pub invocation: String,
    /// Original provider invocation identity.
    pub original_invocation: String,
    /// Whether this invocation reused the original response.
    pub reused: bool,
}

/// In-memory repository for complete normalized model responses.
#[derive(Clone, Default)]
pub struct ModelCache {
    records: Arc<Mutex<BTreeMap<String, CachedResponse>>>,
}

impl ModelCache {
    /// Creates an empty model-result cache.
    ///
    /// An empty cache ready for model responses.
    #[must_use]
    /// # Returns
    /// The operation result.
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes one policy-controlled cache lookup/provider call.
    ///
    /// - `policy`: Cache behavior for the invocation.
    /// - `fingerprint`: Exact request identity.
    /// - `invocation`: Current model invocation identity.
    /// - `provider`: Closure producing a complete normalized response.
    ///
    /// A response and its original/reuse provenance.
    ///
    /// # Returns
    /// The normalized provider response and cache provenance.
    ///
    /// # Errors
    /// Returns the provider or cache lock failure.
    pub fn execute<F>(
        &self,
        policy: CachePolicy,
        fingerprint: impl Into<String>,
        invocation: impl Into<String>,
        provider: F,
    ) -> Result<CachedResponse, String>
    where
        F: FnOnce() -> Result<ModelResponse, String>,
    {
        let fingerprint = fingerprint.into();
        let invocation = invocation.into();
        if policy == CachePolicy::ReuseExact {
            if let Some(record) = self
                .records
                .lock()
                .map_err(|_| "model cache lock failed".to_owned())?
                .get(&fingerprint)
                .cloned()
            {
                return Ok(CachedResponse {
                    response: record.response,
                    invocation,
                    original_invocation: record.original_invocation,
                    reused: true,
                });
            }
        }
        let response = provider()?;
        let result = CachedResponse {
            response,
            invocation: invocation.clone(),
            original_invocation: invocation,
            reused: false,
        };
        if policy == CachePolicy::RecordOnly || policy == CachePolicy::ReuseExact {
            self.records
                .lock()
                .map_err(|_| "model cache lock failed".to_owned())?
                .insert(fingerprint, result.clone());
        }
        Ok(result)
    }
}
