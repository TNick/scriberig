//! Defines bounded opaque pagination requests and results.

/// Conservative default page size.
pub const DEFAULT_PAGE_LIMIT: usize = 50;
/// Maximum page size accepted by the generic query boundary.
pub const MAX_PAGE_LIMIT: usize = 500;

use scriberig_core::SnapshotId;

/// Opaque continuation state tied to its producing operation and snapshot.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct ContinuationToken(String);

impl ContinuationToken {
    /// Creates an opaque continuation token.
    ///
    /// # Parameters
    /// - `value`: Backend-independent opaque token value.
    ///
    /// # Returns
    /// A continuation token whose internals are not exposed as public fields.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Creates a continuation bound to one operation and snapshot.
    ///
    /// # Parameters
    /// - `operation`: Stable operation family producing the page.
    /// - `snapshot`: Exact snapshot used by the operation.
    /// - `offset`: Zero-based offset for the next page.
    ///
    /// # Returns
    /// An opaque continuation token that providers can validate on reuse.
    #[must_use]
    /// - `crate`: Argument named `crate`.
    pub(crate) fn for_page(operation: &str, snapshot: SnapshotId, offset: usize) -> Self {
        Self(format!("v1|{operation}|{snapshot}|{offset}"))
    }

    /// Decodes a continuation for its expected operation and snapshot.
    ///
    /// # Parameters
    /// - `self`: Continuation token to validate.
    /// - `operation`: Operation family receiving the token.
    /// - `snapshot`: Exact snapshot expected by the operation.
    ///
    /// # Returns
    /// The next-page offset when the token belongs to this context.
    #[must_use]
    /// - `crate`: Argument named `crate`.
    pub(crate) fn offset_for(&self, operation: &str, snapshot: SnapshotId) -> Option<usize> {
        let snapshot = snapshot.to_string();
        let mut parts = self.0.split('|');
        if parts.next() != Some("v1")
            || parts.next() != Some(operation)
            || parts.next() != Some(snapshot.as_str())
        {
            return None;
        }
        parts
            .next()?
            .parse()
            .ok()
            .filter(|_| parts.next().is_none())
    }
}

/// Requests one bounded page of semantic results.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct PageRequest {
    /// Maximum number of items requested.
    pub limit: usize,
    /// Continuation from the same operation and snapshot context.
    pub continuation: Option<ContinuationToken>,
}

impl Default for PageRequest {
    /// Creates a request using the conservative default limit.
    ///
    /// # Returns
    /// A page request with no continuation token.
    fn default() -> Self {
        Self {
            limit: DEFAULT_PAGE_LIMIT,
            continuation: None,
        }
    }
}

impl PageRequest {
    /// Validates and constructs a bounded page request.
    ///
    /// # Parameters
    /// - `limit`: Requested item count.
    /// - `continuation`: Optional opaque continuation token.
    ///
    /// # Returns
    /// A valid page request.
    ///
    /// # Errors
    /// Returns `PageRequestError::InvalidLimit` for zero or oversized limits.
    pub fn new(
        limit: usize,
        continuation: Option<ContinuationToken>,
    ) -> Result<Self, PageRequestError> {
        if !(1..=MAX_PAGE_LIMIT).contains(&limit) {
            return Err(PageRequestError::InvalidLimit { limit });
        }
        Ok(Self {
            limit,
            continuation,
        })
    }
}

/// Reports invalid pagination input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageRequestError {
    /// Requested limit is outside the accepted bounds.
    InvalidLimit {
        /// Rejected requested limit.
        limit: usize,
    },
}

/// Contains one page of results and an optional opaque continuation.
#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Page<T> {
    /// Items returned for this page.
    pub items: Vec<T>,
    /// Continuation for the same operation and snapshot context.
    pub next: Option<ContinuationToken>,
}
