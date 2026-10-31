//! Defines transport contracts for snapshot-bound sparse spreadsheet reads.

use crate::ArtifactEditCapability;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Requests one sparse semantic cell at an exact snapshot and coordinate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellInspectRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic worksheet identity.
    pub worksheet: String,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
}

/// Returns one typed cell value or confirms a sparse blank coordinate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellInspectResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity used by the read.
    pub snapshot: String,
    /// Stable semantic worksheet identity.
    pub worksheet: String,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
    /// Semantic cell when present; absent cells are blank by sparse schema.
    pub cell: Option<SpreadsheetCellView>,
}

/// Requests ordered workbook and worksheet summaries at an exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetWorkbookRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic workbook identity.
    pub workbook: String,
}

/// Returns one bounded workbook summary and its ordered worksheets.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetWorkbookResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity used by the read.
    pub snapshot: String,
    /// Stable semantic workbook identity.
    pub workbook: String,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactEditCapability>,
    /// Worksheets in workbook collection order.
    pub worksheets: Vec<SpreadsheetWorksheetSummary>,
}

/// Requests bounded authored-value search in a workbook at an exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellFindRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable semantic workbook identity.
    pub workbook: String,
    /// Non-empty case-insensitive authored-value substring.
    pub query: String,
    /// Zero-based match offset.
    pub offset: usize,
    /// Maximum matches requested, at most one hundred.
    pub limit: usize,
}

/// Describes one semantic spreadsheet cell match.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellMatch {
    /// Stable semantic worksheet identity.
    pub worksheet: String,
    /// Stable semantic cell identity.
    pub cell: String,
    /// One-based cell row.
    pub row: u32,
    /// One-based cell column.
    pub column: u32,
    /// Authored value or formula expression that matched.
    pub authored_value: String,
}

/// Returns bounded snapshot-bound spreadsheet cell matches.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellFindResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot used for every match.
    pub snapshot: String,
    /// Stable semantic workbook identity.
    pub workbook: String,
    /// Matching stable semantic handles in workbook order.
    pub matches: Vec<SpreadsheetCellMatch>,
    /// Total matches before pagination.
    pub total: usize,
    /// Whether another result page exists.
    pub has_more: bool,
}

/// Describes one worksheet's semantic identity and bounded used range.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetWorksheetSummary {
    /// Stable semantic worksheet identity.
    pub id: String,
    /// Authored worksheet name.
    pub name: String,
    /// Bounded authored cell extent when available.
    pub used_range: Option<SpreadsheetRange>,
    /// Named tables available in this worksheet snapshot.
    pub tables: Vec<SpreadsheetTableSummary>,
}

/// Describes one stable semantic table and its inclusive worksheet range.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetTableSummary {
    /// Stable table semantic identity.
    pub id: String,
    /// Authored table name.
    pub name: String,
    /// Exact worksheet range occupied by the table.
    pub range: SpreadsheetRange,
}

/// Requests one bounded worksheet cell page at an exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellsRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity.
    pub snapshot: String,
    /// Stable worksheet semantic identity.
    pub worksheet: String,
    /// Optional inclusive rectangular row and column bounds.
    pub range: Option<SpreadsheetRange>,
    /// Zero-based page offset after row/column ordering.
    pub offset: usize,
    /// Maximum number of cells requested, at most five hundred.
    pub limit: usize,
}

/// Requests an exact-base spreadsheet cell edit candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable worksheet semantic identity.
    pub worksheet: String,
    /// One-based target row.
    pub row: u32,
    /// One-based target column.
    pub column: u32,
    /// New authored semantic value or formula.
    pub value: SpreadsheetCellValue,
}

/// One typed cell value in a bounded spreadsheet range edit request.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellEdit {
    /// One-based target row.
    pub row: u32,
    /// One-based target column.
    pub column: u32,
    /// Authored value or formula.
    pub value: SpreadsheetCellValue,
}

/// Requests a bounded multi-cell edit as one exact-base candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetRangeEditRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable semantic worksheet identity.
    pub worksheet: String,
    /// One through five hundred distinct cell updates.
    pub edits: Vec<SpreadsheetCellEdit>,
}

/// Requests an exact-base semantic worksheet rename candidate.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetRenameRequest {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable base snapshot.
    pub snapshot: String,
    /// Stable semantic worksheet identity.
    pub worksheet: String,
    /// Requested authored worksheet name.
    pub new_name: String,
}

/// Returns the reviewable candidate handle for a spreadsheet edit.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetEditResponse {
    /// Stable project identity.
    pub project: String,
    /// Candidate identity for shared validation and review.
    pub candidate_id: String,
    /// Exact base snapshot used by the candidate.
    pub base_snapshot: String,
    /// Project-relative workbook path.
    pub path: String,
    /// Candidate validation status.
    pub validation: SpreadsheetCandidateValidation,
}

/// Describes validation state without exposing staged source bytes.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SpreadsheetCandidateValidation {
    /// Candidate has not yet been validated in complete project state.
    Pending,
    /// Candidate passed semantic and project validation.
    Passed,
    /// Candidate failed validation with bounded diagnostics.
    Failed {
        /// Safe validation diagnostics.
        diagnostics: Vec<String>,
    },
}

/// Describes inclusive one-based worksheet coordinate bounds.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpreadsheetRange {
    /// First row, inclusive.
    pub first_row: u32,
    /// First column, inclusive.
    pub first_column: u32,
    /// Last row, inclusive.
    pub last_row: u32,
    /// Last column, inclusive.
    pub last_column: u32,
}

/// Returns a bounded page of sparse cells from one exact snapshot.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellsResponse {
    /// Stable project identity.
    pub project: String,
    /// Exact immutable snapshot identity used by the read.
    pub snapshot: String,
    /// Stable worksheet semantic identity.
    pub worksheet: String,
    /// Edit capability of the exact source artifact, when catalogued.
    pub artifact_capability: Option<ArtifactEditCapability>,
    /// Adapter diagnostic explaining restricted edit capability, when present.
    pub artifact_diagnostic: Option<String>,
    /// Sparse cells ordered by row then column.
    pub cells: Vec<SpreadsheetCellView>,
    /// Total matching cell count before pagination.
    pub total: usize,
    /// Whether a later page contains more cells.
    pub has_more: bool,
}

/// Describes one typed semantic spreadsheet cell.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellView {
    /// Stable Scriberig cell identity.
    pub id: String,
    /// One-based row coordinate.
    pub row: u32,
    /// One-based column coordinate.
    pub column: u32,
    /// Authored value or formula with a non-authoritative cached result.
    pub value: SpreadsheetCellValue,
    /// Adapter-neutral style identity, when present.
    pub style: Option<SpreadsheetCellStyle>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
/// Represents an exact semantic cell value on the wire without promoting cached formula results.
pub enum SpreadsheetCellValue {
    /// Explicit blank value.
    Blank,
    /// Authored UTF-8 text.
    Text(String),
    /// Exact numeric text.
    Number(String),
    /// Boolean value.
    Boolean(bool),
    /// Authored spreadsheet error.
    Error(String),
    /// Authored formula and optional advisory cached result.
    Formula {
        /// Formula expression as authored.
        expression: String,
        /// Cached result, not treated as authoritative.
        cached_value: Option<Box<SpreadsheetCellValue>>,
    },
}

/// Describes a format-neutral style identity.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SpreadsheetCellStyle {
    /// Adapter-neutral style key.
    pub key: String,
    /// Source adapter identifier.
    pub adapter: String,
}
