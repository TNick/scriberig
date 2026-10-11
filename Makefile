.DEFAULT_GOAL := help
.PHONY: help init build install docs format lint test check dev dev-stop

ifeq ($(OS),Windows_NT)
PYTHON ?= python
else
PYTHON ?= python3
endif

# List the available developer commands.
help:
	@echo Scriberig developer commands:
	@echo   make init       Install web and pre-commit dependencies.
	@echo   make build      Build all workspace targets.
	@echo   make install    Build and install the srig executable.
	@echo   make docs       Build Rust and TypeScript API documentation.
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

# Build every Rust workspace target.
build:
	cargo build --workspace --all-targets

# Build and install the srig executable into Cargo's bin directory.
install:
	cargo install --path apps/scriberig --locked --force

# Build API documentation from Rust and TypeScript declaration docs.
docs:
	cargo doc --workspace --no-deps
	pnpm --dir web docs

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
