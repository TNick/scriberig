"""Extract Rust parameters without confusing visibility or nested tuple types."""

from __future__ import annotations

import re


def rust_parameter_names(declaration: str) -> list[str]:
    """Extract top-level function parameter names from a Rust declaration.

    Args:
        declaration: Flattened Rust declaration text.

    Returns:
        Actual parameter names excluding receivers and visibility qualifiers.
    """
    match = re.search(r"\bfn\s+\w+\s*\(", declaration)
    if not match:
        return []

    # Match nested type delimiters before treating commas as separators.
    depths = {"(": 1, "[": 0, "<": 0, "{": 0}
    closing = {")": "(", "]": "[", ">": "<", "}": "{"}
    parameters: list[str] = []
    start = match.end()
    for index in range(start, len(declaration)):
        character = declaration[index]
        if character in depths:
            depths[character] += 1
        elif character in closing:
            opener = closing[character]
            depths[opener] = max(0, depths[opener] - 1)
            if character == ")" and depths["("] == 0:
                parameters.append(declaration[start:index])
                break
        elif character == "," and depths == {"(": 1, "[": 0, "<": 0, "{": 0}:
            parameters.append(declaration[start:index])
            start = index + 1

    # Parameter patterns with ordinary identifiers require declaration docs.
    names: list[str] = []
    for parameter in parameters:
        name = parameter.strip().split(":", 1)[0].strip()
        name = name.removeprefix("mut ").strip()
        if name in {"self", "&self", "&mut self", "_"}:
            continue
        if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", name):
            names.append(name)
    return names
