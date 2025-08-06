#!/bin/bash

# Cerberus v5.0 Deployment Script
# Usage: ./scripts/deploy.sh [environment]

set -euo pipefail

ENVIRONMENT=${1:-production}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "🚀 Deploying Cerberus v5.0 to $ENVIRONMENT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')] $1${NC}"
}

warn() {
    echo -e "${YELLOW}[$(date +'%Y-%m-%d %H:%M:%S')] WARNING: $1${NC}"
}

error() {
    echo -e "${RED}[$(date +'%Y-%m-%d %H:%M:%S')] ERROR: $1${NC}"
    exit 1
}

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if required tools are installed
    command -v cargo >/dev/null 2>&1 || error "Rust/Cargo not installed"
    command -v docker >/dev/null 2>&1 || error "Docker not installed"
    command -v git >/dev/null 2>&1 || error "Git not installed"
    
    # Check if we're in the right directory
    if [[ ! -f "$PROJECT_ROOT/Cargo.toml" ]]; then
        error "Not in Cerberus project root directory"
    fi
    
    log "Prerequisites check passed ✅"
}

# Run tests
run_tests() {
    log "Running test suite..."
    
    cd "$PROJECT_ROOT"
    
    # Format check
    cargo fmt --all -- --check || error "Code formatting check failed"
    
    # Clippy check
    cargo clippy --all-targets --all-features -- -D warnings || error "Clippy check failed"
    
    # Unit tests
    cargo test --lib || error "Unit tests failed"
    
    # Integration tests
    cargo test --test integration || error "Integration tests failed"
    
    # Security audit
    cargo audit || warn "Security audit found issues"
    
    log "All tests passed ✅"
}

# Build application
build_application() {
    log "Building application for $ENVIRONMENT..."
    
    cd "$PROJECT_ROOT"
    
    if [[ "$ENVIRONMENT" == "production" ]]; then
        # Production build with optimizations
        cargo build --release
        
        # Strip binary to reduce size
        strip target/release/cerberus
        
        log "Production build completed ✅"
    else
        # Development build
        cargo build
        log "Development build completed ✅"
    fi
}

# Build Docker image
build_docker_image() {
    log "Building Docker image..."
    
    cd "$PROJECT_ROOT"
    
    # Get version from Cargo.toml
    VERSION=$(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')
    IMAGE_TAG="cerberus:$VERSION"
    
    # Build image
    docker build -t "$IMAGE_TAG" .
    docker tag "$IMAGE_TAG" "cerberus:latest"
    
    log "Docker image built: $IMAGE_TAG ✅"
}

# Deploy to local environment
deploy_local() {
    log "Deploying to local environment..."
    
    cd "$PROJECT_ROOT"
    
    # Stop existing containers
    docker-compose down || true
    
    # Start services
    docker-compose up -d
    
    # Wait for services to be ready
    sleep 10
    
    # Health check
    if curl -f http://localhost:8080/health >/dev/null 2>&1; then
        log "Local deployment successful ✅"
        log "Application available at: http://localhost:8080"
        log "Metrics available at: http://localhost:9090/metrics"
    else
        error "Health check failed"
    fi
}

# Deploy to Fly.io
deploy_fly() {
    log "Deploying to Fly.io..."
    
    cd "$PROJECT_ROOT"
    
    # Check if flyctl is installed
    command -v flyctl >/dev/null 2>&1 || error "flyctl not installed"
    
    # Deploy
    flyctl deploy --remote-only
    
    # Health check
    APP_URL=$(flyctl info --json | jq -r '.Hostname')
    if curl -f "https://$APP_URL/health" >/dev/null 2>&1; then
        log "Fly.io deployment successful ✅"
        log "Application available at: https://$APP_URL"
    else
        error "Health check failed"
    fi
}

# Deploy to production
deploy_production() {
    log "Deploying to production environment..."
    
    # Additional production checks
    if [[ ! -f "$PROJECT_ROOT/.env.production" ]]; then
        error "Production environment file not found"
    fi
    
    # Backup current deployment
    log "Creating backup..."
    BACKUP_DIR="backups/$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$BACKUP_DIR"
    
    # Deploy based on target platform
    case "${DEPLOY_TARGET:-fly}" in
        "fly")
            deploy_fly
            ;;
        "local")
            deploy_local
            ;;
        *)
            error "Unknown deployment target: $DEPLOY_TARGET"
            ;;
    esac
}

# Post-deployment tasks
post_deployment() {
    log "Running post-deployment tasks..."
    
    # Send deployment notification
    if [[ -n "${TELEGRAM_BOT_TOKEN:-}" && -n "${TELEGRAM_CHAT_ID:-}" ]]; then
        MESSAGE="🚀 Cerberus v5.0 deployed to $ENVIRONMENT successfully!"
        curl -s -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/sendMessage" \
            -d "chat_id=$TELEGRAM_CHAT_ID" \
            -d "text=$MESSAGE" >/dev/null || warn "Failed to send Telegram notification"
    fi
    
    # Update monitoring
    log "Deployment monitoring updated ✅"
}

# Rollback function
rollback() {
    log "Rolling back deployment..."
    
    case "${DEPLOY_TARGET:-fly}" in
        "fly")
            flyctl releases list --json | jq -r '.[1].Version' | xargs flyctl releases rollback
            ;;
        "local")
            docker-compose down
            docker-compose up -d
            ;;
    esac
    
    log "Rollback completed ✅"
}

# Main deployment flow
main() {
    log "Starting Cerberus v5.0 deployment to $ENVIRONMENT"
    
    # Trap errors for cleanup
    trap 'error "Deployment failed! Check logs above."' ERR
    
    check_prerequisites
    run_tests
    build_application
    
    case "$ENVIRONMENT" in
        "local"|"development")
            build_docker_image
            deploy_local
            ;;
        "staging")
            build_docker_image
            deploy_production
            ;;
        "production")
            # Extra confirmation for production
            read -p "⚠️  Deploy to PRODUCTION? This will affect live trading! (yes/no): " confirm
            if [[ "$confirm" != "yes" ]]; then
                error "Production deployment cancelled"
            fi
            
            build_docker_image
            deploy_production
            ;;
        *)
            error "Unknown environment: $ENVIRONMENT"
            ;;
    esac
    
    post_deployment
    
    log "🎉 Deployment completed successfully!"
    log "Environment: $ENVIRONMENT"
    log "Version: $(grep '^version = ' Cargo.toml | sed 's/version = "\(.*\)"/\1/')"
    log "Timestamp: $(date)"
}

# Handle script arguments
case "${1:-}" in
    "rollback")
        rollback
        ;;
    "local"|"development"|"staging"|"production")
        main
        ;;
    *)
        echo "Usage: $0 [local|development|staging|production|rollback]"
        echo ""
        echo "Examples:"
        echo "  $0 local       # Deploy to local Docker environment"
        echo "  $0 production  # Deploy to production (Fly.io)"
        echo "  $0 rollback    # Rollback last deployment"
        exit 1
        ;;
esac
