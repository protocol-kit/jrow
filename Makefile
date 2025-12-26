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

run-simple: ## Run simple_server example
	@echo "$(BLUE)Running simple_server...$(NC)"
	cargo run --example simple_server

run-client: ## Run simple_client example
	@echo "$(BLUE)Running simple_client...$(NC)"
	cargo run --example simple_client

run-bidirectional: ## Run bidirectional example
	@echo "$(BLUE)Running bidirectional...$(NC)"
	cargo run --example bidirectional

run-pubsub: ## Run pubsub example
	@echo "$(BLUE)Running pubsub...$(NC)"
	cargo run --example pubsub

run-batch: ## Run batch example
	@echo "$(BLUE)Running batch...$(NC)"
	cargo run --example batch

run-pubsub-batch: ## Run pubsub_batch example
	@echo "$(BLUE)Running pubsub_batch...$(NC)"
	cargo run --example pubsub_batch

run-publish-batch: ## Run publish_batch example
	@echo "$(BLUE)Running publish_batch...$(NC)"
	cargo run --example publish_batch

run-server-ui: ## Run server with embedded web UI
	@echo "$(BLUE)Starting JROW server with web UI...$(NC)"
	@echo "$(GREEN)Server will start on:$(NC)"
	@echo "  HTTP: http://127.0.0.1:8080"
	@echo "  WebSocket: ws://127.0.0.1:8081"
	@echo ""
	cargo run --example server_with_ui

##@ Web UI

run-web-ui: ## Start web UI client (requires Python 3)
	@echo "$(BLUE)Starting JROW Web UI...$(NC)"
	@echo "$(GREEN)Open http://localhost:8000 in your browser$(NC)"
	@echo "$(YELLOW)Press Ctrl+C to stop$(NC)"
	@cd web-ui && python3 -m http.server 8000

run-web-ui-node: ## Start web UI client (requires Node.js)
	@echo "$(BLUE)Starting JROW Web UI with npx serve...$(NC)"
	@echo "$(GREEN)Server will open automatically$(NC)"
	@cd web-ui && npx serve

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

##@ Docker

docker-build: ## Build Docker image
	@echo "$(BLUE)Building Docker image...$(NC)"
	docker build -t jrow:latest -f templates/deploy/docker/Dockerfile .

docker-build-dev: ## Build development Docker image
	@echo "$(BLUE)Building development Docker image...$(NC)"
	docker build -t jrow:dev -f templates/deploy/docker/Dockerfile.dev .

docker-run: docker-build ## Run Docker container
	@echo "$(BLUE)Running Docker container...$(NC)"
	docker run -p 8080:8080 jrow:latest

docker-compose-up: ## Start services with docker-compose
	@echo "$(BLUE)Starting docker-compose services...$(NC)"
	cd templates/deploy/docker && docker-compose up -d

docker-compose-down: ## Stop services with docker-compose
	@echo "$(BLUE)Stopping docker-compose services...$(NC)"
	cd templates/deploy/docker && docker-compose down

docker-compose-logs: ## View docker-compose logs
	@echo "$(BLUE)Viewing docker-compose logs...$(NC)"
	cd templates/deploy/docker && docker-compose logs -f

docker-clean: ## Remove Docker images
	@echo "$(BLUE)Removing Docker images...$(NC)"
	docker rmi jrow:latest jrow:dev || true

##@ Kubernetes

k8s-apply: ## Apply Kubernetes manifests
	@echo "$(BLUE)Applying Kubernetes manifests...$(NC)"
	kubectl apply -f templates/deploy/k8s/configmap.yaml
	kubectl apply -f templates/deploy/k8s/deployment.yaml

k8s-delete: ## Delete Kubernetes resources
	@echo "$(BLUE)Deleting Kubernetes resources...$(NC)"
	kubectl delete -f templates/deploy/k8s/deployment.yaml || true
	kubectl delete -f templates/deploy/k8s/configmap.yaml || true

k8s-status: ## Check Kubernetes deployment status
	@echo "$(BLUE)Checking Kubernetes status...$(NC)"
	kubectl get deployments
	kubectl get pods
	kubectl get services

k8s-logs: ## View Kubernetes logs
	@echo "$(BLUE)Viewing Kubernetes logs...$(NC)"
	kubectl logs -f -l app=jrow-server

k8s-scale: ## Scale Kubernetes deployment (usage: make k8s-scale REPLICAS=3)
	@echo "$(BLUE)Scaling deployment to $(REPLICAS) replicas...$(NC)"
	kubectl scale deployment jrow-server --replicas=$(REPLICAS)

##@ Templates

template-gen-build: ## Build template generator tool
	@echo "$(BLUE)Building template generator...$(NC)"
	cd tools/template-gen && cargo build --release
	@echo "$(GREEN)Template generator built!$(NC)"

template-gen-install: template-gen-build ## Install template generator to PATH
	@echo "$(BLUE)Installing template generator...$(NC)"
	cp tools/template-gen/target/release/jrow-template-gen ~/.cargo/bin/
	@echo "$(GREEN)Installed to ~/.cargo/bin/jrow-template-gen$(NC)"

template-init: ## Initialize deployment templates (creates jrow-template.toml)
	@echo "$(BLUE)Initializing deployment templates...$(NC)"
	@if [ ! -f jrow-template.toml ]; then \
		cp templates/jrow-template.toml jrow-template.toml; \
		echo "$(GREEN)Created jrow-template.toml - edit and run 'make template-generate'$(NC)"; \
	else \
		echo "$(YELLOW)jrow-template.toml already exists$(NC)"; \
	fi

template-generate: ## Generate deployment files from templates
	@echo "$(BLUE)Generating deployment files...$(NC)"
	@if [ ! -f jrow-template.toml ]; then \
		echo "$(YELLOW)jrow-template.toml not found. Run 'make template-init' first$(NC)"; \
		exit 1; \
	fi
	cd tools/template-gen && cargo run --release -- -c ../../jrow-template.toml -o ../../deploy
	@echo "$(GREEN)Deployment files generated in deploy/$(NC)"

template-clean: ## Clean generated deployment files
	@echo "$(BLUE)Cleaning generated deployment files...$(NC)"
	rm -rf deploy/

##@ Deployment

deploy-docker: ## Deploy with Docker (uses generated deploy files)
	@echo "$(BLUE)Deploying with Docker...$(NC)"
	@if [ ! -d deploy ]; then \
		echo "$(YELLOW)No deploy/ directory. Run 'make template-generate' first$(NC)"; \
		exit 1; \
	fi
	cd deploy/docker && docker-compose up -d

deploy-k8s: ## Deploy to Kubernetes (uses generated deploy files)
	@echo "$(BLUE)Deploying to Kubernetes...$(NC)"
	@if [ ! -d deploy ]; then \
		echo "$(YELLOW)No deploy/ directory. Run 'make template-generate' first$(NC)"; \
		exit 1; \
	fi
	kubectl apply -f deploy/k8s/

cleanup-docker: ## Clean up Docker deployment
	@echo "$(BLUE)Cleaning up Docker deployment...$(NC)"
	@if [ -d deploy/docker ]; then \
		cd deploy/docker && docker-compose down; \
	fi

cleanup-k8s: ## Clean up Kubernetes deployment
	@echo "$(BLUE)Cleaning up Kubernetes deployment...$(NC)"
	@if [ -d deploy/k8s ]; then \
		kubectl delete -f deploy/k8s/ || true; \
	fi

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

