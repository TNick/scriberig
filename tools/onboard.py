"""Onboard the current project into a Scriberig SQLite database."""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]


def cargo_command() -> str:
    """Resolve the Cargo executable used to run Scriberig.

    Returns:
        The path or command name for Cargo.
    """

    if os.name == "nt":
        user_cargo = Path.home() / ".cargo" / "bin" / "cargo.exe"
        if user_cargo.is_file():
            return str(user_cargo)

    return shutil.which("cargo") or "cargo"


def run_srig(arguments: list[str]) -> dict[str, Any]:
    """Run an srig project command and decode its JSON response.

    Args:
        arguments: Arguments following the `srig` executable.

    Returns:
        The decoded JSON response from the command.

    Raises:
        RuntimeError: If the command fails or does not return valid JSON.
    """

    command = [
        cargo_command(),
        "run",
        "--package",
        "scriberig",
        "--bin",
        "srig",
        "--",
        *arguments,
    ]
    result = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode != 0:
        details = result.stderr.strip() or result.stdout.strip()
        raise RuntimeError(
            "srig command failed with exit code "
            f"{result.returncode}: {details}"
        )

    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(
            "srig command returned invalid JSON: " f"{result.stdout.strip()}"
        ) from error


def parse_arguments() -> argparse.Namespace:
    """Parse command-line options for the onboarding helper.

    Returns:
        Parsed command-line options.
    """

    parser = argparse.ArgumentParser(
        description="Import Git-tracked files from a project into Scriberig."
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="Project root to onboard (default: repository root).",
    )
    parser.add_argument(
        "--db",
        type=Path,
        default=ROOT / "playground" / "dev-app-runtime" / "scriberig.sqlite",
        help="SQLite database to use for the project (supports multiple projects).",
    )
    return parser.parse_args()


def main() -> int:
    """Scan, import, and report the current project.

    Returns:
        Zero on success, otherwise one.
    """

    options = parse_arguments()
    project_root = options.root.resolve()
    database = options.db.resolve()
    scan = run_srig(
        [
            "project",
            "scan",
            str(project_root),
            "--name",
            project_root.name,
            "--git-tracked",
            "--db",
            str(database),
            "--json",
        ]
    )
    project_id = scan["project"]["id"]
    imported = run_srig(
        [
            "project",
            "import",
            project_id,
            "--db",
            str(database),
            "--json",
        ]
    )
    status = run_srig(
        [
            "project",
            "status",
            project_id,
            "--db",
            str(database),
            "--json",
        ]
    )

    print(f"Onboarded {project_root} as {project_id}")
    print(json.dumps({"import": imported, "status": status}, indent=2))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (KeyError, RuntimeError) as error:
        print(f"onboard: {error}", file=sys.stderr)
        sys.exit(1)
