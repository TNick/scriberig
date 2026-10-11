.DEFAULT_GOAL := help
.PHONY: help init format lint test check dev dev-stop

ifeq ($(OS),Windows_NT)
PYTHON ?= python
else
PYTHON ?= python3
endif

# List the available developer commands.
help:
	@echo Scriberig developer commands:
	@echo   make init       Install web and pre-commit dependencies.
	@echo   make format     Apply repository, Rust, and web formatting.
	@echo   make lint       Run repository and web formatting checks.
	@echo   make test       Run Rust and web test suites.
	@echo   make check      Run the full quality gate.
	@echo   make dev        Idempotently start API and web app, then open browser.
	@echo   make dev-stop   Stop processes started by make dev.

# Install the workspace's web and pre-commit dependencies.
init:
	corepack enable
	pnpm --dir web install
	$(PYTHON) -m pip install pre-commit
	pre-commit install
	pre-commit install-hooks

# Apply deterministic source and language-specific formatting.
format:
	$(PYTHON) tools/lint_repo.py --fix
	cargo fmt --all
	pnpm --dir web format

# Check repository conventions and web formatting.
lint:
	$(PYTHON) tools/lint_repo.py
	pnpm --dir web format:check

# Run the Rust and web test suites.
test:
	cargo test --workspace
	pnpm --dir web test

# Run the complete non-mutating quality gate.
check:
	$(PYTHON) tools/lint_repo.py
	cargo fmt --all -- --check
	cargo check --workspace --all-targets
	cargo clippy --workspace --all-targets -- -D warnings
	pnpm --dir web format:check
	pnpm --dir web typecheck

# Start or reuse the API and web app, then open the browser.
dev:
	$(PYTHON) tools/dev.py start

# Stop the services managed by make dev.
dev-stop:
	$(PYTHON) tools/dev.py stop
