"""Coordinate repository convention checks across supported source languages."""

from __future__ import annotations

import sys
from pathlib import Path

from .common import autofix_text, check_file_description, collect_paths, effective_line_count, parse_args
from .config import MAX_SOURCE_LINES
from .languages import check_python, check_rust, check_typescript


# Block: Run all applicable convention checks for one source file.
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
    text = path.read_text(encoding="utf-8")
    if fix:
        fixed = autofix_text(text)
        if fixed != text:
            path.write_text(fixed, encoding="utf-8")
            text = fixed

    issues = check_file_description(path, text)
    count = effective_line_count(path, text)
    if count > MAX_SOURCE_LINES:
        issues.append(
            f"FILE_SIZE: {count} implementation lines exceed the {MAX_SOURCE_LINES}-line limit; split the file."
        )
    if path.suffix == ".rs":
        issues.extend(check_rust(text))
    elif path.suffix in {".ts", ".tsx", ".js", ".jsx"}:
        issues.extend(check_typescript(text))
    elif path.suffix == ".py":
        issues.extend(check_python(text))
    return issues


# Block: Execute repository checks and report all violations in one pass.
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
