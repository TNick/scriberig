"""Runs the mandatory product-level golden workflow suites."""

from __future__ import annotations

import subprocess
import shutil
from pathlib import Path


def run(command: list[str], cwd: str | None = None) -> None:
    """Run one required product-golden command.

    Args:
        command: Process and arguments to execute.
        cwd: Optional working directory for the process.

    Raises:
        SystemExit: If the required command fails.

    Returns:
        None.
    """
    if command and command[0] == "cargo" and shutil.which("cargo") is None:
        rustup = shutil.which("rustup")
        if rustup is None:
            candidate = Path.home() / ".cargo" / "bin" / "rustup.exe"
            rustup = str(candidate) if candidate.exists() else None
        if rustup is None:
            raise SystemExit("cargo and rustup are unavailable")
        command = [rustup, "run", "stable", *command]
    result = subprocess.run(command, cwd=cwd, check=False)
    if result.returncode:
        raise SystemExit(result.returncode)


def main() -> None:
    """Run every mandatory golden suite without allowing skips.

    Returns:
        None.
    """
    run(
        [
            "pnpm",
            "exec",
            "playwright",
            "test",
            "e2e/specs/golden-project-workflow.spec.ts",
            "--config",
            "e2e/playwright.config.ts",
        ],
        cwd="web",
    )
    run(["cargo", "test", "-p", "scriberig", "--test", "tui_pty"])
    run(["cargo", "test", "-p", "scriberig", "--test", "tui_pty_golden_agent"])
    run(["cargo", "test", "-p", "scriberig", "--test", "tui_pty_self_hosting_php"])
    run(
        [
            "cargo",
            "test",
            "-p",
            "scriberig",
            "--test",
            "tui_pty_self_hosting_validation_failure",
        ]
    )
    run(["cargo", "test", "-p", "scriberig-project", "--test", "product_conformance"])
    run(["cargo", "test", "-p", "scriberig-server", "--test", "remote_project_conformance"])


if __name__ == "__main__":
    main()
