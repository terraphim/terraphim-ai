#!/usr/bin/env bash
set -euo pipefail

################################################################################
# publish-pypi.sh
#
# Publish Python package to PyPI
#
# Usage:
#   ./scripts/publish-pypi.sh [OPTIONS]
#
# Options:
#   -v, --version VERSION     Version to publish (e.g., 1.2.3)
#   -d, --dry-run            Dry run mode (validate only)
#   -r, --repository REPO    Repository: pypi or testpypi (default: pypi)
#   -t, --token TOKEN        PyPI API token
#   -h, --help               Show help message
#
# Examples:
#   # Publish to PyPI
#   ./scripts/publish-pypi.sh -v 1.2.3
#
#   # Dry run to TestPyPI
#   ./scripts/publish-pypi.sh -v 1.2.3 -d -r testpypi
#
#   # Use specific token
#   ./scripts/publish-pypi.sh -v 1.2.3 -t $PYPI_API_TOKEN
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
REPOSITORY="pypi"
TOKEN=""
PACKAGE_DIR="crates/terraphim_automata_py"

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
      -r|--repository)
        REPOSITORY="$2"
        shift 2
        ;;
      -t|--token)
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

  # Check if in correct directory
  if [[ ! -f "$PACKAGE_DIR/pyproject.toml" ]]; then
    log_error "Package directory $PACKAGE_DIR not found"
    log_error "Make sure you're running from the repository root"
    exit 1
  fi

  # Check required tools
  if ! command -v python3 &> /dev/null; then
    log_error "python3 not found"
    exit 1
  fi

  if ! command -v cargo &> /dev/null; then
    log_error "cargo not found"
    exit 1
  fi

  # Install maturin if not available
  if ! python3 -m maturin --version &> /dev/null; then
    log_info "Installing maturin..."
    python3 -m pip install --user maturin
  fi

  log_success "Prerequisites validated"
}

# Update version in pyproject.toml and Cargo.toml
update_version() {
  log_info "Updating version to $VERSION..."

  # Update pyproject.toml
  if [[ -f "$PACKAGE_DIR/pyproject.toml" ]]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
      sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$PACKAGE_DIR/pyproject.toml"
    else
      sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PACKAGE_DIR/pyproject.toml"
    fi
    log_success "Updated pyproject.toml"
  fi

  # Update Cargo.toml
  if [[ -f "$PACKAGE_DIR/Cargo.toml" ]]; then
    if [[ "$OSTYPE" == "darwin"* ]]; then
      sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" "$PACKAGE_DIR/Cargo.toml"
    else
      sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PACKAGE_DIR/Cargo.toml"
    fi
    log_success "Updated Cargo.toml"
  fi

  log_success "Version updated to $VERSION"
}

# Get current version
get_current_version() {
  if [[ -f "$PACKAGE_DIR/pyproject.toml" ]]; then
    grep "^version" "$PACKAGE_DIR/pyproject.toml" | head -1 | cut -d'"' -f2 | tr -d ' '
  fi
}

# Build distributions
build_distributions() {
  log_info "Building Python distributions..."

  # Clean previous builds
  rm -rf "$PACKAGE_DIR/dist"
  mkdir -p "$PACKAGE_DIR/dist"

  # Build wheels
  log_info "Building wheel..."
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: maturin build --release --out dist"
    (cd "$PACKAGE_DIR" && python3 -m maturin build --release --out dist --find-interpreter)
  else
    (cd "$PACKAGE_DIR" && python3 -m maturin build --release --out dist --find-interpreter)
    log_success "Wheel built successfully"
  fi

  # Build source distribution
  log_info "Building source distribution..."
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: maturin sdist --out dist"
    (cd "$PACKAGE_DIR" && python3 -m maturin sdist --out dist)
  else
    (cd "$PACKAGE_DIR" && python3 -m maturin sdist --out dist)
    log_success "Source distribution built successfully"
  fi

  # Show built distributions
  log_info "Built distributions:"
  ls -lh "$PACKAGE_DIR/dist/"
}

# Validate distributions
validate_distributions() {
  log_info "Validating distributions..."

  # Install twine if not available
  if ! python3 -m twine --version &> /dev/null; then
    log_info "Installing twine..."
    python3 -m pip install --user twine
  fi

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: twine check dist/*"
  fi

  python3 -m twine check "$PACKAGE_DIR/dist/*"
  log_success "Distribution validation passed"
}

# Check if package already exists
check_if_published() {
  local pkg_version="$1"

  log_info "Checking if version $pkg_version already exists on PyPI..."

  # Try to get package info from PyPI
  if python3 -m pip index versions "terraphim-automata" 2>/dev/null | grep -q "$pkg_version"; then
    log_warning "Version $pkg_version already exists on PyPI"
    return 0
  fi

  return 1
}

# Upload to PyPI
upload_to_pypi() {
  log_info "Uploading to $REPOSITORY..."

  # Set repository URL
  local repository_url="https://upload.pypi.org/legacy/"
  if [[ "$REPOSITORY" == "testpypi" ]]; then
    repository_url="https://test.pypi.org/legacy/"
    log_info "Using TestPyPI: $repository_url"
  fi

  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run: twine upload --skip-existing --dry-run dist/*"
    log_info "Repository: $repository_url"
  else
    if [[ -n "$TOKEN" ]]; then
      log_info "Uploading with token..."
      python3 -m twine upload \
        --repository-url "$repository_url" \
        --username "__token__" \
        --password "$TOKEN" \
        --skip-existing \
        "$PACKAGE_DIR/dist/*"
    else
      log_info "Uploading with default credentials..."
      python3 -m twine upload \
        --repository-url "$repository_url" \
        --skip-existing \
        "$PACKAGE_DIR/dist/*"
    fi

    log_success "Upload completed!"
  fi
}

# Test installation
test_installation() {
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Skipping installation test (dry-run)"
    return 0
  fi

  log_info "Testing installation from $REPOSITORY..."

  # Wait a moment for PyPI to process
  sleep 30

  local pkg_version="$1"

  # Create temporary directory for test
  local test_dir
  test_dir=$(mktemp -d)
  cd "$test_dir"

  # Try to install
  if [[ "$REPOSITORY" == "testpypi" ]]; then
    if python3 -m pip install \
      --index-url "https://test.pypi.org/simple/" \
      --extra-index-url "https://pypi.org/simple/" \
      "terraphim-automata==$pkg_version"; then
      log_success "Test installation from TestPyPI succeeded"
    else
      log_warning "Test installation failed (package may not be indexed yet)"
    fi
  else
    if python3 -m pip install "terraphim-automata==$pkg_version"; then
      log_success "Test installation from PyPI succeeded"
    else
      log_warning "Test installation failed (package may not be indexed yet)"
    fi
  fi

  # Cleanup
  cd -
  rm -rf "$test_dir"
}

# Main function
main() {
  # Validate arguments
  if [[ -z "$VERSION" ]]; then
    # Try to get current version
    VERSION=$(get_current_version)
    if [[ -z "$VERSION" ]]; then
      log_error "Version not provided and could not be determined"
      show_help
    fi
    log_info "Using current version: $VERSION"
  fi

  # Check if already published
  if ! check_if_published "$VERSION"; then
    log_info "Version $VERSION will be published"
  else
    log_warning "Version $VERSION already exists"
    if [[ "$DRY_RUN" != "true" ]]; then
      log_error "Cannot publish existing version"
      exit 1
    fi
  fi

  # Build distributions
  build_distributions

  # Validate
  validate_distributions

  # Upload
  upload_to_pypi

  # Test installation
  test_installation "$VERSION"

  # Summary
  if [[ "$DRY_RUN" == "true" ]]; then
    log_info "Dry-run completed successfully!"
    log_info "No packages were actually published"
  else
    log_success "Publishing completed successfully!"
    log_info "Package: terrraphim-automata"
    log_info "Version: $VERSION"
    log_info "Repository: $REPOSITORY"

    if [[ "$REPOSITORY" == "testpypi" ]]; then
      log_info "URL: https://test.pypi.org/project/terraphim-automata/"
    else
      log_info "URL: https://pypi.org/project/terraphim-automata/"
    fi
  fi
}

# Parse arguments and run
parse_args "$@"
check_prerequisites
main "$@"
