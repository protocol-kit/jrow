.PHONY: help build run-server run-client test fmt clippy clean dev

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build the project
	cargo build --release

build-dev: ## Build in debug mode
	cargo build

run-server: ## Run the server
	cargo run --bin server

run-client: ## Run the client
	cargo run --bin client

dev: ## Run server with auto-reload (requires cargo-watch)
	cargo watch -x 'run --bin server'

test: ## Run tests
	cargo test

test-verbose: ## Run tests with output
	cargo test -- --nocapture

fmt: ## Format code
	cargo fmt

clippy: ## Run clippy linter
	cargo clippy -- -D warnings

check: ## Quick check without building
	cargo check

clean: ## Clean build artifacts
	cargo clean

install-dev-tools: ## Install development tools
	cargo install cargo-watch

docker-build: ## Build Docker image
	docker build -t my-jrow-app .

docker-run: ## Run Docker container
	docker run -p 8080:8080 my-jrow-app

# Benchmarking (requires cargo-bench)
bench: ## Run benchmarks
	cargo bench

# Documentation
doc: ## Generate documentation
	cargo doc --open

# Release
release: test clippy ## Build release binary
	cargo build --release
	@echo "Release binary: ./target/release/server"
	@echo "Release binary: ./target/release/client"



