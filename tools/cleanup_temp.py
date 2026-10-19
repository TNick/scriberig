"""Remove stale Scriberig test artifacts from the system temp directory."""

from __future__ import annotations

import argparse
import os
import shutil
import tempfile
import time
from pathlib import Path


def workspace_state(root: Path) -> tuple[float, int] | None:
    """Return the newest change time and byte size beneath a safe workspace.

    Args:
        root: Scriberig temporary directory to inspect.

    Returns:
        The newest modification time and total file size, or ``None`` when
        inspection encounters a symlink or filesystem error.
    """
    newest = root.stat(follow_symlinks=False).st_mtime
    total_bytes = 0

    for directory, child_directories, files in os.walk(root, followlinks=False):
        current = Path(directory)
        for name in [*child_directories, *files]:
            path = current / name
            try:
                metadata = path.stat(follow_symlinks=False)
            except OSError:
                return None
            newest = max(newest, metadata.st_mtime)
            if not path.is_symlink() and path.is_file():
                total_bytes += metadata.st_size

    return newest, total_bytes


def cleanup_temp_artifacts(*, dry_run: bool = False) -> tuple[int, int, int]:
    """Delete Scriberig temp artifacts unchanged for at least one hour.

    Args:
        dry_run: Report eligible paths without deleting them.

    Returns:
        Counts of removed or eligible, retained, and skipped paths.
    """
    temp_root = Path(tempfile.gettempdir()).resolve()
    cutoff = time.time() - 60 * 60
    removed = 0
    retained = 0
    skipped = 0
    reclaimed_bytes = 0

    for root in temp_root.glob("scriberig-*"):
        if root.is_symlink():
            skipped += 1
            continue
        if root.is_dir():
            state = workspace_state(root)
        else:
            try:
                metadata = root.stat(follow_symlinks=False)
                state = metadata.st_mtime, metadata.st_size
            except OSError:
                state = None
        if state is None:
            skipped += 1
            continue
        newest, size = state
        if newest >= cutoff:
            retained += 1
            continue
        if dry_run:
            print(f"Would remove {root}")
        else:
            try:
                if root.is_dir():
                    shutil.rmtree(root)
                else:
                    root.unlink()
            except OSError as error:
                print(f"Could not remove {root}: {error}")
                skipped += 1
                continue
        removed += 1
        reclaimed_bytes += size

    reclaimed_gigabytes = reclaimed_bytes / (1024**3)
    action = "eligible" if dry_run else "removed"
    print(
        f"{action}: {removed}; retained (<1h): {retained}; "
        f"skipped: {skipped}; size: {reclaimed_gigabytes:.2f} GiB"
    )
    return removed, retained, skipped


def main() -> None:
    """Parse options and clean stale Scriberig temporary artifacts.

    Returns:
        None.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="show eligible paths without deleting them",
    )
    arguments = parser.parse_args()
    cleanup_temp_artifacts(dry_run=arguments.dry_run)


if __name__ == "__main__":
    main()
