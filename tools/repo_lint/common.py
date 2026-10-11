"""Provide language-neutral discovery, safe fixes, sizing, and declaration helpers."""

from __future__ import annotations

import argparse
import ast
import subprocess
from pathlib import Path

from .config import CODE_SUFFIXES, IGNORED_PARTS


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse command-line arguments.

    Args:
        argv: Command-line arguments excluding the executable name.

    Returns:
        Parsed command-line options.
    """

    # Parse command-line options for repository lint and safe auto-fix modes.
    parser = argparse.ArgumentParser(description="Enforce Scriberig source conventions.")
    parser.add_argument("--fix", action="store_true", help="Apply conservative safe fixes.")
    parser.add_argument("paths", nargs="*", help="Optional files to check.")
    return parser.parse_args(argv)


def collect_paths(raw_paths: list[str]) -> list[Path]:
    """Collect code files eligible for repository checks.

    Args:
        raw_paths: Explicit paths supplied by a caller such as pre-commit.

    Returns:
        Sorted source paths that exist and are not ignored.

    Raises:
        RuntimeError: If repository file discovery fails.
    """

    # Discover source files while respecting generated and dependency directories.
    if raw_paths:
        candidates = [Path(value) for value in raw_paths]
    else:
        result = subprocess.run(
            ["git", "ls-files", "-co", "--exclude-standard"],
            check=False,
            capture_output=True,
            text=True,
        )
        if result.returncode != 0:
            raise RuntimeError(result.stderr.strip() or "git ls-files failed")
        candidates = [Path(value) for value in result.stdout.splitlines()]

    return sorted(
        path
        for path in candidates
        if path.is_file()
        and path.suffix in CODE_SUFFIXES
        and not any(part in IGNORED_PARTS for part in path.parts)
    )


def autofix_text(text: str) -> str:
    """Apply conservative whitespace and block-separation fixes.

    Args:
        text: Original source text.

    Returns:
        Safely normalized source text.
    """
    return "\n".join(line.rstrip() for line in text.splitlines()).rstrip() + "\n"


def effective_line_count(path: Path, text: str) -> int:
    """Count source lines after the leading documentation/import preamble.

    Args:
        path: Source file path used to select language rules.
        text: Source text to measure.

    Returns:
        Number of lines subject to the source-file size limit.
    """

    # Measure implementation size while excluding language-specific leading preambles.
    lines = text.splitlines()
    if path.suffix == ".py":
        return _python_effective_line_count(text, lines)

    in_file_doc = False
    end = 0
    for index, line in enumerate(lines, start=1):
        stripped = line.strip()
        if in_file_doc:
            end = index
            if "*/" in stripped:
                in_file_doc = False
            continue
        if not stripped:
            end = index
            continue
        if path.suffix == ".rs" and (
            stripped.startswith("//!")
            or stripped.startswith("#![")
            or stripped.startswith("use ")
            or stripped.startswith("extern crate ")
        ):
            end = index
            continue
        if path.suffix in {".ts", ".tsx", ".js", ".jsx"}:
            if stripped.startswith("/**") and index <= 3:
                end = index
                in_file_doc = "*/" not in stripped
                continue
            if stripped.startswith("import "):
                end = index
                continue
        break
    return max(0, len(lines) - end)


def _python_effective_line_count(text: str, lines: list[str]) -> int:
    """Count Python lines after the leading module documentation/import preamble.

    Args:
        text: Python source text to parse.
        lines: Physical lines for final size calculation.

    Returns:
        Number of Python lines subject to the size limit.
    """

    # Compute Python implementation size from the module docstring and imports.
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return len(lines)
    end = 0
    for node in tree.body:
        is_doc = (
            isinstance(node, ast.Expr)
            and isinstance(node.value, ast.Constant)
            and isinstance(node.value.value, str)
        )
        if is_doc or isinstance(node, (ast.Import, ast.ImportFrom)):
            end = max(end, getattr(node, "end_lineno", node.lineno))
            continue
        break
    return max(0, len(lines) - end)


def check_file_description(path: Path, text: str) -> list[str]:
    """Validate the required source-file description documentation.

    Args:
        path: Source path used to select documentation syntax.
        text: Source text to inspect.

    Returns:
        Human-readable violations for the file-description rule.
    """

    # Verify every supported source language starts with a file-purpose docstring.
    stripped = text.lstrip("\ufeff\n\r ")
    if path.suffix == ".rs" and not stripped.startswith("//!"):
        return ["FILE_DOC: Rust source must begin with `//!` file/module documentation."]
    if path.suffix in {".ts", ".tsx", ".js", ".jsx"}:
        first = "\n".join(stripped.splitlines()[:8])
        if not stripped.startswith("/**") or "@file" not in first:
            return ["FILE_DOC: TypeScript/JavaScript source must begin with a JSDoc `@file` description."]
    if path.suffix == ".py":
        try:
            if not ast.get_docstring(ast.parse(text), clean=False):
                return ["FILE_DOC: Python source must begin with a module docstring."]
        except SyntaxError:
            return []
    return []


def declaration_text(lines: list[str], start: int, limit: int = 20) -> str:
    """Join a declaration header across a bounded number of lines.

    Args:
        lines: Complete source lines.
        start: Zero-based declaration start line.
        limit: Maximum number of physical lines to inspect.

    Returns:
        Joined declaration text ending near the opening body delimiter.
    """

    # Read a declaration that may span several physical source lines.
    parts: list[str] = []
    for line in lines[start : start + limit]:
        parts.append(line.strip())
        joined = " ".join(parts)
        if "{" in joined or joined.rstrip().endswith(";") or "=>" in joined:
            break
    return " ".join(parts)


def explanatory_comment_error(lines: list[str], declaration_index: int, marker: str) -> str | None:
    """Require an explanatory comment immediately after declaration docs.

    Args:
        lines: Complete source lines.
        declaration_index: Zero-based declaration line.
        marker: Language-specific comment prefix.

    Returns:
        A violation message, or `None` when the marker is valid.
    """

    # Locate the explanatory comment after decorators or Rust attributes.
    index = declaration_index - 1
    while index >= 0 and not lines[index].strip():
        index -= 1
    while index >= 0 and lines[index].strip().startswith("#["):
        index -= 1
    if index < 0 or not lines[index].strip().startswith(marker):
        return "COMMENT_MISSING: add an explanatory comment after declaration docs."

    # Require the explanatory comment to follow the docstring directly.
    previous = index - 1
    if marker == "// " and previous >= 0 and lines[previous].strip().startswith("///"):
        return None
    if marker == "// " and previous >= 0 and lines[previous].strip().endswith("*/"):
        return None
    return "COMMENT_ORDER: place the explanatory comment after declaration docs."
