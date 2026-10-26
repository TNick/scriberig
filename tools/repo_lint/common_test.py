"""Tests repository source discovery with vendored dependency exclusions."""

from pathlib import Path

from tools.repo_lint.common import collect_paths
from tools.repo_lint.languages import check_rust, check_typescript


ROOT = Path(__file__).resolve().parents[2]


def test_vendored_sources_are_excluded_from_repository_conventions() -> None:
    """Keeps upstream-owned source outside Scriberig style enforcement.

    Returns:
        `None` after the assertion passes.
    """
    source = ROOT / "third_party" / "rpi-tui" / "crate" / "src" / "lib.rs"

    assert collect_paths([str(source)]) == []


def test_first_party_sources_remain_in_repository_conventions() -> None:
    """Keeps Scriberig source subject to repository conventions.

    Returns:
        `None` after the assertion passes.
    """
    source = ROOT / "crates" / "scriberig-tui" / "src" / "lib.rs"

    assert collect_paths([str(source)]) == [source]


def test_throw_check_stops_at_the_next_typescript_declaration() -> None:
    """Does not attribute a later function's throw to an earlier helper.

    Returns:
        `None` after the assertion passes.
    """
    source = """/** Checks an object. */
function isObject(value: unknown): boolean {
  return typeof value === "object";
}

/** Parses an object. */
function parseObject(value: unknown): object {
  throw new Error("invalid object");
}
"""

    issues = check_typescript(source)

    assert any("parseObject" in issue and "THROW_DOC" in issue for issue in issues)
    assert not any("isObject" in issue and "THROW_DOC" in issue for issue in issues)


def test_rust_return_check_ignores_nested_function_pointer_returns() -> None:
    """Reads a function's return after its nested callback parameter.

    Returns:
        `None` after the assertion passes.
    """
    source = """/// Installs a panic hook.
///
/// - `restore`: Terminal restoration callback.
///
/// The previous hook.
/// # Returns
/// The previous process hook.
fn install(restore: fn() -> io::Result<()>) -> PanicHook {
    let _ = restore;
    todo!()
}
"""

    issues = check_rust(source)

    assert not any("ERROR_DOC" in issue for issue in issues)
