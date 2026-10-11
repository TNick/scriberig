"""Verify the dependency direction of the persistence crate slice."""

from pathlib import Path
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent

ALLOWED = {
    "scriberig-core": set(),
    "scriberig-store": {"scriberig-core"},
    "scriberig-store-memory": {"scriberig-core", "scriberig-store"},
    "scriberig-store-sqlite": {"scriberig-core", "scriberig-store"},
}


def package_dependencies(manifest: Path) -> set[str]:
    """Return local package dependencies declared by one manifest.

    Args:
        manifest: Cargo manifest to inspect.

    Returns:
        Names of dependencies declared with a local path.
    """
    document = tomllib.loads(manifest.read_text(encoding="utf-8"))
    dependencies = document.get("dependencies", {})
    result: set[str] = set()
    for name, declaration in dependencies.items():
        if isinstance(declaration, dict) and "path" in declaration:
            result.add(name)
    return result


def main() -> int:
    """Check persistence manifests and return a process status.

    Returns:
        Zero when the dependency direction is valid, otherwise one.
    """
    failures: list[str] = []
    for package, allowed in ALLOWED.items():
        manifest = ROOT / "crates" / package / "Cargo.toml"
        if not manifest.is_file():
            failures.append(f"missing manifest: {manifest}")
            continue
        actual = package_dependencies(manifest)
        unexpected = sorted(actual - allowed)
        if unexpected:
            failures.append(f"{package} has forbidden local dependencies: {unexpected}")
    if failures:
        print("\n".join(failures), file=sys.stderr)
        return 1
    print("persistence crate dependency direction: ok")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
