"""Run the split Scriberig repository convention linter."""

from repo_lint import main


# Block: Delegate process exit status to the repository linter package.
if __name__ == "__main__":
    raise SystemExit(main())
