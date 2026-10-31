//! Finds authored CAD annotation text in bounded exact-snapshot views.

use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_domain_cad::{CadEntity, DrawingEntity};

use crate::{ProjectError, ProjectService};

/// Maximum number of CAD text matches returned by one search page.
pub const MAX_CAD_SEARCH_PAGE: usize = 100;

/// Identifies one authored text match in a semantic CAD drawing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadTextMatch {
    /// Complete semantic entity containing the matched annotation.
    pub entity: DrawingEntity,
    /// Authored annotation or dimension override text that matched.
    pub text: String,
}

/// Contains a bounded page of semantic CAD text matches.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CadTextSearch {
    /// Project owning the CAD drawing.
    pub project: ProjectId,
    /// Exact immutable snapshot used for every match.
    pub snapshot: SnapshotId,
    /// Stable semantic drawing identity.
    pub drawing: EntityId,
    /// Matching entities in source drawing order.
    pub matches: Vec<CadTextMatch>,
    /// Total matches before pagination.
    pub total: usize,
    /// Whether another page contains matches.
    pub has_more: bool,
}

impl ProjectService {
    /// Finds annotation text in a drawing at an exact project snapshot.
    ///
    /// Matching is case-insensitive substring search across text entities and
    /// authored dimension overrides. Geometry and all other entity fields are
    /// returned unchanged with each semantic match.
    ///
    /// # Parameters
    /// - `project`: Project owning the semantic drawing.
    /// - `snapshot`: Exact active snapshot to inspect.
    /// - `drawing`: Stable semantic drawing identity.
    /// - `query`: Non-empty text substring to find.
    /// - `offset`: Zero-based offset among matching entities.
    /// - `limit`: Maximum number of matches, from one through one hundred.
    ///
    /// # Returns
    /// Bounded semantic matches and continuation state for the exact snapshot.
    ///
    /// # Errors
    /// Returns stale-snapshot, invalid-query/page, ownership, missing-revision,
    /// or malformed-entity errors.
    pub fn find_cad_text(
        &self,
        project: ProjectId,
        snapshot: SnapshotId,
        drawing: EntityId,
        query: &str,
        offset: usize,
        limit: usize,
    ) -> Result<CadTextSearch, ProjectError> {
        if !(1..=MAX_CAD_SEARCH_PAGE).contains(&limit) {
            return Err(cad_search_error(
                "cad_search_page_invalid",
                "CAD search limit must be between one and one hundred",
            ));
        }
        if query.trim().is_empty() {
            return Err(cad_search_error(
                "cad_search_query_invalid",
                "CAD search text is empty",
            ));
        }
        let summary = self.cad_drawing(project, snapshot, drawing)?;
        let needle = query.to_lowercase();
        let mut matches = Vec::with_capacity(limit.saturating_add(1));
        let mut total = 0usize;
        let mut scan_offset = 0;
        while scan_offset < summary.entity_count {
            let entity_page = self.cad_entities(
                project,
                snapshot,
                drawing,
                scan_offset,
                crate::MAX_CAD_ENTITY_PAGE,
            )?;
            let page_len = entity_page.entities.len();
            for entity in &entity_page.entities {
                if let Some(text) = entity_text(entity)
                    && text.to_lowercase().contains(&needle)
                {
                    if total >= offset && matches.len() <= limit {
                        matches.push(CadTextMatch {
                            entity: entity.clone(),
                            text: text.to_owned(),
                        });
                    }
                    total = total.saturating_add(1);
                }
            }
            scan_offset += page_len;
        }
        if offset > total {
            return Err(cad_search_error(
                "cad_search_offset_invalid",
                "CAD search offset exceeds the matching entity count",
            ));
        }
        let has_more = matches.len() > limit;
        Ok(CadTextSearch {
            project,
            snapshot,
            drawing,
            matches: matches.into_iter().take(limit).collect(),
            total,
            has_more,
        })
    }
}

/// Returns the authored searchable annotation for a supported entity.
///
/// # Parameters
/// - `entity`: Semantic CAD entity to inspect.
///
/// # Returns
/// Authored text for text entities or non-empty dimension overrides.
fn entity_text(entity: &DrawingEntity) -> Option<&str> {
    match &entity.geometry {
        CadEntity::Text { value, .. } => Some(value),
        CadEntity::Dimension {
            text: Some(value), ..
        } if !value.trim().is_empty() => Some(value),
        _ => None,
    }
}

/// Creates a structured CAD search error.
///
/// # Parameters
/// - `code`: Stable failure category.
/// - `message`: Safe diagnostic detail.
///
/// # Returns
/// Project error without source package bytes.
fn cad_search_error(code: &'static str, message: &str) -> ProjectError {
    ProjectError {
        code,
        message: message.to_owned(),
        diagnostics: Vec::new(),
    }
}
