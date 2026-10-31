//! Dispatches semantic document paragraph edit candidates.

mod map;

use std::str::FromStr;

use scriberig_app::{ApplicationContext, CapabilityContext, Principal};
use scriberig_core::{EntityId, ProjectId, SnapshotId};
use scriberig_protocol::{
    DocumentCandidateValidation, DocumentInspectRequest, DocumentInspectResponse,
    DocumentParagraphEditRequest, DocumentParagraphEditResponse, DocumentParagraphFindRequest,
    DocumentParagraphFindResponse, DocumentParagraphInsertRequest, DocumentParagraphInsertResponse,
    DocumentParagraphMatch as WireParagraphMatch, DocumentStyleView as WireStyle,
    DocumentTableCellEditRequest, ProtocolError, ProtocolErrorCode, Response,
};

use crate::server::ApplicationService;

/// Finds matching paragraphs using host-injected exact-snapshot Inspect authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Scoped grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact-snapshot document text query and page selection.
/// - `correlation_id`: Protocol request tracing identity.
///
/// # Returns
/// Typed bounded semantic paragraph matches or a structured application error.
pub(super) fn find_paragraphs<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &DocumentParagraphFindRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return schema_error("invalid document snapshot identity", correlation_id);
    };
    let Ok(project) = ProjectId::from_str(&request.project) else {
        return schema_error("invalid document project identity", correlation_id);
    };
    let Ok(document) = EntityId::from_str(&request.document) else {
        return schema_error("invalid document identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    let search = scriberig_app::DocumentParagraphSearchRequest {
        project,
        snapshot,
        document,
        query: request.query.clone(),
        offset: request.offset,
        limit: request.limit,
    };
    service
        .find_document_paragraphs(&context, &search)
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |result| {
                Response::DocumentParagraphFind(DocumentParagraphFindResponse {
                    project: result.project.to_string(),
                    snapshot: result.snapshot.to_string(),
                    document: result.document.to_string(),
                    matches: result
                        .matches
                        .into_iter()
                        .map(|result| WireParagraphMatch {
                            paragraph: result.paragraph.to_string(),
                            section: result.section.to_string(),
                            text: result.text,
                        })
                        .collect(),
                    total: result.total,
                    has_more: result.has_more,
                })
            },
        )
}

/// Reads a typed document tree from one exact snapshot using host-injected authority.
///
/// - service: Shared application boundary.
/// - capabilities: Scoped grants resolved by the trusted host.
/// - principal: Authenticated caller established by the transport.
/// - request: Exact project, snapshot, and document identities.
/// - `correlation_id`: Request tracing identity.
///
/// Typed semantic document tree or structured application error.
pub(super) fn inspect<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &DocumentInspectRequest,
    correlation_id: &str,
) -> Response {
    let identities = (
        request.project.parse::<ProjectId>(),
        request.snapshot.parse::<SnapshotId>(),
        request.document.parse::<EntityId>(),
    );
    let (Ok(project), Ok(snapshot), Ok(document)) = identities else {
        return schema_error("invalid document identity", correlation_id);
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .inspect_document(
            &context,
            &project.to_string(),
            &snapshot.to_string(),
            &document.to_string(),
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |tree| {
                Response::DocumentInspect(DocumentInspectResponse {
                    project: tree.project.to_string(),
                    snapshot: tree.snapshot.to_string(),
                    document: tree.document.to_string(),
                    artifact_capability: tree
                        .artifact_capability
                        .map(super::helpers::map_artifact_edit_capability),
                    styles: tree
                        .styles
                        .into_iter()
                        .map(|style| WireStyle {
                            key: style.key,
                            name: style.name,
                        })
                        .collect(),
                    sections: tree.sections.into_iter().map(map::map_section).collect(),
                })
            },
        )
}

/// Proposes an exact-base paragraph edit using only host-injected authority.
///
/// - service: Shared application boundary.
/// - capabilities: Scoped grants resolved by the trusted host.
/// - principal: Authenticated caller established by the transport.
/// - request: Snapshot, paragraph identity, and replacement text.
/// - `correlation_id`: Request tracing identity.
///
/// Candidate handle for review or structured application error.
pub(super) fn propose<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &DocumentParagraphEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return Response::Error(ProtocolError {
            code: ProtocolErrorCode::SchemaViolation,
            message: "invalid snapshot identity".into(),
            details: None,
            correlation_id: Some(correlation_id.to_owned()),
        });
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_document_paragraph_edit(
            &context,
            &request.project,
            &request.paragraph,
            &request.replacement,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        DocumentCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        DocumentCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        DocumentCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::DocumentParagraphEdit(DocumentParagraphEditResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Creates an exact-base table-cell candidate with trusted Propose authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Scoped grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact snapshot, cell identity, and replacement text.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Candidate handle for shared review or structured application error.
pub(super) fn propose_table_cell<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &DocumentTableCellEditRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return Response::Error(ProtocolError {
            code: ProtocolErrorCode::SchemaViolation,
            message: "invalid snapshot identity".into(),
            details: None,
            correlation_id: Some(correlation_id.to_owned()),
        });
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_document_table_cell_edit(
            &context,
            &request.project,
            &request.cell,
            &request.replacement,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        DocumentCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        DocumentCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        DocumentCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::DocumentTableCellEdit(DocumentParagraphEditResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Creates a paragraph insertion candidate using trusted Propose authority.
///
/// # Parameters
/// - `service`: Shared application boundary.
/// - `capabilities`: Scoped grants resolved by the trusted host.
/// - `principal`: Authenticated caller established by the transport.
/// - `request`: Exact-base section and authored paragraph text.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Candidate handle for shared validation and review, or a structured error.
pub(super) fn insert_paragraph<S: ApplicationService>(
    service: &S,
    capabilities: &CapabilityContext,
    principal: Option<&Principal>,
    request: &DocumentParagraphInsertRequest,
    correlation_id: &str,
) -> Response {
    let Ok(snapshot) = SnapshotId::from_str(&request.snapshot) else {
        return Response::Error(ProtocolError {
            code: ProtocolErrorCode::SchemaViolation,
            message: "invalid snapshot identity".into(),
            details: None,
            correlation_id: Some(correlation_id.to_owned()),
        });
    };
    let context = ApplicationContext {
        snapshot: Some(snapshot),
        principal: principal.cloned(),
        capabilities: capabilities.clone(),
        correlation_id: Some(correlation_id.to_owned()),
        ..ApplicationContext::default()
    };
    service
        .propose_document_paragraph_insert(
            &context,
            &request.project,
            &request.section,
            &request.text,
        )
        .map_or_else(
            |error| Response::Error(super::helpers::map_error(&error, correlation_id)),
            |candidate| {
                let validation = match candidate.validation {
                    scriberig_project::CandidateValidation::Pending => {
                        DocumentCandidateValidation::Pending
                    }
                    scriberig_project::CandidateValidation::Passed => {
                        DocumentCandidateValidation::Passed
                    }
                    scriberig_project::CandidateValidation::Failed { diagnostics } => {
                        DocumentCandidateValidation::Failed { diagnostics }
                    }
                };
                Response::DocumentParagraphInsert(DocumentParagraphInsertResponse {
                    project: candidate.project.to_string(),
                    candidate_id: candidate.id,
                    base_snapshot: candidate.base_snapshot.to_string(),
                    path: candidate.path,
                    validation,
                })
            },
        )
}

/// Returns a schema violation for malformed document identities.
///
/// # Parameters
/// - `message`: Safe identity-validation detail.
/// - `correlation_id`: Request tracing identity.
///
/// # Returns
/// Protocol error response.
fn schema_error(message: &str, correlation_id: &str) -> Response {
    Response::Error(ProtocolError {
        code: ProtocolErrorCode::SchemaViolation,
        message: message.to_owned(),
        details: None,
        correlation_id: Some(correlation_id.to_owned()),
    })
}
