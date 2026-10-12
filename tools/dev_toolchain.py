"""Resolves a usable Rust toolchain for development commands."""

import os
import shutil
import subprocess
from pathlib import Path


def cargo_command() -> str:
    """Return a usable Cargo executable for the local Rust toolchain.

    Returns:
        Path or command name that can launch Cargo.
    """
    cargo = shutil.which("cargo")
    if cargo and Path(cargo).is_file() and Path(cargo).stat().st_size > 0:
        return cargo
    rustup = shutil.which("rustup")
    if rustup is None and os.name == "nt":
        candidate = Path.home() / ".cargo" / "bin" / "rustup.exe"
        if candidate.is_file():
            rustup = str(candidate)
    if rustup:
        try:
            result = subprocess.run(
                [rustup, "which", "cargo"],
                capture_output=True,
                check=True,
                text=True,
                timeout=5,
            )
        except (OSError, subprocess.SubprocessError):
            pass
        else:
            resolved = result.stdout.strip()
            if resolved and Path(resolved).is_file():
                return resolved
    return "cargo"
