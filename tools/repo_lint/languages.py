"""Validate language-specific function, method, class, and type documentation conventions."""

from __future__ import annotations

import ast
import re

from .common import declaration_text
from .config import RUST_FUNCTION, RUST_TYPE, TS_CLASS, TS_FUNCTION, TS_METHOD
from .rust_parameters import rust_parameter_names


def rust_docs_before(lines: list[str], declaration_index: int) -> str:
    """Collect Rust `///` documentation immediately preceding a declaration.

    Args:
        lines: Complete Rust source lines.
        declaration_index: Zero-based declaration line.

    Returns:
        Normalized documentation text without comment prefixes.
    """

    # Collect Rust documentation comments attached to a declaration.
    docs: list[str] = []
    index = declaration_index - 1
    while index >= 0:
        stripped = lines[index].strip()
        if not stripped or stripped.startswith("#[") or (
            stripped.startswith("// ") and not stripped.startswith("///")
        ):
            index -= 1
            continue
        if stripped.startswith("///"):
            docs.append(stripped[3:].strip())
            index -= 1
            continue
        break
    return "\n".join(reversed(docs))


def check_rust(text: str) -> list[str]:
    """Validate Rust declaration documentation and logical block boundaries.

    Args:
        text: Rust source text.

    Returns:
        Human-readable Rust convention violations.
    """

    # Validate Rust functions, methods, and class-like type documentation.
    issues: list[str] = []
    lines = text.splitlines()
    for index, line in enumerate(lines):
        function_match = RUST_FUNCTION.match(line)
        type_match = RUST_TYPE.match(line)
        if function_match:
            _check_rust_function(lines, index, function_match.group(1), issues)
        elif type_match:
            name = type_match.group(2)
            if not rust_docs_before(lines, index):
                issues.append(f"L{index + 1} {name}: DOC: class-like Rust types require documentation.")
    return issues


def _check_rust_function(lines: list[str], index: int, name: str, issues: list[str]) -> None:
    """Validate one Rust function or method.

    Args:
        lines: Complete Rust source lines.
        index: Zero-based declaration line.
        name: Function or method name.
        issues: Mutable issue collection to append violations to.

    Returns:
        `None`; violations are appended to `issues`.
    """

    # Validate one Rust function declaration and its attached documentation.
    docs = rust_docs_before(lines, index)
    if not docs:
        issues.append(f"L{index + 1} {name}: DOC: every function/method requires `///` documentation.")
        return
    declaration = declaration_text(lines, index)
    for parameter_name in rust_parameter_names(declaration):
        if f"`{parameter_name}`" not in docs:
            issues.append(f"L{index + 1} {name}: PARAM_DOC: document `{parameter_name}`.")
    return_match = re.search(r"->\s*([^\{]+)", declaration)
    if return_match:
        return_type = return_match.group(1).strip()
        if return_type not in {"()", "!"} and "# Returns" not in docs:
            issues.append(f"L{index + 1} {name}: RETURN_DOC: non-unit return requires `# Returns`.")
        if "Result" in return_type and "# Errors" not in docs:
            issues.append(f"L{index + 1} {name}: ERROR_DOC: `Result` return requires `# Errors`.")


def jsdoc_before(lines: list[str], declaration_index: int) -> str:
    """Collect the JSDoc block immediately preceding a declaration.

    Args:
        lines: Complete TypeScript/JavaScript source lines.
        declaration_index: Zero-based declaration line.

    Returns:
        Normalized JSDoc text, or an empty string when absent.
    """

    # Collect a TypeScript or JavaScript JSDoc block attached to a declaration.
    index = declaration_index - 1
    if index >= 0 and lines[index].strip().startswith("// "):
        index -= 1
    if index < 0 or "*/" not in lines[index]:
        return ""
    docs: list[str] = []
    while index >= 0:
        docs.append(lines[index].strip())
        if "/**" in lines[index]:
            return "\n".join(reversed(docs))
        index -= 1
    return ""


def check_typescript(text: str) -> list[str]:
    """Validate JavaScript-family documentation and logical block boundaries.

    Args:
        text: TypeScript or JavaScript source text.

    Returns:
        Human-readable JavaScript-family convention violations.
    """

    # Validate TypeScript and JavaScript functions, methods, and class documentation.
    issues: list[str] = []
    lines = text.splitlines()
    controls = {"if", "for", "while", "switch", "catch", "with"}
    for index, line in enumerate(lines):
        match = TS_FUNCTION.match(line) or TS_METHOD.match(line)
        class_match = TS_CLASS.match(line)
        if match and match.group(1) not in controls:
            _check_typescript_function(lines, index, match.group(1), issues)
        elif class_match:
            _check_typescript_class(lines, index, class_match.group(1), issues)
    return issues


def _check_typescript_function(lines: list[str], index: int, name: str, issues: list[str]) -> None:
    """Validate one TypeScript or JavaScript function/method.

    Args:
        lines: Complete source lines.
        index: Zero-based declaration line.
        name: Function or method name.
        issues: Mutable issue collection to append violations to.

    Returns:
        `None`; violations are appended to `issues`.
    """

    # Validate one JavaScript-family function or method documentation block.
    docs = jsdoc_before(lines, index)
    if not docs:
        issues.append(f"L{index + 1} {name}: DOC: every function/method requires JSDoc.")
        return
    declaration = declaration_text(lines, index)
    parameters_match = re.search(r"\((.*?)\)", declaration)
    if parameters_match:
        for raw in parameters_match.group(1).split(","):
            parameter = raw.strip()
            if not parameter:
                continue
            parameter_name = re.sub(r"[?].*$", "", parameter.split(":", 1)[0].strip())
            if re.fullmatch(r"[A-Za-z_$][A-Za-z0-9_$]*", parameter_name):
                if f"@param {parameter_name}" not in docs:
                    issues.append(f"L{index + 1} {name}: PARAM_DOC: document `{parameter_name}` with `@param`.")
    return_match = re.search(r"\)\s*:\s*([^=\{]+)", declaration)
    if return_match and return_match.group(1).strip() not in {"void", "undefined", "never"}:
        if "@returns" not in docs:
            issues.append(f"L{index + 1} {name}: RETURN_DOC: meaningful return requires `@returns`.")
    body = "\n".join(lines[index : index + 80])
    if re.search(r"\bthrow\b", body) and "@throws" not in docs:
        issues.append(f"L{index + 1} {name}: THROW_DOC: throwing code requires `@throws`.")


def _check_typescript_class(lines: list[str], index: int, name: str, issues: list[str]) -> None:
    """Validate one TypeScript or JavaScript class documentation block.

    Args:
        lines: Complete source lines.
        index: Zero-based class declaration line.
        name: Class name.
        issues: Mutable issue collection to append violations to.

    Returns:
        `None`; violations are appended to `issues`.
    """

    # Validate class documentation sections that inventory public and private members.
    docs = jsdoc_before(lines, index)
    if not docs:
        issues.append(f"L{index + 1} {name}: DOC: every class requires JSDoc.")
    elif "Public members" not in docs or "Private members" not in docs:
        issues.append(
            f"L{index + 1} {name}: MEMBER_DOC: class JSDoc needs `Public members` and `Private members` sections."
        )


def check_python(text: str) -> list[str]:
    """Validate Python function and class documentation conventions.

    Args:
        text: Python source text.

    Returns:
        Human-readable Python convention violations.
    """

    # Validate Python declarations using the language AST for reliable function boundaries.
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return []
    issues: list[str] = []
    lines = text.splitlines()
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            _check_python_function(lines, node, issues)
        elif isinstance(node, ast.ClassDef):
            _check_python_class(lines, node, issues)
    return issues


def _check_python_function(
    lines: list[str], node: ast.FunctionDef | ast.AsyncFunctionDef, issues: list[str]
) -> None:
    """Validate one Python function or method.

    Args:
        lines: Complete Python source lines.
        node: Parsed function definition.
        issues: Mutable issue collection to append violations to.

    Returns:
        `None`; violations are appended to `issues`.
    """

    # Validate one Python function or method documentation block.
    docs = ast.get_docstring(node, clean=False) or ""
    if not docs:
        issues.append(f"L{node.lineno} {node.name}: DOC: every function/method requires a docstring.")
        return
    arguments = [*node.args.posonlyargs, *node.args.args, *node.args.kwonlyargs]
    for argument in arguments:
        if argument.arg in {"self", "cls"}:
            continue
        if argument.arg not in docs:
            issues.append(f"L{node.lineno} {node.name}: PARAM_DOC: document `{argument.arg}`.")
    if node.returns and "Returns:" not in docs:
        issues.append(f"L{node.lineno} {node.name}: RETURN_DOC: annotated return requires `Returns:`.")
    if any(isinstance(child, ast.Raise) for child in ast.walk(node)) and "Raises:" not in docs:
        issues.append(f"L{node.lineno} {node.name}: ERROR_DOC: raising code requires `Raises:`.")


def _check_python_class(lines: list[str], node: ast.ClassDef, issues: list[str]) -> None:
    """Validate one Python class documentation block.

    Args:
        lines: Complete Python source lines.
        node: Parsed class definition.
        issues: Mutable issue collection to append violations to.

    Returns:
        `None`; violations are appended to `issues`.
    """

    # Validate one Python class and its public/private member documentation sections.
    docs = ast.get_docstring(node, clean=False) or ""
    if not docs:
        issues.append(f"L{node.lineno} {node.name}: DOC: every class requires a docstring.")
    elif "Public members:" not in docs or "Private members:" not in docs:
        issues.append(
            f"L{node.lineno} {node.name}: MEMBER_DOC: class docstring needs public/private member sections."
        )
