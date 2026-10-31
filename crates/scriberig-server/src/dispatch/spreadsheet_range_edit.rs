//! Dispatches bounded exact-base spreadsheet range candidates.

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::SnapshotId;
use scriberig_protocol::{
    Response, SpreadsheetCandidateValidation, SpreadsheetEditResponse, SpreadsheetRangeEditRequest,
};

use crate::server::ApplicationService;

use super::schema_error;

/// Proposes a bounded set of exact-base spreadsheet cell edits as one candidate.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact base, worksheet, and typed cell updates.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Reviewable candidate handle or structured application error.
pub(crate) fn propose_range<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: SpreadsheetRangeEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid snapshot identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_spreadsheet_range_edit(
            &context,
            &request.project,
            &request.worksheet,
            request.edits,
        )
        .map_or_else(
            |error| Response::Error(super::super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                Response::SpreadsheetRangeEdit(SpreadsheetEditResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation: map_candidate_validation(candidate.validation),
                })
            },
        )
}

/// Maps project candidate validation to the compact spreadsheet protocol form.
///
/// # Parameters
/// - `validation`: Shared project candidate validation state.
///
/// # Returns
/// Spreadsheet transport validation state.
fn map_candidate_validation(
    validation: scriberig_project::CandidateValidation,
) -> SpreadsheetCandidateValidation {
    match validation {
        scriberig_project::CandidateValidation::Pending => SpreadsheetCandidateValidation::Pending,
        scriberig_project::CandidateValidation::Passed => SpreadsheetCandidateValidation::Passed,
        scriberig_project::CandidateValidation::Failed { diagnostics } => {
            SpreadsheetCandidateValidation::Failed { diagnostics }
        }
    }
}
