//! Maps application and protocol artifact-member pages to one TUI model.

use super::artifacts::{ArtifactMemberRow, encode_digest};

/// Contains one bounded terminal-safe page of artifact members.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactMembersModel {
    /// Stable project identity owning the artifact.
    pub project: String,
    /// Stable semantic artifact identity.
    pub artifact: String,
    /// Exact member descriptors in this page.
    pub members: Vec<ArtifactMemberRow>,
    /// Total physical and container-part members.
    pub total: usize,
    /// Offset for the next page, when available.
    pub next_offset: Option<usize>,
}

impl ArtifactMembersModel {
    /// Converts a shared application member page into terminal-safe metadata.
    ///
    /// # Parameters
    /// - `page`: Authorized member page with exact source digests.
    ///
    /// # Returns
    /// A terminal-safe member page without source bytes.
    #[must_use]
    pub fn from_application(page: scriberig_app::ArtifactMembers) -> Self {
        Self {
            project: page.project.to_string(),
            artifact: page.artifact.to_string(),
            members: page
                .members
                .into_iter()
                .map(|member| ArtifactMemberRow {
                    key: member.key.0,
                    project_path: member.project_path,
                    container_part: member.container_part,
                    digest: encode_digest(&member.digest),
                })
                .collect(),
            total: page.total,
            next_offset: page.next_offset,
        }
    }

    /// Converts the typed remote member DTO into the same presentation model.
    ///
    /// # Parameters
    /// - `page`: Typed protocol member page.
    ///
    /// # Returns
    /// A terminal-safe page equivalent to the local application mapping.
    #[must_use]
    pub fn from_protocol(page: scriberig_protocol::ArtifactMembersResponse) -> Self {
        Self {
            project: page.project,
            artifact: page.artifact,
            members: page
                .members
                .into_iter()
                .map(|member| ArtifactMemberRow {
                    key: member.key,
                    project_path: member.project_path,
                    container_part: member.container_part,
                    digest: member.digest,
                })
                .collect(),
            total: page.total,
            next_offset: page.next_offset,
        }
    }
}
