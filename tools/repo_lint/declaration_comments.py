"""Find removable explanatory comments attached to documented declarations."""

from __future__ import annotations

import ast
import re

from .config import RUST_FUNCTION, RUST_TYPE, TS_CLASS, TS_FUNCTION, TS_METHOD

_RUST_ADDITIONAL_TYPE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:type|union)\s+(\w+)"
)
_TS_ADDITIONAL_TYPE = re.compile(
    r"^\s*(?:export\s+)?(?:declare\s+)?(?:interface|type|enum)\s+(\w+)"
)
_TS_ARROW_FUNCTION = re.compile(
    r"^\s*(?:export\s+)?(?:const|let|var)\s+\w+\s*"
    r"(?::[^=]+)?=\s*(?:async\s+)?(?:\([^)]*\)|[\w$]+)\s*=>"
)
_DIRECTIVES = (
    "TODO", "FIXME", "NOTE", "SAFETY", "HACK", "BUG", "NOQA", "TYPE:",
    "PRAGMA", "REGION", "ENDREGION", "COVERAGE", "ESLINT", "TSLINT",
    "CLANG-FORMAT", "C-SPELL", "PRETTIER", "PYLINT", "NOLINT",
    "@DEPRECATED", "@LICENSE", "@TS-", "@NOLINT", "COPYRIGHT",
)


def redundant_comment_lines(suffix: str, text: str) -> set[int]:
    """Find explanatory comments adjacent to documented declarations.

    Args:
        suffix: Source file suffix selecting the language.
        text: Complete source file contents.

    Returns:
        Zero-based line indexes of redundant explanatory comments.
    """

    lines = text.splitlines()
    if suffix == ".rs":
        return _rust_comment_lines(lines)
    if suffix in {".ts", ".tsx", ".js", ".jsx"}:
        return _typescript_comment_lines(lines)
    if suffix == ".py":
        return _python_comment_lines(lines, text)
    return set()


def _rust_comment_lines(lines: list[str]) -> set[int]:
    """Find comments next to documented Rust functions and types.

    Args:
        lines: Complete Rust source lines.

    Returns:
        Zero-based indexes of redundant explanatory comments.
    """

    found: set[int] = set()
    for index, line in enumerate(lines):
        if (
            not RUST_FUNCTION.match(line)
            and not RUST_TYPE.match(line)
            and not _RUST_ADDITIONAL_TYPE.match(line)
        ):
            continue
        docs = _rust_doc_indexes(lines, index)
        if docs:
            found.update(_between_comments(lines, max(docs), index, "rust"))
            found.update(_preceding_comments(lines, min(docs), "rust"))
    return found


def _rust_doc_indexes(lines: list[str], declaration_index: int) -> list[int]:
    """Locate `///` lines attached to a Rust item.

    Args:
        lines: Complete Rust source lines.
        declaration_index: Zero-based item declaration line.

    Returns:
        Zero-based indexes of attached documentation lines.
    """

    indexes: list[int] = []
    index = declaration_index - 1
    while index >= 0:
        stripped = lines[index].strip()
        if not stripped or stripped.startswith("#["):
            index -= 1
            continue
        if _ordinary_comment(stripped, "rust"):
            index -= 1
            continue
        if stripped.startswith("///") and not stripped.startswith("////"):
            indexes.append(index)
            index -= 1
            continue
        break
    return indexes


def _typescript_comment_lines(lines: list[str]) -> set[int]:
    """Find comments next to documented JavaScript-family declarations.

    Args:
        lines: Complete JavaScript or TypeScript source lines.

    Returns:
        Zero-based indexes of redundant explanatory comments.
    """

    found: set[int] = set()
    for index, line in enumerate(lines):
        function_match = (
            TS_FUNCTION.match(line)
            or TS_METHOD.match(line)
            or _TS_ARROW_FUNCTION.match(line)
        )
        if (
            not function_match
            and not TS_CLASS.match(line)
            and not _TS_ADDITIONAL_TYPE.match(line)
        ):
            continue
        span = _jsdoc_span(lines, index)
        if span is None:
            continue
        start, end = span
        found.update(_between_comments(lines, end, index, "typescript"))
        found.update(_preceding_comments(lines, start, "typescript"))
    return found


def _jsdoc_span(
    lines: list[str], declaration_index: int
) -> tuple[int, int] | None:
    """Locate a JSDoc block attached to a declaration.

    Args:
        lines: Complete JavaScript or TypeScript source lines.
        declaration_index: Zero-based declaration line.

    Returns:
        Start and end indexes, or `None` when no JSDoc is attached.
    """

    cursor = declaration_index - 1
    while cursor >= 0:
        stripped = lines[cursor].strip()
        if not stripped or stripped.startswith("@"):
            cursor -= 1
            continue
        if stripped.startswith("//"):
            cursor -= 1
            continue
        if "*/" not in stripped:
            break
        end = cursor
        while cursor >= 0:
            if "/**" in lines[cursor]:
                return cursor, end
            if "/*" in lines[cursor]:
                cursor -= 1
                break
            cursor -= 1
    return None


def _python_comment_lines(lines: list[str], text: str) -> set[int]:
    """Find comments next to documented Python functions and classes.

    Args:
        lines: Complete Python source lines.
        text: Complete Python source text.

    Returns:
        Zero-based indexes of redundant explanatory comments.
    """

    try:
        tree = ast.parse(text)
    except SyntaxError:
        return set()
    found: set[int] = set()
    for node in ast.walk(tree):
        if not isinstance(
            node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)
        ):
            continue
        if not ast.get_docstring(node, clean=False):
            continue
        declaration = min(
            (decorator.lineno for decorator in node.decorator_list),
            default=node.lineno,
        ) - 1
        found.update(_preceding_comments(lines, declaration, "python"))
        first_statement = node.body[0]
        if _is_string_expression(first_statement):
            doc_line = first_statement.lineno - 1
            indentation = len(lines[doc_line]) - len(lines[doc_line].lstrip())
            found.update(
                _preceding_comments(lines, doc_line, "python", indentation)
            )
    return found


def _is_string_expression(node: ast.stmt) -> bool:
    """Return whether a statement is a literal string expression.

    Args:
        node: Python statement to inspect.

    Returns:
        `True` when the statement can serve as a docstring.
    """

    return (
        isinstance(node, ast.Expr)
        and isinstance(node.value, ast.Constant)
        and isinstance(node.value.value, str)
    )


def _between_comments(
    lines: list[str], start: int, end: int, language: str
) -> set[int]:
    """Find explanatory comments between documentation and a declaration.

    Args:
        lines: Complete source lines.
        start: Last documentation line index.
        end: Declaration line index.
        language: Comment syntax identifier.

    Returns:
        Zero-based indexes of explanatory comments.
    """

    return {
        index
        for index in range(start + 1, end)
        if _explanatory_comment(lines[index].strip(), language)
    }


def _preceding_comments(
    lines: list[str], end: int, language: str, indentation: int | None = None
) -> set[int]:
    """Find a directly preceding explanatory comment block.

    Args:
        lines: Complete source lines.
        end: Index immediately after the inspected block.
        language: Comment syntax identifier.
        indentation: Required comment indentation when supplied.

    Returns:
        Zero-based indexes of explanatory comments.
    """

    found: set[int] = set()
    index = end - 1
    if index >= 0 and not lines[index].strip():
        index -= 1
    while index >= 0:
        stripped = lines[index].strip()
        if indentation is not None:
            actual = len(lines[index]) - len(lines[index].lstrip())
            if actual != indentation:
                break
        if not _ordinary_comment(stripped, language):
            break
        if _explanatory_comment(stripped, language):
            found.add(index)
        index -= 1
    return found


def _ordinary_comment(line: str, language: str) -> bool:
    """Return whether a line is an ordinary, non-documentation comment.

    Args:
        line: Trimmed source line.
        language: Comment syntax identifier.

    Returns:
        `True` for an ordinary comment line.
    """

    if language == "rust":
        return line.startswith("//") and not line.startswith(("///", "//!"))
    if language == "typescript":
        return (
            (line.startswith("//") and not line.startswith("///"))
            or (line.startswith("/*") and not line.startswith("/**"))
            or line.startswith("*")
            or line.startswith("*/")
        )
    return line.startswith("#")


def _explanatory_comment(line: str, language: str) -> bool:
    """Return whether a comment contains removable prose.

    Args:
        line: Trimmed source line.
        language: Comment syntax identifier.

    Returns:
        `True` for removable explanatory prose.
    """

    if not _ordinary_comment(line, language):
        return False
    content = line
    if language == "python":
        content = line[1:].strip()
    elif line.startswith("//"):
        content = line[2:].strip()
    elif line.startswith("/*"):
        content = line[2:].strip(" */")
    elif line.startswith(("*", "*/")):
        content = line.lstrip("*/ ").strip()
    return not content.upper().startswith(_DIRECTIVES)
