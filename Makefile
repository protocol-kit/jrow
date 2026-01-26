.PHONY: help build test check fmt clippy clean run doc examples docker k8s asyncapi deploy

# Default target
.DEFAULT_GOAL := help

# Colors for output
BLUE := \033[0;34m
GREEN := \033[0;32m
YELLOW := \033[1;33m
NC := \033[0m # No Color

##@ General

help: ## Display this help
	@echo "$(BLUE)JROW - JSON-RPC over WebSocket$(NC)"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf "Usage:\n  make $(GREEN)<target>$(NC)\n"} /^[a-zA-Z_0-9-]+:.*?##/ { printf "  $(GREEN)%-20s$(NC) %s\n", $$1, $$2 } /^##@/ { printf "\n$(BLUE)%s$(NC)\n", substr($$0, 5) } ' $(MAKEFILE_LIST)

##@ Development

build: ## Build all crates
	@echo "$(BLUE)Building all crates...$(NC)"
	cargo build --all

build-release: ## Build all crates in release mode
	@echo "$(BLUE)Building all crates (release)...$(NC)"
	cargo build --all --release

test: ## Run all tests
	@echo "$(BLUE)Running tests...$(NC)"
	cargo test --all --lib

test-verbose: ## Run all tests with verbose output
	@echo "$(BLUE)Running tests (verbose)...$(NC)"
	cargo test --all --lib -- --nocapture

check: ## Check code without building
	@echo "$(BLUE)Checking code...$(NC)"
	cargo check --all

fmt: ## Format code with rustfmt
	@echo "$(BLUE)Formatting code...$(NC)"
	cargo fmt --all

fmt-check: ## Check code formatting
	@echo "$(BLUE)Checking code format...$(NC)"
	cargo fmt --all -- --check

clippy: ## Run clippy linter
	@echo "$(BLUE)Running clippy...$(NC)"
	cargo clippy --all -- -D warnings

clippy-fix: ## Auto-fix clippy warnings
	@echo "$(BLUE)Auto-fixing clippy warnings...$(NC)"
	cargo clippy --all --fix --allow-dirty

clean: ## Clean build artifacts
	@echo "$(BLUE)Cleaning build artifacts...$(NC)"
	cargo clean
	rm -rf docs/asyncapi

##@ Examples

examples: build ## Build all examples
	@echo "$(BLUE)Building examples...$(NC)"
	cargo build --examples

run-playground: ## Run server with embedded web UI
	@echo "$(BLUE)Starting JROW server with web UI...$(NC)"
	@echo "$(GREEN)Server will start on:$(NC)"
	@echo "  HTTP: http://127.0.0.1:8080"
	@echo "  WebSocket: ws://127.0.0.1:8081"
	@echo ""
	cargo run --example playground_server

##@ Documentation

doc: ## Generate and open Rust documentation
	@echo "$(BLUE)Generating documentation...$(NC)"
	cargo doc --no-deps --open

doc-all: ## Generate documentation for all dependencies
	@echo "$(BLUE)Generating documentation (with deps)...$(NC)"
	cargo doc --open

asyncapi-validate: ## Validate AsyncAPI specification
	@echo "$(BLUE)Validating AsyncAPI spec...$(NC)"
	@command -v asyncapi >/dev/null 2>&1 || { echo "$(YELLOW)asyncapi CLI not found. Install with: npm install -g @asyncapi/cli$(NC)"; exit 1; }
	asyncapi validate templates/asyncapi.yaml

asyncapi-html: ## Generate AsyncAPI HTML documentation
	@echo "$(BLUE)Generating AsyncAPI HTML docs...$(NC)"
	@command -v asyncapi >/dev/null 2>&1 || { echo "$(YELLOW)asyncapi CLI not found. Install with: npm install -g @asyncapi/cli$(NC)"; exit 1; }
	asyncapi generate fromTemplate templates/asyncapi.yaml @asyncapi/html-template -o docs/asyncapi
	@echo "$(GREEN)Documentation generated at: docs/asyncapi/index.html$(NC)"

asyncapi-md: ## Generate AsyncAPI Markdown documentation
	@echo "$(BLUE)Generating AsyncAPI Markdown docs...$(NC)"
	@command -v asyncapi >/dev/null 2>&1 || { echo "$(YELLOW)asyncapi CLI not found. Install with: npm install -g @asyncapi/cli$(NC)"; exit 1; }
	asyncapi generate fromTemplate templates/asyncapi.yaml @asyncapi/markdown-template -o docs/asyncapi.md
	@echo "$(GREEN)Documentation generated at: docs/asyncapi.md$(NC)"

##@ CI/CD

ci-check: fmt-check clippy test ## Run all CI checks
	@echo "$(GREEN)All CI checks passed!$(NC)"

ci-build: build-release ## Build for CI
	@echo "$(GREEN)CI build complete!$(NC)"

pre-commit: fmt clippy test ## Run pre-commit checks
	@echo "$(GREEN)Pre-commit checks passed!$(NC)"

##@ Utilities

watch: ## Watch for changes and rebuild
	@echo "$(BLUE)Watching for changes...$(NC)"
	@command -v cargo-watch >/dev/null 2>&1 || { echo "$(YELLOW)cargo-watch not found. Install with: cargo install cargo-watch$(NC)"; exit 1; }
	cargo watch -x check -x test

bench: ## Run benchmarks
	@echo "$(BLUE)Running benchmarks...$(NC)"
	cargo bench --all

outdated: ## Check for outdated dependencies
	@echo "$(BLUE)Checking for outdated dependencies...$(NC)"
	@command -v cargo-outdated >/dev/null 2>&1 || { echo "$(YELLOW)cargo-outdated not found. Install with: cargo install cargo-outdated$(NC)"; exit 1; }
	cargo outdated

update: ## Update dependencies
	@echo "$(BLUE)Updating dependencies...$(NC)"
	cargo update

tree: ## Show dependency tree
	@echo "$(BLUE)Showing dependency tree...$(NC)"
	cargo tree

audit: ## Audit dependencies for security vulnerabilities
	@echo "$(BLUE)Auditing dependencies...$(NC)"
	@command -v cargo-audit >/dev/null 2>&1 || { echo "$(YELLOW)cargo-audit not found. Install with: cargo install cargo-audit$(NC)"; exit 1; }
	cargo audit

##@ All-in-One

all: build test clippy ## Build, test, and lint everything
	@echo "$(GREEN)All tasks complete!$(NC)"

dev: ## Setup development environment
	@echo "$(BLUE)Setting up development environment...$(NC)"
	rustup component add rustfmt clippy
	@echo "$(GREEN)Development environment ready!$(NC)"

quick: fmt check ## Quick check (format and check)
	@echo "$(GREEN)Quick check complete!$(NC)"
