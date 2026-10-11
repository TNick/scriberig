//! Defines bounded opaque pagination requests and results.

/// Conservative default page size.
pub const DEFAULT_PAGE_LIMIT: usize = 50;
/// Maximum page size accepted by the generic query boundary.
pub const MAX_PAGE_LIMIT: usize = 500;

/// Opaque continuation state tied to its producing operation and snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
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
}

/// Requests one bounded page of semantic results.
#[derive(Clone, Debug, Eq, PartialEq)]
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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Page<T> {
    /// Items returned for this page.
    pub items: Vec<T>,
    /// Continuation for the same operation and snapshot context.
    pub next: Option<ContinuationToken>,
}
