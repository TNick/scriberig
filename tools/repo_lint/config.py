"""Define shared constants and declaration patterns for repository convention checks."""

import re

CODE_SUFFIXES = {".rs", ".ts", ".tsx", ".js", ".jsx", ".py"}
# Vendored dependencies retain upstream style and are audited through provenance.
IGNORED_PARTS = {
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "coverage",
    "third_party",
}
MAX_SOURCE_LINES = 400
# Native TUI trait composition cannot be split across multiple implementations.
# Its behavior is delegated to focused runner submodules; permit modest glue growth.
# Rust also requires the public Action enum to remain indivisible to preserve its
# Action::Variant API; keep it declarative and delegate behavior to sibling modules.
SOURCE_LINE_EXCEPTIONS = {
    "apps/scriberig/src/native_tui_agent/runner.rs": 420,
    "crates/scriberig-tui/src/action.rs": 700,
}
RUST_FUNCTION = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?fn\s+(\w+)\s*\("
)
RUST_TYPE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(struct|enum|trait)\s+(\w+)")
TS_FUNCTION = re.compile(
    r"^\s*(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+(\w+)\s*\("
)
TS_METHOD = re.compile(
    r"^\s*(?:(?:public|private|protected|static|async|readonly|override)\s+)+"
    r"(\w+)\s*\([^;]*\)\s*(?::\s*[^=\{]+)?\s*\{"
)
TS_CLASS = re.compile(r"^\s*(?:export\s+)?(?:default\s+)?class\s+(\w+)")
