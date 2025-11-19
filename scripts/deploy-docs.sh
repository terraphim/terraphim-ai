#!/usr/bin/env bash
# Deploy Terraphim AI Documentation to Cloudflare Pages
# Usage: ./scripts/deploy-docs.sh [production|preview]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
DOCS_DIR="$PROJECT_ROOT/docs"
BUILD_DIR="$DOCS_DIR/book"

# Default environment
ENVIRONMENT="${1:-preview}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check for mdbook
    if ! command -v mdbook &> /dev/null; then
        log_error "mdbook is not installed. Install with: cargo install mdbook"
        exit 1
    fi

    # Check for wrangler
    if ! command -v wrangler &> /dev/null; then
        log_error "wrangler is not installed. Install with: npm install -g wrangler"
        exit 1
    fi

    # Check for mdbook-mermaid (optional)
    if ! command -v mdbook-mermaid &> /dev/null; then
        log_warning "mdbook-mermaid not installed. Mermaid diagrams may not render."
        log_warning "Install with: cargo install mdbook-mermaid"
    fi

    log_success "Prerequisites check passed"
}

# Build documentation
build_docs() {
    log_info "Building documentation..."

    cd "$DOCS_DIR"

    # Install mermaid support if available
    if command -v mdbook-mermaid &> /dev/null; then
        mdbook-mermaid install . 2>/dev/null || true
    fi

    # Build the book
    mdbook build

    log_success "Documentation built successfully"
    log_info "Build output: $BUILD_DIR"
}

# Deploy to Cloudflare Pages
deploy() {
    log_info "Deploying to Cloudflare Pages (Environment: $ENVIRONMENT)..."

    cd "$PROJECT_ROOT"

    if [[ "$ENVIRONMENT" == "production" ]]; then
        # Production deployment (main branch)
        wrangler pages deploy "$BUILD_DIR" \
            --project-name=terraphim-docs \
            --branch=main \
            --commit-dirty=true

        log_success "Production deployment complete!"
        log_info "URL: https://doc.terraphim.ai"
    else
        # Preview deployment
        BRANCH_NAME="${BRANCH_NAME:-preview-$(date +%s)}"

        wrangler pages deploy "$BUILD_DIR" \
            --project-name=terraphim-docs \
            --branch="$BRANCH_NAME"

        log_success "Preview deployment complete!"
        log_info "Check Cloudflare dashboard for preview URL"
    fi
}

# Cleanup
cleanup() {
    log_info "Cleaning up build artifacts..."
    rm -rf "$BUILD_DIR"
    log_success "Cleanup complete"
}

# Main execution
main() {
    echo ""
    echo "======================================"
    echo "  Terraphim AI Documentation Deploy  "
    echo "======================================"
    echo ""

    if [[ "$ENVIRONMENT" != "production" && "$ENVIRONMENT" != "preview" ]]; then
        log_error "Invalid environment: $ENVIRONMENT"
        log_info "Usage: $0 [production|preview]"
        exit 1
    fi

    log_info "Environment: $ENVIRONMENT"
    log_info "Project root: $PROJECT_ROOT"
    echo ""

    check_prerequisites
    build_docs
    deploy

    echo ""
    log_success "Deployment pipeline completed successfully!"
    echo ""
}

# Handle cleanup on exit
trap cleanup EXIT

main "$@"
