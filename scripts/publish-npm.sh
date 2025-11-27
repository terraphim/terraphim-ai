#!/usr/bin/env bash
set -euo pipefail

################################################################################
# publish-npm.sh
#
# Publish Node.js package to npm registry
#
# Usage:
#   ./scripts/publish-npm.sh [OPTIONS]
#
# Options:
#   -v, --version VERSION     Version to publish (e.g., 1.2.3)
#   -d, --dry-run            Dry run mode (validate only)
#   -t, --tag TAG           npm tag: latest, beta, alpha, next (default: latest)
#   -T, --token TOKEN       npm token
#   -h, --help               Show help message
#
# Examples:
#   # Publish to npm
#   ./scripts/publish-npm.sh -v 1.2.3
#
#   # Dry run with beta tag
#   ./scripts/publish-npm.sh -v 1.2.3-beta.1 -d -t beta
#
#   # Use specific token
#   ./scripts/publish-npm.sh -v 1.2.3 -T $NPM_TOKEN
#
################################################################################

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Default values
DRY_RUN=false
VERSION=""
TAG="latest"
TOKEN=""
PACKAGE_DIR="terraphim_ai_nodejs"

# Logging functions
log_info() {
  echo -e "${BLUE}INFO:${NC} $1"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1"
}

log_warning() {
  echo -e "${YELLOW}⚠${NC} $1"
}

log_error() {
  echo -e "${RED}✗${NC} $1"
}

# Help function
show_help() {
  sed -n '2,30p' "$0" | head -n -1 | sed 's/^# //'
  exit 0
}

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case $1 in
      -v|--version)
        VERSION="$2"
        shift 2
        ;;
      -d|--dry-run)
        DRY_RUN=true
        shift
        ;;
      -t|--tag)
        TAG="$2"
        shift 2
        ;;
      -T|--token)
        TOKEN="$2"
        shift 2
        ;;
      -h|--help)
        show_help
        ;;
      *)
        log_error "Unknown option: $1"
        show_help
        ;;
    esac
  done
}

# Validate prerequisites
check_prerequisites() {
  log_info "Checking prerequisites..."

  # Check if yarn is available
  if ! command -v yarn &> /dev/null; then
    log_error "yarn not found. Please install Node.js and yarn."
    exit 1
  fi

  # Check if in correct directory
  if [[ ! -f "$PACKAGE_DIR/package.json" ]]; then
    log_error "Package directory $PACKAGE_DIR not found"
    log_error "Make sure you're running from the repository root"
    exit 1
  fi

  # Check npm is available
  if ! command -v npm &> /dev/null; then
    log_error "npm not found. Please install Node.js."
    exit 1
  fi

  log_success "Prerequisites validated"

  # Check token - try 1Password first if available, then environment
  if [[ -z "$TOKEN" ]]; then
    # Try to get token from 1Password if op CLI is available
    if command -v op &> /dev/null; then
      TOKEN=$(op read "op://TerraphimPlatform/npm.token/password" 2>/dev/null || echo "")
      if [[ -n "$TOKEN" ]]; then
        export NPM_TOKEN="$TOKEN"
        log_info "Using npm token from 1Password"
      fi
    fi

    # If still no token, try environment variable
    if [[ -z "$TOKEN" ]] && [[ -n "${NPM_TOKEN:-}" ]]; then
      TOKEN="$NPM_TOKEN"
      log_info "Using npm token from environment"
    fi

    # If still no token, show warning
    if [[ -z "$TOKEN" ]]; then
      log_info "No npm token provided. Will use npm configuration or prompt."
    fi
  else
    export NPM_TOKEN="$TOKEN"
    log_info "Using provided token for authentication"
  fi
}

# Update version in package.json
update_version() {
  log_info "Updating version to $VERSION..."

  cd "$PACKAGE_DIR"

  if [[ "$OSTYPE" == "darwin"* ]]; then
    sed -i '' "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json
  else
    sed -i "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" package.json
  fi

  cd -

  log_success "Version updated in package.json"
}

# Get current version
get_current_version() {
  cd "$PACKAGE_DIR"
  node -p "require('./package.json').version"
  cd -
}

# Install dependencies
install_dependencies() {
  log_info "Installing dependencies..."

  cd "$PACKAGE_DIR"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: yarn install --frozen-lockfile"
  else
    yarn install --frozen-lockfile
  fi

  cd -

  log_success "Dependencies installed"
}

# Build package
build_package() {
  log_info "Building package..."

  cd "$PACKAGE_DIR"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: yarn build"
  else
    yarn build
  fi

  cd -

  log_success "Package built"
}

# Validate package
validate_package() {
  log_info "Validating package.json..."

  cd "$PACKAGE_DIR"

  # Check package.json is valid
  if node -e "const pkg = require('./package.json'); console.log('Package:', pkg.name); console.log('Version:', pkg.version);"; then
    log_success "Package.json is valid"
  else
    log_error "Package.json validation failed"
    exit 1
  fi

  # Check if main files exist
  local main_file
  main_file=$(node -p "require('./package.json').main")

  if [[ -f "$main_file" ]]; then
    log_success "Main file exists: $main_file"
  else
    log_error "Main file not found: $main_file"
    exit 1
  fi

  cd -
}

# Check if version already exists
check_if_published() {
  local pkg_version="$1"

  log_info "Checking if version $pkg_version already exists on npm..."

  cd "$PACKAGE_DIR"

  local pkg_name
  pkg_name=$(node -p "require('./package.json').name")

  if npm view "$pkg_name@$pkg_version" version 2>&1 | grep -q "$pkg_version"; then
    log_warning "Version $pkg_version already exists on npm"
    cd -
    return 0
  fi

  cd -
  return 1
}

# Configure npm for publishing
configure_npm() {
  log_info "Configuring npm..."

  cd "$PACKAGE_DIR"

  # Set token if provided
  if [[ -n "$TOKEN" ]]; then
    npm config set //registry.npmjs.org/:_authToken="$TOKEN"
    log_info "Token configured"
  fi

  # Enable provenance
  npm config set provenance true

  log_success "npm configured"
  cd -
}

# Publish to npm
publish_to_npm() {
  log_info "Publishing to npm as @$TAG..."

  cd "$PACKAGE_DIR"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: npm publish --access public --tag $TAG --dry-run"
    npm publish --access public --tag "$TAG" --dry-run
  else
    log_info "Running: npm publish --access public --tag $TAG"
    npm publish --access public --tag "$TAG"
    log_success "Published to npm successfully!"
  fi

  cd -
}

# Test installation
test_installation() {
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Skipping installation test (dry-run)"
    return 0
  fi

  log_info "Testing installation from npm..."

  local pkg_name
  local pkg_version

  cd "$PACKAGE_DIR"
  pkg_name=$(node -p "require('./package.json').name")
  pkg_version=$(node -p "require('./package.json').version")
  cd -

  # Wait a moment
  sleep 30

  # Create temp directory
  local test_dir
  test_dir=$(mktemp -d)

  # Try to install
  if cd "$test_dir" && npm install "$pkg_name@$pkg_version"; then
    log_success "Test installation succeeded"
  else
    log_warning "Test installation failed (package may not be indexed yet)"
  fi

  # Cleanup
  rm -rf "$test_dir"
}

# Show summary
show_summary() {
  cd "$PACKAGE_DIR"
  local pkg_name
  local pkg_version

  pkg_name=$(node -p "require('./package.json').name")
  pkg_version=$(node -p "require('./package.json').version")
  cd -

  log_info "Summary:"
  log_info "  Package: $pkg_name"
  log_info "  Version: $pkg_version"
  log_info "  Tag: $TAG"
  log_info "  Registry: npm"

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Mode: Dry-run (no actual publish)"
  else
    log_success "Published successfully!"
    log_info "URL: https://www.npmjs.com/package/$pkg_name"
  fi
}

# Main function
main() {
  check_prerequisites

  # Get or set version
  if [[ -z "$VERSION" ]]; then
    VERSION=$(get_current_version)
    log_info "Using current version: $VERSION"
  fi

  # Check if already published
  if check_if_published "$VERSION"; then
    if [[ "$DRY_RUN" != "true" ]]; then
      log_error "Version $VERSION already exists"
      exit 1
    fi
  fi

  # Update version if provided
  if [[ "$VERSION" != "$(get_current_version)" ]]; then
    update_version
  fi

  # Install dependencies
  install_dependencies

  # Build
  build_package

  # Validate
  validate_package

  # Configure npm
  configure_npm

  # Publish
  publish_to_npm

  # Test installation
  test_installation

  # Show summary
  show_summary
}

# Parse arguments and run
parse_args "$@"
main "$@"
