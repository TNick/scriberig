//! Decodes plan operations from HTTP protocol request bodies.

/// Decodes one plan operation into its stable protocol request variant.
///
/// # Parameters
/// - `operation`: Stable operation identifier from the URL.
/// - `body`: JSON request payload.
///
/// # Returns
/// The typed plan request, or `None` for a non-plan operation.
///
/// # Errors
/// Returns a JSON decoding error when a plan payload is malformed.
pub(super) fn decode(
    operation: &str,
    body: &str,
) -> Result<Option<scriberig_protocol::Request>, serde_json::Error> {
    let request = match operation {
        "plan_create" => Some(scriberig_protocol::Request::PlanCreate(
            serde_json::from_str(body)?,
        )),
        "plan_lifecycle" => Some(scriberig_protocol::Request::PlanLifecycle(
            serde_json::from_str(body)?,
        )),
        "plan_assignment" => Some(scriberig_protocol::Request::PlanAssignment(
            serde_json::from_str(body)?,
        )),
        "plan_replan" => Some(scriberig_protocol::Request::PlanReplan(
            serde_json::from_str(body)?,
        )),
        "plan_mutation" => Some(scriberig_protocol::Request::PlanMutation(
            serde_json::from_str(body)?,
        )),
        "plan_attempts" => Some(scriberig_protocol::Request::PlanAttempts(
            serde_json::from_str(body)?,
        )),
        "plan_details" => Some(scriberig_protocol::Request::PlanDetails(
            serde_json::from_str(body)?,
        )),
        _ => None,
    };
    Ok(request)
}
