//! Reads branch results without reconstructing the complete session graph.

use serde::Deserialize;

/// Selects the current or versioned session body from the stored envelope.
#[derive(Deserialize)]
struct SessionPayload {
    /// Versioned payload body when legacy selections are retained.
    session: Option<SessionBranches>,
    /// Direct session branch list for the normal payload format.
    branches: Option<Vec<BranchResultPayload>>,
}

/// Retains only branch result records from one session body.
#[derive(Deserialize)]
struct SessionBranches {
    /// Branches whose result records support plan readiness evaluation.
    branches: Vec<BranchResultPayload>,
}

/// Skips every branch field except its optional result.
#[derive(Deserialize)]
struct BranchResultPayload {
    /// Exact result record associated with this branch.
    result: Option<scriberig_session::BranchResult>,
}

/// Decodes branch results from either supported session payload shape.
///
/// - `payload`: Stored JSON bytes for one session.
///
/// Branch results needed by compact plan projections.
///
/// Returns a diagnostic when the JSON is invalid or no session body exists.
/// # Returns
/// The branch results from the selected session body.
/// # Errors
/// Returns a decoding diagnostic for malformed payloads.
pub(crate) fn decode(payload: &[u8]) -> Result<Vec<scriberig_session::BranchResult>, String> {
    let parsed: SessionPayload =
        serde_json::from_slice(payload).map_err(|error| error.to_string())?;
    let branches = parsed
        .session
        .map(|session| session.branches)
        .or(parsed.branches)
        .ok_or_else(|| "stored session payload has no branch list".to_owned())?;
    Ok(branches
        .into_iter()
        .filter_map(|branch| branch.result)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::decode;

    /// Verifies branch turns are skipped in both supported payload shapes.
    #[test]
    fn decodes_results_without_decoding_other_branch_fields() {
        let result = scriberig_session::BranchResult {
            id: scriberig_core::EntityId::from_bytes([81; 16]),
            branch: scriberig_core::EntityId::from_bytes([82; 16]),
            status: scriberig_session::BranchStatus::Completed,
            summary: Some("done".to_owned()),
            result_kind: None,
            candidate_context: None,
            references: Vec::new(),
            result_handles: Vec::new(),
            touched: Vec::new(),
            produced_changes: Vec::new(),
            validations: Vec::new(),
            unresolved_questions: Vec::new(),
        };
        let result_json = serde_json::to_value(&result).expect("serialize result");
        let payload = serde_json::json!({
            "id": scriberig_core::EntityId::from_bytes([83; 16]),
            "branches": [{"result": result_json, "turns": "ignored"}],
            "plans": "ignored"
        });

        let direct = serde_json::to_vec(&payload).expect("serialize session");
        let versioned = serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "session": payload,
            "legacy_execution": []
        }))
        .expect("serialize versioned session");
        let decoded = decode(&direct).expect("decode branch results");
        let wrapped = decode(&versioned).expect("decode versioned branch results");

        assert_eq!(decoded[0].id, result.id);
        assert_eq!(wrapped[0].id, result.id);
    }
}
