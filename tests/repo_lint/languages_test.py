"""Verify Rust visibility qualifiers are excluded from parameter checks."""

from tools.repo_lint.languages import check_rust


def test_visibility_is_not_a_function_parameter() -> None:
    """Accept a documented parameter without demanding visibility docs.

    Returns:
        None.
    """
    source = "\n".join([
        "/// Validate a file.",
        "///",
        "/// # Parameters",
        "/// - `path`: File path.",
        "pub(super) fn validate(path: &Path) {}",
    ])
    assert check_rust(source) == []


def test_restricted_visibility_still_checks_actual_parameters() -> None:
    """Reject an undocumented real parameter behind restricted visibility.

    Returns:
        None.
    """
    source = "\n".join([
        "/// Validate a file.",
        "pub(crate) fn validate(path: &Path) {}",
    ])
    issues = check_rust(source)
    assert any("document `path`" in issue for issue in issues)
    assert all("document `crate`" not in issue for issue in issues)


def test_tuple_types_and_mutable_receivers_are_not_parameters() -> None:
    """Inspect real parameters after a nested tuple without treating types as names.

    Returns:
        None.
    """
    source = "\n".join([
        "/// Match nodes.",
        "/// - `candidates`: Candidate tuples.",
        "/// - `path`: Stable path.",
        "pub(crate) fn nodes(mut self, candidates: &[(Vec<usize>, Block)], path: &[usize]) {}",
    ])
    assert check_rust(source) == []
