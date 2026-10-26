"""Coordinate repository convention checks across supported source languages."""

from __future__ import annotations

import sys
from pathlib import Path

from .common import (
    autofix_text,
    check_file_description,
    collect_paths,
    effective_line_count,
    parse_args,
)
from .config import MAX_SOURCE_LINES, SOURCE_LINE_EXCEPTIONS
from .declaration_comments import redundant_comment_lines
from .languages import check_python, check_rust, check_typescript


def check_file(path: Path, fix: bool) -> list[str]:
    """Check one source file and optionally apply safe fixes first.

    Args:
        path: Source file to inspect.
        fix: Whether safe mechanical fixes should be written.

    Returns:
        Human-readable violations prefixed later with the path.

    Raises:
        OSError: If the source file cannot be read or rewritten.
    """

    # Run all applicable convention checks for one source file.
    original = path.read_bytes()
    text = original.decode("utf-8")
    if fix:
        fixed = autofix_text(text)
        comment_lines = redundant_comment_lines(path.suffix, fixed)
        if comment_lines:
            fixed = "\n".join(
                line
                for index, line in enumerate(fixed.splitlines())
                if index not in comment_lines
            ) + "\n"
        if fixed != text or b"\r\n" in original:
            with path.open("w", encoding="utf-8", newline="") as source:
                source.write(fixed)
            text = fixed

    issues = check_file_description(path, text)
    count = effective_line_count(path, text)
    exception_limit = SOURCE_LINE_EXCEPTIONS.get(path.as_posix(), MAX_SOURCE_LINES)
    if count > exception_limit:
        issues.append(
            f"FILE_SIZE: {count} implementation lines exceed the "
            f"{exception_limit}-line limit; split the file."
        )
    if path.suffix == ".rs":
        issues.extend(check_rust(text))
    elif path.suffix in {".ts", ".tsx", ".js", ".jsx"}:
        issues.extend(check_typescript(text))
    elif path.suffix == ".py":
        issues.extend(check_python(text))
    issues.extend(
        f"L{index + 1} DECL_COMMENT: remove explanatory comments adjacent to "
        "declaration documentation."
        for index in sorted(redundant_comment_lines(path.suffix, text))
    )
    return issues


def main(argv: list[str] | None = None) -> int:
    """Run convention checks for selected files or the repository.

    Args:
        argv: Optional command-line arguments; defaults to process arguments.

    Returns:
        Zero when all files conform, otherwise one.

    Raises:
        RuntimeError: If repository file discovery fails.
        OSError: If a source file cannot be read or safely fixed.
    """

    # Execute repository checks and report all violations in one pass.
    args = parse_args(sys.argv[1:] if argv is None else argv)
    failures: list[str] = []
    for path in collect_paths(args.paths):
        for issue in check_file(path, args.fix):
            failures.append(f"{path}: {issue}")
    if failures:
        print("Scriberig repository convention violations:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1
    print("Scriberig repository conventions: OK")
    return 0
