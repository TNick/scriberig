"""Replace the Bash web-format hook with a Windows and Linux Python hook."""

from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
WEB = ROOT / "web"
PRETTIER = WEB / "node_modules" / "prettier" / "bin" / "prettier.cjs"


def staged_web_paths(arguments: list[str]) -> tuple[list[str], list[str]]:
    """Select staged TypeScript and JavaScript paths under the web workspace.

    Args:
        arguments: Paths supplied by pre-commit.

    Returns:
        Repository-relative and web-relative paths to format.
    """
    # Normalize pre-commit paths so matching is independent of the host OS.
    repo_paths = [
        path.replace("\\", "/")
        for path in arguments
        if path.replace("\\", "/").startswith("web/")
        and Path(path).suffix in {".ts", ".tsx", ".js", ".jsx"}
    ]
    web_paths = [path.removeprefix("web/") for path in repo_paths]
    return repo_paths, web_paths


def main(arguments: list[str]) -> int:
    """Format staged web source files and stage their formatted contents.

    Args:
        arguments: Paths supplied by pre-commit.

    Returns:
        Process exit status.
    """
    # Leave the hook unchanged when no web source files are staged.
    repo_paths, web_paths = staged_web_paths(arguments)
    if not repo_paths:
        return 0
    if not PRETTIER.is_file():
        print("Prettier is missing; run pnpm --dir web install.", file=sys.stderr)
        return 1

    # Call Prettier through Node, then stage only the files it formatted.
    subprocess.run(
        ["node", str(PRETTIER), "--write", *web_paths],
        cwd=WEB,
        check=True,
    )
    subprocess.run(["git", "add", "--", *repo_paths], cwd=ROOT, check=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
