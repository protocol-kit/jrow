#!/bin/bash
set -e

# JROW Deployment Script
# This script helps deploy JROW server in various environments

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Deploy with Docker
deploy_docker() {
    log_info "Deploying with Docker..."
    
    if ! command_exists docker; then
        log_error "Docker is not installed"
        exit 1
    fi
    
    cd "$PROJECT_ROOT/templates/deploy/docker"
    
    log_info "Building Docker image..."
    docker build -t jrow:latest -f Dockerfile ../../..
    
    log_info "Starting containers..."
    docker-compose up -d
    
    log_info "Docker deployment complete!"
    log_info "Access the server at: ws://localhost:8080"
}

# Deploy to Kubernetes
deploy_k8s() {
    log_info "Deploying to Kubernetes..."
    
    if ! command_exists kubectl; then
        log_error "kubectl is not installed"
        exit 1
    fi
    
    cd "$PROJECT_ROOT/templates/deploy/k8s"
    
    log_info "Applying ConfigMap..."
    kubectl apply -f configmap.yaml
    
    log_info "Applying Deployment..."
    kubectl apply -f deployment.yaml
    
    log_info "Waiting for deployment to be ready..."
    kubectl wait --for=condition=available --timeout=300s deployment/jrow-server
    
    log_info "Kubernetes deployment complete!"
    log_info "Get service endpoint:"
    log_info "  kubectl get service jrow-server"
}

# Build release binary
build_release() {
    log_info "Building release binary..."
    
    cd "$PROJECT_ROOT"
    cargo build --release
    
    log_info "Release binary built at: target/release/"
}

# Run locally
run_local() {
    log_info "Running server locally..."
    
    cd "$PROJECT_ROOT"
    
    if [ -n "$1" ]; then
        log_info "Running example: $1"
        cargo run --release --example "$1"
    else
        log_info "Running simple_server example..."
        cargo run --release --example simple_server
    fi
}

# Clean up
cleanup() {
    log_info "Cleaning up..."
    
    if [ "$1" = "docker" ]; then
        cd "$PROJECT_ROOT/templates/deploy/docker"
        docker-compose down
        log_info "Docker containers stopped"
    elif [ "$1" = "k8s" ]; then
        cd "$PROJECT_ROOT/templates/deploy/k8s"
        kubectl delete -f deployment.yaml
        kubectl delete -f configmap.yaml
        log_info "Kubernetes resources deleted"
    fi
}

# Show usage
usage() {
    cat << EOF
Usage: $0 <command> [options]

Commands:
    docker              Deploy using Docker Compose
    k8s                 Deploy to Kubernetes
    build               Build release binary
    run [example]       Run locally (optionally specify example)
    cleanup <docker|k8s> Clean up deployment
    help                Show this help message

Examples:
    $0 docker           # Deploy with Docker
    $0 k8s              # Deploy to Kubernetes
    $0 run              # Run simple_server locally
    $0 run pubsub       # Run pubsub example locally
    $0 cleanup docker   # Stop Docker containers

EOF
}

# Main
main() {
    if [ $# -eq 0 ]; then
        usage
        exit 1
    fi
    
    case "$1" in
        docker)
            deploy_docker
            ;;
        k8s)
            deploy_k8s
            ;;
        build)
            build_release
            ;;
        run)
            run_local "$2"
            ;;
        cleanup)
            cleanup "$2"
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown command: $1"
            usage
            exit 1
            ;;
    esac
}

main "$@"

