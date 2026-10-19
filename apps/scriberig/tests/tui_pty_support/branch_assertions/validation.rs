//! Asserts the ordered dependency and code validation evidence on an implementation branch.

use scriberig_session::Branch;

/// Verifies the implementation child requested every registered profile in policy order.
///
/// - `branch`: Durable implementation branch.
/// - `super`: Parent assertion module.
///
/// Unit when all required validation results are present and successful.
///
/// # Panics
/// Panics when a profile call or its durable successful result is missing.
pub(super) fn assert_validation_profiles(branch: &Branch) {
    let expected = [
        "php-domain-focused",
        "project-import-focused",
        "rust-workspace",
        "project-ingestion-e2e",
    ];
    let invocations = branch
        .tool_invocations
        .iter()
        .filter(|invocation| invocation.name == "project_validate_candidate")
        .collect::<Vec<_>>();
    assert_eq!(
        invocations.len(),
        expected.len(),
        "validation profile calls: {:?}; validation results: {:?}",
        invocations
            .iter()
            .map(
                |invocation| serde_json::from_slice::<serde_json::Value>(&invocation.input)
                    .ok()
                    .and_then(|input| input["profile"].as_str().map(str::to_owned))
            )
            .collect::<Vec<_>>(),
        invocations
            .iter()
            .filter_map(|invocation| {
                branch
                    .tool_results
                    .iter()
                    .find(|result| result.invocation == invocation.id)
            })
            .map(|result| (result.failed, diagnostic_tail(&result.output)))
            .collect::<Vec<_>>()
    );
    for (invocation, expected_profile) in invocations.iter().zip(expected) {
        let input: serde_json::Value =
            serde_json::from_slice(&invocation.input).expect("validation profile input");
        assert_eq!(input["profile"], expected_profile);
        let result = branch
            .tool_results
            .iter()
            .find(|result| result.invocation == invocation.id)
            .expect("durable validation result");
        assert!(!result.failed, "profile {expected_profile} failed");
        let output: serde_json::Value =
            serde_json::from_slice(&result.output).expect("validation result JSON");
        assert!(output["handles"].as_array().is_some_and(|handles| {
            handles
                .iter()
                .any(|handle| handle == &format!("validation_profile:{expected_profile}"))
        }));
    }
}

/// Keeps profile diagnostics readable when an assertion fails.
///
/// - `output`: Durable result bytes from one validation profile.
///
/// The final 1200 characters of the result.
/// # Returns
/// The bounded diagnostic suffix.
fn diagnostic_tail(output: &[u8]) -> String {
    let output = String::from_utf8_lossy(output);
    output
        .chars()
        .rev()
        .take(1200)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// Verifies locked dependency acquisition completed before offline code validation.
///
/// - `branch`: Durable implementation branch.
/// - `super`: Parent assertion module.
///
/// Unit when dependency acquisition precedes validation and is durably successful.
///
/// # Panics
/// Panics when acquisition is missing, failed, or ordered after code validation.
pub(super) fn assert_dependency_resolution_precedes_validation(branch: &Branch) {
    let acquisition = branch
        .tool_invocations
        .iter()
        .find(|invocation| invocation.name == "project_resolve_dependencies")
        .expect("locked dependency acquisition call");
    let validation = branch
        .tool_invocations
        .iter()
        .find(|invocation| invocation.name == "project_validate_candidate")
        .expect("first candidate validation call");
    let acquisition_index = branch
        .tool_invocations
        .iter()
        .position(|invocation| invocation.id == acquisition.id)
        .expect("acquisition invocation position");
    let validation_index = branch
        .tool_invocations
        .iter()
        .position(|invocation| invocation.id == validation.id)
        .expect("validation invocation position");
    assert!(acquisition_index < validation_index);
    let result = branch
        .tool_results
        .iter()
        .find(|result| result.invocation == acquisition.id)
        .expect("durable dependency acquisition result");
    assert!(!result.failed, "locked dependency acquisition failed");
    let output: serde_json::Value =
        serde_json::from_slice(&result.output).expect("dependency acquisition result JSON");
    assert!(output["handles"].as_array().is_some_and(|handles| {
        handles
            .iter()
            .any(|handle| handle == "validation_profile:cargo-dependency-resolution")
    }));
}
