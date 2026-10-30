//! Supplies reusable fixtures and assertions for compound artifact adapters.

use std::sync::atomic::{AtomicUsize, Ordering};

use scriberig_core::Provenance;

/// Describes one exact member of a synthetic logical artifact.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactFixtureMember {
    /// Canonical artifact-relative member path.
    pub path: String,
    /// Exact source bytes for the member.
    pub bytes: Vec<u8>,
    /// Container member that owns this part, when the member is nested.
    pub container: Option<String>,
    /// Whether materialization requires this member.
    pub required: bool,
    /// Whether the member is intentionally opaque to semantic interpretation.
    pub opaque: bool,
}

/// Provides deterministic one-file, compound, and container artifact fixtures.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ArtifactFixture {
    /// Ordered artifact members.
    pub members: Vec<ArtifactFixtureMember>,
}

impl ArtifactFixture {
    /// Creates a fixture containing one required source file.
    ///
    /// # Parameters
    /// - `path`: Canonical project-relative source path.
    /// - `bytes`: Exact source bytes.
    ///
    /// # Returns
    /// A single-member artifact fixture.
    pub fn single_file(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self::with_member(path, bytes, None, true, false)
    }

    /// Creates a fixture containing a required compound member.
    ///
    /// # Parameters
    /// - `path`: Canonical path of the member within the artifact.
    /// - `bytes`: Exact member bytes.
    ///
    /// # Returns
    /// A one-member compound artifact fixture.
    pub fn multi_file_member(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self::single_file(path, bytes)
    }

    /// Creates a fixture for one part inside a container member.
    ///
    /// # Parameters
    /// - `container`: Path identifying the containing artifact member.
    /// - `part`: Canonical path of the nested part.
    /// - `bytes`: Exact nested part bytes.
    ///
    /// # Returns
    /// A fixture whose member records its container relationship.
    pub fn container_part(
        container: impl Into<String>,
        part: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> Self {
        Self::with_member(part, bytes, Some(container.into()), true, false)
    }

    /// Creates a required opaque member that must survive semantic edits.
    ///
    /// # Parameters
    /// - `path`: Canonical member path.
    /// - `bytes`: Exact uninterpreted source bytes.
    ///
    /// # Returns
    /// An opaque-preservation fixture.
    pub fn opaque_member(path: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self::with_member(path, bytes, None, true, true)
    }

    /// Appends another member to this artifact fixture.
    ///
    /// # Parameters
    /// - `member`: Member to append.
    ///
    /// # Returns
    /// The combined fixture.
    #[must_use]
    pub fn with(mut self, member: ArtifactFixtureMember) -> Self {
        self.members.push(member);
        self
    }

    /// Creates one member record with explicit ownership and preservation flags.
    ///
    /// # Parameters
    /// - `path`: Canonical member path.
    /// - `bytes`: Exact member bytes.
    /// - `container`: Optional containing member path.
    /// - `required`: Whether successful materialization requires this member.
    /// - `opaque`: Whether bytes are uninterpreted and must be preserved.
    ///
    /// # Returns
    /// A member fixture.
    pub fn member(
        path: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
        container: Option<String>,
        required: bool,
        opaque: bool,
    ) -> ArtifactFixtureMember {
        ArtifactFixtureMember {
            path: path.into(),
            bytes: bytes.into(),
            container,
            required,
            opaque,
        }
    }

    /// Returns the exact BLAKE3 digest for a member path.
    ///
    /// # Parameters
    /// - `path`: Member path to hash.
    ///
    /// # Returns
    /// The member digest when the path exists.
    #[must_use]
    pub fn digest(&self, path: &str) -> Option<[u8; 32]> {
        self.members
            .iter()
            .find(|member| member.path == path)
            .map(|member| *blake3::hash(&member.bytes).as_bytes())
    }

    /// Creates a fixture member using the selected ownership flags.
    ///
    /// # Parameters
    /// - `path`: Canonical member path.
    /// - `bytes`: Exact member bytes.
    /// - `container`: Optional containing member path.
    /// - `required`: Whether successful materialization requires this member.
    /// - `opaque`: Whether bytes are uninterpreted and must be preserved.
    ///
    /// # Returns
    /// A one-member artifact fixture.
    fn with_member(
        path: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
        container: Option<String>,
        required: bool,
        opaque: bool,
    ) -> Self {
        Self {
            members: vec![Self::member(path, bytes, container, required, opaque)],
        }
    }
}

/// Provides deterministic bounded pages over a synthetic large collection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PagedCollectionFixture<T> {
    /// Ordered synthetic collection entries.
    pub entries: Vec<T>,
    /// Largest page this fixture permits.
    pub maximum_page_size: usize,
}

impl<T> PagedCollectionFixture<T> {
    /// Creates a collection fixture with the shared 500-entry page ceiling.
    ///
    /// # Parameters
    /// - `entries`: Ordered collection values.
    ///
    /// # Returns
    /// A bounded synthetic collection.
    #[must_use]
    pub fn new(entries: Vec<T>) -> Self {
        Self {
            entries,
            maximum_page_size: 500,
        }
    }

    /// Reads one bounded page from the synthetic collection.
    ///
    /// # Parameters
    /// - `offset`: Zero-based first entry.
    /// - `limit`: Requested page size.
    ///
    /// # Returns
    /// A borrowed page and whether more entries remain.
    ///
    /// # Errors
    /// Returns an error when the page size is zero or exceeds the fixture cap.
    pub fn page(&self, offset: usize, limit: usize) -> Result<(&[T], bool), &'static str> {
        if limit == 0 || limit > self.maximum_page_size {
            return Err("page size is outside the fixture bound");
        }
        let start = offset.min(self.entries.len());
        let end = start.saturating_add(limit).min(self.entries.len());
        Ok((&self.entries[start..end], end < self.entries.len()))
    }
}

/// Injects a deterministic failure before the next member beyond a commit limit.
#[derive(Debug)]
pub struct MaterializationFailureInjector {
    /// Number of successful member commits allowed before failure.
    fail_after: usize,
    /// Number of successful member commits recorded so far.
    committed: AtomicUsize,
}

impl MaterializationFailureInjector {
    /// Creates an injector that fails after the selected number of commits.
    ///
    /// # Parameters
    /// - `fail_after`: Number of commits allowed before injection.
    ///
    /// # Returns
    /// A thread-safe deterministic failure injector.
    #[must_use]
    pub fn new(fail_after: usize) -> Self {
        Self {
            fail_after,
            committed: AtomicUsize::new(0),
        }
    }

    /// Records a member commit or injects the configured failure.
    ///
    /// # Returns
    /// The zero-based commit index when recorded.
    ///
    /// # Errors
    /// Returns `FailureInjection` once the configured commit count is reached.
    pub fn record_commit(&self) -> Result<usize, FailureInjection> {
        let mut current = self.committed.load(Ordering::SeqCst);
        loop {
            if current >= self.fail_after {
                return Err(FailureInjection {
                    committed_members: current,
                });
            }
            match self.committed.compare_exchange(
                current,
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return Ok(current),
                Err(observed) => current = observed,
            }
        }
    }

    /// Returns the number of member commits already recorded.
    ///
    /// # Returns
    /// Successful commit count.
    pub fn committed(&self) -> usize {
        self.committed.load(Ordering::SeqCst)
    }
}

/// Describes a deterministic materialization failure point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FailureInjection {
    /// Number of members committed before failure.
    pub committed_members: usize,
}

/// Describes whether one output member was required and committed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MaterializationMemberResult {
    /// Whether successful artifact materialization requires this member.
    pub required: bool,
    /// Whether this member was committed.
    pub committed: bool,
}

/// Asserts exact source bytes, digest, and provenance are unchanged.
///
/// # Parameters
/// - `expected_bytes`: Exact source bytes captured by the fixture.
/// - `observed_bytes`: Exact bytes returned by the adapter.
/// - `expected_provenance`: Source provenance recorded by the fixture.
/// - `observed_provenance`: Provenance returned by the adapter.
///
/// # Panics
/// Panics when source bytes, their BLAKE3 digest, or provenance differ.
pub fn assert_exact_source_evidence(
    expected_bytes: &[u8],
    observed_bytes: &[u8],
    expected_provenance: &Provenance,
    observed_provenance: &Provenance,
) {
    assert_eq!(observed_bytes, expected_bytes, "source bytes changed");
    assert_eq!(
        blake3::hash(observed_bytes),
        blake3::hash(expected_bytes),
        "source digest changed"
    );
    assert_eq!(
        observed_provenance, expected_provenance,
        "source provenance changed"
    );
}

/// Asserts a materialization never reports partial required-member success.
///
/// # Parameters
/// - `reported_success`: Overall materialization result.
/// - `members`: Per-member required and committed status.
///
/// # Panics
/// Panics when success is reported while any required member is uncommitted.
pub fn assert_no_partial_success(reported_success: bool, members: &[MaterializationMemberResult]) {
    if reported_success {
        assert!(
            members
                .iter()
                .all(|member| !member.required || member.committed),
            "materialization reported success before all required members committed"
        );
    }
}
